//! **Approaching a building is symmetric** (D165): the owner reports units reaching a building's top side and stopping short of its bottom.
//! That asymmetry, if real, is a pathing fault; if it is not real, the difference is the drawing.

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

use std::path::PathBuf;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

#[test]
fn a_unit_can_reach_a_house_from_the_north_and_from_the_south() {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        return;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&units_path).expect("units");
    let mut world = World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seeding");
    world.load_resources(&resources).expect("stockpile");

    let scale = world.scale();
    let house = world.role_unit("house").expect("house");
    let villager = world.role_unit("villager").expect("villager");
    // A clear stretch of the project map.
    let mut site = None;
    'search: for y in 30..50i64 {
        for x in 30..50i64 {
            let clear = (-3..4).all(|dy: i64| {
                (-1..4).all(|dx: i64| {
                    world.tile_walkable((x + dx) as u16, (y + dy) as u16)
                        && world.is_free_ground((x + dx) as u16, (y + dy) as u16)
                })
            });
            if clear {
                site = Some((x, y));
                break 'search;
            }
        }
    }
    let Some((sx, sy)) = site else { return };

    for (id, at, goal) in [
        (9_950u32, (sx, sy - 3), (sx, sy - 1)),
        (9_951u32, (sx, sy + 5), (sx, sy + 3)),
    ] {
        world
            .submit_checked(Command::Spawn {
                id: EntityId(id),
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager),
                position: (Fixed::from_raw(at.0 * scale), Fixed::from_raw(at.1 * scale)),
                speed: Fixed::parse("1.0", scale).expect("speed"),
                hp: 25,
                amount: 0,
            })
            .expect("a villager");
        world.step().expect("step");
        world
            .submit_checked(Command::Move {
                unit: EntityId(id),
                to: (
                    u16::try_from(goal.0).expect("in range"),
                    u16::try_from(goal.1).expect("in range"),
                ),
            })
            .expect("a walk");
    }
    // The house goes up between them, after they are walking.
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_952),
            player: 1,
            kind: EntityKind::Building,
            unit: Some(house),
            position: (Fixed::from_raw(sx * scale), Fixed::from_raw(sy * scale)),
            speed: Fixed::ZERO,
            hp: 550,
            amount: 0,
        })
        .expect("a house");
    for _ in 0..1_500 {
        world.step().expect("step");
    }

    let north = world.tile_of_position(world.position(EntityId(9_950)).expect("north unit"));
    let south = world.tile_of_position(world.position(EntityId(9_951)).expect("south unit"));
    eprintln!(
        "DBG north ends at {north:?} (goal ({sx},{})), south at {south:?} (goal ({sx},{}))",
        sy - 1,
        sy + 3
    );
    // **Symmetric**: each is at most one tile from its goal, on its own side.
    assert!(
        (i64::from(north.1) - (sy - 1)).abs() <= 1,
        "the northern unit reached {north:?}, not near its goal"
    );
    assert!(
        (i64::from(south.1) - (sy + 3)).abs() <= 1,
        "the southern unit reached {south:?}, not near its goal"
    );
}
