//! A window onto the simulation - blocks and all, for now.
//!
//! ```text
//! cargo run -p ruage-view --bin window -- config/ [--ticks-per-frame N]
//! ```
//!
//! The loop is the point, not the picture: the simulation runs at its own fixed rate (`sim.toml`'s `tick_rate_hz`) and the
//! window draws whatever tick the world has reached, so the two are decoupled and the sim never learns that a screen
//! exists (D55). The rasteriser produces a plain pixel buffer, and `softbuffer` puts it on the surface - no GPU, no
//! shaders, nothing that can disagree with the headless build about what a tick is.
//!
//! What it does not have yet: input (no selection, no orders), a HUD, sprites, terrain art. Those are the next steps in
//! `docs/next-step.md`, and each of them is additive: the loop stays, the blit stays.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use ruage_data::{Economy, Map, Units};
use ruage_sim::{Runner, World, load_scenario_file};
use ruage_view::ui;
use ruage_view::{Camera, Highlights, Pace, Snapshot, Speed, render_with};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

struct Config {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    units: Units,
    combat: ruage_data::Combat,
    resources: ruage_data::Resources,
    // A window that plays the pack but installs no behaviour shows villagers standing on a berry bush: the content
    // policy only *reallocates* villagers, and without the rules nothing trains or builds. So the view loads what the
    // headless runner loads - research, civs and the scenario - rather than a subset that looks alive.
    techs: ruage_data::Techs,
    effects: ruage_data::Effects,
    civs: ruage_data::Civs,
    scenario: ruage_sim::Scenario,
}

fn load(config: &Path) -> Result<Config, String> {
    let resources = ruage_data::load_resources_file(&config.join("resources.toml"))
        .map_err(|err| format!("resources.toml: {err}"))?;
    Ok(Config {
        settings: ruage_data::load_sim_file(&config.join("sim.toml"))
            .map_err(|err| format!("sim.toml: {err}"))?,
        map: ruage_data::load_map_file(&config.join("map.toml"), &resources)
            .map_err(|err| format!("map.toml: {err}"))?,
        economy: ruage_data::load_economy_file(&config.join("economy.toml"))
            .map_err(|err| format!("economy.toml: {err}"))?,
        units: ruage_data::load_units_file(&config.join("gamedata/units.toml"))
            .map_err(|err| format!("units.toml: {err}"))?,
        combat: ruage_data::load_combat_file(&config.join("combat.toml"))
            .map_err(|err| format!("combat.toml: {err}"))?,
        techs: ruage_data::load_techs_file(&config.join("gamedata/techs.toml"))
            .map_err(|err| format!("techs.toml: {err}"))?,
        effects: ruage_data::load_effects_file(&config.join("gamedata/effects.toml"))
            .map_err(|err| format!("effects.toml: {err}"))?,
        civs: ruage_data::load_civs_file(&config.join("gamedata/civs.toml"))
            .map_err(|err| format!("civs.toml: {err}"))?,
        // The same match the headless runner plays, so what the window shows is what the tests assert.
        scenario: load_scenario_file(&config.join("acceptance.toml"))
            .map_err(|err| format!("acceptance.toml: {err}"))?,
        resources,
    })
}

