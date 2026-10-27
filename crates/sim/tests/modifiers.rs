//! Modifiers: what a researched tech does to a unit's attacks and armours (kinds 4 and 5).
//!
//! The importer decodes the dat's packing (`class * 256 + amount`), so what arrives here is already
//! "add 1 to attack class 4". This file checks the three things that can be wrong with that:
//! the amount (published Forting/Scale Mail numbers), the target (a class is not a unit), and the
//! owner (my upgrade is not yours).

use std::path::PathBuf;

use ruage_data::{Economy, Effects, Map, Sim as Settings, Techs, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn slot(name: &str) -> usize {
    ruage_data::load_resources_file(&repo("config/resources.toml"))
        .expect("resources.toml")
        .slot_of(name)
        .unwrap_or_else(|| panic!("the pack has no {name}"))
}

/// Forging: +1 melee attack for infantry. Scale Mail: +1 melee and +1 pierce armour for infantry.
/// Both are blacksmith (103) techs, and both are published numbers.
const FORGING: u32 = 67;
const SCALE_MAIL: u32 = 74;
const SMITH: u32 = 103;
const SPEARMAN: u32 = 74;

/// The stateful ones: Loom (+15 hp to villagers, at the town centre), Masonry (buildings x1.1 hp, at a
/// castle), Wheelbarrow (villager speed x1.1 and carry x1.25, at the town centre).
const LOOM: u32 = 22;
/// Free, instant, and adds +0.35 tiles/s to unit 448 (a scout): the one shipped case of a speed
/// *addition*, which is a different scale from a multiplier.
const SCOUT_SPEED: u32 = 20;
const SCOUT: u32 = 448;
const MASONRY: u32 = 50;
const WHEELBARROW: u32 = 213;
const TOWN_CENTRE: u32 = 109;
const CASTLE: u32 = 209;
const VILLAGER: u32 = 83;
const HOUSE: u32 = 70;

struct Fixture {
    settings: Settings,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
    combat: ruage_data::Combat,
    techs: Techs,
    effects: Effects,
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
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 11).expect("world");
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
    world
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

/// Gather until the bank holds `want` in a column, re-tasking as nodes run dry.
fn bank(world: &mut World, player: u8, villager: EntityId, slot: usize, want: u32) {
    let mut node = nearest_node(world, villager, slot).expect("a node of that column");
    world.submit(Command::Gather { villager, node });
    for _ in 0..400_000 {
        world.step().expect("step");
        if world.stock(player, slot) >= want {
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
        world.stock(player, slot)
    );
}

fn first_villager(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Unit)
        .expect("a villager")
        .id
}

fn place(world: &mut World, player: u8, id: u32, unit: u32, tile: (i64, i64), hp: i32) -> EntityId {
    let scale = world.scale();
    let id = EntityId(id);
    let kind = match unit {
        SMITH | TOWN_CENTRE | CASTLE | 12 | HOUSE => EntityKind::Building,
        _ => EntityKind::Unit,
    };
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
    world.step().expect("step");
    id
}

/// Research a tech for player 1 and run it to completion.
fn research(world: &mut World, player: u8, smith: EntityId, tech: u32, seconds: u32) {
    world
        .submit_checked(Command::Research {
            building: smith,
            tech,
        })
        .expect("the research order is legal");
    for _ in 0..u64::from(seconds) * u64::from(world.tick_rate_hz()) + 8 {
        world.step().expect("step");
        if world.researched(player).contains(&tech) {
            return;
        }
    }
    panic!("tech {tech} never landed");
}

/// One blow from `attacker` against `victim`: how much health the victim loses.
fn blow(world: &mut World, attacker: EntityId, victim: EntityId) -> i32 {
    let before = world.entity(victim).expect("victim").hp;
    world
        .submit_checked(Command::Attack {
            attacker,
            target: victim,
        })
        .expect("the attack order is legal");
    // The swing resolves when the attacker is adjacent; a spawn beside the victim is enough.
    for _ in 0..40 {
        world.step().expect("step");
        let now = world.entity(victim).expect("victim").hp;
        if now != before {
            return before - now;
        }
    }
    panic!("no blow landed");
}

#[test]
fn forging_adds_one_melee_attack_to_the_infantry_it_covers() {
    // Published: Forging is +1 melee attack. A spearman's melee attack is 4, and the target's melee
    // armour is 0, so the blow goes from 4 to 5 - a number the data and the wiki agree on.
    let Some(fx) = fixture() else { return };

    let mut plain = seeded(&fx);
    let attacker = place(&mut plain, 1, 9_100, SPEARMAN, (10, 10), 40);
    let victim = place(&mut plain, 2, 9_101, SPEARMAN, (11, 10), 40);
    assert_eq!(
        blow(&mut plain, attacker, victim),
        4,
        "4 melee attack, 0 armour"
    );

    let mut researched = seeded(&fx);
    let smith = place(&mut researched, 1, 9_102, SMITH, (14, 14), 1_000);
    let attacker = place(&mut researched, 1, 9_103, SPEARMAN, (10, 10), 40);
    let victim = place(&mut researched, 2, 9_104, SPEARMAN, (11, 10), 40);
    let villager = first_villager(&researched);
    bank(&mut researched, 1, villager, slot("food"), 150);
    research(&mut researched, 1, smith, FORGING, 50);

    assert_eq!(
        blow(&mut researched, attacker, victim),
        5,
        "Forging's +1 melee attack reaches the blow"
    );
}

#[test]
fn scale_mail_adds_one_melee_and_one_pierce_armour() {
    // Published: Scale Mail is +1/+1 armour. Against the same 4 melee attack, the victim now takes 3.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    // The armour is the victim's, so the *victim's* owner is the one who researches it.
    let smith = place(&mut world, 2, 9_110, SMITH, (30, 30), 1_000);
    let attacker = place(&mut world, 1, 9_111, SPEARMAN, (10, 10), 40);
    let victim = place(&mut world, 2, 9_112, SPEARMAN, (11, 10), 40);
    let villager = world
        .entities()
        .iter()
        .find(|e| e.player == 2 && e.kind == EntityKind::Unit)
        .expect("a villager for player 2")
        .id;
    bank(&mut world, 2, villager, slot("food"), 100);
    research(&mut world, 2, smith, SCALE_MAIL, 40);

    assert_eq!(
        blow(&mut world, attacker, victim),
        3,
        "the armour is the victim's, and it is now 3 not 4"
    );
}

#[test]
fn a_modifier_only_touches_the_unit_class_it_names() {
    // Forging names infantry classes (6, 45, 12, 46, 47, 50). A villager is class 4, so its attack
    // must not move - the class filter is the whole difference between "an upgrade" and "a cheat".
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let smith = place(&mut world, 1, 9_120, SMITH, (14, 14), 1_000);
    let attacker = place(&mut world, 1, 9_121, 83, (10, 10), 25);
    let victim = place(&mut world, 2, 9_122, 83, (11, 10), 25);
    let villager = first_villager(&world);
    bank(&mut world, 1, villager, slot("food"), 150);
    research(&mut world, 1, smith, FORGING, 50);

    assert_eq!(
        blow(&mut world, attacker, victim),
        3,
        "a villager's 3 melee attack, unmodified: class 4 is not on Forging's list"
    );
}

#[test]
fn a_modifier_is_the_researching_players_alone() {
    // Player 1 researches Forging; player 2's identical spearman must still hit player 1's for 4.
    // If the adjustment were keyed on the unit rather than its owner, both sides would get it.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let smith = place(&mut world, 1, 9_130, SMITH, (14, 14), 1_000);
    let mine = place(&mut world, 1, 9_131, SPEARMAN, (20, 20), 40);
    let theirs = place(&mut world, 2, 9_132, SPEARMAN, (21, 20), 40);
    let villager = first_villager(&world);
    bank(&mut world, 1, villager, slot("food"), 150);
    research(&mut world, 1, smith, FORGING, 50);

    assert_eq!(
        blow(&mut world, theirs, mine),
        4,
        "player 2 researched nothing, so its blow is still 4"
    );
}

/// A pack that names an attack modifier with **no class**. The importer always writes one for a
/// packed attribute, but the pack is content, so the engine has to be safe: the modifier is skipped,
/// and this test pins that a classless attack change cannot move a blow (it is inert either way
/// today - an attack class the target does not list contributes nothing - which is exactly why the
/// guard is written down rather than inferred).
#[test]
fn an_attack_modifier_with_no_class_is_ignored() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let effects = ruage_data::load_effects_str(
        r#"
schema = 1
kind = "effects"

[[effect]]
id = 211
name = "classless"
commands = []
modifiers = [{ op = "add", attribute = "attack", attribute_class = -1, unit = -1, unit_class = -1, value = 99.0 }]
"#,
        "<classless effects>",
    )
    .expect("the effects load");
    let techs = ruage_data::load_techs_str(
        r#"
schema = 1
kind = "techs"

[[tech]]
id = 222
effect_id = 211
research_time = 0
costs = []
required = []
location = 0
"#,
        "<classless techs>",
    )
    .expect("the techs load");
    world
        .load_research(&techs, &effects)
        .expect("the tables load");

    let smith = place(&mut world, 1, 9_140, SMITH, (14, 14), 1_000);
    let attacker = place(&mut world, 1, 9_141, SPEARMAN, (10, 10), 40);
    let victim = place(&mut world, 2, 9_142, SPEARMAN, (11, 10), 40);
    // The custom table's one tech is 222, so that is what this world can research.
    research(&mut world, 1, smith, 222, 0);

    assert_eq!(
        blow(&mut world, attacker, victim),
        4,
        "a classless attack modifier is content we cannot place, not +99 to everything"
    );
}

