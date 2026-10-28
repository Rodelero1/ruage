//! Blast damage: a shot that hurts a group, not a point.
//!
//! Written before the implementation (D7). The dat carries it: `blast_width` is the radius in tiles and
//! `blast_attack_level` how many rings of it bite, and 27 of 1,166 units have a radius - mangonel 1.0,
//! onager 1.25, bombard cannon 0.5, petard 0.5. So this is imported data, not a list of siege ids.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
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
}

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

fn hp_of(world: &World, id: EntityId) -> i32 {
    world.entity(id).map_or(0, |entity| entity.hp)
}

/// Fire at `target` from a shooter standing two tiles beyond the mangonel's dead zone, and let the shot
/// land. The magazine is generous: a mangonel reloads in six seconds.
fn shoot(world: &mut World, shooter: EntityId, target: EntityId) {
    world
        .submit_checked(Command::Attack {
            attacker: shooter,
            target,
        })
        .expect("attack");
    for _ in 0..400 {
        world.step().expect("step");
    }
}

#[test]
fn the_data_carries_a_blast_radius() {
    let Some(fx) = fixture() else { return };
    let mangonel = fx.units.unit(280).expect("the mangonel");
    let archer = fx.units.unit(4).expect("the archer");
    assert_eq!(
        mangonel
            .blast_radius
            .resolve(20_000, "blast_radius")
            .expect("a radius")
            .raw(),
        20_000,
        "a mangonel's blast is one tile"
    );
    assert_eq!(
        archer
            .blast_radius
            .resolve(20_000, "blast_radius")
            .expect("a radius")
            .raw(),
        0,
        "an archer's arrow hits one thing"
    );
}

#[test]
fn a_mangonel_shot_hurts_the_group_around_its_target() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    // Range 7, dead zone 3: four tiles away is a legal shot.
    let mangonel = place(&mut world, 1, 9_600, 280, (20, 20), 50);
    let target = place(&mut world, 2, 9_601, spearman, (24, 20), 500);
    let bystander = place(&mut world, 2, 9_602, spearman, (24, 21), 500);

    shoot(&mut world, mangonel, target);

    assert!(hp_of(&world, target) < 500, "the target was not hit");
    assert!(
        hp_of(&world, bystander) < 500,
        "the unit standing beside the target was untouched: the shot is a point"
    );
}

#[test]
fn an_archer_s_arrow_hits_one_unit() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let archer = place(&mut world, 1, 9_603, 4, (20, 20), 30);
    let target = place(&mut world, 2, 9_604, spearman, (23, 20), 500);
    let bystander = place(&mut world, 2, 9_605, spearman, (23, 21), 500);

    shoot(&mut world, archer, target);

    assert!(hp_of(&world, target) < 500, "the archer missed");
    assert_eq!(
        hp_of(&world, bystander),
        500,
        "an arrow splashed the unit beside its target"
    );
}

#[test]
fn an_instant_shot_still_blasts() {
    let Some(fx) = fixture() else { return };
    // A pack that says shots land instantly. The blast is about where the stone lands, so it must not
    // depend on whether an arrow entity flew there first.
    let text = std::fs::read_to_string(repo("config/combat.toml")).expect("combat.toml");
    let text = text.replace("projectile_speed = 10.0", "projectile_speed = 0.0");
    assert!(
        text.contains("projectile_speed = 0.0"),
        "the fixture edit applied"
    );
    let instant = ruage_data::load_combat_str(&text, "instant-combat.toml").expect("combat");

    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&instant);

    let spearman = fx.economy.role["spearman"];
    let mangonel = place(&mut world, 1, 9_612, 280, (20, 20), 50);
    let target = place(&mut world, 2, 9_613, spearman, (24, 20), 500);
    let bystander = place(&mut world, 2, 9_614, spearman, (24, 21), 500);

    shoot(&mut world, mangonel, target);

    assert!(hp_of(&world, target) < 500, "the target was not hit");
    assert!(
        hp_of(&world, bystander) < 500,
        "an instant shot hit one unit: the blast was attached to the arrow, not to the landing"
    );
}

#[test]
fn the_blast_hurts_the_shooter_s_own_units_too() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let mangonel = place(&mut world, 1, 9_606, 280, (20, 20), 50);
    let target = place(&mut world, 2, 9_607, spearman, (24, 20), 500);
    let friend = place(&mut world, 1, 9_608, spearman, (24, 21), 500);

    shoot(&mut world, mangonel, target);

    assert!(
        hp_of(&world, friend) < 500,
        "a mangonel's stone lands on its own side too: the blast was filtered to enemies"
    );
}

#[test]
fn a_unit_outside_the_blast_is_untouched() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let mangonel = place(&mut world, 1, 9_609, 280, (20, 20), 50);
    let target = place(&mut world, 2, 9_610, spearman, (24, 20), 500);
    // Three tiles from the target: outside a one-tile blast.
    let far = place(&mut world, 2, 9_611, spearman, (24, 23), 500);

    shoot(&mut world, mangonel, target);

    assert!(hp_of(&world, target) < 500, "the target was not hit");
    assert_eq!(
        hp_of(&world, far),
        500,
        "the blast reached a unit three tiles from the landing point"
    );
}
