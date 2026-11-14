//! **Right of way is the lower entity id, and the loser pauses 0.25-1.50 s on its own path** (D205).
//!
//! The original's collision block, read out of `AoK HD.exe` byte for byte (`docs/movement-in-the-binary.md`, last two
//! sections): when a step's collision list holds a unit that is not the order's target, the mover compares the two
//! units' handles and the **higher** one is told to pause - `rand()*1.25/32767+0.25`, i.e. a uniform 0.25 to 1.50
//! seconds - in `path+0x28`, which is the current path's timer and nothing else. The loser keeps its path, advances no
//! waypoint, does not move, and resumes where it left off; the winner (the lower id) repaths or stops.
//!
//! This file is written **before** that rule exists in the engine (D7, SPEC §11), and it pins the three pictures the
//! decision was accepted on:
//!
//! 1. two villagers ordered through each other head-on - the higher id stands still for a quarter to a second and a
//!    half, keeps its waypoint list, and then arrives; the lower id gets through;
//! 2. the owner's own picture - a loaded carrier walking a load home and a gatherer walking to a bush meeting on one
//!    tile, both reaching their destinations;
//! 3. two units jammed on the **same** tile, the case that froze seven villagers for a whole match on 2026-11-13 -
//!    neither may stand for ever.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// Open grass with the two towns far from the men this file orders about: the rule is about bodies, so the map must not
/// be the thing that decides it.
const OPEN: &str = r#"
schema = 1
[map]
width = 32
height = 20
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
]
[[map.town_center]]
player = 1
x = 2
y = 17
[[map.town_center]]
player = 2
x = 29
y = 2
"#;

/// The owner's picture, made into a place: one berry bush at (5,10) holding **one** unit of food - so the man who
/// works it is a **loaded carrier walking his load home** within a second or two - and a full one at (1,10) west of
/// him, which is where a **gatherer walks to a bush** from the east. Their two walks meet on the road between.
const CARRIER: &str = r#"
schema = 1
[map]
width = 32
height = 20
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "food"
amount = 1
tiles = [[5, 10]]
[[map.resource]]
kind = "food"
amount = 1000
tiles = [[1, 10]]
[[map.town_center]]
player = 1
x = 14
y = 13
[[map.town_center]]
player = 2
x = 29
y = 2
"#;

struct Pack {
    settings: Settings,
    economy: Economy,
    units: Units,
    resources: ruage_data::Resources,
}

fn pack() -> Option<Pack> {
    let units_file = repo("config/gamedata/units.toml");
    if !units_file.exists() {
        return None;
    }
    Some(Pack {
        settings: common::present(&repo("config/sim.toml"), ruage_data::load_sim_file),
        economy: common::present(&repo("config/economy.toml"), ruage_data::load_economy_file),
        units: common::must("load_units_file", ruage_data::load_units_file(&units_file)),
        resources: common::present(
            &repo("config/resources.toml"),
            ruage_data::load_resources_file,
        ),
    })
}

fn world_of(pack: &Pack, map_text: &str) -> World {
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(map_text, "right_of_way.toml", &pack.resources),
    );
    let mut world = common::must("World::new", World::new(&pack.settings, 7));
    common::must(
        "seed_from",
        world.seed_from(&map, &pack.economy, &pack.units),
    );
    common::must("load_resources", world.load_resources(&pack.resources));
    world
}

/// A villager of player 1 at a raw point, stepped in so the spawn lands.
fn spawn(pack: &Pack, world: &mut World, id: u32, at: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let villager = world
        .role_unit("villager")
        .expect("the pack has a villager");
    let speed = pack
        .units
        .unit(villager)
        .expect("VMLUM")
        .speed
        .resolve(scale, "speed")
        .expect("a walker");
    let id = EntityId(id);
    common::must(
        "a spawn",
        world.submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (Fixed::from_raw(at.0), Fixed::from_raw(at.1)),
            speed,
            hp: 25,
            amount: 0,
        }),
    );
    world.step().expect("the spawn lands");
    id
}

/// A tile's centre in raw units.
fn centre(scale: i64, tile: (i64, i64)) -> (i64, i64) {
    (tile.0 * scale + scale / 2, tile.1 * scale + scale / 2)
}

fn at(world: &World, id: EntityId) -> (i64, i64) {
    let position = world.position(id).expect("the unit is alive");
    (position.0.raw(), position.1.raw())
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let scale = world.scale();
    let (x, y) = at(world, id);
    (x.div_euclid(scale), y.div_euclid(scale))
}

