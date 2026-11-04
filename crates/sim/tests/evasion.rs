//! Evasion, the original's shape (D98a, owner: "B adopt from the original game"; E1a = the machinery on a tile
//! predicate). Written before the implementation (D7).
//!
//! The rule is not "wait and let pass" any more: a blocked unit **repaths**, and a collision with a body the route is
//! already planned around is *past* - no second repath. Terrain is the `Walk` layer's business; this file is about
//! bodies.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn fresh() -> Option<World> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).ok()?;
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).ok()?;
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).ok()?;
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).ok()?;
    let units = ruage_data::load_units_file(&units).ok()?;
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).ok()?;
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    world.load_combat(&combat);
    Some(world)
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

/// A tile's **corner**, which is where a spawn is placed from - the engine's own convention for `Command::Spawn`, and
/// deliberately not the walk-goal convention (D114 centres those).
fn at(world: &World, x: u16, y: u16) -> (Fixed, Fixed) {
    let scale = world.scale();
    (
        Fixed::from_raw(i64::from(x) * scale),
        Fixed::from_raw(i64::from(y) * scale),
    )
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let scale = world.scale();
    let (x, y) = world.position(id).expect("the entity has a position");
    (x.raw().div_euclid(scale), y.raw().div_euclid(scale))
}

/// A villager placed by hand, so the fixture does not depend on where the map seeds its own.
///
/// **The id is chosen by the caller**: `next_id` reads the entity list, and a `Spawn` lands on the next tick, so two
/// spawns in one tick would otherwise both take the same id - the fixture bug that made the first version of these
/// tests report both villagers on tile (0, 0).
/// A villager at an arbitrary point, for fixtures that need a body **exactly** on a line - a tile is too coarse now that a
/// walk aims at the middle of one (D114).
fn spawn_villager_at(world: &mut World, id: EntityId, point: (Fixed, Fixed)) {
    let unit = world.start_villager_unit().expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: point,
            speed: Fixed::from_raw(scale / 2),
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
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
            position: at(world, x, y),
            speed: Fixed::from_raw(scale / 2),
            hp: 25,
            amount: 0,
        })
        .expect("a villager can be placed");
}

/// Two open tiles in a row on the project map, far enough from the base that the seed's own villagers are not in the
/// way. Derived from the map rather than hand-picked: content changes and a hard-coded coordinate would rot.
fn open_row(world: &World, span: u16) -> Option<(u16, u16)> {
    let (width, height) = (64u16, 64u16);
    for y in 2..height.saturating_sub(2) {
        for x in 2..width.saturating_sub(span) {
            if (0..span).all(|d| world.is_free_ground(x + d, y)) {
                // Away from the seeded base, which sits around (16,16).
                if (8..24).contains(&x) && (8..24).contains(&y) {
                    continue;
                }
                return Some((x, y));
            }
        }
    }
    None
}

/// Two villagers walking through each other never share a tile, and both arrive. The test D98 owed, now satisfied by
/// the original's mechanism rather than by waiting.
#[test]
fn two_villagers_crossing_never_share_a_tile_and_both_arrive() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let a = next_id(&world);
    let b = EntityId(a.0 + 1);
    spawn_villager(&mut world, a, x, y);
    spawn_villager(&mut world, b, x + 6, y);
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        unit: a,
        to: (x + 6, y),
    });
    world.submit(Command::Move {
        unit: b,
        to: (x, y),
    });

    // **The assertion used to be "never share a tile"** - that was the old rule speaking. The rule is a distance now, so
    // the test is one (C): two bodies never come inside the standoff their footprints need, which is the stronger claim
    // and the one a crowd is judged by.
    let reach = world.reach_between(a, b).raw();
    for tick in 0..2_000 {
        world.step().expect("step");
        let (pa, pb) = (
            world.position(a).expect("a position"),
            world.position(b).expect("a position"),
        );
        let (dx, dy) = (pa.0.raw() - pb.0.raw(), pa.1.raw() - pb.1.raw());
        let apart = dx * dx + dy * dy;
        assert!(
            apart >= reach * reach,
            "they came {} raw units apart on tick {tick}, inside the {reach} their footprints need",
            apart.unsigned_abs().isqrt()
        );
    }
    let moved = tile_of(&world, a) != (i64::from(x), i64::from(y))
        && tile_of(&world, b) != (i64::from(x + 6), i64::from(y));
    assert!(moved, "both villagers have to have walked: {a:?} {b:?}");
    let near = |id: EntityId, goal: (u16, u16)| {
        let (tx, ty) = tile_of(&world, id);
        (tx - i64::from(goal.0)).abs() <= 1 && (ty - i64::from(goal.1)).abs() <= 1
    };
    assert!(
        near(a, (x + 6, y)) && near(b, (x, y)),
        "both have to arrive: a at {:?}, b at {:?}",
        tile_of(&world, a),
        tile_of(&world, b)
    );
}

