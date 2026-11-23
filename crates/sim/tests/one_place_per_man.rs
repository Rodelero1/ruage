//! **The place a worker is sent to is not a place another body already holds.**
//!
//! The decomp's only "the spot you were given is taken" rule is `func_002699A0` (`unit_action_resolve.cpp`): step out by
//! `ext_self + ext_other + 0.01`, on one of four sides with an offset across that side, up to fifty tries, and **refuse any
//! candidate whose obstruction query is non-zero** - a body standing there. `func_002697C0` runs it for a group's children and
//! every accepted point joins the list the next child is separated against.
//!
//! Ours picks a gatherer's place from the eight tiles round the node in a fixed ring order and asks only whether the ground is
//! free (`work_position` -> `is_free_ground`, which deliberately ignores units so that a *building* may be placed beside them).
//! So every man sent to one node is sent to **the same tile centre**, walks the same last leg, and comes to rest on the same
//! contact point: measured on the acceptance scene, pairs of gatherers of one node standing **0.28 of a tile** apart against the
//! **0.40** their footprints need, and a deepest group of **11 bodies** (`collision_probe`).
//!
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// One wood tile at `(10,10)` on open grass, so the only reason two men cannot share a place is each other.
fn map_text() -> String {
    let mut text = String::from("schema = 1\n[map]\nwidth = 22\nheight = 22\n");
    text.push_str("terrain = [\n");
    for y in 0..22 {
        let mut row = String::new();
        for x in 0..22 {
            row.push(if y == 10 && x == 10 { 'F' } else { 'G' });
        }
        text.push_str(&format!("  \"{row}\",\n"));
    }
    text.push_str(
        "]\n[[map.resource]]\nkind = \"wood\"\namount = 50000\nhp = 0\ntiles = [[10,10]]\n",
    );
    text.push_str("[[map.town_center]]\nplayer = 1\nx = 1\ny = 1\n[[map.town_center]]\nplayer = 2\nx = 20\ny = 20\n");
    text
}

fn world_of(text: &str) -> Option<World> {
    if !repo("config/gamedata/units.toml").exists() {
        eprintln!("skipping: no units.toml (generated, D8)");
        return None;
    }
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(text, "one_place.toml", &resources),
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
    Some(world)
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

/// **Two men ordered to one tree, from the same side, do not end in one place.** The first is sent and walks; the second is
/// ordered a few ticks later, while the first is still on his way. His place must be one the first does not already hold - the
/// decomp's separation rule - so the pair end at least their own footprints apart.
#[test]
fn two_gatherers_of_one_node_are_given_two_places() {
    let scale = 20_000i64;
    let Some(mut world) = world_of(&map_text()) else {
        return;
    };
    let centre = |x: i64, y: i64| (x * scale + scale / 2, y * scale + scale / 2);
    let tree = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("the map's tree");

    let first = spawn(&mut world, 1_001, centre(7, 10));
    world.step().expect("he lands");
    world
        .submit_checked(Command::Gather {
            villager: first,
            node: tree,
        })
        .expect("the first gather order");
    for _ in 0..8 {
        world.step().expect("a tick");
    }
    let second = spawn(&mut world, 1_002, centre(7, 9));
    world.step().expect("he lands");
    world
        .submit_checked(Command::Gather {
            villager: second,
            node: tree,
        })
        .expect("the second gather order");
    // **While both are working the node**, their closest approach is the measurement: a man on his way home is not a man
    // standing on his friend.
    let reach = world.reach_between(first, second).raw();
    let mut closest = i64::MAX;
    let mut working = 0u32;
    for _ in 0..900 {
        world.step().expect("a tick");
        // **Taking** from the node, not merely ordered at it: the order's task is `Gather` from the click, so a pair still
        // walking in would be counted as working.
        let gathering = |id: EntityId| {
            world.entity(id).is_some_and(
                |entity| matches!(entity.task, Task::Gather { progress, .. } if progress.raw() > 0),
            )
        };
        if !(gathering(first) && gathering(second)) {
            continue;
        }
        working += 1;
        let (a, b) = (
            world.position(first).expect("the first"),
            world.position(second).expect("the second"),
        );
        let apart = ((a.0.raw() - b.0.raw()).pow(2) + (a.1.raw() - b.1.raw()).pow(2)).isqrt();
        closest = closest.min(apart);
    }
    assert!(
        working > 40,
        "the fixture measured nothing: both men were at the node together for {working} tick(s)"
    );
    assert!(
        closest >= reach,
        "MEASURED the two gatherers {closest} raw apart at their closest, against the {reach} their footprints need: a place \
         another body holds is not a place to send a man"
    );
}
