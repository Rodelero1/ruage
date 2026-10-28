//! Foundations: a tile can hold one building per player, and only one that the other can see.
//!
//! Written before the implementation (D7). The owner's rule, in his words:
//!
//! - "buildings foundations of one player cannot be placed one over the other" - one player's own
//!   foundations never stack;
//! - "if player places building foundations other player cannot see it and can place his buildings
//!   foundation over it" - a foundation you cannot see does not block you;
//! - "if other places his building over mine and starts constructing it, my building gets cancelled" -
//!   and the first builder to *work* takes the tile, cancelling the blind one.
//!
//! The timing is the whole rule: the tile's visibility is read **when the order is placed**, because the
//! player chooses blind. By the time the villager walks over, the tile is visible - which is why reading
//! visibility at arrival would make the rule unobservable.

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
    let theirs = villager(&mut world, 2, (31, 30));
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

    let mine = villager(&mut world, 1, (30, 38));
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
fn a_finished_building_blocks_even_when_i_cannot_see_it() {
    // The owner's rule is about *foundations*: a site another player has merely marked out is the one you
    // may take blind. A finished building is not a place to overbuild, fog or no fog - otherwise a player
    // could raze a town centre by walking up to it and laying a house, which is not the game either
    // version plays.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    // Player 2's town centre, far outside player 1's sight.
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 2 && entity.kind == EntityKind::Building)
        .map(|entity| entity.id)
        .expect("the enemy town centre");
    let tile = world
        .entity(centre)
        .map(|entity| {
            (
                entity.position.0.raw() / world.scale(),
                entity.position.1.raw() / world.scale(),
            )
        })
        .expect("its tile");
    let mine = villager(&mut world, 1, (tile.0, tile.1 + 8));
    assert_ne!(
        world.vision(1, (tile.0 as u16, tile.1 as u16)),
        TileVision::Visible,
        "the test's own premise is wrong: player 1 can see the town centre"
    );
    let house = house(&world);
    let refused = world.submit_checked(Command::Build {
        villager: mine,
        unit: house,
        at: (tile.0 as u16, tile.1 as u16),
    });
    assert!(
        matches!(refused, Err(SimError::SiteOccupied { .. })),
        "a finished building in the fog did not block the order: {refused:?}"
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
fn the_first_builder_to_work_cancels_the_blind_foundation() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let theirs = villager(&mut world, 2, (31, 30));
    let house = house(&world);
    world
        .submit_checked(Command::Build {
            villager: theirs,
            unit: house,
            at: (30, 30),
        })
        .expect("their foundation");
    let their_site = wait_for_site(&mut world, 2, (30, 30), 200).expect("their foundation");

    let mine = villager(&mut world, 1, (30, 38));
    world
        .submit_checked(Command::Build {
            villager: mine,
            unit: house,
            at: (30, 30),
        })
        .expect("a blind foundation");

    // Walk over and start work. Their site must be gone, mine must stand and finish.
    for _ in 0..900 {
        world.step().expect("step");
        if world.entity(their_site).is_none() {
            break;
        }
    }
    assert!(
        world.entity(their_site).is_none(),
        "the blind foundation survived being built over"
    );
    let mine_site = site_at(&world, 1, (30, 30)).expect("the builder lost the tile it took");

    for _ in 0..900 {
        world.step().expect("step");
        if world
            .entity(mine_site)
            .is_some_and(|entity| entity.construction.is_none())
        {
            break;
        }
    }
    assert!(
        world
            .entity(mine_site)
            .is_some_and(|entity| entity.construction.is_none()),
        "the house that took the tile never finished"
    );
}

#[test]
fn a_cancelled_foundation_leaves_nothing_behind() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let theirs = villager(&mut world, 2, (31, 30));
    let house = house(&world);
    world
        .submit_checked(Command::Build {
            villager: theirs,
            unit: house,
            at: (30, 30),
        })
        .expect("their foundation");
    let their_site = wait_for_site(&mut world, 2, (30, 30), 200).expect("their foundation");

    let mine = villager(&mut world, 1, (30, 38));
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
