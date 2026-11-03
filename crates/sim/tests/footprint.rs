//! Footprint collision (C1b): a body blocks a disc of its `radius[0]`, summed for the pair.
//!
//! Written before the implementation (D7). The rule that used to be a tile test is a **distance** now, so every
//! assertion in this file is "centres apart", never "same tile" - and the fixtures are built so that the tile rule and
//! the distance rule give different answers.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn fresh() -> Option<World> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).ok()?;
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).ok()?;
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).ok()?;
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).ok()?;
    let units = ruage_data::load_units_file(&units).ok()?;
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).ok()?;
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    world.load_combat(&combat);
    Some(world)
}

fn next_id(world: &World) -> EntityId {
    EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    )
}

fn at(world: &World, x: u16, y: u16) -> (Fixed, Fixed) {
    let scale = world.scale();
    (
        Fixed::from_raw(i64::from(x) * scale),
        Fixed::from_raw(i64::from(y) * scale),
    )
}

/// A villager placed by hand, at a tile centre. The id is chosen by the caller: a `Spawn` lands on the next tick, so
/// two spawns in one tick that both asked `next_id` would take the same id.
fn spawn_villager(world: &mut World, id: EntityId, x: u16, y: u16, speed: Fixed) {
    let unit = world.start_villager_unit().expect("a villager role");
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: at(world, x, y),
            speed,
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
}

/// How far apart two bodies are, in raw units.
fn apart(world: &World, a: EntityId, b: EntityId) -> i64 {
    let (pa, pb) = (
        world.position(a).expect("a position"),
        world.position(b).expect("a position"),
    );
    let (dx, dy) = (pa.0.raw() - pb.0.raw(), pa.1.raw() - pb.1.raw());
    (dx * dx + dy * dy).isqrt()
}

/// **A one-tile corridor.** Terrain cannot block in this model (obstacles are entities), so the walls are two rows of
/// trees. A head-on pair in it has nowhere to slide: the only way past is to make room, which is the rule that keeps a
/// crossing pair from deadlocking on each other's footprints.
const CORRIDOR: &str = r#"
schema = 1
[map]
width = 14
height = 9
terrain = [
  "GGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 100
hp = 0
tiles = [[2,3],[3,3],[4,3],[5,3],[6,3],[7,3],[8,3],[9,3],[10,3],[11,3]]
[[map.resource]]
kind = "wood"
amount = 100
hp = 0
tiles = [[2,5],[3,5],[4,5],[5,5],[6,5],[7,5],[8,5],[9,5],[10,5],[11,5]]
[[map.town_center]]
player = 1
x = 2
y = 7
[[map.town_center]]
player = 2
x = 11
y = 7
"#;

fn corridor() -> Option<World> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).ok()?;
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).ok()?;
    let map = ruage_data::load_map_str(CORRIDOR, "corridor.toml", &resources).ok()?;
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).ok()?;
    let units = ruage_data::load_units_file(&units).ok()?;
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    Some(world)
}

/// Two villagers walk at each other down a corridor one tile wide, with a tree row above and below.
///
/// Without a make-room step they deadlock: each refuses the other's step, a one-tile row gives no sideways component to
/// slide on, and the repath cannot help because both already stand inside the tile it would avoid. The assertion is the
/// standoff the whole slice is about - they may pass, they may not overlap.
#[test]
fn a_head_on_pair_in_a_one_tile_corridor_passes() {
    let Some(mut world) = corridor() else {
        return;
    };
    let scale = world.scale();
    let a = next_id(&world);
    let b = EntityId(a.0 + 1);
    let (left, right) = (3u16, 10u16);
    let row = 4u16;
    // Fast enough that the pair meets inside the test's patience: one tile a second.
    spawn_villager(&mut world, a, left, row, Fixed::from_raw(scale));
    spawn_villager(&mut world, b, right, row, Fixed::from_raw(scale));
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        unit: a,
        to: (right, row),
    });
    world.submit(Command::Move {
        unit: b,
        to: (left, row),
    });

    let reach = world.reach_between(a, b).raw();
    for tick in 0..2_000 {
        world.step().expect("step");
        assert!(
            apart(&world, a, b) >= reach,
            "the pair came {} raw units apart on tick {tick}, inside the {reach} their footprints need",
            apart(&world, a, b)
        );
    }
    // Both got past each other: "arrived" is a tile within one of the goal, because a body stopped by its friend's
    // footprint cannot always reach the centre.
    let near = |id: EntityId, goal: u16| {
        let (x, _) = world.position(id).expect("a position");
        let tile = x.raw().div_euclid(scale);
        (tile - i64::from(goal)).abs() <= 1
    };
    assert!(
        near(a, right) && near(b, left),
        "both have to get past: a at {:?}, b at {:?}",
        world.position(a),
        world.position(b)
    );
}

