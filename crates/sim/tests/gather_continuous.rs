//! **No cap: a place is a place if a villager's circle fits in it** (D188). The owner, after ten men were measured working one
//! bush in his own save:
//!
//! > *"I think that capping is pointless. Rule is rather simple: villager approaches designated resource to gather from, if there
//! > is free place of a full diameter, villager started working. If no, villager looks for another one for the same resource. If
//! > there is none, villager looks for another near source of the same kind."*
//!
//! And on the questions: another **square with the same resource** is what he meant by "another one"; the diameter is the **circle
//! under the villager's legs** - *"either villager fits in or don't"*; if the clear place is round the far side, *"take next tree
//! instead"*; and the ring he dropped as a *rule* - the geometry is the only limit.
//!
//! So: no slot table, no nine. A man takes the nearest point on the 1.156 ring that is one body-diameter clear of every other man,
//! and if the nearest such point is more than a tile and a half of walking away he takes the next source instead.
//!
//! Written before the implementation (D7, SPEC §11).

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

/// Two bushes five tiles apart on open grass, and a town centre to bank into.
const MAP: &str = r#"
schema = 1
[map]
width = 22
height = 16
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "food"
amount = 900
hp = 900
tiles = [[12,7]]
[[map.resource]]
kind = "food"
amount = 900
hp = 900
tiles = [[8,10]]
[[map.town_center]]
player = 1
x = 4
y = 13
[[map.town_center]]
player = 2
x = 19
y = 2
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
        ruage_data::load_map_str(map_text, "continuous.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn nodes(world: &World) -> Vec<EntityId> {
    world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .collect()
}

fn gap(world: &World, a: (Fixed, Fixed), b: (Fixed, Fixed)) -> f64 {
    let scale = world.scale() as f64;
    let dx = a.0.raw() as f64 / scale - b.0.raw() as f64 / scale;
    let dy = a.1.raw() as f64 / scale - b.1.raw() as f64 / scale;
    (dx * dx + dy * dy).sqrt()
}

/// **Ten men work one bush, a body apart.** The owner's own save shows exactly that, and the engine stopped at nine because a
/// table said nine. Nothing may be closer than a diameter (0.40) - that is the *one* rule.
#[test]
fn ten_men_work_one_bush_a_body_apart() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let bush = nodes(&world)[0];
    let villager = world.role_unit("villager").expect("villager");
    let mut ids = Vec::new();
    for index in 0..10u32 {
        let id = EntityId(8_000 + index);
        // All from the same side, which is how a player sends them.
        let x = 10 + i64::from(index % 4);
        let y = 4 + i64::from(index / 4);
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager),
                position: (
                    Fixed::from_raw(x * scale + scale / 2),
                    Fixed::from_raw(y * scale + scale / 2),
                ),
                speed: Fixed::parse("0.8", scale).expect("a speed"),
                hp: 25,
                amount: 0,
            })
            .expect("a villager");
        world.step().expect("the villager lands");
        world
            .submit_checked(Command::Gather {
                villager: id,
                node: bush,
            })
            .expect("a gather order");
        ids.push(id);
    }
    for _ in 0..1_600 {
        world.step().expect("a tick");
    }

    let working: Vec<EntityId> = ids
        .iter()
        .copied()
        .filter(|id| matches!(world.entity(*id).map(|e| e.task), Some(Task::Gather { node, .. }) if node == bush))
        .collect();
    assert_eq!(
        working.len(),
        10,
        "MEASURED {} of 10 men working the bush; ten fit at a body each on the ring, and the owner's save shows ten",
        working.len()
    );
    let spots: Vec<(Fixed, Fixed)> = working
        .iter()
        .filter_map(|id| world.position(*id))
        .collect();
    let mut closest = f64::MAX;
    for a in 0..spots.len() {
        for b in (a + 1)..spots.len() {
            closest = closest.min(gap(&world, spots[a], spots[b]));
        }
    }
    assert!(
        closest >= 0.39,
        "MEASURED the closest pair {closest:.3} tiles apart; a villager's circle is 0.40 wide and must fit"
    );
    // **Each circle touches the bush's tile** (D192): 0.20 of a tile from its border, which is 0.70-0.99 from the centre
    // depending on which part of the border he is against.
    let tile = world.resource_tiles(bush)[0];
    let clear = ruage_sim::tiles::distance_to_tile as fn((Fixed, Fixed), (i64, i64), i64) -> i64;
    for (index, spot) in spots.iter().enumerate() {
        let clearance = clear(*spot, (i64::from(tile.0), i64::from(tile.1)), world.scale());
        assert!(
            (world.scale() / 8..=world.scale() / 3).contains(&clearance),
            "MEASURED man {index} {:.3} tiles from the bush's border; his circle is 0.20 and must touch it",
            clearance as f64 / world.scale() as f64
        );
    }
}

/// **A man who cannot fit takes the next bush, and does not walk round the one he was sent to.** The owner: *"take next tree
/// instead"*. The nearest clear place on the far side would be two tiles of walking; the other bush is nearer than that.
#[test]
fn a_man_who_cannot_fit_takes_the_next_source() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    let scale = world.scale();
    let all = nodes(&world);
    let (bush, other) = (all[0], all[1]);
    let villager = world.role_unit("villager").expect("villager");
    // Fill the first bush completely, from one side only, so every place left is on the far side.
    let mut ids = Vec::new();
    for index in 0..24u32 {
        let id = EntityId(8_200 + index);
        let x = 10 + i64::from(index % 5);
        let y = 3 + i64::from(index / 5);
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager),
                position: (
                    Fixed::from_raw(x * scale + scale / 2),
                    Fixed::from_raw(y * scale + scale / 2),
                ),
                speed: Fixed::parse("0.8", scale).expect("a speed"),
                hp: 25,
                amount: 0,
            })
            .expect("a villager");
        world.step().expect("the villager lands");
        world
            .submit_checked(Command::Gather {
                villager: id,
                node: bush,
            })
            .expect("a gather order");
        ids.push(id);
    }
    for _ in 0..2_000 {
        world.step().expect("a tick");
    }

    let on_bush = ids
        .iter()
        .filter(|id| {
            matches!(world.entity(**id).map(|e| e.task), Some(Task::Gather { node, .. }) if node == bush)
        })
        .count();
    let on_other = ids
        .iter()
        .filter(|id| {
            matches!(world.entity(**id).map(|e| e.task), Some(Task::Gather { node, .. }) if node == other)
        })
        .count();
    let idle = ids
        .iter()
        .filter(|id| matches!(world.entity(**id).map(|e| e.task), Some(Task::Idle)))
        .count();
    // **Eleven places round one tile** (D192): the men who do not fit are sent to the other bush, which is what the owner asked
    // for - "try another place for the same tile, if that fails, look for another one within 4 tiles".
    let places = ruage_sim::touching_place_count(
        &world.resource_tiles(bush),
        world.scale() / 5,
        world.scale(),
    );
    // **No more than the geometry allows on the clicked tile, and the overflow on the other one**: the exact count moves with
    // men walking home with a load, so the claim is the bound plus "somebody went elsewhere".
    assert!(
        on_bush <= places,
        "MEASURED {on_bush} men gathering the bush that was clicked; the touching places round one tile are {places}"
    );
    let _ = idle;
    assert!(
        on_other > 0,
        "MEASURED {on_other} men took the neighbouring bush and {idle} stood idle; a man who cannot fit must take the next \
         source rather than walking round this one"
    );
}