/// Loom is +15 hp for villagers (published), and the decision for this increment is that existing
/// units *gain* it: a villager at 25 becomes 40, and one at 10 becomes 25.
#[test]
fn loom_raises_every_villagers_health_immediately() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = place(&mut world, 1, 9_200, TOWN_CENTRE, (14, 14), 2400);
    // One villager at full health, one wounded to 10 of 25.
    let whole = place(&mut world, 1, 9_201, VILLAGER, (10, 10), 25);
    let hurt = place(&mut world, 1, 9_202, VILLAGER, (10, 12), 10);
    let villager = first_villager(&world);
    bank(&mut world, 1, villager, slot("gold"), 50);
    research(&mut world, 1, centre, LOOM, 25);

    assert_eq!(world.entity(whole).expect("villager").hp, 40, "25 + 15");
    assert_eq!(
        world.entity(hurt).expect("villager").hp,
        25,
        "the grant is health, not a reset: 10 + 15"
    );
    // And it is the villagers', not everything's: a spearman is class 6.
    let spearman = place(&mut world, 1, 9_203, SPEARMAN, (12, 10), 40);
    assert_eq!(
        world.entity(spearman).expect("spearman").hp,
        40,
        "Loom names class 4, so infantry keeps its 40"
    );
}

#[test]
fn masonry_raises_building_health_by_a_tenth() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let castle = place(&mut world, 1, 9_210, CASTLE, (14, 14), 4800);
    let house = place(&mut world, 1, 9_211, HOUSE, (10, 10), 550);
    let wounded = place(&mut world, 1, 9_212, HOUSE, (10, 12), 100);
    let villager = first_villager(&world);
    // 150 food and 175 wood, which is what the table charges for it.
    bank(&mut world, 1, villager, slot("food"), 150);
    bank(&mut world, 1, villager, slot("wood"), 175);
    research(&mut world, 1, castle, MASONRY, 50);

    assert_eq!(world.entity(house).expect("house").hp, 605, "550 x 1.1");
    assert_eq!(
        world.entity(wounded).expect("house").hp,
        155,
        "100 gains the 55 the maximum grew by, not a full heal"
    );
}

