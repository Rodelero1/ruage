//! Population and housing: what a player may support, and what happens at the ceiling.
//!
//! Written before the implementation (D7), and rewritten when the rule turned out to be the other one:
//! the ceiling does not refuse orders - a queue may hold more than the player can support - it **pauses
//! training** until a unit dies or a house finishes (D34).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_sim::{Command, EntityId, EntityKind, World};

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
    civs: ruage_data::Civs,
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
        civs: ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs.toml"),
        techs: ruage_data::load_techs_file(&repo("config/gamedata/techs.toml"))
            .expect("techs.toml"),
        effects: ruage_data::load_effects_file(&repo("config/gamedata/effects.toml"))
            .expect("effects.toml"),
    })
}

fn seeded(fx: &Fixture) -> World {
    seeded_with(fx, &fx.economy, true)
}

/// A world with food and wood to spare: every test here builds and trains, and the price of either is
/// not what is under test.
fn seeded_with(fx: &Fixture, economy: &Economy, civs: bool) -> World {
    let mut economy = economy.clone();
    economy.start.stock = [
        ("food".to_owned(), 4_000),
        ("wood".to_owned(), 4_000),
        ("gold".to_owned(), 1_000),
    ]
    .into_iter()
    .collect();
    let economy = &economy;
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    if civs {
        // The bar rules live in the civ's tech-tree effect, so the effect table comes with them.
        world
            .load_research(&fx.techs, &fx.effects)
            .expect("research tables");
        world.load_civs(&fx.civs).expect("the civ table");
    }
    world
}

fn town_centre(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Building)
        .expect("the town centre")
        .id
}

fn first_villager(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Unit)
        .expect("a villager")
        .id
}

fn villagers(world: &World) -> usize {
    world
        .entities()
        .iter()
        .filter(|e| e.player == 1 && e.kind == EntityKind::Unit && e.unit == Some(VILLAGER))
        .count()
}

fn a_free_tile(world: &World) -> (u16, u16) {
    let scale = world.scale();
    let centre = world.entity(town_centre(world)).expect("tc");
    let origin = (
        u16::try_from(centre.position.0.raw() / scale).unwrap(),
        u16::try_from(centre.position.1.raw() / scale).unwrap(),
    );
    for radius in 2..8u16 {
        for (x, y) in [
            (origin.0 + radius, origin.1),
            (origin.0, origin.1 + radius),
            (origin.0 + radius, origin.1 + radius),
        ] {
            if world.is_free_ground(x, y) {
                return (x, y);
            }
        }
    }
    panic!("no free ground near the town centre");
}

fn a_house_site(world: &mut World) -> EntityId {
    let villager = first_villager(world);
    let at = a_free_tile(world);
    world.submit(Command::Build {
        villager,
        unit: HOUSE,
        at,
    });
    world.step().expect("step");
    world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Building && e.unit == Some(HOUSE))
        .map(|e| e.id)
        .next()
        .expect("the house site exists")
}

fn finish(world: &mut World, site: EntityId) {
    for _ in 0..4_000 {
        if world.entity(site).map(|e| e.construction.is_none()) == Some(true) {
            return;
        }
        world.step().expect("step");
    }
    panic!("the building never finished");
}

fn a_finished_house(world: &mut World) {
    let site = a_house_site(world);
    finish(world, site);
}

/// Order villagers one at a time until one stops arriving - the ceiling pausing it - and return how
/// many arrived. The order that stalls is still queued when this returns.
fn fill_to_the_ceiling(world: &mut World) -> usize {
    let centre = town_centre(world);
    let start = villagers(world);
    for arrived in 0..40 {
        world
            .submit_checked(Command::Train {
                building: centre,
                unit: VILLAGER,
            })
            .expect("a queue may hold more than the player can support");
        let want = start + arrived + 1;
        let mut seen = false;
        for _ in 0..4_000 {
            world.step().expect("step");
            if villagers(world) >= want {
                seen = true;
                break;
            }
        }
        if !seen {
            return arrived;
        }
    }
    panic!("the player trained forty villagers: the ceiling is not pausing anything");
}

