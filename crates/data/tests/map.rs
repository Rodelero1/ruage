//! The authored slice map (D15). Written before the implementation (D7).

use std::path::PathBuf;

use ruage_data::{ResourceKind, Terrain, load_map_file, load_map_str};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
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

#[test]
fn the_real_map_toml_loads_as_a_mirrored_64x64() {
    let map = load_map_file(&repo("config/map.toml")).expect("config/map.toml must load");

    assert_eq!((map.width, map.height), (64, 64));
    assert_eq!(map.terrain.len(), 64 * 64, "one entry per tile");

    let players: Vec<u8> = map.town_centres.iter().map(|tc| tc.player).collect();
    assert_eq!(players, vec![1, 2], "both players get a town centre");

    let count = |kind: ResourceKind| map.resources.iter().filter(|n| n.kind == kind).count();
    assert_eq!(count(ResourceKind::Wood), 90);
    assert_eq!(count(ResourceKind::Gold), 8);
    assert_eq!(count(ResourceKind::Stone), 8);
    assert_eq!(count(ResourceKind::Food), 12);

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
        match node.kind {
            ResourceKind::Wood => {
                assert_eq!(terrain, Terrain::Forest, "a tree must stand on forest")
            }
            _ => assert_eq!(terrain, Terrain::Grass, "resources stand on open ground"),
        }
    }

    // The map is mirrored, so each player has the same total of every resource.
    let half = |kind: ResourceKind| {
        map.resources
            .iter()
            .filter(|n| n.kind == kind && n.x < 32)
            .map(|n| n.amount)
            .sum::<u32>()
    };
    let other = |kind: ResourceKind| {
        map.resources
            .iter()
            .filter(|n| n.kind == kind && n.x >= 32)
            .map(|n| n.amount)
            .sum::<u32>()
    };
    for kind in [
        ResourceKind::Wood,
        ResourceKind::Gold,
        ResourceKind::Stone,
        ResourceKind::Food,
    ] {
        assert_eq!(half(kind), other(kind), "{kind:?} is not mirrored");
    }
}

#[test]
fn a_row_of_the_wrong_length_is_refused() {
    let bad = GOOD.replace(r#"  "GFFG","#, r#"  "GFF","#);
    let err = load_map_str(&bad, "map.toml")
        .expect_err("a short row must be refused")
        .to_string();
    assert!(err.contains("terrain"), "got: {err}");
}

#[test]
fn an_unknown_terrain_character_is_refused() {
    let bad = GOOD.replacen("  \"GGGG\",", "  \"GGGX\",", 1);
    let err = load_map_str(&bad, "map.toml")
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
    let err = load_map_str(&bad, "map.toml")
        .expect_err("an out-of-bounds resource must be refused")
        .to_string();
    assert!(err.contains("outside"), "got: {err}");
}

#[test]
fn a_map_needs_a_town_centre_for_each_player() {
    let bad = GOOD.replace("[[map.town_centre]]\nplayer = 2\nx = 0\ny = 3", "");
    let err = load_map_str(&bad, "map.toml")
        .expect_err("a one-sided map must be refused")
        .to_string();
    assert!(err.contains("town_centre"), "got: {err}");
}

#[test]
fn wood_must_stand_on_forest() {
    let bad = GOOD.replace("tiles = [[1, 1], [2, 1]]", "tiles = [[0, 1]]");
    let err = load_map_str(&bad, "map.toml")
        .expect_err("a tree on grass must be refused")
        .to_string();
    assert!(err.contains("forest"), "got: {err}");
}
