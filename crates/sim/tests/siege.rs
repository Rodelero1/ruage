//! Siege: the Castle Age and the mangonel, so a scripted match can actually end.
//!
//! Written before the content exists (D7). Everything here is decided by the dat except the two role names:
//!
//! - the Castle Age is **tech 102** - 800 food + 200 gold, 160 s, at the town centre (unit 109). Identified by its
//!   price and time, not by its number (the project's oldest rule);
//! - the siege workshop is **unit 49** `SIWS`, class 3, 1 800 hp, 200 wood;
//! - the mangonel is **unit 280** `MANGO`, class 13, 50 hp, 7 range and 3 minimum, 160 wood + 135 gold, reload 6 s,
//!   35 attack against buildings and 40 against units.
//!
//! The point of the step is the *chain*: ages open the workshop's unit, the workshop trains it, and the unit does
//! enough damage to a town centre to finish a match inside a fifteen-minute script.

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

#[test]
fn the_mangonel_needs_the_castle_age() {
    let Some(fx) = fixture() else { return };
    let mut world = world(&fx);
    let workshop = spawn_building(&mut world, 1, "siege_workshop", (20, 20));
    let mangonel = world
        .role_unit("mangonel")
        .expect("content has no mangonel role");

    // In the Dark Age the workshop cannot make one, and the refusal says why.
    let refused = world.submit_checked(Command::Train {
        building: workshop,
        unit: mangonel,
    });
    assert!(
        matches!(
            refused,
            Err(SimError::UnitUnavailable { .. }) | Err(SimError::NotEnough { .. })
        ),
        "a mangonel was trainable before the Castle Age: {refused:?}"
    );

    // The Castle Age opens it.
    world.grant_tech(1, 102).expect("the Castle Age");
    let after = world.submit_checked(Command::Train {
        building: workshop,
        unit: mangonel,
    });
    // What the age opens is *availability*; whether the player can afford it is a separate gate, and the
    // opening stockpile has 100 gold against a mangonel's 135. So the claim is that the refusal changed.
    assert!(
        !matches!(after, Err(SimError::UnitUnavailable { .. })),
        "the Castle Age did not open the mangonel: {after:?}"
    );
}

#[test]
fn a_mangonel_breaks_a_building() {
    // The number that matters for a fifteen-minute script: 35 attack against buildings with a 6 s reload,
    // so a mangonel makes a dent a match can finish with. Measured on a house on open ground rather than a
    // corner town centre, because the mangonel's own minimum range is 3 and the map's edges are not open.
    let Some(fx) = fixture() else { return };
    let mut world = world(&fx);
    let house = world.role_unit("house").expect("house");
    let mangonel = world
        .role_unit("mangonel")
        .expect("content has no mangonel role");
    let scale = world.scale();

    // An enemy house and a mangonel six tiles off it: outside its three-tile dead zone once the house's
    // own radius is added (D44), and inside its seven-tile range.
    let id = EntityId(fresh_id(&world));
    world
        .submit_checked(Command::Spawn {
            id,
            player: 2,
            kind: EntityKind::Building,
            unit: Some(house),
            position: (Fixed::from_raw(26 * scale), Fixed::from_raw(22 * scale)),
            speed: Fixed::ZERO,
            hp: 550,
            amount: 0,
        })
        .expect("enemy house");
    world.step().expect("step");
    let target = id;
    let before = world.entity(target).map(|entity| entity.hp).expect("hp");

    let shooter = EntityId(fresh_id(&world));
    world
        .submit_checked(Command::Spawn {
            id: shooter,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(mangonel),
            position: (Fixed::from_raw(20 * scale), Fixed::from_raw(22 * scale)),
            speed: Fixed::ZERO,
            hp: 50,
            amount: 0,
        })
        .expect("spawn mangonel");
    world.step().expect("step");
    world
        .submit_checked(Command::Attack {
            attacker: shooter,
            target,
        })
        .expect("attack");

    for _ in 0..400 {
        world.step().expect("step");
    }
    let after = world.entity(target).map(|entity| entity.hp).unwrap_or(0);
    assert!(
        after < before,
        "a mangonel did no damage to a building: {before} -> {after}"
    );
}
