//! **D183: a foundation is walkable, invisible and free of cost until a villager touches it.**
//!
//! The owner's model, after his scout was pushed into the forest by a lumber camp placed on him:
//!
//! > "When you place a building on the ground, it is still walkable over (should be light grey colour like finished
//! > farm or part of town center) and you and your allies see it. If any of your or allied units stand over it, they
//! > will leave the area by the shortest route. Then villager can start construction. Enemy units don't see the
//! > foundation until villager started working on it and will not leave area. As soon as the villager starts building,
//! > it will turn dark grey (except farm) and start blocking the tiles."
//!
//! Supersedes D152 (a foundation blocked from insertion). Every test here is written before the code it pins (D7,
//! SPEC §11); the RED run is quoted in the session that landed each one.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// Grass everywhere, a town centre for each side, and one tree beside player 1's base. Nothing else: every rule in this
/// file is about the ground under a building, and the tree is here for the one test that asks what a unit goes back to.
const MAP: &str = r#"
schema = 1
[map]
width = 18
height = 14
terrain = [
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGFGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 100
hp = 20
tiles = [[5,8]]
[[map.town_center]]
player = 1
x = 2
y = 11
[[map.town_center]]
player = 2
x = 15
y = 1
"#;

fn world() -> Option<World> {
    let units_file = repo("config/gamedata/units.toml");
    if !units_file.exists() {
        return None;
    }
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(MAP, "foundation_model.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

/// A free id: `start_build_site` numbers its foundation `max + 1`, so a literal collides with one placed earlier.
fn fresh_id(world: &World) -> u32 {
    world
        .entities()
        .iter()
        .map(|entity| entity.id.0)
        .max()
        .unwrap_or(0)
        + 1
}

/// A unit standing on the **centre of its tile**, which is where a walk's goal is (D114) and therefore the only place a
/// distance can be read without half a tile of fixture arithmetic in the way.
fn unit(world: &mut World, player: u8, role: &str, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let id = EntityId(fresh_id(world));
    let unit = world.role_unit(role).unwrap_or_else(|| panic!("{role}"));
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale + scale / 2),
                Fixed::from_raw(tile.1 * scale + scale / 2),
            ),
            speed: Fixed::parse("1.2", scale).expect("a walkable speed"),
            hp: 25,
            amount: 0,
        })
        .expect("spawn unit");
    world.step().expect("step");
    id
}

/// A **finished** building, standing where it is put.
fn finished(world: &mut World, player: u8, role: &str, tile: (i64, i64)) -> EntityId {
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
            amount: 0,
        })
        .expect("spawn building");
    world.step().expect("step");
    id
}

/// **Look at a piece of ground and then look away**: a villager of `player` stands there for one tick and is deleted, so
/// the tiles around it are remembered (`TileVision::Explored`) rather than watched. A foundation may be placed in fog, so
/// every test that builds near the middle of the map has to have looked there first (D183 addendum).
fn explore(world: &mut World, player: u8, tile: (i64, i64)) {
    let id = unit(world, player, "villager", tile);
    world
        .submit_checked(Command::Delete { player, entity: id })
        .expect("the explorer is deleted");
    world.step().expect("step");
}

fn role(world: &World, name: &str) -> u32 {
    world.role_unit(name).unwrap_or_else(|| panic!("{name}"))
}

fn site_at(world: &World, at: (u16, u16)) -> Option<EntityId> {
    let scale = world.scale();
    let position = (
        Fixed::from_raw(i64::from(at.0) * scale),
        Fixed::from_raw(i64::from(at.1) * scale),
    );
    world
        .entities()
        .iter()
        .find(|entity| {
            entity.kind == EntityKind::Building
                && entity.position == position
                && entity.construction.is_some()
        })
        .map(|entity| entity.id)
}

fn progress(world: &World, site: EntityId) -> Option<u32> {
    world
        .entity(site)
        .and_then(|entity| entity.construction)
        .map(|construction| construction.progress_ticks)
}

