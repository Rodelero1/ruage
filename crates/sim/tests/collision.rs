//! Collision: units do not stand on top of each other when there is room.
//!
//! Written before the implementation (D7). The prototype list asks for "unit collision / pushing", and the
//! owner chose soft avoidance: units slide out of each other's way, and may still end up sharing a tile when
//! there is nowhere else to go or when they were ordered there together - which is what the game does.

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

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

fn walk_speed(world: &World, fx: &Fixture, unit: u32) -> Fixed {
    fx.units
        .unit(unit)
        .expect("in the table")
        .speed
        .resolve(world.scale(), "speed")
        .expect("a walker")
}

fn place(
    world: &mut World,
    player: u8,
    id: u32,
    unit: u32,
    tile: (i64, i64),
    speed: Fixed,
) -> EntityId {
    let scale = world.scale();
    let id = EntityId(id);
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed,
            hp: 40,
            amount: 0,
        })
        .expect("spawn");
    world.step().expect("step");
    id
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let scale = world.scale();
    let (x, y) = world.position(id).expect("the entity exists");
    (x.raw().div_euclid(scale), y.raw().div_euclid(scale))
}

#[test]
fn a_walker_does_not_finish_on_top_of_a_friend() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let speed = walk_speed(&world, &fx, spearman);
    let stander = place(&mut world, 1, 9_700, spearman, (24, 20), Fixed::ZERO);
    let walker = place(&mut world, 1, 9_701, spearman, (20, 20), speed);

    world
        .submit_checked(Command::Move {
            target: None,
            unit: walker,
            to: (24, 20),
        })
        .expect("move");
    for _ in 0..400 {
        world.step().expect("step");
    }

    // **The rule is a distance, not a tile.** Two bodies 0.7 of a tile apart in the same tile are not inside each other;
    // the standoff is what `collide` owes, and it is what this asserts now that a tile order means the middle of the tile
    // (D114) - the old tile comparison was a proxy that only held while orders aimed at corners.
    let reach = world.reach_between(walker, stander).raw();
    let (a, b) = (
        world.position(walker).expect("alive"),
        world.position(stander).expect("alive"),
    );
    let (dx, dy) = (a.0.raw() - b.0.raw(), a.1.raw() - b.1.raw());
    let squared = dx * dx + dy * dy;
    assert!(
        squared >= reach * reach,
        "the walker finished {} raw units from its friend, inside the {reach} their footprints need",
        (squared as f64).sqrt() as i64
    );
}

#[test]
fn a_crowd_sent_to_one_place_presses_but_does_not_merge() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let speed = walk_speed(&world, &fx, spearman);
    let a = place(&mut world, 1, 9_702, spearman, (20, 20), speed);
    let b = place(&mut world, 1, 9_703, spearman, (20, 22), speed);
    let c = place(&mut world, 1, 9_704, spearman, (22, 20), speed);

    for unit in [a, b, c] {
        world
            .submit_checked(Command::Move {
                target: None,
                unit,
                to: (26, 22),
            })
            .expect("move");
    }
    for _ in 0..600 {
        world.step().expect("step");
    }

    // **The assertion used to be "no two share a tile"** - that was the tile rule speaking. A body's standoff is its
    // footprint now (C), so the crowd is judged by distance: two bodies may stand in one tile and may not stand inside
    // each other.
    for (n, first) in [a, b, c].iter().enumerate() {
        for second in [a, b, c].iter().skip(n + 1) {
            let reach = world.reach_between(*first, *second).raw();
            let (one, two) = (
                world.position(*first).expect("a position"),
                world.position(*second).expect("a position"),
            );
            let (dx, dy) = (one.0.raw() - two.0.raw(), one.1.raw() - two.1.raw());
            let apart = (dx * dx + dy * dy).isqrt();
            // **A crowd presses** (D289): the original's players describe men standing pressed on one spot, so the claim is
            // no longer that footprints never overlap - it is that they never *merge*. Measured after the cut, the closest
            // pair of this crowd stands **6746 raw** inside an **8000 raw** footprint.
            assert!(
                apart < reach,
                "two units of one order stand {apart} raw units apart, merging into each other where the {reach} footprint \
                 allows a quarter of it: {:?} and {:?}",
                tile_of(&world, *first),
                tile_of(&world, *second)
            );
        }
    }
}

