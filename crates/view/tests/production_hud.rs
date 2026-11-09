//! **The production bar on the panel** (D157): a building's queue, drawn with its progress.
//!
//! The owner: *"When units are trained in a building, there should be a training queue with visible progress. Same for tech
//! research."* The engine counts the time (`World::production`); this proves the interface shows it, and that the picture cannot
//! disagree with the queue.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_sim::{Command, EntityId, EntityKind, QueuedItem, World};
use ruage_view::ui::{self, Hud};
use ruage_view::{Camera, Frame, Snapshot, palette, render};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    units: Units,
    resources: ruage_data::Resources,
    techs: ruage_data::Techs,
    effects: ruage_data::Effects,
    civs: ruage_data::Civs,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy"),
        units: ruage_data::load_units_file(&units).expect("units"),
        resources,
        techs: ruage_data::load_techs_file(&repo("config/gamedata/techs.toml")).expect("techs"),
        effects: ruage_data::load_effects_file(&repo("config/gamedata/effects.toml"))
            .expect("effects"),
        civs: ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs"),
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world.load_resources(&fx.resources).expect("stockpile");
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("research tables");
    world.load_civs(&fx.civs).expect("civs");
    world
}

fn town_centre(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == 1
                && entity.kind == EntityKind::Building
                && entity.construction.is_none()
        })
        .map(|entity| entity.id)
        .expect("player 1's town centre")
}

/// **The panel's numbers are the queue's numbers**, after ten seconds of a twenty-five second unit.
#[test]
fn the_hud_reports_progress_and_the_waiting_count() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    let villager = world.role_unit("villager").expect("villager");
    let loom = 22u32;
    world
        .submit_checked(Command::Train {
            building: centre,
            unit: villager,
        })
        .expect("a villager");
    world
        .submit_checked(Command::Research {
            building: centre,
            tech: loom,
        })
        .expect("a tech behind it");
    // Ten of the villager's twenty-five seconds.
    for _ in 0..world.tick_rate_hz() * 10 {
        world.step().expect("step");
    }

    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let hud = Hud::of_selected(&snapshot, &fx.resources, &[centre.0]);
    let row = hud.selected.first().expect("the town centre is selected");
    let (item, done, total) = row.production.running.expect("a job is running");
    assert_eq!(
        item,
        QueuedItem::Unit(villager),
        "the running job is the unit"
    );
    assert_eq!(total, 25 * world.tick_rate_hz(), "the unit's own time");
    assert!(
        done >= 8 * world.tick_rate_hz() && done <= 12 * world.tick_rate_hz(),
        "ten seconds in: {done} of {total}"
    );
    assert_eq!(
        row.production.waiting,
        vec![QueuedItem::Tech(loom)],
        "the tech waits behind it"
    );
}

/// **The bar is drawn, and it fills**: the first part of the track is the progress colour and the rest is the track's.
#[test]
fn the_bar_fills_from_the_left_and_leaves_the_rest_empty() {
    let production = ruage_sim::Production {
        running: Some((QueuedItem::Unit(83), 25, 100)),
        waiting: vec![QueuedItem::Tech(22)],
    };
    let mut frame = Frame::new(200, 40);
    ui::production_bar(&mut frame, (10, 10), 100, &production);
    let filled = frame.pixel(20, 11);
    assert_eq!(
        filled,
        palette::PROGRESS,
        "a quarter of the way in is filled"
    );
    let empty = frame.pixel(90, 11);
    assert_eq!(empty, palette::HP_TRACK, "the rest of the track is empty");
    // A building doing nothing draws an empty track and nothing else.
    let mut idle = Frame::new(200, 40);
    ui::production_bar(&mut idle, (10, 10), 100, &ruage_sim::Production::default());
    assert_eq!(idle.pixel(20, 11), palette::HP_TRACK, "nothing to show");
}

/// **And it reaches the screen**: the card of a building that is training has the bar in its panel zone.
#[test]
fn a_training_building_shows_the_bar_in_its_card() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    let villager = world.role_unit("villager").expect("villager");
    world
        .submit_checked(Command::Train {
            building: centre,
            unit: villager,
        })
        .expect("a villager");
    for _ in 0..world.tick_rate_hz() * 12 {
        world.step().expect("step");
    }
    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let hud = Hud::of_selected(&snapshot, &fx.resources, &[centre.0]);
    let camera = Camera::new((1280, 720)).centred_on((16.0, 16.0));
    let mut frame = render(&snapshot, &camera);
    // Draw the panel the window draws; the bar is inside the selected card.
    ui::hud(&mut frame, &hud, &snapshot, ruage_view::Speed::One);
    // The progress colour appears somewhere in the panel's selected zone.
    let layout = ui::Layout::of(1280, 720);
    let (x0, y0, w, h) = layout.selected;
    let mut found = false;
    for y in y0..(y0 + h as i32) {
        for x in x0..(x0 + w as i32) {
            if frame.pixel(x as u32, y as u32) == palette::PROGRESS {
                found = true;
                break;
            }
        }
        if found {
            break;
        }
    }
    assert!(found, "the production bar is in the selected zone");
}