/// The whole application: one world, one window, one accumulator.
struct App {
    config: Config,
    world: Option<World>,
    /// The scenario's steps and the personality's rules. `None` until the world is seeded.
    runner: Option<Runner>,
    window: Option<std::rc::Rc<Window>>,
    surface: Option<softbuffer::Surface<std::rc::Rc<Window>, std::rc::Rc<Window>>>,
    context: Option<softbuffer::Context<std::rc::Rc<Window>>>,
    camera: Camera,
    /// Where the last order went, so the picture can show it. A tile, not an entity: the order is a place.
    order: Option<(f32, f32)>,
    /// How long one simulation tick takes in wall-clock terms: the accumulator's unit.
    tick: Duration,
    /// Time not yet spent on a tick.
    accumulated: Duration,
    last_frame: Instant,
    /// Ticks run so far, shown in the title so a screenshot proves the loop is alive.
    ticks: u64,
    /// Set when a tick failed: the loop keeps drawing the last good state and stops stepping. A sim error is not something
    /// to retry twenty times a second - it would neither fix itself nor leave the state coherent.
    failed: bool,
    /// The seed, so a window session can be reproduced headless.
    seed: u64,
    /// What the player has selected. Orders go to these and nothing else, exactly as a test would name them. It is the
    /// view's own `Selection` - the type that knows how a click, a box and a double-click pick - rather than a second,
    /// hand-rolled copy of the rules.
    selection: ui::Selection,
    /// Where the left button went down: a press and a release in the same place is a click, further apart is a box.
    press_at: Option<(i32, i32)>,
    /// When and where the last left click landed, so a second one in the same place in time is a double-click.
    last_click: Option<(Instant, (i32, i32))>,
    /// The last refusal and the tick it happened on: shown in the HUD for a few seconds, because an order that was not
    /// accepted has to look different from one that was.
    notice: Option<(String, u64)>,
    /// Escape opens this: a modal that stops the clock (`step` returns early) and offers resume, restart and quit.
    menu_open: bool,
    menu_hover: Option<usize>,
    /// **The command card**: what the selected building offers, each button with the command it would send. Rebuilt when
    /// the selection changes or an order lands, never per frame - it is a query over every tech the building knows.
    card: Vec<(ui::CardButton, ruage_sim::CardTarget)>,
    card_hover: Option<usize>,
    /// Which of the bar's five buttons the pointer is on.
    bar_hover: Option<usize>,
    /// Paused freezes the accumulator rather than the loop: the window keeps drawing, the match stops.
    paused: bool,
    /// The window's pace (1x, 1.5x, 2x, 3x, 5x). **Presentation only**: the simulation is fixed-rate (D16), so a speed
    /// change is how many ticks this window runs per interval and nothing in the match knows. A change is allowed
    /// mid-match and is logged here, not recorded in the simulation - a replay is a tick stream.
    pace: Pace,
    /// The tile under the pointer, in tile space.
    cursor: (i32, i32),
    /// The same click in **continuous** tile coordinates: what a move order carries (A), so a unit walks to the place the
    /// player pointed at rather than to the centre of the diamond under the pointer.
    cursor_point: (f32, f32),
    /// The pointer in **pixels**, which is what the HUD's speed selector is picked in: the HUD is not on the ground.
    cursor_pixel: (i32, i32),
}

impl App {
    fn new(config: Config, seed: u64) -> App {
        let tick_hz = config.settings.tick_rate_hz.max(1);
        App {
            config,
            world: None,
            runner: None,
            window: None,
            surface: None,
            context: None,
            camera: Camera::new((960, 720)).centred_on((16.0, 16.0)),
            order: None,
            tick: Duration::from_nanos(1_000_000_000 / u64::from(tick_hz)),
            accumulated: Duration::ZERO,
            last_frame: Instant::now(),
            ticks: 0,
            failed: false,
            seed,
            selection: ui::Selection::new(),
            press_at: None,
            last_click: None,
            notice: None,
            menu_open: false,
            menu_hover: None,
            card: Vec::new(),
            card_hover: None,
            bar_hover: None,
            paused: false,
            pace: Pace::new(Speed::One),
            cursor: (0, 0),
            cursor_point: (0.0, 0.0),
            cursor_pixel: (0, 0),
        }
    }

    /// What is under a tile. The decision lives in `ruage_view::pick_at` so it can be tested without a screen.
    fn pick_entity(&self, tile: (i32, i32)) -> Option<ruage_sim::EntityId> {
        ruage_view::pick_at(self.world.as_ref()?, tile)
    }

