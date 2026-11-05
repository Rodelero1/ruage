//! **The card offers what the age allows.** The owner's note (2026-11-04): "techs, upgrades and units should be shown based
//! on age the player is in. Imperial Age upgrades should not show in Feudal Age."
//!
//! The rule is the refusal's *kind*, not a list of techs: a refusal that says "not available", "not upgraded", "your civ
//! bars it", "you need the age's buildings" or "a prerequisite is missing" is the age talking, and the item is hidden. A
//! refusal about money is the player's, and it stays on the card, greyed, because it is something they can act on.
//!
//! Written before the implementation (D7, SPEC §11).

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn fresh() -> Option<World> {
    if !repo("config/gamedata/units.toml").exists() {
        return None;
    }
    let resources = ruage_data::load_resources_file(&repo("config/resources.toml")).ok()?;
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).ok()?;
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).ok()?;
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).ok()?;
    let units = ruage_data::load_units_file(&repo("config/gamedata/units.toml")).ok()?;
    let combat = ruage_data::load_combat_file(&repo("config/combat.toml")).ok()?;
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    world.load_combat(&combat);
    // The research tables are what make an age mean anything: without them `unit_available` has nothing to say and every
    // item looks available.
    let techs = ruage_data::load_techs_file(&repo("config/gamedata/techs.toml")).ok()?;
    let effects = ruage_data::load_effects_file(&repo("config/gamedata/effects.toml")).ok()?;
    world.load_research(&techs, &effects).ok()?;
    Some(world)
}

fn next_id(world: &World) -> EntityId {
    EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1)
}

/// A finished barracks, standing on ground the town centre's yard is not using.
fn barracks(world: &mut World) -> EntityId {
    let scale = world.scale();
    let unit = world.role_unit("barracks").expect("a barracks role");
    let id = next_id(world);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(unit),
            position: (Fixed::from_raw(24 * scale), Fixed::from_raw(24 * scale)),
            speed: Fixed::ZERO,
            hp: 1200,
            amount: 0,
        })
        .expect("a barracks");
    world.step().expect("one tick");
    id
}

fn names(world: &World, building: EntityId) -> Vec<String> {
    world
        .card(1, building)
        .into_iter()
        .map(|item| item.name)
        .collect()
}

fn spear_names(world: &World) -> Vec<String> {
    world
        .entities()
        .iter()
        .filter(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .filter_map(|entity| entity.unit)
        .filter_map(|unit| world.role_name(unit))
        .collect()
}

/// **A Dark Age barracks offers no spearman; a Feudal one does.** The item is hidden rather than refused, which is the
/// difference the owner asked for: a card of things the age forbids is a card of choices the player does not have.
#[test]
fn a_dark_age_card_hides_the_spear_and_a_feudal_one_shows_it() {
    let Some(mut world) = fresh() else { return };
    let building = barracks(&mut world);
    let dark = names(&world, building);
    let spear = world
        .role_unit("spearman")
        .and_then(|unit| world.role_name(unit))
        .unwrap_or_else(|| "spearman".to_owned());
    assert!(
        !dark.iter().any(|name| name == &spear),
        "a Dark Age barracks still offers a {spear}: {dark:?}"
    );

    // Reach the Feudal Age: grant the age's own tech, as `tech_tree.rs` does (the honest `Command::Research` route would
    // need two Dark Age buildings first, which is a different test's subject).
    let feudal = world
        .tech_role("feudal_age")
        .expect("the pack names its feudal age");
    world.grant_tech(1, feudal).expect("the Feudal Age");

    let feudal = names(&world, building);
    assert!(
        feudal.iter().any(|name| name == &spear),
        "a Feudal barracks does not offer a {spear}: {feudal:?} (spears: {:?})",
        spear_names(&world)
    );
}

/// **A refusal about money keeps its button.** The card is not a list of what is possible, it is a list of what the player
/// may try: an unaffordable villager is still offered, with the engine's sentence on it.
#[test]
fn an_unaffordable_item_stays_on_the_card() {
    let Some(mut world) = fresh() else { return };
    let centre = world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Building)
        .map(|entity| entity.id)
        .expect("a town centre");
    let villager = world.role_unit("villager").expect("a villager role");
    // Spend the food: four villagers at the pack's price empty the starting stock without filling the queue.
    for _ in 0..4 {
        world
            .submit_checked(Command::Train {
                building: centre,
                unit: villager,
            })
            .expect("the queue takes four");
    }
    world.step().expect("one tick");
    let card = world.card(1, centre);
    let offered = card.iter().find(|item| item.name == "villager");
    let offered = offered.unwrap_or_else(|| {
        panic!(
            "an unaffordable villager left the card: {:?}",
            card.iter()
                .map(|item| (&item.name, &item.refusal))
                .collect::<Vec<_>>()
        )
    });
    let refusal = offered
        .refusal
        .as_ref()
        .expect("nothing on the town centre's card is refused, so the money case is not tested");
    assert!(
        refusal.to_lowercase().contains("not enough") || refusal.to_lowercase().contains("queue"),
        "the refusal on an unaffordable villager is not about the money or the queue: {refusal}"
    );
}

/// **A unit is named by its own role, not by a later tier of its family** (the owner's report, 2026-11-04: *"player trains
/// champions in dark age and has hussar instead of scout cavalry"*).
///
/// `role_name` finds a role by **family root**, because an age-up rewrites a building's id and the exact id then has no role
/// (a town centre becomes 71 and the panel said "unit 71"). But the fallback had no tie-break: the role table is a map
/// ordered by name, so `find` returned the **alphabetically first** member of the family - and for an upgrade line that is
/// the *top* tier. A Dark Age militia (74) was labelled **champion** (567), and the opening scout (448) was labelled
/// **hussar** (441). The simulation always had the right unit; only the name was wrong, which is why every existing test
/// passed: they compared `role_name` against `role_name`.
#[test]
fn a_unit_is_named_by_its_own_role_not_by_a_later_tier() {
    let Some(world) = fresh() else { return };
    for role in ["militia", "scout_cavalry", "spearman", "knight", "villager"] {
        let Some(unit) = world.role_unit(role) else {
            continue;
        };
        assert_eq!(
            world.role_name(unit).as_deref(),
            Some(role),
            "unit {unit} is the pack's `{role}` and must be named that, not `{:?}`",
            world.role_name(unit)
        );
    }
    // **And the case the family fallback exists for still works**: an age-up replaces a building's id, and the panel has to
    // keep calling it a town centre. 71 is the Feudal Age town centre.
    if let Some(unit) = world.role_unit("town_center") {
        assert_eq!(
            world.role_name(71).as_deref(),
            Some("town_center"),
            "the Feudal town centre (71) is a town centre, not {}",
            world.role_name(71).unwrap_or_default()
        );
        // The exact id still names itself.
        assert_eq!(world.role_name(unit).as_deref(), Some("town_center"));
    }
}
