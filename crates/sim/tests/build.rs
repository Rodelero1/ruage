//! Building: pay the cost, put up a house, finish in the dat's `create_time`.
//!
//! Written before the implementation (D7).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_sim::{Command, EntityId, EntityKind, MAX_RESOURCES, SimError, Task, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// The project pack's column for a resource, so a test says "wood" the way content does and the
/// pack is what turns that into a stockpile column (D17).
fn slot(name: &str) -> usize {
    slot_in(
        &ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml"),
        name,
    )
}

fn slot_in(pack: &ruage_data::Resources, name: &str) -> usize {
    pack.slot_of(name)
        .unwrap_or_else(|| panic!("the pack has no {name}"))
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
    // The pack first: the map names its resources and only the pack turns a name into a column.
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        resources,
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
        if world.stock(1, slot("wood")) >= want {
            return;
        }
    }
    panic!(
        "could not bank {want} wood; the bank holds {}",
        world.stock(1, slot("wood"))
    );
}

#[test]
fn a_build_without_the_resources_is_refused() {
    let Some(fx) = fixture() else { return };
    // A pack that opens with nothing: the project's own pack starts with the standard stockpile, and
    // the point of this test is what happens when the bank is empty.
    let mut economy = fx.economy.clone();
    economy.start.stock.clear();
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &economy, &fx.units)
        .expect("seeding");
    world.load_resources(&fx.resources).expect("the pack fits");
    let villager = first_villager(&world);
    let site = free_tile_near(&world, villager);
    let house = 70;

    assert_eq!(world.stock(1, slot("wood")), 0, "the player starts broke");
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
        .filter(|e| matches!(e.kind, EntityKind::Resource(s) if s == slot("wood")))
        .map(|e| e.id)
        .next()
        .expect("a tree");
    bank_wood(&mut world, 25, node);
    let before = world.stock(1, slot("wood"));
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
        world.stock(1, slot("wood")),
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

/// A pack with two names, neither of them the one the house is priced in.
const NO_TIMBER_PACK: &str = r#"
schema = 1

[[resource]]
slot = 0
name = "grain"

[[resource]]
slot = 1
name = "iron"
"#;