    /// A press of the left button selects what is under the pointer (empty space clears the selection); the right button
    /// gives the selection an order. Every order goes through `submit_checked`, so a refusal is reported rather than lost.
    fn click(&mut self, button: winit::event::MouseButton) {
        match button {
            winit::event::MouseButton::Left => {
                // The menu takes the click while it is open: it is modal, and the world under it is not a target.
                if self.menu_open {
                    let (width, height) = self.window.as_ref().map_or((0, 0), |window| {
                        let size = window.inner_size();
                        (size.width, size.height)
                    });
                    match ui::menu_at(self.cursor_pixel, width, height) {
                        Some(0) => {
                            self.menu_open = false;
                            self.menu_hover = None;
                        }
                        Some(1) => self.restart(),
                        Some(2) => std::process::exit(0),
                        _ => {}
                    }
                    return;
                }
                // The HUD's speed selector first: it is drawn over the world, so a click on it is a click on the HUD and
                // must not also select whatever stands under it.
                let (frame_width, frame_height) = self.frame_size();
                let layout = ui::Layout::of(frame_width, frame_height);
                // **The bar's five buttons**: the menu works, the rest say they are not built. A button that opens
                // nothing and says nothing is the one thing worse than no button.
                if let Some(index) = ui::bar_button_at(self.cursor_pixel, &layout) {
                    match index {
                        0 => {
                            self.menu_open = true;
                            self.menu_hover = None;
                        }
                        _ => {
                            self.notice = Some((
                                format!("{} is not built yet", ui::BAR_LABELS[index]),
                                self.ticks,
                            ));
                        }
                    }
                    return;
                }
                if let Some(speed) = ui::speed_at(self.cursor_pixel, frame_width, frame_height) {
                    self.pace.set(speed);
                    eprintln!("speed -> {}", speed.label());
                    return;
                }
                // The command card next: it is drawn above the panel, so a click on a button is not a click on the ground.
                let (width, height) = self.frame_size();
                if let Some(index) = ui::card_at(self.cursor_pixel, self.card.len(), width, height)
                {
                    self.use_card(index);
                    return;
                }
                // The minimap next, for the same reason: it is a picture of the world drawn over the world.
                if let Some(tile) = self.minimap_at(self.cursor_pixel) {
                    self.camera = self
                        .camera
                        .centred_on((f32::from(tile.0), f32::from(tile.1)));
                    return;
                }
                // **Anything else on the interface is still the interface**: the empty parts of the bars swallow the click
                // rather than selecting whatever is behind them (openaok asks its art's alpha; our layout is the mask).
                if ui::over_hud(self.cursor_pixel, &layout) {
                    return;
                }
                // Otherwise this is either a click or the start of a box; which one is decided on release.
                self.press_at = Some(self.cursor_pixel);
            }
            winit::event::MouseButton::Right => {
                let (tile, point) = (self.cursor, self.cursor_point);
                self.order_at(tile, point);
            }
            _ => {}
        }
    }

    /// Rebuild the command card from what is selected: one building, the player's own, gives buttons.
    fn refresh_card(&mut self) {
        self.card.clear();
        self.card_hover = None;
        if self.selection.ids.len() != 1 {
            return;
        }
        let Some(world) = self.world.as_ref() else {
            return;
        };
        let id = ruage_sim::EntityId(self.selection.ids[0]);
        for item in world.card(1, id) {
            self.card.push((
                ui::CardButton {
                    name: item.name,
                    costs: item.costs,
                    refusal: item.refusal,
                },
                item.target,
            ));
        }
    }

    /// Send the card's order: train a unit, or research a tech, on the selected building.
    fn use_card(&mut self, index: usize) {
        let Some((_, target)) = self.card.get(index).cloned() else {
            return;
        };
        let Some(building) = self.selection.ids.first().copied() else {
            return;
        };
        let building = ruage_sim::EntityId(building);
        let command = match target {
            ruage_sim::CardTarget::Unit(unit) => ruage_sim::Command::Train { building, unit },
            ruage_sim::CardTarget::Tech(tech) => ruage_sim::Command::Research { building, tech },
        };
        let Some(world) = self.world.as_mut() else {
            return;
        };
        match world.submit_checked(command) {
            Ok(()) => self.notice = None,
            Err(err) => self.notice = Some((err.to_string(), self.ticks)),
        }
        // The card says what the *next* button does: a queue may now be full, or the money gone.
        self.refresh_card();
    }

    /// Which map tile a pixel on the minimap means, if it is on the minimap at all.
    fn minimap_at(&self, pixel: (i32, i32)) -> Option<(u16, u16)> {
        let (width, height) = self.frame_size();
        let (map_width, map_height) = (self.config.map.width, self.config.map.height);
        let rect = ui::minimap_rect(width, height, map_width, map_height);
        ui::minimap_tile(pixel, rect, map_width, map_height)
    }

    /// The frame's size in pixels, or zero before the window exists.
    fn frame_size(&self) -> (u32, u32) {
        self.window.as_ref().map_or((0, 0), |window| {
            let size = window.inner_size();
            (size.width, size.height)
        })
    }

