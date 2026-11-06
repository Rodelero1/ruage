//! **The farm ring.** The owner's note (2026-11-04):
//!
//!   "Houses are built too close to the town center, ideally, mill should be built 2 farms away from the town center, then
//!    the it should be possible to build farms around town center and the around the mill wight next to them. And houses and
//!    othrer buildings should be constructed in a way to allow such farm placement"
//!
//! Three claims, in the owner's order: a farm goes in the ring beside the centre, the mill stands two farm widths out, and a
//! house stays out of the ring so the plots have room.
//!
//! Written before the implementation (D7, SPEC §11).

use ruage_sim::{EntityKind, Runner, Scenario, World};

mod common;

fn fresh() -> Option<World> {
    common::fresh_research()
}

fn tile_of(world: &World, id: ruage_sim::EntityId) -> (u16, u16) {
    world
        .position(id)
        .map(|position| world.tile_of_position(position))
        .expect("a position")
}

fn centre(world: &World) -> (u16, u16) {
    world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .map(|entity| tile_of(world, entity.id))
        .expect("a town centre")
}

fn distance(a: (u16, u16), b: (u16, u16)) -> i64 {
    let dx = i64::from(a.0) - i64::from(b.0);
    let dy = i64::from(a.1) - i64::from(b.1);
    dx * dx + dy * dy
}

fn built(world: &World, role: &str) -> Vec<(u16, u16)> {
    let unit = world.role_unit(role).expect("a role");
    world
        .entities()
        .iter()
        .filter(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .filter(|entity| entity.unit.is_some_and(|id| world.is_family(id, unit)))
        .map(|entity| tile_of(world, entity.id))
        .collect()
}

fn run(scenario_text: &str, ticks: u64) -> Option<World> {
    let mut world = fresh()?;
    let scenario: Scenario = toml::from_str(scenario_text).expect("a scenario the format can read");
    let mut runner = Runner::new(&scenario, 20).expect("a runner");
    for _ in 0..ticks {
        runner.due(&mut world);
        world.step().expect("stepping");
    }
    Some(world)
}

/// **A farm goes in the ring beside the town centre**, not beside the berries: within the ring's reach of the centre.
#[test]
fn a_farm_is_built_beside_the_town_centre() {
    let Some(world) = run(
        r#"
schema = 2
duration_seconds = 90.0

[[rule]]
player = 1
cooldown_seconds = 3.0
do = "build"
role = "farm"
where = "food"
"#,
        1800,
    ) else {
        return;
    };
    let farms = built(&world, "farm");
    assert!(!farms.is_empty(), "no farm was built at all");
    let ring = world.farm_ring_end();
    let centre = centre(&world);
    let mills = built(&world, "mill");
    for farm in farms {
        let d = distance(farm, centre);
        assert!(
            d <= i64::from(ring + 3) * i64::from(ring + 3),
            "a farm stands {d} squared tiles from {centre:?}, outside the ring (end {ring}): {farm:?}"
        );
        // **Right next to the centre or right next to a mill**, the owner's "around the town centre and around the mill tight
        // next to them": a plot three tiles from the nearest thing it could be attached to is a plot in the middle of a field.
        let nearest = mills
            .iter()
            .map(|mill| distance(farm, *mill))
            .chain(std::iter::once(d))
            .min()
            .unwrap_or(d);
        assert!(
            nearest <= i64::from(ring + 3) * i64::from(ring + 3),
            "a farm stands {nearest} squared tiles from the nearest drop building, outside its ring: {farm:?}"
        );
    }
}

/// **The mill stands two farm widths out.** The owner's number: the ring is `farm_spacing` plots wide, so a mill nearer the
/// centre than the ring's inner half is a mill the farms will crowd.
#[test]
fn the_mill_stands_two_farm_widths_from_the_town_centre() {
    let Some(world) = run(
        r#"
schema = 2
duration_seconds = 90.0

[[rule]]
player = 1
cooldown_seconds = 3.0
do = "build"
role = "mill"
where = "food"
"#,
        1800,
    ) else {
        return;
    };
    let mills = built(&world, "mill");
    assert!(!mills.is_empty(), "no mill was built at all");
    let ring = world.farm_ring_end();
    let centre = centre(&world);
    for mill in mills {
        let d = distance(mill, centre);
        // The mill may stand in the ring's outer half; `ring - 2` is that half's inner edge.
        let near = i64::from(ring.saturating_sub(2));
        assert!(
            d >= near * near,
            "the mill stands {d} squared tiles from {centre:?}, inside the ring's inner edge ({near}): {mill:?}"
        );
    }
}

/// **A house stays out of the ring.** The plots need the ground next to the centre, and a house in the middle of it is the
/// owner's complaint in as many words.
#[test]
fn a_house_is_not_built_in_the_farm_ring() {
    let Some(world) = run(
        r#"
schema = 2
duration_seconds = 90.0

[[rule]]
player = 1
cooldown_seconds = 3.0
do = "build"
role = "house"
"#,
        1800,
    ) else {
        return;
    };
    let houses = built(&world, "house");
    assert!(!houses.is_empty(), "no house was built at all");
    let ring = world.farm_ring_end();
    let centre = centre(&world);
    for house in houses {
        let d = distance(house, centre);
        assert!(
            d >= i64::from(ring) * i64::from(ring),
            "a house stands {d} squared tiles from {centre:?}, inside the farm ring (end {ring}): {house:?}"
        );
    }
}
