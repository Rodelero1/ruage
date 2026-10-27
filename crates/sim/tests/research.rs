//! Research: a tech is a price and a timer, and its effect is applied when it lands.
//!
//! This is the kind-3 slice of the effect interpreter: an **upgrade** replaces one unit with another
//! for the player who researched it (D17's importer keeps the meaning, the engine applies
//! "every 74 becomes 75"). The tech used throughout is the dat's Man-at-Arms: tech 222, effect 211,
//! `[3, 74, 75, -1, 0]`, 100 food + 40 gold, 40 s — the published cost and time, which is how the
//! tech was identified in the first place (never by id).

use std::path::PathBuf;

use ruage_data::{Economy, Effects, Map, Sim as Settings, Techs, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, SimError, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// The project pack's column for a resource, so a test says "food" the way content does.
fn slot(name: &str) -> usize {
    ruage_data::load_resources_file(&repo("config/resources.toml"))
        .expect("resources.toml")
        .slot_of(name)
        .unwrap_or_else(|| panic!("the pack has no {name}"))
}

/// Man-at-Arms, and the two units it swaps.
const MAN_AT_ARMS: u32 = 222;
/// Unit 74 is the **militia** (40 hp, 60 food/20 gold, 21 s) - the dat's name field calls it SPRMN,
/// but the upgrade graph is decisive: the Man-at-Arms, Long Swordsman, Two-Handed Swordsman and
/// Champion effects all upgrade *from* 74.
const MILITIA: u32 = 74;
/// The spear line: spearman 93 (45 hp), pikeman 358 (55 hp), halberdier 359 (60 hp).
const SPEARMAN: u32 = 93;
const PIKEMAN: u32 = 358;
const HALBERDIER: u32 = 359;
/// The upgrades: tech 197 (45 s, 215 food + 90 gold, Castle Age) and 429 (50 s, 300 food + 600 gold,
/// Imperial, and it upgrades spearmen *and* pikemen).
const PIKEMAN_TECH: u32 = 197;
const HALBERDIER_TECH: u32 = 429;
const SWORDSMAN: u32 = 75;
const BARRACKS: u32 = 12;
const TOWN_CENTRE: u32 = 109;
/// The techs the pack's `[start] techs` names, and the one every other tech needs.
const DARK_AGE: u32 = 104;
const FEUDAL_AGE: u32 = 101;
const LOOM: u32 = 22;
/// The Hussar upgrade (428) and the building it lives in: on the British restriction list.
const HUSSAR: u32 = 428;
const STABLE: u32 = 101;
const SMITH: u32 = 103;
const CASTLE: u32 = 82;
const CASTLE_MAKE_AVAIL: u32 = 137;
const BARRACKS_MAKE_AVAIL: u32 = 220;
const TREBUCHET: u32 = 331;
const TREBUCHET_MAKE_AVAIL: u32 = 256;
const FORGING: u32 = 67;

struct Fixture {
    settings: Settings,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
    combat: ruage_data::Combat,
    techs: Techs,
    effects: Effects,
    civs: ruage_data::Civs,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    let techs = repo("config/gamedata/techs.toml");
    let effects = repo("config/gamedata/effects.toml");
    if !units.exists() || !techs.exists() || !effects.exists() {
        eprintln!("skipping: generated data is absent (D8)");
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
        techs: ruage_data::load_techs_file(&techs).expect("techs.toml"),
        effects: ruage_data::load_effects_file(&effects).expect("effects.toml"),
        civs: ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs.toml"),
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
    world.load_combat(&fx.combat);
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("the tech and effect tables");
    world.load_civs(&fx.civs).expect("the civ table");
    world
}

/// The same, with the civ table supplied by the caller: for the test that gives content a civ the
/// table does not have.
fn seeded_with_civs(fx: &Fixture, economy: &ruage_data::Economy, civs: &ruage_data::Civs) -> World {
    let mut world = seeded_with(fx, economy);
    world.load_civs(civs).expect("the civ table");
    world
}

/// The same world with a different economy: how a test says "this pack opens broke", which the
/// project's own pack no longer does.
fn seeded_with(fx: &Fixture, economy: &ruage_data::Economy) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("the research tables load");
    world
}

/// The first tile beside a villager that is free of entities: enough for a build order in a test.
fn free_tile(world: &World, from: EntityId) -> (u16, u16) {
    let position = world.position(from).expect("position");
    let scale = world.scale();
    let x = u16::try_from(position.0.raw() / scale).expect("in range");
    let y = u16::try_from(position.1.raw() / scale).expect("in range");
    for radius in 1..8u16 {
        for dy in 0..=radius {
            for dx in 0..=radius {
                let candidate = (x + dx, y + dy);
                let occupied = world.entities().iter().any(|entity| {
                    entity.position
                        == (
                            ruage_fixed::Fixed::from_raw(i64::from(candidate.0) * scale),
                            ruage_fixed::Fixed::from_raw(i64::from(candidate.1) * scale),
                        )
                });
                if !occupied {
                    return candidate;
                }
            }
        }
    }
    panic!("nowhere to build");
}

/// Place an order and step until the tech lands, or fail saying it never did.
fn research(world: &mut World, player: u8, building: EntityId, tech: u32, seconds: u32) {
    world
        .submit_checked(Command::Research { building, tech })
        .expect("the research order is legal");
    for _ in 0..u64::from(seconds) * u64::from(world.tick_rate_hz()) + 40 {
        world.step().expect("step");
        if world.researched(player).contains(&tech) {
            return;
        }
    }
    panic!("tech {tech} never landed");
}

fn first_villager(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Unit)
        .expect("a villager")
        .id
}

