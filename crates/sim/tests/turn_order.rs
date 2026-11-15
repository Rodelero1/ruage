//! **T12 stage 2: the players take turns.** Gaia's objects act first, then player 1's, then player 2's - each
//! player's work is applied as a unit before the next player's begins, so a man killed by a lower player's blow does
//! nothing for the rest of the tick, and a man killed by a *higher* player's blow has already done his.
//!
//! 5.7's `BaseWorld::update` (`FUN_00738720`, `0x738720`, D229) walks the players array at `+0x184..+0x188` from index 0
//! up and steps that player's objects inside the player's own turn. This slice is the **outer** loop: players in index
//! order, each player's work applied as a turn. The inner loop - one object at a time inside a player's turn - is still
//! to come, and `decide` still reads the state the tick opened with.
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
        combat: ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat.toml"),
    })
}

/// **A world with player 2 in it before the stock is applied.** `apply_start_stock` runs once, for the players that
/// exist at that moment, so a player-2 builder spawned afterwards has no wood and cannot pay for a site.
fn world_with_player_two(fx: &Fixture, place: impl FnOnce(&mut World)) -> World {
    let mut world = World::new(&fx.settings, 3).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    place(&mut world);
    world.step().expect("the player-2 ground lands");
    world
        .load_resources(&fx.resources)
        .expect("the start stock reaches a player that has entities");
    let mut combat = fx.combat.clone();
    combat.accuracy_scatter = ruage_data::MaybeFixed::from_f64(0.0);
    world.load_combat(&combat);
    world
}

fn spawn_unit(
    world: &mut World,
    id: u32,
    player: u8,
    unit: u32,
    tile: (i64, i64),
    hp: i32,
    speed: Fixed,
) -> EntityId {
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id: EntityId(id),
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed,
            hp,
            amount: 0,
        })
        .expect("spawn");
    EntityId(id)
}

fn tile_of(world: &World, id: EntityId) -> (i64, i64) {
    let scale = world.scale();
    let (x, y) = world.position(id).expect("a position");
    (x.raw() / scale, y.raw() / scale)
}

/// One tile further from the site than `tile`: the killer stands next to the builder without standing on the ground the
/// house is being raised on.
fn outward(tile: (i64, i64)) -> (i64, i64) {
    let step = |here: i64, there: i64| (here - there).signum();
    (tile.0 + step(tile.0, SITE.0), tile.1 + step(tile.1, SITE.1))
}

fn progress(world: &World, site: EntityId) -> Option<u32> {
    world
        .entity(site)
        .and_then(|entity| entity.construction)
        .map(|construction| construction.progress_ticks)
}

fn roles(fx: &Fixture) -> (u32, u32) {
    let mut probe = World::new(&fx.settings, 3).expect("world");
    probe
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    (
        probe.role_unit("house").expect("a house role"),
        probe.role_unit("villager").expect("a villager role"),
    )
}

const SITE: (i64, i64) = (24, 24);

/// Put a one-hit-point builder beside a foundation and step until it is **working**, then return the site and builder.
///
/// The builder is spawned rather than seeded so its health is the fixture's: a villager's 3-damage blow must take the
/// last hit point in one swing, or "killed before his turn" needs a reload and the tick under test is not the tick of
/// the blow.
fn working_builder(
    world: &mut World,
    player: u8,
    house: u32,
    villager: u32,
) -> (EntityId, EntityId) {
    let builder_id = if player == 1 { 900 } else { 901 };
    // One tile a second: he has two tiles to walk, and the walk is not what these tests measure.
    let walk = Fixed::from_raw(world.scale());
    let builder = spawn_unit(
        world,
        builder_id,
        player,
        villager,
        (SITE.0 - 2, SITE.1),
        1,
        walk,
    );
    world.step().expect("the builder exists");
    world
        .submit_checked(Command::Build {
            villager: builder,
            unit: house,
            at: (
                u16::try_from(SITE.0).expect("a map tile"),
                u16::try_from(SITE.1).expect("a map tile"),
            ),
        })
        .expect("a house site");
    // The engine allocates the site's id, so it is found rather than guessed: a foundation is a building of the
    // builder's player that still has construction left.
    world.step().expect("the site exists");
    let site = world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == player
                && entity.kind == EntityKind::Building
                && entity.construction.is_some()
        })
        .map(|entity| entity.id)
        .expect("a foundation of the builder's player");
    let mut ticks = 0;
    loop {
        world.step().expect("step");
        ticks += 1;
        assert!(ticks < 5_000, "the builder never started working");
        if progress(world, site).is_some_and(|p| p >= 2) {
            break;
        }
    }
    (site, builder)
}

/// **A lower player's builder has already worked when a higher player's blow kills him.** Player 1's turn comes first, so
/// the builder-tick is laid on the site, and only then does player 2's man swing.
#[test]
fn the_lower_players_builder_lays_his_stone_before_he_dies() {
    let Some(fx) = fixture() else { return };
    let (house, villager) = roles(&fx);
    let mut world = world_with_player_two(&fx, |_| {});
    let (site, builder) = working_builder(&mut world, 1, house, villager);

    // The killer stands one tile **outward** from the site, so the first blow lands on the tick it is ordered and no
    // man is spawned inside the footprint.
    let at = tile_of(&world, builder);
    let killer = spawn_unit(&mut world, 950, 2, villager, outward(at), 25, Fixed::ZERO);
    world.step().expect("the killer exists");

    let before = progress(&world, site).expect("a site under construction");
    world
        .submit_checked(Command::Attack {
            attacker: killer,
            target: builder,
        })
        .expect("player 2 attacks the builder");
    world.step().expect("the blow lands");

    assert!(
        world.entity(builder).is_none(),
        "the builder is dead: one hit point against a villager's blow"
    );
    assert_eq!(
        progress(&world, site),
        Some(before + 1),
        "player 1's turn came first, so his builder worked the site before the blow"
    );
}

/// **A higher player's builder lays no stone when a lower player's blow kills him.** The mirror of the test above:
/// player 2's turn is after player 1's, so the builder is dead before his own turn and the site does not move.
#[test]
fn the_higher_players_builder_lays_no_stone_when_he_dies() {
    let Some(fx) = fixture() else { return };
    let (house, villager) = roles(&fx);
    let mut world = world_with_player_two(&fx, |_| {});
    let (site, builder) = working_builder(&mut world, 2, house, villager);

    let at = tile_of(&world, builder);
    let killer = spawn_unit(&mut world, 951, 1, villager, outward(at), 25, Fixed::ZERO);
    world.step().expect("the killer exists");

    let before = progress(&world, site).expect("a site under construction");
    world
        .submit_checked(Command::Attack {
            attacker: killer,
            target: builder,
        })
        .expect("player 1 attacks the builder");
    world.step().expect("the blow lands");

    assert!(
        world.entity(builder).is_none(),
        "the builder is dead: one hit point against a villager's blow"
    );
    assert_eq!(
        progress(&world, site),
        Some(before),
        "player 1's turn comes first, so the dead builder's turn laid no stone"
    );
}
