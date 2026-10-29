//! A herdable changes hands by line of sight (D70, item 5).
//!
//! The manual's words, which are the rule:
//!
//! > "Sheep start the game neutral and unowned. As soon as a sheep enters the line of sight of a unit, the sheep is
//! > under control of that player. You can steal ownership of another player's sheep if it comes within your unit's
//! > line of sight and there are no other players' units near the sheep."
//!
//! So the trigger is a **unit's** line of sight - not a building's, not adjacency - the change is **immediate**, only
//! **herdables** convert, and a tie goes to whoever has more units near the animal.
//!
//! Everything here happens at tile (30, 30), which is fourteen tiles from both starting town centres: no seeded
//! villager can see it, so each test's sight is the sight it placed.

use std::path::PathBuf;

use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};

/// The neutral ground these tests use. Fourteen tiles from (16, 16) and from (47, 47).
const OPEN: (i64, i64) = (30, 30);

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

/// Put an entity on a tile near [`OPEN`], with a real speed so it can walk and its own line of sight.
fn place(world: &mut World, player: u8, unit: u32, dx: i64, dy: i64, hp: i32) -> EntityId {
    let scale = world.scale();
    let id = EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw((OPEN.0 + dx) * scale),
                Fixed::from_raw((OPEN.1 + dy) * scale),
            ),
            speed: Fixed::from_raw(scale),
            hp,
            amount: 0,
        })
        .expect("the unit can be placed");
    world.step().expect("one tick");
    id
}

fn owner_of(world: &World, id: EntityId) -> Option<u8> {
    world.entity(id).map(|entity| entity.player)
}

/// An unowned sheep joins the first player whose **unit** sees it - and a town centre's sight does not count.
#[test]
fn an_unowned_sheep_joins_the_first_player_who_sees_it() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let sheep = place(&mut world, 0, sheep_unit, 0, 0, 7);
    assert_eq!(owner_of(&world, sheep), Some(0), "a sheep starts neutral");

    // A villager's line of sight is four tiles: three away is inside it, six is not.
    let _seer = place(&mut world, 1, 83, -3, 0, 25);

    let mut converted = false;
    for _ in 0..40 {
        world.step().expect("stepping");
        if owner_of(&world, sheep) == Some(1) {
            converted = true;
            break;
        }
    }
    assert!(
        converted,
        "a sheep in a unit's line of sight belongs to that player at once"
    );
}

/// An owned sheep is not stolen while its owner still has a unit watching it.
#[test]
fn an_owned_sheep_is_not_stolen_while_its_owner_sees_it() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let sheep = place(&mut world, 1, sheep_unit, 0, 0, 7);
    let _owner = place(&mut world, 1, 83, -3, 0, 25);
    let _thief = place(&mut world, 2, 83, 3, 0, 25);

    for _ in 0..60 {
        world.step().expect("stepping");
    }
    assert_eq!(
        owner_of(&world, sheep),
        Some(1),
        "the manual's second sentence: no steal while the owner's units are near"
    );
}

/// ... and it is stolen the moment the owner's units cannot see it.
#[test]
fn an_owned_sheep_is_stolen_when_its_owner_cannot_see_it() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let sheep = place(&mut world, 1, sheep_unit, 0, 0, 7);
    let _thief = place(&mut world, 2, 83, 3, 0, 25);

    let mut stolen = false;
    for tick in 0..40 {
        world.step().expect("stepping");
        if tick % 10 == 0 {
            eprintln!(
                "t={tick} owner={:?} sheep_pos={:?}",
                owner_of(&world, sheep),
                world.position(sheep)
            );
        }
        if owner_of(&world, sheep) == Some(2) {
            stolen = true;
            break;
        }
    }
    assert!(
        stolen,
        "a sheep nobody of its owner's is watching belongs to whoever is watching it"
    );
}

