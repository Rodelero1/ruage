//! Foundations: a tile can hold one building per player, and only one that the other can see.
//!
//! Written before the implementation (D7). The owner's rule, in his words:
//!
//! - "buildings foundations of one player cannot be placed one over the other" - one player's own
//!   foundations never stack;
//! - "if player places building foundations other player cannot see it and can place his buildings
//!   foundation over it" - a foundation you cannot see does not block you;
//! - "whoever starts building first wins and the other building is cancelled";
//! - you may place over a building you cannot see, and the placement is cancelled when your own unit
//!   or the villager you sent can see the tile;
//! - a cancelled foundation is refunded (D58a), a destroyed one is not (D60).
//!
//! Two clocks, not one: the tile's visibility when the order is placed, and who first put a builder to
//! work. Reading visibility at arrival would refuse every blind overbuild and make the rule
//! unobservable; reading it *later* is exactly what cancels a dead placement on sight.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, SimError, TileVision, World};

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

/// A free id. `start_build` numbers its foundation `max + 1`, so a hand-picked literal is a trap: it
/// collides with a foundation built a moment earlier.
fn fresh_id(world: &World) -> u32 {
    world
        .entities()
        .iter()
        .map(|entity| entity.id.0)
        .max()
        .unwrap_or(0)
        + 1
}

fn villager(world: &mut World, player: u8, tile: (i64, i64)) -> EntityId {
    villager_at_speed(world, player, tile, "1.0")
}

fn villager_at_speed(world: &mut World, player: u8, tile: (i64, i64), speed: &str) -> EntityId {
    let scale = world.scale();
    let id = EntityId(fresh_id(world));
    let unit = world.role_unit("villager").expect("villager");
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            // A villager that cannot walk cannot build eight tiles away, and the whole rule is about a
            // player choosing a tile it cannot see. One tile a second is enough to arrive inside the loop.
            speed: Fixed::parse(speed, scale).expect("a walkable speed"),
            hp: 25,
            amount: 0,
        })
        .expect("spawn villager");
    world.step().expect("step");
    id
}

/// The foundation of `player` on `tile`, if one is there.
fn site_at(world: &World, player: u8, tile: (i64, i64)) -> Option<EntityId> {
    let scale = world.scale();
    let position = (
        Fixed::from_raw(tile.0 * scale),
        Fixed::from_raw(tile.1 * scale),
    );
    world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == player && entity.position == position && entity.construction.is_some()
        })
        .map(|entity| entity.id)
}

/// Step until `player` has a foundation on `tile`, or give up.
fn wait_for_site(
    world: &mut World,
    player: u8,
    tile: (i64, i64),
    ticks: usize,
) -> Option<EntityId> {
    for _ in 0..ticks {
        world.step().expect("step");
        if let Some(id) = site_at(world, player, tile) {
            return Some(id);
        }
    }
    None
}

/// Every resource this player has, by slot. Resource names are content's; the slots are enough here.
fn stockpile(world: &World, player: u8) -> Vec<u32> {
    (0..4).map(|slot| world.stock(player, slot)).collect()
}

/// A finished building, placed directly - for the cases where what matters is what stands there.
fn building(world: &mut World, player: u8, role: &str, tile: (i64, i64)) -> EntityId {
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

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    world
        .entity(id)
        .map(|entity| {
            (
                entity.position.0.raw() / world.scale(),
                entity.position.1.raw() / world.scale(),
            )
        })
        .expect("an entity with a tile")
}

fn house(world: &World) -> u32 {
    world.role_unit("house").expect("house")
}

#[test]
fn my_own_foundation_is_in_the_way() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let first = villager(&mut world, 1, (18, 16));
    let second = villager(&mut world, 1, (20, 16));
    let house = house(&world);

    world
        .submit_checked(Command::Build {
            villager: first,
            unit: house,
            at: (19, 16),
        })
        .expect("the first foundation");
    assert!(
        wait_for_site(&mut world, 1, (19, 16), 200).is_some(),
        "the first foundation never appeared"
    );

    let refused = world.submit_checked(Command::Build {
        villager: second,
        unit: house,
        at: (19, 16),
    });
    assert!(
        matches!(refused, Err(SimError::SiteOccupied { tile }) if tile == (19, 16)),
        "a player stacked two foundations on one tile: {refused:?}"
    );
}

#[test]
fn a_foundation_i_cannot_see_does_not_block_me() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    // Player 2 raises a foundation well outside player 1's sight.
    let theirs = villager(&mut world, 2, (31, 30));
    let house = house(&world);
    world
        .submit_checked(Command::Build {
            villager: theirs,
            unit: house,
            at: (30, 30),
        })
        .expect("their foundation");
    wait_for_site(&mut world, 2, (30, 30), 200).expect("their foundation");

    // Player 1 cannot see that tile, so the order is taken: the player chose blind.
    let mine = villager(&mut world, 1, (30, 38));
    assert_ne!(
        world.vision(1, (30, 30)),
        TileVision::Visible,
        "the test's own premise is wrong: player 1 can see the tile"
    );
    let placed = world.submit_checked(Command::Build {
        villager: mine,
        unit: house,
        at: (30, 30),
    });
    assert!(
        placed.is_ok(),
        "a foundation player 1 cannot see refused the order: {placed:?}"
    );
}

