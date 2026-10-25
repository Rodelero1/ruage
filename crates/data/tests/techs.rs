//! Techs and effects: what the data says about effect commands, tested rather than assumed.

use std::path::PathBuf;

use ruage_data::{Effects, MaybeFixed, Meaning, Techs, load_effects_str, load_techs_str};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn effects() -> Effects {
    let path = repo("config/gamedata/effects.toml");
    ruage_data::load_effects_file(&path).expect("config/gamedata/effects.toml")
}

fn techs() -> Techs {
    let path = repo("config/gamedata/techs.toml");
    ruage_data::load_techs_file(&path).expect("config/gamedata/techs.toml")
}

fn units() -> ruage_data::Units {
    let path = repo("config/gamedata/units.toml");
    ruage_data::load_units_file(&path).expect("config/gamedata/units.toml")
}

#[test]
fn the_project_files_load() {
    let effects = effects();
    let techs = techs();
    assert_eq!(techs.techs.len(), 801, "801 techs in the dat");
    assert_eq!(effects.effects.len(), 798, "798 effects in the dat");
    assert!(
        effects.total_commands() > 3_000,
        "effects carry commands; got {}",
        effects.total_commands()
    );
    // Tech 2 is the first with a real price: 1000 food and 500 gold.
    let tech = techs.get(2).expect("tech 2");
    assert_eq!(tech.costs, vec![(0, 1000), (3, 500)]);
    assert_eq!(tech.research_time, 45);
    assert_eq!(tech.effect_id, Some(454));
}

#[test]
fn every_tech_points_at_an_effect_that_exists() {
    let effects = effects();
    let techs = techs();
    let dangling: Vec<u32> = techs
        .techs
        .values()
        .filter(|tech| {
            // -1 means "applies nothing", which the loader turns into `None`; it is not dangling.
            tech.effect_id.is_some_and(|id| effects.get(id).is_none())
        })
        .map(|tech| tech.id)
        .collect();
    assert!(
        dangling.is_empty(),
        "{} techs point at an effect that does not exist, first {}",
        dangling.len(),
        dangling.first().copied().unwrap_or(0)
    );
}

/// Kind 102's fifth argument is an id. If that id is a *tech* in every one of its occurrences, then
/// 102 takes techs away — and that is a fact about the data, not a convention we imported.
#[test]
fn kind_102_names_techs() {
    let effects = effects();
    let techs = techs();
    let mut checked = 0;
    let mut missing = 0;
    for effect in effects.effects.values() {
        for command in &effect.commands {
            if command.kind != 102 {
                continue;
            }
            checked += 1;
            match Effects::meaning(command, &techs) {
                Meaning::DisableTech { .. } => {}
                Meaning::Unpinned { .. } => missing += 1,
                other => panic!("kind 102 read as {other:?}"),
            }
        }
    }
    assert!(checked > 900, "kind 102 is common: {checked} commands");
    // One command names nothing at all: `[102, -1, -1, -1, -1]` in the Koreans tech tree. It is a
    // no-op entry in the data, not a counterexample, so the test names it rather than tolerating a
    // mystery.
    assert_eq!(
        missing, 1,
        "exactly one kind-102 command names no tech: the -1 no-op in the Koreans tree"
    );
    // The -1 value is the same sentinel that means "unresolved" everywhere else, so it cannot be
    // resolved to an id: that is precisely why the command reads as a no-op rather than as tech -1.
    let noop = effects
        .effects
        .values()
        .flat_map(|effect| effect.commands.iter().map(move |c| (effect, *c)))
        .find(|(_, command)| {
            command.kind == 102
                && matches!(Effects::meaning(command, &techs), Meaning::Unpinned { .. })
        })
        .expect("the no-op command exists");
    assert_eq!(
        noop.0.name, "Koreans Technology Tree",
        "the one kind-102 command that names no tech is the no-op in the Koreans tree"
    );
}

