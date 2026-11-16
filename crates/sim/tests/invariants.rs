//! **Four body invariants the owner asked for by name** (D243, 2026-11-16), each one checked **every tick** of a working economy
//! rather than sampled: a sample catches a man mid-stride and calls it a violation, or misses the one tick that is wrong.
//!
//! The owner's four sentences, verbatim, and how each is measured:
//!
//! 1. *"If villagers approach each other closer than 1 diameter -> FAIL"* - the distance between two villagers' centres must stay
//!    at or above **one diameter** (their two radii), on every tick.
//! 2. *"If villager gathers resource more than 1/2 diameter away from the source -> FAIL"* - while a villager's task is
//!    `Gather`, the gap between his body and the node's tile must be at most **half a diameter** (his own radius), which is what
//!    "his body touches the tile" means for a square tile.
//! 3. *"If villager drops resource when further from the drop site than 1/2 diameter -> FAIL"* - on the tick a villager's load
//!    leaves him, he must be within **half a diameter** of the building he delivered to.
//! 4. *"If villager gathers from the resource, resource is not empty, villager is not at 100% carry capacity and changes position
//!    -> FAIL"* - a man who is at his node, with wood left in it and room in his hands, does not move: standing still while
//!    working is the whole of that sentence.
//!
//! Written before any of them is enforced (D7, SPEC §11): red here means the engine does not yet keep the invariant, which is a
//! finding, not a broken test.
//!
//! **A second set of four, asked for the same way (D250, 2026-11-16)**: *"Please add unit tests never to be skipped or
//! ignored"* - and **nothing in this file is `#[ignore]`d**, which is part of what was asked.
//!
//! 5. *"If villager or any other unit circle crosses tile (approaches closer than 1/2 diameter) with resource that has not
//!    been depleted yet (mine, bush, tree...) -> FAIL"* - the gap between a unit's centre and the edge of a **node's** tile
//!    is never less than the unit's own radius, for every unit and every node still holding something.
//! 6. *"... with building under the construction (except farm) or finished building (except farm and walkable part of town
//!    center) -> FAIL"* - the same gap against every tile a building's footprint **blocks**. The two exceptions are not
//!    listed in the test: they are the engine's own masks (`solid_tiles`), so a farm's floor and a town centre's yard have
//!    no blocked tile to cross. A foundation becomes solid on its **first stroke** (D183 §5, the owner's own model: *"As
//!    soon as the villager starts building, it will turn dark grey (except farm) and start blocking the tiles"*), which the
//!    fixture watches for rather than assumes.
//! 7. *"If villager constructs farm while not inside it (at least 1/2 diameter inside farm plot) -> FAIL"* - while a farm's
//!    construction is progressing, its builder's whole circle lies inside the plot.
//! 8. *"If technology that is not available at current age (for example Imperial Age in Dark Age) shows -> FAIL"* - the
//!    card is where a technology "shows", and Imperial Age is what the owner named: `card_age.rs`.

// **`let … else` is the clearer form in these fixtures** (D248): each one guards a `None` the check reports as "nothing
// to judge yet", and `?` in a closure returning `Option<String>` reads as an error path rather than a skip.
#![allow(clippy::question_mark)]

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, Task, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn pack() -> Option<ruage_data::ContentPack> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        eprintln!(
            "skipping: {} is not present (generated data, D8)",
            units.display()
        );
        return None;
    }
    Some(ruage_data::ContentPack::load(&repo("config")).expect("the pack"))
}

struct Scene {
    world: World,
    workers: Vec<EntityId>,
    drop: EntityId,
}

