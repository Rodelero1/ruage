//! **A builder works from a tile beside the footprint, so the free ground is the cap** (D186, extended to construction).
//!
//! The owner, on his `house.aoe2spgame`: *"With house there were more villagers than the free space around the house. So they
//! were walking around fruitlessly. With bush probably, too."* That says the game asks the **ground** how many men may work a
//! site, not the player's order - and our build arm has a second way to count a man as working: `gap² <= reach²`, where `reach`
//! is the two bodies' radii (a villager's 0.2 plus a building's), which can reach past the tiles that touch the footprint.
//!
//! What this measures: with sixteen builders ordered onto one house, how many advance its progress in a single tick, against
//! how many tiles there are beside its footprint. More than the tiles means a man is working from beyond the ring the owner is
//! describing, and the cap is not the ground.
//!
//! Written before the change (D7, SPEC §11).

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind};

mod common;

/// **How many men advance one site in a single tick, and how many tiles the site has to stand on.** The count is taken from the
/// progress counter's own step between two ticks - the engine's answer, not a recount of who looks busy.
#[test]
fn no_more_builders_work_a_site_than_it_has_room_for() {
    let Some(mut world) = common::fresh() else {
        return;
    };
    let scale = world.scale();
    let house = world.role_unit("house").expect("a house role");
    let villager = world.role_unit("villager").expect("a villager role");
    // Ground the player has seen: a tower-less corner of the base map, revealed the way the tests do it.
    let (ax, ay) = (18u16, 18u16);
    for dy in 0..6u16 {
        for dx in 0..6u16 {
            common::look_away(&mut world, 1, (ax + dx, ay + dy));
        }
    }
    let builder = world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == 1
                && entity
                    .unit
                    .is_some_and(|unit| world.is_family(unit, villager))
        })
        .map(|entity| entity.id)
        .expect("the opening villager");
    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: house,
            at: (ax + 2, ay + 2),
        })
        .expect("a house site");
    world.step().expect("the site lands");

    let site = world
        .entities()
        .iter()
        .find(|entity| entity.construction.is_some() && entity.unit == Some(house))
        .map(|entity| entity.id)
        .expect("the site");
    let (wide, tall) = world.footprint_of(house).expect("a house has a footprint");

    // Sixteen more builders, ringing the site from four to six tiles out.
    let mut ordered = 0;
    for index in 0..16u32 {
        let angle = f64::from(index) * std::f64::consts::TAU / 16.0;
        let (dx, dy) = (
            (5.0 * angle.cos()).round() as i64,
            (5.0 * angle.sin()).round() as i64,
        );
        let id = EntityId(9_700 + index);
        let at = (i64::from(ax + 2) + dx, i64::from(ay + 2) + dy);
        if world
            .submit_checked(Command::Spawn {
                id,
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
            .is_err()
        {
            continue;
        }
        world.step().expect("the builder lands");
        match world.submit_checked(Command::JoinBuild { villager: id, site }) {
            Ok(()) => ordered += 1,
            Err(error) => eprintln!("JOIN refused: {error}"),
        }
    }
    assert!(ordered >= 8, "only {ordered} builders could be ordered");

    // The tiles beside the footprint: the rectangle grown by one, less the rectangle itself.
    let (wide, tall) = (wide as usize, tall as usize);
    let ring = (wide + 2) * (tall + 2) - wide * tall;

    let mut most = 0u32;
    let mut last = world
        .entity(site)
        .and_then(|entity| entity.construction)
        .map_or(0, |construction| construction.progress_ticks);
    for _ in 0..1_200 {
        world.step().expect("a tick");
        let Some(now) = world
            .entity(site)
            .and_then(|entity| entity.construction)
            .map(|construction| construction.progress_ticks)
        else {
            break; // finished
        };
        let step = now.saturating_sub(last);
        most = most.max(step);
        last = now;
        // Keep the site alive for the whole window: a house is cheap, and a finished one ends the measurement.
        if world
            .entity(site)
            .is_some_and(|entity| entity.construction.is_none())
        {
            break;
        }
    }

    assert!(
        most as usize <= ring,
        "MEASURED {most} builders advancing one house in a single tick, but a {wide}x{tall} footprint has {ring} tiles \
         beside it - a man is working from beyond the ring"
    );
    eprintln!(
        "MEASURED at most {most} builders a tick on a {wide}x{tall} footprint with {ring} tiles beside it"
    );
}