#[test]
fn wheelbarrow_changes_what_is_gathered_next_and_not_a_load_in_hand() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = place(&mut world, 1, 9_220, TOWN_CENTRE, (14, 14), 2400);
    let villager = first_villager(&world);

    // An order placed *before* the tech: its rate and cap are already resolved.
    let node = nearest_node(&world, villager, slot("wood")).expect("a tree");
    world
        .submit_checked(Command::Gather { villager, node })
        .expect("gather");
    world.step().expect("step");
    let before = match world.entity(villager).expect("villager").task {
        ruage_sim::Task::Gather {
            rate_per_tick,
            carry_cap,
            ..
        } => (rate_per_tick.raw(), carry_cap),
        other => panic!("expected a gather order, got {other:?}"),
    };

    bank(&mut world, 1, villager, slot("food"), 175);
    bank(&mut world, 1, villager, slot("wood"), 50);
    research(&mut world, 1, centre, WHEELBARROW, 75);

    // Speed is entity state, so it moves at once: 0.8 x 1.1 = 0.88.
    assert_eq!(
        world.entity(villager).expect("villager").speed.raw(),
        17_600,
        "0.88 at scale 20000"
    );

    // The load in hand keeps the numbers it started with...
    let after = match world.entity(villager).expect("villager").task {
        ruage_sim::Task::Gather {
            rate_per_tick,
            carry_cap,
            ..
        } => (rate_per_tick.raw(), carry_cap),
        other => panic!("expected the same gather order, got {other:?}"),
    };
    assert_eq!(after, before, "a load in hand is not re-priced");

    // ...and the next order is: carry 10 x 1.25 = 12.5, rounded half away from zero.
    world
        .submit_checked(Command::Stop { unit: villager })
        .expect("stop");
    world.step().expect("step");
    world
        .submit_checked(Command::Gather { villager, node })
        .expect("gather again");
    world.step().expect("step");
    match world.entity(villager).expect("villager").task {
        ruage_sim::Task::Gather { carry_cap, .. } => {
            assert_eq!(carry_cap, 13, "10 x 1.25 = 12.5, rounded up");
        }
        other => panic!("expected a new gather order, got {other:?}"),
    }
}