fn near(world: &World, id: EntityId, point: (i64, i64)) -> bool {
    let scale = world.scale();
    let (x, y) = at(world, id);
    let (dx, dy) = (x - point.0, y - point.1);
    dx * dx + dy * dy <= (scale + scale / 4) * (scale + scale / 4)
}

/// A raw position: `(x, y)` in engine units.
type Point = (i64, i64);

/// One body's remaining waypoint list.
type Route = Vec<(Fixed, Fixed)>;

/// One crossing, walked to its end: the higher id's tick-by-tick record of `(position, waypoints left)`, whether both
/// men arrived, how many ticks it took, the closest the pair ever came (squared raw units), and the reach their two
/// footprints need.
struct Crossing {
    /// `(where the higher id stood, its remaining waypoint list)` per tick.
    trail: Vec<(Point, Route)>,
    /// Where the lower id stood, per tick.
    lower: Vec<Point>,
    arrived: bool,
    ticks: i64,
    closest: i64,
    reach: i64,
}

/// **One crossing**: the two given men are ordered to each other's place and walked until both arrive (or 2,500 ticks,
/// twenty times the longest pause).
fn cross(
    pack: &Pack,
    lower_at: (i64, i64),
    higher_at: (i64, i64),
    goal_low: (i64, i64),
    goal_high: (i64, i64),
) -> Crossing {
    let mut world = world_of(pack, OPEN);
    let scale = world.scale();
    let lower = spawn(pack, &mut world, 9_000, lower_at);
    let higher = spawn(pack, &mut world, 9_001, higher_at);
    let tile = |point: (i64, i64)| {
        (
            u16::try_from(point.0.div_euclid(scale)).expect("a tile x"),
            u16::try_from(point.1.div_euclid(scale)).expect("a tile y"),
        )
    };
    common::must(
        "a move order",
        world.submit_checked(Command::Move {
            unit: lower,
            to: tile(goal_low),
        }),
    );
    common::must(
        "a move order",
        world.submit_checked(Command::Move {
            unit: higher,
            to: tile(goal_high),
        }),
    );
    let reach = world.reach_between(lower, higher).raw();

    let mut trail = Vec::new();
    let mut lower_trail = Vec::new();
    let mut closest = i64::MAX;
    let mut arrived = false;
    let mut ticks = 0;
    for tick in 0..2_500i64 {
        world.step().expect("a tick");
        trail.push((at(&world, higher), world.route_points(higher)));
        lower_trail.push(at(&world, lower));
        let (one, two) = (at(&world, higher), at(&world, lower));
        closest = closest.min((one.0 - two.0).pow(2) + (one.1 - two.1).pow(2));
        ticks = tick;
        if near(&world, lower, goal_low) && near(&world, higher, goal_high) {
            arrived = true;
            break;
        }
    }
    Crossing {
        trail,
        lower: lower_trail,
        arrived,
        ticks,
        closest,
        reach,
    }
}

/// The **first** run of consecutive ticks in which the higher id did not move, as `(ticks, first index)`.
///
/// The pause the original sets is a *single* draw of 0.25-1.50 s, and a man who is paused, resumes into the same body and is
/// paused again has stood for longer than one draw without the rule being broken - each fresh collision re-rolls the timer
/// (`0x8aa6d0` overwrites the path's float). So the bound is asserted on **one pause**, not on the longest stand of the match,
/// and a genuine freeze is caught by arrival instead.
fn first_still(trail: &[(Point, Route)]) -> (i64, usize) {
    let mut run = 1i64;
    for i in 1..trail.len() {
        if trail[i].0 == trail[i - 1].0 {
            run += 1;
            if run >= 2 {
                return (run, i + 1 - usize::try_from(run).expect("a run longer than a usize"));
            }
        } else {
            run = 1;
        }
    }
    (1, 0)
}

/// The longest run of consecutive ticks in which the higher id did not move, as `(ticks, first index)`.
fn longest_still(trail: &[(Point, Route)]) -> (i64, usize) {
    let mut best = (1i64, 0usize);
    let mut run = 1i64;
    for i in 1..trail.len() {
        if trail[i].0 == trail[i - 1].0 {
            run += 1;
            if run > best.0 {
                best = (
                    run,
                    i + 1 - usize::try_from(run).expect("a run longer than a usize"),
                );
            }
        } else {
            run = 1;
        }
    }
    best
}