/// **A house foundation is walkable ground until somebody works on it.**
///
/// The owner's rule, and the whole of D183 §1: the ground a fresh foundation covers stays walkable - the ghost is a
/// drawing, not an obstacle - and the finished building beside it is not.
#[test]
fn a_foundation_nobody_has_touched_is_walkable_ground() {
    let Some(mut world) = world() else { return };
    explore(&mut world, 1, (9, 7));
    let builder = unit(&mut world, 1, "villager", (5, 6));
    let house = role(&world, "house");
    let done = finished(&mut world, 1, "house", (11, 6));

    // The control first: a building that has been raised blocks its whole footprint (D76).
    for tile in [(11u16, 6u16), (12, 6), (11, 7), (12, 7)] {
        assert!(
            !world.tile_walkable(tile.0, tile.1),
            "the finished house does not block {tile:?}, so this test cannot see a foundation differ from a house"
        );
    }

    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: house,
            at: (8, 6),
        })
        .expect("a house foundation");
    world.step().expect("step");
    let site = site_at(&world, (8, 6)).expect("the foundation stands where it was placed");
    assert_eq!(progress(&world, site), Some(0), "the site is untouched");
    let _ = done;

    for tile in [(8u16, 6u16), (9, 6), (8, 7), (9, 7)] {
        assert!(
            world.tile_walkable(tile.0, tile.1),
            "a foundation nobody has touched blocks {tile:?}: the ground it covers is not walkable"
        );
    }
}

/// **Construction cannot start while anybody stands on the site** - and it says so once, not once a tick.
///
/// D183 §4: every player's units and animals hold the start up; an enemy has no reason to leave, so an enemy standing on
/// a site stalls it until it moves or dies. That is accepted as a tactic, and it is the only case this test can pin
/// deterministically, because an ally is *told* to leave and would walk off by itself.
#[test]
fn construction_waits_while_a_unit_stands_on_the_site() {
    let Some(mut world) = world() else { return };
    explore(&mut world, 1, (9, 7));
    // Far enough that the enemy is on the site long before the builder arrives.
    let builder = unit(&mut world, 1, "villager", (2, 9));
    let squatter = unit(&mut world, 2, "villager", (10, 6));
    let house = role(&world, "house");
    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: house,
            at: (8, 6),
        })
        .expect("a house foundation");
    world.step().expect("the order lands");
    let site = site_at(&world, (8, 6)).expect("the foundation");
    // The enemy walks onto the ghost - which is walkable ground, so the order is an ordinary move.
    world
        .submit_checked(Command::Move {
            unit: squatter,
            to: (9, 6),
        })
        .expect("an enemy walks onto the site");

    // Long enough for both: the enemy arrives in ~20 ticks, the builder in ~120.
    for _ in 0..200 {
        world.step().expect("step");
    }
    assert!(
        world.tile_of_position(
            world
                .position(squatter)
                .expect("the squatter is in the world")
        ) == (9, 6),
        "the test's own premise is wrong: the enemy never reached the site"
    );
    assert_eq!(
        progress(&world, site),
        Some(0),
        "construction started while an enemy stood on the site - the site was not empty"
    );
    let reported = world
        .last_refusal()
        .map(|(_, why, at)| (why.to_string(), at))
        .expect("a stalled site must say why it is stalled");
    assert!(
        reported.0.contains("site"),
        "the refusal does not name the site: {}",
        reported.0
    );

    // **Once per state change**: two hundred more blocked ticks must not restamp the same message.
    for _ in 0..200 {
        world.step().expect("step");
    }
    assert_eq!(
        world.last_refusal().map(|(_, _, at)| at),
        Some(reported.1),
        "the same stalled site was reported again on a later tick: the log would fill with one line per tick"
    );
    assert_eq!(
        progress(&world, site),
        Some(0),
        "construction advanced anyway"
    );

    // The site is clear: the first stroke lands, and the ground turns solid.
    world
        .submit_checked(Command::Delete {
            player: 2,
            entity: squatter,
        })
        .expect("the enemy is deleted");
    for _ in 0..20 {
        world.step().expect("step");
    }
    assert!(
        progress(&world, site).is_some_and(|ticks| ticks > 0),
        "construction did not start once the site was empty"
    );
}