    /// The left button came up: a box if the pointer travelled, a click if it did not, a double-click if it landed in the
    /// same place twice in half a second.
    fn release(&mut self) {
        let Some(from) = self.press_at.take() else {
            return;
        };
        // A press that began on the interface is not a box over the world, even if the pointer left it.
        let (frame_width, frame_height) = self.frame_size();
        if ui::over_hud(from, &ui::Layout::of(frame_width, frame_height)) {
            return;
        }
        let to = self.cursor_pixel;
        let travelled = (to.0 - from.0).abs() > 4 || (to.1 - from.1).abs() > 4;
        let Some(world) = self.world.as_ref() else {
            return;
        };
        let snapshot = Snapshot::of(world, &self.config.map, 1);
        let camera = self.camera;
        if travelled {
            self.selection.band(&snapshot, &camera, from, to);
            self.last_click = None;
            self.refresh_card();
            return;
        }
        let twice = self.last_click.is_some_and(|(when, at)| {
            when.elapsed() < std::time::Duration::from_millis(400)
                && (at.0 - to.0).abs() <= 4
                && (at.1 - to.1).abs() <= 4
        });
        if twice {
            self.selection.double_click(&snapshot, &camera, to);
            self.last_click = None;
        } else {
            self.selection.click(&snapshot, &camera, to);
            self.last_click = Some((Instant::now(), to));
        }
        // The card belongs to the selection: a new selection is a new set of buttons.
        self.refresh_card();
    }

    /// Send the selected units an order at a tile: gather from a resource, attack a living thing, otherwise walk. This is
    /// the same `Command` a test submits through `World::submit_checked`, so nothing here can reach a state a test cannot.
    fn order_at(&mut self, tile: (i32, i32), point: (f32, f32)) {
        self.order = Some(point);
        // Pick first, borrow mutably second: picking reads the world, ordering writes it.
        let target = self.pick_entity(tile);
        let target_kind = self
            .world
            .as_ref()
            .and_then(|world| target.and_then(|id| world.entity(id)))
            .map(|entity| entity.kind);
        let Some(world) = self.world.as_mut() else {
            return;
        };
        let selected: Vec<ruage_sim::EntityId> = self
            .selection
            .ids
            .iter()
            .map(|id| ruage_sim::EntityId(*id))
            .collect();
        for id in selected {
            let Some(entity) = world.entity(id) else {
                continue;
            };
            if entity.player != 1 || entity.kind != ruage_sim::EntityKind::Unit {
                continue;
            }
            let command = match (target, target_kind) {
                (Some(node), Some(ruage_sim::EntityKind::Resource(_))) => {
                    ruage_sim::Command::Gather { villager: id, node }
                }
                (Some(foe), Some(_))
                    if world.entity(foe).is_some_and(|e| e.player != 1 && e.hp > 0) =>
                {
                    ruage_sim::Command::Attack {
                        attacker: id,
                        target: foe,
                    }
                }
                _ => {
                    // **The point, not the tile** (A): the engine's movement is free of the grid, so a walk order carries
                    // where the player pointed. A tile would put the unit on a centre it never clicked.
                    let scale = world.scale();
                    #[allow(clippy::cast_possible_truncation)]
                    let raw = |value: f32| (f64::from(value) * scale as f64).round() as i64;
                    ruage_sim::Command::WalkTo {
                        unit: id,
                        to: (
                            ruage_fixed::Fixed::from_raw(raw(point.0)),
                            ruage_fixed::Fixed::from_raw(raw(point.1)),
                        ),
                    }
                }
            };
            // A refused order is shown twice over: in the window's title (for a log) and **in the HUD** (for the player,
            // who is not reading titles). This is how a player learns why nothing happened.
            match world.submit_checked(command) {
                Ok(()) => self.notice = None,
                Err(err) => {
                    if let Some(window) = self.window.as_ref() {
                        window.set_title(&format!("ruage - refused: {err}"));
                    }
                    self.notice = Some((err.to_string(), self.ticks));
                    self.order = None;
                }
            }
        }
    }

    /// How long a refusal stays on the panel: three seconds at the world's own tick rate.
    fn notice_ticks(&self) -> u64 {
        u64::from(
            self.world
                .as_ref()
                .map_or(30, ruage_sim::World::tick_rate_hz),
        ) * 3
    }

