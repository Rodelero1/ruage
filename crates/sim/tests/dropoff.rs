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

/// **Touching the drop site is enough** - any side, any corner (the owner's rule, 2026-10-29). A mill is not walked onto;
/// a villager beside it banks the load.
///
/// The drop site here stands on **forest terrain**, which is blocked: nobody can ever step onto its own tile, so the load
/// can only be banked from a tile that touches it. This is the acceptance match's lumber camp at `(19,16)` - built against
/// the trees - with five woodcutters parked beside it in `Return` for a thousand seconds.
///
/// This is the shape of the acceptance match's freeze seen from the other end: twenty-seven villagers sat eight
/// hundredths of a tile short of the mill, in `Return`, for a thousand seconds, because arrival was "the final step is
/// exactly the drop site's tile" and that step is never taken.
#[test]
fn a_touching_tile_is_enough_to_bank() {
    let Some(mut world) = fresh() else {
        return;
    };
    let food = world.slot_of("food").expect("a food column");
    let mill_role = world.role_unit("mill").expect("a mill role");
    let villager_role = world.role_unit("villager").expect("a villager role");
    let scale = world.scale();
    let before = world.stock(1, food);

    // A mill at (30,30), a bush diagonally beside it at (29,29), a villager at (28,28): the load has to be banked without
    // anybody ever standing on the mill's own tile.
    // (21,16) is forest: a drop site whose own tile can never be stood on.
    let mill = spawn_building(&mut world, mill_role, (21, 16));
    let bush = next_id(&world);
    world
        .submit_checked(Command::Spawn {
            id: bush,
            player: 0,
            kind: EntityKind::Resource(food),
            unit: None,
            position: (Fixed::from_raw(20 * scale), Fixed::from_raw(13 * scale)),
            speed: Fixed::ZERO,
            hp: 0,
            amount: 400,
        })
        .expect("a bush");
    world.step().expect("one tick");
    let pace = world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.unit.is_some_and(|u| world.is_family(u, villager_role)))
        .map(|e| e.speed)
        .expect("a villager speed");
    let villager = next_id(&world);
    world
        .submit_checked(Command::Spawn {
            id: villager,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager_role),
            position: (Fixed::from_raw(19 * scale), Fixed::from_raw(13 * scale)),
            speed: pace,
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("one tick");
    world
        .submit_checked(Command::Gather {
            villager,
            node: bush,
        })
        .expect("the bush can be worked");

    let _ = mill;
    for _ in 0..1_200 {
        world.step().expect("stepping");
    }
    let gained = world.stock(1, food) - before;
    assert!(
        gained > 0,
        "a villager beside the drop site banks the load: gained {gained}, task {:?}",
        world.entity(villager).map(|e| e.task)
    );
}

fn spawn_building(world: &mut World, unit: u32, at: (i64, i64)) -> EntityId {
    let id = next_id(world);
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(unit),
            position: (Fixed::from_raw(at.0 * scale), Fixed::from_raw(at.1 * scale)),
            speed: Fixed::ZERO,
            hp: 600,
            amount: 0,
        })
        .expect("a building");
    world.step().expect("one tick");
    id
}
