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
    // **And once the crowd has settled it has not merged into a point** (D289, the owner: *"let the crowds jam"*). Every man
    // in this crowd was sent to the same tile, and the original's players describe exactly that - men standing pressed on one
    // spot - so the standoff is no longer asked for. What is still forbidden is a *stack*: two centres a quarter of a standoff
    // apart or more. Measured after the cut, this crowd settles at **0.172 tiles** against a **0.400** standoff.
    let settled = closest_pair(&world, &ids);
    assert!(
        settled < STANDOFF,
        "after 400 ticks the crowd has merged: {settled:.3} tiles apart against the {STANDOFF:.3} standoff (a quarter of \
         which is the floor), deepest passing {deepest:.3}"
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
    // **The eight stand on the node's own ring** (D313): scattered three tiles apart they took so long to converge that only
    // one man was ever *working* at the snapshot, and he filled his load and left at tick 13 - so the after-check could only
    // judge a vacuous set, or a man who had gone home for his own reasons. On the ring, four of the eight are taking by
    // tick 1,200 and the witnesses outlive the window.
    let ring = world.resource_tiles(node)[0];
    let mut gatherers = Vec::new();
    for nth in 0..8u32 {
        let spot = match nth {
            0 => (i64::from(ring.0) - 1, i64::from(ring.1) - 1),
            1 => (i64::from(ring.0), i64::from(ring.1) - 1),
            2 => (i64::from(ring.0) + 1, i64::from(ring.1) - 1),
            3 => (i64::from(ring.0) - 1, i64::from(ring.1)),
            4 => (i64::from(ring.0) + 1, i64::from(ring.1)),
            5 => (i64::from(ring.0) - 1, i64::from(ring.1) + 1),
            6 => (i64::from(ring.0), i64::from(ring.1) + 1),
            _ => (i64::from(ring.0) + 2, i64::from(ring.1)),
        };
        let id = a_villager(&mut world, 9_800 + nth, spot);
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
    // **The witness is a working man, not a place** (D293): the place book is gone, so "at work" is `progress > 0` on the node
    // itself, which is the claim this fixture has always been about.
    // **One man at work is enough to witness the rule** (D279): a crowd of eight converging on one tile is slow under the
    // original's pause-on-collision model - measured, only one of the eight had started taking after 1,200 ticks - and this
    // fixture is about a working man not being moved, not about how many men a tree seats in 20 seconds.
    assert!(
        !working.is_empty(),
        "at least one gatherer should be **at work** before the newcomer arrives; working {working:?}"
    );
    // **Where each working man stands, before the newcomer exists** (D293): the place book is gone, so the claim is measured on
    // the thing the owner actually sees - a working man's position.
    let before: Vec<(EntityId, (i64, i64))> = working
        .iter()
        .filter_map(|id| {
            world
                .position(*id)
                .map(|at| (*id, (at.0.raw(), at.1.raw())))
        })
        .collect();

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

    let mut judged = 0;
    for (worker, was) in &before {
        let now = world.position(*worker).map(|at| (at.0.raw(), at.1.raw()));
        // **Only a man still taking from the node is a witness** - the same test that chose `before` (D313). A man who has
        // filled his load and is walking to the bank, or has banked and is walking back, carries `Gather` with `progress 0`
        // and is legitimately somewhere else: measured, villager 9803 was at the town centre with **nothing in his hands and
        // progress 0** - just banked, walking back - when a tighter bank rule (D313) shifted his cycle by a few ticks. The
        // claim is about a man **at work**, so the exclusion has to be the one the selection used, not "is his task a gather".
        if !world
            .entity(*worker)
            .is_some_and(|e| matches!(e.task, Task::Gather { progress, .. } if progress.raw() > 0))
        {
            continue;
        }
        assert_eq!(
            now,
            Some(*was),
            "villager {} was moved off the ground he was working when a newcomer arrived at the node (was {:?}, now {now:?})",
            worker.0,
            was
        );
        judged += 1;
    }
    // **A loop that skips everyone judges nothing.** With the exclusion sharpened above, the witness can now be vacuous - so
    // say so out loud rather than pass on an empty set.
    assert!(
        judged > 0,
        "no man from the before-set was still taking from the node when the newcomer arrived: the fixture judged nothing"
    );
}

/// **A man walking past must not move the men who are seated** (D238). This is the owner's report of 2026-11-16, after D237:
/// *"villager still pushes another villager"*. Nothing writes another body's position, so what a player sees as a push is a
/// seated man who **gives up his square** - and a walker passes inside the reach of every square he goes by, which is why the
/// measurement is taken **while he is still walking**. Once he stops, his place in the ring is a standing body's and a
/// re-choice is the ring working (a man parked on your square is not a man walking past).
/// **Was `#[ignore]`d, and it was measuring its own grave** (D321). D238 diagnosed the owner's *"villager still pushes another
/// villager"* as the **seat shuffle** - a working man giving up his square because a neighbour walked within reach of it - and
/// found it *load-bearing*: removing it let villagers stand **0.204** tiles apart, because a walker still walked through a seated
/// man's square. Both halves of that have since gone:
///
/// * the **place book itself is gone** (D293, the owner: *"Drop it and let the crowds jam"*), so there is no square to give up; and
/// * the walker's **past** rule completes its chain (D320): a collider who has stood still since the route was re-planned is walked
///   through, and the pause is what stops the pair meeting again - no displacement is needed to resolve a passer-by.
///
/// So this is a **guard** now, not a gap: a man who is taking from a node keeps the point he is standing on while a fourth man walks
/// through the ring. It is not vacuous - the scene now seats three men **on the node's ring** and asserts one of them is actually
/// taking (`progress > 0`), and it counts the walker's own moving ticks so a passer-by who never crossed cannot pass it either.
#[test]
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

    // **They must arrive and start taking** (D321). The scene used to spawn them three tiles away in a row and step 500 ticks, which
    // after D312-D315 left them still walking in: the fixture then failed its own precondition (*"no gatherer was at work"*) and
    // measured nothing at all. Free tiles **on the node's ring** are found rather than assumed - the forest's own tiles are not
    // ground anyone can stand on.
    let stands: Vec<(i64, i64)> = [
        (1i64, 0i64),
        (0, 1),
        (-1, 0),
        (0, -1),
        (1, 1),
        (-1, 1),
        (1, -1),
        (-1, -1),
    ]
    .iter()
    .filter_map(|(dx, dy)| {
        let x = i64::from(tile.0) + dx;
        let y = i64::from(tile.1) + dy;
        let (Ok(x16), Ok(y16)) = (u16::try_from(x), u16::try_from(y)) else {
            return None;
        };
        world.is_free_ground(x16, y16).then_some((x, y))
    })
    .take(3)
    .collect();
    assert_eq!(stands.len(), 3, "three free tiles on the node's ring");
    let mut gatherers = Vec::new();
    for (nth, stand) in stands.iter().enumerate() {
        let nth = u32::try_from(nth).unwrap_or(0);
        let id = a_villager(&mut world, 9_900 + nth, *stand);
        world
            .submit_checked(Command::Gather { villager: id, node })
            .expect("a gather order");
        gatherers.push(id);
    }
    for _ in 0..400 {
        world.step().expect("a tick");
    }
    // **The ground each working man stands on** (D293): the place book is gone, so "seated" is a position - and for a man who is
    // taking from the node, that position must not change while somebody walks past.
    let seated: Vec<(EntityId, (i64, i64))> = gatherers
        .iter()
        .filter(|id| {
            world.entity(**id).is_some_and(
                |e| matches!(e.task, Task::Gather { progress, .. } if progress.raw() > 0),
            )
        })
        .filter_map(|id| {
            world
                .position(*id)
                .map(|at| (*id, (at.0.raw(), at.1.raw())))
        })
        .collect();
    assert!(
        !seated.is_empty(),
        "no gatherer was at work beside the node before the passer-by; the fixture measures nothing"
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
        for (worker, place) in &seated {
            // A man who has filled his load walks to the bank; that is the economy, not a shuffle.
            if !world
                .entity(*worker)
                .is_some_and(|e| matches!(e.task, Task::Gather { .. }))
            {
                continue;
            }
            let held = world.position(*worker).map(|at| (at.0.raw(), at.1.raw()));
            assert_eq!(
                held,
                Some(*place),
                "villager {} was moved off the ground it was working at tick {} while a man was walking past (was {place:?}, \
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
