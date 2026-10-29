//! The step-5 scenario: one scripted match, run to the end, reported tick by tick.
//!
//! Written before the runner (D7).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_sim::{
    Command, EntityKind, Outcome, Runner, Task, World, load_scenario_file, load_scenario_str,
};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// The project pack's column for a resource, so a test says "wood" the way content does and the
/// pack is what turns that into a stockpile column (D17).
fn slot(name: &str) -> usize {
    slot_in(
        &ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml"),
        name,
    )
}

fn slot_in(pack: &ruage_data::Resources, name: &str) -> usize {
    pack.slot_of(name)
        .unwrap_or_else(|| panic!("the pack has no {name}"))
}

struct Fixture {
    settings: Settings,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
    combat: ruage_data::Combat,
    techs: ruage_data::Techs,
    effects: ruage_data::Effects,
    civs: ruage_data::Civs,
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
        techs: ruage_data::load_techs_file(&repo("config/gamedata/techs.toml"))
            .expect("techs.toml"),
        effects: ruage_data::load_effects_file(&repo("config/gamedata/effects.toml"))
            .expect("effects.toml"),
        civs: ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs.toml"),
    })
}

fn world(fx: &Fixture, seed: u64) -> World {
    let mut world = World::new(&fx.settings, seed).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("the research tables load");
    // The pack is played as a civilisation, and that is what bars the techs it cannot have.
    world
        .load_civs(&fx.civs)
        .expect("the civ table holds the pack's civilisation");
    world
}

/// Run a scenario and collect what the world looked like at the end, plus the step outcomes.
fn play(fx: &Fixture, text: &str, seed: u64) -> (World, Runner) {
    let scenario = load_scenario_str(text, "<scenario>").expect("the scenario parses");
    let mut world = world(fx, seed);
    let mut runner = Runner::new(&scenario, world.tick_rate_hz()).expect("a runnable scenario");
    let ticks =
        u64::try_from((scenario.duration_seconds * f64::from(world.tick_rate_hz())).round() as i64)
            .expect("a length in ticks");
    for _ in 0..ticks {
        runner.due(&mut world);
        world.step().expect("step");
    }
    (world, runner)
}

#[test]
fn the_project_scenario_parses_and_is_runnable() {
    let Some(fx) = fixture() else { return };
    let scenario = load_scenario_file(&repo("config/scenario.toml")).expect("config/scenario.toml");
    assert!(
        (scenario.duration_seconds - 600.0).abs() < f64::EPSILON,
        "the step-5 scenario is a ten-minute match, got {} s",
        scenario.duration_seconds
    );
    let mut world = world(&fx, 3);
    let runner = Runner::new(&scenario, world.tick_rate_hz()).expect("runnable");
    assert!(runner.pending(), "there are steps to run");
    let _ = world.step();
}

