//! **T12: the tick is the original's sequence** - Gaia, then players 1-8, each player's objects in slot order.
//!
//! `BaseWorld::update` (`FUN_00738720`, `0x738720`) walks the players array at `+0x184..+0x188` from index 0 upward and,
//! inside a player's turn, steps that player's objects one at a time; an object that is dead is unlinked and destroyed in
//! its **own** turn (`+0xa0` answers, the destructor runs) instead of surviving to a sweep at the end of the tick. So
//! two things follow that entity-id order does not give:
//!
//! 1. the order of a blow is **player, then slot** - the lower player's man swings first whatever the ids are;
//! 2. an object killed by an earlier turn does **nothing** for the rest of the tick - it does not finish a job.
//!
//! Written before the implementation (D7, SPEC §11).

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: ruage_data::Map,
    economy: ruage_data::Economy,
    resources: ruage_data::Resources,
    units: ruage_data::Units,
    combat: ruage_data::Combat,
    techs: ruage_data::Techs,
    effects: ruage_data::Effects,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        eprintln!(
            "skipping: {} is not present (generated data, D8)",
            units.display()
        );
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
        combat: ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat.toml"),
        techs: ruage_data::load_techs_file(&repo("config/gamedata/techs.toml"))
            .expect("techs.toml"),
        effects: ruage_data::load_effects_file(&repo("config/gamedata/effects.toml"))
            .expect("effects.toml"),
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 3).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    // Accuracy is its own test; these two stand still and swing at arm's length.
    let mut combat = fx.combat.clone();
    combat.accuracy_scatter = ruage_data::MaybeFixed::from_f64(0.0);
    world.load_combat(&combat);
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("the tech and effect tables load");
    world
}

/// **An explicit id**, because a spawn is queued: two `max + 1` calls before the first one lands both read the same
/// maximum and the second spawn is refused as a duplicate (the first version of this fixture did exactly that).
fn spawn(
    world: &mut World,
    id: u32,
    player: u8,
    unit: u32,
    kind: EntityKind,
    tile: (i64, i64),
    hp: i32,
) -> EntityId {
    let scale = world.scale();
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp,
            amount: 0,
        })
        .expect("spawn");
    id
}

const VILLAGER: u32 = 83;

/// **A world with player 2 in it before the stock is applied.** `apply_start_stock` runs once, for the players that exist
/// at that moment, so a player-2 building spawned after `load_resources` has no stock - and could not pay for a recruit or
/// a tech. The closure is the caller's own player-2 ground.
fn world_with_player_two(fx: &Fixture, place: impl FnOnce(&mut World)) -> World {
    let mut world = World::new(&fx.settings, 3).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    place(&mut world);
    world.step().expect("the player-2 ground lands");
    world
        .load_resources(&fx.resources)
        .expect("the start stock reaches a player that has entities");
    let mut combat = fx.combat.clone();
    combat.accuracy_scatter = ruage_data::MaybeFixed::from_f64(0.0);
    world.load_combat(&combat);
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("the tech and effect tables load");
    world
}

/// **The lower player swings first, whatever the ids are.** The player-2 man is created first, so its id is lower and the
/// two candidate orders disagree about who dies: id order kills the player-1 man, the original's order kills player-2's.
#[test]
fn the_lower_player_acts_first_whatever_the_ids() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let two = spawn(&mut world, 900, 2, VILLAGER, EntityKind::Unit, (30, 30), 1);
    let one = spawn(&mut world, 901, 1, VILLAGER, EntityKind::Unit, (31, 30), 1);
    assert!(
        two.0 < one.0,
        "the fixture's premise: player 2's man carries the lower id ({:?} and {:?})",
        two,
        one
    );
    world
        .step()
        .expect("the two exist before an order names them");

    world
        .submit_checked(Command::Attack {
            attacker: one,
            target: two,
        })
        .expect("player 1 attacks player 2");
    world
        .submit_checked(Command::Attack {
            attacker: two,
            target: one,
        })
        .expect("player 2 attacks player 1");

    for _ in 0..200 {
        world.step().expect("step");
    }

    assert!(
        world.entity(two).is_none(),
        "player 1's turn comes first, so its man's blow landed and player 2's man is dead"
    );
    assert!(
        world.entity(one).is_some(),
        "an id order would have let the lower id swing first and kill this man; the tick's order is player, then slot"
    );
}

