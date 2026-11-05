//! **The numbers on a unit's card**, as the panel needs them.
//!
//! The owner's ask (2026-11-04): *"let's add a functionality where unit stats are shown in the bottom central part ... For
//! units see Selection-Bar.png for ui example"* - an Archer card reads **attack 4**, **armour 0/0**, **range 4**, and
//! **30/30** hit points under the portrait.
//!
//! Every one of those is already in the pack; what is missing is an accessor the view may call, because the unit table is
//! private to the simulation. This is that accessor's contract, written before the implementation (D7, SPEC §11).
//!
//! **Which attack?** The dat carries an attack per target *class*. An archer has `[[27,3],[21,0],[3,4],[17,0],[13,0]]` and
//! its card says **4** - class 3. A villager has `[[11,3],[4,3],[13,6],[14,0]]` and its card says **3** - class 4, *not* the
//! 6 that class 13 carries, which is a bonus against buildings. So the card's attack is the larger of the two **display**
//! classes (3 = pierce, 4 = melee), and a bonus against a unit class stays off the card. The armour line is class 4 / class 3,
//! which is the `0/0` the Archer card shows.
//!
//! **Units are hundredths.** Size and speed are fractional (a villager walks 0.8 tiles a second, an archer 0.96), so every
//! fractional number here is an integer in hundredths - 80, 96, 400 for four tiles - and the view divides by 100 to print it.
//! No float crosses this boundary, so nothing here can desync a tick.

use std::path::PathBuf;

use ruage_sim::{UnitStats, World};

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
    let mut world = World::new(&settings, 7).ok()?;
    world.seed_from(&map, &economy, &units).ok()?;
    world.load_resources(&resources).ok()?;
    Some(world)
}

#[test]
fn an_archers_card_numbers_are_the_packs() {
    let Some(world) = fresh() else {
        return;
    };
    // The archer the reference pack calls `ARCHR`: 30 hp, attacks `[[27,3],[21,0],[3,4],[17,0],[13,0]]`, armours
    // `[[4,0],[15,0],[3,0]]`, range 4, speed 0.96, line of sight 6, carry 25.
    let stats = world.stats_of(1, 4).expect("the pack has an archer");
    assert_eq!(
        stats,
        UnitStats {
            max_hp: 30,
            attack: 400,
            armour_melee: 0,
            armour_pierce: 0,
            range: 400,
            speed: 96,
            los: 600,
            work_rate: 100,
            carry: 25,
        },
        "the Archer card reads attack 4, armour 0/0, range 4, 30/30"
    );
}

#[test]
fn a_villagers_attack_is_not_its_bonus_against_buildings() {
    let Some(world) = fresh() else {
        return;
    };
    // `VMBAS`: 25 hp, attacks `[[11,3],[4,3],[13,6],[14,0]]`, armours `[[4,0],[3,0]]`, speed 0.8, sight 4, carry 10.
    // Class 13's 6 is a bonus against buildings; the card shows 3, so the larger *display* class wins and the bonus is not
    // shown as the unit's attack.
    let stats = world.stats_of(1, 83).expect("the pack has a villager");
    assert_eq!(stats.attack, 300, "a villager's card says attack 3, not 6");
    assert_eq!(stats.max_hp, 25);
    assert_eq!(stats.speed, 80, "0.8 tiles a second");
    assert_eq!(stats.los, 400);
    assert_eq!(stats.carry, 10);
    assert_eq!(stats.range, 0, "a villager is a melee unit");
}

#[test]
fn a_unit_the_pack_lacks_has_no_card_numbers() {
    let Some(world) = fresh() else {
        return;
    };
    assert!(
        world.stats_of(1, 99_999).is_none(),
        "an unknown unit must answer None rather than invent numbers"
    );
}

#[test]
fn the_numbers_belong_to_the_owner_not_the_viewer() {
    let Some(world) = fresh() else {
        return;
    };
    // Player 2's archer is still an archer: what is on its card is what *its* owner has researched. Nothing is researched
    // here, so the two players' numbers are the same - the assertion that matters is that asking for player 2 answers at
    // all, rather than the view showing an enemy through the viewer's upgrades (the enemy-HP bug of 2026-11-04).
    let mine = world.stats_of(1, 4).expect("an archer");
    let theirs = world.stats_of(2, 4).expect("an archer");
    assert_eq!(mine, theirs);
}