const HOUSE: u32 = 70;
const VILLAGER: u32 = 83;
const SPEARMAN: u32 = 93;

#[test]
fn a_town_centre_supports_five_units() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    // Three villagers start and the town centre holds five, so two more arrive and no more.
    assert_eq!(villagers(&world), 3);
    assert_eq!(fill_to_the_ceiling(&mut world), 2);
    assert_eq!(villagers(&world), 5);
}

/// The order is not refused at the ceiling: it is held, and its clock does not run.
#[test]
fn a_full_player_holds_the_order_and_pauses_the_clock() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    fill_to_the_ceiling(&mut world);
    let centre = town_centre(&world);
    world
        .submit_checked(Command::Train {
            building: centre,
            unit: VILLAGER,
        })
        .expect("a queue may hold more than the player can support");
    world.step().expect("step");
    let paused = world
        .entity(centre)
        .and_then(|entity| entity.training)
        .expect("the job started");
    let before = villagers(&world);

    for _ in 0..2_000 {
        world.step().expect("step");
    }
    assert_eq!(villagers(&world), before, "nothing arrived");
    let still = world
        .entity(centre)
        .and_then(|entity| entity.training)
        .expect("the job is still there");
    assert_eq!(
        still.remaining_ticks, paused.remaining_ticks,
        "a paused job does not count down"
    );
}

#[test]
fn a_finished_house_lets_a_paused_unit_arrive() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    fill_to_the_ceiling(&mut world);
    let centre = town_centre(&world);
    world
        .submit_checked(Command::Train {
            building: centre,
            unit: VILLAGER,
        })
        .expect("the order is held");
    world.step().expect("step");
    let before = villagers(&world);

    a_finished_house(&mut world);
    for _ in 0..6_000 {
        world.step().expect("step");
        if villagers(&world) > before {
            return;
        }
    }
    panic!("the house finished and the held villager still never arrived");
}

/// Killing a unit frees its population, and the held order is the thing that notices.
#[test]
fn a_dead_unit_lets_a_paused_unit_arrive() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    fill_to_the_ceiling(&mut world);
    let centre = town_centre(&world);
    world
        .submit_checked(Command::Train {
            building: centre,
            unit: VILLAGER,
        })
        .expect("the order is held");
    world.step().expect("step");
    let before = villagers(&world);

    // An enemy spearman beside one of the player's villagers: seven blows and a slot is free.
    let scale = world.scale();
    let victim = first_villager(&world);
    let spot = world.position(victim).expect("position");
    let attacker = EntityId(9_999);
    world
        .submit_checked(Command::Spawn {
            id: attacker,
            player: 2,
            kind: EntityKind::Unit,
            unit: Some(SPEARMAN),
            position: (spot.0, ruage_fixed::Fixed::from_raw(spot.1.raw() + scale)),
            speed: ruage_fixed::Fixed::ZERO,
            hp: 200,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    world
        .submit_checked(Command::Attack {
            attacker,
            target: victim,
        })
        .expect("attack");

    for _ in 0..6_000 {
        world.step().expect("step");
        if villagers(&world) < before {
            break;
        }
    }
    assert!(villagers(&world) < before, "the villager died");
    for _ in 0..6_000 {
        world.step().expect("step");
        if villagers(&world) >= before {
            return;
        }
    }
    panic!("a death freed population and the held villager still never arrived");
}

/// The Huns cannot build houses at all (their tech tree takes the house away), so a cap that counts
/// houses would leave them at zero for the whole match. Content's ceiling is their ceiling.
#[test]
fn a_civilisation_that_cannot_build_houses_gets_the_whole_cap() {
    let Some(fx) = fixture() else { return };
    let mut economy = fx.economy.clone();
    economy.start.civs = vec!["Huns".to_owned()];
    let mut world = seeded_with(&fx, &economy, true);
    let centre = town_centre(&world);
    let before = villagers(&world);
    for _ in 0..20 {
        // The queue holds five, so this keeps it fed rather than trying to place twenty at once.
        if world
            .submit_checked(Command::Train {
                building: centre,
                unit: VILLAGER,
            })
            .is_err()
        {
            for _ in 0..4_000 {
                world.step().expect("step");
                if world
                    .entity(centre)
                    .is_some_and(|entity| entity.training.is_none())
                {
                    break;
                }
            }
            world
                .submit_checked(Command::Train {
                    building: centre,
                    unit: VILLAGER,
                })
                .expect("the Huns train without housing");
        }
        world.step().expect("step");
    }
    for _ in 0..80_000 {
        world.step().expect("step");
        if villagers(&world) >= before + 20 {
            return;
        }
    }
    panic!(
        "the Huns trained {} of 20: something capped them by houses",
        villagers(&world) - before
    );
}

/// The ceiling is content's, not the sum of the buildings: a small `cap_default` holds even with a house.
#[test]
fn the_cap_stops_at_what_content_allows() {
    let Some(fx) = fixture() else { return };
    let mut economy = fx.economy.clone();
    economy.population.cap_default = 8;
    let mut world = seeded_with(&fx, &economy, true);
    a_finished_house(&mut world);
    // Three villagers and a ceiling of eight: five arrive, where house plus town centre would say
    // thirteen.
    assert_eq!(fill_to_the_ceiling(&mut world), 5);
    assert_eq!(villagers(&world), 8);
}

/// A queue may be longer than the ceiling, and it drains as the population frees up - that is the rule
/// the ceiling pause exists for.
#[test]
fn a_queue_can_exceed_the_ceiling_and_drains_later() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = town_centre(&world);
    for placed in 0..5 {
        world
            .submit_checked(Command::Train {
                building: centre,
                unit: VILLAGER,
            })
            .unwrap_or_else(|err| panic!("order {placed} refused: {err:?}"));
    }
    world.step().expect("step");
    for _ in 0..8_000 {
        world.step().expect("step");
    }
    assert_eq!(
        villagers(&world),
        5,
        "three started, two arrived, the rest wait"
    );

    a_finished_house(&mut world);
    for _ in 0..20_000 {
        world.step().expect("step");
        if villagers(&world) >= 8 {
            return;
        }
    }
    panic!(
        "the queue did not drain after the house: only {} villagers",
        villagers(&world)
    );
}

