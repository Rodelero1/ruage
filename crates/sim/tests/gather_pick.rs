//! **Who picks the tree, and what it costs** (D138).
//!
//! The script's `do = "gather"` action asks the engine which node to work. The engine already had an answer to "which
//! node" - `World::nodes_of_slot`: distance order, **no reachability test** (a path search per candidate cost 37x the
//! whole run, measured 2026-10-29), failures remembered in `share_failed` and retried after `share_retry_ticks`. The
//! script had its own copy that did the expensive thing, and this file holds the pair to one rule.
//!
//! Written before the change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Sim as Settings, Units};
use ruage_fixed::Fixed;
use ruage_script::{Runner, load_scenario_str};
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// **Two woods, one walled off.** The near block (x 13-15) is across a palisade line at x = 11; the far block (x 5-6)
/// is on the villager's side of it. The near block is nearer to the town centre, so a picker that asks only "how far?"
/// takes it - and a picker that asks "can I walk there?" for every candidate pays a search per tile to find that out.
const MAP: &str = r#"
schema = 1
[map]
width = 20
height = 14
terrain = [
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGFFGGGGGGGGGGGGG",
  "GGGGGFFGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGFFFGGGG",
  "GGGGGGGGGGGGGFFFGGGG",
  "GGGGGGGGGGGGGFFFGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGG",
]
[[map.resource]]
kind = "wood"
amount = 100
hp = 0
tiles = [[13,6],[14,6],[15,6],[13,7],[14,7],[15,7],[13,8],[14,8],[15,8]]
[[map.resource]]
kind = "wood"
amount = 100
hp = 0
tiles = [[5,1],[6,1],[5,2],[6,2]]
[[map.town_center]]
player = 1
x = 9
y = 7
[[map.town_center]]
player = 2
x = 1
y = 12
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
        ruage_data::load_map_str(map_text, "picks.toml", &resources),
    );
    let economy: Economy =
        common::present(&repo("config/economy.toml"), ruage_data::load_economy_file);
    let units: Units = common::must("load_units_file", ruage_data::load_units_file(&units_file));
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

/// A palisade segment, one tile, placed by hand: the wall is a fixture, not something under test.
fn segment(world: &mut World, id: u32, x: u16, y: u16) {
    let scale = world.scale();
    let unit = world.role_unit("palisade_wall").expect("a palisade role");
    world
        .submit_checked(Command::Spawn {
            id: EntityId(id),
            player: 1,
            kind: EntityKind::Building,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(x) * scale),
                Fixed::from_raw(i64::from(y) * scale),
            ),
            speed: Fixed::ZERO,
            hp: 9_900,
            amount: 0,
        })
        .expect("a segment can be placed");
}

/// The rule, written the way a pack writes one: a question about the world, and one order.
const RULE: &str = r#"
schema = 2
duration_seconds = 30

[[rule]]
player = 1
cooldown_seconds = 1.0
do = "gather"
resource = "wood"
"#;

/// **The picker asks nothing it does not have to** (D138).
///
/// With the near block walled off, three things have to be true at once, and they are what this test is for:
///
/// 1. the picker **takes the near block** - it asks "how far", not "can I walk there", so the refusal that follows is
///    the engine's own (`start_gather` is the authority on whether a node can be worked);
/// 2. the villager **ends up chopping the far block**, so a refusal is not a dead end - the node is remembered and the
///    next firing moves on;
/// 3. the refusals stay **bounded** by the number of walled nodes (nine), not by the number of firings (thirty), so the
///    memory is doing its job rather than the rule retrying the same tree for the whole match.
#[test]
fn a_walled_off_wood_is_tried_once_and_then_left_alone() {
    let Some(mut world) = world_of(MAP) else {
        return;
    };
    for y in 0..14u16 {
        segment(&mut world, 9_000 + u32::from(y), 11, y);
    }
    world.step().expect("the segments land");

    // What the far block holds **before**, so the assertion is a change rather than a number I guessed: node amounts are
    // in the engine's raw units, not in "wood".
    let far_before = far_wood(&world);

    let scenario = load_scenario_str(RULE, "<picks>").expect("the scenario parses");
    let mut runner = Runner::new(&scenario, world.tick_rate_hz()).expect("a runnable scenario");
    let ticks = u64::try_from((scenario.duration_seconds * f64::from(world.tick_rate_hz())) as i64)
        .expect("a length in ticks");
    for _ in 0..ticks {
        runner.due(&mut world);
        world.step().expect("step");
    }

    // 1. The picker did not pay for a reachability test: the walled node was chosen, and the engine said no.
    assert!(
        !runner.rule_refusals.is_empty(),
        "the picker must take the nearest node and let the engine refuse it; it refused nothing, so it pre-filtered"
    );

    // 2. A refusal is not a dead end: the far block was **worked**, so the picker moved on after the walled one was
    //    remembered. Read from the nodes rather than from the villager's task, because 30 s is long enough for it to be
    //    carrying a load home at the moment the loop ends.
    let far_after = far_wood(&world);
    assert!(
        far_after < far_before,
        "the far block still holds all {far_after} raw units (it held {far_before}): the remembered failure did not move \
         the picker on"
    );

    // 3. Bounded by the walled nodes, not by the firings.
    assert!(
        runner.rule_refusals.len() <= 12,
        "the rule was refused {} times in 30 s for nine walled nodes: something is retrying the same node",
        runner.rule_refusals.len()
    );
}

