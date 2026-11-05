//! What stands on a footprint **blocks**: every building is an obstacle from the first tick of its foundation, except a
//! farm (a floor, walkable while it is built and after) and a town centre, whose own mask makes its yard walkable and its
//! core solid.
//!
//! Written before the implementation (D7), from the owner's observation of the running game (2026-11-03):
//!
//! - "Villagers pass through the buildings" - they should not;
//! - "only farm and partially Town Center can be walked over when done, and only farm can be passed over during
//!   building it";
//! - "all other buildings block passage since start of construction";
//! - "building may not overlap in any case (only expired farm can be)".

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, SimError, World};

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

/// A free id: `start_build` numbers its foundation `max + 1`, so a hand-picked literal is a trap.
fn fresh_id(world: &World) -> u32 {
    world
        .entities()
        .iter()
        .map(|entity| entity.id.0)
        .max()
        .unwrap_or(0)
        + 1
}

/// Whether a unit may **walk** on this tile. `is_free_ground` also refuses a tile some entity's anchor stands on -
/// which is the right question for placing a building and the wrong one here, because a farm's floor is walked on.
fn free_ground(world: &World, tile: (i64, i64)) -> bool {
    u16::try_from(tile.0)
        .ok()
        .zip(u16::try_from(tile.1).ok())
        .is_some_and(|(x, y)| world.tile_walkable(x, y))
}

/// An empty rectangle of the project map, at least three tiles clear on its west side, away from the seeded base (which
/// sits around (16, 16)) - so a fixture does not depend on a hand-picked tile that content may one day fill.
fn open_patch(world: &World, width: i64, height: i64) -> Option<(i64, i64)> {
    for y in 26..56i64 {
        for x in 4..50i64 {
            let clear =
                (0..width).all(|dy| (0..height).all(|dx| free_ground(world, (x + dx, y + dy))));
            let approach = (1..=3).all(|back| free_ground(world, (x - back, y)));
            if clear && approach {
                return Some((x, y));
            }
        }
    }
    None
}

fn villager(world: &mut World, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let id = EntityId(fresh_id(world));
    let unit = world.role_unit("villager").expect("villager");
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::parse("1.0", scale).expect("a walkable speed"),
            hp: 25,
            amount: 0,
        })
        .expect("spawn villager");
    world.step().expect("step");
    id
}

/// A finished building, placed directly - for the cases where what matters is what stands there.
fn building(world: &mut World, player: u8, role: &str, tile: (i64, i64), amount: u32) -> EntityId {
    let scale = world.scale();
    let id = EntityId(fresh_id(world));
    let unit = world.role_unit(role).unwrap_or_else(|| panic!("{role}"));
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Building,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp: 550,
            amount,
        })
        .expect("spawn building");
    world.step().expect("step");
    id
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let entity = world.entity(id).expect("an entity with a tile");
    (
        entity.position.0.raw() / world.scale(),
        entity.position.1.raw() / world.scale(),
    )
}

/// The tiles a building of this role covers, from its own footprint - the ground the renderer draws as its diamond.
fn footprint_tiles(world: &World, role: &str, tile: (i64, i64)) -> Vec<(i64, i64)> {
    let unit = world.role_unit(role).unwrap_or_else(|| panic!("{role}"));
    let (width, height) = world.footprint_of(unit).expect("a footprint");
    let mut tiles = Vec::new();
    for dy in 0..i64::from(height) {
        for dx in 0..i64::from(width) {
            tiles.push((tile.0 + dx, tile.1 + dy));
        }
    }
    tiles
}

/// **A finished house blocks every tile of its footprint.** The owner saw villagers walk through buildings; before this
/// slice the only role that blocked anything was the palisade wall (`[build] blocking_roles`), so a house was a picture.
#[test]
fn a_finished_house_blocks_its_whole_footprint() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 5, 5) else {
        return;
    };
    building(&mut world, 1, "house", at, 0);

    let walkable: Vec<(i64, i64)> = footprint_tiles(&world, "house", at)
        .into_iter()
        .filter(|tile| free_ground(&world, *tile))
        .collect();
    assert!(
        walkable.is_empty(),
        "a house leaves {} of its footprint walkable: {walkable:?}",
        walkable.len()
    );
}

