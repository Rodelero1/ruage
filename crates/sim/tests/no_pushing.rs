//! **Nobody is pushed at a resource** (D204). The owner, playing the D200 merge: *"I don't like villagers pushing
//! other villagers. I want instead to look for another free place at next tile with same resource instead of fighting
//! over the place at the same tile."*
//!
//! So displacement is not the answer to a contested work place. The fixture is the owner's own picture - a crowd of
//! bodies round one node, so that there is no square left for the next man - and the two things it pins are the two
//! halves of the rule:
//!
//! - the man who cannot get his square **takes the next node of the same column** rather than standing there, and
//! - **nobody in the crowd is moved off his square**, not even a twentieth of a tile, by the man who wanted it.
//!
//! Measured on `main` before the change: the thirteenth man ends `Task::Gather` on the crowded tree with
//! **progress 0** after 2,000 ticks while the tree of the same column five tiles east stands untouched, and one
//! member of the crowd is nudged **0.15** tiles off his place by `push_the_standing`.
//!
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// Two trees of one column: a **crowded** one at (7,6) - the crowd is bodies, not terrain - and a **free** one at
/// (12,6), five tiles east along the same row, with open ground on every side.
const MAP: &str = r#"
schema = 1
[map]
width = 20
height = 14
terrain = [
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGFGGGGFGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 100000
hp = 20
tiles = [[7,6]]
[[map.resource]]
kind = "wood"
amount = 100000
hp = 20
tiles = [[12,6]]
[[map.town_center]]
player = 1
x = 2
y = 12
[[map.town_center]]
player = 2
x = 18
y = 1
"#;

fn world_of(map_text: &str) -> Option<World> {
    let units_file = repo("config/gamedata/units.toml");
    if !units_file.exists() {
        return None;
    }
    let resources = common::present(
        &repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings: Settings = common::present(&repo("config/sim.toml"), ruage_data::load_sim_file);
    let map: Map = common::must(
        "load_map_str",
        ruage_data::load_map_str(map_text, "no_pushing.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

/// **The owner's picture**: every square of one tree's ring is taken by a standing body, and one more man is sent at
/// that tree. The crowd stands there with nothing to do, so nothing but a nudge can move them.
struct Crowd {
    world: World,
    crowded: EntityId,
    free: EntityId,
    newcomer: EntityId,
    /// Every body of the crowd, and the exact point it was put on.
    crowd: Vec<(EntityId, (Fixed, Fixed))>,
}

fn a_crowd_round_one_tree() -> Option<Crowd> {
    let mut world = world_of(MAP)?;
    let scale = world.scale();
    let crowded = world
        .entities()
        .iter()
        .find(|entity| world.resource_tiles(entity.id).first() == Some(&(7, 6)))
        .map(|entity| entity.id)?;
    let free = world
        .entities()
        .iter()
        .find(|entity| world.resource_tiles(entity.id).first() == Some(&(12, 6)))
        .map(|entity| entity.id)?;
    let villager = world.role_unit("villager").expect("villager");

    // The man who will be refused, two and a half tiles west of the crowded tree. He is placed first so that the
    // ring round the tree can be laid out at **his** body's spacing, which is what the settle itself asks for.
    let newcomer = EntityId(9_990);
    let spawn = |world: &mut World, id: EntityId, at: (Fixed, Fixed)| {
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager),
                position: at,
                speed: Fixed::parse("0.8", scale).expect("a speed"),
                hp: 25,
                amount: 0,
            })
            .expect("a villager on clear ground");
        world.step().expect("the villager lands");
    };
    spawn(
        &mut world,
        newcomer,
        (
            Fixed::from_raw(4 * scale + scale / 2),
            Fixed::from_raw(6 * scale + scale / 2),
        ),
    );
    let radius = world.body_radius_of(newcomer);
    let packed = (2 * radius).max(scale / 5);
    let places = ruage_sim::tiles::touching_places((7, 6), radius, packed, scale);
    assert!(
        places.len() >= 7,
        "PRECONDITION: one tree's packed ring holds {} places, which is not a crowd",
        places.len()
    );
    let mut crowd = Vec::new();
    for (index, place) in places.into_iter().enumerate() {
        let id = EntityId(9_900 + index as u32);
        spawn(&mut world, id, place);
        crowd.push((id, place));
    }
    Some(Crowd {
        world,
        crowded,
        free,
        newcomer,
        crowd,
    })
}

/// **A man who cannot get his square takes the next tree of the same column.** He is sent at a tree whose every
/// square is taken; the tree of the same column five tiles east has room. Standing beside the first one for the rest
/// of the match - which is what `main` does - is the defect D204 names.
#[test]
fn a_man_who_cannot_get_his_square_takes_the_next_tree() {
    let Some(mut crowd) = a_crowd_round_one_tree() else {
        return;
    };
    let scale = crowd.world.scale();
    crowd
        .world
        .submit_checked(Command::Gather {
            villager: crowd.newcomer,
            node: crowd.crowded,
        })
        .expect("a gather order");
    for _ in 0..2_000 {
        crowd.world.step().expect("a tick");
    }
    let entity = crowd
        .world
        .entity(crowd.newcomer)
        .expect("the newcomer is alive");
    let at = entity.position;
    let task = entity.task;
    let node = match task {
        Task::Gather { node, .. } => node,
        other => panic!(
            "MEASURED the man refused a square is not gathering at all after 2,000 ticks: {other:?} at ({:.3},{:.3})",
            at.0.raw() as f64 / scale as f64,
            at.1.raw() as f64 / scale as f64
        ),
    };
    assert_eq!(
        node,
        crowd.free,
        "MEASURED he is still working the tree with no square for him ({:?}) at ({:.3},{:.3}) - the free tree of the \
         same column is {:?}",
        crowd.crowded,
        at.0.raw() as f64 / scale as f64,
        at.1.raw() as f64 / scale as f64,
        crowd.free
    );
    let Task::Gather { progress, .. } = task else {
        unreachable!("just matched")
    };
    assert!(
        progress.raw() > 0,
        "MEASURED he was sent to the free tree but has taken nothing from it in 2,000 ticks"
    );
}

/// **Nobody in the crowd is moved off his square.** `push_the_standing` displaces a body a twentieth of a tile after
/// forty refused ticks; at a resource that is the owner's *"villagers pushing other villagers"*, and D204's answer is
/// that the man who cannot get a square looks at the next tile instead.
#[test]
fn a_crowd_at_a_tree_is_never_pushed() {
    let Some(mut crowd) = a_crowd_round_one_tree() else {
        return;
    };
    let scale = crowd.world.scale();
    crowd
        .world
        .submit_checked(Command::Gather {
            villager: crowd.newcomer,
            node: crowd.crowded,
        })
        .expect("a gather order");
    for _ in 0..2_000 {
        crowd.world.step().expect("a tick");
    }
    for (id, place) in &crowd.crowd {
        let now = crowd.world.position(*id).expect("a body of the crowd");
        let dx = (now.0.raw() - place.0.raw()) as f64 / scale as f64;
        let dy = (now.1.raw() - place.1.raw()) as f64 / scale as f64;
        let moved = (dx * dx + dy * dy).sqrt();
        assert!(
            moved == 0.0,
            "MEASURED body {id:?} was shoved {moved:.3} tiles off his square, from ({:.3},{:.3}) to ({:.3},{:.3})",
            place.0.raw() as f64 / scale as f64,
            place.1.raw() as f64 / scale as f64,
            now.0.raw() as f64 / scale as f64,
            now.1.raw() as f64 / scale as f64
        );
    }
}
