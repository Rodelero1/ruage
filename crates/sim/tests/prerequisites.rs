//! Building prerequisites: a workshop needs a blacksmith, and losing the blacksmith does not matter.
//!
//! Written before the implementation (D7, D63). The game's rule, from two independent descriptions and the
//! building table: a **Siege Workshop requires a Blacksmith** (AoE2; the Archery Range prerequisite the owner
//! remembered belongs to the first Age of Empires, where it is Barracks + Archery Range).
//!
//! The part that decides the implementation is the *tense*: the requirement is that the building has been built
//! **once**, not that it still stands. Losing the blacksmith does not take the workshop away, so this is a
//! per-player memory rather than a scan of the map.

use std::path::PathBuf;

use ruage_data::{Economy, Effects, Map, Sim as Settings, Techs, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, SimError, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: Settings,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
    combat: ruage_data::Combat,
    techs: Techs,
    effects: Effects,
    civs: ruage_data::Civs,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
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
        techs: ruage_data::load_techs_file(&repo("config/gamedata/techs.toml")).expect("techs"),
        effects: ruage_data::load_effects_file(&repo("config/gamedata/effects.toml"))
            .expect("effects"),
        civs: ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs"),
    })
}

fn world(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("research tables");
    world.load_civs(&fx.civs).expect("civs");
    world
}

fn fresh_id(world: &World) -> u32 {
    world
        .entities()
        .iter()
        .map(|entity| entity.id.0)
        .max()
        .unwrap_or(0)
        + 1
}

fn spawn_building(world: &mut World, player: u8, role: &str, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let id = EntityId(fresh_id(world));
    let unit = world
        .role_unit(role)
        .unwrap_or_else(|| panic!("content has no role {role}"));
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Building,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp: 1_800,
            amount: 0,
        })
        .expect("spawn building");
    world.step().expect("step");
    id
}

fn villager(world: &mut World, player: u8, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let id = EntityId(fresh_id(world));
    let unit = world.role_unit("villager").expect("villager");
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
            speed: Fixed::parse("1.0", scale).expect("a walkable speed"),
            hp: 25,
            amount: 0,
        })
        .expect("spawn villager");
    world.step().expect("step");
    id
}

/// A build order next to `worker`, at the first neighbouring tile that will take it.
fn build_beside(world: &mut World, worker: EntityId, role: &str) -> Result<(), SimError> {
    let unit = world
        .role_unit(role)
        .unwrap_or_else(|| panic!("content has no role {role}"));
    let scale = world.scale();
    let (x, y) = world
        .entity(worker)
        .map(|entity| {
            (
                entity.position.0.raw() / scale,
                entity.position.1.raw() / scale,
            )
        })
        .expect("the worker");
    let mut last = Ok(());
    for (dx, dy) in [(1i64, 0i64), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, -1)] {
        let order = world.submit_checked(Command::Build {
            villager: worker,
            unit,
            at: ((x + dx) as u16, (y + dy) as u16),
        });
        match order {
            Ok(()) => return Ok(()),
            Err(err) => last = Err(err),
        }
    }
    last
}

#[test]
fn a_siege_workshop_needs_a_blacksmith() {
    let Some(fx) = fixture() else { return };
    let mut world = world(&fx);
    // The Castle Age so the *age* gate cannot be what refuses; the prerequisite is the only variable.
    world.grant_tech(1, 102).expect("the Castle Age");
    let worker = villager(&mut world, 1, (20, 20));

    let refused = build_beside(&mut world, worker, "siege_workshop");
    assert!(
        matches!(refused, Err(SimError::MissingBuilding { .. })),
        "a siege workshop was placed with no blacksmith: {refused:?}"
    );

    // A blacksmith that has been built once is enough.
    spawn_building(&mut world, 1, "blacksmith", (23, 20));
    let allowed = build_beside(&mut world, worker, "siege_workshop");
    assert!(
        allowed.is_ok(),
        "a blacksmith did not open the siege workshop: {allowed:?}"
    );
}

#[test]
fn a_lost_blacksmith_does_not_take_the_workshop_away() {
    // The tense of the rule: built **once**. Destroy the blacksmith and the right remains.
    let Some(fx) = fixture() else { return };
    let mut world = world(&fx);
    world.grant_tech(1, 102).expect("the Castle Age");
    let worker = villager(&mut world, 1, (20, 20));
    let smith = spawn_building(&mut world, 1, "blacksmith", (23, 20));
    world
        .submit_checked(Command::Delete {
            player: 1,
            entity: smith,
        })
        .expect("raze the blacksmith");
    world.step().expect("step");
    assert!(
        world.entity(smith).is_none(),
        "the blacksmith survived being razed"
    );

    let allowed = build_beside(&mut world, worker, "siege_workshop");
    assert!(
        allowed.is_ok(),
        "losing the blacksmith took the siege workshop away: {allowed:?}"
    );
}