/// The Slavs team bonus is "military buildings provide +5 population", and it arrives in the dat as an
/// effect command of **kind 0** - a unit-addressed `set` of attribute 21, which is population provided.
/// This is the first consumer of that kind, and the first thing to apply a civ's *team bonus* effect.
///
/// Observed the way a player observes it: the ceiling is what it is, so a Slav who has built a barracks
/// can support five more units than the same Slav without one - and a Briton gains nothing from theirs.
#[test]
fn a_slav_barracks_supports_five_more_units_and_a_british_one_none() {
    let Some(fx) = fixture() else { return };
    for (civ, extra) in [("Slavs", 5usize), ("British", 0usize)] {
        let plain = arrivals(&fx, civ, false);
        let with_barracks = arrivals(&fx, civ, true);
        assert_eq!(
            with_barracks,
            plain + extra,
            "{civ}'s barracks is worth {extra} more units at the ceiling"
        );
    }
}

/// How many villagers a player of this civ can add before the ceiling stalls training, optionally with a
/// finished barracks in the yard. A fresh world each time: a stalled order left queued would be counted as
/// an arrival by the next measurement.
fn arrivals(fx: &Fixture, civ: &str, barracks: bool) -> usize {
    let mut economy = fx.economy.clone();
    economy.start.civs = vec![civ.to_owned()];
    let mut world = seeded_with(fx, &economy, true);
    if barracks {
        let villager = first_villager(&world);
        let at = a_free_tile(&world);
        world.submit(Command::Build {
            villager,
            unit: BARRACKS,
            at,
        });
        world.step().expect("step");
        let site = world
            .entities()
            .iter()
            .find(|e| e.kind == EntityKind::Building && e.unit == Some(BARRACKS))
            .map(|e| e.id)
            .expect("the barracks site exists");
        finish(&mut world, site);
    }
    fill_to_the_ceiling(&mut world)
}

const BARRACKS: u32 = 12;