#[test]
fn a_named_cost_the_pack_does_not_define_is_refused() {
    // Same failure class as an unknown reference id, one namespace over: the pack is the only
    // authority on what a name means, so a name it does not carry must refuse the order rather than
    // be quietly dropped - a dropped cost is a free building.
    //
    // The content is the finish-line fixture's (a house priced in `timber`), with a pack whose names
    // are grain and iron: everything about this world is legal except the one word the cost uses.
    let Some(fx) = fixture() else { return };
    let pack =
        ruage_data::load_resources_str(NO_TIMBER_PACK, "<no timber>").expect("the pack loads");
    let units = ruage_data::load_units_str(OWN_UNITS, "<own units>").expect("the units load");
    let map = ruage_data::load_map_str(
        &OWN_MAP
            .replace("kind = \"timber\"", "kind = \"grain\"")
            .replace("tiles = [[1, 1], [2, 1]]", "tiles = [[3, 3]]"),
        "<own map>",
        &pack,
    )
    .expect("the map loads: grain on open ground");

    let mut economy = fx.economy.clone();
    // The pack is our own, so the opening stockpile has to be too: the project's names food, wood,
    // gold and stone, and this pack has grain, iron, salt, wool and timber.
    economy.start.stock.clear();
    economy.gather.variant.insert("wood".to_owned(), 83);
    economy
        .gather
        .resource_task
        .insert("grain".to_owned(), "wood".to_owned());

    let mut world = World::new(&fx.settings, 5).expect("world");
    world.seed_from(&map, &economy, &units).expect("seeding");
    world.load_resources(&pack).expect("the pack fits");

    let grain = pack.slot_of("grain").expect("grain");
    let villager = world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Unit)
        .map(|e| e.id)
        .expect("a villager");
    let node = world
        .entities()
        .iter()
        .find(|e| e.kind == EntityKind::Resource(grain))
        .map(|e| e.id)
        .expect("a grain node");
    world
        .submit_checked(Command::Gather { villager, node })
        .expect("gather");
    for _ in 0..2_000 {
        world.step().expect("step");
        if world.stock(1, grain) > 0 {
            break;
        }
    }
    let before = world.stock(1, grain);
    assert!(before > 0, "the villager banked nothing to spend");

    let err = world
        .submit_checked(Command::Build {
            villager,
            unit: 70,
            at: (4, 4),
        })
        .expect_err("a named cost the pack does not define must be refused");
    assert!(
        matches!(err, SimError::UnknownResourceName { ref name } if name == "timber"),
        "expected UnknownResourceName for timber, got {err:?}"
    );
    let text = format!("{err}");
    assert!(
        text.contains("timber") && text.contains("does not define"),
        "the refusal must name the resource and say the pack lacks it: {text}"
    );
    world.step().expect("step");
    assert_eq!(
        world.stock(1, grain),
        before,
        "a refused order must not be paid"
    );
}

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
        .filter(|e| matches!(e.kind, EntityKind::Resource(s) if s == slot("wood")))
        .min_by_key(|e| {
            let dx = e.position.0.raw() - here.0.raw();
            let dy = e.position.1.raw() - here.1.raw();
            (dx * dx + dy * dy, e.id.0)
        })
        .expect("a tree")
        .id;

    bank_wood(&mut world, 25, node);
    let before = world.stock(1, slot("wood"));
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
        world.stock(1, slot("wood")),
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

    let mut worked = 0;
    for _ in 0..(total_ticks * 4) {
        world.step().expect("step");
        if let Some(construction) = world.entity(site_entity).and_then(|e| e.construction) {
            worked = worked.max(u64::from(construction.progress_ticks));
        }
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
    // The count comes from `progress_ticks`, which the finishing tick clears along with the whole
    // `construction` block - so the last builder-tick is only visible as the house reaching full
    // health. Hence: one short, plus a finished house.
    assert_eq!(
        worked + 1,
        total_ticks,
        "the house needs exactly {total_ticks} builder-ticks, whatever the walk was"
    );
    assert_eq!(
        world.entity(site_entity).expect("site").hp,
        max_hp,
        "and the last of them is the one that finishes it"
    );
    // Not asserted: the ticks between "arrival" (inferred from position) and the finish. The work
    // phase reads positions before movement, so whether the arrival tick counts depends on how far
    // the villager walked - and the walk moved when the pack started opening with a stockpile. The
    // rule is the builder-tick count above; this only confirms the builder arrives and it finishes.
    let _ = (
        arrived_at.expect("the builder reaches the site"),
        finished_at.expect("the house is finished"),
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
        .filter(|e| matches!(e.kind, EntityKind::Resource(s) if s == slot("wood")))
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
    let map = ruage_data::load_map_str(WALLED_MAP, "<walled fixture>", &fx.resources)
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
        .filter(|e| matches!(e.kind, EntityKind::Resource(s) if s == slot("wood")))
        .find(|e| e.position.0.raw() < wall.raw())
        .expect("wood on the near side")
        .id;
    bank_wood(&mut world, 25, wood);
    let before = world.stock(1, slot("wood"));
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
        world.stock(1, slot("wood")),
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

/// A content pack of our own, all the way to a paid-for building: five resources (so the priced one
/// is in the fifth column the enum could never have held), a timber node in a forest, and a house
/// that costs `timber`.
///
/// This is the finish line of D17. Nothing in it names Age of Empires II: no resource ids, no
/// `ResourceKind`, no fixed count of four. `grain` sits in slot 0, which is where a fallback-to-zero
/// would put the money - so a wrong mapping pays with the wrong goods and this test says so.
const OWN_FIVE_PACK: &str = r#"
schema = 1

[[resource]]
slot = 0
name = "grain"

[[resource]]
slot = 1
name = "iron"

[[resource]]
slot = 2
name = "salt"

[[resource]]
slot = 3
name = "wool"

[[resource]]
slot = 4
name = "timber"
site = "forest"
"#;

/// Three units and nothing else: a town centre to start from and drop at, a villager that carries
/// and works, and a house priced in a name.
const OWN_UNITS: &str = r#"
schema = 1
kind = "units"
civ = 1
civ_name = "ours"

[[unit]]
id = 109
name = "TOWNC"
class = 3
hp = 2400
los = 4.0
speed = 0.0
carry = 0
work_rate = 0.0
drop_sites = []
garrison = 0
radius = [1.0, 1.0, 2.0]
costs = []
attacks = []
armours = []
range = 0.0
reload = 0.0
accuracy = 0
train_time = 200
base_id = 109

[[unit]]
id = 83
name = "VILL"
class = 4
hp = 25
los = 4.0
speed = 0.8
carry = 35
work_rate = 0.39
drop_sites = [109]
garrison = 0
radius = [0.2, 0.2, 0.0]
costs = []
attacks = []
armours = []
range = 0.0
reload = 0.0
accuracy = 0
train_time = 25
base_id = 83

[[unit]]
id = 70
name = "HOUS"
class = 3
hp = 550
los = 2.0
speed = 0.0
carry = 0
work_rate = 0.0
drop_sites = []
garrison = 0
radius = [1.0, 1.0, 2.0]
costs = [{ resource = "timber", amount = 25 }]
attacks = []
armours = []
range = 0.0
reload = 0.0
accuracy = 0
train_time = 25
base_id = 70
"#;

/// A 6x6 map with a four-tile wood in the corner, named the way our pack names it.
const OWN_MAP: &str = r#"
schema = 1
[map]
width = 6
height = 6
terrain = [
  "GGGGGG",
  "GFFFGG",
  "GFFFGG",
  "GGGGGG",
  "GGGGGG",
  "GGGGGG",
]
[[map.resource]]
kind = "timber"
amount = 500
tiles = [[1, 1], [2, 1]]
[[map.town_centre]]
player = 1
x = 5
y = 5
[[map.town_centre]]
player = 2
x = 0
y = 5
"#;

#[test]
fn a_content_pack_with_our_own_resources_loads_and_spends() {
    let Some(fx) = fixture() else { return };

    // The pack, the unit table and the map, all of our own.
    let pack = ruage_data::load_resources_str(OWN_FIVE_PACK, "<own pack>").expect("the pack loads");
    let timber = pack.slot_of("timber").expect("timber is in the pack");
    let grain = pack.slot_of("grain").expect("grain is in the pack");
    let units = ruage_data::load_units_str(OWN_UNITS, "<own units>").expect("the units load");
    let map = ruage_data::load_map_str(OWN_MAP, "<own map>", &pack).expect("the map loads");

    // The economy is the project's, with one entry added: our own name for the wood task. (The
    // economy file is content too, and this is the line it would carry in a pack of our own.)
    let mut economy = fx.economy.clone();
    // The pack is our own, so the opening stockpile has to be too: the project's names food, wood,
    // gold and stone, and this pack has grain, iron, salt, wool and timber.
    economy.start.stock.clear();
    economy.gather.variant.insert("wood".to_owned(), 83);
    economy
        .gather
        .resource_task
        .insert("timber".to_owned(), "wood".to_owned());

    let mut world = World::new(&fx.settings, 5).expect("world");
    world.seed_from(&map, &economy, &units).expect("seeding");
    world.load_resources(&pack).expect("the pack fits");

    assert_eq!(world.stock(1, grain), 0, "a fresh world has nothing banked");

    // Gather until the bank holds the price of a house, in the fifth column.
    let villager = world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Unit)
        .map(|e| e.id)
        .expect("a villager");
    let node = world
        .entities()
        .iter()
        .find(|e| e.kind == EntityKind::Resource(timber))
        .map(|e| e.id)
        .expect("a timber node");
    world
        .submit_checked(Command::Gather { villager, node })
        .expect("a gather order on a resource the pack defines");
    for _ in 0..4_000 {
        world.step().expect("step");
        if world.stock(1, timber) >= 25 {
            break;
        }
    }
    let banked = world.stock(1, timber);
    assert!(
        banked >= 25,
        "the villager banked {banked} timber, which does not pay for a house"
    );
    assert_eq!(
        world.stock(1, grain),
        0,
        "gathering timber must not credit slot 0 (grain): the cost named a resource, not a column"
    );

    // Build it, and let it finish.
    // Two tiles diagonal from the town centre: grass, empty, and reachable from the wood.
    let site = (4, 4);
    assert!(
        map.is_walkable(site.0, site.1),
        "the site must be open ground, terrain was {:?}",
        map.terrain_at(site.0, site.1)
    );
    let before = world.stock(1, timber);
    world
        .submit_checked(Command::Build {
            villager,
            unit: 70,
            at: site,
        })
        .expect("a house priced in timber, a resource the pack defines");
    // Queued commands land on the next tick (`CommandApply::NextTick`), so the price is paid then.
    world.step().expect("step");
    assert_eq!(
        world.stock(1, timber),
        before - 25,
        "the price came out of the timber column"
    );
    assert_eq!(world.stock(1, grain), 0, "and not out of grain");

    let mut finished = None;
    for _ in 0..2_000 {
        world.step().expect("step");
        if let Some(house) = world
            .entities()
            .iter()
            .find(|e| e.unit == Some(70) && e.construction.is_none() && e.player == 1)
        {
            finished = Some(house.hp);
            break;
        }
    }
    assert_eq!(
        finished,
        Some(550),
        "an own-content house must be built to full hp"
    );
}

/// A match opens with the standard stockpile, named by the pack: `[start] stock` is resource *names*
/// to amounts, so the engine never learns that 200 means "wood".
#[test]
fn a_match_opens_with_the_stockpile_the_pack_names() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    for player in [1, 2] {
        for (name, want) in [
            ("food", 200u32),
            ("wood", 200),
            ("gold", 100),
            ("stone", 200),
        ] {
            assert_eq!(
                world.stock(player, slot(name)),
                want,
                "player {player} opens with {want} {name}"
            );
        }
    }
}

/// The opening stock is content, so it can be wrong: a name the pack has no column for is refused
/// rather than silently dropped (a player with no gold cannot pay for anything, and would not know why).
#[test]
fn a_starting_stockpile_naming_a_resource_the_pack_lacks_is_refused() {
    let Some(fx) = fixture() else { return };
    let text = std::fs::read_to_string(repo("config/economy.toml")).expect("economy.toml");
    let text = text.replace("wood = 200", "timber = 200");
    let economy = ruage_data::load_economy_str(&text, "<economy with timber>").expect("it parses");

    let mut world = World::new(&fx.settings, 7).expect("world");
    // Seeding is fine: the opening stockpile cannot be resolved until the pack that owns the columns
    // is loaded, and that is what refuses.
    world
        .seed_from(&fx.map, &economy, &fx.units)
        .expect("the world seeds");
    let err = world
        .load_resources(&fx.resources)
        .expect_err("the pack cannot hold timber, so the match cannot start with it")
        .to_string();
    assert!(err.contains("timber"), "the refusal names it: {err}");
}
