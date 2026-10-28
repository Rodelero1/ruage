//! The generated unit table (`config/gamedata/units.toml`, D6/D8).
//!
//! Values that enter simulation state stay [`MaybeFixed`] until something resolves them at the
//! engine scale, so the `-1.0` sentinel survives all the way to the code that needs the number.
//! The map is keyed by unit id and iterated in id order — determinism is a property of the table,
//! not of the caller.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::cost::{Cost, RawCost};

use crate::{ConfigError, MaybeFixed, SUPPORTED_SCHEMA};

#[derive(Debug, Clone)]
pub struct Unit {
    pub id: u32,
    pub name: String,
    /// \-1 for 236 of the 1,166 units (buildings and scenery carry no class).
    pub class: i16,
    pub hp: i32,
    pub los: MaybeFixed,
    pub speed: MaybeFixed,
    pub carry: u16,
    pub work_rate: MaybeFixed,
    pub drop_sites: Vec<u32>,
    pub garrison: u16,
    pub radius: [MaybeFixed; 3],
    pub costs: Vec<Cost>,
    /// Class id and amount. The class can be -1, which is how the dat says "no armour".
    pub attacks: Vec<(i16, i16)>,
    pub armours: Vec<(i16, i16)>,
    pub range: MaybeFixed,
    pub reload: MaybeFixed,
    pub accuracy: u16,
    /// The blast: a shot that hurts a group, in tiles. 27 of 1,166 units carry one - a mangonel 1.0,
    /// an onager 1.25, a bombard cannon 0.5 - and everything else is zero, which is a shot at a point.
    pub blast_radius: MaybeFixed,
    /// How many rings of the blast bite (`blast_attack_level` in the dat). Carried, and the conservative
    /// reading is applied: a level of zero means the radius does nothing.
    pub blast_level: u16,
    /// The dead zone: this unit cannot shoot anything closer than this, in tiles. Three for a mangonel,
    /// five for a bombard cannon, four for a trebuchet, zero for everything that is not siege.
    pub min_range: MaybeFixed,
    /// The dat's `create_time`: how long this takes to appear. For a unit that is training time;
    /// **for a building it is build time** - one villager finishes a house in 25 s, a barracks in
    /// 50, a farm in 15. -1 on four units means "not creatable", not "instant".
    pub train_time: i32,
    /// The unit this one is an upgraded variant of, when a tech replaces an id instead of changing a
    /// number: an age-up turns the town centre 109 into 71 and the barracks 12 into 498. Everything
    /// that names a building by id has to keep matching across that, so the engine resolves through
    /// this rather than comparing ids. `None` for the 976 units nothing upgrades into.
    pub variant_of: Option<u32>,
    pub base_id: i32,
}

#[derive(Debug, Clone)]
pub struct Units {
    pub civ: u32,
    pub civ_name: String,
    /// Every populated id, in id order.
    pub units: BTreeMap<u32, Unit>,
    /// Variant id -> the unit it was upgraded from (an age-up replaces 109 with 71, 12 with 498).
    ///
    /// Its own table rather than a field on `Unit`: only 190 of 1,166 units have a parent, and
    /// widening every record for them cost 2-4% of a tick at 1,126 entities in table footprint alone
    /// (`docs/performance.md`, seventh pass). A lookup keeps `Unit` exactly as it was.
    pub variants: BTreeMap<u32, u32>,
}

impl Units {
    pub fn unit(&self, id: u32) -> Option<&Unit> {
        self.units.get(&id)
    }

    /// The same lookup, writable: only the override layer (`overrides.rs`) has any business changing a
    /// loaded table, and it is the one caller.
    pub fn unit_mut(&mut self, id: u32) -> Option<&mut Unit> {
        self.units.get_mut(&id)
    }

    /// The unit this id belongs to by *family*: a variant resolves to the unit it was upgraded from,
    /// all the way up. An age-up replaces a building's id (town centre 109 -> 71 -> 141), and every
    /// rule that names a building - what a tech is researched at, where a villager drops wood, what a
    /// barracks trains - has to keep holding after it, so those comparisons go through here rather
    /// than comparing ids directly.
    ///
    /// An id the table does not know is its own root: the engine would rather match nothing than
    /// invent a family for a unit that does not exist. The hop limit is a cycle guard - the data has
    /// no cycles, but a hand-written pack could, and an infinite loop in a tick is not a failure mode
    /// worth risking for a lookup this cheap.
    pub fn root(&self, id: u32) -> u32 {
        let mut current = id;
        for _ in 0..8 {
            match self.variants.get(&current).copied() {
                Some(parent) if parent != current => current = parent,
                _ => return current,
            }
        }
        current
    }
}

