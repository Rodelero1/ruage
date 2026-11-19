//! **The owner's two clicks, and the one distance the game answers both with** (D286).
//!
//! Written before the implementation, from his report (2026-11-19): *"units can no longer approach to the building to the
//! distance of 1/2 radius when I click to the middle of the building, but will happily approach building to distance ~0
//! radius (center of unit base right beside the building border) when I right click right next to the border."*
//!
//! In the original there is one ruler for both: `func_002690E0` subtracts both bodies' sizes from each axis and clamps at
//! zero, so arrival is **box to box**, and the collision query (`FUN_00840830`) grows the mover's box by its radius and
//! collides on touching. A move order carries the object it is about (`"That collision is with our target, so we're done"`),
//! so a click on a house and a click a hand's width beside one end the same place: the body's box against the wall.
//!
//! Both clicks are measured here, in one scene, against one house - the two fixtures that agree are the whole point.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

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

fn fresh_id(world: &World) -> u32 {
    world
        .entities()
        .iter()
        .map(|entity| entity.id.0)
        .max()
        .unwrap_or(0)
        + 1
}

fn free_ground(world: &World, tile: (i64, i64)) -> bool {
    u16::try_from(tile.0)
        .ok()
        .zip(u16::try_from(tile.1).ok())
        .is_some_and(|(x, y)| world.tile_walkable(x, y))
}

/// An empty rectangle of the project map with clear ground **all round it**: the two men are spawned three tiles west and
/// six east of the footprint, on the row below its top, so the whole band `x - 4 ..= x + width + 2` by `y ..= y + height`
/// has to be open or the fixture would be measuring the map's furniture.
fn open_patch(world: &World, width: i64, height: i64) -> Option<(i64, i64)> {
    for y in 26..52i64 {
        for x in 8..44i64 {
            let clear = (0..height + 1)
                .all(|dy| (-4..width + 3).all(|dx| free_ground(world, (x + dx, y + dy))));
            if clear {
                return Some((x, y));
            }
        }
    }
    None
}

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

fn villager(world: &mut World, tile: (i64, i64), offset: (i64, i64)) -> EntityId {
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
                Fixed::from_raw(tile.0 * scale + offset.0),
                Fixed::from_raw(tile.1 * scale + offset.1),
            ),
            speed: Fixed::parse("0.8", scale).expect("a walkable speed"),
            hp: 25,
            amount: 0,
        })
        .expect("spawn villager");
    world.step().expect("step");
    id
}

/// A building's rectangle in raw units: `(left, top, right, bottom)`, from its own footprint.
fn rect_of(world: &World, id: EntityId) -> (i64, i64, i64, i64) {
    let scale = world.scale();
    let entity = world.entity(id).expect("the building");
    let unit = entity.unit.expect("a building has a variant");
    let (width, height) = world.footprint_of(unit).expect("a footprint");
    let (ax, ay) = (
        entity.position.0.raw() / scale,
        entity.position.1.raw() / scale,
    );
    (
        ax * scale,
        ay * scale,
        (ax + i64::from(width)) * scale,
        (ay + i64::from(height)) * scale,
    )
}

/// **How far a body's *centre* is from a rectangle's border**, in raw units: `max(0, |dx| - half)` per axis, then the
/// hypotenuse. Zero means the centre is on the wall, which is the owner's `~0 radius`; the game's own answer is that this
/// number is the body's **radius** - the boxes touch and cannot overlap.
fn centre_gap(world: &World, id: EntityId, rect: (i64, i64, i64, i64)) -> i64 {
    let entity = world.entity(id).expect("the body");
    let (x, y) = (entity.position.0.raw(), entity.position.1.raw());
    let dx = (rect.0 - x).max(x - rect.2).max(0);
    let dy = (rect.1 - y).max(y - rect.3).max(0);
    ((dx * dx + dy * dy) as f64).sqrt() as i64
}

/// A walk order at a point, naming the object the click landed on - the game's own shape for an order (D286). A click on
/// empty ground names nothing.
fn walk_to(world: &mut World, unit: EntityId, to: (Fixed, Fixed), target: Option<EntityId>) {
    world
        .submit_checked(Command::WalkTo { unit, to, target })
        .expect("a walk order");
}

