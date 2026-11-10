//! **A gathering villager is on screen on every tick** (D173).
//!
//! The owner: *"appear and disappear in rapid succession (see pictures 2 and 3 taken one after another, villagers didn't move in meanwhile as they were
//! gathering"*. Two frames, one villager gone. This walks the frames and says which tick it vanished on.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};
use ruage_view::{Camera, Snapshot, palette, render};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

#[test]
fn a_working_villager_is_drawn_on_every_tick() {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        return;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&units_path).expect("units");
    let mut world = World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seeding");
    world.load_resources(&resources).expect("stockpile");

    // A node the player already knows, with free ground beside it.
    let Some((node, at)) = world
        .entities()
        .iter()
        .filter(|e| matches!(e.kind, EntityKind::Resource(_)))
        .map(|e| (e.id, world.tile_of_position(e.position)))
        .find(|(_, (x, y))| world.is_free_ground(x.saturating_sub(2), *y) && (20..60).contains(x))
    else {
        return;
    };
    let scale = world.scale();
    let villager = world.role_unit("villager").expect("villager");
    let start = (i64::from(at.0) - 2, i64::from(at.1));
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_650),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(start.0 * scale + scale / 2),
                Fixed::from_raw(start.1 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("step");
    world
        .submit_checked(Command::Gather {
            villager: EntityId(9_650),
            node,
        })
        .expect("a gather order");

    let camera = Camera::new((640, 480)).centred_on((f32::from(at.0), f32::from(at.1)));
    let colour = palette::PLAYER_ONE;
    let mut absent = Vec::new();
    for tick in 0..120 {
        world.step().expect("a tick");
        let snapshot = Snapshot::of(&world, &map, 1);
        if snapshot.entities.iter().all(|e| e.id != 9_650) {
            absent.push((tick, 0usize, "not in the snapshot".to_owned()));
            continue;
        }
        let frame = render(&snapshot, &camera);
        let mut painted = 0usize;
        for py in 0..480u32 {
            for px in 0..640u32 {
                if frame.pixel(px, py) == colour {
                    painted += 1;
                }
            }
        }
        if painted == 0 {
            let pose = snapshot
                .entities
                .iter()
                .find(|e| e.id == 9_650)
                .map(|e| format!("{:?} {:?}", e.pose, e.position))
                .unwrap_or_default();
            absent.push((tick, painted, pose));
        }
    }
    assert!(
        absent.is_empty(),
        "the villager was not drawn on {} of 120 ticks: {:?}",
        absent.len(),
        &absent[..absent.len().min(6)]
    );
}
