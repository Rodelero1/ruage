//! **Two bodies collide as boxes, not as discs** (P16, sequence step 4).
//!
//! The binary's query is four `<=` on the grown axis-aligned boxes (`FUN_00840830`: `_DAT_00ddeef4` is the mover's
//! `max_x + inset`, compared against the candidate's own `+0x20 + idx*4` entries), and the first-party talk says a unit is
//! shrunk to a point and the others grown - a box. Ours refused a landing whose **centre** was inside the pair's reach, a
//! disc, which is strictly smaller than the box of the same half-extent: measured over an acceptance match, **4,741 of
//! 218,920** body-steps (2.17%) ended with both boxes overlapping and the discs clear (`collision_probe`).
//!
//! The witness is the pocket the two shapes disagree on: two axes closer than the pair's reach and a distance beyond it.
//! A villager spawned there is already inside the box and outside the disc, so the first step he takes - one that stays in
//! the pocket - is the step the disc allows and the box refuses.
//!
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

const SCALE: i64 = 20_000;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

const MAP: &str = "schema = 1\n[map]\nwidth = 24\nheight = 24\nterrain = [\n\
  \"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\
  \"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\
  \"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\
  \"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\
  \"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\
  \"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\
  \"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\
  \"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\"GGGGGGGGGGGGGGGGGGGGGGGG\",\
]\n[[map.town_center]]\nplayer = 1\nx = 1\ny = 1\n[[map.town_center]]\nplayer = 2\nx = 22\ny = 22\n";

fn world_of() -> World {
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(MAP, "body_boxes.toml", &resources),
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

/// A villager at a raw position: the pockets this fixture needs are between tile lines, so a tile helper would not do.
fn a_villager(world: &mut World, id: u32, at: (i64, i64)) -> EntityId {
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

/// **A step into the pocket is refused.** `reach` is two radii (0.400); the stander is placed 0.350 from the mover on both
/// axes - inside the box, 0.495 away, outside the disc - and the mover's next step stays inside both.
///
/// **The stander is spawned after the mover's plan on purpose**: a collider that has not moved since the route was planned is
/// *past* (D281/D320) and exempt from the step's body test, so a stander placed first would never reach it at all - the first
/// version of this fixture did exactly that and passed without the change.
#[test]
fn a_step_that_lands_in_the_box_pocket_is_refused() {
    let mut world = world_of();
    let mover = a_villager(
        &mut world,
        801,
        (
            10 * SCALE + SCALE / 2 + 7_000,
            10 * SCALE + SCALE / 2 + 7_000,
        ),
    );
    world.step().expect("the mover lands");
    world
        .submit_checked(Command::Move {
            unit: mover,
            to: (13, 13),
            target: None,
        })
        .expect("a walk");
    world.step().expect("the route is planned");
    assert!(world.path_length(mover) > 0, "the mover walks a route");

    // 0.350 on both axes from where he stands now: inside the box, outside the disc.
    let here = world.position(mover).expect("the mover");
    let stander = a_villager(
        &mut world,
        800,
        (here.0.raw() + 7_000, here.1.raw() + 7_000),
    );

    // **The tick under test** is the one that both lands the stander and takes the mover's next step, so the re-path
    // baseline is taken immediately before it: `repaths()` is cumulative and a bump on any earlier tick would make the
    // assertion pass for the wrong reason.
    let before = world.position(mover).expect("the mover");
    let repaths_before = world.repaths();
    world
        .step()
        .expect("the stander lands and the mover takes its step");

    assert_eq!(world.path_length(stander), 0, "the stander has no route");
    let apart = world.position(stander).expect("the stander");
    let reach = world.reach_between(mover, stander).raw();
    let (dx, dy) = (
        (before.0.raw() - apart.0.raw()).abs(),
        (before.1.raw() - apart.1.raw()).abs(),
    );
    assert!(
        dx < reach && dy < reach,
        "the fixture must stand inside the box ({dx}, {dy} of {reach})"
    );
    assert!(
        dx * dx + dy * dy >= reach * reach,
        "and outside the disc, or there is no pocket to test ({dx}, {dy} of {reach})"
    );

    // **The observable: this tick counted a collision.** The collider joins the plan's avoid list and the route is re-made -
    // `repaths` rises by exactly one on the tick whose landing sits in the pocket. Under the disc the same landing is taken
    // silently (`repaths` unchanged), which is the failure this fixture was written to catch.
    assert_eq!(
        world.repaths(),
        repaths_before + 1,
        "a landing whose boxes overlap is a collision in the original, so this tick must re-plan once; the disc counted none"
    );
}