#[test]
fn a_step_that_cannot_be_resolved_is_reported_not_dropped() {
    let Some(fx) = fixture() else { return };
    // No such villager: player 1 starts with three.
    let text = r#"
schema = 2
duration_seconds = 1.0

[[step]]
at_seconds = 0.1
player = 1
kind = "gather"
villager = 9
resource = "wood"
"#;
    let (_, runner) = play(&fx, text, 3);
    assert_eq!(runner.outcomes.len(), 1, "the step was attempted");
    match &runner.outcomes[0].1 {
        Outcome::Refused(reason) => assert!(reason.contains("no villager 9"), "got: {reason}"),
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[test]
fn a_scripted_order_moves_the_world() {
    let Some(fx) = fixture() else { return };
    let text = r#"
schema = 2
duration_seconds = 60.0

[[step]]
at_seconds = 0.1
player = 1
kind = "gather"
villager = 0
resource = "wood"
"#;
    let (world, runner) = play(&fx, text, 3);
    assert_eq!(runner.outcomes.len(), 1);
    assert_eq!(runner.outcomes[0].1, Outcome::Ran);
    assert!(
        world.stock(1, slot("wood")) > 0,
        "a minute of gathering should reach the bank"
    );
}

#[test]
fn the_ten_minute_scenario_builds_trains_and_fights() {
    let Some(fx) = fixture() else { return };
    let text = std::fs::read_to_string(repo("config/scenario.toml")).expect("config/scenario.toml");
    let (world, runner) = play(&fx, &text, 7);

    for (seconds, outcome) in &runner.outcomes {
        assert_eq!(
            *outcome,
            Outcome::Ran,
            "the step due at {seconds} s did not run: {outcome:?}"
        );
    }

    // A house and a barracks stand for player 1, finished, not still going up.
    for unit in [70u32, 12] {
        let built = world.entities().iter().any(|entity| {
            entity.player == 1
                && entity.kind == EntityKind::Building
                && entity.unit == Some(unit)
                && entity.construction.is_none()
        });
        assert!(
            built,
            "unit {unit} should be standing and finished by ten minutes"
        );
    }

    // The spearman exists, and something died: the fight happened.
    let spearmen = world
        .entities()
        .iter()
        .filter(|entity| {
            entity.player == 1 && entity.kind == EntityKind::Unit && entity.unit == Some(74)
        })
        .count();
    assert_eq!(spearmen, 1, "one spearman was trained");
    let villagers_left = world
        .entities()
        .iter()
        .filter(|entity| entity.player == 2 && entity.kind == EntityKind::Unit)
        .count();
    assert!(
        villagers_left < 3,
        "the spearman should have killed an opponent villager, {villagers_left} of 3 remain"
    );
}

#[test]
fn the_scenario_is_deterministic() {
    let Some(fx) = fixture() else { return };
    let text = std::fs::read_to_string(repo("config/scenario.toml")).expect("config/scenario.toml");
    let (first, _) = play(&fx, &text, 7);
    let (second, _) = play(&fx, &text, 7);
    assert_eq!(
        first.state_hash(),
        second.state_hash(),
        "two runs of the same script must end in the same state"
    );
    // The seed does not change this match: nothing draws from the RNG streams, and the map is
    // authored. So the second check is on the *script*: drop the last step and the state must differ,
    // or the hash is not measuring the match at all.
    let shortened = text
        .rsplit_once("[[step]]")
        .map(|(head, _)| head.to_owned())
        .expect("the scenario has steps");
    let (other, _) = play(&fx, &shortened, 7);
    assert_ne!(
        first.state_hash(),
        other.state_hash(),
        "a script with one step fewer must end in a different state"
    );
}

#[test]
fn stopping_a_unit_is_always_accepted() {
    let Some(fx) = fixture() else { return };
    let mut world = world(&fx, 3);
    let villager = world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Unit)
        .expect("a villager")
        .id;
    world
        .submit_checked(Command::Stop { unit: villager })
        .expect("stopping needs no preconditions");
}

/// A step that names a resource the pack does not define must be *reported*, not skipped. A replay
/// is the record of a match: a silently ignored step is a match that diverges from its own script
/// with nothing to say why, and a misspelt resource name is exactly how that would start (D17).
#[test]
fn a_scenario_naming_a_resource_the_pack_lacks_is_refused() {
    let Some(fx) = fixture() else { return };
    let scenario = r#"
schema = 2
duration_seconds = 2

[[step]]
at_seconds = 0.0
player = 1
kind = "gather"
villager = 0
resource = "silver"
"#;
    let (_, runner) = play(&fx, scenario, 3);
    let refusal = runner
        .outcomes
        .iter()
        .map(|(_, outcome)| outcome.clone())
        .find_map(|outcome| match outcome {
            Outcome::Refused(reason) => Some(reason),
            _ => None,
        })
        .expect("the step must be refused, not silently dropped");
    assert!(
        refusal.contains("silver"),
        "the refusal must name the resource the pack lacks: {refusal}"
    );
}

/// The scenario format names roles, not dat ids (D17). A role the economy does not define must be
/// refused with the name in the message, because the alternative - a step that silently targets
/// unit 0 - is a match that diverges from its own script with nothing to explain it.
#[test]
fn a_scenario_naming_a_role_the_economy_lacks_is_refused() {
    let Some(fx) = fixture() else { return };
    let scenario = r#"
schema = 2
duration_seconds = 3

[[step]]
at_seconds = 0.0
player = 1
kind = "build"
villager = 0
role = "wonder"
"#;
    let (_, runner) = play(&fx, scenario, 3);
    let refusal = runner
        .outcomes
        .iter()
        .find_map(|(_, outcome)| match outcome {
            Outcome::Refused(reason) => Some(reason.clone()),
            _ => None,
        })
        .expect("the step must be refused, not silently dropped");
    assert!(
        refusal.contains("wonder"),
        "the refusal must name the role: {refusal}"
    );
}

/// The engine must not know the *word* "town_centre": it is a role name from `economy.toml`, and a
/// pack of our own may call its keep anything it likes. This builds a world whose only gathering
/// point is called `keep` and asks for a house beside it, so the engine's own free-tile search has
/// to resolve the role rather than a literal (D17).
#[test]
fn a_world_whose_town_centre_role_is_renamed_still_finds_a_build_site() {
    let Some(fx) = fixture() else { return };
    let mut economy = fx.economy.clone();
    let tc = economy
        .role
        .remove("town_centre")
        .expect("the project pack has one");
    economy.role.insert("keep".to_owned(), tc);
    economy.start.town_centre = "keep".to_owned();

    let mut world = World::new(&fx.settings, 5).expect("world");
    world
        .seed_from(&fx.map, &economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);

    // Three villagers gather, then one of them puts up a house beside the renamed centre: the site
    // is the thing under test, and wood is only what makes the order legal.
    let scenario = load_scenario_str(
        r#"
schema = 2
duration_seconds = 150

[[step]]
at_seconds = 0.0
player = 1
kind = "gather"
villager = 0
resource = "wood"

[[step]]
at_seconds = 0.0
player = 1
kind = "gather"
villager = 1
resource = "wood"
nth = 1

[[step]]
at_seconds = 0.0
player = 1
kind = "gather"
villager = 2
resource = "wood"
nth = 2

[[step]]
at_seconds = 120.0
player = 1
kind = "build"
villager = 0
role = "house"
"#,
        "<scenario>",
    )
    .expect("the scenario parses");

    let mut runner = Runner::new(&scenario, world.tick_rate_hz()).expect("runnable");
    for _ in 0..u64::from(world.tick_rate_hz()) * 150 {
        runner.due(&mut world);
        world.step().expect("step");
    }
    let refused: Vec<String> = runner
        .outcomes
        .iter()
        .filter_map(|(_, outcome)| match outcome {
            Outcome::Refused(reason) => Some(reason.clone()),
            _ => None,
        })
        .collect();
    assert!(
        refused.is_empty(),
        "a build step must not need the literal \"town_centre\": {refused:?}"
    );
}

/// A scenario could not name a tech at all until `[tech_role]` existed (techs have no name in the dat),
/// so research had no end-to-end demonstration outside a unit test. This is that demonstration: Loom,
/// researched at the town centre, reaches every villager in a scripted match.
#[test]
fn a_scenario_researches_a_tech_named_by_a_role() {
    let Some(fx) = fixture() else { return };
    // Three villagers on gold first: the pack starts a match with an empty bank, and Loom costs 50,
    // so the scenario has to earn it - which is also the honest check that research and gathering
    // live in the same economy.
    let text = r#"
schema = 2
duration_seconds = 220

[[step]]
at_seconds = 0.0
player = 1
kind = "gather"
villager = 0
resource = "gold"

[[step]]
at_seconds = 0.5
player = 1
kind = "gather"
villager = 1
resource = "gold"
nth = 1

[[step]]
at_seconds = 1.0
player = 1
kind = "gather"
villager = 2
resource = "gold"
nth = 2

[[step]]
at_seconds = 120.0
player = 1
kind = "research"
building = "town_centre"
tech = "loom"
"#;
    let (world, runner) = play(&fx, text, 12_345);
    assert!(
        runner
            .outcomes
            .iter()
            .any(|(_, outcome)| *outcome == Outcome::Ran),
        "the research order is accepted: {:?}",
        runner.outcomes
    );
    assert!(
        runner
            .outcomes
            .iter()
            .all(|(_, outcome)| *outcome == Outcome::Ran),
        "every step ran, including the three gathers: {:?}",
        runner.outcomes
    );
    let villagers: Vec<i32> = world
        .entities()
        .iter()
        .filter(|entity| entity.player == 1 && entity.unit == Some(83))
        .map(|entity| entity.hp)
        .collect();
    assert!(
        !villagers.is_empty(),
        "player 1 has villagers to begin with"
    );
    assert!(
        villagers.iter().all(|hp| *hp == 40),
        "Loom is +15 for a 25-health villager, so all of them: {villagers:?}"
    );
}

#[test]
fn a_scenario_naming_a_tech_the_economy_lacks_is_refused() {
    let Some(fx) = fixture() else { return };
    let text = r#"
schema = 2
duration_seconds = 10

[[step]]
at_seconds = 0.0
player = 1
kind = "research"
building = "town_centre"
tech = "bronze_working"
"#;
    let (_, runner) = play(&fx, text, 12_345);
    match runner.outcomes.first().map(|(_, outcome)| outcome.clone()) {
        Some(Outcome::Refused(reason)) => assert!(
            reason.contains("bronze_working"),
            "the refusal names what it could not find: {reason}"
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Purpose-built research content: a free, instant-to-research tech that doubles the health of
/// buildings. The pack's own health techs are 25 s (Loom, villagers) and 50 s (Masonry, buildings) and
/// cost resources, so a test that needs a tech to land *during* a 25-second build would be racing them.
const DOUBLING_TECHS: &str = r#"
schema = 1
kind = "techs"

# The pack's `[start] techs` names the Dark Age, so the tables must carry it even here.
[[tech]]
id = 104
effect_id = -1
research_time = 0
costs = []
required = []
location = 0

[[tech]]
id = 900
effect_id = 900
research_time = 1
costs = []
required = []
location = 0
"#;

const DOUBLING_EFFECTS: &str = r#"
schema = 1
kind = "effects"

[[effect]]
id = 900
name = "Test: buildings double"
commands = []
modifiers = [{ op = "multiply", attribute = "hit_points", attribute_class = -1, unit = -1, unit_class = 3, value = 2.0 }]
"#;

/// A world that uses the real pack but purpose-built research, plus a `doubling` tech role so a step
/// can name the tech the way content would.
fn doubling_world(fx: &Fixture, seed: u64) -> World {
    let techs = ruage_data::load_techs_str(DOUBLING_TECHS, "<doubling techs>").expect("techs");
    let effects =
        ruage_data::load_effects_str(DOUBLING_EFFECTS, "<doubling effects>").expect("effects");
    let text = std::fs::read_to_string(repo("config/economy.toml")).expect("economy.toml");
    let text = text.replace("[tech_role]", "[tech_role]\ndoubling = 900");
    let economy = ruage_data::load_economy_str(&text, "<economy with doubling>").expect("economy");
    let mut world = World::new(&fx.settings, seed).expect("world");
    world
        .seed_from(&fx.map, &economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    world
        .load_research(&techs, &effects)
        .expect("the research tables load");
    world
}

fn play_with(world: World, text: &str) -> (World, Runner) {
    let scenario = load_scenario_str(text, "<scenario>").expect("the scenario parses");
    let mut world = world;
    let mut runner = Runner::new(&scenario, world.tick_rate_hz()).expect("a runnable scenario");
    let ticks =
        u64::try_from((scenario.duration_seconds * f64::from(world.tick_rate_hz())).round() as i64)
            .expect("a length in ticks");
    for _ in 0..ticks {
        runner.due(&mut world);
        world.step().expect("step");
    }
    (world, runner)
}

fn houses_of(world: &World, player: u8) -> Vec<(i32, bool)> {
    world
        .entities()
        .iter()
        .filter(|entity| entity.player == player && entity.unit == Some(70))
        .map(|entity| (entity.hp, entity.construction.is_some()))
        .collect()
}

/// Three villagers on wood, then a house. The house costs 25 wood and takes 25 s.
const THREE_ON_WOOD: &str = r#"
[[step]]
at_seconds = 0.0
player = 1
kind = "gather"
villager = 0
resource = "wood"

[[step]]
at_seconds = 0.5
player = 1
kind = "gather"
villager = 1
resource = "wood"
nth = 1

[[step]]
at_seconds = 1.0
player = 1
kind = "gather"
villager = 2
resource = "wood"
nth = 2
"#;

/// A site ordered *after* a health tech must be quoted the new maximum: `check_build` copied the
/// table's value, so a house put up after the tech finished at the old one.
#[test]
fn a_house_ordered_after_a_health_tech_is_quoted_the_new_maximum() {
    let Some(fx) = fixture() else { return };
    let text = format!(
        r#"
schema = 2
duration_seconds = 140
{THREE_ON_WOOD}

[[step]]
at_seconds = 5.0
player = 1
kind = "research"
building = "town_centre"
tech = "doubling"

[[step]]
at_seconds = 60.0
player = 1
kind = "build"
villager = 0
role = "house"
"#
    );
    let (world, runner) = play_with(doubling_world(&fx, 7), &text);
    assert!(
        runner
            .outcomes
            .iter()
            .all(|(_, outcome)| *outcome == Outcome::Ran),
        "every step ran: {:?}",
        runner.outcomes
    );
    let houses = houses_of(&world, 1);
    assert_eq!(houses.len(), 1, "one house: {houses:?}");
    assert_eq!(houses[0], (1100, false), "550 x 2, finished");
}

/// The same defect one layer down: a building already going up when the tech lands used to finish at
/// the maximum it was quoted, because the site stores that number and nothing refreshed it.
#[test]
fn a_house_finished_after_a_health_tech_reaches_the_new_maximum() {
    let Some(fx) = fixture() else { return };
    let text = format!(
        r#"
schema = 2
duration_seconds = 160
{THREE_ON_WOOD}

[[step]]
at_seconds = 60.0
player = 1
kind = "build"
villager = 0
role = "house"

# 1 s of research, landing while the 25 s build is still going.
[[step]]
at_seconds = 70.0
player = 1
kind = "research"
building = "town_centre"
tech = "doubling"
"#
    );
    let (world, runner) = play_with(doubling_world(&fx, 7), &text);
    assert!(
        runner
            .outcomes
            .iter()
            .all(|(_, outcome)| *outcome == Outcome::Ran),
        "every step ran: {:?}",
        runner.outcomes
    );
    let houses = houses_of(&world, 1);
    assert_eq!(houses.len(), 1, "one house: {houses:?}");
    assert_eq!(houses[0], (1100, false), "550 x 2, finished");
}

/// `where` decides *where*: a building placed for a resource goes to that resource. The field used to be parsed and
/// ignored - `deny_unknown_fields` is not set on a step, so `where = "wood"` was silently dropped and a lumber camp**
/// landed beside the town centre, which is wood spent on a longer walk.
///
/// **Gold, not wood, and that matters:** player 1's town centre is at (16, 16) and the nearest tree is at (20, 16), so
/// a camp placed by the *town centre* is within three tiles of a tree anyway - the first version of this test passed
/// with `where` disabled entirely. The nearest gold is eight tiles out, outside the placement ring, so where the camp
/// lands can only be explained by the selector.
#[test]
fn a_building_placed_where_the_resource_is_lands_by_the_resource() {
    let Some(fx) = fixture() else {
        return;
    };
    let text = r#"
schema = 2
duration_seconds = 40.0

[[step]]
at_seconds = 1.0
player = 1
kind = "build"
villager = 0
role = "mining_camp"
where = "gold"
"#;
    let (world, runner) = play(&fx, text, 7);
    assert!(
        runner
            .outcomes
            .iter()
            .all(|(_, outcome)| *outcome == Outcome::Ran),
        "the step has to run: {:?}",
        runner.outcomes
    );
    let camp = world
        .entities()
        .iter()
        .filter(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .filter(|entity| {
            entity.unit.is_some_and(|unit| world.is_family(unit, 584)) // mining camp (MINE)
        })
        .map(|entity| entity.position)
        .next()
        .expect("a mining camp was founded");
    let scale = world.scale();
    let camp_tile = (camp.0.raw() / scale, camp.1.raw() / scale);
    let gold = world.slot_of("gold").expect("gold");
    let nearest_gold = world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(slot) if slot == gold))
        .map(|entity| {
            let tile = (
                entity.position.0.raw() / scale,
                entity.position.1.raw() / scale,
            );
            (tile.0 - camp_tile.0)
                .abs()
                .max((tile.1 - camp_tile.1).abs())
        })
        .min()
        .expect("gold");
    assert!(
        nearest_gold <= 3,
        "a mining camp belongs within three tiles of gold, not {nearest_gold} tiles away"
    );
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .map(|entity| entity.position)
        .expect("a town centre");
    let centre_tile = (centre.0.raw() / scale, centre.1.raw() / scale);
    let from_centre = (camp_tile.0 - centre_tile.0)
        .abs()
        .max((camp_tile.1 - centre_tile.1).abs());
    assert!(
        from_centre > 4,
        "the camp is {from_centre} tiles from the town centre, which is where the ignored selector put it"
    );
}

/// A `where` naming something the pack has no column for is refused with the name, not quietly placed elsewhere.
#[test]
fn a_where_selector_naming_nothing_is_refused() {
    let Some(fx) = fixture() else {
        return;
    };
    let text = r#"
schema = 2
duration_seconds = 10.0

[[step]]
at_seconds = 1.0
player = 1
kind = "build"
villager = 0
role = "house"
where = "fish"
"#;
    let (_, runner) = play(&fx, text, 7);
    let refused = runner
        .outcomes
        .iter()
        .find_map(|(_, outcome)| match outcome {
            Outcome::Refused(why) => Some(why.clone()),
            _ => None,
        })
        .expect("a step naming a resource the pack lacks has to be refused");
    assert!(
        refused.contains("fish"),
        "the refusal names the word that was wrong: {refused}"
    );
}

/// A target selector that names a *building*: the acceptance match's last step is a siege engine aimed at a town
/// centre, which `nearest_enemy` (a unit) could never express.
#[test]
fn an_attack_step_can_name_the_enemy_town_centre() {
    let Some(fx) = fixture() else {
        return;
    };
    let text = r#"
schema = 2
duration_seconds = 20.0

[[step]]
at_seconds = 2.0
player = 1
kind = "attack"
attacker = "villager"
target = "enemy_town_centre"
"#;
    let (world, runner) = play(&fx, text, 7);
    assert!(
        runner
            .outcomes
            .iter()
            .all(|(_, outcome)| *outcome == Outcome::Ran),
        "the step has to run: {:?}",
        runner.outcomes
    );
    let centre_unit = world.start_town_centre_unit().expect("a town centre role");
    let enemy_centre = world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == 2
                && entity.kind == EntityKind::Building
                && entity
                    .unit
                    .is_some_and(|unit| world.is_family(unit, centre_unit))
        })
        .map(|entity| entity.id)
        .expect("player 2's town centre");
    let targeted = world.entities().iter().any(|entity| {
        entity.player == 1
            && matches!(entity.task, Task::Attack { target, .. } if target == enemy_centre)
    });
    assert!(
        targeted,
        "the villager's task should name the enemy town centre"
    );
}

/// A farm is a **building**, so `nearest_resource` could never find one and a script could not put a villager on the
/// food engine that a long opening needs. `node = "farm"` names a building by role instead.
#[test]
fn a_gather_step_can_put_a_villager_on_a_farm() {
    let Some(fx) = fixture() else {
        return;
    };
    let text = r#"
schema = 2
duration_seconds = 300.0

[[step]]
at_seconds = 1.0
player = 1
kind = "build"
villager = 0
role = "farm"

[[step]]
at_seconds = 40.0
player = 1
kind = "gather"
villager = 1
resource = "food"
node = "farm"
"#;
    let (world, runner) = play(&fx, text, 7);
    let last = runner.outcomes.last().map(|(_, outcome)| outcome.clone());
    assert_eq!(
        last,
        Some(Outcome::Ran),
        "sending a villager to a farm has to work: {:?}",
        runner.outcomes
    );
    let farm = world
        .entities()
        .iter()
        .filter(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .find(|entity| entity.unit.is_some_and(|unit| world.is_family(unit, 50)))
        .map(|entity| entity.id)
        .expect("a farm was built");
    let gathering = world
        .entities()
        .iter()
        .any(|entity| matches!(entity.task, Task::Gather { node, .. } if node == farm));
    assert!(gathering, "the villager's gather task names the farm");
}

/// The acceptance script parses and its opening runs without a refusal: a thirty-minute match whose first minute is
/// already refused would report a build order that never existed. Its *timings* are not asserted here - they are a
/// measurement, printed by `ruage-batch`, and a number in a test would go stale the first time the economy changed.
#[test]
fn the_acceptance_scenario_parses_and_its_opening_runs() {
    let Some(fx) = fixture() else {
        return;
    };
    let scenario =
        load_scenario_file(&repo("config/acceptance.toml")).expect("config/acceptance.toml parses");
    let mut world = world(&fx, 7);
    let mut runner = Runner::new(&scenario, world.tick_rate_hz()).expect("runnable");
    let rate = u64::from(world.tick_rate_hz());
    for _ in 0..(60 * rate) {
        runner.due(&mut world);
        world.step().expect("step");
    }
    let refused: Vec<String> = runner
        .outcomes
        .iter()
        .filter_map(|(at, outcome)| match outcome {
            Outcome::Refused(why) => Some(format!("{at:.0}s: {why}")),
            _ => None,
        })
        .collect();
    assert!(
        refused.is_empty(),
        "the first minute of the acceptance match is refused: {refused:?}"
    );
}