/// And a villager **walks round it**: the order is to the far side, and the body never stands on the house.
#[test]
fn a_villager_walks_round_a_house_rather_than_through_it() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 5, 5) else {
        return;
    };
    building(&mut world, 1, "house", at, 0);
    let house = footprint_tiles(&world, "house", at);
    let start = (at.0 - 4, at.1 + 1);
    let goal = (at.0 + 6, at.1 + 1);
    let walker = villager(&mut world, start);

    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (
                u16::try_from(goal.0).expect("in range"),
                u16::try_from(goal.1).expect("in range"),
            ),
        })
        .expect("a move past the house");
    let mut on_the_house = Vec::new();
    for tick in 0..1_600 {
        world.step().expect("step");
        let now = tile_of(&world, walker);
        if house.contains(&now) {
            on_the_house.push((tick, now));
        }
    }
    assert!(
        on_the_house.is_empty(),
        "the walker stood on the house at {on_the_house:?}"
    );
    let (x, y) = tile_of(&world, walker);
    assert!(
        x >= goal.0 - 1 && (goal.1 - y).abs() <= 1,
        "and it still arrives: {x} {y} for {goal:?}"
    );
}

/// **A foundation blocks from the tick it exists**, not from the tick it finishes: a half-built house is not a gate.
#[test]
fn a_foundation_blocks_while_it_is_still_being_built() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 5, 5) else {
        return;
    };
    let builder = villager(&mut world, (at.0 - 3, at.1));
    let house = world.role_unit("house").expect("house");
    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: house,
            at: (
                u16::try_from(at.0).expect("in range"),
                u16::try_from(at.1).expect("in range"),
            ),
        })
        .expect("a build order");

    let mut site = None;
    for _ in 0..600 {
        world.step().expect("step");
        site = world
            .entities()
            .iter()
            .find(|entity| {
                entity.kind == EntityKind::Building
                    && entity.construction.is_some()
                    && entity.unit == Some(house)
                    && tile_of(&world, entity.id) == at
            })
            .map(|entity| entity.id);
        if site.is_some() {
            break;
        }
    }
    let site = site.expect("the foundation exists");
    assert!(
        world
            .entity(site)
            .is_some_and(|entity| entity.construction.is_some()),
        "it is still being built, which is the point of the test"
    );
    let walkable: Vec<(i64, i64)> = footprint_tiles(&world, "house", at)
        .into_iter()
        .filter(|tile| free_ground(&world, *tile))
        .collect();
    assert!(
        walkable.is_empty(),
        "a foundation is walkable on {} tiles: {walkable:?}",
        walkable.len()
    );
}

/// **A farm is a floor**: nothing of it blocks, while it is being built or after - the one building a villager may cross.
#[test]
fn a_farm_is_a_floor_while_it_is_built_and_after() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 5, 5) else {
        return;
    };
    building(&mut world, 1, "farm", at, 0);
    let farm = footprint_tiles(&world, "farm", at);

    let blocked: Vec<(i64, i64)> = farm
        .iter()
        .copied()
        .filter(|tile| !free_ground(&world, *tile))
        .collect();
    assert!(
        blocked.is_empty(),
        "a finished farm blocks {} tiles: {blocked:?}",
        blocked.len()
    );

    // And a villager ordered onto it stands on it: a floor is walked on.
    let walker = villager(&mut world, (at.0 - 3, at.1));
    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (
                u16::try_from(at.0).expect("in range"),
                u16::try_from(at.1).expect("in range"),
            ),
        })
        .expect("a move onto the farm");
    for _ in 0..600 {
        world.step().expect("step");
    }
    assert_eq!(
        tile_of(&world, walker),
        at,
        "the walker did not end on the farm it was sent to"
    );
}

/// The town centre is the one building with a **partly** walkable footprint: its core is solid and its yard is ground,
/// because content says so (`[footprint.town_center]`). The engine's answer is what this pins.
#[test]
fn the_town_centre_keeps_its_solid_core_and_its_yard() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    let at = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .map(|entity| tile_of(&world, entity.id))
        .expect("a town centre");
    let tiles = footprint_tiles(&world, "town_center", at);
    let blocked: Vec<(i64, i64)> = tiles
        .iter()
        .copied()
        .filter(|tile| !free_ground(&world, *tile))
        .collect();
    assert_eq!(
        blocked.len(),
        4,
        "the mask's four core cells should be the only blocked ground of {}: {blocked:?}",
        tiles.len()
    );
    assert!(
        blocked.contains(&at),
        "the anchor tile is the core: {blocked:?}"
    );
    assert!(
        free_ground(&world, (at.0 + 3, at.1 + 3)),
        "the far corner is the yard and should be walkable"
    );
}