/// **The first stroke turns the site solid, and nobody is ever caught inside it.**
///
/// D183 §5: the ground is walkable while the site is a ghost and blocks from the tick construction starts. The safety
/// argument is by construction - the site was empty the tick before - so the invariant this test pins, "no unit stands
/// inside a solid footprint", is the one the whole model rests on.
#[test]
fn the_first_stroke_turns_the_ground_solid() {
    let Some(mut world) = world() else { return };
    explore(&mut world, 1, (9, 7));
    let builder = unit(&mut world, 1, "villager", (4, 6));
    let house = role(&world, "house");
    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: house,
            at: (8, 6),
        })
        .expect("a house foundation");
    world.step().expect("the order lands");
    let site = site_at(&world, (8, 6)).expect("the foundation");
    let footprint = [(8u16, 6u16), (9, 6), (8, 7), (9, 7)];
    for tile in footprint {
        assert!(
            world.tile_walkable(tile.0, tile.1),
            "the site blocked {tile:?} before anybody worked on it"
        );
    }

    let mut started = None;
    for tick in 0..600 {
        world.step().expect("step");
        if progress(&world, site).is_some_and(|ticks| ticks > 0) {
            started = Some(tick);
            break;
        }
    }
    let started = started.expect("construction never started on an empty site");

    for tile in footprint {
        assert!(
            !world.tile_walkable(tile.0, tile.1),
            "{tile:?} is walkable on tick {started}, the tick construction started: the first stroke did not take the ground"
        );
    }
    // **Nobody inside a solid footprint**, anywhere: every living unit must be standing on walkable ground.
    for entity in world.entities() {
        if entity.kind != EntityKind::Unit || entity.hp <= 0 {
            continue;
        }
        let tile = world.tile_of_position(entity.position);
        assert!(
            world.tile_walkable(tile.0, tile.1),
            "unit {} stands on {tile:?}, which is inside a solid footprint, on tick {started}",
            entity.id.0
        );
    }
}

/// **A cancelled site gives its ground back.** The block is taken once, at the first stroke, and the death sweep frees every
/// solid tile of a dead building - so the ground returns exactly once. If it did not, a cancelled house would leave an
/// invisible wall standing for the rest of the match.
#[test]
fn a_cancelled_site_gives_its_ground_back() {
    let Some(mut world) = world() else { return };
    explore(&mut world, 1, (9, 7));
    let builder = unit(&mut world, 1, "villager", (4, 6));
    let house = role(&world, "house");
    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: house,
            at: (8, 6),
        })
        .expect("a house foundation");
    world.step().expect("the order lands");
    let site = site_at(&world, (8, 6)).expect("the foundation");
    let footprint = [(8u16, 6u16), (9, 6), (8, 7), (9, 7)];

    let mut started = false;
    for _ in 0..600 {
        world.step().expect("step");
        if progress(&world, site).is_some_and(|ticks| ticks > 0) {
            started = true;
            break;
        }
    }
    assert!(started, "construction never started");
    for tile in footprint {
        assert!(
            !world.tile_walkable(tile.0, tile.1),
            "{tile:?} is still walkable after the first stroke"
        );
    }

    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: site,
        })
        .expect("the owner cancels the site");
    for _ in 0..4 {
        world.step().expect("step");
    }
    assert!(
        world.entity(site).is_none(),
        "the cancelled site is still there: {:?}",
        world.last_refusal().map(|(_, why, _)| why.to_string())
    );
    for tile in footprint {
        assert!(
            world.tile_walkable(tile.0, tile.1),
            "{tile:?} is still solid after the site was cancelled: the ground was never given back"
        );
    }
}

