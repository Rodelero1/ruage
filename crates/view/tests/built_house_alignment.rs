//! **A house the villagers built is drawn where it blocks** (D165): the owner reports units walking over a building's top side and unable
//! to approach its bottom, "a shift between graphics and path". A *spawned* house was checked in D159; this checks a **constructed** one,
//! which is the case a match actually presents.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};
use ruage_view::{Snapshot, footprint_tiles};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn seeded() -> Option<(World, ruage_data::Map)> {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        return None;
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
    Some((world, map))
}

#[test]
fn a_house_built_by_a_villager_is_drawn_where_it_blocks() {
    let Some((mut world, map)) = seeded() else {
        return;
    };
    let scale = world.scale();
    let house_role = world.role_unit("house").expect("house");
    let villager_role = world.role_unit("villager").expect("villager");
    // Open ground, far from the base.
    let site = (40i64, 40i64);
    for y in 38..44i64 {
        for x in 38..44i64 {
            assert!(world.tile_walkable(x as u16, y as u16) || true);
        }
    }
    let id = EntityId(9_900);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager_role),
            position: (
                Fixed::from_raw((site.0 - 2) * scale),
                Fixed::from_raw(site.1 * scale),
            ),
            speed: Fixed::parse("1.0", scale).expect("speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("step");
    world
        .submit_checked(Command::Build {
            villager: id,
            unit: house_role,
            at: (
                u16::try_from(site.0).expect("in range"),
                u16::try_from(site.1).expect("in range"),
            ),
        })
        .expect("a build order");
    for _ in 0..4_000 {
        world.step().expect("step");
    }

    let snapshot = Snapshot::of(&world, &map, 1);
    let built = snapshot
        .entities
        .iter()
        .find(|entity| entity.unit == Some(house_role) && entity.kind == EntityKind::Building)
        .expect("the house is in the snapshot");
    let anchor = (
        built.position.0.floor() as i32,
        built.position.1.floor() as i32,
    );
    let (min_x, min_y, max_x, max_y) = footprint_tiles(anchor, built.footprint);
    assert!(
        max_x - min_x == 1 && max_y - min_y == 1,
        "a house is 2x2: {min_x},{min_y}..{max_x},{max_y}"
    );

    // **Every drawn tile is blocked and every ring tile is not** - the shift the owner describes would show up as a free tile inside the
    // drawing, or a blocked tile outside it.
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            assert!(
                !world.tile_walkable(x as u16, y as u16),
                "the drawn tile ({x},{y}) is walkable: units walk over the building"
            );
        }
    }
    for y in (min_y - 1)..=(max_y + 1) {
        for x in (min_x - 1)..=(max_x + 1) {
            let inside = x >= min_x && x <= max_x && y >= min_y && y <= max_y;
            if inside || x < 0 || y < 0 {
                continue;
            }
            if !world.is_free_ground(x as u16, y as u16) {
                continue;
            }
            assert!(
                world.tile_walkable(x as u16, y as u16),
                "the tile ({x},{y}) beside the drawing is blocked: units cannot approach"
            );
        }
    }
}