    /// Start the match again from tick zero: a new world from the same config and seed, an empty selection, and the menu
    /// closed. The seed is the config's, so "restart" is reproducible and two restarts of one match are the same match.
    fn restart(&mut self) {
        self.card.clear();
        self.card_hover = None;
        self.world = None;
        self.runner = None;
        self.ticks = 0;
        self.accumulated = Duration::ZERO;
        self.selection.clear();
        self.press_at = None;
        self.last_click = None;
        self.notice = None;
        self.order = None;
        self.menu_open = false;
        self.menu_hover = None;
        eprintln!("restart: seed {}", self.seed);
    }

    /// One simulation tick. The world is seeded on the first call, so a failure to seed is a failure to start rather than
    /// a blank window that never explains itself.
    fn step(&mut self) -> Result<(), String> {
        if self.world.is_none() {
            let mut world =
                World::new(&self.config.settings, self.seed).map_err(|err| err.to_string())?;
            world
                .seed_from(&self.config.map, &self.config.economy, &self.config.units)
                .map_err(|err| err.to_string())?;
            world
                .load_resources(&self.config.resources)
                .map_err(|err| err.to_string())?;
            world.load_combat(&self.config.combat);
            world
                .load_research(&self.config.techs, &self.config.effects)
                .map_err(|err| err.to_string())?;
            world
                .load_civs(&self.config.civs)
                .map_err(|err| err.to_string())?;
            world.apply_policy(&self.config.scenario.policy);
            let runner = Runner::new(&self.config.scenario, world.tick_rate_hz())
                .map_err(|err| err.to_string())?;
            self.world = Some(world);
            self.runner = Some(runner);
        }
        let world = self.world.as_mut().ok_or("the world was not seeded")?;
        // Rules before the tick: the scenario's steps and the personality's rules are due against the state the last
        // tick left, exactly as the headless runner applies them.
        if let Some(runner) = self.runner.as_mut() {
            runner.due(world);
        }
        world.step().map_err(|err| err.to_string())?;
        self.ticks += 1;
        Ok(())
    }

