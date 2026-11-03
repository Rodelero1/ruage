//! A spent farm stays on the map and is replaced by a new one on the same tile, for the full 60 wood (the owner's rule,
//! 2026-10-30).

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
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).ok()?;
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    world.load_combat(&combat);
    Some(world)
}

fn next_id(world: &World) -> EntityId {
    EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1)
}

/// Place a farm at `at` with `food` left in it, and return its id.
fn farm(world: &mut World, at: (u16, u16), food: u32) -> EntityId {
    let role = world.role_unit("farm").expect("a farm role");
    let id = next_id(world);
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(role),
            position: (
                Fixed::from_raw(i64::from(at.0) * scale),
                Fixed::from_raw(i64::from(at.1) * scale),
            ),
            speed: Fixed::ZERO,
            hp: 45,
            amount: food,
        })
        .expect("a farm");
    world.step().expect("one tick");
    id
}

/// The **placement search** asks `is_free_ground`, not `site_fits`: a spent farm has to read as free ground or the search
/// never offers its tile, and the farm rule refuses with *"no free tile for a farm"* while nine husks lie in its own base
/// (measured: 105 refusals in one acceptance run). A working farm still holds its ground.
#[test]
fn a_spent_farm_is_free_ground() {
    let Some(mut world) = fresh() else {
        return;
    };
    assert!(
        world.is_free_ground(40, 40),
        "nothing stands there to begin with"
    );
    farm(&mut world, (40, 40), 250);
    assert!(
        !world.is_free_ground(40, 40),
        "a farm with food left holds its ground"
    );
    farm(&mut world, (44, 44), 0);
    assert!(
        world.is_free_ground(44, 44),
        "a spent farm is a husk, and a husk is ground the next farm can use"
    );
}

/// **A farm is a floor, so a farm beside a farm walls nobody in.** The lane rule exists so a base does not become walls -
/// and a plot whose footprint is entirely walkable is not a wall. Without this exemption a spent farm *inside* a field
/// cannot be rebuilt: the search finds the husk's tile, `is_free_ground` says yes, and `site_has_lane` says no, which is
/// what the 105 *"no free tile for a farm"* refusals in an acceptance run were. A house next to a house is still refused.
#[test]
fn a_farm_beside_a_farm_leaves_a_lane() {
    let Some(mut world) = fresh() else {
        return;
    };
    let farm_unit = world.role_unit("farm").expect("a farm role");
    let house_unit = world.role_unit("house").expect("a house role");
    farm(&mut world, (40, 40), 250);
    assert!(
        world.site_has_lane(1, farm_unit, (43, 40)),
        "a farm may stand beside a farm: the plot blocks nothing"
    );
    let house = next_id(&world);
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id: house,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(house_unit),
            position: (Fixed::from_raw(30 * scale), Fixed::from_raw(30 * scale)),
            speed: Fixed::ZERO,
            hp: 550,
            amount: 0,
        })
        .expect("a house");
    world.step().expect("one tick");
    assert!(
        !world.site_has_lane(1, house_unit, (32, 30)),
        "two houses edge to edge are a wall, and the lane rule still says so"
    );
}

/// A farm with no food left is an expiring husk: it is still there, and a new farm may be built on its tile.
#[test]
fn a_spent_farm_is_replaced_on_its_own_tile() {
    let Some(mut world) = fresh() else {
        return;
    };
    let spent = farm(&mut world, (40, 40), 0);
    assert!(
        world.is_expired_farm(spent),
        "a farm with no food is expired"
    );
    assert!(
        world.entity(spent).is_some(),
        "and it is still on the map until it is replaced"
    );

    let builder = world
        .entities()
        .iter()
        .find(|e| e.player == 1 && e.kind == EntityKind::Unit)
        .map(|e| e.id)
        .expect("a builder");
    let role = world.role_unit("farm").expect("a farm role");
    let before = world.stock(1, world.slot_of("wood").expect("wood"));

    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: role,
            at: (40, 40),
        })
        .expect("a new farm may be placed on the spent one");

    world.step().expect("one tick");
    assert!(
        world.entity(spent).is_none(),
        "the spent farm is replaced, not stacked"
    );
    let after = world.stock(1, world.slot_of("wood").expect("wood"));
    assert_eq!(before - after, 60, "and the rebuild costs the full 60 wood");
}