/// Gather until the bank holds at least `want` in a column, re-tasking the villager whenever its
/// node runs dry (the engine has no "seek the next node" yet, so the test drives it).
fn bank(world: &mut World, slot: usize, want: u32) {
    let villager = first_villager(world);
    let mut node = nearest_node(world, villager, slot).expect("a node of that column");
    world.submit(Command::Gather { villager, node });
    for _ in 0..400_000 {
        world.step().expect("step");
        if world.stock(1, slot) >= want {
            return;
        }
        let idle = world
            .entity(villager)
            .is_some_and(|entity| entity.task == ruage_sim::Task::Idle);
        let empty = world
            .entity(node)
            .is_none_or(|entity| entity.amount.raw() <= 0);
        if idle || empty {
            let Some(next) = nearest_node(world, villager, slot) else {
                break;
            };
            node = next;
            world.submit(Command::Gather { villager, node });
        }
    }
    panic!(
        "could not bank {want} in column {slot}; the bank holds {}",
        world.stock(1, slot)
    );
}

fn nearest_node(world: &World, from: EntityId, slot: usize) -> Option<EntityId> {
    let here = world.position(from)?;
    world
        .entities()
        .iter()
        .filter(|e| e.kind == EntityKind::Resource(slot) && e.amount.raw() > 0)
        .min_by_key(|e| {
            let dx = e.position.0.raw() - here.0.raw();
            let dy = e.position.1.raw() - here.1.raw();
            (dx * dx + dy * dy, e.id.0)
        })
        .map(|e| e.id)
}

/// A finished building, spawned rather than built: these tests are about research, not masonry.
fn place(world: &mut World, player: u8, id: u32, unit: u32, tile: (i64, i64), hp: i32) -> EntityId {
    let scale = world.scale();
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: if unit == MILITIA {
                EntityKind::Unit
            } else {
                EntityKind::Building
            },
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
    // Commands are queued (`CommandApply::NextTick`), so the entity exists after one step.
    world.step().expect("step");
    id
}

