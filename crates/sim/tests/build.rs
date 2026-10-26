//! Building: pay the cost, put up a house, finish in the dat's `create_time`.
//!
//! Written before the implementation (D7).

use std::path::PathBuf;

use ruage_data::{Economy, Map, ResourceKind, Sim as Settings, Units};
use ruage_sim::{Command, EntityId, EntityKind, MAX_RESOURCES, SimError, Task, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: Settings,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        eprintln!(
            "skipping: {} is not present (generated data, D8)",
            units.display()
        );
        return None;
    }
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml")).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        resources: ruage_data::load_resources_file(&repo("config/resources.toml"))
            .expect("resources.toml"),
        units: ruage_data::load_units_file(&units).expect("units.toml"),
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
    world
}

fn first_villager(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| e.kind == EntityKind::Unit && e.player == 1)
        .expect("a villager")
        .id
}

/// A free grass tile near the villager, so the site is legal and close.
fn free_tile_near(world: &World, from: EntityId) -> (u16, u16) {
    let position = world.position(from).expect("position");
    let scale = world.scale();
    let x = u16::try_from(position.0.raw() / scale).expect("in range");
    let y = u16::try_from(position.1.raw() / scale).expect("in range");
    for radius in 1..6u16 {
        for dy in 0..=radius {
            for dx in 0..=radius {
                let candidate = (x + dx, y + dy);
                let occupied = world.entities().iter().any(|e| {
                    e.position
                        == (
                            ruage_fixed::Fixed::from_raw(i64::from(candidate.0) * scale),
                            ruage_fixed::Fixed::from_raw(i64::from(candidate.1) * scale),
                        )
                });
                if !occupied {
                    return candidate;
                }
            }
        }
    }
    panic!("no free tile near the villager");
}

/// Run gathering until the player has at least `want` wood in the bank.
fn bank_wood(world: &mut World, want: u32, node: EntityId) {
    let villager = first_villager(world);
    world.submit(Command::Gather { villager, node });
    for _ in 0..60_000 {
        world.step().expect("step");
        if world.stock(1, ResourceKind::Wood) >= want {
            return;
        }
    }
    panic!(
        "could not bank {want} wood; the bank holds {}",
        world.stock(1, ResourceKind::Wood)
    );
}

#[test]
fn a_build_without_the_resources_is_refused() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let site = free_tile_near(&world, villager);
    let house = 70;

    assert_eq!(
        world.stock(1, ResourceKind::Wood),
        0,
        "the player starts broke"
    );
    match world.submit_checked(Command::Build {
        villager,
        unit: house,
        at: site,
    }) {
        Err(SimError::NotEnough {
            resource,
            need,
            have,
        }) => {
            // "wood" is the pack's name for slot 0 (config/resources.toml), not a fixed word: the
            // error reports content, so it says what the pack says.
            assert_eq!(resource, "wood");
            assert_eq!(need, 25, "a house costs 25 wood");
            assert_eq!(have, 0);
        }
        other => panic!("expected NotEnough, got {other:?}"),
    }
}

/// A pack of *our own* resources: names and slots, no reference ids at all. Nothing in it can
/// resolve a cost that arrived from the reference pack, which is the shape a mod has (D17/D18).
const OWN_PACK: &str = r#"
schema = 1

[[resource]]
slot = 0
name = "timber"

[[resource]]
slot = 1
name = "grain"
"#;

/// The same, but with one reference id, in a column past the stockpile's width.
#[allow(dead_code)]
const WIDE_PACK: &str = r#"
schema = 1

[[resource]]
slot = 0
name = "timber"

[[resource]]
slot = 1
name = "grain"

[[resource]]
slot = 2
name = "iron"

[[resource]]
slot = 3
name = "salt"

[[resource]]
slot = 4
name = "clay"
dat_id = 1
"#;

#[test]
fn a_pack_that_does_not_carry_a_cost_id_refuses_the_order() {
    // The pack is the authority on what a cost's id means (D17). This pack has no reference ids,
    // so the house's "25 of dat id 1" names nothing. Paying from slot 0 anyway would take the
    // timber column because the enum happens to number wood 0 - a house bought with the wrong
    // goods, which no test could notice and no player could explain.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let site = free_tile_near(&world, villager);
    let node = world
        .entities()
        .iter()
        .filter(|e| matches!(e.kind, EntityKind::Resource(ResourceKind::Wood)))
        .map(|e| e.id)
        .next()
        .expect("a tree");
    bank_wood(&mut world, 25, node);
    let before = world.stock(1, ResourceKind::Wood);
    assert!(before >= 25, "the villager banked {before} wood");

    let pack = ruage_data::load_resources_str(OWN_PACK, "<own pack>").expect("the pack loads");
    world
        .load_resources(&pack)
        .expect("the pack fits the stockpile");
    let err = world
        .submit_checked(Command::Build {
            villager,
            unit: 70,
            at: site,
        })
        .expect_err("a cost naming an id the pack lacks must be refused");

    assert!(
        matches!(err, SimError::UnknownResource { dat_id: 1 }),
        "expected UnknownResource for dat id 1, got {err:?}"
    );
    let text = format!("{err}");
    assert!(
        text.contains("id 1") && text.contains("does not carry"),
        "the refusal must name the id and say the pack lacks it: {text}"
    );
    assert_eq!(
        world.stock(1, ResourceKind::Wood),
        before,
        "a refused order must not be paid from slot 0"
    );
}