#[test]
fn a_cancelled_foundation_refunds_its_owner() {
    // The owner's rule: a foundation cancelled by an overbuild gives its resources back, so a player whose
    // town centre was placed in the wrong spot is not ruined, merely delayed.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    // Their builder is eight tiles away, so their site stays untouched; mine starts two tiles closer,
    // which is what makes mine the first builder - and the one that keeps the tile (D61).
    let theirs = villager(&mut world, 2, (30, 38));
    let house = house(&world);
    let before = stockpile(&world, 2);

    world
        .submit_checked(Command::Build {
            villager: theirs,
            unit: house,
            at: (30, 30),
        })
        .expect("their foundation");
    world.step().expect("step");
    let paid = stockpile(&world, 2);
    assert_ne!(paid, before, "the foundation cost its owner nothing");

    let mine = villager(&mut world, 1, (30, 35));
    let my_before = stockpile(&world, 1);
    world
        .submit_checked(Command::Build {
            villager: mine,
            unit: house,
            at: (30, 30),
        })
        .expect("a blind foundation");
    for _ in 0..900 {
        world.step().expect("step");
        if site_at(&world, 2, (30, 30)).is_none() {
            break;
        }
    }
    assert!(
        site_at(&world, 2, (30, 30)).is_none(),
        "their foundation was not cancelled, so nothing was refunded"
    );
    assert_eq!(
        stockpile(&world, 2),
        before,
        "the cancelled foundation did not give its cost back"
    );
    // The winner paid and keeps paying: a refund is for the one whose building went, not for both.
    let mine_now: u32 = stockpile(&world, 1).iter().sum();
    let mine_then: u32 = my_before.iter().sum();
    assert!(
        mine_now < mine_then,
        "the overbuilder was paid for its own house"
    );
}

#[test]
fn a_building_i_cannot_see_does_not_block_my_placement() {
    // The owner's rule: you may lay a foundation in fog where a finished building already stands. You
    // find out when you look, and the game cancels your placement then - not at the click.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let theirs = building(&mut world, 2, "house", (30, 30));
    let tile = tile_of(&world, theirs);
    let mine = villager(&mut world, 1, (30, 38));
    assert_ne!(
        world.vision(1, (tile.0 as u16, tile.1 as u16)),
        TileVision::Visible,
        "the test's own premise is wrong: player 1 can see the house"
    );
    let house = house(&world);
    let placed = world.submit_checked(Command::Build {
        villager: mine,
        unit: house,
        at: (tile.0 as u16, tile.1 as u16),
    });
    assert!(
        placed.is_ok(),
        "a placement over a building in the fog was refused: {placed:?}"
    );
}

#[test]
fn a_placement_over_a_hidden_building_is_cancelled_when_it_is_seen() {
    // The same placement, once the villager sent to build it can see the tile: cancelled, and the
    // resources come back, because the player was never given a chance to know.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let theirs = building(&mut world, 2, "house", (30, 30));
    let tile = tile_of(&world, theirs);
    let mine = villager(&mut world, 1, (30, 38));
    let house = house(&world);
    let before: u32 = stockpile(&world, 1).iter().sum();
    world
        .submit_checked(Command::Build {
            villager: mine,
            unit: house,
            at: (tile.0 as u16, tile.1 as u16),
        })
        .expect("a blind placement");
    world.step().expect("step");
    let site = site_at(&world, 1, (tile.0, tile.1)).expect("my foundation");

    for _ in 0..900 {
        world.step().expect("step");
        if world.entity(site).is_none() {
            break;
        }
    }
    assert!(
        world.entity(site).is_none(),
        "a placement over a hidden building survived being seen"
    );
    let after: u32 = stockpile(&world, 1).iter().sum();
    assert!(
        after >= before.saturating_sub(1),
        "the cancelled placement did not give its resources back"
    );
}

#[test]
fn a_foundation_i_can_see_blocks_me() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let theirs = villager(&mut world, 2, (16, 16));
    let house = house(&world);
    world
        .submit_checked(Command::Build {
            villager: theirs,
            unit: house,
            at: (17, 16),
        })
        .expect("their foundation");
    wait_for_site(&mut world, 2, (17, 16), 200).expect("their foundation");

    let mine = villager(&mut world, 1, (18, 16));
    assert_eq!(
        world.vision(1, (17, 16)),
        TileVision::Visible,
        "the test's own premise is wrong: player 1 cannot see the tile"
    );
    let refused = world.submit_checked(Command::Build {
        villager: mine,
        unit: house,
        at: (17, 16),
    });
    assert!(
        matches!(refused, Err(SimError::SiteOccupied { tile }) if tile == (17, 16)),
        "a foundation player 1 can see did not block the order: {refused:?}"
    );
}