/// **A building destroyed in the blow pass finishes nothing.** The barracks is player 2's and the killer is player 1's, so
/// player 1's objects come first in the original's order: the building is dead before its own turn and the recruit it had
/// one tick left on never appears. Under entity-id order - or with production running after the blows and reading no
/// health - the recruit arrives out of a ruin.
#[test]
fn a_building_destroyed_this_tick_finishes_nothing() {
    let Some(fx) = fixture() else { return };
    // Roles, not dat ids: a rename in the pack is a behaviour change the engine must follow (D17).
    let probe = seeded(&fx);
    let barracks_role = probe.role_unit("barracks").expect("a barracks role");
    let house_role = probe.role_unit("house").expect("a house role");
    let militia_role = probe.role_unit("militia").expect("a militia role");
    drop(probe);

    let mut world = world_with_player_two(&fx, |world| {
        spawn(
            world,
            900,
            2,
            barracks_role,
            EntityKind::Building,
            (24, 24),
            1,
        );
        // A house for the headroom: a recruit needs population, and a barracks provides none.
        spawn(
            world,
            901,
            2,
            house_role,
            EntityKind::Building,
            (28, 28),
            550,
        );
    });
    let barracks = EntityId(900);

    // Player 1's killer stands on the tile next to the barracks' anchor, so its blow lands on the tick it is ordered.
    let killer = spawn(&mut world, 910, 1, VILLAGER, EntityKind::Unit, (23, 24), 25);
    world.step().expect("the killer exists");

    world
        .submit_checked(Command::Train {
            building: barracks,
            unit: militia_role,
        })
        .expect("a barracks in the Dark Age takes a militia");

    // The order lands on the next step; only then is there a job to read.
    world.step().expect("the order lands");
    // Step to the tick **before** the recruit appears. `fresh` spends the order's own tick, so the count is one short.
    let mut ticks = 0;
    loop {
        let remaining = world
            .entity(barracks)
            .and_then(|building| building.training)
            .map(|job| job.remaining_ticks);
        match remaining {
            Some(1) => break,
            Some(_) => {}
            None => {
                panic!("the barracks is not training anything, so the fixture measures nothing")
            }
        }
        world.step().expect("step");
        ticks += 1;
        assert!(ticks < 10_000, "the job never reached its last tick");
    }

    let trained_before = world
        .entities()
        .iter()
        .filter(|entity| entity.player == 2 && entity.unit == Some(militia_role))
        .count();
    assert_eq!(trained_before, 0, "the recruit has not appeared yet");

    world
        .submit_checked(Command::Attack {
            attacker: killer,
            target: barracks,
        })
        .expect("player 1 attacks the barracks");
    world
        .step()
        .expect("the blow lands on the tick the job would have finished");

    assert!(
        world.entity(barracks).is_none(),
        "the barracks is dead: the blow took its last hit point"
    );
    let trained_after = world
        .entities()
        .iter()
        .filter(|entity| entity.player == 2 && entity.unit == Some(militia_role))
        .count();
    assert_eq!(
        trained_after, 0,
        "a building killed before its turn finished its recruit anyway"
    );
}

/// **A building destroyed in the blow pass finishes no research either.** Same rule as production, same gate: a town
/// centre killed by the lower player's man never lands the tech it had one tick left on.
#[test]
fn a_building_destroyed_this_tick_finishes_no_research() {
    let Some(fx) = fixture() else { return };
    let probe = seeded(&fx);
    let centre_role = probe.role_unit("town_center").expect("a town centre role");
    let loom = probe.tech_role("loom").expect("a loom tech");
    drop(probe);

    let mut world = world_with_player_two(&fx, |world| {
        spawn(
            world,
            900,
            2,
            centre_role,
            EntityKind::Building,
            (24, 24),
            1,
        );
    });
    let centre = EntityId(900);
    let killer = spawn(&mut world, 910, 1, VILLAGER, EntityKind::Unit, (23, 24), 25);
    world.step().expect("the killer exists");

    // Loom takes a villager from 25 health to 40, so the villager's ceiling is the witness: it moves only if the tech lands.
    let before = world
        .max_hp_of(2, VILLAGER)
        .expect("the villager's health is in the pack");
    world
        .submit_checked(Command::Research {
            building: centre,
            tech: loom,
        })
        .expect("a town centre takes Loom in the Dark Age");
    world.step().expect("the order lands");

    let mut ticks = 0;
    loop {
        let remaining = world
            .entity(centre)
            .and_then(|building| building.researching)
            .map(|job| job.remaining_ticks);
        match remaining {
            Some(1) => break,
            Some(_) => {}
            None => panic!(
                "the town centre is not researching anything, so the fixture measures nothing"
            ),
        }
        world.step().expect("step");
        ticks += 1;
        assert!(ticks < 10_000, "the tech never reached its last tick");
    }

    world
        .submit_checked(Command::Attack {
            attacker: killer,
            target: centre,
        })
        .expect("player 1 attacks the town centre");
    world
        .step()
        .expect("the blow lands on the tick the tech would have finished");

    assert!(
        world.entity(centre).is_none(),
        "the town centre is dead: the blow took its last hit point"
    );
    assert_eq!(
        world.max_hp_of(2, VILLAGER),
        Some(before),
        "a building killed before its turn finished the tech anyway"
    );
}