/// Four villagers put on the nearest wood to the town centre, each with a gather order, and the town centre as the drop site.
fn scene(pack: &ruage_data::ContentPack) -> Option<Scene> {
    let mut economy = pack.economy.clone();
    economy.start.civs = vec!["British".to_owned()];
    economy.start.stock = [("food".to_owned(), 2_000), ("wood".to_owned(), 2_000)]
        .into_iter()
        .collect();
    let mut world = World::new(&pack.sim, 7).expect("world");
    world
        .seed_from(&pack.map, &economy, &pack.units)
        .expect("seeding");
    world
        .load_resources(&pack.resources)
        .expect("the pack fits the stockpile");
    world
        .load_research(&pack.techs, &pack.effects)
        .expect("research tables");
    world.load_civs(&pack.civs).expect("the civ table");

    let drop = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .map(|entity| entity.id)?;
    let nodes: Vec<EntityId> = world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .take(4)
        .collect();

    let villager = world.role_unit("villager").expect("a villager role");
    let scale = world.scale();
    let mut workers = Vec::new();
    for (nth, node) in nodes.iter().enumerate() {
        let id = EntityId(9_000 + u32::try_from(nth).unwrap_or(0));
        let (tx, ty) = world.resource_tiles(*node)[0];
        // **A free tile beside the node**, found rather than assumed: the first draft put the man three tiles east of it and
        // the order was refused with `NoPath`, because the forest's own tiles are not walkable ground.
        let stand = (1u16..=3)
            .flat_map(|d| {
                [
                    (tx + d, ty),
                    (tx, ty + d),
                    (tx.saturating_sub(d), ty),
                    (tx, ty.saturating_sub(d)),
                ]
            })
            .find(|(x, y)| world.is_free_ground(*x, *y))
            .expect("a free tile beside the node");
        let be_at = (
            Fixed::from_raw(i64::from(stand.0) * scale + scale / 2),
            Fixed::from_raw(i64::from(stand.1) * scale + scale / 2),
        );
        world
            .submit_checked(Command::Spawn {
                id,
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(villager),
                position: be_at,
                speed: Fixed::parse("0.8", scale).expect("a speed"),
                hp: 25,
                amount: 0,
            })
            .expect("a villager");
        world.step().expect("the villager lands");
        world
            .submit_checked(Command::Gather {
                villager: id,
                node: *node,
            })
            .expect("a gather order");
        workers.push(id);
    }
    Some(Scene {
        world,
        workers,
        drop,
    })
}

/// The world-space rectangle a building occupies, as `(left, top, right, bottom)` in raw units.
fn building_rect(world: &World, id: EntityId) -> Option<(i64, i64, i64, i64)> {
    let entity = world.entity(id)?;
    let (width, height) = world.footprint_of(entity.unit?)?;
    let scale = world.scale();
    let left = entity.position.0.raw();
    let top = entity.position.1.raw();
    Some((
        left,
        top,
        left + i64::from(width) * scale,
        top + i64::from(height) * scale,
    ))
}

/// The gap between a body's centre and a rectangle, in raw units: zero when the centre is inside it.
fn gap_to_rect(point: (Fixed, Fixed), rect: (i64, i64, i64, i64)) -> i64 {
    let (x, y) = (point.0.raw(), point.1.raw());
    let (left, top, right, bottom) = rect;
    let dx = (left - x).max(x - right).max(0);
    let dy = (top - y).max(y - bottom).max(0);
    dx.max(dy)
}

/// The gap between a body's centre and a **tile**, in raw units: the tile is a square, so this is the Chebyshev gap less half a
/// tile, which is the distance from the centre to the tile's edge.
fn gap_to_tile(point: (Fixed, Fixed), tile: (u16, u16), scale: i64) -> i64 {
    let centre_x = i64::from(tile.0) * scale + scale / 2;
    let centre_y = i64::from(tile.1) * scale + scale / 2;
    let dx = (point.0.raw() - centre_x).abs();
    let dy = (point.1.raw() - centre_y).abs();
    dx.max(dy) - scale / 2
}

/// **One step of slack**, which every "at the wall" rule carries: the body is refused the step that would put its disc
/// inside the obstacle, so the closest it can come is one step short of touching (measured 4,155 raw against a 4,000
/// radius, a step being 758). Without it the bar is a fraction of a step that no walker can satisfy.
fn step_of(world: &World, id: EntityId) -> i64 {
    let speed = world.entity(id).map_or(Fixed::ZERO, |entity| entity.speed);
    let rate = i64::from(world.tick_rate_hz());
    if rate == 0 {
        return 0;
    }
    speed.raw() / rate
}

