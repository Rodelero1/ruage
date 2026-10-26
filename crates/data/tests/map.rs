//! The authored slice map (D15). Written before the implementation (D7).

use std::path::PathBuf;

use ruage_data::{
    Resources, Terrain, load_map_file, load_map_str, load_resources_file, load_resources_str,
};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// The project's pack: wood on forest, gold and stone on open ground, slots 0..=3.
fn project() -> Resources {
    load_resources_file(&repo("config/resources.toml")).expect("config/resources.toml must load")
}

const GOOD: &str = r#"
schema = 1
[map]
width = 4
height = 4
terrain = [
  "GGGG",
  "GFFG",
  "GFFG",
  "GGGG",
]
[[map.resource]]
kind = "wood"
amount = 100
tiles = [[1, 1], [2, 1]]
[[map.resource]]
kind = "gold"
amount = 400
tiles = [[0, 0]]
[[map.town_centre]]
player = 1
x = 3
y = 3
[[map.town_centre]]
player = 2
x = 0
y = 3
"#;

/// A pack of our own making: the names, the slots and where each resource may stand all come from
/// content (D17). Nothing in it means anything to the reference pack's numbering.
const OWN: &str = r#"
schema = 1

[[resource]]
slot = 0
name = "timber"
site = "forest"

[[resource]]
slot = 1
name = "clay"
"#;

#[test]
fn the_real_map_toml_loads_as_a_mirrored_64x64() {
    let pack = project();
    let map = load_map_file(&repo("config/map.toml"), &pack).expect("config/map.toml must load");

    assert_eq!((map.width, map.height), (64, 64));
    assert_eq!(map.terrain.len(), 64 * 64, "one entry per tile");

    let players: Vec<u8> = map.town_centres.iter().map(|tc| tc.player).collect();
    assert_eq!(players, vec![1, 2], "both players get a town centre");

    let count = |slot: usize| map.resources.iter().filter(|n| n.slot == slot).count();
    let of = |name: &str| pack.slot_of(name).expect("the project pack names it");
    assert_eq!(count(of("wood")), 90);
    assert_eq!(count(of("gold")), 8);
    assert_eq!(count(of("stone")), 8);
    assert_eq!(count(of("food")), 12);

    // Every town centre stands on walkable ground, and every tree on a forest tile.
    for tc in &map.town_centres {
        assert_eq!(
            map.terrain_at(tc.x, tc.y),
            Some(Terrain::Grass),
            "town centre for player {} is not on grass",
            tc.player
        );
    }
    for node in &map.resources {
        let terrain = map.terrain_at(node.x, node.y).expect("in bounds");
        let site = pack.site(node.slot).expect("a slot the pack defines");
        match site {
            ruage_data::ResourceSite::Forest => {
                assert_eq!(terrain, Terrain::Forest, "a tree must stand on forest")
            }
            ruage_data::ResourceSite::Open => {
                assert_eq!(terrain, Terrain::Grass, "resources stand on open ground")
            }
        }
    }

    // The map is mirrored, so each player has the same total of every resource.
    let half = |slot: usize| {
        map.resources
            .iter()
            .filter(|n| n.slot == slot && n.x < 32)
            .map(|n| n.amount)
            .sum::<u32>()
    };
    let other = |slot: usize| {
        map.resources
            .iter()
            .filter(|n| n.slot == slot && n.x >= 32)
            .map(|n| n.amount)
            .sum::<u32>()
    };
    for name in ["wood", "food", "gold", "stone"] {
        let slot = of(name);
        assert_eq!(half(slot), other(slot), "{name} is not mirrored");
    }
}

