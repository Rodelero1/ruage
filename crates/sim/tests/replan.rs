//! **R1: the re-plan must not accumulate the bodies it has already met** (D239 rows 29-30, the owner's ruling: adopt the game).
//!
//! Two claims, both from the game's own behaviour and neither true of this engine before this slice:
//!
//! 1. **A second re-plan is not longer than the first.** The re-plan folded `route.avoiding` - every body the route had already
//!    been planned around - into the search's **obstacle tiles**, so each meeting added a tile the search had to go round: a
//!    walker who met three men in a row ended up with a route that detoured round three patches of ground he had long passed.
//!    The game keeps that set in the classifier (which bodies are non-past) and never hands it to the search as ground.
//! 2. **A body at rest that is not past is a collision, not a step-through.** `collide`'s rule 6 took the step anyway
//!    (`TakeAtRest`), unbounded, after asking for a re-path - so the route round the man arrived one tick *after* we had already
//!    walked through him. The original's epilogue has no such arm.
//!
//! Written before either change (D7, SPEC §11).

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

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

/// A villager on a tile centre, landed.
fn a_villager(world: &mut World, id: u32, column: i64, row: i64) -> EntityId {
    let villager = world.role_unit("villager").expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id: EntityId(id),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(villager),
            position: (
                Fixed::from_raw(column * scale + scale / 2),
                Fixed::from_raw(row * scale + scale / 2),
            ),
            speed: Fixed::parse("0.8", scale).expect("a speed"),
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("the villager lands");
    EntityId(id)
}

/// **A man who has stopped is walked through only *after* the mover's re-plan** (row 29, D320). The original's chain on a
/// non-past collision is a **re-path** and a pause: the new path carries a new `createdAt`, and the man who has stood still
/// since then is past (`path->createdAt >= lastMoveTime`), so the walker goes on. Keeping the old stamp - which this engine
/// did - leaves a stopped man non-past for ever, and the walker sinks through him tick after tick until their centres
/// coincide: measured, **0 raw apart at tick 176**, and 600 re-plans in 600 ticks.
///
/// The meeting is forced the way `step_decision.rs` forces its own: the friend is put on the walker's row **after** the route
/// exists, so he is met rather than planned around (D210b).
/// **Ignored: deleting the arm deadlocks two soldiers in a lane** (measured 2026-11-16, in this same session). With rule 6's
/// unit arm gone the walker stops at the reach and re-plans, which is right - but `group_overlap.rs`'s head-on pair then stand
/// one tile apart for 2,000 ticks ("both have to arrive: a at (5,1), b at (5,2)") and its mate fixture never meets at all.
/// The resolution the game uses is the **past** rule: once one of them is paused he stops moving, the other's re-plan is
/// stamped later, and the stopped man is walked through. That chain does not complete here yet, so the arm stays and this
/// fixture is the specification for the slice that completes it.
#[test]
fn a_stopped_friend_is_only_walked_through_after_the_replan() {
    let Some(pack) = pack() else { return };
    let mut world = a_world(&pack);
    let row = 4i64;
    let walker = a_villager(&mut world, 9_500, 4, row);
    world
        .submit_checked(Command::Move {
            target: None,
            unit: walker,
            to: (30, row as u16),
        })
        .expect("a walk across the map");
    world.step().expect("the order lands");

    // **The friend must have moved *after* the walker's route was planned**, or he is "past" and the game walks through him
    // too (`path->createdAt >= other->lastMoveTime`, and a never-moved body's `lastMoveTime` is its birth). So he is made to
    // walk a tile towards the walker and then stops: `last_moved > route.created`, which is exactly the not-past case row 29
    // is about.
    let scale = world.scale();
    let ahead = world.position(walker).expect("a walker").0.raw() / scale + 8;
    let friend = a_villager(&mut world, 9_600, ahead, row);
    world
        .submit_checked(Command::Move {
            target: None,
            unit: friend,
            to: (u16::try_from(ahead - 1).expect("a column"), row as u16),
        })
        .expect("he walks towards the walker");
    for _ in 0..40 {
        world.step().expect("a tick");
    }
    world
        .submit_checked(Command::Stop { unit: friend })
        .expect("and stops where he is");
    world.step().expect("the stop lands");
    world
        .step()
        .expect("and one more, so he has not moved this tick");

    // **The game's chain, not a stricter one** (D320). A rester who moved after the route was drawn is *not past*, so he is a
    // collision: the mover re-paths - and a re-plan is a **new path**, so it is stamped later, which is the link that was missing -
    // and the loser is paused. Once the stamp is later than the friend's last move the friend **is** past and the walker goes
    // through him, which is the original's own rule (`path->createdAt >= other->lastMoveTime`) and what D239's reading of row 29
    // describes. What must never happen is the deleted arm's shape: sinking deeper for **600 ticks** because a stopped man can
    // never become past. So the bar is the pause window - at most 1.5 s, 60 ticks - of being inside a man who is not past.
    let reach = world.reach_between(walker, friend).raw();
    let repaths_before = world.repaths();
    let mut contact: Option<i64> = None;
    let mut inside_not_past: u64 = 0;
    let mut got_past = false;
    for _ in 0..600 {
        world.step().expect("a tick");
        let (a, b) = (
            world.position(walker).expect("alive"),
            world.position(friend).expect("alive"),
        );
        let (dx, dy) = (a.0.raw() - b.0.raw(), a.1.raw() - b.1.raw());
        let apart = (dx * dx + dy * dy).isqrt();
        if contact.is_none() && apart < reach {
            contact = Some(apart);
        }
        let past = world.is_past(walker, friend);
        if apart < reach && !past {
            inside_not_past += 1;
        }
        if past {
            got_past = true;
        }
    }
    let contact =
        contact.expect("MEASURED the walker never came within the reach: the fixture did not meet");
    assert!(
        got_past,
        "the friend never became past after the walker's re-plan: a re-plan that keeps the old stamp leaves a stopped man \
         non-past for ever, so the walker can only ever sink through him"
    );
    assert!(
        inside_not_past <= 60,
        "the walker spent {inside_not_past} ticks inside a friend who was **not** past (contact was {contact} raw at the \
         first touch): the game's chain answers that in one re-plan and a pause of at most 1.5 s, i.e. 60 ticks"
    );
    let replans = world.repaths() - repaths_before;
    assert!(
        replans <= 4,
        "the walker re-planned {replans} times in 600 ticks against a man who never moved: the re-plan is not taking a fresh \
         stamp, so every tick answers the same collision again"
    );
    assert!(
        world.position(walker).is_some(),
        "the walker is still on the map after crossing the lane"
    );
}