/// **A unit standing where a foundation is placed walks off it; it is never placed somewhere else.**
///
/// D183 §1, and the owner's own reproducer: *"villager placed there a lumber camp colliding with his position, that pushed
/// scout to the forest."* The old engine answered this with `push_units_off`, a teleport to the first free tile beside the
/// building - which is how a body ended up inside a tree. Now the unit is given an order and covers the ground at its own
/// speed, so every tick of its escape is one step and the place it stops is a place a route exists to.
#[test]
fn a_unit_on_a_new_foundation_walks_off_it_instead_of_jumping() {
    let Some(mut world) = world() else { return };
    explore(&mut world, 1, (9, 7));
    let builder = unit(&mut world, 1, "villager", (4, 6));
    let squatter = unit(&mut world, 1, "villager", (9, 6));
    let house = role(&world, "house");
    let footprint = [(8u16, 6u16), (9, 6), (8, 7), (9, 7)];
    let scale = world.scale();
    let step = Fixed::parse("1.2", scale).expect("speed").raw() / 20;

    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: house,
            at: (8, 6),
        })
        .expect("a house foundation");

    let mut previous = world
        .position(squatter)
        .expect("the squatter is in the world");
    let mut largest = 0i64;
    for _ in 0..400 {
        world.step().expect("step");
        let now = world
            .position(squatter)
            .expect("the squatter is still in the world");
        let moved = (now.0.raw() - previous.0.raw())
            .abs()
            .max((now.1.raw() - previous.1.raw()).abs());
        largest = largest.max(moved);
        previous = now;
        if !footprint.contains(&world.tile_of_position(now)) {
            break;
        }
    }
    // **A walk, not a placement.** One tick may cover at most one step; the teleport this replaced moved a whole tile.
    assert!(
        largest <= step * 2,
        "the squatter moved {largest} raw units in one tick, and its speed allows {step}: it was placed, not walked"
    );
    let tile = world.tile_of_position(previous);
    assert!(
        !footprint.contains(&tile),
        "the squatter is still standing on the footprint at {tile:?} after 400 ticks"
    );
    assert!(
        world.tile_walkable(tile.0, tile.1),
        "the squatter was walked off the site onto {tile:?}, which is not walkable ground"
    );
    assert!(
        world.body_clearance(squatter) >= 0,
        "the squatter's body overlaps a blocked tile where it stopped"
    );
    // **Not onto a resource or a building**: the D184 guard, unchanged by the walk.
    let on_something = world.entities().iter().any(|entity| {
        matches!(entity.kind, EntityKind::Resource(_) | EntityKind::Building)
            && entity.id != site_at(&world, (8, 6)).unwrap_or(EntityId(0))
            && world.tile_of_position(entity.position) == tile
    });
    assert!(
        !on_something,
        "the squatter was sent to {tile:?}, which holds a resource or a building"
    );
}

/// **A worker told to step aside goes back to its job when it arrives.**
///
/// The owner, asked what happens to a villager whose gather order a foundation interrupted: *"I think that villager
/// resumes. Please implement resume, but document this as an item to review on in the future."* The order is remembered in
/// `resuming` and re-issued when the step-aside walk arrives - the same mechanism a hunt uses when it finds an enemy (D47).
#[test]
fn a_worker_steps_aside_and_goes_back_to_its_gathering() {
    let Some(mut world) = world() else { return };
    explore(&mut world, 1, (4, 8));
    let worker = unit(&mut world, 1, "villager", (4, 8));
    let builder = unit(&mut world, 1, "villager", (2, 8));
    let node = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("the map's tree");
    world
        .submit_checked(Command::Gather {
            villager: worker,
            node,
        })
        .expect("a gather order");

    // Long enough to arrive, settle and have wood in hand.
    let mut working = false;
    for _ in 0..400 {
        world.step().expect("step");
        if world
            .entity(worker)
            .is_some_and(|entity| entity.carry.raw() > 0)
        {
            working = true;
            break;
        }
    }
    assert!(
        working,
        "the test's own premise is wrong: the villager never gathered anything"
    );

    // A house whose footprint covers the ground the villager is standing on.
    let house = role(&world, "house");
    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: house,
            at: (3, 8),
        })
        .expect("a house foundation over the worker");

    // It walks off, and when it arrives it is a forager again - not an idle unit beside a tree.
    let mut resumed = false;
    for _ in 0..400 {
        world.step().expect("step");
        if world.entity(worker).is_some_and(
            |entity| matches!(entity.task, ruage_sim::Task::Gather { node: at, .. } if at == node),
        ) {
            resumed = true;
            break;
        }
    }
    assert!(
        resumed,
        "the villager stepped off the foundation and forgot its gather order: it is idle beside a tree it was working"
    );
    // And it is working, not merely pointed at the tree.
    let before = world
        .entity(worker)
        .map(|entity| entity.amount.raw())
        .unwrap_or(0);
    let carry = world
        .entity(worker)
        .map(|entity| entity.carry.raw())
        .unwrap_or(0);
    for _ in 0..400 {
        world.step().expect("step");
    }
    let after = world
        .entity(worker)
        .map(|entity| entity.amount.raw())
        .unwrap_or(0);
    let carry_after = world
        .entity(worker)
        .map(|entity| entity.carry.raw())
        .unwrap_or(0);
    assert!(
        after != before || carry_after > carry,
        "the villager never gathered again after resuming: amount {before} -> {after}, carry {carry} -> {carry_after}"
    );
}

