//! A site may be placed where a villager stands, and the villager is pushed off it.
//!
//! The game lets you drop a building on your own units. It has to: once deposits are made from a **touching** tile
//! (2026-10-29), villagers stand around every drop site and around the town centre, and a placement rule that treats a
//! unit as an obstruction leaves a player unable to build anywhere near their own base - which is exactly what the
//! acceptance match hit, with *"no free tile for a market"* refused 177 times in a row.

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

/// A villager standing exactly where the site goes: the site is accepted and the villager ends up somewhere else.
#[test]
fn a_site_is_allowed_where_a_villager_stands() {
    let Some(mut world) = fresh() else {
        return;
    };
    let house = world.role_unit("house").expect("a house role");
    let villager = world.role_unit("villager").expect("a villager role");
    let scale = world.scale();
    let spot = (40u16, 40u16);

    let walker = next_id(&world);
    world
        .submit_checked(Command::Spawn {
            id: walker,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (Fixed::from_raw(40 * scale), Fixed::from_raw(40 * scale)),
            speed: Fixed::from_raw(scale / 2),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("one tick");

    let builder = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit && entity.id != walker)
        .map(|entity| entity.id)
        .expect("a builder");

    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: house,
            at: spot,
        })
        .expect("a site where a villager stands is legal");

    world.step().expect("one tick");
    let pushed = world.entity(walker).map(|entity| {
        (
            entity.position.0.raw() / scale,
            entity.position.1.raw() / scale,
        )
    });
    assert_ne!(
        pushed,
        Some((i64::from(spot.0), i64::from(spot.1))),
        "the villager is pushed off the new site, not buried in it"
    );
}