// --- wire format --------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUnits {
    schema: u32,
    kind: String,
    civ: u32,
    civ_name: String,
    #[serde(default)]
    unit: Vec<RawUnit>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUnit {
    id: u32,
    name: String,
    class: i16,
    hp: i32,
    los: f64,
    speed: f64,
    carry: u16,
    work_rate: f64,
    #[serde(default)]
    drop_sites: Vec<u32>,
    garrison: u16,
    radius: [f64; 3],
    costs: Vec<RawCost>,
    #[serde(default)]
    attacks: Vec<[i64; 2]>,
    #[serde(default)]
    armours: Vec<[i64; 2]>,
    range: f64,
    reload: f64,
    accuracy: u16,
    /// Defaulted to zero: a pack written before blast damage existed hits one thing, which is what
    /// every unit but siege means.
    #[serde(default)]
    blast_radius: f64,
    #[serde(default)]
    blast_level: u16,
    /// Defaulted to zero: a pack written before siege dead zones existed has none, which is exactly
    /// what every non-siege unit means.
    #[serde(default)]
    min_range: f64,
    train_time: i32,
    #[serde(default)]
    variant_of: Option<u32>,
    base_id: i32,
}

fn invalid(path: &str, key: String, detail: impl Into<String>) -> ConfigError {
    ConfigError::Invalid {
        key: format!("{path}: {key}"),
        detail: detail.into(),
    }
}

fn pairs(
    path: &str,
    id: u32,
    field: &str,
    rows: &[[i64; 2]],
) -> Result<Vec<(i16, i16)>, ConfigError> {
    rows.iter()
        .map(|row| {
            // -1 is a real class id in this data ("no armour"), so the key is signed too.
            let key = i16::try_from(row[0]).map_err(|_| {
                invalid(
                    path,
                    format!("units[{id}].{field}"),
                    format!("{} does not fit an i16 class id", row[0]),
                )
            })?;
            let amount = i16::try_from(row[1]).map_err(|_| {
                invalid(
                    path,
                    format!("units[{id}].{field}"),
                    format!("{} does not fit an i16", row[1]),
                )
            })?;
            Ok((key, amount))
        })
        .collect()
}

pub fn load_units_str(text: &str, path: &str) -> Result<Units, ConfigError> {
    let raw: RawUnits = toml::from_str(text).map_err(|err| ConfigError::Parse {
        path: path.to_owned(),
        detail: err.to_string(),
    })?;

    if raw.schema != SUPPORTED_SCHEMA {
        return Err(invalid(
            path,
            "schema".to_owned(),
            format!("{} is not this engine's schema", raw.schema),
        ));
    }
    if raw.kind != "units" {
        return Err(invalid(
            path,
            "kind".to_owned(),
            format!("{:?} is not \"units\"", raw.kind),
        ));
    }
    if raw.civ == 0 {
        return Err(invalid(
            path,
            "civ".to_owned(),
            "0 is Gaia; the canonical table must be a playable civ (D3)",
        ));
    }
    if raw.unit.is_empty() {
        return Err(invalid(path, "unit".to_owned(), "the table has no units"));
    }

    let mut units = BTreeMap::new();
    let mut variants: BTreeMap<u32, u32> = BTreeMap::new();
    for r in raw.unit {
        let costs = r
            .costs
            .iter()
            .map(|c| crate::cost::convert(c, || format!("units[{}].costs", r.id)))
            .collect::<Result<Vec<_>, ConfigError>>()?;

        let unit = Unit {
            id: r.id,
            name: r.name,
            class: r.class,
            hp: r.hp,
            los: MaybeFixed::from_f64(r.los),
            speed: MaybeFixed::from_f64(r.speed),
            carry: r.carry,
            work_rate: MaybeFixed::from_f64(r.work_rate),
            drop_sites: r.drop_sites,
            garrison: r.garrison,
            radius: [
                MaybeFixed::from_f64(r.radius[0]),
                MaybeFixed::from_f64(r.radius[1]),
                MaybeFixed::from_f64(r.radius[2]),
            ],
            costs,
            attacks: pairs(path, r.id, "attacks", &r.attacks)?,
            armours: pairs(path, r.id, "armours", &r.armours)?,
            range: MaybeFixed::from_f64(r.range),
            reload: MaybeFixed::from_f64(r.reload),
            accuracy: r.accuracy,
            blast_radius: MaybeFixed::from_f64(r.blast_radius),
            blast_level: r.blast_level,
            min_range: MaybeFixed::from_f64(r.min_range),
            train_time: r.train_time,
            base_id: r.base_id,
            variant_of: r.variant_of,
        };

        if let Some(parent) = r.variant_of {
            if parent == r.id {
                return Err(invalid(
                    path,
                    format!("units[{}].variant_of", r.id),
                    "a unit cannot be a variant of itself",
                ));
            }
            variants.insert(r.id, parent);
        }
        if units.insert(unit.id, unit).is_some() {
            return Err(invalid(
                path,
                format!("units[{}]", r.id),
                "duplicate id: the table's contents would depend on parse order",
            ));
        }
    }

    // A drop site is a building the villager walks to. A dangling id is a villager who can carry
    // wood forever and never deposit it - visible ten minutes into a game, not at startup.
    for unit in units.values() {
        for site in &unit.drop_sites {
            if !units.contains_key(site) {
                return Err(invalid(
                    path,
                    format!("units[{}].drop_sites", unit.id),
                    format!("{site} is not a unit in this table"),
                ));
            }
        }
    }

    Ok(Units {
        civ: raw.civ,
        civ_name: raw.civ_name,
        units,
        variants,
    })
}

pub fn load_units_file(path: &Path) -> Result<Units, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
        path: path.display().to_string(),
        detail: err.to_string(),
    })?;
    load_units_str(&text, &path.display().to_string())
}