/// **A foundation may go in fog, never in the black** (D183 addendum).
///
/// The owner: *"building can be placed only on the scouted area (can be in fog but never in black)"*. The engine has kept a
/// per-player `explored` bitset since the fog landed, so this is a gate on the order rather than new state.
#[test]
fn a_foundation_may_not_be_placed_on_ground_the_player_has_never_seen() {
    let Some(mut world) = world() else { return };
    let builder = unit(&mut world, 1, "villager", (2, 8));
    let house = role(&world, "house");
    let at = (13u16, 3u16);
    assert_eq!(
        world.vision(1, at),
        ruage_sim::TileVision::Unseen,
        "the test's own premise is wrong: player 1 has already seen the tile"
    );
    let refused = world.submit_checked(Command::Build {
        villager: builder,
        unit: house,
        at,
    });
    assert!(
        matches!(refused, Err(ruage_sim::SimError::SiteUnscouted { .. })),
        "a foundation was placed on ground the player has never looked at: {refused:?}"
    );

    // Look at it once, then look away: remembered ground is a place a player may choose to build blind.
    look_away(&mut world, 1, at);
    assert_eq!(
        world.vision(1, at),
        ruage_sim::TileVision::Explored,
        "the test's own premise is wrong: the tile is still watched, not merely remembered"
    );
    let placed = world.submit_checked(Command::Build {
        villager: builder,
        unit: house,
        at,
    });
    assert!(
        placed.is_ok(),
        "a foundation in remembered ground was refused: {placed:?}"
    );
}

/// **A foundation may not be placed on a unit of another player the player can see** (D183 addendum): *"you also cannot
/// place any building over tile with neutral/enemy in"*.
#[test]
fn a_foundation_may_not_be_placed_on_a_visible_enemy_unit() {
    let Some(mut world) = world() else { return };
    let builder = unit(&mut world, 1, "villager", (2, 8));
    let enemy = unit(&mut world, 2, "villager", (9, 6));
    let house = role(&world, "house");
    // Somebody of ours watching that piece of ground: without it the tile is black and the order would be refused for a
    // different reason, which would hide the unit rule.
    let watcher = unit(&mut world, 1, "villager", (7, 6));
    assert!(
        world.visible_to(1, enemy),
        "the test's own premise is wrong: player 1 cannot see the enemy"
    );
    let refused = world.submit_checked(Command::Build {
        villager: builder,
        unit: house,
        at: (8, 6),
    });
    assert!(
        matches!(refused, Err(ruage_sim::SimError::SiteOccupied { tile }) if tile == (9, 6)),
        "a foundation was placed on a visible enemy unit: {refused:?}"
    );
    let _ = watcher;
}

/// **An enemy the player cannot see does not refuse the order.** His being there is not the player's to know, and a
/// refusal would leak it: the placement is allowed, and the empty-site gate is what keeps construction from starting.
#[test]
fn a_foundation_may_be_placed_over_an_enemy_in_the_fog() {
    let Some(mut world) = world() else { return };
    let builder = unit(&mut world, 1, "villager", (2, 8));
    let house = role(&world, "house");
    // **Away from the town centre's sight**, which reaches eight tiles: remembered ground is what a blind placement needs,
    // and a tile the town centre watches is not remembered - it is watched.
    let at = (10u16, 3u16);
    assert_eq!(
        world.vision(1, at),
        ruage_sim::TileVision::Unseen,
        "the test's own premise is wrong: player 1 has already looked at {at:?}"
    );
    look_away(&mut world, 1, at);
    let enemy = unit(&mut world, 2, "villager", (11, 3));
    assert!(
        !world.visible_to(1, enemy),
        "the test's own premise is wrong: player 1 can see the enemy"
    );
    assert_eq!(
        world.vision(1, at),
        ruage_sim::TileVision::Explored,
        "the test's own premise is wrong: the ground is not remembered"
    );
    let placed = world.submit_checked(Command::Build {
        villager: builder,
        unit: house,
        at,
    });
    assert!(
        placed.is_ok(),
        "a foundation was refused because of an enemy the player cannot see: {placed:?}"
    );
}