/// Run `check` on every tick, and fail with the first violation **and the tick it happened on** - a fixture that reports only
/// "a violation" leaves the next reader to find it again.
fn run(scene: &mut Scene, ticks: u32, mut check: impl FnMut(&mut Scene, u64) -> Option<String>) {
    for _ in 0..ticks {
        scene.world.step().expect("a tick");
        if let Some(why) = check(scene, scene.world.tick()) {
            panic!("{why}");
        }
    }
}

/// **1. Two villagers never come closer than one diameter.** Villager radius is a fifth of a tile, so the bar is 0.4 tiles
/// between centres.
#[test]
fn villagers_never_come_closer_than_one_diameter() {
    let Some(pack) = pack() else { return };
    let mut scene = scene(&pack).expect("a scene");
    let workers = scene.workers.clone();
    run(&mut scene, 2_000, |scene, tick| {
        for (nth, a) in workers.iter().enumerate() {
            for b in workers.iter().skip(nth + 1) {
                let (Some(pa), Some(pb)) = (scene.world.position(*a), scene.world.position(*b))
                else {
                    continue;
                };
                let apart = (((pa.0.raw() - pb.0.raw()).pow(2) + (pa.1.raw() - pb.1.raw()).pow(2))
                    as f64)
                    .sqrt();
                let need = (scene.world.body_radius_of(*a) + scene.world.body_radius_of(*b)) as f64;
                if apart < need - 1.0 {
                    return Some(format!(
                        "tick {tick}: villagers {:?} and {:?} stand {:.3} tiles apart, closer than the {:.3} their two \
                         diameters need",
                        a.0,
                        b.0,
                        apart / scene.world.scale() as f64,
                        need / scene.world.scale() as f64
                    ));
                }
            }
        }
        None
    });
}

/// **2. A gathering villager's body touches the node's tile.** "Half a diameter away" is his own radius from the tile's edge.
#[test]
fn a_gathering_villager_stands_within_half_a_diameter_of_the_node() {
    let Some(pack) = pack() else { return };
    let mut scene = scene(&pack).expect("a scene");
    let workers = scene.workers.clone();
    run(&mut scene, 2_000, |scene, tick| {
        let scale = scene.world.scale();
        for id in &workers {
            let Some(entity) = scene.world.entity(*id) else {
                continue;
            };
            let Task::Gather { node, progress, .. } = entity.task else {
                continue;
            };
            // **Only a man who is actually taking from the node.** A villager walking to his tree already carries the `Gather`
            // task, so a task test alone flags every step of the approach (measured on the first draft: a man 0.371 tiles out
            // was reported as "gathering"); `progress` is non-zero only once he is working at it.
            if progress.raw() <= 0 {
                continue;
            }
            let radius = scene.world.body_radius_of(*id);
            let point = entity.position;
            let tile = scene.world.resource_tiles(node)[0];
            let gap = gap_to_tile(point, tile, scale);
            if gap > radius + step_of(&scene.world, *id) {
                return Some(format!(
                    "tick {tick}: villager {:?} is gathering from node {:?} with a gap of {:.3} tiles between his body and \
                     the tile, more than the {:.3} half-diameter allowed",
                    id.0,
                    node.0,
                    gap as f64 / scale as f64,
                    radius as f64 / scale as f64
                ));
            }
        }
        None
    });
}