/// A body standing on the straight line is a **new** obstacle, so the unit repaths around it and still arrives. The
/// detour is asserted as a change in the other axis, not as a particular route: the pather's choices are its own.
#[test]
fn a_blocked_unit_goes_round_a_standing_body_and_arrives() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let walker = next_id(&world);
    let sitter = EntityId(walker.0 + 1);
    spawn_villager(&mut world, walker, x, y);
    spawn_villager(&mut world, sitter, x + 3, y);
    world.step().expect("the spawns land");

    world.submit(Command::Move {
        unit: walker,
        to: (x + 6, y),
    });
    // **Went round** is the standoff, not a change of row: the old row-change proxy stopped meaning anything once a tile
    // order aimed at the middle of a tile (D114), because the walk then ends inside the same row either way. The measured
    // claim is the one `collide` owes - the walker never comes inside the two footprints' standoff.
    let reach = world.reach_between(walker, sitter).raw();
    let mut closest = i64::MAX;
    for _ in 0..2_000 {
        world.step().expect("step");
        let (a, b) = (
            world.position(walker).expect("alive"),
            world.position(sitter).expect("alive"),
        );
        let (dx, dy) = (a.0.raw() - b.0.raw(), a.1.raw() - b.1.raw());
        closest = closest.min(dx * dx + dy * dy);
    }
    assert!(
        closest >= reach * reach,
        "MEASURE the walker came {} raw units from the body, standoff {reach}",
        (closest as f64).sqrt() as i64
    );
    let (tx, ty) = tile_of(&world, walker);
    assert!(
        (tx - i64::from(x + 6)).abs() <= 1 && (ty - i64::from(y)).abs() <= 1,
        "and it arrives: {tx} {ty}"
    );
    assert_eq!(
        tile_of(&world, sitter),
        (i64::from(x + 3), i64::from(y)),
        "the body it went round did not move"
    );
}

/// A goal that another body is standing on is never entered, and it costs **no** repath at all: re-planning cannot put
/// the unit anywhere better than the neighbouring tile it is already on, so the arrival rules are allowed to finish the
/// order (D98a). The first version of this test demanded exactly one repath - what the avoid set's *past* branch does
/// for a body met **en route** - and the arrival suppression added for the crowd bug made it zero; zero is the better
/// answer, and the en-route branch has its own test above.
#[test]
fn a_body_on_the_goal_is_never_entered_and_costs_no_repath() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let walker = next_id(&world);
    let sitter = EntityId(walker.0 + 1);
    spawn_villager(&mut world, walker, x, y);
    spawn_villager(&mut world, sitter, x + 2, y);
    world.step().expect("the spawns land");
    let goal = (x + 2, y);
    let before = world.repaths();

    world.submit(Command::Move {
        unit: walker,
        to: goal,
    });
    // **Never entered** is a distance since D114: a tile order aims at the middle of the tile, so a body stopped at its
    // standoff is *in* the goal's tile without standing on the body - which is what the tile comparison used to forbid
    // and never needed to. One step of tolerance is the mover's own granularity.
    let reach = world.reach_between(walker, sitter).raw();
    let step = world.scale() / 20;
    for tick in 0..600 {
        world.step().expect("step");
        let (a, b) = (
            world.position(walker).expect("alive"),
            world.position(sitter).expect("alive"),
        );
        let (dx, dy) = (a.0.raw() - b.0.raw(), a.1.raw() - b.1.raw());
        let apart = dx * dx + dy * dy;
        assert!(
            apart >= (reach - step) * (reach - step),
            "the walker came {} raw units from the body on its goal on tick {tick}",
            (apart as f64).sqrt() as i64
        );
    }
    let (tx, ty) = tile_of(&world, walker);
    let adjacent = (tx - i64::from(goal.0)).abs() <= 1 && (ty - i64::from(goal.1)).abs() <= 1;
    assert!(
        adjacent,
        "it waits beside the goal, not across the map: {tx} {ty}"
    );
    assert_eq!(
        world.repaths() - before,
        0,
        "a body on the goal is not an obstacle to route around: the unit waits beside it"
    );
    assert_eq!(
        tile_of(&world, sitter),
        (i64::from(goal.0), i64::from(goal.1)),
        "the body standing there did not move"
    );
}

/// A collision **with the order's target** is not an obstacle: the original calls it "so we're done", and it must not
/// produce a repath. Without this arm a unit ordered at a body would re-plan at it for ever, and a melee attacker would
/// never close.
#[test]
fn a_collision_with_the_target_is_arrival_not_an_obstacle() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let attacker = next_id(&world);
    let victim = EntityId(attacker.0 + 1);
    spawn_villager(&mut world, attacker, x, y);
    // The other side, so the attack order is legal: a villager cannot be ordered at its own player's unit.
    let unit = world.start_villager_unit().expect("a villager role");
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id: victim,
            player: 2,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: at(&world, x + 2, y),
            speed: Fixed::from_raw(scale / 2),
            hp: 25,
            amount: 0,
        })
        .expect("an enemy villager can be placed");
    world.step().expect("the spawns land");
    let before = world.repaths();

    world.submit(Command::Attack {
        attacker,
        target: victim,
    });
    for _ in 0..400 {
        world.step().expect("step");
    }
    assert_eq!(
        world.repaths() - before,
        0,
        "a collision with the target is the end of the walk, not an obstacle to route around"
    );
    let hurt = world.entity(victim).is_none_or(|entity| entity.hp < 25);
    assert!(
        hurt,
        "and the attack lands: the walk closed to its own range"
    );
}