/// **A farm is the exception** (D183 §7): it is seeded by walking over it, so it has no empty site to wait for and nobody
/// standing on it holds it up. Unlike a house, whose floor is solid the moment the first stroke lands, a farm is a floor
/// for ever - which is exactly why it cannot trap anyone.
#[test]
fn a_farm_starts_with_a_worker_standing_on_it() {
    let Some(mut world) = world() else { return };
    let builder = unit(&mut world, 1, "villager", (2, 8));
    // On the plot itself, where a house site would be waiting for him to leave.
    let farmhand = unit(&mut world, 1, "villager", (6, 7));
    let farm = role(&world, "farm");
    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: farm,
            at: (6, 6),
        })
        .expect("a farm plot");
    world.step().expect("the order lands");
    let site = site_at(&world, (6, 6)).expect("the plot");

    let mut started = false;
    for _ in 0..600 {
        world.step().expect("step");
        if progress(&world, site).is_some_and(|ticks| ticks > 0) {
            started = true;
            break;
        }
    }
    assert!(
        started,
        "the farm never started: a unit standing on a farm plot held it up, but a farm is seeded by walking over it"
    );
    // And the man is still on it: a farm does not evict anybody, because it traps nobody.
    let on_the_plot = world
        .position(farmhand)
        .map(|at| world.tile_of_position(at))
        .is_some_and(|(x, y)| (6..=8).contains(&x) && (6..=8).contains(&y));
    assert!(
        on_the_plot,
        "the farmhand was moved off the plot: a farm should not ask anybody to leave"
    );
    assert!(
        world.tile_walkable(6, 7),
        "a farm plot under construction blocked a tile - a farm is walkable while it is seeded and after (D163)"
    );
}

/// **An untouched foundation is invisible to the enemy** (D183 §2): *"Enemy units don't see the foundation until villager
/// started working on it"* - and it is invisible as a fact, not as a drawing, so his pathing and his eyes both treat the
/// ground as free. The moment the first stroke lands, everybody in line of sight can see it.
#[test]
fn an_untouched_foundation_is_invisible_to_the_enemy() {
    let Some(mut world) = world() else { return };
    explore(&mut world, 1, (11, 6));
    let builder = unit(&mut world, 1, "villager", (2, 8));
    // Standing two tiles away, with the ground in plain sight: this is line of sight, not fog.
    let watcher = unit(&mut world, 2, "villager", (13, 6));
    let house = role(&world, "house");
    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: house,
            at: (11, 6),
        })
        .expect("a foundation");
    world.step().expect("the order lands");
    let site = site_at(&world, (11, 6)).expect("the foundation");
    assert_eq!(
        world.vision(2, (11, 6)),
        ruage_sim::TileVision::Visible,
        "the test's own premise is wrong: player 2 is not watching the ground"
    );
    assert_eq!(progress(&world, site), Some(0), "the site is untouched");
    assert!(
        !world.visible_to(2, site),
        "the enemy can see a foundation nobody has started: the ghost is supposed to be invisible until a villager works"
    );
    assert!(
        world.visible_to(1, site),
        "the owner cannot see their own foundation"
    );

    // The first stroke: now it is a building everybody in sight can see.
    let mut started = false;
    for _ in 0..600 {
        world.step().expect("step");
        if progress(&world, site).is_some_and(|ticks| ticks > 0) {
            started = true;
            break;
        }
    }
    assert!(started, "construction never started");
    assert!(
        world.visible_to(2, site),
        "a foundation under construction is still invisible to the enemy"
    );
    let _ = watcher;
}

/// **Make a tile remembered but not watched**: step one of the player's own units past it, then take it away.
fn look_away(world: &mut World, player: u8, tile: (u16, u16)) {
    let beside = (i64::from(tile.0) + 1, i64::from(tile.1));
    let watcher = unit(world, player, "villager", beside);
    assert_eq!(
        world.vision(player, tile),
        ruage_sim::TileVision::Visible,
        "the watcher did not look at {tile:?}"
    );
    world
        .submit_checked(Command::Delete {
            player,
            entity: watcher,
        })
        .expect("the watcher is deleted");
    world.step().expect("step");
}