#[test]
fn the_first_builder_keeps_the_tile() {
    // The owner's rule: whoever starts building first wins and the other building is cancelled. Here
    // the newcomer loses, because the other side was already working when the foundation arrived.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let theirs = villager(&mut world, 2, (31, 30));
    let house = world.role_unit("house").expect("house");
    world
        .submit_checked(Command::Build {
            villager: theirs,
            unit: house,
            at: (30, 30),
        })
        .expect("their foundation");
    let their_site = wait_for_site(&mut world, 2, (30, 30), 200).expect("their foundation");
    world.step().expect("step");
    assert!(
        world
            .entity(their_site)
            .and_then(|entity| entity.construction)
            .is_some_and(|construction| construction.progress_ticks > 0),
        "the test's premise is wrong: their foundation has not been started"
    );

    let mine = villager(&mut world, 1, (30, 38));
    let before: u32 = stockpile(&world, 1).iter().sum();
    world
        .submit_checked(Command::Build {
            villager: mine,
            unit: house,
            at: (30, 30),
        })
        .expect("a blind placement");
    world.step().expect("step");
    let my_site = site_at(&world, 1, (30, 30)).expect("my foundation");

    for _ in 0..900 {
        world.step().expect("step");
        if world.entity(my_site).is_none() {
            break;
        }
    }
    assert!(
        world.entity(my_site).is_none(),
        "the later builder kept a tile the other side had already started"
    );
    assert!(
        world.entity(their_site).is_some(),
        "the side that started first lost the tile"
    );
    let after: u32 = stockpile(&world, 1).iter().sum();
    assert!(
        after >= before.saturating_sub(1),
        "the cancelled site was not refunded"
    );
}

#[test]
fn an_untouched_foundation_loses_to_the_first_builder() {
    // Both placed, neither started: the one who puts a builder to work takes the tile.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    // Theirs, placed from eight tiles away, so their builder is still walking and the site untouched.
    let theirs = villager(&mut world, 2, (30, 38));
    let house = world.role_unit("house").expect("house");
    world
        .submit_checked(Command::Build {
            villager: theirs,
            unit: house,
            at: (30, 30),
        })
        .expect("their foundation");
    let their_site = wait_for_site(&mut world, 2, (30, 30), 200).expect("their foundation");
    assert_eq!(
        world
            .entity(their_site)
            .and_then(|entity| entity.construction)
            .map(|construction| construction.progress_ticks),
        Some(0),
        "the test's premise is wrong: their foundation has been worked"
    );

    // Mine: placed from three tiles closer, so my builder arrives first - and far enough to be blind.
    let mine = villager(&mut world, 1, (30, 35));
    let before: u32 = stockpile(&world, 2).iter().sum();
    world
        .submit_checked(Command::Build {
            villager: mine,
            unit: house,
            at: (30, 30),
        })
        .expect("my foundation");
    world.step().expect("step");
    let my_site = site_at(&world, 1, (30, 30)).expect("my foundation");

    for _ in 0..120 {
        world.step().expect("step");
        if world.entity(their_site).is_none() {
            break;
        }
    }
    assert!(
        world.entity(their_site).is_none(),
        "an untouched foundation survived the first builder"
    );
    assert!(
        world.entity(my_site).is_some(),
        "the first builder lost the tile to an untouched foundation"
    );
    let after: u32 = stockpile(&world, 2).iter().sum();
    assert!(
        after >= before.saturating_sub(1),
        "the cancelled foundation was not refunded"
    );
}

#[test]
fn a_cancelled_foundation_leaves_nothing_behind() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let theirs = villager(&mut world, 2, (30, 38));
    let house = house(&world);
    world
        .submit_checked(Command::Build {
            villager: theirs,
            unit: house,
            at: (30, 30),
        })
        .expect("their foundation");
    let their_site = wait_for_site(&mut world, 2, (30, 30), 200).expect("their foundation");

    let mine = villager(&mut world, 1, (30, 35));
    world
        .submit_checked(Command::Build {
            villager: mine,
            unit: house,
            at: (30, 30),
        })
        .expect("a blind foundation");
    for _ in 0..900 {
        world.step().expect("step");
        if world.entity(their_site).is_none() {
            break;
        }
    }
    assert!(world.entity(their_site).is_none(), "not cancelled");
    // No side table may remember the cancelled site - the leak the `removal` tests exist for.
    assert!(
        !world.bookkeeping_touches(their_site),
        "the cancelled foundation left bookkeeping behind"
    );
}