/// **3. A load leaves a man only while he is at the drop site.** Half a diameter from the building's own footprint, plus the
/// one step a discretised walker stops short by (`step_of`), which is the bar D248's fix met.
/// **Was `#[ignore]`d and red**: measured failing at 0.281 tiles on tick 792, because the delivery range was the pair's *reach*
/// (0.4) rather than the body's radius. Kept, then met by D248's `radius + one step` delivery test and the aim at
/// `World::wall_point` - so this fixture is a **guard** now, not a gap.
#[test]
fn a_villager_drops_only_within_half_a_diameter_of_the_drop_site() {
    let Some(pack) = pack() else { return };
    let mut scene = scene(&pack).expect("a scene");
    let workers = scene.workers.clone();
    let drop = scene.drop;
    let mut carried: Vec<i64> = workers
        .iter()
        .map(|id| scene.world.carry(*id).raw())
        .collect();
    run(&mut scene, 2_000, |scene, tick| {
        let scale = scene.world.scale();
        let rect = building_rect(&scene.world, drop)?;
        for (nth, id) in workers.iter().enumerate() {
            let now = scene.world.carry(*id).raw();
            let before = carried[nth];
            carried[nth] = now;
            if before > 0 && now == 0 {
                let Some(point) = scene.world.position(*id) else {
                    continue;
                };
                let gap = gap_to_rect(point, rect);
                let radius = scene.world.body_radius_of(*id);
                if gap > radius + step_of(&scene.world, *id) {
                    return Some(format!(
                        "tick {tick}: villager {:?} delivered {:.2} of a load with his body {:.3} tiles from the drop \
                         building - past the {:.3} half-diameter, even allowing the one step a walker stops short by",
                        id.0,
                        before as f64 / scale as f64,
                        gap as f64 / scale as f64,
                        radius as f64 / scale as f64
                    ));
                }
            }
        }
        None
    });
}

/// **4. A man who is working does not move.** At his node, wood still in it, room in his hands: his position must be the same
/// on the next tick as it was on this one.
/// **Was `#[ignore]`d and red**: measured failing at 0.011 tiles on tick 11 - the seat settle nudging a man onto his square
/// while he was already within reach of the tree, which is exactly what *"he gathers ... and changes position"* forbids. Kept
/// by D248's freeze of a working man's place and by not stepping a man who is on his own place; a **guard** now, not a gap.
#[test]
fn a_villager_who_is_gathering_does_not_move() {
    let Some(pack) = pack() else { return };
    let mut scene = scene(&pack).expect("a scene");
    let workers = scene.workers.clone();
    let mut was_at: Vec<Option<(Fixed, Fixed)>> = vec![None; workers.len()];
    run(&mut scene, 2_000, |scene, tick| {
        let scale = scene.world.scale();
        for (nth, id) in workers.iter().enumerate() {
            let Some(entity) = scene.world.entity(*id) else {
                continue;
            };
            let here = entity.position;
            let previous = was_at[nth].replace(here);
            let Task::Gather {
                node, carry_cap, ..
            } = entity.task
            else {
                continue;
            };
            let Some(node_entity) = scene.world.entity(node) else {
                continue;
            };
            let carry = scene.world.carry(*id).raw();
            let full = carry >= i64::from(carry_cap) * scale;
            let radius = scene.world.body_radius_of(*id);
            let tile = scene.world.resource_tiles(node)[0];
            let at_the_node = gap_to_tile(here, tile, scale) <= radius;
            if let Some(previous) = previous
                && at_the_node
                && node_entity.amount.raw() > 0
                && !full
                && previous != here
            {
                return Some(format!(
                    "tick {tick}: villager {:?} moved {:.3} tiles while gathering from node {:?}, which still holds {:.1} \
                     and his hands hold {:.1} of {carry_cap}",
                    id.0,
                    (((here.0.raw() - previous.0.raw()).pow(2)
                        + (here.1.raw() - previous.1.raw()).pow(2)) as f64)
                        .sqrt()
                        / scale as f64,
                    node.0,
                    node_entity.amount.raw() as f64 / scale as f64,
                    carry as f64 / scale as f64
                ));
            }
        }
        None
    });
}

