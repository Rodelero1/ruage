//! Six villagers on one bush all get their loads into the bank.
//!
//! The dump at 1,600 s of the acceptance match showed the shape of the freeze: **27 villagers in `Return`, all pointing at
//! the mill, all stuck at (13.08, 19.92)** - eight hundredths of a tile short of a tile they can never stand on. The
//! deposit test was "no step left *and* the step is the drop site", so the last waypoint - the building's own, blocked,
//! tile - was never reached and nothing was ever banked.
//!
//! **It passes.** Written to reproduce the pile-up and it did not: six villagers, one bush, one drop site, all banked. So
//! the crowd is not the fault by itself - the acceptance match's mill stands **among farms** (`[13,19]` with farms at
//! `[12,18]`, `[13,18]`, `[14,18]`, `[14,20]`), and the 27 stuck villagers sat 0.08 tiles outside a mill whose standable
//! neighbours were built over. What this file is worth is the guard: no change may make a crowd stop banking. The
//! remaining question - a drop site with no free neighbour - is in `docs/next-step.md`.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn fresh() -> Option<World> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).ok()?;
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).ok()?;
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).ok()?;
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).ok()?;
    let units = ruage_data::load_units_file(&units).ok()?;
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).ok()?;
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    world.load_combat(&combat);
    Some(world)
}

fn next_id(world: &World) -> EntityId {
    EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    )
}

/// A crowd at one node: every load has to reach the bank, because that is what a real opening looks like - six villagers
/// on the berries, one drop site.
#[test]
fn a_crowd_all_banks_its_loads() {
    let Some(mut world) = fresh() else {
        return;
    };
    let food = world.slot_of("food").expect("a food column");
    let villager_role = world.role_unit("villager").expect("a villager role");
    let scale = world.scale();
    let before = world.stock(1, food);

    // A bush with plenty in it, two tiles from the town centre, and six villagers round it.
    let bush = next_id(&world);
    world
        .submit_checked(Command::Spawn {
            id: bush,
            player: 0,
            kind: EntityKind::Resource(food),
            unit: None,
            position: (Fixed::from_raw(18 * scale), Fixed::from_raw(14 * scale)),
            speed: Fixed::ZERO,
            hp: 0,
            amount: 600,
        })
        .expect("a bush");
    world.step().expect("one tick");

    let pace = world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == 1
                && entity
                    .unit
                    .is_some_and(|u| world.is_family(u, villager_role))
        })
        .map(|entity| entity.speed)
        .expect("a villager with a speed");
    let spots = [(17, 14), (17, 15), (18, 15), (19, 14), (19, 15), (17, 13)];
    let mut crew = Vec::new();
    for (index, (x, y)) in spots.iter().enumerate() {
        let id = next_id(&world);
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager_role),
                position: (
                    Fixed::from_raw(i64::from(*x) * scale),
                    Fixed::from_raw(i64::from(*y) * scale),
                ),
                speed: pace,
                hp: 25,
                amount: 0,
            })
            .expect("a villager");
        crew.push((id, index));
        world.step().expect("one tick");
    }
    for (id, _) in &crew {
        world
            .submit_checked(Command::Gather {
                villager: *id,
                node: bush,
            })
            .expect("the bush can be worked");
    }

    // Two minutes: six villagers, ten food a trip, one drop site a few tiles away.
    for _ in 0..2_400 {
        world.step().expect("stepping");
    }
    let gained = world.stock(1, food) - before;
    assert!(
        gained >= 20,
        "the loads have to reach the bank: gained {gained} in two minutes"
    );
    let returning = crew
        .iter()
        .filter(|(id, _)| {
            world
                .entity(*id)
                .is_some_and(|entity| matches!(entity.task, Task::Return { .. }))
        })
        .count();
    assert!(
        returning < crew.len(),
        "somebody has to be banking, not all {returning} of {} stuck on the way",
        crew.len()
    );
}