/// One resource past the widest stockpile a world can hold (eight columns): nine named resources,
/// the last of them answering to the reference pack's wood id.
const TOO_WIDE_PACK: &str = r#"
schema = 1

[[resource]]
slot = 0
name = "timber"

[[resource]]
slot = 1
name = "grain"

[[resource]]
slot = 2
name = "iron"

[[resource]]
slot = 3
name = "salt"

[[resource]]
slot = 4
name = "clay"

[[resource]]
slot = 5
name = "wool"

[[resource]]
slot = 6
name = "resin"

[[resource]]
slot = 7
name = "pitch"

[[resource]]
slot = 8
name = "sand"
dat_id = 1
"#;

#[test]
fn a_pack_wider_than_the_stockpile_is_refused_when_it_is_loaded() {
    // The stockpile is a fixed array, so its width is a fact about the engine and a pack has to fit
    // it. Refusing at load - rather than at the first cost - means no order can ever name a column
    // that does not exist, and it is the same fact stated once instead of once per site.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);

    let pack =
        ruage_data::load_resources_str(TOO_WIDE_PACK, "<too wide>").expect("the pack parses");
    let err = world
        .load_resources(&pack)
        .expect_err("nine resources do not fit eight stockpile columns");

    assert!(
        matches!(
            err,
            SimError::TooManyResources {
                count: 9,
                max: MAX_RESOURCES
            }
        ),
        "expected TooManyResources {{ count: 9, max: {MAX_RESOURCES} }}, got {err:?}"
    );
    let text = format!("{err}");
    assert!(
        text.contains('9') && text.contains(&MAX_RESOURCES.to_string()),
        "the refusal must name the count and the width: {text}"
    );
}

#[test]
fn a_house_is_paid_for_and_finishes_in_its_create_time() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let here = world.position(villager).expect("position");
    let node = world
        .entities()
        .iter()
        .filter(|e| matches!(e.kind, EntityKind::Resource(ResourceKind::Wood)))
        .min_by_key(|e| {
            let dx = e.position.0.raw() - here.0.raw();
            let dy = e.position.1.raw() - here.1.raw();
            (dx * dx + dy * dy, e.id.0)
        })
        .expect("a tree")
        .id;

    bank_wood(&mut world, 25, node);
    let before = world.stock(1, ResourceKind::Wood);
    let site = free_tile_near(&world, villager);
    let house_unit = 70;
    let build_time = fx.units.unit(house_unit).expect("HOUS").train_time;
    assert_eq!(build_time, 25, "a house takes 25 seconds");

    world
        .submit_checked(Command::Build {
            villager,
            unit: house_unit,
            at: site,
        })
        .expect("the build order must be accepted");

    // Commands are queued and land on the next tick (`CommandApply::NextTick`), so the price is
    // paid then - not when the house finishes, and not before the order is actually being run.
    world.step().expect("step");
    assert_eq!(
        world.stock(1, ResourceKind::Wood),
        before - 25,
        "the wood is spent when the order lands, not when the house finishes"
    );

    let site_entity = world
        .entities()
        .iter()
        .find(|e| {
            e.kind == EntityKind::Building
                && e.unit == Some(house_unit)
                && e.position
                    == (
                        ruage_fixed::Fixed::from_raw(i64::from(site.0) * world.scale()),
                        ruage_fixed::Fixed::from_raw(i64::from(site.1) * world.scale()),
                    )
        })
        .expect("the construction site exists")
        .id;

    let max_hp = fx.units.unit(house_unit).expect("HOUS").hp;
    let total_ticks =
        u64::from(u32::try_from(build_time).expect("positive")) * u64::from(world.tick_rate_hz());
    let mut last_hp = 0;
    let mut arrived_at = None;
    let mut finished_at = None;

    for _ in 0..(total_ticks * 4) {
        world.step().expect("step");
        if arrived_at.is_none()
            && world.position(villager).expect("position")
                == world.position(site_entity).expect("position")
        {
            arrived_at = Some(world.tick());
        }
        let hp = world.entity(site_entity).expect("site").hp;
        assert!(hp >= last_hp, "construction must not go backwards");
        last_hp = hp;
        if hp == max_hp {
            finished_at = Some(world.tick());
            break;
        }
    }

    // Walking time is not build time: the clock starts when the builder is on the site, and the
    // work phase reads positions before movement, so the tick of arrival is not a working tick.
    // N working ticks therefore end exactly N ticks after arrival.
    let arrived = arrived_at.expect("the builder reaches the site");
    let finished = finished_at.expect("the house is finished");
    assert_eq!(
        finished - arrived,
        total_ticks,
        "a {build_time} s house at {} Hz must take {total_ticks} working ticks, walked {} ticks first",
        world.tick_rate_hz(),
        arrived
    );
    assert_eq!(
        world.entity(site_entity).expect("site").kind,
        EntityKind::Building
    );
    assert_eq!(
        world.entity(villager).expect("villager").task,
        Task::Idle,
        "the builder stops when the house is finished"
    );
}