/// Put the world in the Feudal Age, because every tech in this file lists it as a prerequisite.
///
/// **Order matters twice.** Grant before placing the buildings a test researches at: the age effect
/// carries kind-3 commands that upgrade building ids (town centre 109 -> 71, barracks 12 -> 498), so a
/// building created later keeps the id the tech names. And grant *after* gathering, because the town
/// centre's upgrade means the villagers' drop sites no longer match it.
fn feudal(world: &mut World) {
    // 104 comes from the pack's `[start] techs`; 659 is the other parent of Feudal and has to be
    // granted, then Feudal itself.
    for tech in [659, FEUDAL_AGE] {
        world.grant_tech(1, tech).expect("seeding the Feudal Age");
    }
}

fn money(world: &mut World) {
    bank(world, slot("food"), 100);
    bank(world, slot("gold"), 40);
}

#[test]
fn the_man_at_arms_effect_is_an_upgrade_and_nothing_else() {
    // The slice is only honest if the tech it uses is genuinely a kind-3 effect. If the dat ever
    // gives effect 211 a second kind, this test says so rather than the interpreter silently
    // applying half a tech.
    let Some(fx) = fixture() else { return };
    let tech = fx.techs.get(MAN_AT_ARMS).expect("tech 222");
    assert_eq!(tech.research_time, 40, "40 s, as published");
    let effect_id = tech.effect_id.expect("an effect");
    let effect = fx.effects.get(effect_id).expect("effect 211");
    let kinds: Vec<i16> = effect.commands.iter().map(|c| c.kind).collect();
    assert_eq!(kinds, vec![3], "a pure upgrade");
    let command = effect.commands[0];
    assert_eq!(
        (command.a, command.b),
        (MILITIA as i16, SWORDSMAN as i16),
        "74 becomes 75"
    );
}

#[test]
fn research_is_refused_without_the_money() {
    let Some(fx) = fixture() else { return };
    // The pack opens with the standard stockpile - enough to pay for Man-at-Arms twice over - so this
    // world starts broke: a pack with no `[start] stock` is how content says that.
    let mut broke = fx.economy.clone();
    broke.start.stock.clear();
    let mut world = seeded_with(&fx, &broke);
    feudal(&mut world);
    let barracks = place(&mut world, 1, 9_000, BARRACKS, (8, 8), 1200);

    let err = world
        .submit_checked(Command::Research {
            building: barracks,
            tech: MAN_AT_ARMS,
        })
        .expect_err("100 food and 40 gold are not free");
    match err {
        SimError::NotEnough {
            ref resource,
            need,
            have,
        } => {
            assert_eq!((resource.as_str(), need, have), ("food", 100, 0));
        }
        other => panic!("expected NotEnough, got {other:?}"),
    }
    assert!(
        !world.researched(1).contains(&MAN_AT_ARMS),
        "the refused order researched nothing: {:?}",
        world.researched(1)
    );
}

#[test]
fn research_needs_the_building_the_tech_names_and_a_finished_one() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    money(&mut world);

    // A town centre is a building, but Man-at-Arms is a barracks tech (location 12).
    let centre = place(&mut world, 1, 9_001, TOWN_CENTRE, (10, 10), 2400);
    let err = world
        .submit_checked(Command::Research {
            building: centre,
            tech: MAN_AT_ARMS,
        })
        .expect_err("the wrong building cannot research this");
    assert!(
        matches!(err, SimError::CannotResearchHere { .. }),
        "expected CannotResearchHere, got {err:?}"
    );

    // A barracks still under construction cannot, so build one for real.
    bank(&mut world, slot("wood"), 175);
    world
        .submit_checked(Command::Build {
            villager: first_villager(&world),
            unit: BARRACKS,
            at: (14, 14),
        })
        .expect("a barracks costs 175 wood, which the villager just banked");
    world.step().expect("step");
    let under_construction = world
        .entities()
        .iter()
        .find(|e| e.unit == Some(BARRACKS) && e.construction.is_some())
        .map(|e| e.id)
        .expect("the site exists");
    let err = world
        .submit_checked(Command::Research {
            building: under_construction,
            tech: MAN_AT_ARMS,
        })
        .expect_err("a building site cannot research");
    assert!(
        matches!(err, SimError::BuildingNotReady { .. }),
        "expected BuildingNotReady, got {err:?}"
    );

    // And a unit cannot research at all.
    let villager = first_villager(&world);
    let err = world
        .submit_checked(Command::Research {
            building: villager,
            tech: MAN_AT_ARMS,
        })
        .expect_err("only buildings research");
    assert!(
        matches!(err, SimError::TrainNeedsABuilding { .. }),
        "expected TrainNeedsABuilding, got {err:?}"
    );
}