/// **Two villagers ordered to pass through each other head-on.** The higher id stands still for 0.25 to 1.50 s, its
/// waypoint list is unchanged while it stands, and it then arrives; the lower id gets through. This is the original's
/// tie-break and nothing else - a give-way, a slide or a nudge all fail at least one of the three.
#[test]
fn the_higher_id_pauses_for_a_quarter_to_one_and_a_half_seconds_and_keeps_its_route() {
    let Some(pack) = pack() else { return };
    let probe = world_of(&pack, OPEN);
    let scale = probe.scale();
    let tick_rate = i64::from(probe.tick_rate_hz());
    let shortest = tick_rate / 4; // 0.25 s
    let longest = tick_rate * 3 / 2; // 1.50 s

    let crossing = cross(
        &pack,
        centre(scale, (8, 8)),
        centre(scale, (14, 8)),
        centre(scale, (14, 8)),
        centre(scale, (8, 8)),
    );

    let (still, start) = longest_still(&crossing.trail);
    assert!(
        still >= shortest,
        "MEASURED the higher id stood still for {still} ticks at the longest ({:.3} s): the loser has to pause for at \
         least 0.25 s",
        still as f64 / tick_rate as f64
    );
    // **One pause is one draw**, and that is what is bounded. A man paused, resumed into the same body and paused again has
    // stood for longer than one draw - correctly - so the bound is asserted on the first stand and the freeze is caught by
    // arrival below.
    let (first, _) = first_still(&crossing.trail);
    assert!(
        first <= longest,
        "MEASURED the higher id's first pause lasted {first} ticks ({:.3} s): a single pause is bounded at 1.50 s by the \
         original's own draw",
        first as f64 / tick_rate as f64
    );

    // **It keeps its route.** No waypoint may be spent while the timer runs: the pause writes the path's timer and
    // nothing else (`0x8aa6d0`), so the waypoint list before, during and after the stand is the same list.
    let before = crossing.trail[start.saturating_sub(1)].1.clone();
    let during: Vec<Route> = crossing.trail[start..start + still as usize]
        .iter()
        .map(|(_, left)| left.clone())
        .collect();
    assert!(
        during.iter().all(|left| *left == before),
        "MEASURED the paused unit's waypoint list changed while it stood still: {} before, {:?} during",
        before.len(),
        during.iter().map(std::vec::Vec::len).collect::<Vec<_>>()
    );

    // **The winner proceeds.** During the loser's stand the lower id's position must change: if both stand, the pair is
    // jammed and the rule has not done its job.
    let moved =
        (start..start + still as usize).any(|i| crossing.lower.get(i + 1) != crossing.lower.get(i));
    assert!(
        moved,
        "MEASURED the lower id did not move while the higher id was paused: both stood still, which is the freeze D205 \
         exists to remove"
    );
    assert!(
        crossing.arrived,
        "MEASURED the crossing never finished in {} ticks: the lower id has to get through and the higher id to resume",
        crossing.ticks
    );
}