/// **The two clicks, measured against one house.**
#[test]
fn a_click_on_a_house_and_a_click_beside_it_end_at_the_same_distance() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 4, 4) else {
        return;
    };
    let house = building(&mut world, 1, "house", at, 0);
    let rect = rect_of(&world, house);
    let scale = world.scale();
    // Two men: one west of the house, one east. The west man clicks **the middle of the house**; the east man clicks a
    // point a twentieth of a tile clear of its east wall - the owner's two clicks, side by side.
    let west = villager(&mut world, (at.0 - 3, at.1 + 1), (scale / 2, scale / 2));
    let east = villager(&mut world, (at.0 + 6, at.1 + 1), (scale / 2, scale / 2));
    let middle = (
        Fixed::from_raw((rect.0 + rect.2) / 2),
        Fixed::from_raw((rect.1 + rect.3) / 2),
    );
    let beside = (
        Fixed::from_raw(rect.2 + scale / 20),
        Fixed::from_raw((rect.1 + rect.3) / 2),
    );
    walk_to(&mut world, west, middle, Some(house));
    walk_to(&mut world, east, beside, None);

    let radius = world.body_radius_of(west);
    let step = world
        .entity(west)
        .map(|entity| entity.speed.raw() / i64::from(world.tick_rate_hz()))
        .expect("a speed");
    assert!(step > 0, "the fixture needs a moving body");

    // **Both men are done well before this**: the west one ends on contact and the east one when the ground refuses it for
    // forty ticks. The loop still runs a while, because the invariant it holds is *per tick* - nobody's box ever enters the
    // wall - and stopping early would leave the last few ticks unwatched.
    for tick in 0..2_400 {
        world.step().expect("step");
        for (which, id) in [
            ("the click on the house", west),
            ("the click beside it", east),
        ] {
            let gap = centre_gap(&world, id, rect);
            assert!(
                gap >= radius,
                "{which}: the body reached {gap} raw from the wall at tick {tick}, inside its own radius {radius}"
            );
        }
    }
    let on_the_house = centre_gap(&world, west, rect);
    let beside_it = centre_gap(&world, east, rect);
    assert!(
        (radius..=radius + step).contains(&on_the_house),
        "the click on the house stopped {on_the_house} raw from the wall: the body's radius is {radius}, one step {step}"
    );
    assert!(
        (radius..=radius + step).contains(&beside_it),
        "the click beside the house stopped {beside_it} raw from the wall: the body's radius is {radius}, one step {step}"
    );
    assert!(
        (on_the_house - beside_it).abs() <= step,
        "the two clicks disagree: {on_the_house} raw against the middle, {beside_it} beside it"
    );
}

/// **And from the east**, where the step runs *against* the axis it cuts on.
///
/// Written after the review found the axis arithmetic could carry a negative denominator into the cut's comparison: a body
/// walking west or north into its target would then be cut to **zero movement** and stand still, while the same walk
/// eastwards worked. The two are one rule or they are not a rule.
#[test]
fn a_click_on_a_house_from_the_east_stops_at_the_same_distance() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 4, 4) else {
        return;
    };
    let house = building(&mut world, 1, "house", at, 0);
    let rect = rect_of(&world, house);
    let scale = world.scale();
    let east = villager(&mut world, (at.0 + 6, at.1 + 1), (scale / 2, scale / 2));
    let middle = (
        Fixed::from_raw((rect.0 + rect.2) / 2),
        Fixed::from_raw((rect.1 + rect.3) / 2),
    );
    walk_to(&mut world, east, middle, Some(house));

    let radius = world.body_radius_of(east);
    let step = world
        .entity(east)
        .map(|entity| entity.speed.raw() / i64::from(world.tick_rate_hz()))
        .expect("a speed")
        .max(1);
    let mut ended = None;
    for tick in 0..u64::try_from(10 * scale / step).unwrap_or(600) + 60 {
        world.step().expect("step");
        if world
            .entity(east)
            .is_some_and(|entity| matches!(entity.task, ruage_sim::Task::Idle))
        {
            ended = Some(tick);
            break;
        }
    }
    ended.expect("the walk from the east never ended: it is standing short of the wall");
    let gap = centre_gap(&world, east, rect);
    assert!(
        (radius..=radius + step).contains(&gap),
        "the click from the east stopped {gap} raw from the wall - the radius is {radius}, one step {step}"
    );
}

/// **The order names the house, and touching it ends the order** - not a forty-tick stall against something in the way.
///
/// The original's collision block: *"That collision is with our target, so we're done"* (`0x127d`). Our order carried only a
/// place, so that arm could never fire for a walk and the man stood at the wall until `finish_blocked_orders` gave up.
#[test]
fn a_walk_that_names_a_house_ends_when_it_touches_it() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let Some(at) = open_patch(&world, 4, 4) else {
        return;
    };
    let house = building(&mut world, 1, "house", at, 0);
    let rect = rect_of(&world, house);
    let scale = world.scale();
    let walker = villager(&mut world, (at.0 - 3, at.1 + 1), (scale / 2, scale / 2));
    let middle = (
        Fixed::from_raw((rect.0 + rect.2) / 2),
        Fixed::from_raw((rect.1 + rect.3) / 2),
    );
    walk_to(&mut world, walker, middle, Some(house));

    // **The deadline is derived, not chosen**: ten tiles of travel at this body's own step, plus a margin for the pause the
    // crowd rule may draw. A literal tick count would silently stop measuring anything if the tick rate or the speed moved -
    // `config/sim.toml` already contemplates 60 Hz.
    let step = world
        .entity(walker)
        .map(|entity| entity.speed.raw() / i64::from(world.tick_rate_hz()))
        .expect("a speed")
        .max(1);
    let deadline = u64::try_from(10 * scale / step).unwrap_or(600) + 60;
    let mut ended = None;
    for tick in 0..deadline {
        world.step().expect("step");
        if world
            .entity(walker)
            .is_some_and(|entity| matches!(entity.task, ruage_sim::Task::Idle))
        {
            ended = Some(tick);
            break;
        }
    }
    let ended =
        ended.expect("the walk never ended: it is still trying to reach the middle of the house");
    let gap = centre_gap(&world, walker, rect);
    let radius = world.body_radius_of(walker);
    let step = world
        .entity(walker)
        .map(|entity| entity.speed.raw() / i64::from(world.tick_rate_hz()))
        .unwrap_or(0);
    assert!(
        (radius..=radius + step).contains(&gap),
        "it ended at tick {ended}, {gap} raw from the wall - the radius is {radius}, one step {step}"
    );
}