#[test]
fn a_tech_cannot_be_researched_twice() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    money(&mut world);
    money(&mut world);
    feudal(&mut world);
    let barracks = place(&mut world, 1, 9_003, BARRACKS, (16, 16), 1200);

    world
        .submit_checked(Command::Research {
            building: barracks,
            tech: MAN_AT_ARMS,
        })
        .expect("the first order is legal");
    for _ in 0..900 {
        world.step().expect("step");
    }
    assert!(world.researched(1).contains(&MAN_AT_ARMS), "it landed");

    let err = world
        .submit_checked(Command::Research {
            building: barracks,
            tech: MAN_AT_ARMS,
        })
        .expect_err("a tech is researched once");
    assert!(
        matches!(err, SimError::AlreadyResearched { tech } if tech == MAN_AT_ARMS),
        "expected AlreadyResearched, got {err:?}"
    );
}

#[test]
fn research_takes_the_techs_own_time_and_then_upgrades_what_its_owner_has() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    money(&mut world);
    feudal(&mut world);
    let barracks = place(&mut world, 1, 9_004, BARRACKS, (18, 18), 1200);
    // Two of the player's spearmen: one untouched, one at half health, so the fraction can be seen.
    let whole = place(&mut world, 1, 9_010, MILITIA, (18, 20), 40);
    let hurt = place(&mut world, 1, 9_011, MILITIA, (20, 18), 20);
    // And one of the opponent's, which must not be touched.
    let enemy = place(&mut world, 2, 9_012, MILITIA, (20, 20), 40);

    let before = world.stock(1, slot("food"));
    let gold_before = world.stock(1, slot("gold"));
    world
        .submit_checked(Command::Research {
            building: barracks,
            tech: MAN_AT_ARMS,
        })
        .expect("the order is legal");

    // The order lands on the next tick, which is when the price is paid (`NextTick`).
    world.step().expect("step");
    assert_eq!(world.stock(1, slot("food")), before - 100, "food paid");
    assert_eq!(world.stock(1, slot("gold")), gold_before - 40, "gold paid");
    let started = world.tick();

    // 40 s at 20 Hz is 800 ticks, counted from the tick the order landed.
    let ticks = u64::from(40 * world.tick_rate_hz());
    for _ in 0..ticks - 1 {
        world.step().expect("step");
    }
    assert_eq!(
        world.entity(whole).expect("still there").unit,
        Some(MILITIA),
        "one tick before its time, nothing has happened yet"
    );

    world.step().expect("step");
    assert_eq!(
        world.tick() - started,
        ticks,
        "the tech lands exactly research_time x tick_rate ticks after it started"
    );
    assert!(world.researched(1).contains(&MAN_AT_ARMS));

    // The whole unit is now a swordsman at its full health.
    let upgraded = world.entity(whole).expect("still there");
    assert_eq!(upgraded.unit, Some(SWORDSMAN), "74 became 75");
    assert_eq!(upgraded.hp, 45, "45 hp is the swordsman's maximum");

    // The damaged one keeps its *fraction*: 20 of 40 is half, so 45 becomes 22.5, rounded up.
    let wounded = world.entity(hurt).expect("still there");
    assert_eq!(wounded.unit, Some(SWORDSMAN));
    assert_eq!(
        wounded.hp, 23,
        "half health stays half health (45 / 2, rounded up)"
    );

    // And the opponent's identical unit is untouched.
    assert_eq!(
        world.entity(enemy).expect("still there").unit,
        Some(MILITIA),
        "an upgrade is the researching player's alone"
    );
}

