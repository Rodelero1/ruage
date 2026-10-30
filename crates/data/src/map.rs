//! The authored slice map (`config/map.toml`, D15): terrain, resource nodes and town centres.
//!
//! This is our own content, not extracted game data — a 64×64 map mirrored point-for-point so
//! neither player has an advantage. The loader's job is to make an authoring mistake loud: a row
//! of the wrong width, a tree on open grass, a resource off the edge, or a side without a town
//! centre are all startup failures rather than a subtly unfair map.

use std::path::Path;

use serde::Deserialize;

use crate::{ConfigError, ResourceSite, Resources, SUPPORTED_SCHEMA};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terrain {
    Grass,
    Forest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceNode {
    /// The content pack's stockpile column for this resource. A column, not a name and not a kind:
    /// the map file says `kind = "timber"`, the pack answers "slot 0", and everything above the
    /// pack layer works in columns (D17).
    pub slot: usize,
    pub x: u16,
    pub y: u16,
    pub amount: u32,
    /// Hit points to fell before this node yields: a tree has 20 (25, 30 for variants), a bush or a mine has none.
    pub hp: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TownCentre {
    pub player: u8,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone)]
pub struct Map {
    pub width: u16,
    pub height: u16,
    /// Row-major, `width * height` entries.
    pub terrain: Vec<Terrain>,
    pub resources: Vec<ResourceNode>,
    pub town_centres: Vec<TownCentre>,
}

impl Map {
    pub fn terrain_at(&self, x: u16, y: u16) -> Option<Terrain> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.terrain
            .get(usize::from(y) * usize::from(self.width) + usize::from(x))
            .copied()
    }

    pub fn is_walkable(&self, x: u16, y: u16) -> bool {
        self.terrain_at(x, y) == Some(Terrain::Grass)
    }
}