/// How much wood the **far** block (x <= 6, the villager's side of the wall) still holds, in the engine's raw units.
fn far_wood(world: &World) -> i64 {
    let scale = world.scale();
    let slot = world.slot_of("wood").expect("a wood slot");
    world
        .entities()
        .iter()
        .filter(|entity| entity.kind == EntityKind::Resource(slot))
        .filter(|entity| entity.position.0.raw().div_euclid(scale) <= 6)
        .map(|entity| entity.amount.raw())
        .sum()
}

/// **A honeycomb of single trees** - twenty-four of them, four tiles apart, every one with its whole ring on open ground.
///
/// A wood laid out this way is the expensive shape for a picker: no tree's ring is blocked, so a picker that asks
/// "can a body stand beside this?" with a path search asks it eight times per tree. The stump at (5,7) holds one unit, so
/// the villager spends it in his first bite and the re-task fires.
const HONEYCOMB: &str = r#"
schema = 1
[map]
width = 24
height = 16
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGFFGGGGFGGGGFGGGGFGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGFFGGGGFGGGGFGGGGFGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGFFGGGGFGGGGFGGGGFGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGFFGGGGFGGGGFGGGGFGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
]
# The stump: one unit of wood left, in a tile of its own just east of the villager, so the villager spends it in one
# bite and the re-task fires inside this fixture's window.
[[map.resource]]
kind = "wood"
amount = 2
hp = 0
tiles = [[5,7]]
# The rest of the wood, in the same resource so the picker counts them as the same kind.
[[map.resource]]
kind = "wood"
amount = 100
hp = 0
tiles = [[6,4],[11,4],[16,4],[21,4],[6,7],[11,7],[16,7],[21,7],[6,10],[11,10],[16,10],[21,10],[6,13],[11,13],[16,13],[21,13]]
[[map.town_center]]
player = 1
x = 3
y = 7
[[map.town_center]]
player = 2
x = 1
y = 14
"#;

/// The one entity of a kind whose tile is `(x, y)` - a fixture names its actors by where they stand, not by an id the
/// map loader happens to assign.
fn at(
    world: &World,
    kind_matches: impl Fn(&ruage_sim::Entity) -> bool,
    x: u16,
    y: u16,
) -> EntityId {
    let scale = world.scale();
    world
        .entities()
        .iter()
        .find(|entity| {
            kind_matches(entity)
                && entity.position.0.raw().div_euclid(scale) == i64::from(x)
                && entity.position.1.raw().div_euclid(scale) == i64::from(y)
        })
        .map(|entity| entity.id)
        .expect("the fixture's entity is on the map")
}

/// A map whose only feature is its wood: `trees` are forest tiles, everything else is grass. The village sits at (2, 10),
/// far enough from the wood that the picker has to think, close enough that all of it is in reach.
fn forest_map(trees: &[(u16, u16)]) -> String {
    let (width, height) = (30usize, 22usize);
    let mut rows = vec![vec!['G'; width]; height];
    for (x, y) in trees {
        rows[usize::from(*y)][usize::from(*x)] = 'F';
    }
    let terrain: String = rows
        .iter()
        .map(|row| format!("  \"{}\",\n", row.iter().collect::<String>()))
        .collect();
    let tiles: Vec<String> = trees.iter().map(|(x, y)| format!("[{x},{y}]")).collect();
    format!(
        "schema = 1\n[map]\nwidth = {width}\nheight = {height}\nterrain = [\n{terrain}]\n\
         [[map.resource]]\nkind = \"wood\"\namount = 100\nhp = 0\ntiles = [{}]\n\
         [[map.town_center]]\nplayer = 1\nx = 2\ny = 10\n\
         [[map.town_center]]\nplayer = 2\nx = 1\ny = 20\n",
        tiles.join(",")
    )
}