/// The **Euclidean** gap between a body's centre and a square tile, in raw units: zero when the centre is inside it.
///
/// `gap_to_tile` is the Chebyshev version, which is what the gather standoff is measured in; a circle against a square is
/// the Euclidean question, and at a tile's corner the two disagree by up to a radius. Integer throughout (`isqrt`), so no
/// float enters a fixture that judges a fixed-point tick.
fn gap_to_square(point: (Fixed, Fixed), tile: (u16, u16), scale: i64) -> i64 {
    let (x, y) = (point.0.raw(), point.1.raw());
    let left = i64::from(tile.0) * scale;
    let top = i64::from(tile.1) * scale;
    let gx = (left - x).max(x - (left + scale)).max(0);
    let gy = (top - y).max(y - (top + scale)).max(0);
    (gx * gx + gy * gy).isqrt()
}

/// The first free `wide` x `tall` patch of ground, as its anchor tile: the fixture's own placement search, so a test never
/// builds on a forest because it guessed a coordinate.
fn free_plot(world: &World, wide: u16, tall: u16) -> (u16, u16) {
    for y in 1..60u16 {
        for x in 1..60u16 {
            let fits = (0..tall).all(|dy| (0..wide).all(|dx| world.is_free_ground(x + dx, y + dy)));
            if fits {
                return (x, y);
            }
        }
    }
    panic!("the fixture's map has no free {wide}x{tall} plot");
}

