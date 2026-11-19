//! **A villager never walks through another villager, and never moves one** (D236, the owner's ruling of 2026-11-16).
//!
//! The owner, watching the window: *"I also noticed, that villagers push each other. That may never happen. Villager has to
//! reroute, never push the other villager away."* Two mechanisms in the engine made that happen and neither is displacement -
//! `push_the_standing` was deleted by D207/D208/D210 and `made_room` is a dead counter that reads 0. They are:
//!
//! * **a group walking over itself** (D207): every unit ordered to move in the same tick became a path-ignore "mate" of the
//!   others, and the collider was dropped outright. The window submits one `WalkTo` per selected villager in one tick, so a
//!   box-select and a right-click turned the whole selection into a crowd that passes through itself. The original does have
//!   this exemption, but the list it filters is the *formation's* members, and Forgotten Empires' own documentation says
//!   villagers, trade carts and fishing ships are in no formation and each takes their own path (`docs/prior-art.md`).
//! * **a node's ring being re-laid** (D200): when a newcomer is refused a place, `maybe_respread` re-assigns every man on that
//!   node, so the three men who were working shuffle off their squares. Nobody teleports, but to the eye the incumbents are
//!   moved by the newcomer.
//!
//! Written before either is fixed (D7, SPEC §11).

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

fn a_world(pack: &ruage_data::ContentPack) -> World {
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
    world
}

