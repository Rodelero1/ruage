//! Ballistics: whether a shot is aimed where the target **is** or where it **will be**.
//!
//! Written before the implementation (D7). A projectile already exists, already flies to an aim point and
//! already lands there (D35); what it does not do is care whether anything is *at* that point, so no shot can
//! miss and "ballistics" would have nothing to change. Both halves are needed together: an arrow that can miss
//! a runner, and a researched flag that stops it missing.

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};
use std::path::PathBuf;

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

/// Ballistics as content: a free, instant tech whose effect sets attribute 19 on the archer. The real tech
/// requires a university and a barracks; the attribute is the same one either way, and this test is about
/// what the flag *does*, not about how it is bought.
const BALLISTICS_TECHS: &str = r#"
schema = 1
kind = "techs"
[[tech]]
id = 900
effect_id = 900
research_time = 0
costs = []
required = []
min_required = 0
location = 0
"#;

const BALLISTICS_EFFECTS: &str = r#"
schema = 1
kind = "effects"
[[effect]]
id = 900
name = "Ballistics (test)"
commands = []
resource_modifiers = []
modifiers = [{ op = "set", attribute = "ballistics", attribute_class = -1, unit = 4, unit_class = -1, value = 1.0 }]
"#;

const ARCHER: u32 = 4;
const VILLAGER: u32 = 83;
const DECOY: u32 = 93;
const BALLISTICS: u32 = 900;

fn world_with(fx: &Fixture, ballistics: bool) -> World {
    let mut economy = fx.economy.clone();
    economy.start.stock = [("food".to_owned(), 4_000), ("wood".to_owned(), 4_000)]
        .into_iter()
        .collect();
    // The pack's starting techs are real dat ids; this fixture brings its own one-line table, so it
    // asks for none. Content is the test's, not the match's.
    economy.start.techs.clear();
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    let techs = ruage_data::load_techs_str(BALLISTICS_TECHS, "techs.toml").expect("test techs");
    let effects =
        ruage_data::load_effects_str(BALLISTICS_EFFECTS, "effects.toml").expect("test effects");
    world.load_research(&techs, &effects).expect("research");
    if ballistics {
        world.grant_tech(1, BALLISTICS).expect("Ballistics");
    }
    world
}

fn place(world: &mut World, player: u8, id: u32, unit: u32, tile: (u16, u16), hp: i32) -> EntityId {
    spawn(world, player, id, unit, tile, hp, 0.0)
}

fn spawn(
    world: &mut World,
    player: u8,
    id: u32,
    unit: u32,
    tile: (u16, u16),
    hp: i32,
    speed: f64,
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
                Fixed::from_raw(i64::from(tile.0) * scale),
                Fixed::from_raw(i64::from(tile.1) * scale),
            ),
            speed: Fixed::from_raw((speed * f64::from(scale as u16)) as i64),
            hp,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

/// The shot: an archer at its full reach fires at a runner. Without Ballistics the arrow goes where the
/// runner *was* and lands in the grass; with it, the arrow goes where the runner *will be* and draws blood.
///
/// The runner is deliberately quick and the shot deliberately long, because a slow target a short way off
/// would be inside the arrow's landing tolerance either way and would prove nothing.
#[test]
fn an_arrow_misses_a_runner_unless_ballistics_leads_the_shot() {
    let Some(fx) = fixture() else { return };
    let run = |ballistics: bool| -> (i32, usize) {
        let mut world = world_with(&fx, ballistics);
        let archer = spawn(&mut world, 1, 9_960, ARCHER, (10, 10), 30, 0.96);
        // Three tiles away, inside an archer's four-tile reach, walking hard south from the moment it
        // exists: it is attacking a decoy far below, which is how a unit is made to move for a while.
        let runner = spawn(&mut world, 2, 9_961, VILLAGER, (13, 10), 400, 2.5);
        let decoy = place(&mut world, 1, 9_962, DECOY, (14, 40), 5_000);
        world
            .submit_checked(Command::Attack {
                attacker: runner,
                target: decoy,
            })
            .expect("the runner walks away");
        world
            .submit_checked(Command::Attack {
                attacker: archer,
                target: runner,
            })
            .expect("the archer shoots at it");
        for _ in 0..40 {
            world.step().expect("step");
        }
        let hp = world.entity(runner).expect("the runner").hp;
        let arrows = world
            .entities()
            .iter()
            .filter(|e| e.kind == EntityKind::Projectile)
            .count();
        (hp, arrows)
    };

    let (plain_hp, _) = run(false);
    let (led_hp, _) = run(true);
    assert_eq!(
        plain_hp, 400,
        "without Ballistics the arrow lands where the runner was, and it is not there"
    );
    assert!(
        led_hp < 400,
        "with Ballistics the arrow is aimed at where the runner will be"
    );
}

/// The control: a target that does not move is hit either way, so the miss above is about movement and not
/// about the landing check being broken.
#[test]
fn an_arrow_still_hits_a_target_that_stands_still() {
    let Some(fx) = fixture() else { return };
    let mut world = world_with(&fx, false);
    let archer = place(&mut world, 1, 9_970, ARCHER, (10, 10), 30);
    let victim = place(&mut world, 2, 9_971, VILLAGER, (14, 10), 400);
    assert_eq!(
        world.entity(victim).expect("victim").task,
        Task::Idle,
        "the victim is standing still"
    );
    world
        .submit_checked(Command::Attack {
            attacker: archer,
            target: victim,
        })
        .expect("an attack order");
    for _ in 0..40 {
        world.step().expect("step");
    }
    assert!(
        world.entity(victim).expect("victim").hp < 400,
        "a stationary target takes the arrow"
    );
}
