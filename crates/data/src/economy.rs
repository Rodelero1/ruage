//! The economy config (`config/economy.toml`): the rates the dat does not settle, plus the
//! task → villager-variant table from D11.
//!
//! Same rules as `sim.toml` (D6): unknown keys and missing keys are startup failures, and `-1.0`
//! means *unresolved*, not zero. The one deliberate difference is `farm.rate`: minus one is never
//! a plausible gather rate, so a negative value there is a typo and is refused rather than
//! silently treated as a sentinel.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::{ConfigError, MaybeFixed, SUPPORTED_SCHEMA};

/// Which cells of a building's footprint are **solid** (D76). Content describes it per role; the engine only asks which
/// tiles block.
///
/// Rows run top-to-bottom and columns left-to-right from the building's **anchor tile**, which is the north corner of its
/// diamond (D84), so a 4x4 Town Centre with `["##..", "##..", "....", "...."]` keeps its back core solid and its wings and
/// courtyard walkable - the owner's diagram of the building, as data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Footprint {
    pub width: usize,
    pub height: usize,
    /// Row-major, `true` where the cell is impassable.
    pub blocked: Vec<bool>,
}

impl Footprint {
    /// Parse the rows content writes. Refused rather than guessed: a ragged mask is a typo in a block that decides where
    /// units can walk, and a silent default would make a building hollow on one axis.
    pub fn from_rows(rows: &[String]) -> Result<Footprint, String> {
        if rows.is_empty() {
            return Err("a footprint needs at least one row".to_owned());
        }
        let width = rows[0].chars().count();
        if width == 0 {
            return Err("a footprint row cannot be empty".to_owned());
        }
        let mut blocked = Vec::with_capacity(width * rows.len());
        for (index, row) in rows.iter().enumerate() {
            if row.chars().count() != width {
                return Err(format!(
                    "row {index} is {} characters wide, expected {width}",
                    row.chars().count()
                ));
            }
            for cell in row.chars() {
                match cell {
                    '#' => blocked.push(true),
                    '.' => blocked.push(false),
                    other => {
                        return Err(format!(
                            "row {index}: unknown cell {other:?} (expected '#' or '.')"
                        ));
                    }
                }
            }
        }
        Ok(Footprint {
            width,
            height: rows.len(),
            blocked,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Economy {
    /// **How much larger than their body a unit's selection ring is drawn** (D125). View-side only: no tick reads it.
    pub ring: Ring,
    /// Which buildings may create which units. Authored (see `config/economy.toml`): the dat
    /// carries unit costs and create times but not the building-to-unit edge we read.
    pub train: Train,
    pub schema: u32,
    pub farm: Farm,
    pub gather: Gather,
    pub build: Build,
    pub decay: Decay,
    pub population: Population,
    /// What a match begins with, by role name. Config, never code (D6).
    pub start: Start,
    /// Role name → the solid cells of its building's footprint (D76). A role with no entry blocks only its own tile, which
    /// is what every building did before masks existed.
    pub footprint: std::collections::BTreeMap<String, Footprint>,
    /// Role name → unit id. One vocabulary for everything that must not quote the dat's numbering:
    /// the start of a match, the scenario format, and the engine's own seeding (D17).
    pub role: BTreeMap<String, u32>,
    /// Tech role → tech id. Techs have no name in the dat, so content that names one needs this.
    pub tech_role: BTreeMap<String, u32>,
    /// What each age needs before it can be researched (D65): how many buildings of the current age, and
    /// which roles count. The Castle Age's exception - a single Castle also opens the Imperial Age - is
    /// the `single` list.
    pub age: Ages,
    /// What counts towards a side still being in the match (D59). The original's exclusion list is a
    /// fact about the game, not about the engine, so it is content: role names that do *not* keep a
    /// player alive.
    pub conquest: Conquest,
    /// Which animals herd, and where a gathered one walks (D70).
    pub hunt: Hunt,
    /// What the batch runner counts as military (items 7-8).
    pub metrics: MetricsSection,
    /// Role name → the **mark** the view draws on its footprint (item 6). The vocabulary of marks belongs to the view
    /// ("ring", "roof", "sword", ...), so content names one and the view decides what it looks like - the same split the
    /// palette has (D55). A role with no entry is drawn as a plain block.
    pub mark: BTreeMap<String, String>,
}

/// Herding: the animals that walk to a drop site when a villager works them. Content, because the engine
/// must not know that a sheep is different from a cow (D17).
#[derive(Debug, Clone, Default)]
pub struct Hunt {
    /// Role names, resolved to unit families at seed time: these walk to a drop site when gathered (D70).
    pub herdable_roles: Vec<String>,
    /// Role names of every animal worth meat - both kinds, herdable and huntable.
    pub animal_roles: Vec<String>,
    /// The resource an animal yields, by name in the pack's stockpile.
    pub resource: Option<String>,
    /// Role names of the bodies an animal becomes when slaughtered. A carcass is gathered from, so it has to
    /// be nameable rather than inferred from "has no health" (D70).
    pub carcass_roles: Vec<String>,
    /// How long a struck animal keeps chasing after the last blow, in whole seconds (D71). `None` means
    /// content states no time limit. The dat carries no leash field, so this is authored, and it is a knob
    /// a balance run turns - whole seconds because a fractional one would only invite the float the
    /// fixed-point rule exists to keep out of state.
    pub leash_seconds: Option<u32>,
    /// How far from where it was first struck an animal will chase, in whole tiles (D71). Authored for the
    /// same reason: `BOARX` has a line of sight (4.0) and no notion of giving up.
    pub leash_tiles: Option<u32>,
    /// Roles whose **last** blow leaves meat in the body (D70). Roles, not ids - the engine never learns that a
    /// villager is 83 - and an empty list means every kill leaves its meat.
    pub butcher_roles: Vec<String>,
}

/// `[metrics]`: what a balance run counts as a military unit, by role. Not a rule of the game - a measurement - so a
/// role the pack cannot name is skipped when it is resolved rather than refused at load.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MetricsSection {
    pub military_roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ages {
    /// Tech role -> requirement. Keyed by role because a tech has no name in the dat (D17).
    pub requires: BTreeMap<String, AgeRequirement>,
    /// **The ages in order, oldest first.** A map cannot say "which of these is later" - and the tech ids do not either
    /// (dark 104, feudal 101, castle 102, imperial 103 in this pack) - so the order is content's, like every other name
    /// (D17). Empty means "no order given", and then no age is reported rather than the alphabetically last one.
    pub order: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgeRequirement {
    /// How many buildings of the current age are needed. Zero means "no building requirement".
    pub count: u32,
    /// Roles that count towards the number.
    pub of: Vec<String>,
    /// Roles that satisfy the requirement **on their own** - the Castle Age's exception for the Imperial Age.
    pub single: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Conquest {
    /// Unit *classes* whose buildings keep a player in the match (D59b). The original's exclusion list is
    /// a list of classes - towers, walls, gates, farms and relics each have their own - so "class 3
    /// counts" is the whole rule, and it needs no per-building list to keep in step. Dat numbering, so
    /// content may quote it and the engine may not (D17).
    pub surviving_classes: Vec<i16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Start {
    /// Role names, not unit ids: the roles below are the vocabulary content uses, and the engine
    /// never has to know that a town centre is unit 109 (D17).
    pub town_center: String,
    pub villager: String,
    pub villagers_per_player: u32,
    /// The role a match's **scouting unit** is, if the pack has one (the owner's note, 2026-11-04: "player should start
    /// with scout cavalry, villagers shouldn't scout"). Content's word, like every other role here.
    pub scout: Option<String>,
    /// How many scouts each player begins with. Zero means none, and is the default so a pack that says nothing gets none.
    pub scouts_per_player: u32,
    /// Tech roles a match begins with, already researched. Without them a match could research
    /// nothing at all: every tech in the table lists the Dark Age as a prerequisite.
    pub techs: Vec<String>,
    /// The civilisations the match is played as, by the names `civs.toml` gives them: entry *n* is
    /// player *n+1*, and the last entry covers any further player. Content's words, because the civ
    /// table owns the ids and the effect that lists what each civ cannot research.
    pub civs: Vec<String>,
    /// Team id per player: entry *n* is player *n+1*, and the last entry covers any further player.
    /// Empty means nobody has allies - every player is their own team, which is what a free-for-all is.
    pub teams: Vec<u32>,
    /// Resource *name* -> how much each player begins with. Names, not columns: the pack owns the
    /// stockpile's layout (D17), so a pack with five resources can open with its own four.
    pub stock: BTreeMap<String, u32>,
}

#[derive(Debug, Clone)]
pub struct Farm {
    pub rate: MaybeFixed,
    pub use_villager_rate: bool,
    pub food_per_farm: u32,
    /// The role that names the farm building, if this pack has farms. A building is worth gathering
    /// from only because content says so: the engine has no idea what a farm is.
    pub unit: Option<String>,
    /// The resource a worked farm yields, by name.
    pub resource: Option<String>,
    /// The dat's *internal* resource that a farm upgrade adds to - 36 for the farm food line
    /// (Horse Collar +75, Heavy Plow +125, Crop Rotation +175). The dat names no resources, so this
    /// is an id and content has to say which id means "food per farm" (D28).
    pub food_resource: Option<u32>,
    /// **How long an empty farm stays as a husk**, in seconds (the owner, 2026-11-04: *"vanishing takes some time, husk
    /// first ... It obstructs nothing as if it was not there"*). A husk is a picture with a lifetime: it blocks nothing, holds
    /// nothing and cannot be worked, and after this long it is removed. Content, so a mod can change it.
    pub husk_seconds: u32,
    /// **How far a second farmer looks for a free plot**, in tiles (the owner: *"if there is no empty farm nearby, villager
    /// becomes idle"*). Beyond this a free farm is not "nearby" and the villager stands.
    pub free_farm_radius: u32,
}

/// The selection ring's scale, as content (D125).
#[derive(Debug, Clone, Copy)]
pub struct Ring {
    /// `radius * scale`, **in hundredths** - 150 is one and a half - so the number that crosses into the simulation is an
    /// integer. The `no_float` guard is right: a float in the tick is a desync waiting for a different CPU (D9).
    pub scale_percent: i32,
}

#[derive(Debug, Clone)]
pub struct Gather {
    pub rate_from: String,
    pub carry_from: String,
    pub drop_sites_from: String,
    /// Task name → the villager-variant unit id that performs it (D11).
    pub variant: BTreeMap<String, u32>,
    /// Resource *name*, as `config/resources.toml` spells it → the gather task it presents. Keyed by
    /// name because the engine has no resource kinds of its own (D17): the pack says a node is
    /// "timber", this table says timber is worked by the "wood" task.
    pub resource_task: BTreeMap<String, String>,
    /// Hit points a villager takes off a **standing tree** per second. Trees are felled before they yield (the owner's
    /// rule, 2026-10-30): the dat gives them 20 hit points (25, 30 for variants) and no wood until they come down. Authored
    /// pacing, because the dat does not say how fast a chop is; zero turns felling off and every node yields as it stands.
    pub fell_hp_per_second: f64,
}

impl Gather {
    /// Hit points a villager takes off a standing tree **per tick**, rounded up so a slow chop still lands. Here rather
    /// than in the simulation because this is where the fractional rate lives: the tick only ever sees whole hit points
    /// (D9 - no floats near the simulation's state).
    #[must_use]
    pub fn fell_hp_per_tick(&self, tick_rate_hz: u32) -> i32 {
        let per_tick = self.fell_hp_per_second * f64::from(tick_rate_hz.max(1));
        if per_tick <= 0.0 {
            0
        } else {
            #[allow(clippy::cast_possible_truncation)]
            {
                per_tick.ceil() as i32
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Train {
    /// Building unit id -> the unit ids it may create.
    pub by_building: BTreeMap<u32, Vec<u32>>,
    /// How many units a building may hold at once: the one being made plus those behind it. The game
    /// uses five, and content owns the number because a pack of our own may want another.
    pub queue_length: u32,
}

impl Train {
    /// May this building create this unit?
    pub fn allows(&self, building: u32, unit: u32) -> bool {
        self.by_building
            .get(&building)
            .is_some_and(|units| units.contains(&unit))
    }
}

#[derive(Debug, Clone)]
pub struct Build {
    pub repair_hp_per_second: MaybeFixed,
    /// **How near an existing drop site makes another pointless**, in tiles: the reference implementation's `kCovered` is 5
    /// (its own AI's rule: "a drop site already this near a forest, mine or forage makes another there pointless"). Content
    /// so a pack with bigger maps or cheaper camps can say otherwise.
    pub drop_site_spacing: u32,
    /// Which roles must not be built where a drop site already serves the resource (the owner's rule: a lumber camp
    /// near the town centre or another camp is a walk saved twice). Content names them, like `blocking_roles`: the owner
    /// asked for lumber camps, and a mine can join the list without the engine learning either word.
    pub spaced_roles: Vec<String>,
    /// **The role whose building is a farm plot**, if the pack has one: farms are placed in rings around the town centre and
    /// around the mill, which is a placement rule rather than a gather rule (the owner's note, 2026-11-04).
    pub farm: Option<String>,
    /// **How many farm widths to leave between the town centre and the mill** (the owner: "mill should be built 2 farms away
    /// from the town centre"). The ring a house may not be built in is this plus one, measured in tiles.
    pub farm_spacing: u32,
    /// Which roles block movement once built: walls and their kin. Empty means nothing built blocks,
    /// which is how every pack behaved before walls existed (D54).
    pub blocking_roles: Vec<String>,
    /// Role → the role that must have been **built once** before it may be placed (D63). The game's
    /// building prerequisites: a siege workshop needs a blacksmith, an archery range needs a barracks.
    /// "Built once", not "still standing": the game does not take the right away when the prerequisite
    /// is destroyed, which is why this is a per-player memory rather than a scan of the map.
    pub requires: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Decay {
    pub enabled: bool,
    pub rate_from: String,
}

#[derive(Debug, Clone)]
pub struct Population {
    /// The ceiling no amount of building raises.
    pub cap_default: u32,
    /// What one finished house supports.
    pub house_population: u32,
    /// What one finished town centre supports. Published: a house and a town centre both support five,
    /// a castle twenty - the castle is not modelled, so a pack that wants its cap says it here or not
    /// at all (D31).
    pub town_center_population: u32,
}

// --- wire format: one struct per table, strict -------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFootprint {
    blocked: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEconomy {
    schema: u32,
    #[serde(default)]
    ring: RawRing,
    farm: RawFarm,
    gather: RawGather,
    train: RawTrain,
    build: RawBuild,
    decay: RawDecay,
    population: RawPopulation,
    start: RawStart,
    #[serde(default)]
    conquest: RawConquest,
    #[serde(default)]
    hunt: RawHunt,
    #[serde(default)]
    metrics: RawMetrics,
    #[serde(default)]
    age: RawAges,
    /// `[footprint.<role>] blocked = ["##..", ...]`. Optional: a building without one blocks its own tile.
    #[serde(default)]
    footprint: BTreeMap<String, RawFootprint>,
    /// Role name → unit id. Required: content that cannot name a unit at all is not content.
    role: BTreeMap<String, u32>,
    #[serde(default)]
    tech_role: BTreeMap<String, u32>,
    /// `[mark] <role> = "<shape>"`. Optional: a role without one is drawn as a plain block.
    #[serde(default)]
    mark: BTreeMap<String, String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawAges {
    #[serde(default)]
    requires: BTreeMap<String, RawAgeRequirement>,
    /// The ages in order, oldest first: `["dark_age", "feudal_age", ...]`.
    #[serde(default)]
    order: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawAgeRequirement {
    count: u32,
    #[serde(default)]
    of: Vec<String>,
    #[serde(default)]
    single: Vec<String>,
}

/// `[hunt]`: which role names herd. Defaulted, so a pack written before herding existed has none.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHunt {
    #[serde(default)]
    herdable_roles: Vec<String>,
    #[serde(default)]
    animal_roles: Vec<String>,
    #[serde(default)]
    resource: Option<String>,
    #[serde(default)]
    carcass_roles: Vec<String>,
    #[serde(default)]
    leash_seconds: Option<u32>,
    #[serde(default)]
    leash_tiles: Option<u32>,
    #[serde(default)]
    butcher_roles: Vec<String>,
}

/// `[policy]`: villager shares, in the game's own AI shape. Each row is the share from its `at_villagers` upwards;
/// the highest row the player has reached applies. Resource names key the shares, so the pack's own vocabulary is
/// what content writes.
///
/// Parsed here rather than in a raw struct beside the economy, because the table is **behaviour** and since D94 it
/// lives in a personality (`config/ai/<name>.toml`) while the type stays with the rest of the content vocabulary.
#[derive(Debug, Clone, Default)]
pub struct Policy {
    pub share: Vec<ShareRow>,
}

impl<'de> Deserialize<'de> for Policy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawPolicy::deserialize(deserializer)?;
        let mut rows: Vec<ShareRow> = Vec::with_capacity(raw.share.len());
        for row in raw.share {
            // A share row whose percentages do not sum to 100 is a typo, not a policy: the engine would silently
            // under-serve every resource. Refused at load (D6's rule - a config that cannot mean what it says fails).
            let total: u32 = row.per.values().sum();
            if total != 100 {
                return Err(serde::de::Error::custom(format!(
                    "[policy] row at {} villagers sums to {total}%, not 100%",
                    row.at_villagers
                )));
            }
            rows.push(ShareRow {
                at_villagers: row.at_villagers,
                per: row.per,
            });
        }
        rows.sort_by_key(|row| row.at_villagers);
        Ok(Policy { share: rows })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShareRow {
    pub at_villagers: usize,
    /// Resource name → percent. A resource the row does not name is 0.
    pub per: BTreeMap<String, u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPolicy {
    #[serde(default)]
    share: Vec<RawShare>,
}

/// One row: `at_villagers` plus any number of `resource = percent` keys beside it.
#[derive(Debug, Deserialize)]
struct RawShare {
    at_villagers: usize,
    #[serde(flatten)]
    per: BTreeMap<String, u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMetrics {
    #[serde(default)]
    military_roles: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConquest {
    #[serde(default)]
    surviving_classes: Vec<i16>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFarm {
    rate: f64,
    use_villager_rate: bool,
    depletion: RawDepletion,
    #[serde(default = "default_free_farm_radius")]
    free_farm_radius: u32,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    resource: Option<String>,
    #[serde(default)]
    food_resource: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRing {
    #[serde(default = "default_ring_scale")]
    scale: f64,
}

fn default_ring_scale() -> f64 {
    1.5
}

impl Default for RawRing {
    fn default() -> Self {
        RawRing {
            scale: default_ring_scale(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDepletion {
    food_per_farm: u32,
    /// Absent means the pack says nothing about husks, and the default below applies.
    #[serde(default = "default_husk_seconds")]
    husk_seconds: u32,
}

fn default_husk_seconds() -> u32 {
    30
}

fn default_free_farm_radius() -> u32 {
    12
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGather {
    rate_from: String,
    carry_from: String,
    drop_sites_from: String,
    variant: BTreeMap<String, u32>,
    /// A resource name → the gather task that works it. Free-form keys: which resources exist is the
    /// content pack's business, and this loader can only check the task side of the join.
    resource_task: BTreeMap<String, String>,
    #[serde(default)]
    fell_hp_per_second: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTrain {
    entry: Vec<RawTrainEntry>,
    /// Defaulted to the game's five, so a pack written before the queue existed queues like the game
    /// rather than refusing the second order.
    #[serde(default = "default_queue_length")]
    queue_length: u32,
}

fn default_queue_length() -> u32 {
    5
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTrainEntry {
    /// A role from `[role]`, never a dat id: everything that names a unit in content names it by
    /// role, and accepting both spellings is how the numbering creeps back in (D17).
    building: String,
    units: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBuild {
    repair_hp_per_second: f64,
    /// Defaulted to 5 - the reference implementation's own `kCovered` - so a pack that says nothing still gets the game's
    /// spacing rather than no spacing at all.
    #[serde(default = "default_drop_site_spacing")]
    drop_site_spacing: u32,
    /// Roles that must not be built where a drop site already serves the resource. Empty by default: no spacing rule.
    #[serde(default)]
    spaced_roles: Vec<String>,
    /// The role whose building is a farm plot. Unset means the pack has no farms and no rings are reserved.
    #[serde(default)]
    farm: Option<String>,
    /// Farm widths between the town centre and the mill: 2 is the owner's number.
    #[serde(default = "default_farm_spacing")]
    farm_spacing: u32,
    /// Defaulted to empty: a pack that says nothing about walls builds nothing that blocks.
    #[serde(default)]
    blocking_roles: Vec<String>,
    /// Defaulted to empty: a pack with no building prerequisites builds anything anywhere (D63).
    #[serde(default)]
    requires: std::collections::BTreeMap<String, String>,
}

/// A drop site within this many tiles of a forest, mine or forage makes another there pointless: the reference
/// implementation's `kCovered`, which its AI derives from the original's placement rules.
fn default_drop_site_spacing() -> u32 {
    5
}

/// Two farm widths between the town centre and the mill: the owner's number (2026-11-04), and the game's own habit - a mill
/// with two plots between it and the centre leaves room for the ring that follows the first farms.
fn default_farm_spacing() -> u32 {
    2
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDecay {
    enabled: bool,
    rate_from: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStart {
    town_center: String,
    villager: String,
    villagers_per_player: u32,
    /// The scouting unit's role, if any. Unset means the pack opens with no scout.
    #[serde(default)]
    scout: Option<String>,
    /// How many scouts each player begins with. Defaulted to none.
    #[serde(default)]
    scouts_per_player: u32,
    #[serde(default)]
    techs: Vec<String>,
    #[serde(default)]
    stock: BTreeMap<String, u32>,
    #[serde(default)]
    civs: Vec<String>,
    #[serde(default)]
    teams: Vec<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPopulation {
    cap_default: u32,
    house_population: u32,
    /// Defaulted to zero: a pack written before housing existed supported nothing from its town
    /// centres, which is exactly what "the field is absent" should mean.
    #[serde(default)]
    town_center_population: u32,
}

fn invalid(path: &str, key: &str, detail: impl Into<String>) -> ConfigError {
    ConfigError::Invalid {
        key: format!("{path}: {key}"),
        detail: detail.into(),
    }
}

pub fn load_economy_str(text: &str, path: &str) -> Result<Economy, ConfigError> {
    let raw: RawEconomy = match toml::from_str(text) {
        Ok(raw) => raw,
        Err(err) => {
            // **`[policy]` is behaviour and it moved (D94).** `deny_unknown_fields` refuses it anyway, but "unknown
            // field `policy`" hides the only fact the author needs: where it went. A share table nothing reads looks
            // exactly like a policy that does nothing - the dead-knob failure this project has paid for once - so the
            // refusal names the block and the destination.
            if let Ok(document) = text.parse::<toml::Value>()
                && document.get("policy").is_some()
            {
                return Err(invalid(
                    path,
                    "[policy]",
                    "villager shares are behaviour and live in the personality (config/ai/<name>.toml, D94); \
                     remove [policy] from the economy and put it beside the rules that use it",
                ));
            }
            return Err(ConfigError::Parse {
                path: path.to_owned(),
                detail: err.to_string(),
            });
        }
    };

    if raw.schema != SUPPORTED_SCHEMA {
        return Err(invalid(
            path,
            "schema",
            format!("{} is not this engine's schema", raw.schema),
        ));
    }

    // A negative rate is a typo, not the unresolved sentinel (see the module docs). NaN is
    // rejected by the same test: `is_finite` is false for it, so it never silently passes.
    let farm_rate = raw.farm.rate;
    if !farm_rate.is_finite() || farm_rate <= 0.0 {
        return Err(invalid(
            path,
            "farm.rate",
            format!(
                "{} is not a rate: it must be greater than zero (-1.0 is only a sentinel where \
                 the docs say so)",
                farm_rate
            ),
        ));
    }

    // One variant per task, and no unset ids: a shared id makes one variant serve two
    // resources, and a zero means the field was never filled in.
    let mut claimed: BTreeMap<u32, &String> = BTreeMap::new();
    for (task, id) in &raw.gather.variant {
        if *id == 0 {
            return Err(invalid(
                path,
                "gather.variant",
                format!("{task} = 0: unit id zero is not a villager; the field looks unset"),
            ));
        }
        if let Some(other) = claimed.insert(*id, task) {
            return Err(invalid(
                path,
                "gather.variant",
                format!(
                    "{task} and {other} both name variant {id}; the mapping must be one-to-one"
                ),
            ));
        }
    }

    if raw.gather.variant.is_empty() {
        return Err(invalid(
            path,
            "gather.variant",
            "empty: rate_from = \"dat\" with no variants says one thing and means another (D11)",
        ));
    }

    if raw.population.cap_default == 0 || raw.population.house_population == 0 {
        return Err(invalid(
            path,
            "population",
            "cap_default and house_population must be greater than zero",
        ));
    }

    if raw.farm.depletion.food_per_farm == 0 {
        return Err(invalid(
            path,
            "farm.depletion.food_per_farm",
            "must be greater than zero",
        ));
    }

    // A farm that names a building but no resource (or the reverse) would gather into nothing or
    // from nothing, so both or neither.
    if raw.farm.unit.is_some() != raw.farm.resource.is_some() {
        return Err(invalid(
            path,
            "farm",
            "`unit` and `resource` go together: a farm that yields nothing, or a resource that comes \
             from no building, is half a rule",
        ));
    }
    if let Some(role) = &raw.farm.unit
        && !raw.role.contains_key(role)
    {
        return Err(invalid(
            path,
            "farm.unit",
            format!("`{role}` is not a role: the farm building is named like every other unit"),
        ));
    }

    if raw.role.is_empty() {
        return Err(invalid(
            path,
            "role",
            "no roles: the start and every scenario step name units by role, so an empty table \
             names nothing",
        ));
    }
    // Zero is a real unit id, which is exactly why it cannot be a role: the field would read as
    // unset and a match would seed whatever id 0 happens to be.
    for (name, id) in &raw.role {
        if *id == 0 {
            return Err(invalid(
                path,
                "role",
                format!("{name} = 0: unit id zero is a real unit, never a role"),
            ));
        }
    }
    // A starting tech is a *tech role*, because the engine would otherwise have to be told that
    // "dark age" is id 104.
    for name in &raw.start.techs {
        if !raw.tech_role.contains_key(name) {
            return Err(invalid(
                path,
                "start.techs",
                format!(
                    "{name:?} is not a [tech_role], so no tech can be named by it: the match would \
                     start with a tech the pack cannot describe"
                ),
            ));
        }
    }
    // Tech 0 is a real tech ("New Technology"), so it cannot mean "no role" either.
    for (name, id) in &raw.tech_role {
        if *id == 0 {
            return Err(invalid(
                path,
                "tech_role",
                format!("{name} = 0: tech id zero is a real tech, never a role"),
            ));
        }
    }
    for (field, name) in [
        ("town_center", &raw.start.town_center),
        ("villager", &raw.start.villager),
    ] {
        if !raw.role.contains_key(name) {
            return Err(invalid(
                path,
                "start",
                format!(
                    "{field} names the role {name:?}, which [role] does not define (it defines: {})",
                    raw.role.keys().cloned().collect::<Vec<_>>().join(", ")
                ),
            ));
        }
    }
    if raw.start.town_center == raw.start.villager {
        return Err(invalid(
            path,
            "start",
            format!(
                "{} is both the town centre and the villager; a match starts with one of each",
                raw.start.town_center
            ),
        ));
    }
    // A start above 50 is a typo, not a game variant: the dat's own largest starting count is
    // nowhere near it, and spawning a thousand villagers would hide the mistake behind lag.
    // A count with no role is a pack that means to scout and cannot, which is a mistake worth refusing at load rather than
    // silently opening with no scout.
    if raw.start.scouts_per_player > 0 && raw.start.scout.is_none() {
        return Err(invalid(
            path,
            "start.scouts_per_player",
            format!(
                "{} scouts per player, but start.scout names no role",
                raw.start.scouts_per_player
            ),
        ));
    }
    if let Some(scout) = &raw.start.scout
        && !raw.role.contains_key(scout)
    {
        return Err(invalid(
            path,
            "start.scout",
            format!("{scout:?} is not a name in this pack's [role] table"),
        ));
    }
    if !(1..=50).contains(&raw.start.villagers_per_player) {
        return Err(invalid(
            path,
            "start.villagers_per_player",
            format!("{} is not between 1 and 50", raw.start.villagers_per_player),
        ));
    }

    let resource_task = raw.gather.resource_task;
    // An empty join is a match where nothing on the map can be worked at all.
    if resource_task.is_empty() {
        return Err(invalid(
            path,
            "gather.resource_task",
            "no resource names a gather task: nothing on the map could be worked".to_owned(),
        ));
    }
    // A task name that names no variant is a typo that would otherwise surface much later, as a
    // villager who walks to a tree and never gathers.
    for (resource, task) in &resource_task {
        if !raw.gather.variant.contains_key(task) {
            return Err(invalid(
                path,
                "gather.resource_task",
                format!("{resource} names task {task:?}, which is not in [gather.variant]"),
            ));
        }
    }

    // **Every age in the order must be a tech this pack names**, or the interface would have a word for an age that cannot
    // be researched - a typo that would otherwise show as an age that never arrives.
    for role in &raw.age.order {
        if !raw.tech_role.contains_key(role) {
            return Err(invalid(
                path,
                "age.order",
                format!("{role:?} is not a name in this pack's [tech_role] table"),
            ));
        }
    }

    if let Some(farm) = &raw.build.farm
        && !raw.role.contains_key(farm)
    {
        return Err(invalid(
            path,
            "build.farm",
            format!("{farm:?} is not a name in this pack's [role] table"),
        ));
    }
    if !(1..=4).contains(&raw.build.farm_spacing) {
        return Err(invalid(
            path,
            "build.farm_spacing",
            format!(
                "{} is not between 1 and 4 farm widths",
                raw.build.farm_spacing
            ),
        ));
    }

    // Every role with a spacing rule must be a role this pack has, or the rule silently applies to nothing.
    for role in &raw.build.spaced_roles {
        if !raw.role.contains_key(role) {
            return Err(invalid(
                path,
                "build.spaced_roles",
                format!("{role:?} is not a name in this pack's [role] table"),
            ));
        }
    }

    Ok(Economy {
        schema: raw.schema,
        ring: Ring {
            scale_percent: (raw.ring.scale * 100.0).round() as i32,
        },
        farm: Farm {
            rate: MaybeFixed::from_f64(raw.farm.rate),
            use_villager_rate: raw.farm.use_villager_rate,
            food_per_farm: raw.farm.depletion.food_per_farm,
            unit: raw.farm.unit,
            resource: raw.farm.resource,
            food_resource: raw.farm.food_resource,
            husk_seconds: raw.farm.depletion.husk_seconds,
            free_farm_radius: raw.farm.free_farm_radius,
        },
        gather: Gather {
            rate_from: raw.gather.rate_from,
            carry_from: raw.gather.carry_from,
            drop_sites_from: raw.gather.drop_sites_from,
            variant: raw.gather.variant,
            resource_task,
            fell_hp_per_second: {
                let value = raw.gather.fell_hp_per_second.unwrap_or(0.0);
                if !value.is_finite() || value < 0.0 {
                    return Err(ConfigError::Invalid {
                        key: "gather.fell_hp_per_second".to_owned(),
                        detail: format!("{value} is not a rate of hit points per second"),
                    });
                }
                value
            },
        },
        train: {
            let mut by_building: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
            for entry in raw.train.entry {
                let role_of = |role: &str| {
                    raw.role.get(role).copied().ok_or_else(|| ConfigError::Invalid {
                        key: format!("train.entry[{:?}]", entry.building),
                        detail: format!(
                            "{role:?} is not a role in this pack's [role] table, so no unit can be \
                             named by it"
                        ),
                    })
                };
                if entry.units.is_empty() {
                    return Err(ConfigError::Invalid {
                        key: format!("train.entry[{}].units", entry.building),
                        detail: "a building that trains nothing is a mistake, not a rule"
                            .to_owned(),
                    });
                }
                let building = role_of(&entry.building)?;
                let mut units = Vec::with_capacity(entry.units.len());
                for role in &entry.units {
                    units.push(role_of(role)?);
                }
                if by_building.insert(building, units).is_some() {
                    return Err(ConfigError::Invalid {
                        key: format!("train.entry[{}]", entry.building),
                        detail: "two entries for one building: the later would silently win"
                            .to_owned(),
                    });
                }
            }
            let queue_length = raw.train.queue_length;
            if queue_length == 0 {
                return Err(invalid(
                    path,
                    "train.queue_length",
                    "a building that may hold no units cannot train at all",
                ));
            }
            Train {
                by_building,
                queue_length,
            }
        },
        build: Build {
            repair_hp_per_second: MaybeFixed::from_f64(raw.build.repair_hp_per_second),
            drop_site_spacing: raw.build.drop_site_spacing,
            spaced_roles: raw.build.spaced_roles,
            farm: raw.build.farm.clone(),
            farm_spacing: raw.build.farm_spacing,
            blocking_roles: raw.build.blocking_roles,
            requires: raw.build.requires,
        },
        decay: Decay {
            enabled: raw.decay.enabled,
            rate_from: raw.decay.rate_from,
        },
        population: Population {
            cap_default: raw.population.cap_default,
            house_population: raw.population.house_population,
            town_center_population: raw.population.town_center_population,
        },
        start: Start {
            town_center: raw.start.town_center,
            villager: raw.start.villager,
            villagers_per_player: raw.start.villagers_per_player,
            scout: raw.start.scout.clone(),
            scouts_per_player: raw.start.scouts_per_player,
            techs: raw.start.techs,
            stock: raw.start.stock,
            civs: raw.start.civs,
            teams: raw.start.teams,
        },
        mark: {
            // **A mark must name a role.** A typo in `[mark]` would otherwise be a building drawn plain with no complaint,
            // which is the shape of mistake this loader refuses everywhere else.
            for role in raw.mark.keys() {
                if !raw.role.contains_key(role) {
                    return Err(ConfigError::Invalid {
                        key: format!("mark.{role}"),
                        detail: format!(
                            "{role:?} is not a role in this pack's [role] table, so no building can carry its mark"
                        ),
                    });
                }
            }
            raw.mark
        },
        role: raw.role,
        footprint: raw
            .footprint
            .iter()
            .map(|(role, raw)| {
                Footprint::from_rows(&raw.blocked)
                    .map(|mask| (role.clone(), mask))
                    .map_err(|detail| invalid("economy.footprint", role, detail))
            })
            .collect::<Result<_, _>>()?,
        tech_role: raw.tech_role,
        age: Ages {
            order: raw.age.order,
            requires: raw
                .age
                .requires
                .into_iter()
                .map(|(role, raw)| {
                    (
                        role,
                        AgeRequirement {
                            count: raw.count,
                            of: raw.of,
                            single: raw.single,
                        },
                    )
                })
                .collect(),
        },
        hunt: Hunt {
            herdable_roles: raw.hunt.herdable_roles,
            animal_roles: raw.hunt.animal_roles,
            resource: raw.hunt.resource,
            carcass_roles: raw.hunt.carcass_roles,
            leash_seconds: raw.hunt.leash_seconds,
            leash_tiles: raw.hunt.leash_tiles,
            butcher_roles: raw.hunt.butcher_roles,
        },
        metrics: MetricsSection {
            military_roles: raw.metrics.military_roles,
        },
        conquest: Conquest {
            surviving_classes: raw.conquest.surviving_classes,
        },
    })
}

pub fn load_economy_file(path: &Path) -> Result<Economy, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
        path: path.display().to_string(),
        detail: err.to_string(),
    })?;
    load_economy_str(&text, &path.display().to_string())
}