/// **What one pick costs, in routes** - the plans `nearest_like` spends naming the next source for a villager standing at
/// `old` on a map of `trees`.
fn pick_cost(trees: &[(u16, u16)]) -> Option<u64> {
    let world = world_of(&forest_map(trees))?;
    let wood = world.slot_of("wood").expect("a wood slot");
    let old = world
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Resource(wood))
        .map(|entity| entity.id)
        .expect("the forest is on the map");
    let role = world.role_unit("villager").expect("a villager role");
    let villager = world
        .entities()
        .iter()
        .find(|entity| {
            entity.kind == EntityKind::Unit
                && entity.player == 1
                && entity.unit.is_some_and(|unit| world.is_family(unit, role))
        })
        .map(|entity| entity.id)
        .expect("the village has a villager");
    let from = world
        .position(villager)
        .expect("the villager stands somewhere");
    let before = world.routes_planned();
    let next = world.next_source_near(from, old, villager);
    let spent = world.routes_planned() - before;
    assert!(
        next.is_some(),
        "nothing was picked at all on a map of {} trees: the measurement is empty",
        trees.len()
    );
    eprintln!("DEBUG pick over {} trees cost {spent}", trees.len());
    Some(spent)
}

/// **The picker's cost is set by the candidates it may choose, not by the map** (D280).
///
/// `nearest_like` is asked what is next every time a node runs out, and before this rule it answered by asking a **path
/// question** - "can a body stand beside this?" - of *every* resource on the map, before it had sorted them by distance:
/// **114 of those per pick** on the acceptance map, and a pick cost **13 ms** of the tick. Measured on this pair, the
/// same question on the same village with nine times the wood:
///
/// * `few` (6 trees) and `many` (54 trees, a grid at the same spacing) - the difference is what the map size is worth.
///
/// The rule: what a pick may pay is bounded by the **eight** candidates `truncate(8)` leaves it - one place and one route
/// each - so nine times the wood must not cost nine times the answer.
#[test]
fn nine_times_the_wood_does_not_cost_nine_times_the_pick() {
    let few_trees: Vec<(u16, u16)> = [(6, 10), (8, 10), (10, 10), (6, 8), (8, 8), (10, 8)]
        .into_iter()
        .collect();
    let many_trees: Vec<(u16, u16)> = (0..11)
        .flat_map(|column| (0..6).map(move |row| (6 + 2 * column, 6 + 2 * row)))
        .collect();
    let (Some(few), Some(many)) = (pick_cost(&few_trees), pick_cost(&many_trees)) else {
        return;
    };
    assert!(
        many <= few + 8,
        "a pick over 54 trees cost {many} routes against {few} over 6: the picker is pricing the map, not the eight \
         candidates it can choose between"
    );
}

/// **And a tree that runs out still sends the crew on.** The companion to the bound above: a pick that costs nothing is
/// worth nothing. Three villagers are sent at a stump holding two tiles' worth of wood; when it goes, they must end up
/// gathering or carrying rather than standing at the place it used to be.
#[test]
fn a_tree_that_runs_out_sends_the_crew_on() {
    let Some(mut world) = world_of(HONEYCOMB) else {
        return;
    };
    let wood = world.slot_of("wood").expect("a wood slot");
    let stump = at(
        &world,
        |entity| entity.kind == EntityKind::Resource(wood),
        5,
        7,
    );
    let role = world.role_unit("villager").expect("a villager role");
    let crew: Vec<EntityId> = world
        .entities()
        .iter()
        .filter(|entity| {
            entity.kind == EntityKind::Unit
                && entity.player == 1
                && entity.unit.is_some_and(|unit| world.is_family(unit, role))
        })
        .take(3)
        .map(|entity| entity.id)
        .collect();
    assert!(crew.len() >= 2, "the fixture needs a crew, not one man");
    for villager in &crew {
        world
            .submit_checked(Command::Gather {
                villager: *villager,
                node: stump,
            })
            .expect("a villager can be sent at a tree");
    }
    for _ in 0..600 {
        world.step().expect("stepping");
    }

    let working = world
        .entities()
        .iter()
        .filter(|entity| crew.contains(&entity.id))
        .filter(|entity| {
            matches!(
                entity.task,
                ruage_sim::Task::Gather { .. } | ruage_sim::Task::Return { .. }
            )
        })
        .count();
    assert!(
        working > 0,
        "not one of the crew is gathering or carrying any more: the stump emptied and left them standing"
    );
}

