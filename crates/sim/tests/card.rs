//! The command card: what the selected thing could do, priced and named, from content.
//!
//! The owner's D1 answer is that units, buildings, techs, descriptions, AI and civs must all be **moddable**, so the card
//! cannot be a hand-written list of buttons in the window: it is a query over the same tables the engine already validates
//! orders against, and every name on it comes from the pack's own vocabulary (`[role]`, `[tech_role]`).
//!
//! Written before the implementation (D7, SPEC §11).

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{CardTarget, Command, EntityId, EntityKind, World};

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
    let techs = ruage_data::load_techs_file(&repo("config/gamedata/techs.toml")).ok()?;
    let effects = ruage_data::load_effects_file(&repo("config/gamedata/effects.toml")).ok()?;
    let civs = ruage_data::load_civs_file(&repo("config/gamedata/civs.toml")).ok()?;
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    world.load_combat(&combat);
    world.load_research(&techs, &effects).ok()?;
    world.load_civs(&civs).ok()?;
    Some(world)
}

fn town_centre(world: &World, player: u8) -> EntityId {
    world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == player
                && entity.kind == EntityKind::Building
                && entity.unit == Some(109)
        })
        .map(|entity| entity.id)
        .expect("a town centre")
}

fn next_id(world: &World) -> EntityId {
    EntityId(world.entities().iter().map(|e| e.id.0).max().unwrap_or(0) + 1)
}

fn a_finished_barracks(world: &mut World) -> EntityId {
    let role = world.role_unit("barracks").expect("a barracks role");
    let scale = world.scale();
    let id = next_id(world);
    world
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(role),
            position: (Fixed::from_raw(24 * scale), Fixed::from_raw(24 * scale)),
            speed: Fixed::ZERO,
            hp: 1200,
            amount: 0,
        })
        .expect("a barracks");
    world.step().expect("one tick");
    id
}

/// **The town centre's card offers a villager, at the pack's price.** The name is the pack's word for the unit and the price
/// is the pack's cost columns - a card that quoted its own numbers would be a second copy of the content.
#[test]
fn a_town_centre_offers_a_villager() {
    let Some(world) = fresh() else { return };
    let centre = town_centre(&world, 1);
    let villager = world.role_unit("villager").expect("a villager role");
    let food = world.slot_of("food").expect("a food column");

    let card = world.card(1, centre);
    let item = card
        .iter()
        .find(|item| item.target == CardTarget::Unit(villager))
        .expect("a villager on the town centre's card");

    assert_eq!(item.name, "villager", "the pack's word for the unit");
    assert!(item.refusal.is_none(), "a villager is available at once");
    assert!(
        item.costs
            .iter()
            .any(|(slot, amount)| *slot == food && *amount > 0),
        "the villager's price is not the food the pack says: {:?}",
        item.costs
    );
}

/// **A card says why a button would be refused**, in the engine's words - the same `check_train` the order goes through, so a
/// button can never be offered for an order that would be refused.
#[test]
fn a_barracks_card_refuses_the_spear_line_until_it_is_unlocked() {
    let Some(mut world) = fresh() else { return };
    let barracks = a_finished_barracks(&mut world);
    let spearman = world.role_unit("spearman").expect("a spearman role");

    let card = world.card(1, barracks);
    let item = card
        .iter()
        .find(|item| item.target == CardTarget::Unit(spearman))
        .expect("the spear line is on a barracks' card, refused or not");
    assert!(
        item.refusal.is_some(),
        "the spearman is gated in the Dark Age and the card offered it"
    );

    world.grant_tech(1, 101).expect("the Feudal Age");
    let card = world.card(1, barracks);
    let item = card
        .iter()
        .find(|item| item.target == CardTarget::Unit(spearman))
        .expect("the spear line");
    assert!(
        item.refusal.is_none(),
        "after the Feudal Age the spear line is offered: {:?}",
        item.refusal
    );
}

/// **A card is the player's own.** Clicking the enemy's town centre must not offer the enemy's villagers for you to pay for.
#[test]
fn an_enemy_building_has_no_card() {
    let Some(world) = fresh() else { return };
    let enemy = town_centre(&world, 2);
    assert!(
        world.card(1, enemy).is_empty(),
        "the enemy's town centre offered player 1 its own buttons"
    );
}

/// **The techs a building researches are on its card, named from the pack.** A town centre researches the ages; the names
/// come from `[tech_role]`, so a mod that renames an age renames the button.
#[test]
fn a_town_centre_offers_its_ages() {
    let Some(world) = fresh() else { return };
    let centre = town_centre(&world, 1);
    let card = world.card(1, centre);
    let names: Vec<&str> = card
        .iter()
        .filter(|item| matches!(item.target, CardTarget::Tech(_)))
        .map(|item| item.name.as_str())
        .collect();
    assert!(
        names.contains(&"feudal_age"),
        "the town centre does not offer the Feudal Age: {names:?}"
    );
}