#[test]
fn the_researched_set_is_part_of_the_state_hash() {
    let Some(fx) = fixture() else { return };
    let mut a = seeded(&fx);
    let mut b = seeded(&fx);
    money(&mut a);
    money(&mut b);
    feudal(&mut a);
    feudal(&mut b);
    let barracks_a = place(&mut a, 1, 9_005, BARRACKS, (22, 22), 1200);
    let _barracks_b = place(&mut b, 1, 9_005, BARRACKS, (22, 22), 1200);
    assert_eq!(a.state_hash(), b.state_hash(), "identical worlds agree");

    // A third world that researches the same tech at the same tick, so "deterministic" is checked
    // for the new path and not just assumed from the old ones.
    let mut c = seeded(&fx);
    money(&mut c);
    feudal(&mut c);
    let barracks_c = place(&mut c, 1, 9_005, BARRACKS, (22, 22), 1200);

    a.submit_checked(Command::Research {
        building: barracks_a,
        tech: MAN_AT_ARMS,
    })
    .expect("legal");
    c.submit_checked(Command::Research {
        building: barracks_c,
        tech: MAN_AT_ARMS,
    })
    .expect("legal");
    for _ in 0..900 {
        a.step().expect("step");
        b.step().expect("step");
        c.step().expect("step");
    }
    assert_ne!(
        a.state_hash(),
        b.state_hash(),
        "a researched tech must change the hash: it changes what later orders resolve to"
    );
    assert_eq!(
        a.state_hash(),
        c.state_hash(),
        "two worlds that researched the same tech agree, so the new state is deterministic"
    );
}

#[test]
fn the_same_tech_cannot_be_researched_at_two_buildings_at_once() {
    // Two barracks, one tech, two orders: without this check both would be accepted, both would
    // charge, and the second would re-apply an upgrade that has nothing left to upgrade. Paying
    // twice for one tech is the kind of bug no endpoint assertion notices.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    // Enough for two techs, so the refusal under test cannot be "cannot afford".
    bank(&mut world, slot("food"), 200);
    bank(&mut world, slot("gold"), 80);
    feudal(&mut world);
    let first = place(&mut world, 1, 9_020, BARRACKS, (24, 24), 1200);
    let second = place(&mut world, 1, 9_021, BARRACKS, (26, 26), 1200);

    world
        .submit_checked(Command::Research {
            building: first,
            tech: MAN_AT_ARMS,
        })
        .expect("the first order is legal");
    world.step().expect("step");
    let paid_once = world.stock(1, slot("food"));

    let err = world
        .submit_checked(Command::Research {
            building: second,
            tech: MAN_AT_ARMS,
        })
        .expect_err("a tech already in progress cannot be started again");
    assert!(
        matches!(err, SimError::TechInProgress { tech } if tech == MAN_AT_ARMS),
        "expected TechInProgress, got {err:?}"
    );
    world.step().expect("step");
    assert_eq!(
        world.stock(1, slot("food")),
        paid_once,
        "a refused order must not be paid"
    );
}

/// 222 Man-at-Arms lists `required = [101]` in the table, and 101 is the Feudal Age (500 food, at the
/// town centre). Requirements are *tech* ids - all 614 entries across the table resolve to techs, even
/// where the number also happens to be a building - so the gate is "has this player researched it".
#[test]
fn a_tech_whose_prerequisite_is_not_researched_is_refused() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let barracks = place(&mut world, 1, 9_300, BARRACKS, (14, 14), 1200);
    money(&mut world);

    let err = world
        .submit_checked(Command::Research {
            building: barracks,
            tech: MAN_AT_ARMS,
        })
        .expect_err("Man-at-Arms before the Feudal Age is not legal")
        .to_string();
    assert!(
        err.contains("feudal_age") || err.contains("101"),
        "the refusal names the missing prerequisite, and prefers the content word: {err}"
    );
    assert!(
        !world.researched(1).contains(&MAN_AT_ARMS),
        "a refused order researches nothing"
    );
}