/// **A source with nothing left in it is not a source** (D280).
///
/// The acceptance match spent two hundred game-seconds with four villagers bouncing between two exhausted berry bushes,
/// re-tasked one to the other **every tick** - 7,369 re-tasks by tick 14,400, 1,630 of them refused - because
/// `nearest_like` matched on the column and never asked whether there was anything left in the node. Each bounce planned a
/// route: `retask_workers` was **59.8 s** of a 155 s run on its own.
///
/// Two drained trees sit nearer to the villager than any living one, so a picker that does not ask the question answers a
/// stump.
#[test]
fn the_picker_never_answers_a_source_with_nothing_left_in_it() {
    let Some(mut world) = world_of(HONEYCOMB) else {
        return;
    };
    let wood = world.slot_of("wood").expect("a wood slot");
    let scale = world.scale();
    let stump = at(
        &world,
        |entity| entity.kind == EntityKind::Resource(wood),
        5,
        7,
    );
    let role = world.role_unit("villager").expect("a villager role");
    let villager = world
        .entities()
        .iter()
        .find(|entity| {
            entity.kind == EntityKind::Unit
                && entity.player == 1
                && entity.unit.is_some_and(|unit| world.is_family(unit, role))
        })
        .map(|entity| entity.id)
        .expect("the village has a villager");
    // Two drained trees, nearer to him than the wood in the map.
    for (id, (x, y)) in [(9_001u32, (4u16, 6u16)), (9_002, (4, 8))] {
        world
            .submit_checked(Command::Spawn {
                id: EntityId(id),
                player: 0,
                kind: EntityKind::Resource(wood),
                unit: None,
                position: (
                    Fixed::from_raw(i64::from(x) * scale),
                    Fixed::from_raw(i64::from(y) * scale),
                ),
                speed: Fixed::ZERO,
                hp: 0,
                amount: 0,
            })
            .expect("a drained tree can be placed");
    }
    world.step().expect("the trees land");
    let from = world
        .position(villager)
        .expect("the villager stands somewhere");

    let next = world
        .next_source_near(from, stump, villager)
        .expect("there is wood in reach of him");
    let left = world
        .entity(next)
        .map(|node| node.amount.raw())
        .unwrap_or(0);
    assert!(
        left > 0,
        "the picker answered {next:?} with {left} raw units left in it: a worker sent there is sent back to the picker"
    );
}

/// **And a crew at a drained node stops bouncing** (D280). The end-to-end shape of the same rule: two drained trees and one
/// living one, a villager ordered at a stump - the re-task must cost a route, not one a tick for the rest of the match.
#[test]
fn a_crew_at_a_drained_node_does_not_bounce() {
    let Some(mut world) = world_of(HONEYCOMB) else {
        return;
    };
    let wood = world.slot_of("wood").expect("a wood slot");
    let scale = world.scale();
    let stump = at(
        &world,
        |entity| entity.kind == EntityKind::Resource(wood),
        5,
        7,
    );
    let role = world.role_unit("villager").expect("a villager role");
    let villager = world
        .entities()
        .iter()
        .find(|entity| {
            entity.kind == EntityKind::Unit
                && entity.player == 1
                && entity.unit.is_some_and(|unit| world.is_family(unit, role))
        })
        .map(|entity| entity.id)
        .expect("the village has a villager");
    for (id, (x, y)) in [(9_001u32, (4u16, 6u16)), (9_002, (4, 8))] {
        world
            .submit_checked(Command::Spawn {
                id: EntityId(id),
                player: 0,
                kind: EntityKind::Resource(wood),
                unit: None,
                position: (
                    Fixed::from_raw(i64::from(x) * scale),
                    Fixed::from_raw(i64::from(y) * scale),
                ),
                speed: Fixed::ZERO,
                hp: 0,
                amount: 0,
            })
            .expect("a drained tree can be placed");
    }
    world
        .submit_checked(Command::Gather {
            villager,
            node: stump,
        })
        .expect("a villager can be sent at a tree");

    let before = world.routes_planned();
    for _ in 0..300 {
        world.step().expect("stepping");
    }
    let spent = world.routes_planned() - before;
    // One route to the stump, one to whatever is next, and the two drained trees cost nothing: the ping-pong this fixture
    // was written for spent a route per villager per tick, which is 300 here.
    assert!(
        spent <= 40,
        "{spent} routes planned in 300 ticks with two drained trees beside the villager: he is being re-tasked to them"
    );
}