#[test]
fn a_maps_resource_names_resolve_to_the_packs_slots() {
    // The map says what a node *is* - "timber" - and the pack says which stockpile column that is.
    // The loader hands the map a column and never a name, so an engine with no idea what timber is
    // still knows where to bank it.
    let pack = load_resources_str(OWN, "<own pack>").expect("the pack loads");
    let text = GOOD
        .replace(r#"kind = "wood""#, r#"kind = "timber""#)
        .replace(r#"kind = "gold""#, r#"kind = "clay""#);

    let map = load_map_str(&text, "map.toml", &pack).expect("the map loads with its own pack");

    let timber = pack.slot_of("timber").expect("timber is in the pack");
    let clay = pack.slot_of("clay").expect("clay is in the pack");
    assert_eq!(
        map.resources.iter().filter(|n| n.slot == timber).count(),
        2,
        "both timber tiles are in the timber column"
    );
    assert_eq!(
        map.resources.iter().filter(|n| n.slot == clay).count(),
        1,
        "the clay tile is in the clay column"
    );
}

#[test]
fn a_resource_name_the_pack_does_not_carry_is_refused() {
    // A typo in a map is a resource nobody can ever gather, and it must not become slot 0 (which
    // would quietly make it a tree).
    let pack = project();
    let bad = GOOD.replace(r#"kind = "gold""#, r#"kind = "silver""#);
    let err = load_map_str(&bad, "map.toml", &pack)
        .expect_err("a resource the pack does not name must be refused")
        .to_string();
    assert!(err.contains("silver"), "the error should name it: {err}");
}

#[test]
fn a_row_of_the_wrong_length_is_refused() {
    let bad = GOOD.replace(r#"  "GFFG","#, r#"  "GFF","#);
    let err = load_map_str(&bad, "map.toml", &project())
        .expect_err("a short row must be refused")
        .to_string();
    assert!(err.contains("terrain"), "got: {err}");
}

#[test]
fn an_unknown_terrain_character_is_refused() {
    let bad = GOOD.replacen("  \"GGGG\",", "  \"GGGX\",", 1);
    let err = load_map_str(&bad, "map.toml", &project())
        .expect_err("an unknown terrain character must be refused")
        .to_string();
    assert!(
        err.contains("X"),
        "the error should name the character, got: {err}"
    );
}

#[test]
fn a_resource_outside_the_map_is_refused() {
    let bad = GOOD.replace("tiles = [[0, 0]]", "tiles = [[4, 0]]");
    let err = load_map_str(&bad, "map.toml", &project())
        .expect_err("an out-of-bounds resource must be refused")
        .to_string();
    assert!(err.contains("outside"), "got: {err}");
}

#[test]
fn a_map_needs_a_town_centre_for_each_player() {
    let bad = GOOD.replace("[[map.town_centre]]\nplayer = 2\nx = 0\ny = 3", "");
    let err = load_map_str(&bad, "map.toml", &project())
        .expect_err("a one-sided map must be refused")
        .to_string();
    assert!(err.contains("town_centre"), "got: {err}");
}

#[test]
fn a_resource_cannot_sit_under_a_town_centre() {
    // Both are on grass, so nothing else in the loader notices - and a tree inside the town
    // centre is a villager who can never reach it.
    let bad = GOOD.replace("tiles = [[0, 0]]", "tiles = [[3, 3]]");
    let err = load_map_str(&bad, "map.toml", &project())
        .expect_err("a resource on the town-centre tile must be refused")
        .to_string();
    assert!(err.contains("town centre"), "got: {err}");
}

#[test]
fn a_resource_the_pack_places_in_forest_must_stand_on_forest() {
    // "Wood grows in forests" is a fact about wood, so it lives in the pack, not in a match arm on
    // an enum: our own pack can put anything at all in a forest.
    let pack = load_resources_str(OWN, "<own pack>").expect("the pack loads");
    let on_grass = GOOD
        .replace(r#"kind = "wood""#, r#"kind = "timber""#)
        .replace(r#"kind = "gold""#, r#"kind = "clay""#)
        .replace("tiles = [[1, 1], [2, 1]]", "tiles = [[0, 1]]");
    let err = load_map_str(&on_grass, "map.toml", &pack)
        .expect_err("timber on grass must be refused: the pack says it lives in forest")
        .to_string();
    assert!(err.contains("forest"), "got: {err}");
}

#[test]
fn a_resource_the_pack_leaves_on_open_ground_cannot_stand_in_a_forest() {
    let pack = load_resources_str(OWN, "<own pack>").expect("the pack loads");
    let in_forest = GOOD
        .replace(r#"kind = "wood""#, r#"kind = "timber""#)
        .replace(r#"kind = "gold""#, r#"kind = "clay""#)
        .replace("tiles = [[0, 0]]", "tiles = [[1, 2]]");
    let err = load_map_str(&in_forest, "map.toml", &pack)
        .expect_err("clay inside a forest must be refused")
        .to_string();
    assert!(err.contains("forest"), "got: {err}");
}