#[test]
fn a_site_on_top_of_something_is_refused() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let here = world.position(villager).expect("position");
    let node = world
        .entities()
        .iter()
        .filter(|e| matches!(e.kind, EntityKind::Resource(ResourceKind::Wood)))
        .min_by_key(|e| {
            let dx = e.position.0.raw() - here.0.raw();
            let dy = e.position.1.raw() - here.1.raw();
            (dx * dx + dy * dy, e.id.0)
        })
        .expect("a tree")
        .id;
    bank_wood(&mut world, 25, node);

    let town_centre = world
        .entities()
        .iter()
        .find(|e| e.kind == EntityKind::Building && e.player == 1)
        .expect("TC");
    let scale = world.scale();
    let occupied = (
        u16::try_from(town_centre.position.0.raw() / scale).unwrap(),
        u16::try_from(town_centre.position.1.raw() / scale).unwrap(),
    );

    match world.submit_checked(Command::Build {
        villager,
        unit: 70,
        at: occupied,
    }) {
        Err(SimError::SiteOccupied { tile }) => assert_eq!(tile, occupied),
        other => panic!("expected SiteOccupied, got {other:?}"),
    }
}

/// A 16x16 map split in two by a wall of forest running the full height. Resources and town
/// centres are mirrored point-for-point, so the loader accepts it.
const WALLED_MAP: &str = r#"
schema = 1

[map]
width = 16
height = 16
terrain = [
  "GGGGGGGFFGGGGGGG",
  "GGGGGGGFFGGGGGGG",
  "GGFFGGGFFGGGGGGG",
  "GGFFGGGFFGGGGGGG",
  "GGGGGGGFFGGGGGGG",
  "GGGGGGGFFGGGGGGG",
  "GGGGGGGFFGGGGGGG",
  "GGGGGGGFFGGGGGGG",
  "GGGGGGGFFGGGGGGG",
  "GGGGGGGFFGGGGGGG",
  "GGGGGGGFFGGGGGGG",
  "GGGGGGGFFGGGGGGG",
  "GGGGGGGFFGGGFFGG",
  "GGGGGGGFFGGGFFGG",
  "GGGGGGGFFGGGGGGG",
  "GGGGGGGFFGGGGGGG",
]

[[map.resource]]
kind = "wood"
amount = 200
tiles = [[2, 2], [2, 3], [3, 2], [3, 3], [12, 12], [12, 13], [13, 12], [13, 13]]

[[map.town_centre]]
x = 3
y = 6
player = 1

[[map.town_centre]]
x = 12
y = 9
player = 2

"#;

#[test]
fn an_unreachable_site_is_refused_before_the_wood_is_spent() {
    let Some(fx) = fixture() else { return };
    let map = ruage_data::load_map_str(WALLED_MAP, "<walled fixture>")
        .expect("the walled fixture map loads");
    let mut world = World::new(&fx.settings, 11).expect("world");
    world
        .seed_from(&map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    let villager = first_villager(&world);
    let scale = world.scale();
    let wall = ruage_fixed::Fixed::from_raw(8 * scale);
    let wood = world
        .entities()
        .iter()
        .filter(|e| matches!(e.kind, EntityKind::Resource(ResourceKind::Wood)))
        .find(|e| e.position.0.raw() < wall.raw())
        .expect("wood on the near side")
        .id;
    bank_wood(&mut world, 25, wood);
    let before = world.stock(1, ResourceKind::Wood);
    assert!(before >= 25, "the villager banked {before} wood");
    let buildings = world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Building)
        .count();

    // (11, 9) is legal ground on the far side of the wall, and the villager cannot get there.
    match world.submit_checked(Command::Build {
        villager,
        unit: 70,
        at: (11, 9),
    }) {
        Err(SimError::NoPath { .. }) => {}
        other => panic!("expected NoPath, got {other:?}"),
    }

    assert_eq!(
        world.stock(1, ResourceKind::Wood),
        before,
        "a refused order must cost nothing"
    );
    assert_eq!(
        world
            .entities()
            .iter()
            .filter(|e| e.kind == EntityKind::Building)
            .count(),
        buildings,
        "a refused order must not leave a construction site behind"
    );
}