#[test]
fn a_tech_is_legal_once_its_prerequisite_is_researched() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    money(&mut world);
    feudal(&mut world);
    let barracks = place(&mut world, 1, 9_310, BARRACKS, (14, 14), 1200);
    world
        .submit_checked(Command::Research {
            building: barracks,
            tech: MAN_AT_ARMS,
        })
        .expect("Man-at-Arms is legal in the Feudal Age");

    for _ in 0..2_000 {
        world.step().expect("step");
        if world.researched(1).contains(&MAN_AT_ARMS) {
            break;
        }
    }
    assert!(
        world.researched(1).contains(&MAN_AT_ARMS),
        "the tech lands once researched"
    );
}

/// A match opens with the techs content says it does. The pack names `start.techs = ["dark_age"]`,
/// which is tech 104 - free, instant, and the prerequisite of every other tech in the table - so a
/// world that skipped it could research nothing, and every test would have to grant it by hand.
#[test]
fn a_world_opens_with_the_techs_the_pack_starts_with() {
    let Some(fx) = fixture() else { return };
    let world = seeded(&fx);
    for player in [1, 2] {
        assert!(
            world.researched(player).contains(&DARK_AGE),
            "player {player} opens with the Dark Age tech: {:?}",
            world.researched(player)
        );
    }
    assert!(
        !world.researched(1).contains(&FEUDAL_AGE),
        "and not with an age it has to earn"
    );

    // Loom lists 104 as its prerequisite, so it is legal on turn one - the point of starting with it.
    let mut world = seeded(&fx);
    let centre = place(&mut world, 1, 9_500, TOWN_CENTRE, (14, 14), 2400);
    bank(&mut world, slot("gold"), 50);
    world
        .submit_checked(Command::Research {
            building: centre,
            tech: LOOM,
        })
        .expect("Loom is legal in the opening position");
}

/// The tables can be loaded before the world is seeded - a headless runner may do either - and the
/// starting techs must arrive the same way in both orders. Granting them only where `load_research`
/// finds entities would make the match's opening depend on call order, which is exactly the kind of
/// bug that shows up as "research does nothing" in someone else's integration.
#[test]
fn starting_techs_arrive_whichever_order_the_tables_are_loaded() {
    let Some(fx) = fixture() else { return };
    let mut world = World::new(&fx.settings, 7).expect("world");
    world.load_research(&fx.techs, &fx.effects).expect("tables");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    assert!(
        world.researched(1).contains(&DARK_AGE),
        "the match opens with the Dark Age even when the tables came first"
    );
}

/// The British tech tree lists the Hussar upgrade (tech 428, scout 448 -> 441 HUSSAR) among the 31
/// techs they cannot research - one of the facts that pins the reading of a civ's tree effect as a
/// list of *restrictions* rather than of permissions.
#[test]
fn a_barred_tech_is_refused_by_the_civilisation() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded_with_civs(&fx, &fx.economy, &fx.civs);
    feudal(&mut world);
    // A stable is where the Hussar line lives; the civ bar is the reason under test, not the building.
    let stable = place(&mut world, 1, 9_600, STABLE, (14, 14), 1200);
    money(&mut world);

    let err = world
        .submit_checked(Command::Research {
            building: stable,
            tech: HUSSAR,
        })
        .expect_err("the British cannot research the Hussar")
        .to_string();
    assert!(err.contains("British"), "the refusal names the civ: {err}");

    // And it is a bar, not a blanket: a tech the civ does have is allowed through the same door.
    let smith = place(&mut world, 1, 9_601, SMITH, (16, 16), 1_000);
    world
        .submit_checked(Command::Research {
            building: smith,
            tech: FORGING,
        })
        .expect("Forging is not on the British list");
}

