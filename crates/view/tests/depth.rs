//! Depth: a 2:1 diamond grid has a front (the bottom of the screen) and a back (the top), so what is painted later
//! must be what stands in front. Written before the implementation (D7, SPEC §11).
//!
//! The keys are derived from what the screen shows: a unit's depth is `x + y`, and a **building's is the far corner of
//! its footprint**, because a 4x4 town centre occupies ground up to `anchor + 3` and only the things beyond that are
//! in front of it.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};
use ruage_view::{Camera, EntityView, Pose, Snapshot, palette, render};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
    combat: ruage_data::Combat,
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
    })
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    world
}

fn entity(id: u32, kind: EntityKind, at: (f32, f32), footprint: (u32, u32)) -> EntityView {
    EntityView {
        id,
        player: 1,
        kind,
        unit: None,
        position: at,
        hp_fraction: 1.0,
        pose: Pose::Idle,
        carry: 0,
        construction: None,
        garrison: 0,
        amount: 0,
        pending: 0,
        remembered: false,
        footprint,
        shape: ruage_view::Silhouette::Square,
        mark: ruage_view::Mark::None,
    }
}

/// Back to front: a unit behind a building is painted first, a unit in front of it is painted last, and ties go to
/// the lower id so the order never depends on how the list was built.
#[test]
fn things_are_painted_from_the_back_of_the_diamond_to_the_front() {
    let building = entity(10, EntityKind::Building, (16.0, 16.0), (4, 4));
    // The anchor is the diamond's north corner (D84), so the footprint runs x,y = 16..19 and its far corner is (19,19) -
    // depth 38. Everything between 38 and the back of the diamond is painted before the building.
    let behind = entity(20, EntityKind::Unit, (14.0, 14.0), (1, 1));
    let level = entity(21, EntityKind::Unit, (17.0, 17.0), (1, 1));
    let in_front = entity(22, EntityKind::Unit, (18.0, 18.0), (1, 1));
    let lower_id_at_the_same_depth = entity(9, EntityKind::Unit, (17.0, 17.0), (1, 1));
    let mut snapshot = Snapshot {
        tick: 0,
        player: 1,
        width: 32,
        height: 32,
        terrain: vec![false; 32 * 32],
        stock: Vec::new(),
        population: 0,
        population_cap: 0,
        outcome: None,
        visible: vec![u64::MAX; 16],
        explored: vec![u64::MAX; 16],
        entities: vec![
            in_front.clone(),
            building.clone(),
            level.clone(),
            behind.clone(),
            lower_id_at_the_same_depth.clone(),
        ],
    };
    snapshot.entities.reverse();

    let order: Vec<u32> = ruage_view::draw_order(&snapshot)
        .iter()
        .map(|entity| entity.id)
        .collect();
    assert_eq!(
        order,
        vec![20, 9, 21, 22, 10],
        "expected back (20), then the depth-34 pair by id (9, 21), then the depth-36 unit (22), then the building whose \
         far corner is 38 - got the front of a diamond drawn over the back of it"
    );
}

/// The visible consequence: a unit standing inside a building's footprint but behind its front corner is painted
/// over by the building, and one in front of the footprint is not.
#[test]
fn a_unit_behind_a_building_is_covered_by_it() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let scale = world.scale();
    let town = world
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Building && entity.player == 1)
        .expect("the player's town centre")
        .id;
    let footprint = world
        .footprint_of(
            world
                .entity(town)
                .and_then(|entity| entity.unit)
                .expect("a town centre has a unit id"),
        )
        .expect("a footprint from the pack");
    // Inside the footprint but in its back half, and with an id higher than the town centre's, so an id-ordered draw
    // paints it on top: exactly the mistake depth sorting exists to prevent. The footprint runs 16..19 from the anchor,
    // so (17,17) is inside it and behind its far corner at (19,19).
    let behind = (17, 17);
    world
        .submit_checked(Command::Spawn {
            id: EntityId(9_001),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(world.role_unit("villager").expect("villager")),
            position: (
                Fixed::from_raw(behind.0 * scale),
                Fixed::from_raw(behind.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp: 25,
            amount: 0,
        })
        .expect("a villager in the back of the building");
    world.step().expect("step");

    let snapshot = Snapshot::of(&world, &fx.map, 1);
    let building = snapshot
        .entities
        .iter()
        .find(|entity| entity.id == town.0)
        .expect("the town centre");
    let camera = Camera::new((640, 480)).centred_on(building.position);
    let frame = render(&snapshot, &camera);

    // The villager is inside the diamond the building covers, so **the building's paint** is what a player sees there -
    // its block or its mark, never the unit. (The strict "exactly the building colour" version broke when buildings got
    // marks: a mark is part of the building's paint, and a pixel on a mark is still not a villager.)
    let (px, py) = camera.project((behind.0 as f32, behind.1 as f32));
    let seen = frame.pixel(px as u32, py as u32);
    assert_ne!(
        seen,
        palette::PLAYER_ONE,
        "a villager standing behind a {0}x{1} building was painted on top of it",
        footprint.0,
        footprint.1
    );
    assert!(
        seen == palette::BUILDING || seen == palette::MARK,
        "and what is seen there is the building's own paint: {seen:?}"
    );
}

/// The owner's report from `make run`: "the villagers upon approaching to the berry bush disappeared as if entering a
/// building". The simulation is right - `crates/sim/tests/neighbours.rs` proves the gatherer stands *beside* the node -
/// so it is the drawing: a bush one tile in front of a worker has a greater depth and used to paint over it. There are
/// no sprite heights in this renderer, so a low bush cannot be allowed to occlude a unit at all.
#[test]
fn a_worker_behind_a_bush_is_not_hidden_by_it() {
    let worker = entity(30, EntityKind::Unit, (10.0, 10.0), (1, 1));
    let bush = entity(31, EntityKind::Resource(0), (11.0, 11.0), (1, 1));
    let snapshot = Snapshot {
        tick: 0,
        player: 1,
        width: 32,
        height: 32,
        terrain: vec![false; 32 * 32],
        stock: Vec::new(),
        population: 0,
        population_cap: 0,
        outcome: None,
        visible: vec![u64::MAX; 16],
        explored: vec![u64::MAX; 16],
        entities: vec![worker.clone(), bush.clone()],
    };
    let order = ruage_view::draw_order(&snapshot);
    let worker_at = order
        .iter()
        .position(|e| e.id == 30)
        .expect("the worker is drawn");
    let bush_at = order
        .iter()
        .position(|e| e.id == 31)
        .expect("the bush is drawn");
    assert!(
        worker_at > bush_at,
        "the bush is nearer the camera, but a bush is not tall enough to hide the villager working it"
    );
}