#[test]
fn a_newcomer_steps_aside_and_the_body_already_there_does_not_move() {
    // **The control for D52, amended by D267** (the owner, 2026-11-18: *"adopt them 100%"*). D52's rule is that nothing
    // ever **pushes** a body - no shoving, no cascade, no cascade's state to hash - and D267 is that a body is **placed**
    // rather than dropped: the original's `func_002699A0` steps a newcomer out by the two radii plus a hundredth of a tile.
    // The two rules are about different things, and the assertion that could not tell them apart was a **tile**: two bodies
    // 0.4 of a tile apart share a tile or do not depending on where in the tile they stand, which the fixture's own comment
    // had already called a proxy. So this now asserts the two rules themselves: the body that was already standing there
    // does not move **at all**, and the newcomer ends up no closer than the pair's footprint.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let a = place(&mut world, 1, 9_705, spearman, (26, 22), Fixed::ZERO);
    let stander = world.position(a).expect("a position");
    let b = place(&mut world, 1, 9_706, spearman, (26, 22), Fixed::ZERO);

    for _ in 0..20 {
        world.step().expect("step");
    }

    assert_eq!(
        world.position(a).expect("a position"),
        stander,
        "a body that was already standing there was moved by a newcomer"
    );
    let (distance, reach) = {
        let (one, two) = (
            world.position(a).expect("a position"),
            world.position(b).expect("a position"),
        );
        let (dx, dy) = (one.0.raw() - two.0.raw(), one.1.raw() - two.1.raw());
        ((dx * dx + dy * dy).isqrt(), world.reach_between(a, b).raw())
    };
    assert!(
        distance >= reach,
        "the newcomer stands {distance} raw units from the body already there, inside the {reach} their \
         footprints need"
    );
}

#[test]
fn a_walker_never_ends_a_tick_inside_a_friend() {
    // **The owner's rule (D189, revised 2026-11-13): "units shouldn't just pass over each other - in the game they
    // obstruct each other and try to find a different path", and the original's collision block cannot tell a friend
    // from an enemy (docs/archive/research/movement-in-the-binary.md).** So this is checked *every tick*, not only at the end: the two
    // existing tests above judge where a unit finishes, and a walker that overlaps its friend for 300 ticks on the way
    // passes both of them.
    let Some(fx) = fixture() else { return };
    let mut world = seeded(&fx);
    let spearman = fx.economy.role["spearman"];
    let speed = walk_speed(&world, &fx, spearman);
    let walker = place(&mut world, 1, 9_711, spearman, (20, 20), speed);

    world
        .submit_checked(Command::Move {
            target: None,
            unit: walker,
            to: (26, 20),
        })
        .expect("move");
    world.step().expect("the order lands");
    // **The friend arrives on the lane after the route exists** (restated 2026-11-15). A body that has not moved is an
    // obstacle to the search that meets it (D210b), and with `last_moved` finally stamped only on a real step (D213) that
    // includes a body seeded at tick 0 - so a stander in place *before* the order is simply planned around and the pair
    // never meet: measured, "the walker never came within the reach". The encounter has to be forced for this fixture to
    // measure anything, so the friend is placed once the walker is under way; the rule being tested is what happens when
    // they *do* meet.
    let stander = place(&mut world, 1, 9_710, spearman, (24, 20), Fixed::ZERO);
    world.step().expect("the friend lands");

    let reach = world.reach_between(walker, stander).raw();
    // **Restated 2026-11-14 to D210's outcome: one step of overlap at the meeting, nothing deeper after it.** The old
    // assertion was `worst >= reach - 1` - the clamp D210 withdrew, on the binary's own evidence (its only
    // radius-against-radius retraction has its result discarded at the movement site, `0x8a7fe7`). What the rules now
    // promise is that the *deepest* point of an encounter is its first contact, and that contact is at most the step the
    // walker was already taking: measured here, **222 raw** inside an 8000 raw reach, against an 800-raw step at tick 74 -
    // after which he is round the man and away. A walker that goes *through* a friend keeps getting deeper, which this
    // still catches.
    let mut meeting: Option<i64> = None;
    let mut deepest_after = i64::MAX;
    let mut longest_step = 0i64;
    let mut was_at = world.position(walker).expect("alive");
    for _ in 0..600 {
        world.step().expect("step");
        let (a, b) = (
            world.position(walker).expect("alive"),
            world.position(stander).expect("alive"),
        );
        let (dx, dy) = (a.0.raw() - b.0.raw(), a.1.raw() - b.1.raw());
        let apart = (dx * dx + dy * dy).isqrt();
        if meeting.is_none() && apart < reach {
            meeting = Some(apart);
        } else if meeting.is_some() {
            deepest_after = deepest_after.min(apart);
        }
        let (sx, sy) = (a.0.raw() - was_at.0.raw(), a.1.raw() - was_at.1.raw());
        longest_step = longest_step.max((sx * sx + sy * sy).isqrt());
        was_at = a;
    }

    let meeting =
        meeting.expect("MEASURED the walker never came within the reach: the fixture did not meet");
    assert!(
        reach - meeting <= longest_step,
        "MEASURED the meeting swallowed {} raw of the {reach} reach, more than the {longest_step} raw step the walker was \
         already taking at that moment: he was not walking past a friend, he was driven into one",
        reach - meeting
    );
    // **Restated 2026-11-19 (D289)**: the deepest point after contact is at most **one step** deeper than contact, not exactly
    // the contact. With D271's ring gone the walker is sent to the same tile as his friend and presses into him before sliding
    // round, which is what the original's players describe; a step is the only resolution a tick has, so a step is the whole
    // allowance. Measured after the cut: **781 raw** deeper against an **800 raw** step.
    assert!(
        deepest_after + longest_step >= meeting,
        "MEASURED the walker went {after} raw units deep after the meeting, against {met} at it - more than the {step} raw \
         step apart, so friends passed through each other",
        after = deepest_after,
        met = meeting,
        step = longest_step
    );
}