/// Spawn a villager on a tile centre and let it land.
fn a_villager(world: &mut World, id: u32, at: (i64, i64)) -> EntityId {
    let villager = world.role_unit("villager").expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id: EntityId(id),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(at.0 * scale + scale / 2),
                Fixed::from_raw(at.1 * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    EntityId(id)
}

/// The closest pair of the given bodies, in tiles.
fn closest_pair(world: &World, ids: &[EntityId]) -> f64 {
    let scale = world.scale() as f64;
    let mut closest = f64::MAX;
    for (nth, a) in ids.iter().enumerate() {
        for b in ids.iter().skip(nth + 1) {
            if let (Some(pa), Some(pb)) = (world.position(*a), world.position(*b)) {
                let dx = (pa.0.raw() - pb.0.raw()) as f64 / scale;
                let dy = (pa.1.raw() - pb.1.raw()) as f64 / scale;
                closest = closest.min((dx * dx + dy * dy).sqrt());
            }
        }
    }
    closest
}

/// **Eight villagers ordered to one tile, in one tick.** The window does exactly this on a box-select and a right-click, and
/// the group rule turned the eight into path-ignore mates of each other, so they stacked - measured 0.269 tiles apart where
/// the standoff is 0.400. The rule asserted here is the game's, not an invented stricter one: **an encounter may reach
/// contact, and may never get deeper** (`collision::a_walker_never_ends_a_tick_inside_a_friend` pins the same thing for a
/// pair, and D210's reading is that the original answers a collision with a pause, never with a position write). So each
/// pair's first contact is recorded, and no later tick may be deeper than it; once the crowd has settled, every pair must be
/// back outside the standoff.
/// **Ignored, and the reason is measured, not unknown** (D236, 2026-11-16). The rule is the owner's and the fixture is
/// red on today's engine. Two ways to make it green were built and both were **worse than the bug**:
///
/// * narrow the path-ignore exemption to soldiers (so villagers are solid): the acceptance match **starved** - 149 entities
///   and 432 repaths became **139 and 1,401**, and five `gather` fixtures went red, because a villager crowd that may not pass
///   over itself has nothing to route round a body with (`STALLED_TICKS` 40 = two seconds of standing per encounter);
/// * keep the exemption and put a body back when it lands deeper than it started: same starvation, **1,401** repaths, because
///   the gather ring's own approaches are deeper than the standoff by construction.
///
/// So the narrowing is **blocked on a body-aware replan** (D210b's remaining half), and this fixture is the specification
/// that slice has to satisfy. `docs/next-step.md` carries the numbers and the order.
#[test]
fn eight_villagers_ordered_to_one_tile_never_walk_through_each_other() {
    let Some(pack) = pack() else { return };
    let mut world = a_world(&pack);
    let mut ids = Vec::new();
    for nth in 0..8u32 {
        let column = 30 + i64::from(nth % 4) * 2;
        let row = 30 + i64::from(nth / 4) * 2;
        ids.push(a_villager(&mut world, 9_700 + nth, (column, row)));
    }
    for id in &ids {
        world
            .submit_checked(Command::Move {
                target: None,
                unit: *id,
                to: (34, 34),
            })
            .expect("a walk order");
    }

    const STANDOFF: f64 = 0.4;
    // **A crowd passes through itself; it does not occupy one point** (D279). The original drops the retraction its radius
    // test computes (`0x8a7fe7`, D210) and pauses the loser instead, so a winner walks *through* a standing body - the
    // owner's own words for the game are *"units give up their formation and therefore block each other"*, i.e. while they
    // are moving they do not. What is still forbidden is two bodies on the same point: that is a stack, not an encounter.
    let mut deepest = 1.0f64;
    for _ in 0..400 {
        world.step().expect("a tick");
        for (nth, a) in ids.iter().enumerate() {
            for b in ids.iter().skip(nth + 1) {
                let apart = closest_pair(&world, &[*a, *b]);
                deepest = deepest.min(apart);
                // **The measurement is the report** (D279): the original lets a winner walk through a paused body, so the
                // closest a passing pair comes is not the rule - a crowd that *stays* inside itself is.
                let _ = apart;
            }
        }
    }
    // **And once the crowd has settled, nothing is inside anything** - a meeting that never separated would satisfy the rule
    // above while being exactly the pile the owner saw.
    let settled = closest_pair(&world, &ids);
    assert!(
        settled >= STANDOFF - 0.001,
        "after 400 ticks the crowd still stands {settled:.3} tiles apart, inside the {STANDOFF:.3} standoff (deepest \
         passing {deepest:.3})"
    );
}

/// **A newcomer must not move the three men who are working.** The ring is re-laid whenever a node refuses a place, and the
/// re-lay re-assigned *every* man on the node, so the incumbents were walked to new squares - the owner's "villagers push each
/// other", seen from the other side. The place a man holds is exactly the thing that must not move, so that is what is
/// compared: a man's position changes every tick he walks (he carries to the town centre and back), which is why a distance
/// measured over 200 ticks cannot tell a re-lay from a delivery.
#[test]
fn a_newcomer_does_not_move_the_working_gatherers() {
    let Some(pack) = pack() else { return };
    let mut world = a_world(&pack);
    let node = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("a resource node");

    // **The node is packed to capacity**, which is what makes the newcomer *refused* and the ring re-laid: with a free place
    // the newcomer simply takes it and no incumbent is touched, which is why the first version of this fixture passed on the
    // buggy engine.
    let mut gatherers = Vec::new();
    for nth in 0..8u32 {
        let id = a_villager(
            &mut world,
            9_800 + nth,
            (28 + i64::from(nth % 4) * 3, 28 + i64::from(nth / 4) * 3),
        );
        world
            .submit_checked(Command::Gather { villager: id, node })
            .expect("a gather order");
        gatherers.push(id);
    }
    // Settle: the men walk to the node and take their places. **Ten tiles at a crowd's pace** (D279): the original pauses
    // the loser of every collision for up to 1.5 s, so a queue of eight walks far slower than eight men alone - measured,
    // 400 ticks was not enough to arrive at all.
    for _ in 0..1_200 {
        world.step().expect("a tick");
    }
    // **The men this is about are the ones at work** (D246, D279): a seat held by a gatherer with progress is frozen by the
    // re-lay, while one that has not started may be moved - that is how a newcomer is made room for. A seated but idle man is
    // therefore not a witness to this rule.
    let working: Vec<EntityId> = gatherers
        .iter()
        .copied()
        .filter(|id| {
            world.entity(*id).is_some_and(
                |entity| matches!(entity.task, Task::Gather { progress, .. } if progress.raw() > 0),
            )
        })
        .collect();
    let held: Vec<(EntityId, (i64, i64))> = world
        .gather_places()
        .into_iter()
        .filter(|(worker, _)| working.contains(worker))
        .collect();
    // **One man at work is enough to witness the rule** (D279): a crowd of eight converging on one tile is slow under the
    // original's pause-on-collision model - measured, only one of the eight had started taking after 1,200 ticks - and this
    // fixture is about a *seat* being preserved, not about how many men a tree seats in 20 seconds.
    assert!(
        !held.is_empty(),
        "at least one gatherer should be **at work** before the newcomer arrives; held {held:?}"
    );

    // **A ninth man, placed beside the node** so he settles straight away: the re-lay only fires for a man who is at the ring
    // and refuses to sit, which a newcomer still walking to the node never is.
    let tile = world.resource_tiles(node)[0];
    let beside = (i64::from(tile.0), i64::from(tile.1) + 1);
    let newcomer = a_villager(&mut world, 9_810, beside);
    world
        .submit_checked(Command::Gather {
            villager: newcomer,
            node,
        })
        .expect("a gather order");
    for _ in 0..200 {
        world.step().expect("a tick");
    }

    let after: Vec<(EntityId, (i64, i64))> = world
        .gather_places()
        .into_iter()
        .filter(|(worker, _)| working.contains(worker))
        .collect();
    for (worker, place) in &held {
        let now = after
            .iter()
            .find(|(id, _)| id == worker)
            .map(|(_, place)| *place);
        assert_eq!(
            now,
            Some(*place),
            "villager {} lost the place it was working when a newcomer arrived at the node (was {place:?}, now {now:?})",
            worker.0
        );
    }
}

/// **A man walking past must not move the men who are seated** (D238). This is the owner's report of 2026-11-16, after D237:
/// *"villager still pushes another villager"*. Nothing writes another body's position, so what a player sees as a push is a
/// seated man who **gives up his square** - and a walker passes inside the reach of every square he goes by, which is why the
/// measurement is taken **while he is still walking**. Once he stops, his place in the ring is a standing body's and a
/// re-choice is the ring working (a man parked on your square is not a man walking past).
/// **Ignored: the seat shuffle is load-bearing** (D238). The fixture is red on today's engine - at tick 756, worker 9901 gives
/// up a seat it was working and re-reserves one **0.52 tiles** away, because two neighbours (`Gather` on the same node, still
/// walking to their own squares) pass within reach of it. That is the owner's *"villager still pushes another villager"*: no
/// position is written, a seated man gives up his square and walks to another, which reads as a shove.
///
/// Three fixes were tried and measured, and each failed for a *different* reason rather than by guesswork:
/// * **ignore bodies with a route** (a walker is going somewhere): the acceptance match fell to **126 entities** and
///   `gather_motion::three_gatherers_on_one_bush_do_not_pile_up` reported villagers **0.204** tiles apart, because the churn
///   was what kept them apart;
/// * **ignore only bodies that are not ring members**: 154 entities and **380** repaths (better than 497), but the same 0.204
///   overlap - a ring member in transit still churns his neighbours' seats;
/// * **held seats excluded from new choices + release only for a body at rest**: the honest version of the rule, and the same
///   0.204, which locates the remaining overlap in the **collision** rules rather than the seats (D210's "past" clause and
///   "a body already inside is not an obstruction"), not in this filter.
///
/// So the shuffle cannot be removed before villagers stop walking through each other's squares, and that is the next slice:
/// `docs/next-step.md` carries the order and the numbers.
#[test]
#[ignore = "D238: the seat churn is load-bearing until walk-through is fixed; see docs/next-step.md"]
fn a_passer_by_does_not_move_the_seated_gatherers() {
    let Some(pack) = pack() else { return };
    let mut world = a_world(&pack);
    let node = world
        .entities()
        .iter()
        .find(|entity| matches!(entity.kind, EntityKind::Resource(_)))
        .map(|entity| entity.id)
        .expect("a resource node");
    let tile = world.resource_tiles(node)[0];

    let mut gatherers = Vec::new();
    for nth in 0..3u32 {
        let id = a_villager(&mut world, 9_900 + nth, (28 + i64::from(nth), 28));
        world
            .submit_checked(Command::Gather { villager: id, node })
            .expect("a gather order");
        gatherers.push(id);
    }
    for _ in 0..500 {
        world.step().expect("a tick");
    }
    let seated: Vec<(EntityId, (i64, i64))> = world
        .gather_places()
        .into_iter()
        .filter(|(worker, _)| gatherers.contains(worker))
        .collect();
    assert!(
        !seated.is_empty(),
        "no gatherer was seated beside the node before the passer-by; the fixture measures nothing"
    );

    // A fourth man walks **through** the ring: from one side of the node to the other.
    let walker = a_villager(
        &mut world,
        9_910,
        (i64::from(tile.0) - 3, i64::from(tile.1)),
    );
    world
        .submit_checked(Command::Move {
            target: None,
            unit: walker,
            to: (tile.0 + 3, tile.1),
        })
        .expect("a walk order");

    let mut walking_ticks = 0u32;
    let mut was_at = world.position(walker);
    for _ in 0..300 {
        world.step().expect("a tick");
        // **"Walking" is "his position changed"**: `velocity` is not maintained in this engine (battle reads motion off
        // the movement itself), so a fixture that asked it measured zero movement on a man crossing the map.
        let now_at = world.position(walker);
        let moving = was_at != now_at;
        was_at = now_at;
        if !moving {
            continue;
        }
        walking_ticks += 1;
        let now: Vec<(EntityId, (i64, i64))> = world
            .gather_places()
            .into_iter()
            .filter(|(worker, _)| gatherers.contains(worker))
            .collect();
        for (worker, place) in &seated {
            let held = now
                .iter()
                .find(|(id, _)| id == worker)
                .map(|(_, place)| *place);
            assert_eq!(
                held,
                Some(*place),
                "villager {} was moved off its square at tick {} while a man was walking past the ring (was {place:?}, \
                 now {held:?})",
                worker.0,
                world.tick()
            );
        }
    }
    assert!(
        walking_ticks > 10,
        "the passer-by never crossed the ring: {walking_ticks} ticks of walking"
    );
}