// --- wire format --------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    schema: u32,
    map: RawMap,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMap {
    width: u16,
    height: u16,
    terrain: Vec<String>,
    #[serde(default)]
    resource: Vec<RawResource>,
    #[serde(default)]
    town_centre: Vec<RawTownCentre>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResource {
    /// The resource's name, as `config/resources.toml` spells it.
    kind: String,
    amount: u32,
    /// Hit points, for a node that has to be **felled** before it yields (a tree: 20 in the dat, 25 or 30 for the
    /// variants). Zero for a bush, a gold mine or a stone mine, which are worked as they stand.
    #[serde(default)]
    hp: i32,
    tiles: Vec<[u16; 2]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTownCentre {
    player: u8,
    x: u16,
    y: u16,
}

fn invalid(path: &str, key: impl AsRef<str>, detail: impl Into<String>) -> ConfigError {
    ConfigError::Invalid {
        key: format!("{path}: {}", key.as_ref()),
        detail: detail.into(),
    }
}

pub fn load_map_str(text: &str, path: &str, pack: &Resources) -> Result<Map, ConfigError> {
    let raw: RawFile = toml::from_str(text).map_err(|err| ConfigError::Parse {
        path: path.to_owned(),
        detail: err.to_string(),
    })?;

    if raw.schema != SUPPORTED_SCHEMA {
        return Err(invalid(
            path,
            "schema",
            format!("{} is not this engine's schema", raw.schema),
        ));
    }

    let (width, height) = (raw.map.width, raw.map.height);
    if width == 0 || height == 0 {
        return Err(invalid(
            path,
            "map",
            "width and height must be greater than zero",
        ));
    }

    // --- terrain ---
    if raw.map.terrain.len() != usize::from(height) {
        return Err(invalid(
            path,
            "map.terrain",
            format!("{} row(s) for a height of {height}", raw.map.terrain.len()),
        ));
    }
    let mut terrain = Vec::with_capacity(usize::from(width) * usize::from(height));
    for (y, row) in raw.map.terrain.iter().enumerate() {
        let chars: Vec<char> = row.chars().collect();
        if chars.len() != usize::from(width) {
            return Err(invalid(
                path,
                "map.terrain",
                format!(
                    "row {y} is {} character(s) wide, expected {width}",
                    chars.len()
                ),
            ));
        }
        for (x, ch) in chars.into_iter().enumerate() {
            let tile = match ch {
                'G' => Terrain::Grass,
                'F' => Terrain::Forest,
                other => {
                    return Err(invalid(
                        path,
                        "map.terrain",
                        format!(
                            "row {y}, column {x}: unknown terrain {other:?} (expected 'G' or 'F')"
                        ),
                    ));
                }
            };
            terrain.push(tile);
        }
    }
    let map = Map {
        width,
        height,
        terrain,
        resources: Vec::new(),
        town_centres: Vec::new(),
    };

    // --- resources ---
    // The pack is the authority on what a name means: an unknown one is a typo nobody could ever
    // gather, and it must not default to a column (slot 0 would quietly make it a tree).
    let mut resources = Vec::new();
    for group in &raw.map.resource {
        let Some(slot) = pack.slot_of(&group.kind) else {
            return Err(invalid(
                path,
                "map.resource",
                format!(
                    "{:?} is not a resource the content pack defines ({} are: {})",
                    group.kind,
                    pack.count(),
                    (0..pack.count())
                        .filter_map(|slot| pack.name(slot))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        };
        if group.tiles.is_empty() {
            return Err(invalid(
                path,
                "map.resource",
                format!("{:?} group has no tiles", group.kind),
            ));
        }
        if group.amount == 0 {
            return Err(invalid(
                path,
                "map.resource",
                format!("{:?} group has amount 0", group.kind),
            ));
        }
        for [x, y] in &group.tiles {
            let Some(tile) = map.terrain_at(*x, *y) else {
                return Err(invalid(
                    path,
                    "map.resource",
                    format!("{:?} at ({x}, {y}) is outside the map", group.kind),
                ));
            };
            // Where this resource may stand is content (`site` in resources.toml), because "wood
            // grows in forests" is a fact about wood. This is the check that catches "a tree you
            // can walk through" or "gold in the trees". A column the pack cannot describe at all is
            // a contradiction in the pack, so it is refused rather than treated as open ground.
            let Some(site) = pack.site(slot) else {
                return Err(invalid(
                    path,
                    "map.resource",
                    format!(
                        "the pack names {:?} in column {slot} but does not say where it may stand",
                        group.kind
                    ),
                ));
            };
            match site {
                ResourceSite::Forest if tile != Terrain::Forest => {
                    return Err(invalid(
                        path,
                        "map.resource",
                        format!(
                            "{:?} at ({x}, {y}) is not on a forest tile, and the pack says it grows in forests",
                            group.kind
                        ),
                    ));
                }
                ResourceSite::Forest => {}
                ResourceSite::Open if tile == Terrain::Forest => {
                    return Err(invalid(
                        path,
                        "map.resource",
                        format!(
                            "{:?} at ({x}, {y}) is inside a forest, and the pack does not put it there",
                            group.kind
                        ),
                    ));
                }
                ResourceSite::Open => {}
            }
            resources.push(ResourceNode {
                slot,
                x: *x,
                y: *y,
                amount: group.amount,
                hp: group.hp,
            });
        }
    }

    // --- town centres ---
    let mut saw = [false; 3];
    for tc in &raw.map.town_centre {
        if !(1..=2).contains(&tc.player) {
            return Err(invalid(
                path,
                "map.town_centre",
                format!("player {} is not 1 or 2", tc.player),
            ));
        }
        if saw[usize::from(tc.player)] {
            return Err(invalid(
                path,
                "map.town_centre",
                format!("player {} has more than one town centre", tc.player),
            ));
        }
        if map.terrain_at(tc.x, tc.y).is_none() {
            return Err(invalid(
                path,
                "map.town_centre",
                format!(
                    "player {} town centre at ({}, {}) is outside the map",
                    tc.player, tc.x, tc.y
                ),
            ));
        }
        if !map.is_walkable(tc.x, tc.y) {
            return Err(invalid(
                path,
                "map.town_centre",
                format!(
                    "player {} town centre at ({}, {}) is not on open ground",
                    tc.player, tc.x, tc.y
                ),
            ));
        }
        saw[usize::from(tc.player)] = true;
    }
    if !saw[1] || !saw[2] {
        return Err(invalid(
            path,
            "map.town_centre",
            "both players need a town centre: a slice with one side is not a game",
        ));
    }

    // A resource under a town centre is unreachable: the building occupies the tile, and the
    // map loader alone can see both, so this is the only place the clash can be caught.
    for node in &resources {
        if let Some(town) = raw
            .map
            .town_centre
            .iter()
            .find(|town| town.x == node.x && town.y == node.y)
        {
            return Err(invalid(
                path,
                "map.resource",
                format!(
                    "a resource at ({}, {}) sits under player {}'s town centre",
                    node.x, node.y, town.player
                ),
            ));
        }
    }

    Ok(Map {
        resources,
        town_centres: raw
            .map
            .town_centre
            .iter()
            .map(|tc| TownCentre {
                player: tc.player,
                x: tc.x,
                y: tc.y,
            })
            .collect(),
        ..map
    })
}

pub fn load_map_file(path: &Path, pack: &Resources) -> Result<Map, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
        path: path.display().to_string(),
        detail: err.to_string(),
    })?;
    load_map_str(&text, &path.display().to_string(), pack)
}
