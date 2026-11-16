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
                unit: *id,
                to: (34, 34),
            })
            .expect("a walk order");
    }

    const STANDOFF: f64 = 0.4;
    let mut contact: std::collections::BTreeMap<(u32, u32), f64> =
        std::collections::BTreeMap::new();
    for _ in 0..400 {
        world.step().expect("a tick");
        for (nth, a) in ids.iter().enumerate() {
            for b in ids.iter().skip(nth + 1) {
                let pair = (a.0.min(b.0), a.0.max(b.0));
                let apart = closest_pair(&world, &[*a, *b]);
                if apart < STANDOFF {
                    let entry = contact.entry(pair).or_insert(apart);
                    assert!(
                        apart + 1e-9 >= *entry,
                        "villagers {:?} got {:.3} tiles apart at tick {}, deeper than the {:.3} they first touched at - \
                         that is walking through, not walking past",
                        pair,
                        apart,
                        world.tick(),
                        *entry
                    );
                    *entry = (*entry).min(apart);
                }
            }
        }
    }
    // **And once the crowd has settled, nothing is inside anything** - a meeting that never separated would satisfy the
    // no-deepening rule while being exactly the pile the owner saw.
    let settled = closest_pair(&world, &ids);
    assert!(
        settled >= STANDOFF - 0.001,
        "after 400 ticks the crowd still stands {settled:.3} tiles apart, inside the {STANDOFF:.3} standoff"
    );
    assert!(
        !contact.is_empty(),
        "the crowd never met - the fixture did not exercise the rule"
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
    // Settle: the three walk to the node and take their places.
    for _ in 0..400 {
        world.step().expect("a tick");
    }
    let held: Vec<(EntityId, (i64, i64))> = world
        .gather_places()
        .into_iter()
        .filter(|(worker, _)| gatherers.contains(worker))
        .collect();
    assert!(
        held.len() >= 3,
        "the gatherers should hold places before the newcomer arrives; held {held:?}"
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
        .filter(|(worker, _)| gatherers.contains(worker))
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
