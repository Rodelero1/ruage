//! Render one frame of a match to a PPM file.
//!
//! ```text
//! cargo run -p ruage-view --bin frame -- <config-dir> <ticks> <out.ppm> [player]
//! ```
//!
//! It is a diagnostic rather than a viewer: no window, no input, no clock - the point is that a frame is a
//! pure function of a world, so it can be produced, compared and looked at without a display.

use std::path::PathBuf;
use std::process::ExitCode;

use ruage_data::{Economy, Map, Units};
use ruage_sim::{Command, EntityKind, World};
use ruage_view::ui;
use ruage_view::{Camera, Highlights, Snapshot, render_with};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        eprintln!("usage: frame <config-dir> <ticks> <out.ppm> [player]");
        return ExitCode::from(2);
    }
    let config = PathBuf::from(&args[0]);
    let ticks: u32 = match args[1].parse() {
        Ok(ticks) => ticks,
        Err(err) => {
            eprintln!("ticks: {err}");
            return ExitCode::from(2);
        }
    };
    let out = PathBuf::from(&args[2]);
    let player: u8 = args.get(3).and_then(|p| p.parse().ok()).unwrap_or(1);

    let resources = match ruage_data::load_resources_file(&config.join("resources.toml")) {
        Ok(resources) => resources,
        Err(err) => {
            eprintln!("resources.toml: {err}");
            return ExitCode::FAILURE;
        }
    };
    let load = || -> Result<(ruage_data::Sim, Map, Economy, Units, ruage_data::Combat), String> {
        Ok((
            ruage_data::load_sim_file(&config.join("sim.toml")).map_err(|e| e.to_string())?,
            ruage_data::load_map_file(&config.join("map.toml"), &resources)
                .map_err(|e| e.to_string())?,
            ruage_data::load_economy_file(&config.join("economy.toml"))
                .map_err(|e| e.to_string())?,
            ruage_data::load_units_file(&config.join("gamedata/units.toml"))
                .map_err(|e| e.to_string())?,
            ruage_data::load_combat_file(&config.join("combat.toml")).map_err(|e| e.to_string())?,
        ))
    };
    let (settings, map, economy, units, combat) = match load() {
        Ok(parts) => parts,
        Err(err) => {
            eprintln!("config: {err}");
            return ExitCode::FAILURE;
        }
    };

    let mut world = match World::new(&settings, 7) {
        Ok(world) => world,
        Err(err) => {
            eprintln!("world: {err}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(err) = world.seed_from(&map, &economy, &units) {
        eprintln!("seeding: {err}");
        return ExitCode::FAILURE;
    }
    world.load_resources(&resources).ok();
    world.load_combat(&combat);
    // **The same tables the window loads.** A diagnostic that plays a different game from the window - no techs, so no
    // research on the card, no rules, so a village that never builds - is a picture that can look right while the window
    // is wrong.
    let techs = match ruage_data::load_techs_file(&config.join("gamedata/techs.toml")) {
        Ok(techs) => techs,
        Err(err) => {
            eprintln!("techs.toml: {err}");
            return ExitCode::FAILURE;
        }
    };
    let effects = match ruage_data::load_effects_file(&config.join("gamedata/effects.toml")) {
        Ok(effects) => effects,
        Err(err) => {
            eprintln!("effects.toml: {err}");
            return ExitCode::FAILURE;
        }
    };
    let civs = match ruage_data::load_civs_file(&config.join("gamedata/civs.toml")) {
        Ok(civs) => civs,
        Err(err) => {
            eprintln!("civs.toml: {err}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(err) = world.load_research(&techs, &effects) {
        eprintln!("research: {err}");
        return ExitCode::FAILURE;
    }
    if let Err(err) = world.load_civs(&civs) {
        eprintln!("civs: {err}");
        return ExitCode::FAILURE;
    }
    if let Ok(personality) = ruage_sim::load_personality_file(&config.join("ai/base.toml")) {
        world.apply_policy(&personality.policy);
    }

    // Something to look at: the first villager of the player under test walks out, so the frame is not a
    // still life of a town centre.
    if let Some(villager) = world
        .entities()
        .iter()
        .find(|entity| entity.player == player && entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
    {
        // `job`: send him to work the nearest tree, so the panel can be looked at **while he works** and not only while he
        // walks - the job name is the thing this diagnostic exists to show (D121).
        let tree = if args.get(4).map(String::as_str) == Some("job") {
            let at = world.position(villager).expect("the villager");
            let wood = resources.slot_of("wood");
            world
                .entities()
                .iter()
                .filter(|entity| matches!(entity.kind, EntityKind::Resource(slot) if Some(slot) == wood))
                .min_by_key(|entity| {
                    let dx = entity.position.0.raw() - at.0.raw();
                    let dy = entity.position.1.raw() - at.1.raw();
                    (dx / 1000) * (dx / 1000) + (dy / 1000) * (dy / 1000)
                })
                .map(|entity| entity.id)
        } else {
            None
        };
        match tree {
            Some(node) => {
                world.submit(Command::Gather { villager, node });
            }
            None => {
                world.submit(Command::Move {
                    unit: villager,
                    to: (24, 20),
                });
            }
        }
    }
    for _ in 0..ticks {
        if let Err(err) = world.step() {
            eprintln!("tick: {err}");
            return ExitCode::FAILURE;
        }
    }

    let snapshot = Snapshot::of(&world, &map, player);
    // Centre on the player's own base rather than on a fixed corner: with a 4x4 town centre the base fills most of
    // the frame, and a camera pointed at empty ground proves nothing about what was drawn.
    // Centre on the selected unit when there is one, so the ring and the bar are in the picture; otherwise on the
    // player's base.
    let selected_unit = snapshot
        .entities
        .iter()
        .find(|entity| entity.player == player && entity.kind == EntityKind::Unit)
        .map(|entity| entity.position);
    let focus = selected_unit.or_else(|| {
        snapshot
            .entities
            .iter()
            .find(|entity| entity.player == player && entity.kind == EntityKind::Building)
            .map(|entity| entity.position)
    });
    let focus = focus.unwrap_or((20.0, 20.0));
    // The frame's size: `WIDTHxHEIGHT` as the sixth argument, so a widescreen frame can be rendered and looked at -
    // "it works on a wide screen" has to be something a person can see, not something the code intends.
    let (width, height) = args
        .get(5)
        .and_then(|size| size.split_once('x'))
        .and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)))
        .unwrap_or((640, 480));
    let camera = Camera::new((width, height)).centred_on(focus);
    // What the diagnostic selects: the player's first unit (ring, health bar), the first building with `card` as the fifth
    // argument, so the command card can be looked at without a display, or **a resource node** with `node` - the panel's
    // hit points and holdings lines are a picture, and a picture has to be looked at (`SELECTING A NODE`, 2026-11-04).
    let mode = args.get(4).map(String::as_str);
    let selected: Vec<u32> = match mode {
        Some("node") => snapshot
            .entities
            .iter()
            .filter(|entity| matches!(entity.kind, EntityKind::Resource(_)))
            // The tree nearest the camera's focus, so the node in the panel is the node in the picture.
            .min_by(|a, b| {
                let distance = |entity: &&ruage_view::EntityView| {
                    let (dx, dy) = (entity.position.0 - focus.0, entity.position.1 - focus.1);
                    dx * dx + dy * dy
                };
                distance(a)
                    .partial_cmp(&distance(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|entity| entity.id)
            .into_iter()
            .collect(),
        _ => {
            let kind = if mode == Some("card") {
                EntityKind::Building
            } else {
                EntityKind::Unit
            };
            snapshot
                .entities
                .iter()
                .filter(|entity| entity.player == player && entity.kind == kind)
                .map(|entity| entity.id)
                .take(1)
                .collect()
        }
    };
    let mut frame = render_with(
        &snapshot,
        &camera,
        &Highlights {
            selected: &selected,
            order: None,
        },
    );
    // The HUD goes in the diagnostic picture too: a step that draws numbers has to be looked at, not only asserted on.
    // It is the whole HUD now - the panel, the minimap and any banner - because a diagnostic that draws less than the
    // window is a diagnostic that can pass while the window is wrong.
    let mut hud = ui::Hud::of_selected(&snapshot, &resources, &selected);
    // **The pack names what is selected**, exactly as the window does: a diagnostic that shows an empty name is a
    // diagnostic that hides the bug it was built to show.
    let jobs = ruage_data::load_jobs_file(&config.join("jobs.toml")).unwrap_or_default();
    for row in &mut hud.selected {
        // **The job first** (D121), exactly as the window does it, so the picture cannot disagree with the game.
        let job = world
            .work_target(ruage_sim::EntityId(row.id))
            .and_then(|target| {
                let column = target.column.and_then(|slot| resources.name(slot));
                jobs.name_for(target.role.as_deref(), column)
                    .map(str::to_owned)
            });
        let role = row.unit.and_then(|unit| world.role_name(unit));
        let fallback = if row.unit.is_some() {
            format!("unit {}", row.unit.unwrap_or(0))
        } else {
            "resource".to_owned()
        };
        row.name = ruage_view::label_for(job, role, fallback);
        // A picture cannot be read back here, so the diagnostic says out loud what it called the thing it drew.
        eprintln!("selected {} is \"{}\"", row.id, row.name);
    }
    hud.view = Some(camera.visible_corners());
    ui::hud(&mut frame, &hud, &snapshot, ruage_view::Speed::One);
    // `... ppm 1 menu`: the same picture with the Escape menu over it, so the modal can be looked at without a display.
    // The command card, when what is selected is one of the player's buildings: the diagnostic shows what the window shows.
    if let Some(id) = selected.first().copied() {
        let card = world.card(player, ruage_sim::EntityId(id));
        if !card.is_empty() {
            let buttons: Vec<ui::CardButton> = card
                .into_iter()
                .map(|item| ui::CardButton {
                    name: item.name,
                    costs: item.costs,
                    refusal: item.refusal,
                })
                .collect();
            let (width, height) = (frame.width, frame.height);
            ui::draw_card(&mut frame, &buttons, width, height, None);
        }
    }
    if args.get(4).is_some_and(|arg| arg == "menu") {
        let (width, height) = (frame.width, frame.height);
        ui::draw_menu(&mut frame, width, height, Some(0));
    }
    if let Err(err) = std::fs::write(&out, frame.to_ppm()) {
        eprintln!("write {}: {err}", out.display());
        return ExitCode::FAILURE;
    }
    println!(
        "tick={} entities={} visible={} -> {}",
        snapshot.tick,
        snapshot.entities.len(),
        snapshot.visible.iter().map(|w| w.count_ones()).sum::<u32>(),
        out.display()
    );
    ExitCode::SUCCESS
}
