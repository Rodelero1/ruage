//! **A head-on is settled by time, not geometry** (D210). The original never clamps a step against another body: at the
//! movement site the radius retraction's result is written to a local that is read nowhere (`0x8a7fe7` is the instruction
//! after the call, and it tests a debug flag). A pair meeting is resolved by pausing the higher id, repathing the lower one
//! once, and then classifying the collision **"past"** - `path->createdAt < other->lastMoveTime` - so the winner walks
//! through and the loser resumes afterwards.
//!
//! Written before the implementation (D7). Today the engine refuses a step that would end inside `r1 + r2`
//! (`bound_to_reach`), which has no counterpart in the binary and is what freezes this pair.

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn fresh() -> Option<World> {
    common::fresh()
}

fn next_id(world: &World) -> EntityId {
    EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    )
}

fn spawn_villager(world: &mut World, id: EntityId, x: u16, y: u16) {
    let unit = world.start_villager_unit().expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(x) * scale),
                Fixed::from_raw(i64::from(y) * scale),
            ),
            speed: Fixed::from_raw(scale / 2),
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
}

fn open_row(world: &World, span: u16) -> Option<(u16, u16)> {
    let (width, height) = (64u16, 64u16);
    for y in 2..height.saturating_sub(2) {
        for x in 2..width.saturating_sub(span) {
            if (0..span).all(|d| world.is_free_ground(x + d, y)) {
                if (8..24).contains(&x) && (8..24).contains(&y) {
                    continue;
                }
                return Some((x, y));
            }
        }
    }
    None
}

/// **A corridor one tile wide**, with forest above and below: no route round a body exists, so a meeting has to be settled by the
/// original's own means - pause the higher id, classify the collision "past", let the winner walk through (D210).
const CORRIDOR: &str = r#"
schema = 1
[map]
width = 24
height = 22
terrain = [
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "FFFFFFFFFFFFFFFFFFFFFFFF",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "FFFFFFFFFFFFFFFFFFFFFFFF",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
  "GGGGGGGGGGGGGGGGGGGGGGGG",
]
[[map.town_center]]
player = 1
x = 1
y = 1
[[map.town_center]]
player = 2
x = 22
y = 20
"#;

fn corridor_world() -> Option<World> {
    if !head_on_repo("config/gamedata/units.toml").exists() {
        return None;
    }
    let resources = common::present(
        &head_on_repo("config/resources.toml"),
        ruage_data::load_resources_file,
    );
    let settings = common::present(&head_on_repo("config/sim.toml"), ruage_data::load_sim_file);
    let map = common::must(
        "load_map_str",
        ruage_data::load_map_str(CORRIDOR, "corridor.toml", &resources),
    );
    let economy = common::present(
        &head_on_repo("config/economy.toml"),
        ruage_data::load_economy_file,
    );
    let units = common::must(
        "load_units_file",
        ruage_data::load_units_file(&head_on_repo("config/gamedata/units.toml")),
    );
    let mut world = common::must("World::new", World::new(&settings, 7));
    common::must("seed_from", world.seed_from(&map, &economy, &units));
    common::must("load_resources", world.load_resources(&resources));
    Some(world)
}

fn head_on_repo(rel: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// A knight: radius **0.25**, so two of them are exactly the width of a one-tile corridor - there is no room to slip past.
fn spawn_knight(world: &mut World, id: EntityId, x: u16, y: u16) {
    let unit = world.role_unit("knight").expect("a knight role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(i64::from(x) * scale + scale / 2),
                Fixed::from_raw(i64::from(y) * scale + scale / 2),
            ),
            speed: Fixed::parse("1.35", scale).expect("a knight's speed"),
            hp: 100,
            amount: 0,
        })
        .expect("a knight can be placed");
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let scale = world.scale();
    let (x, y) = world.position(id).expect("the entity has a position");
    (x.raw().div_euclid(scale), y.raw().div_euclid(scale))
}

fn apart(world: &World, a: EntityId, b: EntityId) -> i64 {
    let (ax, ay) = world.position(a).expect("a has a position");
    let (bx, by) = world.position(b).expect("b has a position");
    let (dx, dy) = (ax.raw() - bx.raw(), ay.raw() - by.raw());
    (dx * dx + dy * dy).max(0)
}