/// Two thieves and no owner: the side with **more units near** the animal takes it.
#[test]
fn a_contested_sheep_goes_to_the_side_with_more_units_near_it() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    // The thieves are placed **before** the animal, because ownership is decided the moment anybody sees it: with
    // the sheep first, the single player-3 unit would take it and then its own sight would protect it - which is
    // the rule working, and a fixture that cannot test the tie-break.
    let _few = place(&mut world, 3, 83, -3, 0, 25);
    let _one = place(&mut world, 2, 83, 3, 0, 25);
    let _two = place(&mut world, 2, 83, 3, 1, 25);
    let sheep = place(&mut world, 1, sheep_unit, 0, 0, 7);

    let mut taken = None;
    for _ in 0..40 {
        world.step().expect("stepping");
        if owner_of(&world, sheep) == Some(2) {
            taken = owner_of(&world, sheep);
            break;
        }
    }
    assert_eq!(
        taken,
        Some(2),
        "the side with more units near the animal wins it, not the lowest player number"
    );
}

/// Food is not ownership: a **boar** is worth meat and never changes hands.
#[test]
fn a_boar_is_not_herdable_and_never_converts() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let boar_unit = world.role_unit("boar").expect("a boar role");
    let boar = place(&mut world, 0, boar_unit, 0, 0, 75);
    let _seer = place(&mut world, 1, 83, -2, 0, 25);

    for _ in 0..60 {
        world.step().expect("stepping");
    }
    assert_eq!(
        owner_of(&world, boar),
        Some(0),
        "only herdables change hands - a boar is food, not livestock (D70)"
    );
}

/// The other half of the tie-break: with the **same** number of units near it, the lower player number takes the
/// animal, so two runs of one match agree. Written after `a_contested_sheep...`, whose counts were two against one
/// and therefore could not tell a count rule from a numbering rule - the mutation that reversed the comparison
/// passed it. This test fails under that mutation, which is the point.
#[test]
fn a_tie_between_two_watchers_goes_to_the_lower_player_number() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    // Two units each, all four seeing the animal, and the animal placed last (see the test above).
    let _a = place(&mut world, 3, 83, -3, 0, 25);
    let _b = place(&mut world, 3, 83, -3, 1, 25);
    let _c = place(&mut world, 2, 83, 3, 0, 25);
    let _d = place(&mut world, 2, 83, 3, 1, 25);
    let sheep = place(&mut world, 1, sheep_unit, 0, 0, 7);

    for _ in 0..40 {
        world.step().expect("stepping");
    }
    assert_eq!(
        owner_of(&world, sheep),
        Some(2),
        "an even contest goes to the lower player number, not to whichever the map happens to visit last"
    );
}

/// One consequence of conversion that the friendly-fire rule had to learn: a **sheep that joined your player is
/// still your shepherd's to kill**. Without it the slaughter is impossible for the ordinary case - the villager
/// walking a sheep is the reason that sheep becomes yours in the first place.
#[test]
fn a_player_can_kill_its_own_sheep() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let villager = place(&mut world, 1, 83, -2, 0, 25);
    let sheep = place(&mut world, 1, sheep_unit, 0, 0, 7);

    world
        .submit_checked(Command::Attack {
            attacker: villager,
            target: sheep,
        })
        .expect("a shepherd may be ordered at its own sheep");

    let mut struck = false;
    for _ in 0..200 {
        world.step().expect("stepping");
        if world.entity(sheep).is_some_and(|entity| entity.hp < 7) {
            struck = true;
            break;
        }
    }
    assert!(
        struck,
        "own livestock is not friendly fire: the blow has to land or no sheep is ever slaughtered"
    );
}

/// ... but the exemption is *livestock*, not "my own things": ordering a villager to kill another of your villagers
/// is still refused.
#[test]
fn friendly_fire_is_still_refused_for_units() {
    let Some(fx) = fixture() else {
        return;
    };
    let mut world = seeded(&fx);
    let one = place(&mut world, 1, 83, -2, 0, 25);
    let other = place(&mut world, 1, 83, 2, 0, 25);

    let refused = world.submit_checked(Command::Attack {
        attacker: one,
        target: other,
    });
    assert!(
        matches!(refused, Err(ruage_sim::SimError::FriendlyFire { .. })),
        "a villager is not livestock: got {refused:?}"
    );
}
