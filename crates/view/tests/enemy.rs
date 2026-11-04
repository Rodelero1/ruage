//! **An enemy's numbers, seen through the fog.** The owner asked for "enemy hit points on click" (2026-11-04); the panel
//! gained the bar, and this file is what keeps it honest: a full-health enemy must read as full, and an enemy the player
//! cannot see must not be in the snapshot at all.
//!
//! Written before the fix (D7, SPEC §11): the snapshot computes each entity's maximum health with the **viewing** player's
//! upgrades, which is right for your own things and wrong for somebody else's - an enemy carrying its own hit points read
//! as a fraction of *your* maximum.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};
use ruage_view::Snapshot;

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
    let techs = ruage_data::load_techs_file(&repo("config/gamedata/techs.toml")).ok()?;
    let effects = ruage_data::load_effects_file(&repo("config/gamedata/effects.toml")).ok()?;
    let civs = ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).ok()?;
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    world.load_combat(&combat);
    world.load_research(&techs, &effects).ok()?;
    world.load_civs(&civs).ok()?;
    Some(world)
}

fn unit_role(world: &World) -> u32 {
    world.start_villager_unit().expect("a villager role")
}

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

/// A free tile within a few tiles of the player's first building, so an enemy placed there is inside the base's sight.
fn spot_beside_the_base(world: &World) -> Option<(u16, u16)> {
    let scale = world.scale();
    let home = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)?;
    let (hx, hy) = (home.position.0.raw() / scale, home.position.1.raw() / scale);
    for radius in 2..6i64 {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx.abs() != radius && dy.abs() != radius {
                    continue;
                }
                let (x, y) = (hx + dx, hy + dy);
                let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
                    continue;
                };
                if world.is_free_ground(x, y) {
                    return Some((x, y));
                }
            }
        }
    }
    None
}

fn spawn(world: &mut World, player: u8, at: (u16, u16)) -> EntityId {
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
    let unit = unit_role(world);
    let hp = world
        .max_hp_of(player, unit)
        .expect("the pack gives a villager health");
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(at.0) * scale),
                Fixed::from_raw(i64::from(at.1) * scale),
            ),
            speed: Fixed::ZERO,
            hp,
            amount: 0,
        })
        .expect("a spawn lands");
    world.step().expect("step");
    id
}

/// **A full-health enemy reads as full.** The player researches something that raises *their* villagers' health; the enemy
/// on the map keeps the health the enemy's own pack gives it, and the panel must not divide it by the viewer's maximum.
#[test]
fn an_enemy_at_full_health_reads_as_full() {
    let Some(mut world) = fresh() else { return };
    // **In sight of the player's own base**, which is the only way an enemy is in the snapshot at all: the fog is the
    // filter this whole slice rests on.
    let Some(at) = spot_beside_the_base(&world) else {
        return;
    };
    let theirs = spawn(&mut world, 2, at);
    // A tech that raises villager health for player 1 only (Loom is +15 in the reference pack, and it is a Dark Age tech
    // at the town centre, which is what the opening has).
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .map(|entity| entity.id)
        .expect("the seeded base has a building");
    let loom = world
        .tech_role("loom")
        .expect("the pack names Loom as a role");
    world
        .submit_checked(Command::Research {
            building: centre,
            tech: loom,
        })
        .expect("Loom can be researched by player 1");
    // **Wait for the tech to land**, and say so if it never does: Loom takes 25 seconds, and a test that stepped a few
    // ticks would find the same maximum for both players and prove nothing (the first version of this test did exactly
    // that and passed with the bug in place).
    let role = unit_role(&world);
    let mut researched = false;
    for _ in 0..4_000 {
        world.step().expect("step");
        if world.max_hp_of(1, role) != world.max_hp_of(2, role) {
            researched = true;
            break;
        }
    }
    assert!(
        researched,
        "player 1's Loom never changed its villagers' maximum health, so this test proves nothing"
    );

    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).expect("pack");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let snapshot = Snapshot::of(&world, &map, 1);
    let row = snapshot
        .entities
        .iter()
        .find(|entity| entity.id == theirs.0)
        .expect("the enemy is in sight of the base");
    assert!(
        (row.hp_fraction - 1.0).abs() < 0.001,
        "an enemy at full health reads as {} - the viewer's maximum was used for somebody else's body",
        row.hp_fraction
    );
}

/// **An enemy out of sight is not in the snapshot**, so no click can reach them: the fog is the filter, not the selection.
#[test]
fn an_enemy_across_the_map_is_not_in_the_snapshot() {
    let Some(mut world) = fresh() else { return };
    // The free tile furthest from the player's own base: out of every sight radius on this map.
    let Some(here) = open_spot(&world) else {
        return;
    };
    let mut far = here;
    let mut best = 0i64;
    for y in 2..62u16 {
        for x in 2..62u16 {
            if !world.is_free_ground(x, y) {
                continue;
            }
            let (dx, dy) = (
                i64::from(x) - i64::from(here.0),
                i64::from(y) - i64::from(here.1),
            );
            if dx * dx + dy * dy > best {
                best = dx * dx + dy * dy;
                far = (x, y);
            }
        }
    }
    let theirs = spawn(&mut world, 2, far);
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).expect("pack");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let snapshot = Snapshot::of(&world, &map, 1);
    assert!(
        !snapshot.entities.iter().any(|entity| entity.id == theirs.0),
        "an enemy the player has never seen is in the snapshot"
    );
}