/// Kind 101 was expected to be the unit counterpart of 102, and it is not. Its `a` field names a
/// **unit** 84 times and a **tech** 13 times, so the namespace depends on a field we have not
/// pinned. This test records the split instead of pretending to a rule: a `UnitTouch` reading would
/// be right 87% of the time, and 87% is how a "free lumberjack upgrade" quietly becomes a free
/// villager.
#[test]
fn kind_101_is_not_a_clean_unit_command() {
    let units = units();
    let techs = techs();
    let effects = effects();
    let (mut as_unit, mut as_tech, mut ambiguous) = (0, 0, 0);
    let mut examples = Vec::new();
    for effect in effects.effects.values() {
        for command in &effect.commands {
            if command.kind != 101 {
                continue;
            }
            let id = u32::try_from(command.a).unwrap_or(0);
            match (units.unit(id).is_some(), techs.get(id).is_some()) {
                (true, true) => ambiguous += 1,
                (true, false) => as_unit += 1,
                (false, true) => {
                    as_tech += 1;
                    if examples.len() < 3 {
                        examples.push(format!("{} -> tech {id}", effect.name));
                    }
                }
                (false, false) => {
                    panic!(
                        "effect {:?} names id {id}, which is neither a unit nor a tech",
                        effect.name
                    )
                }
            }
        }
    }
    // The finding, asserted: no id is unit-only, 84 are in both tables, 13 are tech-only. Unit and
    // tech ids share a range, so an id alone cannot say which table a command means.
    assert_eq!(as_unit, 0, "no kind-101 id is unit-only");
    assert!(ambiguous > 50, "{ambiguous} ids are in both tables");
    assert!(as_tech > 5, "{as_tech} ids are tech-only");
    println!("kind 101: {as_unit} unit ids, {as_tech} tech ids, {ambiguous} in both tables");
    for example in examples {
        println!("  {example}");
    }
}

#[test]
fn the_command_kinds_are_counted_not_guessed() {
    let effects = effects();
    let census = effects.census();
    // The census is the interpreter's to-do list. These are the kinds the dat actually uses, in
    // ascending order; a new one appearing here means the interpreter has to learn it.
    let kinds: Vec<i16> = census.keys().copied().collect();
    assert!(
        kinds.contains(&4) && kinds.contains(&5) && kinds.contains(&102),
        "the expected kinds are present, got {kinds:?}"
    );
    assert!(
        census.values().sum::<usize>() == effects.total_commands(),
        "the census adds up to the command count"
    );
    // Printed so the numbers land in the test log rather than in someone's memory.
    for (kind, count) in &census {
        println!("kind {kind:>4}: {count:>5} commands");
    }
}

#[test]
fn a_non_integral_argument_is_refused() {
    let bad = r#"
schema = 1
kind = "effects"

[[effect]]
id = 1
name = "broken"
commands = [[1.5, 0, 0, 0, 1.0]]
"#;
    let err = load_effects_str(bad, "effects.toml")
        .expect_err("a fractional command kind is not a kind")
        .to_string();
    assert!(err.contains("1.5"), "got: {err}");
}

#[test]
fn an_effect_defined_twice_is_refused() {
    let bad = r#"
schema = 1
kind = "effects"

[[effect]]
id = 1
name = "a"
commands = []

[[effect]]
id = 1
name = "b"
commands = []
"#;
    let err = load_effects_str(bad, "effects.toml")
        .expect_err("two effects with one id: the later would silently win")
        .to_string();
    assert!(err.contains("effect[1]"), "got: {err}");
}

#[test]
fn a_negative_research_time_is_refused() {
    let bad = r#"
schema = 1
kind = "techs"

[[tech]]
id = 1
effect_id = 0
research_time = -1
costs = []
required = []
location = 0
"#;
    let err = load_techs_str(bad, "techs.toml")
        .expect_err("-1 is not a duration, and the loader must not invent a meaning for it")
        .to_string();
    assert!(err.contains("research_time"), "got: {err}");
}

#[test]
fn a_value_is_fixed_point_not_a_float() {
    // Float state is banned in the simulation (D9); the loader's job is to hand the engine fixed
    // point, even for effects, which are read once and then applied.
    let text = r#"
schema = 1
kind = "effects"

[[effect]]
id = 1
name = "x"
commands = [[4, 83, -1, 9, 1.25]]
"#;
    let effects = load_effects_str(text, "effects.toml").expect("loads");
    let command = effects.get(1).expect("effect 1").commands[0];
    assert_eq!(command.kind, 4);
    assert_eq!(command.value, MaybeFixed::from_f64(1.25));
}