/// **No building may overlap another in any case** - and the check is the footprint, not the anchor tile. A house on the
/// town centre's own yard is the case the owner can see: the yard is walkable ground, so every rule that asked "is this
/// tile free" said yes.
#[test]
fn a_house_cannot_be_built_on_the_town_centres_yard() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let at = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .map(|entity| tile_of(&world, entity.id))
        .expect("a town centre");
    let yard = (at.0 + 3, at.1 + 3);
    assert!(free_ground(&world, yard), "the yard is walkable ground");
    let builder = villager(&mut world, (at.0 - 3, at.1 - 3));
    let house = world.role_unit("house").expect("house");

    let refused = world.submit_checked(Command::Build {
        villager: builder,
        unit: house,
        at: (
            u16::try_from(yard.0).expect("in range"),
            u16::try_from(yard.1).expect("in range"),
        ),
    });
    assert!(
        matches!(refused, Err(SimError::SiteOccupied { .. })),
        "a house inside the town centre's footprint must be refused as occupied ground: {refused:?}"
    );
}

/// A live farm's **whole floor** is the farm's, so a building cannot be raised on its corner - and an expired one is the
/// single exception the owner named, because replacing it is the point.
#[test]
fn a_live_farm_is_not_buildable_over_but_an_expired_one_is() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 12, 6) else {
        return;
    };
    let house = world.role_unit("house").expect("house");
    // Food in the plot: a farm with something left in it is in use.
    building(&mut world, 1, "farm", at, 600);
    let builder = villager(&mut world, (at.0 - 3, at.1 + 3));
    let corner = (at.0 + 2, at.1 + 2);
    let refused = world.submit_checked(Command::Build {
        villager: builder,
        unit: house,
        at: (
            u16::try_from(corner.0).expect("in range"),
            u16::try_from(corner.1).expect("in range"),
        ),
    });
    assert!(
        matches!(refused, Err(SimError::SiteOccupied { .. })),
        "a house on a live farm's floor must be refused: {refused:?}"
    );

    // The same plot with its food gone is one waiting to be replaced, which is the only overlap allowed. A farm cannot
    // be *spawned* spent - `Command::Spawn` fills a farm with its own food - so it is drained the way a player drains it.
    let dead = (at.0 + 6, at.1);
    let farm = building(&mut world, 1, "farm", dead, 0);
    let harvester = villager(&mut world, (dead.0 - 2, dead.1));
    world
        .submit_checked(Command::Gather {
            villager: harvester,
            node: farm,
        })
        .expect("a villager can be sent to a farm");
    let mut drained = false;
    for _ in 0..80_000 {
        world.step().expect("step");
        if world
            .entity(farm)
            .is_none_or(|entity| entity.amount.raw() <= 0)
        {
            drained = true;
            break;
        }
    }
    assert!(drained, "the plot never ran out of food");
    assert!(
        world
            .entity(farm)
            .is_some_and(|entity| entity.construction.is_none()),
        "the plot is finished and spent, which is what makes it replaceable"
    );
    let replaced = world.submit_checked(Command::Build {
        villager: builder,
        unit: house,
        at: (
            u16::try_from(dead.0 + 1).expect("in range"),
            u16::try_from(dead.1 + 1).expect("in range"),
        ),
    });
    assert!(
        replaced.is_ok(),
        "a house may be raised on a spent farm: {replaced:?}"
    );
}

