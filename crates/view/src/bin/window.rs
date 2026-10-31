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
use ruage_sim::World;
use ruage_view::ui;
use ruage_view::{Camera, Highlights, Snapshot, render_with};
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
        resources,
    })
}

/// The whole application: one world, one window, one accumulator.
struct App {
    config: Config,
    world: Option<World>,
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
    /// What the player has selected. Orders go to these and nothing else, exactly as a test would name them.
    selected: Vec<ruage_sim::EntityId>,
    /// Paused freezes the accumulator rather than the loop: the window keeps drawing, the match stops.
    paused: bool,
    /// Simulation ticks per tick interval: 1, 2, 4 or 8 (the speed keys).
    speed: u32,
    /// The tile under the pointer, in tile space.
    cursor: (i32, i32),
}

impl App {
    fn new(config: Config, seed: u64) -> App {
        let tick_hz = config.settings.tick_rate_hz.max(1);
        App {
            config,
            world: None,
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
            selected: Vec::new(),
            paused: false,
            speed: 1,
            cursor: (0, 0),
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
                self.selected = self.pick_entity(self.cursor).into_iter().collect();
            }
            winit::event::MouseButton::Right => {
                let tile = self.cursor;
                self.order_at(tile);
            }
            _ => {}
        }
    }

    /// Send the selected units an order at a tile: gather from a resource, attack a living thing, otherwise walk. This is
    /// the same `Command` a test submits through `World::submit_checked`, so nothing here can reach a state a test cannot.
    fn order_at(&mut self, tile: (i32, i32)) {
        self.order = Some((tile.0 as f32, tile.1 as f32));
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
        let selected = self.selected.clone();
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
                    #[allow(clippy::cast_possible_truncation)]
                    let (x, y) = (
                        tile.0.clamp(0, i32::from(u16::MAX)) as u16,
                        tile.1.clamp(0, i32::from(u16::MAX)) as u16,
                    );
                    ruage_sim::Command::Move {
                        unit: id,
                        to: (x, y),
                    }
                }
            };
            // A refused order is shown, not swallowed: this is how a player learns why nothing happened.
            if let (Err(err), Some(window)) = (world.submit_checked(command), self.window.as_ref())
            {
                window.set_title(&format!("ruage - refused: {err}"));
            }
        }
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
            world.apply_content_policy();
            self.world = Some(world);
        }
        let world = self.world.as_mut().ok_or("the world was not seeded")?;
        world.step().map_err(|err| err.to_string())?;
        self.ticks += 1;
        Ok(())
    }

    /// Put the current world on the surface. Cheap enough to run every frame: it is one blit of a 960x720 buffer.
    fn draw(&mut self) {
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
        let selected: Vec<u32> = self.selected.iter().map(|id| id.0).collect();
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
        ui::hud(&mut frame, &snapshot);
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
            "ruage - tick {} ({:.1} s) - {}x speed{} - {} entities - {} selected{}",
            self.ticks,
            self.ticks as f64 / f64::from(self.config.settings.tick_rate_hz.max(1)),
            self.speed,
            if self.paused { " (paused)" } else { "" },
            world.entities().len(),
            self.selected.len(),
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
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    // Space freezes the match, not the window: the camera and the picture stay live.
                    Key::Named(NamedKey::Space) => {
                        self.paused = !self.paused;
                        self.accumulated = Duration::ZERO;
                    }
                    // Arrows pan the view by a tile a press, in the direction the *screen* moves: left takes the
                    // camera towards -x, which is up-left in the diamond. A held key autorepeats, which is the pan.
                    Key::Named(NamedKey::ArrowLeft) => self.camera.scroll((-1.0, 0.0)),
                    Key::Named(NamedKey::ArrowRight) => self.camera.scroll((1.0, 0.0)),
                    Key::Named(NamedKey::ArrowUp) => self.camera.scroll((0.0, -1.0)),
                    Key::Named(NamedKey::ArrowDown) => self.camera.scroll((0.0, 1.0)),
                    // 1..4 are the speed keys; the loop runs one, two, four or eight ticks per tick interval.
                    Key::Character(ref key) => match key.as_str() {
                        "1" => self.speed = 1,
                        "2" => self.speed = 2,
                        "3" => self.speed = 4,
                        "4" => self.speed = 8,
                        _ => {}
                    },
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
                self.cursor = self.camera.pick((position.x as i32, position.y as i32));
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => self.click(button),
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
        while self.accumulated >= self.tick && !self.failed && !self.paused {
            self.accumulated -= self.tick;
            // Speed is *ticks per interval*, not a shorter interval: the simulation's notion of a tick never changes, which
            // is why a fast window session and a headless run still agree on every tick boundary.
            let mut ticks_this_interval = 0;
            while ticks_this_interval < self.speed && !self.failed {
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
