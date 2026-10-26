//! Combat: damage from the dat's attack and armour arrays, and a spearman killing a villager.
//!
//! Written before the implementation (D7).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, DamageRules, EntityId, EntityKind, SimError, Task, World, damage_dealt};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    units: Units,
    combat: ruage_data::Combat,
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
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml")).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        units: ruage_data::load_units_file(&units).expect("units.toml"),
        combat: ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat.toml"),
    })
}

fn rules(fx: &Fixture) -> DamageRules {
    DamageRules {
        minimum: fx.combat.damage.minimum,
        missing_armour: 10_000,
    }
}

/// A world with a spearman of player 1 and a villager of player 2 standing next to each other on
/// open ground, away from the authored town centres.
fn duel(fx: &Fixture) -> (World, EntityId, EntityId) {
    let mut world = World::new(&fx.settings, 3).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world.load_combat(&fx.combat);
    let scale = world.scale();
    let at = |x: i64, y: i64| (Fixed::from_raw(x * scale), Fixed::from_raw(y * scale));

    // Command::Spawn is the test hook: it lets a test place the two combatants without gathering
    // 175 wood, 60 food and 20 gold first. Real units come from training.
    let spearman = EntityId(900);
    let villager = EntityId(901);
    world
        .submit_checked(Command::Spawn {
            id: spearman,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(74),
            position: at(30, 30),
            // Speed zero on purpose: `Command::Spawn` is the mover hook, so a nonzero speed also
            // means "keep walking". These two are placed adjacent, so neither needs to close.
            speed: Fixed::ZERO,
            hp: 40,
            amount: 0,
        })
        .expect("a spearman");
    world
        .submit_checked(Command::Spawn {
            id: villager,
            player: 2,
            kind: EntityKind::Unit,
            unit: Some(83),
            position: at(31, 30),
            speed: Fixed::ZERO,
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    // Commands are queued and land on the next tick, so the two must exist before an order can
    // name them.
    world.step().expect("step");
    (world, spearman, villager)
}

#[test]
fn damage_is_attack_minus_armour_per_class() {
    let Some(fx) = fixture() else { return };
    let rules = rules(&fx);
    let spearman = fx.units.unit(74).expect("SPRMN");
    let villager = fx.units.unit(83).expect("VMBAS");

    // Spearman: class 4 attack 4. Villager: class 4 armour 0. Everything else is unlisted on one
    // side or the other, so it contributes nothing.
    assert_eq!(
        damage_dealt(spearman, villager, &rules),
        4,
        "spearman -> villager"
    );
    // Villager: class 11 attack 3, class 4 attack 3. Spearman has class 4 armour 0 and no class 11.
    assert_eq!(
        damage_dealt(villager, spearman, &rules),
        3,
        "villager -> spearman"
    );
    // A spearman against a building: the house lists armour classes 21, 11, 4 and 3, and its
    // seven points are pierce (class 3). Class 4 armour is zero, so the spearman's class 4 attack
    // lands in full; the other classes are unlisted on one side or the other.
    let house = fx.units.unit(70).expect("HOUS");
    assert_eq!(
        damage_dealt(spearman, house, &rules),
        4,
        "spearman -> house"
    );
    // The town centre is the one carrying class 4 armour 3, so the same spearman manages one.
    let centre = fx.units.unit(109).expect("RTWC");
    assert_eq!(
        damage_dealt(spearman, centre, &rules),
        1,
        "spearman -> town centre"
    );
}

#[test]
fn no_attack_never_floors_below_the_minimum() {
    let Some(fx) = fixture() else { return };
    let mut rules = rules(&fx);
    let villager = fx.units.unit(83).expect("VMBAS");
    let spearman = fx.units.unit(74).expect("SPRMN");
    // Without the floor this pair does 3. Give the villager's only useful attack class the same
    // armour value as the weapon and the sum is zero, which must become the minimum, not nothing.
    let mut armoured = spearman.clone();
    armoured.armours = vec![(11, 3), (4, 3), (3, 1), (1, 0)];
    assert_eq!(
        damage_dealt(villager, &armoured, &rules),
        rules.minimum,
        "a stopped attack still lands for the minimum"
    );
    rules.minimum = 2;
    assert_eq!(damage_dealt(villager, &armoured, &rules), 2);
}

#[test]
fn a_spearman_kills_a_villager_in_seven_hits() {
    let Some(fx) = fixture() else { return };
    let (mut world, spearman, villager) = duel(&fx);

    world
        .submit_checked(Command::Attack {
            attacker: spearman,
            target: villager,
        })
        .expect("the attack order must be accepted");

    let damage = 4;
    let hits_needed = 25_i32.div_euclid(damage) + i32::from(25 % damage != 0);
    let reload_ticks = 2 * world.tick_rate_hz(); // SPRMN reload 2.0 s
    let expected_ticks = u64::try_from(hits_needed - 1).unwrap() * u64::from(reload_ticks);

    assert_eq!(
        world.entity(villager).expect("villager").hp,
        25,
        "nothing happens until the next tick"
    );

    let mut first_hit = None;
    let mut died_at = None;
    let mut hits = 0;
    let mut last_hp = 25;
    for _ in 0..(expected_ticks * 3) {
        world.step().expect("step");
        if world.entity(villager).is_none() {
            died_at = Some(world.tick());
            hits += 1; // the killing blow
            break;
        }
        let hp = world.entity(villager).expect("villager").hp;
        if hp < last_hp {
            assert_eq!(last_hp - hp, damage, "every hit must be worth {damage}");
            hits += 1;
            first_hit.get_or_insert(world.tick());
            last_hp = hp;
        } else {
            assert_eq!(hp, last_hp, "hp only ever goes down");
        }
    }

    let first = first_hit.expect("the first hit lands");
    let died = died_at.expect("the villager dies");
    assert_eq!(hits, hits_needed, "seven hits of four kill twenty-five");
    assert_eq!(
        died - first,
        expected_ticks,
        "{hits_needed} hits of {damage} at {reload_ticks}-tick intervals must take {expected_ticks} ticks"
    );
}

#[test]
fn the_attacker_stops_when_the_target_is_gone() {
    let Some(fx) = fixture() else { return };
    let (mut world, spearman, villager) = duel(&fx);
    world
        .submit_checked(Command::Attack {
            attacker: spearman,
            target: villager,
        })
        .expect("the order");
    for _ in 0..2_000 {
        world.step().expect("step");
        if world.entity(villager).is_none() {
            break;
        }
    }
    assert!(world.entity(villager).is_none(), "the villager dies");
    for _ in 0..20 {
        world.step().expect("step");
    }
    assert_eq!(
        world.entity(spearman).expect("spearman").task,
        Task::Idle,
        "a soldier with no target stands down instead of attacking a corpse"
    );
}

#[test]
fn attacking_your_own_unit_is_refused() {
    let Some(fx) = fixture() else { return };
    let (mut world, spearman, _) = duel(&fx);
    let own = world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Unit && e.id != spearman)
        .expect("a fellow villager")
        .id;
    match world.submit_checked(Command::Attack {
        attacker: spearman,
        target: own,
    }) {
        Err(SimError::FriendlyFire { id }) => assert_eq!(id, own),
        other => panic!("expected FriendlyFire, got {other:?}"),
    }
}

#[test]
fn a_dead_unit_does_not_leave_its_route_behind() {
    // Every unit that dies used to keep its remaining waypoints in `paths` forever: a long 4v4 with
    // hundreds of deaths would accumulate routes nobody walks, and the state hash walks them too.
    let Some(fx) = fixture() else { return };
    let (mut world, spearman, villager) = duel(&fx);

    // Give the villager a route of its own by sending it at the spearman.
    world
        .submit_checked(Command::Attack {
            attacker: villager,
            target: spearman,
        })
        .expect("the villager may attack");
    world.step().expect("step");
    assert!(
        world.path_len(villager).is_some(),
        "the order should have planned a route"
    );

    // Then let the spearman finish it. Damage comes from the attacker's stats, not the command, so
    // this takes the same seven blows the kill test already measures.
    world
        .submit_checked(Command::Attack {
            attacker: spearman,
            target: villager,
        })
        .expect("the spearman may attack");
    let mut steps = 0;
    while world.entity(villager).is_some() {
        world.step().expect("step");
        steps += 1;
        // Seven blows at a two-second reload is 280 ticks; anything near 40 would be the first hit.
        assert!(steps < 400, "the villager should have died by now");
    }
    assert_eq!(
        world.path_len(villager),
        None,
        "a dead unit's route must be released with it"
    );
    println!("villager died after {steps} ticks, route released");
}