#[test]
fn a_pack_naming_a_civilisation_the_table_lacks_is_refused() {
    let Some(fx) = fixture() else { return };
    let civs = fx.civs.clone();
    let mut economy = fx.economy.clone();
    economy.start.civ = Some("Martian".to_owned());
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &economy, &fx.units)
        .expect("seeding");
    let err = world
        .load_civs(&civs)
        .expect_err("there is no civ called Martian")
        .to_string();
    assert!(err.contains("Martian"), "the refusal names it: {err}");
}

/// Availability: a unit that some tech *makes available* cannot be built or trained before that tech
/// lands. The castle is the cleanest case in the data - tech 137 "Castle -- Age Three" (free, instant,
/// no home building) requires the Castle Age (102) and enables unit 82.
#[test]
fn a_castle_cannot_be_built_before_the_age_that_enables_it() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    // Enough for a castle (650 stone), so the refusal under test is availability and not the price.
    bank(&mut world, slot("food"), 400);
    bank(&mut world, slot("stone"), 700);
    let site = free_tile(&world, villager);

    let err = world
        .submit_checked(Command::Build {
            villager,
            unit: CASTLE,
            at: site,
        })
        .expect_err("a castle is not available in the Dark Age")
        .to_string();
    assert!(
        err.contains("castle") || err.contains("82"),
        "the refusal names the unit: {err}"
    );

    // The Castle Age arrives; its bookkeeping tech (137) is free and instant, so it lands with it.
    feudal(&mut world);
    world.grant_tech(1, 102).expect("the Castle Age");
    assert!(
        world.researched(1).contains(&CASTLE_MAKE_AVAIL),
        "enabling a unit is bookkeeping, so the engine applies it: {:?}",
        world.researched(1)
    );
    world
        .submit_checked(Command::Build {
            villager,
            unit: CASTLE,
            at: site,
        })
        .expect("and now the castle is available");
}

/// The other half: a unit nothing gates stays ungated. `[train.entry]` trains spearmen from the Dark
/// Age and a villager is a villager, so availability must not turn into a blanket lock.
#[test]
fn a_unit_that_no_tech_gates_is_unaffected() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let villager = first_villager(&world);
    let site = free_tile(&world, villager);
    bank(&mut world, slot("wood"), 100);
    world
        .submit_checked(Command::Build {
            villager,
            unit: 70,
            at: site,
        })
        .expect("a house needs no availability tech");

    // And the barracks is available from the Dark Age, because tech 220 needs only that.
    assert!(
        world.researched(1).contains(&BARRACKS_MAKE_AVAIL),
        "the match opens with the barracks available: {:?}",
        world.researched(1)
    );
}

/// Training obeys the same rule as building. The trebuchet (331) is made available by tech 256, which
/// requires the Imperial Age (103) - so a castle at the Castle Age can hold everything except it.
#[test]
fn a_trebuchet_cannot_be_trained_before_the_imperial_age() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    feudal(&mut world);
    world.grant_tech(1, 102).expect("the Castle Age");
    let castle = place(&mut world, 1, 9_700, CASTLE, (14, 14), 4800);
    bank(&mut world, slot("wood"), 400);
    bank(&mut world, slot("gold"), 400);

    let err = world
        .submit_checked(Command::Train {
            building: castle,
            unit: TREBUCHET,
        })
        .expect_err("a trebuchet is not available before the Imperial Age")
        .to_string();
    assert!(err.contains("331"), "the refusal names the unit: {err}");

    world.grant_tech(1, 103).expect("the Imperial Age");
    assert!(
        world.researched(1).contains(&TREBUCHET_MAKE_AVAIL),
        "and the Imperial Age brings it: {:?}",
        world.researched(1)
    );
    world
        .submit_checked(Command::Train {
            building: castle,
            unit: TREBUCHET,
        })
        .expect("now it can be trained");
}

