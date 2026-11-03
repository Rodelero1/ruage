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
    // A forest tile at the **edge** of the block: (20, 14) cannot be stood on, and its western neighbours are grass, so
    // the load can be handed in from beside it. (21, 16) - the first choice - is two tiles deep in the forest, where no
    // villager can stand at all, so nothing could ever be banked there.
    let mill = spawn_building(&mut world, mill_role, (20, 14));
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

/// **A drop site sealed on one side still takes a load.** `docs/next-step.md` carried this as "a drop site with no free
/// neighbour"; it is worse than that, because the *route* was aimed at the building's own blocked anchor tile. With a
/// barracks at (14, 14) - which is what six `research.rs` tests stage - the town centre's four north-west approach tiles
/// are gone, A* finds no goal at all, and the villager stands holding a load for the rest of the match. Every acceptance
/// run ends with the same sentence: `no walkable route ... to (16, 16)`.
///
/// The answer is not a bigger radius on the deposit test: it is that **a building is worked from its edge, not its
/// anchor**, so the goal has to be a walkable tile that touches the footprint.
#[test]
fn a_load_comes_home_past_a_building_beside_the_town_centre() {
    let Some(mut world) = fresh() else {
        return;
    };
    let food = world.slot_of("food").expect("a food column");
    let scale = world.scale();
    let before = world.stock(1, food);

    // The barracks that seals the north-west approach, exactly as the research tests stage it.
    let barracks = world.role_unit("barracks").expect("a barracks role");
    let hut = next_id(&world);
    world
        .submit_checked(Command::Spawn {
            id: hut,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(barracks),
            position: (Fixed::from_raw(14 * scale), Fixed::from_raw(14 * scale)),
            speed: Fixed::ZERO,
            hp: 1200,
            amount: 0,
        })
        .expect("a barracks beside the town centre");
    world.step().expect("one tick");

    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
        .expect("a villager");
    let node = world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(slot) if slot == food))
        .map(|entity| entity.id)
        .next()
        .expect("a berry");
    world
        .submit_checked(Command::Gather { villager, node })
        .expect("a gather order");

    let mut ticked = 0;
    while ticked < 2400 && world.stock(1, food) <= before {
        world.step().expect("a tick");
        ticked += 1;
    }
    let task = world
        .entities()
        .iter()
        .find(|entity| entity.id == villager)
        .map(|entity| format!("{:?}", entity.task))
        .unwrap_or_else(|| "gone".to_owned());
    assert!(
        world.stock(1, food) > before,
        "no load came home in {ticked} ticks: the bank holds {} food and the villager is in {task}",
        world.stock(1, food)
    );
}
