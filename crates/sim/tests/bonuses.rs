//! **The master list (R6, D234)**: one resolved bonus set per player, and every consumer reads it.
//!
//! Five consumers re-walk three effect tables today - the civ's tech tree, its team bonus, and the player's researched
//! techs - and each one decides for itself which effects belong to a player. That is why a bonus can be half-applied for
//! years: the farm line and the construction rate were each "one resource row somebody forgot". These fixtures pin the
//! answers the table must give, so moving the walk into a table cannot move them, and the one that bites is the **second
//! research**: a table built when the first tech lands and not rebuilt on the next *looks* right until a player takes two
//! techs on the same line.
//!
//! Written before the refactor (D7, SPEC §11).

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: ruage_data::Map,
    economy: ruage_data::Economy,
    resources: ruage_data::Resources,
    units: ruage_data::Units,
    techs: ruage_data::Techs,
    effects: ruage_data::Effects,
    civs: ruage_data::Civs,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        eprintln!(
            "skipping: {} is not present (generated data, D8)",
            units.display()
        );
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
        techs: ruage_data::load_techs_file(&repo("config/gamedata/techs.toml"))
            .expect("techs.toml"),
        effects: ruage_data::load_effects_file(&repo("config/gamedata/effects.toml"))
            .expect("effects.toml"),
        civs: ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).expect("civs.toml"),
    })
}

/// A world whose player 1 is `civ`, with everything the bonuses read loaded **in the order the loader demands**: seeding,
/// the stockpile, the research tables, then the civ row.
fn seeded_as_civ(fx: &Fixture, civ: &str) -> World {
    let mut economy = fx.economy.clone();
    economy.start.civs = vec![civ.to_owned()];
    economy.start.stock = [
        ("food".to_owned(), 4_000),
        ("wood".to_owned(), 4_000),
        ("gold".to_owned(), 2_000),
    ]
    .into_iter()
    .collect();
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world
        .load_research(&fx.techs, &fx.effects)
        .expect("research tables");
    world.load_civs(&fx.civs).expect("the civ table");
    world
}

/// A finished farm for player 1, and the plot's food budget: `amount_of` on the farm entity.
fn a_finished_farm(world: &mut World) -> f64 {
    let villager = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .expect("a villager")
        .id;
    let farm = world.role_unit("farm").expect("a farm role");
    // Somewhere the town centre's yard is not using: the first tile whose whole footprint fits.
    let at = (0..40u16)
        .flat_map(|x| (0..40u16).map(move |y| (x, y)))
        .find(|(x, y)| world.site_fits(1, farm, (*x, *y)))
        .expect("a free tile for a farm");
    world
        .submit_checked(Command::Build {
            villager,
            unit: farm,
            at,
        })
        .expect("a farm site");
    let mut ticks = 0;
    let site = loop {
        world.step().expect("step");
        ticks += 1;
        assert!(ticks < 20_000, "the farm never rose");
        if let Some(entity) = world
            .entities()
            .iter()
            .find(|entity| entity.player == 1 && entity.unit == Some(farm))
            .filter(|entity| entity.construction.is_none())
        {
            break entity.id;
        }
    };
    world.entity(site).expect("the farm").amount.raw() as f64 / world.scale() as f64
}

const HORSE_COLLAR: u32 = 14;
const HEAVY_PLOW: u32 = 13;
const CROP_ROTATION: u32 = 12;

/// **The second research reaches the plot.** The base is the civ's own row (175 for a Briton), and each farm tech is a
/// kind-1 `add` to that line: 175, then 250, then 375. A table filled when Horse Collar lands and never rebuilt answers
/// **250** to the second question - which is the bug this fixture exists to catch.
#[test]
fn every_farm_tech_reaches_the_plot_and_the_second_one_counts_too() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded_as_civ(&fx, "British");
    assert_eq!(
        a_finished_farm(&mut world),
        175.0,
        "a Briton's plot holds the civ row's 175"
    );

    world.grant_tech(1, HORSE_COLLAR).expect("Horse Collar");
    assert_eq!(
        a_finished_farm(&mut world),
        250.0,
        "the first farm tech adds its 75"
    );

    world.grant_tech(1, HEAVY_PLOW).expect("Heavy Plow");
    assert_eq!(
        a_finished_farm(&mut world),
        375.0,
        "the second farm tech adds its 125 on top, which a once-built table loses"
    );

    world.grant_tech(1, CROP_ROTATION).expect("Crop Rotation");
    assert_eq!(
        a_finished_farm(&mut world),
        550.0,
        "and the third adds its 175"
    );
}

/// **A team bonus and a research answer the same table.** The Chinese team bonus is a resource-36 `add` of 45 that nothing
/// researches - a property of who the player is - so a Chinese plot starts at 220 and a Horse Collar adds 75 on top.
#[test]
fn a_team_bonus_and_a_research_are_one_answer() {
    let Some(fx) = fixture() else { return };
    let mut world = seeded_as_civ(&fx, "Chinese");
    assert_eq!(
        a_finished_farm(&mut world),
        220.0,
        "175 from the Chinese row plus the team bonus's 45"
    );

    world.grant_tech(1, HORSE_COLLAR).expect("Horse Collar");
    assert_eq!(
        a_finished_farm(&mut world),
        295.0,
        "the civ's 220 plus the tech's 75, from one list"
    );
}

/// **Two consumers, one player, one answer.** A Slav barracks supports five more units than a British one - that is the
/// Slavs' **team bonus** naming `population`, read by `modifiers_for` - while a British player's *tech tree* bars the Hussar
/// (pinned by `research::a_barred_tech_is_refused_by_the_civilisation`). Both answers come from the effects that belong to
/// that player; both must survive the move into one table, and the population half is the one this fixture can build
/// without a Feudal Age.
#[test]
fn a_team_bonus_and_a_tech_bar_are_answers_about_the_same_player() {
    let Some(fx) = fixture() else { return };
    let mut british = seeded_as_civ(&fx, "British");
    let mut slav = seeded_as_civ(&fx, "Slavs");

    spawn_barracks(&mut british, 1);
    spawn_barracks(&mut slav, 1);
    let british_cap = british.population_cap(1);
    let slav_cap = slav.population_cap(1);
    assert_eq!(
        slav_cap,
        british_cap + 5,
        "a Slav barracks supports five more: {slav_cap} against {british_cap}"
    );
}

fn spawn_barracks(world: &mut World, player: u8) -> EntityId {
    let role = world.role_unit("barracks").expect("a barracks role");
    let at = (0..40u16)
        .flat_map(|x| (0..40u16).map(move |y| (x, y)))
        .find(|(x, y)| world.site_fits(player, role, (*x, *y)))
        .expect("a free tile for a barracks");
    let id = EntityId(9_000 + u32::from(player));
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Building,
            unit: Some(role),
            position: (
                Fixed::from_raw(i64::from(at.0) * scale),
                Fixed::from_raw(i64::from(at.1) * scale),
            ),
            speed: Fixed::ZERO,
            hp: 1_200,
            amount: 0,
        })
        .expect("a barracks");
    world.step().expect("the barracks stands");
    id
}