/// **The owner's own picture.** A loaded carrier walking his load home and a gatherer walking to a bush meet on one
/// tile. Both reach their destinations: the load is banked and the gatherer works his bush.
#[test]
fn a_loaded_carrier_and_a_gatherer_meeting_on_one_tile_both_arrive() {
    let Some(pack) = pack() else { return };
    let mut world = world_of(&pack, CARRIER);
    let scale = world.scale();
    let food = world.slot_of("food").expect("a food slot");
    let opening = world.stock(1, food);

    // The carrier: a bush holding one unit of food, so the first take empties it and the man turns for home with it.
    let carrier = spawn(&pack, &mut world, 9_000, centre(scale, (7, 10)));
    let near_bush = world
        .entities()
        .iter()
        .find(|entity| world.resource_tiles(entity.id).first() == Some(&(5, 10)))
        .map(|entity| entity.id)
        .expect("the small bush");
    let far_bush = world
        .entities()
        .iter()
        .find(|entity| world.resource_tiles(entity.id).first() == Some(&(1, 10)))
        .map(|entity| entity.id)
        .expect("the full bush");
    common::must(
        "a gather order",
        world.submit_checked(Command::Gather {
            villager: carrier,
            node: near_bush,
        }),
    );

    // Walk him until he is carrying and walking home - that is the moment the picture is about.
    let mut loaded = false;
    for _ in 0..1_200 {
        world.step().expect("a tick");
        let entity = world.entity(carrier).expect("the carrier is alive");
        if entity.carry.raw() > 0 && matches!(entity.task, Task::Return { .. }) {
            loaded = true;
            break;
        }
    }
    assert!(
        loaded,
        "PRECONDITION: the carrier never took a load, so this fixture is not the picture it claims to be (task {:?})",
        world.entity(carrier).map(|entity| entity.task)
    );

    // The gatherer, east of the carrier and walking west at the full bush: their two walks cross on the road.
    let gatherer = spawn(&pack, &mut world, 9_500, centre(scale, (10, 10)));
    common::must(
        "a gather order",
        world.submit_checked(Command::Gather {
            villager: gatherer,
            node: far_bush,
        }),
    );

    let mut banked = false;
    let mut worked = false;
    let mut met = false;
    for _ in 0..3_000 {
        world.step().expect("a tick");
        if !banked && world.stock(1, food) > opening {
            banked = true;
        }
        if world.entity(gatherer).is_some_and(
            |entity| matches!(entity.task, Task::Gather { progress, .. } if progress.raw() > 0),
        ) {
            worked = true;
        }
        let (one, two) = (at(&world, carrier), at(&world, gatherer));
        let reach = world.reach_between(carrier, gatherer).raw() + reach_slack(&world);
        if (one.0 - two.0).pow(2) + (one.1 - two.1).pow(2) < reach * reach {
            met = true;
        }
        if banked && worked {
            break;
        }
    }
    assert!(
        met,
        "PRECONDITION: the two men never came near each other, so their walks did not meet on a tile"
    );
    assert!(
        banked,
        "MEASURED the loaded carrier never banked his load: he still carries {} at {:?}",
        world.carry(carrier).raw(),
        tile_of(&world, carrier)
    );
    assert!(
        worked,
        "MEASURED the gatherer never took from the bush: his task is {:?} at {:?}",
        world.entity(gatherer).map(|entity| entity.task),
        tile_of(&world, gatherer)
    );
}

/// A few tenths of a tile: close enough to call two bodies "on one tile" for the fixture's own precondition.
fn reach_slack(world: &World) -> i64 {
    world.scale() / 2
}

/// **Two units jammed on the same tile** - the geometry that froze seven villagers for a whole match on 2026-11-13
/// (`WaitOwnTile` from tick 40,000 to the end, the acceptance bank at 0 food). Each wants the other's side, so neither
/// can be answered by a step that has room; under the new rule the higher id pauses for at most 1.50 s, the lower id
/// repaths, and **neither may stand for ever**: both finish the crossing, and neither is ever inside the other.
#[test]
fn two_units_jammed_on_the_same_tile_neither_stands_for_ever() {
    let Some(pack) = pack() else { return };
    let probe = world_of(&pack, OPEN);
    let scale = probe.scale();
    let tick_rate = i64::from(probe.tick_rate_hz());
    let longest = tick_rate * 3 / 2;

    // One tile, a hand's width apart: 10.30 and 10.75 tiles along x, both at y = 10.5. That is 0.45 tiles - past the
    // 0.40 two bodies need - so the pair starts *not* overlapping and the only way past each other is the rule tested.
    let crossing = cross(
        &pack,
        (10 * scale + 3 * scale / 10, 10 * scale + scale / 2),
        (10 * scale + 3 * scale / 4, 10 * scale + scale / 2),
        centre(scale, (14, 10)),
        centre(scale, (6, 10)),
    );

    let (still, _) = longest_still(&crossing.trail);
    assert!(
        still >= tick_rate / 4,
        "MEASURED a unit jammed on one tile stood still for {still} ticks at the longest ({:.3} s): the loser of a \
         collision pauses for at least 0.25 s, and this is the geometry the 2026-11-13 freeze was made of",
        still as f64 / tick_rate as f64
    );
    let (first, _) = first_still(&crossing.trail);
    assert!(
        first <= longest,
        "MEASURED the first pause on one tile lasted {first} ticks ({:.3} s): one pause is bounded at 1.50 s, and a \
         longer *total* stand with arrivals between the pauses is the game's own behaviour, not a freeze",
        first as f64 / tick_rate as f64
    );
    // **Nobody is inside anybody.** The pair may share a tile - the game does - but the two footprints may not overlap:
    // a lower id that simply stepped into the other man would show up here as a distance below the reach.
    assert!(
        crossing.closest >= crossing.reach * crossing.reach,
        "MEASURED the pair came {} raw units from each other where the two footprints need {}",
        (crossing.closest as f64).sqrt() as i64,
        crossing.reach
    );
    assert!(
        crossing.arrived,
        "MEASURED the two men jammed on one tile never finished the crossing in {} ticks: neither can stand for ever",
        crossing.ticks
    );
}
