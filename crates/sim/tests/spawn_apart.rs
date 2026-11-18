//! **Where a body goes when the spot it was given is already taken** (D267), from the original's own resolution pass.
//!
//! The game resolves this when it *places* a body, not while things walk. `func_002699A0`
//! (`aoe2-decomp/src/unit/unit_action_resolve.cpp`, 80 lines) takes the two bodies' extents and steps out by
//! **`ext_a + ext_b + 0.01`** - the sum of the radii plus one hundredth of a tile, so the pair **touches and does not
//! overlap** - on one of **four sides** (`coord +/- step` on an axis, with a **random offset along that side**), trying up
//! to **50** times and refusing a spot that is off the map, in a terrain class the order does not accept, or obstructed
//! (a real query at the candidate point). The caller writes the position and adds it to the coordinate list, so each
//! accepted body becomes an obstruction for the next one and a group on one point fans out from the first man outwards.
//!
//! Our engine has never had that pass. Measured over 36,000 ticks of a real match: **103,408 pair-ticks** were bodies the
//! map had seeded inside each other, with `made_room=0` - nothing is ever pushed apart. (The **978** that movement created
//! are the "past" and "mate" exemptions, which the original grants too.)

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
    combat: ruage_data::Combat,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        resources,
        units: ruage_data::load_units_file(&units).expect("units.toml"),
        combat: ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat.toml"),
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    world
}

/// A body put down **exactly** where another one stands: same raw point, same tick, no movement involved.
fn place_on(world: &mut World, player: u8, id: u32, unit: u32, at: (i64, i64)) -> EntityId {
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (Fixed::from_raw(at.0), Fixed::from_raw(at.1)),
            speed: Fixed::from_raw(world.scale() / 2),
            hp: 40,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("the spawn lands on a tick");
    id
}

fn apart(world: &World, one: EntityId, two: EntityId) -> (i64, i64) {
    let (a, b) = (
        world.position(one).expect("a position"),
        world.position(two).expect("a position"),
    );
    let (dx, dy) = (a.0.raw() - b.0.raw(), a.1.raw() - b.1.raw());
    (
        (dx * dx + dy * dy).isqrt(),
        world.reach_between(one, two).raw(),
    )
}

/// **The rule**: two bodies may touch and may not overlap, and the one placed second is the one that moves.
#[test]
fn a_body_put_on_top_of_a_friend_is_set_beside_him() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let villager = fx.economy.role["villager"];
    let scale = world.scale();
    let spot = (24 * scale, 24 * scale);

    let first = place_on(&mut world, 1, 9_800, villager, spot);
    let second = place_on(&mut world, 1, 9_801, villager, spot);

    let (distance, reach) = apart(&world, first, second);
    assert!(
        distance >= reach,
        "two bodies put on one spot stand {distance} raw units apart, inside the {reach} their footprints need"
    );
    // The one that was already there keeps its place: it is the newcomer that steps aside.
    assert_eq!(
        world.position(first).expect("a position"),
        (Fixed::from_raw(spot.0), Fixed::from_raw(spot.1)),
        "the body that was already standing there is the one that stays"
    );
}

/// **A measurement, not a rule**: how close do the map's own seeded bodies start?
#[test]
#[ignore = "measurement, not a rule"]
fn how_close_do_the_seeded_bodies_start() {
    let Some(fx) = fixture() else {
        return;
    };
    let world = seeded(&fx);
    let bodies: Vec<EntityId> = world
        .entities()
        .iter()
        .filter(|entity| entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
        .collect();
    let mut worst = (i64::MAX, None);
    let mut inside = 0;
    for (index, one) in bodies.iter().enumerate() {
        for two in bodies.iter().skip(index + 1) {
            let (distance, reach) = apart(&world, *one, *two);
            if distance < reach {
                inside += 1;
                if distance - reach < worst.0 {
                    worst = (distance - reach, Some((*one, *two, distance, reach)));
                }
            }
        }
    }
    println!(
        "SEED bodies={} pairs-inside={} worst={:?}",
        bodies.len(),
        inside,
        worst.1
    );
}

/// The same rule for a group on one point: every pair, not just the first, and each newcomer measured against all of them.
#[test]
fn a_group_put_on_one_point_all_stand_clear() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let villager = fx.economy.role["villager"];
    let scale = world.scale();
    let spot = (24 * scale, 24 * scale);

    let mut ids = Vec::new();
    for n in 0..5 {
        ids.push(place_on(&mut world, 1, 9_810 + n, villager, spot));
    }

    for (index, first) in ids.iter().enumerate() {
        for second in ids.iter().skip(index + 1) {
            let (distance, reach) = apart(&world, *first, *second);
            assert!(
                distance >= reach,
                "two of five bodies put on one point stand {distance} raw units apart, inside the {reach} \
                 their footprints need"
            );
        }
    }
}
