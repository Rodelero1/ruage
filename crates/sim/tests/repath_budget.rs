//! **A re-plan that fails must cost one search every so often, not one every tick** (D135).
//!
//! The engine answers ground it cannot cross by re-planning the route, at most twice per route (D127). It counted the
//! **successful** re-plans; a re-plan that *failed* left the count where it was, so the unit asked again on the next tick,
//! and the next. Measured on the acceptance match of the D134 branch: **2,533 failed searches in 12,000 ticks, 8.1 s of a
//! 20 s run**, one every tick a unit spent against ground it could not answer for.
//!
//! Written before the implementation (D7, SPEC §11). `World::repath_failures` is scaffolding added first, so that the
//! failure below is an assertion rather than a compile error.

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

/// Nine by nine of open grass: a wall across the middle is the whole width of the world, so there is no way round it.
const PLATE: &str = r#"
schema = 1
[map]
width = 9
height = 9
terrain = [
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
  "GGGGGGGGG",
]
[[map.town_center]]
player = 1
x = 0
y = 0
[[map.town_center]]
player = 2
x = 8
y = 8
"#;

fn loaded(text: &str) -> Option<World> {
    let units_path = common::repo("config/gamedata/units.toml");
    if !units_path.exists() {
        return None;
    }
    let resources = common::present(
        &common::repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings = common::present(&common::repo("config/sim.toml"), ruage_data::load_sim_file);
    let map = common::must(
        "load_map_str",
        ruage_data::load_map_str(text, "plate.toml", &resources),
    );
    let economy = common::present(
        &common::repo("config/economy.toml"),
        ruage_data::load_economy_file,
    );
    let units = common::must("load_units_file", ruage_data::load_units_file(&units_path));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn spawn_wall(world: &mut World, id: u32, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let unit = world.role_unit("palisade_wall").expect("a wall role");
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp: 250,
            amount: 0,
        })
        .expect("a wall can be placed");
    world.step().expect("the spawn lands");
    id
}

fn spawn_walker(world: &mut World, id: u32, tile: (i64, i64), speed: Fixed) -> EntityId {
    let scale = world.scale();
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: world.start_villager_unit(),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed,
            hp: 25,
            amount: 0,
        })
        .expect("a walker can be placed");
    world.step().expect("the spawn lands");
    id
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let entity = world.entity(id).expect("an entity with a tile");
    (
        entity.position.0.raw().div_euclid(world.scale()),
        entity.position.1.raw().div_euclid(world.scale()),
    )
}

/// A walker on its way, then a wall across its route **and on its goal**: every re-plan from then on fails, and the unit
/// must stop asking after its two attempts instead of spending a search a tick for the rest of the match.
#[test]
fn ground_it_cannot_cross_is_re_planned_at_most_twice() {
    let Some(mut world) = loaded(PLATE) else {
        return;
    };
    let speed = Fixed::from_raw(world.scale());
    let walker = spawn_walker(&mut world, 9_960, (1, 4), speed);

    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (7, 4),
        })
        .expect("a move across open ground");
    for _ in 0..5 {
        world.step().expect("step");
    }

    // The ground goes stale under it: a wall across the whole width, and the goal tile itself walled, so no route to the
    // goal exists at all - the case a re-plan cannot answer.
    for y in 0..9i64 {
        spawn_wall(&mut world, 9_970 + y as u32, (4, y));
    }
    spawn_wall(&mut world, 9_980, (7, 4));

    let before = world.repath_failures();
    for _ in 0..600 {
        world.step().expect("step");
    }
    let failed = world.repath_failures() - before;

    assert!(
        failed <= 2,
        "{failed} failed re-plans in 600 ticks against ground that cannot be crossed: a search a tick is the cost that \
         made the D134 branch's match run at 3.5 ms a tick instead of 0.3"
    );
    // And it is still where the walls allow - it did not walk through them.
    let (x, y) = tile_of(&world, walker);
    assert!(
        world.tile_walkable(
            u16::try_from(x).expect("in range"),
            u16::try_from(y).expect("in range")
        ),
        "it ended on blocked ground at ({x}, {y})"
    );
}
