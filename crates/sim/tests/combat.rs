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
    resources: ruage_data::Resources,
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
    // The pack first: the map names its resources and only the pack turns a name into a column.
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        resources,
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
/// A world on a given economy, for tests that vary content (a team, a civ) rather than behaviour.
fn world_with(fx: &Fixture, economy: &Economy) -> World {
    let mut world = World::new(&fx.settings, 3).expect("world");
    world
        .seed_from(&fx.map, economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    // Accuracy is its own test (`ballistics.rs`): every archer here fires at a target that stands still,
    // and a scatter roll would make a hit depend on the tick the test happened to fire on.
    let mut combat = fx.combat.clone();
    combat.accuracy_scatter = ruage_data::MaybeFixed::from_f64(0.0);
    world.load_combat(&combat);
    world
}

/// Put a unit somewhere without asking the economy first.
fn place(world: &mut World, player: u8, id: u32, unit: u32, tile: (i64, i64), hp: i32) -> EntityId {
    let scale = world.scale();
    let id = EntityId(id);
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
            speed: Fixed::ZERO,
            hp,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

fn duel(fx: &Fixture) -> (World, EntityId, EntityId) {
    let mut world = World::new(&fx.settings, 3).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
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

/// A team is content: `[start] teams` names it per player, and two players on one team cannot attack
/// each other. Without it every player is their own team, which is what the rest of this suite assumes.
#[test]
fn players_on_one_team_cannot_attack_each_other() {
    let Some(fx) = fixture() else { return };
    let mut economy = fx.economy.clone();
    economy.start.teams = vec![1, 1];
    let mut world = world_with(&fx, &economy);
    let mine = place(&mut world, 1, 9_900, 74, (10, 10), 40);
    let theirs = place(&mut world, 2, 9_901, 74, (11, 10), 40);

    let err = world
        .submit_checked(Command::Attack {
            attacker: mine,
            target: theirs,
        })
        .expect_err("allies do not fight each other")
        .to_string();
    assert!(err.contains("team"), "the refusal says why: {err}");
}

#[test]
fn players_without_a_team_are_each_their_own() {
    let Some(fx) = fixture() else { return };
    let mut world = world_with(&fx, &fx.economy);
    let mine = place(&mut world, 1, 9_910, 74, (10, 10), 40);
    let theirs = place(&mut world, 2, 9_911, 74, (11, 10), 40);
    world
        .submit_checked(Command::Attack {
            attacker: mine,
            target: theirs,
        })
        .expect("with no teams named, everybody is an enemy of everybody");
}

// --- ranged combat -------------------------------------------------------------------------------
//
// Attribute 1 is `range`, in tiles: 277 of the 1,166 units have it above zero, and until now every
// one of them closed to adjacency and fought like a spearman.

const ARCHER: u32 = 4;

/// Put a unit somewhere with a speed, so it can walk to its range.
fn place_moving(
    world: &mut World,
    player: u8,
    id: u32,
    unit: u32,
    tile: (i64, i64),
    hp: i32,
    speed_tiles: f64,
) -> EntityId {
    let scale = world.scale();
    let id = EntityId(id);
    let speed = ruage_fixed::Fixed::from_raw((speed_tiles * scale as f64) as i64);
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
            speed,
            hp,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

fn tiles_between(world: &World, a: EntityId, b: EntityId) -> f64 {
    let scale = world.scale() as f64;
    let (pa, pb) = (world.position(a).expect("a"), world.position(b).expect("b"));
    let dx = (pb.0.raw() - pa.0.raw()) as f64 / scale;
    let dy = (pb.1.raw() - pa.1.raw()) as f64 / scale;
    (dx * dx + dy * dy).sqrt()
}

#[test]
fn an_archer_shoots_from_four_tiles_and_does_not_close() {
    let Some(fx) = fixture() else { return };
    let mut world = world_with(&fx, &fx.economy);
    world.load_combat(&fx.combat);
    let archer = place_moving(&mut world, 1, 9_920, ARCHER, (10, 10), 30, 0.96);
    let victim = place(&mut world, 2, 9_921, 83, (14, 10), 25);
    let before = world.entity(victim).expect("victim").hp;

    world
        .submit_checked(Command::Attack {
            attacker: archer,
            target: victim,
        })
        .expect("four tiles is inside an archer's range");
    for _ in 0..120 {
        world.step().expect("step");
    }

    assert!(
        world.entity(victim).expect("victim").hp < before,
        "the arrow landed"
    );
    assert_eq!(
        world.position(archer).expect("archer").0.raw() / world.scale(),
        10,
        "the archer did not walk: it was already in range"
    );
}

#[test]
fn an_archer_walks_into_range_and_then_stops() {
    let Some(fx) = fixture() else { return };
    let mut world = world_with(&fx, &fx.economy);
    world.load_combat(&fx.combat);
    let archer = place_moving(&mut world, 1, 9_930, ARCHER, (6, 10), 30, 0.96);
    // A villager would be dead long before the archer has finished walking and shooting; the point
    // here is where it stops, so the target is given health a test can keep looking at.
    let victim = place(&mut world, 2, 9_931, 83, (14, 10), 500);
    let before = world.entity(victim).expect("victim").hp;

    world
        .submit_checked(Command::Attack {
            attacker: archer,
            target: victim,
        })
        .expect("an attack order");
    // Eight tiles at 0.96 tiles a second is nine seconds of walking, then it shoots on a two-second
    // reload; the budget is a wide margin rather than a measured number.
    for _ in 0..1_600 {
        world.step().expect("step");
    }

    assert!(
        world.entity(victim).expect("victim").hp < before,
        "the archer got in range and shot"
    );
    let gap = tiles_between(&world, archer, victim);
    assert!(
        (3.9..5.0).contains(&gap),
        "the archer stopped at its range, not in melee: {gap} tiles"
    );
}

/// Range is attribute 1, so the upgrades that move it are the same kind-4 additions that move attack:
/// Fletching is +1 range on class 0, which is what an archer is. Five tiles is out of an archer's
/// reach and inside a Fletching archer's.
#[test]
fn fletching_adds_a_tile_of_reach() {
    let Some(fx) = fixture() else { return };
    let run = |research: bool| {
        let mut world = world_with(&fx, &fx.economy);
        world.load_combat(&fx.combat);
        if research {
            let techs = ruage_data::load_techs_file(&repo("config/gamedata/techs.toml"))
                .expect("techs.toml");
            let effects = ruage_data::load_effects_file(&repo("config/gamedata/effects.toml"))
                .expect("effects.toml");
            world
                .load_research(&techs, &effects)
                .expect("research tables");
            world.grant_tech(1, FLETCHING).expect("Fletching");
        }
        let archer = place_moving(&mut world, 1, 9_940, ARCHER, (10, 10), 30, 0.96);
        let victim = place(&mut world, 2, 9_941, 83, (15, 10), 500);
        world
            .submit_checked(Command::Attack {
                attacker: archer,
                target: victim,
            })
            .expect("an attack order");
        for _ in 0..200 {
            world.step().expect("step");
        }
        (
            world.position(archer).expect("archer").0.raw() / world.scale(),
            world.entity(victim).expect("victim").hp,
        )
    };

    let (walked_to, plain_hp) = run(false);
    let (stayed_at, fletched_hp) = run(true);
    assert_eq!(walked_to, 11, "without Fletching the archer closes a tile");
    assert_eq!(
        stayed_at, 10,
        "with Fletching it reaches from where it stands"
    );
    assert!(plain_hp < 500 && fletched_hp < 500, "both archers shot");
}

const FLETCHING: u32 = 199;

// --- projectiles --------------------------------------------------------------------------------
//
// A shot is an entity that flies: it takes time to land, it can be aimed at something that dies
// first, and it is not a unit - it does not count towards population and it cannot be attacked.

fn projectiles(world: &World) -> usize {
    world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Projectile)
        .count()
}

#[test]
fn an_arrow_takes_time_to_land() {
    let Some(fx) = fixture() else { return };
    let mut world = world_with(&fx, &fx.economy);
    world.load_combat(&fx.combat);
    let archer = place_moving(&mut world, 1, 9_950, ARCHER, (10, 10), 30, 0.96);
    let victim = place(&mut world, 2, 9_951, 83, (14, 10), 500);
    let before = world.entity(victim).expect("victim").hp;

    world
        .submit_checked(Command::Attack {
            attacker: archer,
            target: victim,
        })
        .expect("an attack order");
    world.step().expect("step");

    assert_eq!(
        world.entity(victim).expect("victim").hp,
        before,
        "the first tick of a four-tile shot cannot already have landed"
    );
    assert_eq!(projectiles(&world), 1, "there is an arrow in the air");
    assert_eq!(
        world
            .entities()
            .iter()
            .find(|e| e.kind == EntityKind::Projectile)
            .expect("arrow")
            .player,
        1,
        "the arrow belongs to the player who shot it"
    );

    // Four tiles at ten tiles a second is eight ticks at 20 Hz, plus slack for the reload.
    for _ in 0..200 {
        world.step().expect("step");
    }
    assert!(
        world.entity(victim).expect("victim").hp < before,
        "the arrow landed"
    );
}

/// An arrow is not a soldier: it must not be attackable, and it must not cost the shooter population.
#[test]
fn an_arrow_is_not_a_unit() {
    let Some(fx) = fixture() else { return };
    let mut world = world_with(&fx, &fx.economy);
    world.load_combat(&fx.combat);
    let archer = place_moving(&mut world, 1, 9_960, ARCHER, (10, 10), 30, 0.96);
    let victim = place(&mut world, 2, 9_961, 83, (14, 10), 500);
    // Player 2's villager exists before the shot is fired, because an arrow takes the next free id.
    let villager = place(&mut world, 2, 9_962, 83, (14, 11), 25);
    world
        .submit_checked(Command::Attack {
            attacker: archer,
            target: victim,
        })
        .expect("an attack order");
    world.step().expect("step");
    let arrow = world
        .entities()
        .iter()
        .find(|e| e.kind == EntityKind::Projectile)
        .map(|e| e.id)
        .expect("an arrow in the air");
    match world.submit_checked(Command::Attack {
        attacker: villager,
        target: arrow,
    }) {
        Err(SimError::NotATarget { id }) => assert_eq!(id, arrow),
        other => panic!("expected NotATarget, got {other:?}"),
    }
}