/// Nine speed modifiers in the data are additions in tiles per second (this one: +0.35 to unit 448, a
/// scout at 1.2). The engine stores speed as a fixed-point number at `scale`, so an addition has to be
/// scaled up before it is added - adding the raw value would move the speed by 0.35/20000 of a tile
/// per second, which is indistinguishable from not applying it at all.
fn place_moving(
    world: &mut World,
    player: u8,
    id: u32,
    unit: u32,
    tile: (i64, i64),
    hp: i32,
    speed: Fixed,
) -> EntityId {
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
            speed,
            hp,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

#[test]
fn a_speed_addition_arrives_in_tiles_per_second_and_not_in_scale_units() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let centre = place(&mut world, 1, 9_230, TOWN_CENTRE, (14, 14), 2400);
    // 1.2 tiles/s, which is what the table says for this unit.
    let scout = place_moving(
        &mut world,
        1,
        9_231,
        SCOUT,
        (10, 10),
        45,
        Fixed::from_raw(24_000),
    );
    // 1.2 tiles/s at scale 20000.
    assert_eq!(world.entity(scout).expect("scout").speed.raw(), 24_000);
    research(&mut world, 1, centre, SCOUT_SPEED, 0);
    assert_eq!(
        world.entity(scout).expect("scout").speed.raw(),
        31_000,
        "1.2 + 0.35 = 1.55 tiles/s, which is 31000 at scale 20000 - not 24000.35"
    );
}