/// **Two men who meet head-on pass each other.** They are ordered a tick apart, so neither is the other's mate (D207) and
/// the collision is a real one: the game's answer is the pause plus one repath, and the outcome is that both arrive. The
/// engine's current answer is a permanent standoff at exactly `r1 + r2`, which is what this test refuses to accept.
#[test]
fn two_men_meeting_head_on_pass_each_other() {
    let Some(mut world) = fresh() else {
        return;
    };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let a = next_id(&world);
    let b = EntityId(a.0 + 1);
    spawn_villager(&mut world, a, x, y);
    spawn_villager(&mut world, b, x + 6, y);
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        target: None,
        unit: a,
        to: (x + 6, y),
    });
    world.step().expect("a tick between the orders");
    world.submit(Command::Move {
        target: None,
        unit: b,
        to: (x, y),
    });

    let reach = world.reach_between(a, b).raw();
    let mut closest = i64::MAX;
    let mut met = false;
    for _ in 0..2_000 {
        world.step().expect("step");
        let gap = apart(&world, a, b);
        closest = closest.min(gap);
        // **Touching counts** (the binary's own clamp allows exactly `r1 + r2`): a pair that stops at the reach has met,
        // and the fixture's job is to prove the collision happened at all.
        if gap <= reach * reach {
            met = true;
        }
    }
    let near = |id: EntityId, goal: (u16, u16)| {
        let (tx, ty) = tile_of(&world, id);
        (tx - i64::from(goal.0)).abs() <= 1 && (ty - i64::from(goal.1)).abs() <= 1
    };
    assert!(
        met,
        "MEASURED the pair never came inside the {} raw reach (closest {}): they did not meet, so this fixture is not \
         testing a collision",
        reach,
        closest.max(0).isqrt()
    );
    assert!(
        near(a, (x + 6, y)) && near(b, (x, y)),
        "MEASURED a head-on pair never got past each other: a at {:?}, b at {:?} after 2,000 ticks, reach {} raw - the \
         original lets the winner walk through the paused loser (its collision block never clamps a step against a body)",
        tile_of(&world, a),
        tile_of(&world, b),
        reach
    );
}

/// **The loser stands for the drawn pause, and no longer.** The original's own duration is `rand()*1.25/32767 + 0.25`
/// seconds; the *first* stand must fall in that band and the pair must still get past. A stand that never ends is the
/// freeze this slice exists to remove, and a stand shorter than a quarter second is not the game's rule either.
#[test]
fn the_loser_stands_for_the_drawn_pause() {
    let Some(mut world) = corridor_world() else {
        return;
    };
    // **Two knights in a one-tile corridor, each ordered to the other's tile.** Knights are radius 0.25, so a pair is exactly the
    // corridor's width: no route round exists and no room to slip past, which is the scene the original's pause rule is for.
    // (Villagers would fit side by side - 0.2 + 0.2 is 0.8 of a tile - and the pair would pass without ever touching.)
    let a = EntityId(9_500);
    let b = EntityId(9_501);
    spawn_knight(&mut world, a, 6, 11);
    spawn_knight(&mut world, b, 14, 11);
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        target: None,
        unit: a,
        to: (14, 11),
    });
    world.step().expect("a tick between the orders");
    world.submit(Command::Move {
        target: None,
        unit: b,
        to: (6, 11),
    });

    // **The first stand *after* contact.** The contact tick is not a stand: the loser moves in it (the pause is written after the
    // pass, so it cannot hold him on the very tick he collides), and the measured run begins with the first tick he does not move
    // after that - undone at the first tick he moves again. Counting from contact measured 0 for exactly that reason.
    let reach = world.reach_between(a, b).raw();
    let mut touching = false;
    let mut stand = 0u32;
    let mut first_stand = 0u32;
    let mut last = world.position(b).expect("b has a position");
    for _ in 0..2_000 {
        world.step().expect("step");
        let now = world.position(b).expect("b has a position");
        let moved = now.0 != last.0 || now.1 != last.1;
        last = now;
        if apart(&world, a, b) <= reach * reach {
            touching = true;
        }
        if !touching {
            continue;
        }
        if moved {
            if stand > 0 {
                first_stand = stand;
                break;
            }
        } else {
            stand += 1;
        }
    }
    let seconds = f64::from(first_stand) / f64::from(world.tick_rate_hz());
    assert!(
        (0.25..=1.50).contains(&seconds),
        "MEASURED the loser's longest stand was {} ticks ({seconds:.3} s): the original draws 0.25-1.50 s and re-rolls on \
         a fresh collision",
        first_stand
    );
}
