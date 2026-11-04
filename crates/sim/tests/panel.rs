//! **What a resource node holds, as the panel needs to know it.**
//!
//! The owner's ask (2026-11-04): *"I would like trees to be clickable showing hit points and volume of wood they have.
//! Same for farm, can I see amount of food it has left? Same for berries, stone and gold."*
//!
//! The snapshot already carries an entity's amount and its hit points. What it does **not** carry is *which resource* the
//! amount is: a node knows its column (`EntityKind::Resource(slot)`), but a **farm is a building**, and only content says
//! which column a farm fills (`economy.farm.resource`). Without that the panel can print a number and not say whose number
//! it is. `World::held_slot` is the engine's answer, and the pack names the column - the view never learns what food is
//! (D17).
//!
//! Written before the implementation (D7, SPEC §11).

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

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
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    Some(world)
}

/// A free patch of ground, away from the seeded base, so a hand-placed thing does not land on a tree.
fn open_spot(world: &World) -> Option<(u16, u16)> {
    for y in 2..50u16 {
        for x in 2..50u16 {
            if (24..40).contains(&x) && (24..40).contains(&y) {
                continue;
            }
            if world.is_free_ground(x, y) {
                return Some((x, y));
            }
        }
    }
    None
}

fn spawn(
    world: &mut World,
    kind: EntityKind,
    unit: Option<u32>,
    at: (u16, u16),
    amount: u32,
) -> EntityId {
    let id = EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: if matches!(kind, EntityKind::Resource(_)) {
                0
            } else {
                1
            },
            kind,
            unit,
            position: (
                Fixed::from_raw(i64::from(at.0) * scale),
                Fixed::from_raw(i64::from(at.1) * scale),
            ),
            speed: Fixed::ZERO,
            hp: 20,
            amount,
        })
        .expect("a spawn lands");
    world.step().expect("step");
    id
}

/// A tree knows its column from its own kind - the dat's wood line, resolved by the pack.
#[test]
fn a_tree_reports_the_column_it_holds() {
    let Some(mut world) = fresh() else { return };
    let Some(at) = open_spot(&world) else { return };
    let wood = world.slot_of("wood").expect("a wood column");
    let tree = spawn(&mut world, EntityKind::Resource(wood), None, at, 100);
    assert_eq!(
        world.held_slot(tree),
        Some(wood),
        "a tree that holds wood does not say so"
    );
}

/// **A farm is a building**, so its column is content's answer (`[farm] resource`), not the view's guess.
#[test]
fn a_farm_reports_the_food_it_holds() {
    let Some(mut world) = fresh() else { return };
    let Some(at) = open_spot(&world) else { return };
    let food = world.slot_of("food").expect("a food column");
    let farm = world.farm_unit().expect("a farm role");
    let entity = spawn(&mut world, EntityKind::Building, Some(farm), at, 250);
    assert_eq!(
        world.held_slot(entity),
        Some(food),
        "a farm that holds food does not say so"
    );
}

/// A house holds nothing, and the panel must not invent a column for it.
#[test]
fn a_house_holds_nothing() {
    let Some(mut world) = fresh() else { return };
    let Some(at) = open_spot(&world) else { return };
    let house = world.role_unit("house").expect("a house role");
    let entity = spawn(&mut world, EntityKind::Building, Some(house), at, 0);
    assert_eq!(world.held_slot(entity), None, "a house is not a resource");
}