/// Two bodies that already stand inside each other can still separate: a step that **widens** an overlap is always
/// allowed, or a pair that finds itself overlapped (a spawn, a push out of a building) is frozen for ever.
#[test]
fn a_pair_standing_inside_each_other_can_separate() {
    let Some(mut world) = fresh() else {
        return;
    };
    let scale = world.scale();
    let a = next_id(&world);
    let b = EntityId(a.0 + 1);
    spawn_villager(&mut world, a, 26, 24, Fixed::from_raw(scale / 2));
    spawn_villager(&mut world, b, 26, 24, Fixed::from_raw(scale / 2));
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        unit: a,
        to: (30, 24),
    });
    world.submit(Command::Move {
        unit: b,
        to: (22, 24),
    });
    for _ in 0..600 {
        world.step().expect("step");
    }

    let reach = world.reach_between(a, b).raw();
    assert!(
        apart(&world, a, b) >= reach,
        "the pair stayed inside each other: {} raw units apart, {} needed",
        apart(&world, a, b),
        reach
    );
}

/// An open patch of the project map, far enough from the seeded base that its own villagers are not in the way.
fn open_ground(world: &World) -> Option<(u16, u16)> {
    let (width, height) = (64u16, 64u16);
    for y in 2..height.saturating_sub(2) {
        for x in 2..width.saturating_sub(6) {
            if (0..6).all(|d| world.is_free_ground(x + d, y))
                && !((8..24).contains(&x) && (8..24).contains(&y))
            {
                return Some((x + 3, y));
            }
        }
    }
    None
}

/// **C2: two workers on one body both take from it.** A body lying on walkable ground used to be worked only from its
/// exact centre (`position == target`), so with footprints in place the second villager stops 0.4 tiles short and takes
/// nothing for the rest of the match - the deadlock that made collision unusable on its own (89 wood at 1,600 s against
/// 729). The rule is a distance now: standing on the node's ground, or close enough for the footprints to touch, is
/// being at it.
#[test]
fn both_workers_on_one_body_take_from_it() {
    let Some(mut world) = fresh() else {
        return;
    };
    let scale = world.scale();
    let Some((x, y)) = open_ground(&world) else {
        return;
    };
    let corpse_unit = world.role_unit("wild_boar_corpse").expect("a carcass role");
    let corpse = next_id(&world);
    world
        .submit_checked(Command::Spawn {
            id: corpse,
            player: 0,
            kind: EntityKind::Unit,
            unit: Some(corpse_unit),
            position: at(&world, x, y),
            speed: Fixed::ZERO,
            hp: 0,
            amount: 340,
        })
        .expect("a body can be placed");
    let a = EntityId(corpse.0 + 1);
    let b = EntityId(corpse.0 + 2);
    spawn_villager(&mut world, a, x - 2, y, Fixed::from_raw(scale / 2));
    spawn_villager(&mut world, b, x + 2, y, Fixed::from_raw(scale / 2));
    world.step().expect("the spawns land");

    world.submit(Command::Gather {
        villager: a,
        node: corpse,
    });
    world.submit(Command::Gather {
        villager: b,
        node: corpse,
    });
    for _ in 0..800 {
        world.step().expect("step");
    }

    let reach = world.reach_between(a, b).raw();
    assert!(
        apart(&world, a, b) >= reach,
        "the two workers stand inside each other: {} raw units apart, {reach} needed",
        apart(&world, a, b)
    );
    for id in [a, b] {
        let carry = world.entity(id).expect("a worker").carry.raw();
        assert!(
            carry > 0,
            "villager {} never took anything from the body: a body on walkable ground can only be worked from its \
             exact centre",
            id.0
        );
    }
}