/// **A walk past a house keeps the body's clearance** (M2, D113). The route used to bend at *tile origins* - the point where
/// four cells meet - so a villager rounding a house passed exactly through the corner of its footprint: distance zero, body
/// inside the wall. The owner saw that as "villagers went through the house". With the geometric route the closest approach
/// is the body's own radius, and this measures it every tick.
#[test]
fn a_walk_past_a_house_keeps_its_bodys_clearance() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 5, 5) else {
        return;
    };
    building(&mut world, 1, "house", at, 0);
    let house = footprint_tiles(&world, "house", at);
    let start = (at.0 - 4, at.1 + 1);
    let goal = (at.0 + 6, at.1 + 1);
    let walker = villager(&mut world, start);
    // The dat's radius for a villager, in raw units: the clearance the route has to keep.
    let body = world.body_radius_of(walker);
    assert!(
        body > 0,
        "the pack gives a villager no radius, so this test proves nothing"
    );

    world
        .submit_checked(Command::Move {
            unit: walker,
            to: (
                u16::try_from(goal.0).expect("in range"),
                u16::try_from(goal.1).expect("in range"),
            ),
        })
        .expect("a move past the house");

    let scale = world.scale();
    let mut closest = i64::MAX;
    for _ in 0..1_600 {
        world.step().expect("step");
        let position = world.position(walker).expect("alive");
        for (x, y) in &house {
            // Distance from the body's centre to that tile's square, in raw units; zero when inside it.
            let (x0, y0) = (x * scale, y * scale);
            let (dx, dy) = (
                (x0 - position.0.raw())
                    .max(0)
                    .max(position.0.raw() - (x0 + scale)),
                (y0 - position.1.raw())
                    .max(0)
                    .max(position.1.raw() - (y0 + scale)),
            );
            let squared = dx * dx + dy * dy;
            closest = closest.min(squared);
        }
    }
    // A fortieth of a tile of slack for the mover's own rounding; the route's clearance is the body's radius.
    let floor = (body - scale / 40).max(0);
    assert!(
        closest >= floor * floor,
        "the walker came within {} raw units of the house (a fortieth of a tile of slack over the body's {body})",
        (closest as f64).sqrt() as i64
    );
}

/// **Passage is a property of the ground, not of the body** (D117 - the owner's rule, 2026-11-04).
///
/// *"Either fully occupied tiles touch (even by corners) and there is no passage ever at all, or [they] don't, and then
/// there is passage for anyone. No matter the size of the enveloping circle or number of units."* So the planner's clearance
/// must **not** be the body's radius: a siege unit with a half-tile circle walks through a one-tile gap exactly as a
/// villager does, because a one-tile gap is a gap - and a formation that cannot fit squashes through rather than being
/// refused. The radius still governs how close two **bodies** stand to each other (`collide`), which is where AoE2 spends
/// it too.
#[test]
fn a_big_unit_walks_through_a_one_tile_gap() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 10, 6) else {
        return;
    };
    building(&mut world, 1, "house", at, 0);
    let width = i64::from(
        world
            .footprint_of(world.role_unit("house").expect("house"))
            .expect("a footprint")
            .0,
    );
    // A second house one tile east: the gap is exactly one tile, open north and south.
    building(&mut world, 1, "house", (at.0 + width + 1, at.1), 0);
    let gap = at.0 + width;
    let role = "trebuchet_packed";
    let scale = world.scale();
    let unit = EntityId(fresh_id(&world));
    let variant = world.role_unit(role).expect("the role");
    world
        .submit_checked(Command::Spawn {
            id: unit,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(variant),
            position: (
                Fixed::from_raw(gap * scale),
                Fixed::from_raw((at.1 - 3) * scale),
            ),
            speed: Fixed::parse("0.8", scale).expect("a walkable speed"),
            hp: 200,
            amount: 0,
        })
        .expect("the siege unit is placed");
    world.step().expect("step");
    let radius = world.body_radius_of(unit);
    assert!(
        radius >= scale / 3,
        "this test needs a body whose circle is big next to the gap: it is {radius}"
    );

    let goal = (gap, at.1 + 5);
    world
        .submit_checked(Command::Move {
            unit,
            to: (
                u16::try_from(goal.0).expect("in range"),
                u16::try_from(goal.1).expect("in range"),
            ),
        })
        .expect("a one-tile gap is a gap, whatever the circle");

    for _ in 0..2_400 {
        world.step().expect("step");
        if let Some(entity) = world.entity(unit) {
            let (x, y) = (entity.position.0.raw(), entity.position.1.raw());
            if y >= (goal.1 - 1) * scale && x / scale == gap {
                return;
            }
        }
    }
    panic!(
        "the big body never came through a one-tile gap: {:?}",
        world.entity(unit).map(|entity| entity.position)
    );
}