/// The spear line end to end, as the game has it: a spearman exists once the Feudal Age brings the
/// availability tech, a pikeman needs a researched upgrade, and a barracks keeps training "the spear
/// line" as it advances - but never a unit whose upgrade has not been researched.
#[test]
fn a_barracks_trains_the_spear_line_only_as_far_as_it_is_unlocked() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let barracks = place(&mut world, 1, 9_800, BARRACKS, (14, 14), 1200);
    bank(&mut world, slot("food"), 600);
    bank(&mut world, slot("gold"), 300);

    // Dark Age: no spearman. The availability tech (87 "Pike") requires the Feudal Age.
    let err = world
        .submit_checked(Command::Train {
            building: barracks,
            unit: SPEARMAN,
        })
        .expect_err("a spearman is not available in the Dark Age")
        .to_string();
    assert!(err.contains("93"), "the refusal names the unit: {err}");

    feudal(&mut world);
    world
        .submit_checked(Command::Train {
            building: barracks,
            unit: SPEARMAN,
        })
        .expect("the Feudal Age makes spearmen available");

    // The pikeman is an *upgrade*, not an availability: the barracks may train it only once the
    // Pikeman tech has been researched, even though it is the same line.
    feudal(&mut world);
    world.grant_tech(1, 102).expect("the Castle Age");
    let err = world
        .submit_checked(Command::Train {
            building: barracks,
            unit: PIKEMAN,
        })
        .expect_err("a pikeman needs its upgrade researched")
        .to_string();
    assert!(
        err.contains("358") || err.contains("197"),
        "the refusal names the unit or its tech: {err}"
    );

    bank(&mut world, slot("food"), 300);
    bank(&mut world, slot("gold"), 200);
    research(&mut world, 1, barracks, PIKEMAN_TECH, 45);
    world
        .submit_checked(Command::Train {
            building: barracks,
            unit: PIKEMAN,
        })
        .expect("after the Pikeman tech the barracks trains pikemen");
}

/// The other half: the upgrade reaches units that already exist, and the halberdier tech upgrades a
/// spearman directly - that is how the effect spells it (`[3, 93, 359]` *and* `[3, 358, 359]`).
#[test]
fn the_spear_line_upgrades_the_units_the_player_already_has() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    feudal(&mut world);
    let barracks = place(&mut world, 1, 9_810, BARRACKS, (14, 14), 1200);
    let spearman = place(&mut world, 1, 9_811, SPEARMAN, (10, 10), 45);
    assert_eq!(
        world.entity(spearman).expect("spearman").unit,
        Some(SPEARMAN)
    );

    world.grant_tech(1, 102).expect("the Castle Age");
    bank(&mut world, slot("food"), 400);
    bank(&mut world, slot("gold"), 200);
    research(&mut world, 1, barracks, PIKEMAN_TECH, 45);
    let pikeman = world.entity(spearman).expect("the upgraded unit");
    assert_eq!(pikeman.unit, Some(PIKEMAN), "the spearman is now a pikeman");
    assert_eq!(pikeman.hp, 55, "45 of 45 becomes 55 of 55, not a reset");

    world.grant_tech(1, 103).expect("the Imperial Age");
    bank(&mut world, slot("food"), 400);
    bank(&mut world, slot("gold"), 700);
    research(&mut world, 1, barracks, HALBERDIER_TECH, 50);
    let halberdier = world.entity(spearman).expect("the upgraded unit");
    assert_eq!(
        halberdier.unit,
        Some(HALBERDIER),
        "and the halberdier tech takes the pikeman on"
    );
    assert_eq!(halberdier.hp, 60);
}
