//! **A walk ends at contact with its target only when the target is the only thing it touches.**
//!
//! The original's collision block classifies every entry of the query at the new position (terrain, path-ignore, target,
//! non-target) and then has exactly one "arrival" arm, gated on the **list**:
//!
//! ```c
//! if ((DAT_00de1998 != 1) || (local_dc != 1)) { ... repath ... }   // count != 1, or the one entry is not the target
//! else "That collision is with our target, so we're done"          // move_obj.cpp:0x127d
//! ```
//!
//! so *"That collision is with our target, so we're done"* fires only when the collision list is **exactly one entry and it is
//! the target**. With a second body in the way the mover **re-paths** instead (`"We have %d non-past collisions and %d terrain
//! collisions, repathing"`, 0x128b).
//!
//! Ours takes the contact cut first and breaks out of the step before the body test is ever asked, so a man whose last step
//! touches his target lands on whatever friend is already standing at the contact point. That is the state the owner reported
//! on 2026-11-22 - *"villagers stack on each other a lot (like 5 in the same spot)"* - and `pile_probe` names it: the
//! offenders are men whose last step ends on a node's contact point with another gatherer of the same node 0.36 of a tile
//! away against a 0.40 reach.
//!
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// Open grass with the two town centres in opposite corners, so the only body in the lane is the one the fixture puts there.
fn open_map() -> String {
    let mut text = String::from("schema = 1\n[map]\nwidth = 40\nheight = 26\n");
    text.push_str("terrain = [\n");
    for _ in 0..26 {
        text.push_str("  \"");
        text.push_str(&"G".repeat(40));
        text.push_str("\",\n");
    }
    text.push_str("]\n[[map.town_center]]\nplayer = 1\nx = 1\ny = 1\n[[map.town_center]]\nplayer = 2\nx = 38\ny = 24\n");
    text
}

fn world_of(text: &str) -> World {
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(text, "contact.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must(
        "load_units_file",
        ruage_data::load_units_file(&repo("config/gamedata/units.toml")),
    );
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    world
}

fn spawn(world: &mut World, id: u32, at: (i64, i64)) -> EntityId {
    let villager = world.role_unit("villager").expect("a villager role");
    world
        .submit_checked(Command::Spawn {
            id: EntityId(id),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (Fixed::from_raw(at.0), Fixed::from_raw(at.1)),
            speed: Fixed::parse("0.8", world.scale()).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    EntityId(id)
}

/// **Two men sent at one target from the same side do not end in one place.** The first to arrive stands at the contact point;
/// the second's own last step touches the same target, and must not be allowed to end there as well - the game re-paths him
/// round the body, so he comes to rest beside his friend instead of inside him.
#[test]
fn a_walk_ends_at_contact_only_when_the_target_is_all_it_touches() {
    let scale = 20_000i64;
    let centre = |x: i64, y: i64| (x * scale + scale / 2, y * scale + scale / 2);
    let mut world = world_of(&open_map());
    let target = spawn(&mut world, 1_001, centre(20, 12));
    world.step().expect("the target lands");
    let first = spawn(&mut world, 1_002, centre(13, 12));
    let second = spawn(&mut world, 1_003, centre(12, 12));
    world.step().expect("both walkers land");
    for _ in 0..5 {
        world.step().expect("a tick");
    }

    let aim = world.position(target).expect("the target stands");
    for walker in [first, second] {
        world
            .submit_checked(Command::WalkTo {
                unit: walker,
                to: aim,
                target: Some(target),
            })
            .expect("a walk at the target is ordered");
    }
    for tick in 0..800u32 {
        world.step().expect("a tick");
        let (p, q) = (
            world.position(first).unwrap(),
            world.position(second).unwrap(),
        );
        if tick % 25 == 0 || (q.0.raw() - p.0.raw()).abs() < 100 {
            eprintln!(
                "t={tick} first=({},{}) last_step={:?} second=({},{}) last_step={:?} task={:?}",
                p.0.raw(),
                p.1.raw(),
                world.entity(first).unwrap().last_step,
                q.0.raw(),
                q.1.raw(),
                world.entity(second).unwrap().last_step,
                world.entity(second).unwrap().task
            );
        }
    }

    let (a, b) = (
        world.position(first).expect("the first stands"),
        world.position(second).expect("the second stands"),
    );
    let reach = world.reach_between(first, second).raw();
    let apart = ((a.0.raw() - b.0.raw()).pow(2) + (a.1.raw() - b.1.raw()).pow(2)).isqrt();
    assert!(
        apart >= reach,
        "MEASURED the two men {apart} raw apart against the {reach} their footprints need, at ({}, {}) and ({}, {}) - the \
         second's final step touched the target and was taken without asking whether his friend was there",
        a.0.raw() as f64 / scale as f64,
        a.1.raw() as f64 / scale as f64,
        b.0.raw() as f64 / scale as f64,
        b.1.raw() as f64 / scale as f64
    );
    // And both still have to be *at* the target: a fix that keeps them apart by refusing to walk is not the rule.
    for walker in [first, second] {
        let at = world.position(walker).expect("a walker stands");
        let gap = ((at.0.raw() - aim.0.raw()).abs()).max((at.1.raw() - aim.1.raw()).abs());
        assert!(
            gap <= scale,
            "MEASURED a walker {gap} raw from the target, more than a tile: he did not arrive at all"
        );
    }
}