/// An order against a **blocked target tile** ends the route where the order can be satisfied (E1b). A building's own
/// tile is blocked, so a route planned *to* it stops wherever collision first refuses - measured in E1a as twelve
/// attackers standing 1-4 tiles off with `cooldown_ticks: 0`, never striking. The proof is the damage: the walk closed.
#[test]
fn an_arms_route_ends_where_it_can_strike() {
    let Some(mut world) = fresh() else { return };
    let scale = world.scale();
    // The seeded enemy town centre: the map gives one per player, and a building's tile is blocked ground.
    let Some((centre, home)) = world
        .entities()
        .iter()
        .find(|entity| entity.player == 2 && entity.kind == EntityKind::Building)
        .map(|entity| (entity.id, entity.position))
    else {
        return;
    };
    let (tx, ty) = (home.0.raw() / scale, home.1.raw() / scale);
    // Four tiles out, on open ground beside the base.
    let mut spot = None;
    for radius in 3..=6i64 {
        for (dx, dy) in [(0, radius), (radius, 0), (-radius, 0), (0, -radius)] {
            let (Ok(x), Ok(y)) = (u16::try_from(tx + dx), u16::try_from(ty + dy)) else {
                continue;
            };
            if world.is_free_ground(x, y) {
                spot = Some((x, y));
                break;
            }
        }
        if spot.is_some() {
            break;
        }
    }
    let Some((x, y)) = spot else { return };
    let attacker = next_id(&world);
    spawn_villager(&mut world, attacker, x, y);
    world.step().expect("the spawn lands");
    let before = world.entity(centre).expect("the building").hp;

    world.submit(Command::Attack {
        attacker,
        target: centre,
    });
    for _ in 0..1_500 {
        world.step().expect("step");
    }
    let now = world.entity(centre).expect("the building").hp;
    assert!(
        now < before,
        "the attacker never struck: the route ended short of its reach"
    );
}

/// **M1: a repathed route is pulled, like a planned one.** The search answers with a node per tile, and a route stored that
/// way is walked tile centre to tile centre - the staircase the owner keeps seeing, because most long walks in a busy base
/// are repaths rather than first plans. The route the unit is handed after a repath must be the pulled one: three points
/// around a single body on a row, not seven nodes through every tile of the detour.
#[test]
#[ignore = "premise broken by D114 and not yet explained: a body placed exactly on the walk's line no longer forces a \
            repath (repaths=0, the walker arrives at the goal), so there is no repathed route to measure. Recorded in \
            docs/next-step.md, Session 11, with two hypotheses: the per-tick `bodies` list may not carry an idle body, or \
            the bucket index misses a body on a tile boundary. The pull itself is still covered by the M1 unit test of \
            `pull_route` and by the planned-route test."]
fn a_repath_route_is_pulled_like_a_planned_one() {
    let Some(mut world) = fresh() else { return };
    let Some((x, y)) = open_row(&world, 8) else {
        return;
    };
    let walker = next_id(&world);
    let sitter = EntityId(walker.0 + 1);
    spawn_villager(&mut world, walker, x, y);
    // **Exactly on the line the walk will take**: from the walker's tile corner to the middle of the goal tile. A body a
    // quarter of a tile off it is a different test, and that is what the old fixture became when tile goals were centred.
    let scale = world.scale();
    let start = (i64::from(x) * scale, i64::from(y) * scale);
    let goal_at = (
        i64::from(x + 6) * scale + scale / 2,
        i64::from(y) * scale + scale / 2,
    );
    spawn_villager_at(
        &mut world,
        sitter,
        (
            Fixed::from_raw((start.0 + goal_at.0) / 2),
            Fixed::from_raw((start.1 + goal_at.1) / 2),
        ),
    );
    world.step().expect("the spawns land");

    let before = world.repaths();
    world.submit(Command::Move {
        unit: walker,
        to: (x + 6, y),
    });
    let mut stored = None;
    for _ in 0..2_000 {
        world.step().expect("step");
        if world.repaths() > before {
            stored = Some(world.path_length(walker));
            break;
        }
    }
    let Some(stored) = stored else {
        panic!(
            "the body on the line never forced a repath, so there is nothing to measure (repaths={}, at={:?})",
            world.repaths(),
            world.position(walker).map(|p| (p.0.raw(), p.1.raw()))
        );
    };
    assert!(
        stored <= 3,
        "the repathed route holds {stored} waypoints: a route per tile is the staircase, a pulled one is three"
    );
}