    /// Put the current world on the surface. Cheap enough to run every frame: it is one blit of a 960x720 buffer.
    fn draw(&mut self) {
        // Read the HUD's own state **before** the world and the surface are borrowed mutably below.
        let notice_ticks = self.notice_ticks();
        let notice = self
            .notice
            .as_ref()
            .filter(|(_, at)| self.ticks.saturating_sub(*at) < notice_ticks)
            .map(|(text, _)| text.clone());
        let (menu_open, menu_hover) = (self.menu_open, self.menu_hover);
        // **The help line**: what the pointer is over, in words the interface already has - a button's name and price, or
        // why it would be refused. openaok's `SetWorldHint`, and the cheapest teaching device in the interface.
        let help = self.card_hover.and_then(|index| {
            self.card.get(index).map(|(button, _)| {
                let mut line = format!("{} x{}", button.name.to_uppercase(), button.costs.len().max(1));
                if let Some(refusal) = &button.refusal {
                    line = format!("{} - {}", button.name.to_uppercase(), refusal);
                }
                let _ = &mut line;
                line
            })
        });
        let bar_hover = self.bar_hover;
        let (Some(world), Some(surface), Some(window)) = (
            self.world.as_ref(),
            self.surface.as_mut(),
            self.window.as_ref(),
        ) else {
            return;
        };
        let size = window.inner_size();
        let (width, height) = (size.width.max(1), size.height.max(1));
        // The camera is **state**: a resize changes what it looks through, never where it is. Rebuilding it here
        // re-centred it on a fixed tile every frame, which looks harmless until there is a pan or a zoom key - and then
        // neither can ever work (`Camera::resized`, pinned by `resizing_the_window_keeps_the_camera_on_its_tile`).
        self.camera.resized((width, height));
        let snapshot = Snapshot::of(world, &self.config.map, 1);
        // Selection rings, health bars and the last order's marker are `ruage_view`'s work, not the binary's: a marker
        // nobody can test is a marker that is wrong for a month without anyone noticing.
        let selected: Vec<u32> = self.selection.ids.clone();
        let mut frame = render_with(
            &snapshot,
            &self.camera,
            &Highlights {
                selected: &selected,
                order: self.order,
            },
        );
        // The HUD reads the same snapshot the picture was drawn from - stock, population - so what it reports cannot
        // drift from what is on the screen (D74).
        let mut hud = ui::Hud::of_selected(&snapshot, &self.config.resources, &selected);
        // **The pack names what is selected.** The view has no vocabulary of its own (D17): the window asks the
        // simulation, which holds the economy's `[role]` table.
        for row in &mut hud.selected {
            if let Some(unit) = row.unit {
                row.name = world
                    .role_name(unit)
                    .unwrap_or_else(|| format!("unit {unit}"));
            } else {
                row.name = "resource".to_owned();
            }
        }
        // A refusal stays on the panel for three seconds of match time: long enough to read, short enough that it does not
        // become furniture.
        hud.notice = notice;
        hud.help = help;
        hud.bar_hover = bar_hover;
        ui::hud(&mut frame, &hud, &snapshot, self.pace.speed());
        let buttons: Vec<ui::CardButton> =
            self.card.iter().map(|(button, _)| button.clone()).collect();
        if !buttons.is_empty() {
            ui::draw_card(&mut frame, &buttons, width, height, self.card_hover);
        }
        if menu_open {
            ui::draw_menu(&mut frame, width, height, menu_hover);
        }
        // `softbuffer` wants non-zero dimensions; a minimised window has none, and drawing nothing is correct there.
        let (Some(w), Some(h)) = (
            std::num::NonZeroU32::new(width),
            std::num::NonZeroU32::new(height),
        ) else {
            return;
        };
        if surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };
        // `Frame` is RGBA bytes; `softbuffer` wants 0x00RRGGBB words.
        let texels = (width as usize) * (height as usize);
        for index in 0..texels.min(buffer.len()).min(frame.pixels.len() / 4) {
            let p = index * 4;
            buffer[index] = u32::from(frame.pixels[p]) << 16
                | u32::from(frame.pixels[p + 1]) << 8
                | u32::from(frame.pixels[p + 2]);
        }
        // Anything the frame does not cover is cleared: a window bigger than the rasteriser's buffer would otherwise show
        // whatever the previous resize left behind.
        if buffer.len() > texels {
            buffer[texels..].fill(0);
        }
        let _ = buffer.present();
        window.set_title(&format!(
            "ruage - tick {} ({:.1} s) - {} speed{} - {} entities - {} selected{}",
            self.ticks,
            self.ticks as f64 / f64::from(self.config.settings.tick_rate_hz.max(1)),
            self.pace.speed().label(),
            if self.paused { " (paused)" } else { "" },
            world.entities().len(),
            self.selection.ids.len(),
            if self.failed { " - STOPPED" } else { "" }
        ));
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Some platforms resume more than once; a second window would leak the first and leave two surfaces drawing.
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("ruage")
            .with_inner_size(winit::dpi::LogicalSize::new(960.0, 720.0));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => std::rc::Rc::new(window),
            Err(err) => {
                eprintln!("window: {err}");
                event_loop.exit();
                return;
            }
        };
        let context = match softbuffer::Context::new(window.clone()) {
            Ok(context) => context,
            Err(err) => {
                eprintln!("softbuffer context: {err}");
                event_loop.exit();
                return;
            }
        };
        let surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(surface) => surface,
            Err(err) => {
                eprintln!("softbuffer surface: {err}");
                event_loop.exit();
                return;
            }
        };
        self.context = Some(context);
        self.surface = Some(surface);
        self.window = Some(window);
        // The loop never sleeps: the accumulator decides how much work a frame carries, so a slow frame catches up
        // instead of slowing the match down.
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + self.tick));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key {
                    // Escape is the menu, not the end of the match: quitting the process because somebody pressed Escape
                    // is how a player loses a game they were winning.
                    Key::Named(NamedKey::Escape) => {
                        self.menu_open = !self.menu_open;
                        self.menu_hover = None;
                    }
                    // Space freezes the match, not the window: the camera and the picture stay live.
                    Key::Named(NamedKey::Space) => {
                        self.paused = !self.paused;
                        self.accumulated = Duration::ZERO;
                    }
                    // Arrows pan in the direction the **screen** moves, not the grid's: the map is a diamond, so a tile
                    // step would move the picture diagonally (the owner's report, 2026-11-03). A held key autorepeats,
                    // which is the pan.
                    Key::Named(NamedKey::ArrowLeft) => self.camera.scroll_screen((-1.0, 0.0)),
                    Key::Named(NamedKey::ArrowRight) => self.camera.scroll_screen((1.0, 0.0)),
                    Key::Named(NamedKey::ArrowUp) => self.camera.scroll_screen((0.0, -1.0)),
                    Key::Named(NamedKey::ArrowDown) => self.camera.scroll_screen((0.0, 1.0)),
                    // 1..5 are the speed keys, the same five the HUD selector shows. A change is allowed mid-match and
                    // logged here: it is how fast this window watches, not something the match contains.
                    Key::Character(ref key) => {
                        // With the menu open the keys are the menu's: R restarts, Q quits, Escape (above) resumes.
                        if self.menu_open {
                            match key.as_str() {
                                "r" | "R" => self.restart(),
                                "q" | "Q" => event_loop.exit(),
                                _ => {}
                            }
                            return;
                        }
                        if let Some(index) = key
                            .as_str()
                            .parse::<usize>()
                            .ok()
                            .filter(|index| (1..=5).contains(index))
                        {
                            let speed = Speed::ALL[index - 1];
                            self.pace.set(speed);
                            eprintln!("speed -> {}", speed.label());
                        }
                    }
                    _ => {}
                }
            }
            // The wheel is the zoom: three levels, a notch each, and the tile under the cursor stays where it is.
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(_, up) if up > 0.0 => self.camera.zoom_in(),
                MouseScrollDelta::LineDelta(_, down) if down < 0.0 => self.camera.zoom_out(),
                _ => {}
            },
            WindowEvent::CursorMoved { position, .. } => {
                let pixel = (position.x as i32, position.y as i32);
                self.cursor = self.camera.pick(pixel);
                self.cursor_point = self.camera.pick_point(pixel);
                self.cursor_pixel = pixel;
                let (width, height) = self.frame_size();
                let layout = ui::Layout::of(width, height);
                if self.menu_open {
                    self.menu_hover = ui::menu_at(pixel, width, height);
                } else {
                    self.card_hover = ui::card_at(pixel, self.card.len(), width, height);
                    self.bar_hover = ui::bar_button_at(pixel, &layout);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => self.click(button),
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: winit::event::MouseButton::Left,
                ..
            } => self.release(),
            WindowEvent::RedrawRequested => self.draw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.accumulated += now - self.last_frame;
        self.last_frame = now;
        // Catch up at most a second of ticks in one frame: a debugger pause should not fast-forward the match.
        let ceiling = self.tick * 20;
        if self.accumulated > ceiling {
            self.accumulated = ceiling;
        }
        let mut ran = false;
        // **The menu stops the clock** as well as the space bar: a menu the match runs under is a menu nobody can read.
        while self.accumulated >= self.tick && !self.failed && !self.paused && !self.menu_open {
            self.accumulated -= self.tick;
            // Speed is *ticks per interval*, not a shorter interval: the simulation's notion of a tick never changes, which
            // is why a fast window session and a headless run still agree on every tick boundary.
            // `Pace` carries the fraction, so 1.5x runs 1, 2, 1, 2 ticks: the simulation's tick never changes, only how
            // many of them an interval runs.
            let ticks_this_interval_target = self.pace.ticks();
            let mut ticks_this_interval = 0;
            while ticks_this_interval < ticks_this_interval_target && !self.failed {
                ticks_this_interval += 1;
                if let Err(err) = self.step() {
                    eprintln!("step: {err}");
                    self.failed = true;
                    if let Some(window) = self.window.as_ref() {
                        window.set_title(&format!("ruage - stopped: {err}"));
                    }
                }
            }
            ran = true;
        }
        // The next wake-up is when the next tick is due, not "immediately": a poll loop here is a core spent idling.
        _event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + self.tick.saturating_sub(self.accumulated),
        ));
        if let (true, Some(window)) = (ran, self.window.as_ref()) {
            window.request_redraw();
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config = PathBuf::from(args.first().cloned().unwrap_or_else(|| "config".to_owned()));
    let config = match load(&config) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };
    let event_loop = match EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(err) => {
            eprintln!("event loop: {err}");
            return ExitCode::FAILURE;
        }
    };
    let seed: u64 = args
        .iter()
        .position(|arg| arg == "--seed")
        .and_then(|at| args.get(at + 1))
        .and_then(|value| value.parse().ok())
        .unwrap_or(7);
    let mut app = App::new(config, seed);
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("run: {err}");
        return ExitCode::FAILURE;
    }
    println!("ticks={}", app.ticks);
    ExitCode::SUCCESS
}