/// Spawn one villager on the first free tile beside `at`, and land him: the fixture's builder, who has to walk in.
fn spawn_builder(world: &mut World, id: EntityId, at: (u16, u16)) -> bool {
    let Some(villager) = world.role_unit("villager") else {
        return false;
    };
    let scale = world.scale();
    let Some(stand) = (1u16..=4)
        .flat_map(|d| {
            [
                (at.0 + d, at.1),
                (at.0, at.1 + d),
                (at.0.saturating_sub(d), at.1),
                (at.0, at.1.saturating_sub(d)),
            ]
        })
        .find(|(x, y)| world.is_free_ground(*x, *y))
    else {
        return false;
    };
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(i64::from(stand.0) * scale + scale / 2),
                Fixed::from_raw(i64::from(stand.1) * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a builder");
    world.step().expect("the builder lands");
    true
}

/// **5. No unit's circle crosses an undepleted resource tile.** Every tick of a working economy, for every unit on the map
/// and every node that still holds something: the distance from the unit's centre to the tile is at least its own radius.
///
/// One raw unit of slack, and it is the integer tick's: at 20,000 units to a tile a step is 800, so a body that "touches"
/// a tile is 1 raw unit from it in the arithmetic, while a circle *inside* a tile is hundreds.
#[test]
fn no_unit_crosses_an_undepleted_resource_tile() {
    let Some(pack) = pack() else { return };
    let mut scene = scene(&pack).expect("a scene");
    let nodes: Vec<EntityId> = scene
        .world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .collect();
    run(&mut scene, 2_000, |scene, tick| {
        let scale = scene.world.scale();
        for unit in scene
            .world
            .entities()
            .iter()
            .filter(|entity| entity.kind == EntityKind::Unit && entity.hp > 0)
        {
            let radius = scene.world.body_radius_of(unit.id);
            for node in &nodes {
                // **Not depleted** is the owner's own qualifier: an empty node frees its tile, and standing there is
                // then ordinary ground rather than a crossed tile.
                let Some(node_entity) = scene.world.entity(*node) else {
                    continue;
                };
                if node_entity.amount.raw() <= 0 {
                    continue;
                }
                for tile in scene.world.resource_tiles(*node) {
                    let gap = gap_to_square(unit.position, tile, scale);
                    if gap < radius - 1 {
                        return Some(format!(
                            "tick {tick}: unit {:?} stands {:.4} tiles from the edge of tile {:?}, which holds a \
                             resource with {:.1} left in it - the circle crosses the tile by {:.4} of a tile",
                            unit.id.0,
                            gap as f64 / scale as f64,
                            tile,
                            node_entity.amount.raw() as f64 / scale as f64,
                            (radius - gap) as f64 / scale as f64
                        ));
                    }
                }
            }
        }
        None
    });
}

/// **6. No unit's circle crosses a building - finished or under construction - except a farm's floor and a town centre's
/// yard.**
///
/// The exceptions are the engine's own masks rather than a list in the test: a footprint tile that `tile_walkable` says
/// blocks is a tile a body may not cross, and a farm (all walkable) and a town centre's yard simply have none. A house
/// raised in this fixture is the "under construction" half - and the fixture **checks that it became solid**, because a
/// site that never started would leave this test asserting the town centre alone (D116).
#[test]
fn no_unit_crosses_a_building_either_finished_or_under_construction() {
    let Some(pack) = pack() else { return };
    let mut scene = scene(&pack).expect("a scene");
    let house = scene.world.role_unit("house").expect("a house role");
    let (wide, tall) = scene.world.footprint_of(house).expect("a house footprint");
    let (wide, tall) = (
        u16::try_from(wide).expect("a width"),
        u16::try_from(tall).expect("a height"),
    );
    // **A plot with a free ring round it**, so a body can stand at its edge - the case the tile-only site-clear gate
    // missed.
    let anchor = free_plot(&scene.world, wide + 2, tall + 2);
    let at = (anchor.0 + 1, anchor.1 + 1);
    let builder = EntityId(9_100);
    assert!(
        spawn_builder(&mut scene.world, builder, at),
        "a builder beside the plot"
    );
    // **An idle bystander whose circle is across the footprint's edge**: his tile is outside the site, so the gate that
    // asks "is anybody standing on these tiles" let the first stroke land on him. The wall must wait for his body, not
    // for his tile.
    let scale = scene.world.scale();
    let bystander = EntityId(9_101);
    let villager = scene.world.role_unit("villager").expect("a villager role");
    let edge = (
        Fixed::from_raw(i64::from(at.0) * scale - scale / 10),
        Fixed::from_raw(i64::from(at.1 + tall / 2) * scale + scale / 2),
    );
    assert!(
        scene.world.is_free_ground(at.0 - 1, at.1 + tall / 2),
        "the fixture's plot has no free ground at its left edge"
    );
    scene
        .world
        .submit_checked(Command::Spawn {
            id: bystander,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: edge,
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a bystander at the edge");
    scene.world.step().expect("the bystander lands");
    scene
        .world
        .submit_checked(Command::Build {
            villager: builder,
            unit: house,
            at,
        })
        .expect("a house order");
    let mut solid_seen = false;
    run(&mut scene, 2_000, |scene, tick| {
        let scale = scene.world.scale();
        // **Progress, not existence**: `raise_site` gives the foundation a `Construction` the moment the order lands
        // (`lib.rs:2757`), so `construction.is_some()` is true before a single tile blocks - a guard built on it would pass
        // for a site that never started, which is the failure it exists to catch (D116; found by the adversarial review).
        let site = scene.world.entities().iter().find(|entity| {
            entity.kind == EntityKind::Building && entity.position == scene.world.tile(at)
        });
        if site.is_some_and(|entity| {
            entity
                .construction
                .is_some_and(|construction| construction.progress_ticks > 0)
        }) {
            solid_seen = true;
        }
        for unit in scene
            .world
            .entities()
            .iter()
            .filter(|entity| entity.kind == EntityKind::Unit && entity.hp > 0)
        {
            let radius = scene.world.body_radius_of(unit.id);
            for building in scene
                .world
                .entities()
                .iter()
                .filter(|entity| entity.kind == EntityKind::Building && entity.hp > 0)
            {
                let Some(unit_id) = building.unit else {
                    continue;
                };
                let Some((wide, tall)) = scene.world.footprint_of(unit_id) else {
                    continue;
                };
                let anchor = scene.world.tile_of_position(building.position);
                for dy in 0..u16::try_from(tall).unwrap_or(1) {
                    for dx in 0..u16::try_from(wide).unwrap_or(1) {
                        let (Some(tx), Some(ty)) =
                            (anchor.0.checked_add(dx), anchor.1.checked_add(dy))
                        else {
                            continue;
                        };
                        // **A tile that does not block is not something to cross**: a farm's floor, a town centre's yard,
                        // and a foundation nobody has started (D183's ghost, the owner's own model).
                        if scene.world.tile_walkable(tx, ty) {
                            continue;
                        }
                        let gap = gap_to_square(unit.position, (tx, ty), scale);
                        if gap < radius - 1 {
                            return Some(format!(
                                "tick {tick}: unit {:?} stands {:.4} tiles from the edge of tile {:?} of building \
                                 {:?} (unit {unit_id}) - the circle crosses it by {:.4} of a tile",
                                unit.id.0,
                                gap as f64 / scale as f64,
                                (tx, ty),
                                building.id.0,
                                (radius - gap) as f64 / scale as f64
                            ));
                        }
                    }
                }
            }
        }
        None
    });
    assert!(
        solid_seen,
        "the fixture never had a building under construction: a site that never started tests nothing"
    );
}

/// **7. A farm is constructed from inside its plot.**
///
/// The owner's rule, and the engine's one deliberate exception to "a builder works from outside the site": a farm is a
/// floor, its site-clear gate does not apply (D183 §7), so there is no ground to hold up and nothing to stand outside of.
/// "At least 1/2 diameter inside" is read as the circle lying wholly inside the plot - the centre at least `radius` from
/// every edge - and it is checked on every tick the plot is making progress, not when it finishes.
#[test]
fn a_villager_constructs_a_farm_from_inside_its_plot() {
    let Some(pack) = pack() else { return };
    let mut scene = scene(&pack).expect("a scene");
    let farm = scene.world.role_unit("farm").expect("a farm role");
    let (wide, tall) = scene.world.footprint_of(farm).expect("a farm footprint");
    let (wide, tall) = (
        u16::try_from(wide).expect("a width"),
        u16::try_from(tall).expect("a height"),
    );
    let at = free_plot(&scene.world, wide, tall);
    let builder = EntityId(9_200);
    assert!(
        spawn_builder(&mut scene.world, builder, at),
        "a builder beside the plot"
    );
    scene
        .world
        .submit_checked(Command::Build {
            villager: builder,
            unit: farm,
            at,
        })
        .expect("a farm order");
    let mut worked = 0u32;
    run(&mut scene, 2_000, |scene, tick| {
        let scale = scene.world.scale();
        let Some(plot) =
            scene.world.entities().iter().find(|entity| {
                entity.unit == Some(farm) && entity.position == scene.world.tile(at)
            })
        else {
            return None;
        };
        let Some(construction) = plot.construction else {
            return None;
        };
        if construction.progress_ticks == 0 {
            // Still walking in: the rule is about constructing, and the walk there crosses the plot's edge.
            return None;
        }
        worked = construction.progress_ticks;
        let Some(position) = scene.world.position(builder) else {
            return None;
        };
        let radius = scene.world.body_radius_of(builder);
        // The plot's own rectangle: its anchor is a tile corner, so each edge is a whole number of tiles from it.
        let (left, top) = (i64::from(at.0) * scale, i64::from(at.1) * scale);
        let (right, bottom) = (
            left + i64::from(wide) * scale,
            top + i64::from(tall) * scale,
        );
        let inset = (position.0.raw() - left)
            .min(right - position.0.raw())
            .min(position.1.raw() - top)
            .min(bottom - position.1.raw());
        if inset < radius {
            return Some(format!(
                "tick {tick}: villager {:?} is constructing a farm with only {:.4} of a tile between his centre and \
                 the plot's nearest edge - his circle is not inside the plot, which needs {:.4}",
                builder.0,
                inset as f64 / scale as f64,
                radius as f64 / scale as f64
            ));
        }
        None
    });
    assert!(
        worked > 0,
        "the farm never made progress, so the rule was never measured"
    );
}
