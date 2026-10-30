//! Deterministic tick loop.
//!
//! Rules this crate exists to enforce (SPEC §5, `docs/mechanics/determinism.md`):
//!
//! - iteration is by entity id, never insertion or hash order;
//! - state is fixed-point only (D9), and per-tick displacement is *exact* — `speed /
//!   tick_rate` must land on a whole `1/scale` unit or the step is an error, not a rounded
//!   move;
//! - randomness comes from two separate seeded streams, so map generation cannot shift the
//!   simulation's draws;
//! - the state hash is a pure function of state, so two runs with the same inputs produce the
//!   same hash at every tick. That is the acceptance test for step 4 (SPEC §6.1).
//!
//! Numeric literals in this file are *algorithm constants* of named algorithms (PCG32, FNV-1a),
//! not tunable game values — D6 governs the latter. Everything game-shaped comes from
//! `ruage_data::Sim`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use ruage_data::{
    CommandApply, Cost, CostResource, Economy, Effects, Map, MaybeFixed, Resources,
    Sim as Settings, Unit, Units,
};

mod metrics;
mod path;
mod script;
pub use metrics::Metrics;
pub use path::{Walk, find_path};
use ruage_fixed::Fixed;
pub use script::{
    Outcome, Runner, SCENARIO_SCHEMA, Scenario, Step, StepKind, describe, load_scenario_file,
    load_scenario_str,
};

/// The largest number of players a match can have: eight, so four-versus-four (D20). It is a
/// constant rather than a runtime number because stockpiles are fixed-size arrays, and it is eight
/// rather than two because the cost of widening is paid once and the cost of a later re-refactor is
/// paid in every state hash.
pub const MAX_PLAYERS: usize = 8;

/// The largest number of stockpiled resources a match can have. Like `MAX_PLAYERS` this is a
/// constant rather than a runtime number because a stockpile is a fixed-size row, and the content
/// pack is refused at load when it defines more than this (D17: the pack says *which and how many*
/// resources exist, the engine only says how wide a row it can store).
///
/// Eight rather than four: four is what the reference pack needs, and the cost of the headroom is
/// a zeroed `u32` per player per unused column, while a later widening would mean re-baselining
/// every state hash.
pub const MAX_RESOURCES: usize = 8;

/// Stable handle for an entity. Ordered, so iteration by id is just sorting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u32);

/// One damage per second, in the integer millionths `pierce_dps` counts in (D67).
const DPS_ONE: i64 = 1_000_000;

/// A villager's waiting sites: unit id and tile, oldest first (D66).
type BuildQueue = BTreeMap<EntityId, VecDeque<(u32, (u16, u16))>>;

/// A building is a place; a unit is a thing that moves. Splitting them now keeps "villagers
/// gather into the town centre" from being a special case of "entities with zero speed".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Building,
    Unit,
    /// An arrow in the air. Not a unit: it costs no population, it cannot be attacked, and it exists
    /// only until it lands (D35).
    Projectile,
    /// A tree, a berry bush or a mine: a place that holds a quantity and can be worked down. The
    /// payload is the content pack's stockpile column, because that is what a deposit needs and it
    /// is the only thing the engine may know about a resource (D17).
    Resource(usize),
}

/// The subset of an entity the tick loop touches today. Fixed-point only (D9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entity {
    pub id: EntityId,
    /// 1 or 2. Zero is reserved for entities a test spawns with no owner.
    pub player: u8,
    pub kind: EntityKind,
    /// The dat unit id this entity is an instance of (`config/gamedata/units.toml`). `None` for
    /// a resource node: **unit 0 is a real dat unit**, so a zero here would read as a genuine id
    /// and hand a tree the stats of an academy.
    pub unit: Option<u32>,
    pub position: (Fixed, Fixed),
    /// How fast this entity can move. A capability, not a motion.
    pub speed: Fixed,
    /// Current motion, per tick, for movers with no task. Zero for a unit at rest.
    pub velocity: (Fixed, Fixed),
    pub hp: i32,
    /// What is left in a resource node, in fixed point so a take can be fractional. Zero for
    /// everything that is not a node.
    pub amount: Fixed,
    /// What the unit is doing. Buildings and nodes are always idle.
    pub task: Task,
    /// Present while a building is producing a unit: one job at a time in this slice.
    pub training: Option<Training>,
    /// Present while a building is going up. Progress is counted in ticks worked, so a
    /// construction finishes exactly at its build time even when HP/second is not a whole number.
    pub construction: Option<Construction>,
    /// Present while this building is researching a tech: one at a time, like production.
    pub researching: Option<Research>,
    /// What the unit is carrying, in fixed point; whole resources are taken on deposit.
    pub carry: Fixed,
    /// The villager variant currently in use (the task's unit id), if any.
    pub variant: Option<u32>,
}

/// What a hit is worth. Comes from `combat.toml`; the per-unit numbers come from `units.toml`.
#[derive(Debug, Clone, Copy)]
pub struct DamageRules {
    /// A stopped attack still lands for this much.
    pub minimum: i32,
    /// Armour assumed for a class the target does not list, high enough to contribute nothing.
    pub missing_armour: i32,
}

/// Damage of one hit, class by class: every attack class is reduced by the target's armour in that
/// class, sums that go negative contribute zero, and the total floors at `minimum`.
///
/// Elevation is not applied here: the slice has no terrain heights, so there is no elevation to
/// apply. `combat.toml` keeps the multipliers for when there is.
pub fn damage_dealt(attacker: &Unit, target: &Unit, rules: &DamageRules) -> i32 {
    damage_between(&attacker.attacks, &target.armours, rules)
}

/// The formula itself, over the *effective* lists: attacks and armours after every modifier the two
/// owners have researched. Splitting it this way keeps the arithmetic testable on its own and lets
/// the tick loop pass adjusted values without inventing a modified `Unit`.
pub fn damage_between(attacks: &[(i16, i16)], armours: &[(i16, i16)], rules: &DamageRules) -> i32 {
    let mut total: i64 = 0;
    for (class, amount) in attacks {
        let armour = armours
            .iter()
            .find(|(target_class, _)| target_class == class)
            .map_or(rules.missing_armour, |(_, value)| i32::from(*value));
        total += i64::from((i32::from(*amount) - armour).max(0));
    }
    i32::try_from(total).unwrap_or(i32::MAX).max(rules.minimum)
}

/// The same for a fixed-point quantity (speed, gather rate). A multiplier is scale-free, but an
/// **addition** is stated in the attribute's own unit - tiles per second, work per second - so it has
/// to be scaled before it is added to a raw value: nine speed additions ship in the data (+0.35 tiles
/// per second is one), and adding 0.35 to a raw fixed value would move the unit by a twenty-thousandth
/// of a tile per second and look like nothing happened.
fn scale_fixed(base: Fixed, scale: i64, modifiers: &[&ruage_data::Modifier]) -> Fixed {
    let mut raw = base.raw() as f64;
    // A kind-0 `set` replaces the base rather than adjusting it, and it has to be honoured here or the
    // attributes that only ever arrive as sets - Andean Sling's removal of a skirmisher's dead zone
    // (D44), the Koreans' reduction of a mangonel's - would silently do nothing. The last set wins, in
    // research order, which is the same rule every other modifier follows.
    if let Some(set) = modifiers
        .iter()
        .rev()
        .find(|modifier| modifier.op == ruage_data::ModOp::Set)
    {
        raw = set.value * scale as f64;
    }
    for modifier in modifiers.iter().filter(|m| m.op == ruage_data::ModOp::Add) {
        raw += modifier.value * scale as f64;
    }
    for modifier in modifiers
        .iter()
        .filter(|m| m.op == ruage_data::ModOp::Multiply)
    {
        raw *= modifier.value;
    }
    Fixed::from_raw(raw.round() as i64)
}

/// A unit's maximum health with its hit-point modifiers applied, additions first. Floored at 1: a
/// modifier that would zero a maximum is content we refuse to believe rather than a unit that cannot
/// exist.
fn scaled_health(unit: &Unit, modifiers: &[&ruage_data::Modifier]) -> i32 {
    let mut hp = f64::from(unit.hp);
    for modifier in modifiers.iter().filter(|m| m.op == ruage_data::ModOp::Add) {
        hp += modifier.value;
    }
    for modifier in modifiers
        .iter()
        .filter(|m| m.op == ruage_data::ModOp::Multiply)
    {
        hp *= modifier.value;
    }
    i32::try_from(hp.round() as i64).unwrap_or(unit.hp).max(1)
}

/// Add `amount` to a class in an attack/armour list, creating the entry when the unit does not list
/// that class: an upgrade can grant an attack class a unit does not otherwise have.
fn add_to_class(list: &mut Vec<(i16, i16)>, class: i16, amount: i16) {
    match list.iter_mut().find(|(entry, _)| *entry == class) {
        Some((_, value)) => *value = value.saturating_add(amount),
        None => list.push((class, amount)),
    }
}

/// Multiply a class's value, rounded half away from zero: a multiplier on a class the unit does not
/// have is a no-op, because there is nothing to scale.
fn multiply_class(list: &mut [(i16, i16)], class: i16, factor: f64) {
    if let Some((_, value)) = list.iter_mut().find(|(entry, _)| *entry == class) {
        *value = i16::try_from((f64::from(*value) * factor).round() as i64).unwrap_or(*value);
    }
}

/// A tech being researched: which one, and how long it has left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Research {
    pub tech: u32,
    pub remaining_ticks: u32,
    /// The tick an order lands is not a working tick (see `Training::fresh`), and research has the
    /// same rule, so a 40 s tech at 20 Hz lands exactly 800 ticks after it starts.
    pub fresh: bool,
}

/// Where a building's finished units go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RallyPoint {
    /// Walk to this tile and stop.
    Tile(u16, u16),
    /// Stay inside the building: the only way a production building holds a unit (D38).
    Inside,
}

/// How a match ended. A side is out when nothing that *counts* of theirs still stands - no unit, no
/// building - where content names the families that do not count (walls, farms; D59). The town centre
/// is not special: losing it is a setback, not a defeat, which is what the original game does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchOutcome {
    Victory {
        winner: u8,
    },
    /// Every side lost its seat on the same tick.
    Draw,
}

/// What one player knows about one tile (D50).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileVision {
    /// Never looked at.
    Unseen,
    /// Looked at once and remembered: no unit sees it now, but the ground is known.
    Explored,
    /// Somebody of this player sees it now.
    Visible,
}

/// How much of a fight a unit picks on its own. Aggressive is the default and is what every unit did
/// before stances existed, so a pack that says nothing keeps the behaviour it had (D48).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stance {
    /// Chase anything in reach, wherever it goes.
    #[default]
    Aggressive,
    /// Chase, but only near the ground the stance was given on, and go back to it afterwards.
    Defensive,
    /// Never move to fight; still strike what is already within reach.
    StandGround,
    /// Never fight at all.
    NoAttack,
}

/// A shot decided this tick: who fired it, from where, where it will land, what it was aimed at and
/// what it does when it gets there. Created after the movement pass, because the entity list is
/// index-stable while positions are applied.
/// A blow that has landed: who dealt it, whom it landed on, and where it came down. The shooter rides
/// along because ranged damage has to be attributable exactly as melee damage is (D71).
struct Landing {
    shooter: EntityId,
    target: EntityId,
    damage: i32,
    aim: (Fixed, Fixed),
    blast: Fixed,
}

/// A shot leaving the world this tick: who loosed it, from where, at what, and for how much. Recorded
/// rather than spawned in place, because the entity list is index-stable for the length of the pass.
struct Shot {
    player: u8,
    shooter: EntityId,
    from: (Fixed, Fixed),
    aim: (Fixed, Fixed),
    target: EntityId,
    damage: i32,
    blast: Fixed,
}

/// A training order that passed every check: what it costs, how long it takes, and what comes out.
struct TrainPlan {
    costs: Vec<Cost>,
    ticks: u32,
    hp: i32,
    speed: Fixed,
    player: u8,
}

/// A unit being produced. Progress counts ticks remaining rather than accumulated work, so the
/// finish lands exactly on `create_time x tick_rate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Training {
    pub unit: u32,
    pub remaining_ticks: u32,
    /// Resolved when the order lands, so the tick loop never reads a table.
    pub hp: i32,
    pub speed: Fixed,
    /// True on the tick the order lands. That tick is spent starting the job, not working on it,
    /// so a 25 s unit ordered now appears 500 ticks from now rather than 499.
    pub fresh: bool,
}

/// A research order that passed every check: what it costs, how long it takes, and whose it is.
#[derive(Debug, Clone)]
struct ResearchPlan {
    costs: Vec<Cost>,
    ticks: u32,
    player: u8,
}

/// A build order that passed every check: what it costs, how long it takes, whose money it is.
struct BuildPlan {
    total_ticks: u32,
    max_hp: i32,
    costs: Vec<Cost>,
    player: u8,
}

/// A building under construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Construction {
    /// Builder-ticks contributed so far. Counts only while somebody is actually working.
    pub progress_ticks: u32,
    /// Builder-ticks needed: `create_time x tick_rate`.
    pub total_ticks: u32,
    /// The finished hit points, so progress can be turned into HP for display and combat.
    pub max_hp: i32,
}

/// Unit work in progress. A state machine small enough to hash and to explain: walking to a node
/// and taking from it. Returning to a drop site is the next increment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Task {
    #[default]
    Idle,
    /// Putting up a building at `site`.
    Build { site: EntityId },
    /// Walking to a place with nothing to do there. The rally point's task: the unit goes, then stops.
    Move { to: (Fixed, Fixed) },
    /// Walking to a place *looking for a fight*: attack-move and patrol are the same walk with two
    /// arrival rules, and the difference between them lives in `World::patrols` rather than in the
    /// task. What a hunt does in the tick loop is: take the nearest enemy in reach if there is one,
    /// otherwise step along the route. A fight it starts is an ordinary `Task::Attack`, and the walk
    /// it interrupted is remembered in `World::resuming` so it can pick the order back up (D47).
    Hunt { to: (Fixed, Fixed) },
    /// Inside a building. A garrisoned unit still exists - it costs population and dies with its
    /// shelter - but it is not on the map: it cannot act, and it cannot be attacked.
    Garrisoned { building: EntityId },
    /// A shot in flight: where it will land, what it does when it gets there, and who it was meant
    /// for. The aim is a position, not the target, so an arrow at a unit that then walks away lands
    /// where it was fired - and finds nothing worth hurting.
    Fly {
        target: EntityId,
        /// Who loosed it. A shot used to carry only its damage, so ranged damage was unattributable:
        /// retaliation could not answer an archer and the last blow could not be named (D71).
        shooter: EntityId,
        damage: i32,
        aim: (Fixed, Fixed),
        /// How far the landing hurts everything around it, in engine units. Zero is a point, which is
        /// what every non-siege unit means, and it rides on the arrow because the arrow is the only
        /// thing that knows where it came down (D51).
        blast: Fixed,
    },
    /// Fighting `target`. Damage and reload are resolved when the order lands, so the tick loop
    /// never reads a table and a replay cannot name its own numbers.
    Attack {
        target: EntityId,
        damage: i32,
        reload_ticks: u32,
        /// Ticks until the next hit. Zero on the landing tick, so the first blow is immediate.
        cooldown_ticks: u32,
        /// How far this attacker reaches, in engine fixed point. Zero means melee: adjacency is the
        /// only place it can strike from. Resolved at order time like damage and reload, so a tech that
        /// lands mid-fight does not move an arrow already in the air.
        range: Fixed,
        /// The dead zone, in engine fixed point: this attacker cannot strike anything closer than this,
        /// and backs away until it can. Resolved at order time like `range` (D44).
        min_range: Fixed,
    },
    Gather {
        node: EntityId,
        /// Resource per tick, resolved from the variant's `work_rate` at the engine scale.
        rate_per_tick: Fixed,
        /// The variant's carry, in whole resources.
        carry_cap: u32,
        /// The fraction accumulated toward the next **whole** unit, always under one unit. Resources are whole numbers:
        /// a villager carries `10` wood, never `9.27`, and a node holds `100`, never `87.4` (0 A.D.'s
        /// `ResourceGatherer` takes `GATHER_AMOUNT = 1` at a time and documents its carry as an *integer* amount). This
        /// is the accumulator that was missing here, and its absence is where `0.03`-of-a-unit carries came from.
        progress: Fixed,
    },
    /// Carrying a full load to somewhere that will take it. The node is remembered so the
    /// villager can go back to work once the load is dropped.
    Return {
        node: EntityId,
        drop: EntityId,
        /// The stockpile column this load banks into, taken from the node when the order landed.
        slot: usize,
        rate_per_tick: Fixed,
        carry_cap: u32,
    },
}

/// Inputs to the simulation. A replay is a list of these with the ticks they land on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Take from a resource node. The rate, the capacity and the variant come from the tables,
    /// not from the command: a replay must not be able to name its own numbers.
    Gather { villager: EntityId, node: EntityId },
    /// Stand a unit down: drop its path and return it to idle.
    Stop { unit: EntityId },
    /// Raze something of your own. The player is named because ownership is the only thing there is
    /// to check: every other command inherits its owner from the actor, and a deletion has no actor.
    Delete { player: u8, entity: EntityId },
    /// Walk to a tile and stop. The ordinary order, and the one the rally point uses.
    Move { unit: EntityId, to: (u16, u16) },
    /// Walk to a tile, fighting whatever enemy it meets on the way.
    AttackMove { unit: EntityId, to: (u16, u16) },
    /// Walk to a tile, then back to where the order was given, forever - fighting on the way.
    Patrol { unit: EntityId, to: (u16, u16) },
    /// How much of a fight this unit picks on its own. The ground it stands on becomes its post, which
    /// is what a defensive stance returns to.
    Stance { unit: EntityId, stance: Stance },
    /// Attack another player's unit or building.
    Attack {
        attacker: EntityId,
        target: EntityId,
    },
    /// Produce a unit at a building. Cost and duration come from the unit table; whether this
    /// building may make this unit comes from `economy.toml` (the dat does not carry that edge).
    Train { building: EntityId, unit: u32 },
    /// Put up a building at a tile. Cost and duration come from the unit table.
    Build {
        villager: EntityId,
        unit: u32,
        at: (u16, u16),
    },
    /// Research a tech at a building. Price, duration, where it may happen and what it applies all
    /// come from the tech and effect tables - a replay carries the id, never the numbers.
    Research { building: EntityId, tech: u32 },
    /// Where a building's finished units go. `Some(tile)` walks them there; `Some` of the building's own
    /// tile keeps them inside it, which is the only way a production building holds anybody (D38);
    /// `None` clears the rally and leaves them beside the building.
    Rally {
        building: EntityId,
        point: Option<(u16, u16)>,
    },
    /// Put a unit inside a building. The capacity is the building's own `garrison` field, so the only
    /// numbers involved are the data's.
    Garrison { unit: EntityId, building: EntityId },
    /// Let everyone out of a building.
    Ungarrison { building: EntityId },
    Spawn {
        id: EntityId,
        player: u8,
        kind: EntityKind,
        unit: Option<u32>,
        position: (Fixed, Fixed),
        speed: Fixed,
        hp: i32,
        amount: u32,
    },
}

#[derive(Debug)]
pub enum SimError {
    TooManyEntities {
        max: u32,
    },
    DuplicateEntity(EntityId),
    Displacement {
        id: EntityId,
        detail: String,
    },
    Overflow(EntityId),
    /// A config named a unit id the generated table does not contain.
    MissingUnit {
        id: u32,
    },
    /// The dat value is `-1.0`, i.e. unresolved, so it cannot enter state.
    Unresolved {
        id: u32,
        detail: String,
    },
    /// No free tile near the town centre to place a starting villager.
    NoSpace {
        player: u8,
    },
    MissingEntity(EntityId),
    /// A gather order whose target is not a resource node.
    NotAResource {
        id: EntityId,
    },
    /// A gather order given to something that is not a unit.
    NotAUnit {
        id: EntityId,
    },
    /// The world was built without the tables, so it cannot resolve a gather order.
    NoTables,
    /// The economy names a task with no variant, or a variant with no usable rate.
    NoVariant {
        task: String,
        detail: String,
    },
    /// There is no walkable route between two tiles.
    NoPath {
        from: (u16, u16),
        to: (u16, u16),
    },
    /// A player number outside `1..=MAX_PLAYERS`.
    PlayerOutOfRange {
        player: u8,
    },
    /// You cannot attack your own side.
    FriendlyFire {
        id: EntityId,
    },
    /// Resources are gathered, not fought.
    NotATarget {
        id: EntityId,
    },
    /// This unit carries no weapon.
    Unarmed {
        id: EntityId,
    },
    /// A cost names a resource id this content pack does not carry.
    UnknownResource {
        dat_id: u16,
    },
    /// A cost names a resource this content pack does not define.
    UnknownResourceName {
        name: String,
    },
    /// Content named a role the economy does not define.
    NoRole {
        role: String,
    },
    /// A research order names a tech the tech table does not contain.
    MissingTech {
        id: u32,
    },
    /// A tech names an effect the effect table does not contain.
    MissingEffect {
        id: u32,
    },
    /// This building is not where that tech is researched (the tech's own `location`).
    CannotResearchHere {
        building: u32,
        tech: u32,
    },
    /// The building is already researching something.
    AlreadyResearching {
        id: EntityId,
    },
    /// This player already has the tech.
    AlreadyResearched {
        tech: u32,
    },
    /// This player is already researching the tech at another building.
    TechInProgress {
        tech: u32,
    },
    /// This building's queue is full: the unit being made plus everything behind it.
    QueueFull {
        id: EntityId,
        length: u32,
    },
    /// This unit is inside a building and cannot act until it is let out.
    Garrisoned {
        id: EntityId,
    },
    /// The unit is too far away to walk into the building: it has to be standing on it or beside it.
    TooFar {
        id: EntityId,
    },
    /// A tile outside the map.
    OffMap {
        tile: (u16, u16),
    },
    /// Only a building that can fight takes units in from outside.
    CannotGarrisonHere {
        id: EntityId,
    },
    /// The building is too badly damaged to hold a garrison: it has ejected the last one.
    GarrisonTooDamaged {
        id: EntityId,
    },
    /// The building has no room for another unit.
    GarrisonFull {
        id: EntityId,
        capacity: u32,
    },
    /// The target is on the attacker's own team.
    AlliedTarget {
        id: EntityId,
    },
    /// The player's civilisation does not have this unit at all.
    UnitBarred {
        unit: u32,
        civ: String,
    },
    /// The unit is an upgrade, and the tech that upgrades into it has not been researched.
    UnitNotUpgraded {
        unit: u32,
        tech: u32,
    },
    /// The player has no way to have this unit yet: some tech makes it available, and that tech has not
    /// been researched.
    UnitUnavailable {
        unit: u32,
    },
    /// A building's prerequisite has never been built by this player (D63): a siege workshop needs a
    /// blacksmith, and "built once" is enough - the game does not take the right away again.
    MissingBuilding {
        need: String,
    },
    /// An age needs buildings of the current age that the player does not have standing (D65).
    AgeNeedsBuildings {
        need: u32,
        have: u32,
    },
    /// The player's civilisation cannot research the tech at all.
    TechBarred {
        tech: u32,
        civ: String,
    },
    /// Content names a civilisation the civ table does not have.
    UnknownCiv {
        name: String,
    },
    /// The tech lists a prerequisite this player has not researched.
    MissingPrerequisite {
        tech: u32,
        missing: u32,
        /// The content word for the prerequisite, when the pack has one.
        role: Option<String>,
    },
    /// The pack defines more resources than a stockpile has columns for.
    TooManyResources {
        count: usize,
        max: usize,
    },
    /// The player cannot pay for the order. The name is the pack's, so the message says "wood"
    /// only because the pack calls slot 0 that (D17).
    NotEnough {
        resource: String,
        need: u32,
        have: u32,
    },
    /// Something already stands on the chosen tile.
    SiteOccupied {
        tile: (u16, u16),
    },
    /// The chosen tile is not open ground.
    SiteBlocked {
        tile: (u16, u16),
    },
    /// This building does not make this unit.
    NotTrainableHere {
        building: u32,
        unit: u32,
    },
    /// The building is still being built.
    BuildingNotReady {
        id: EntityId,
    },

    /// Training happens at buildings, not at units.
    TrainNeedsABuilding {
        id: EntityId,
    },
    /// The chosen unit is not a building (it can move, or it has no creation time).
    NotABuilding {
        unit: u32,
    },
    /// A delete order for something another player owns, or for neutral scenery.
    NotYours {
        id: EntityId,
        player: u8,
    },
    /// A delete order for something that is not a unit or a building: an arrow in flight, which is
    /// not a thing a player owns in any sense worth acting on.
    NotDeletable {
        id: EntityId,
    },
    /// The unit's stance does not allow it to fight.
    StanceForbidsAttack {
        id: EntityId,
    },
    /// The match is over: it accepts no further orders.
    MatchOver,
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimError::TooManyEntities { max } => write!(
                f,
                "max_entities = {max} reached; refusing to spawn more (raise sim.max_entities)"
            ),
            SimError::DuplicateEntity(id) => write!(f, "entity {} already exists", id.0),
            SimError::Displacement { id, detail } => write!(
                f,
                "entity {} cannot move exactly at this scale/tick rate: {detail}",
                id.0
            ),
            SimError::Overflow(id) => write!(f, "entity {} position overflowed i64", id.0),
            SimError::MissingUnit { id } => write!(
                f,
                "unit {id} is not in the unit table; the config points at an id the data lacks"
            ),
            SimError::Unresolved { id, detail } => {
                write!(f, "unit {id} has an unresolved value: {detail}")
            }
            SimError::NoSpace { player } => write!(
                f,
                "no free tile next to player {player}'s town centre to place a villager"
            ),
            SimError::MissingEntity(id) => write!(f, "entity {} does not exist", id.0),
            SimError::NotAResource { id } => {
                write!(f, "entity {} is not a resource node", id.0)
            }
            SimError::NotAUnit { id } => write!(f, "entity {} is not a unit", id.0),
            SimError::NoTables => write!(
                f,
                "this world has no unit/economy tables, so it cannot resolve a gather order"
            ),
            SimError::NoVariant { task, detail } => {
                write!(f, "gather task {task:?} cannot be resolved: {detail}")
            }
            SimError::NoPath { from, to } => write!(
                f,
                "no walkable route from {:?} to {:?}: the target is walled off",
                from, to
            ),
            SimError::UnknownResource { dat_id } => write!(
                f,
                "a cost names resource id {dat_id}, which this content pack does not carry: \
                 refusing rather than paying from another slot"
            ),
            SimError::UnknownResourceName { name } => write!(
                f,
                "a cost names the resource {name:?}, which this content pack does not define: \
                 refusing rather than paying from another slot"
            ),
            SimError::NoRole { role } => write!(
                f,
                "{role:?} is not a role in economy.toml's [role] table, so there is no unit behind it"
            ),
            SimError::MissingTech { id } => write!(
                f,
                "tech {id} is not in the tech table; the order names something the data lacks"
            ),
            SimError::MissingEffect { id } => write!(
                f,
                "a tech names effect {id}, which is not in the effect table, so it cannot be applied"
            ),
            SimError::CannotResearchHere { building, tech } => write!(
                f,
                "unit {building} is not where tech {tech} is researched: see the tech's `location`"
            ),
            SimError::AlreadyResearching { id } => {
                write!(f, "entity {} is already researching a tech", id.0)
            }
            SimError::AlreadyResearched { tech } => {
                write!(f, "tech {tech} has already been researched by this player")
            }
            SimError::TechInProgress { tech } => write!(
                f,
                "tech {tech} is already being researched at another building"
            ),
            SimError::Garrisoned { id } => {
                write!(f, "entity {} is garrisoned and cannot act", id.0)
            }
            SimError::TooFar { id } => write!(
                f,
                "entity {} is not next to the building: move it there before garrisoning",
                id.0
            ),
            SimError::GarrisonFull { id, capacity } => {
                write!(f, "entity {} is full: it holds {capacity} units", id.0)
            }
            SimError::GarrisonTooDamaged { id } => {
                write!(f, "entity {} is too badly damaged to hold a garrison", id.0)
            }
            SimError::OffMap { tile } => {
                write!(f, "{tile:?} is not on the map: a place has to be somewhere")
            }
            SimError::CannotGarrisonHere { id } => write!(
                f,
                "entity {} holds only the units it has just trained: it cannot be entered from outside",
                id.0
            ),
            SimError::AlliedTarget { id } => write!(
                f,
                "entity {} is on the attacker's team: allies are not targets",
                id.0
            ),
            SimError::UnitBarred { unit, civ } => write!(
                f,
                "unit {unit} is not available to the {civ}: it is not in their tech tree"
            ),
            SimError::UnitNotUpgraded { unit, tech } => write!(
                f,
                "unit {unit} is an upgrade: tech {tech} has not been researched by this player"
            ),
            SimError::MissingBuilding { need } => {
                write!(f, "this building needs a {need} first")
            }
            SimError::AgeNeedsBuildings { need, have } => write!(
                f,
                "this age needs {need} buildings of the current age and this player has {have}"
            ),
            SimError::UnitUnavailable { unit } => write!(
                f,
                "unit {unit} is not available yet: a tech that makes it available has not been researched"
            ),
            SimError::TechBarred { tech, civ } => {
                write!(
                    f,
                    "tech {tech} is not available to the {civ}: it is on their restriction list"
                )
            }
            SimError::UnknownCiv { name } => write!(
                f,
                "{name:?} is not a civilisation in civs.toml, so no match can be played as it"
            ),
            SimError::MissingPrerequisite {
                tech,
                missing,
                role,
            } => match role {
                Some(role) => write!(
                    f,
                    "tech {tech} needs {role} ({missing}) first, and this player has not researched it"
                ),
                None => write!(
                    f,
                    "tech {tech} needs tech {missing} first, and this player has not researched it"
                ),
            },
            SimError::TooManyResources { count, max } => write!(
                f,
                "the content pack defines {count} resources; a stockpile has {max} columns \
                 (MAX_RESOURCES in the engine, because a stockpile is a fixed-size row)"
            ),
            SimError::NotEnough {
                resource,
                need,
                have,
            } => write!(
                f,
                "not enough {resource}: {need} needed, {have} in the bank"
            ),
            SimError::SiteOccupied { tile } => {
                write!(f, "something already stands at {:?}", tile)
            }
            SimError::SiteBlocked { tile } => {
                write!(
                    f,
                    "{:?} is not open ground: a building cannot go there",
                    tile
                )
            }
            SimError::NotTrainableHere { building, unit } => write!(
                f,
                "building {building} does not train unit {unit}: see [train] in economy.toml"
            ),
            SimError::BuildingNotReady { id } => {
                write!(f, "entity {} is still under construction", id.0)
            }
            SimError::PlayerOutOfRange { player } => write!(
                f,
                "player {player} is outside 1..={MAX_PLAYERS}: a match has at most {MAX_PLAYERS} sides"
            ),
            SimError::FriendlyFire { id } => {
                write!(f, "entity {} is on your own side", id.0)
            }
            SimError::NotATarget { id } => {
                write!(
                    f,
                    "entity {} cannot be attacked: resources are gathered",
                    id.0
                )
            }
            SimError::Unarmed { id } => {
                write!(f, "entity {} has no attack: it cannot fight", id.0)
            }
            SimError::QueueFull { id, length } => write!(
                f,
                "entity {} already holds {length} units: the queue is full",
                id.0
            ),
            SimError::TrainNeedsABuilding { id } => {
                write!(f, "entity {} is not a building: only buildings train", id.0)
            }
            SimError::NotABuilding { unit } => {
                write!(
                    f,
                    "unit {unit} is not a building: it has no build time or it moves"
                )
            }
            SimError::NotYours { id, player } => {
                write!(f, "entity {} does not belong to player {player}", id.0)
            }
            SimError::NotDeletable { id } => {
                write!(f, "entity {} is not a unit or a building", id.0)
            }
            SimError::StanceForbidsAttack { id } => write!(
                f,
                "entity {} is standing down: its stance does not allow it to attack",
                id.0
            ),
            SimError::MatchOver => write!(f, "the match is over: no further orders are accepted"),
        }
    }
}

impl std::error::Error for SimError {}

/// PCG32 (pcg32_fast, the classic variant named in `config/sim.toml`).
///
/// Constants are from the reference implementation and are part of the algorithm's identity:
/// changing them changes every replay, so they are not tunables.
struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    const MULTIPLIER: u64 = 6_364_136_223_846_793_005;

    fn new(seed: u64, stream: u64) -> Self {
        let mut rng = Pcg32 {
            state: 0,
            inc: (stream << 1) | 1,
        };
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(Self::MULTIPLIER).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }
}

/// Stream identifiers are the ASCII of their config names, so a logged seed is self-describing.
const SIM_STREAM: u64 = 0x0073_696d; // "sim"
const MAP_STREAM: u64 = 0x006d_6170; // "map"

/// FNV-1a 64-bit. Chosen because it is a dozen lines with no dependency and no version drift:
/// the hash must mean the same thing in ten years, on any machine.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

/// One tick of movement from `from` toward `to`, in raw fixed units on each axis.
///
/// Each axis is clamped independently, so the unit lands exactly on the target instead of
/// overshooting and oscillating. This is a straight line with no obstacle test: pathing replaces
/// it, and until then a villager can walk through a tree.
fn step_towards(from: (Fixed, Fixed), to: (Fixed, Fixed), delta: i64) -> (i64, i64) {
    let axis = |from: Fixed, to: Fixed| -> i64 {
        let gap = to.raw() - from.raw();
        if gap.abs() <= delta {
            gap
        } else if gap > 0 {
            delta
        } else {
            -delta
        }
    };
    (axis(from.0, to.0), axis(from.1, to.1))
}

/// The nearest free walkable tile around `from`, searched in ring order so the choice is
/// deterministic and independent of any hash iteration.
fn free_neighbour(
    map: &Map,
    from: (u16, u16),
    used: &std::collections::BTreeSet<(u16, u16)>,
) -> Option<(u16, u16)> {
    for radius in 1..=8i32 {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx.abs().max(dy.abs()) != radius {
                    continue;
                }
                let (Some(x), Some(y)) = (
                    u16::try_from(i32::from(from.0) + dx).ok(),
                    u16::try_from(i32::from(from.1) + dy).ok(),
                ) else {
                    continue;
                };
                if map.is_walkable(x, y) && !used.contains(&(x, y)) {
                    return Some((x, y));
                }
            }
        }
    }
    None
}

/// An animal's chase, as three states in one entry (D71): absent means grazing at home, `chasing` means
/// it has a target of its own, and the same entry with `chasing` false means it is walking back to `home`.
/// The origin is recorded at the *first* blow, so a blow landed on the way home resumes the chase from
/// where it stands rather than inventing a new home.
#[derive(Debug, Clone, Copy)]
struct Leash {
    home: (Fixed, Fixed),
    /// The tick interest expires at. Each blow pushes it out; when it passes, the animal goes home.
    until: u64,
    chasing: bool,
}

/// A body's clock (items 4 and 6). `per_tick` is how much meat it loses a tick - zero for a body that was never
/// butchered - and `expires` is the tick it leaves the map regardless, derived from the species' own meat and rot
/// rate. Both numbers come from the pack, so a pack whose animals change changes the lifetime with them.
#[derive(Debug, Clone, Copy)]
struct Rot {
    per_tick: i64,
    expires: u64,
}

pub struct World {
    tick: u64,
    tick_rate_hz: u32,
    scale: i64,
    max_entities: u32,
    command_apply: CommandApply,
    entities: Vec<Entity>,
    queue: Vec<Command>,
    /// Cloned at seed time so a command can be resolved without threading references through the
    /// tick loop. A replay carries ids, never numbers.
    units: Option<Units>,
    economy: Option<Economy>,
    /// Per player, per resource. Index is `player - 1`; player 0 has no stockpile and neither does
    /// anything above `MAX_PLAYERS`. The width is a constant because the row is an array, and the
    /// pack is checked against it when it is loaded - so a slot is always inside the row.
    stock: [[u32; MAX_RESOURCES]; MAX_PLAYERS],
    /// Terrain, copied once at seed time. Static for a match.
    walk: Option<Walk>,
    /// Damage constants from `combat.toml`, set with `load_combat`.
    damage_rules: Option<DamageRules>,
    /// How fast a shot flies, resolved once at load: zero means the pack shoots instantly.
    combat_projectile_speed: Option<Fixed>,
    /// The same speed as a per-tick step, because the tick loop must not divide.
    projectile_step: i64,
    /// How far off the mark a shot lands when its accuracy roll fails, in engine units. Zero means every
    /// shot is aimed true (D43).
    accuracy_scatter: i64,
    /// Fraction of its health (in engine units) at or below which a building ejects its garrison.
    garrison_eject_at: Fixed,
    /// Remaining waypoints per unit, in tiles. Part of state: it decides where the unit goes next.
    paths: BTreeMap<EntityId, Vec<(u16, u16)>>,
    /// The tech and effect tables, when they have been loaded. A research order is resolved against
    /// them the way damage is resolved against `combat.toml`: once, at order time.
    techs: Option<ruage_data::Techs>,
    effects: Option<Effects>,
    /// The civilisations table and the civ this match is played as. The bar list is *derived* from the
    /// civ's tree effect whenever a research is ordered - it is content, so keeping a copy in state
    /// would be a second source of truth for the same fact.
    civs: Option<ruage_data::Civs>,
    /// Team id per player, resolved from `[start] teams` at seeding. Empty in content means every
    /// player is their own team, so a free-for-all needs no configuration.
    team_by_player: [u8; MAX_PLAYERS],
    /// Whether content named teams at all. Only then are they hashed, so a world with no teams hashes
    /// exactly as it did before teams existed.
    teams_named: bool,
    /// Which civilisation each player is, once resolved against the table above. Index 0 is player 1.
    /// Part of state (hashed), because it changes what the world will allow.
    civ_by_player: [Option<u32>; MAX_PLAYERS],
    /// The effect ids a player has by civilisation rather than by research: the team bonus. Derived
    /// from `civ_by_player`, so it is not hashed separately. The data writes a team bonus with the same
    /// command shapes as a tech's effect - the Slavs' is twelve kind-0 population sets (D41).
    bonus_effects: [Vec<u32>; MAX_PLAYERS],
    /// Units that some tech *makes available* (kind-2 rows with a positive flag). Derived from the
    /// effects table at load: content, not state, so it is not hashed.
    gated_units: BTreeSet<u32>,
    /// Unit id -> the tech whose effect upgrades into it. A pikeman is not "available", it is an
    /// *upgrade*: the barracks may train one only once the Pikeman tech has been researched.
    upgrade_tech: BTreeMap<u32, u32>,
    /// Techs that cost nothing, take no time, have no home building and do nothing but make units
    /// available. They are bookkeeping - "the barracks exists now" - so the engine applies them as soon
    /// as their prerequisites are met instead of asking a player to spend a click on them.
    availability_techs: Vec<u32>,
    /// Re-entrancy guard for `grant_availability`, which grants techs that may unlock further ones.
    resolving_availability: bool,
    /// Whether the opening stockpile has been applied. Part of state, because it decides whether a
    /// later `load_resources` re-applies it and wipes what the players have gathered since.
    start_stock_applied: bool,
    /// The unit classes whose buildings keep a player alive (`[conquest] surviving_classes`, D59b).
    /// Content, not state - it does not change during a match - so it is not hashed.
    conquest_classes: BTreeSet<i16>,
    /// What each player has ever **finished building**, by unit family (D63). Building prerequisites are
    /// about history, not about the map: losing the blacksmith does not take the siege workshop away, so
    /// the fact has to be remembered. Part of state - it changes what the world will allow.
    built: BTreeSet<(u8, u32)>,
    /// Build orders a villager is waiting to start, in the order they were given (D66). A second `Build`
    /// while one is running queues instead of replacing: without it a builder handed a new site abandons
    /// the half-built one, which is exactly what a scenario or a player would never mean.
    build_queue: BuildQueue,
    /// Unit families whose meat can be gathered: both animal kinds, from content's `[hunt] animal_roles`.
    /// Resolved at seed so the engine never learns what a sheep is (D17, D70).
    animals: BTreeSet<u32>,
    /// The smaller list: animals that walk to a drop site when a villager works them, from
    /// `[hunt] herdable_roles`. A boar is food but not herdable (D70).
    herdable: BTreeSet<u32>,
    /// Bodies: what a slaughtered animal becomes in place, from `[hunt] carcass_roles`. A carcass is gathered
    /// from, and it is a named unit rather than an inference from having no health (D70).
    carcasses: BTreeSet<u32>,
    /// Animal unit -> the body it leaves, from the pack's own `dead_unit`. Resolved at seed so a death in the
    /// tick needs a map lookup and not a table read (item 2, D70/D71).
    animal_bodies: BTreeMap<u32, u32>,
    /// Unit families whose **last** blow leaves meat: `[hunt] butcher_roles`. Held as ids *and* families, so the
    /// tick needs no table walk to answer it (D70).
    butchers: BTreeSet<u32>,
    /// Animals that carry an attack, ids and families: only these answer a blow with one (item 3).
    armed_animals: BTreeSet<u32>,
    /// A shepherd's kill in progress: villager -> the animal it was sent to work and is now striking. The gather
    /// order is replaced by the fight, so it is remembered here to be resumed on the body (D70, item 3).
    slaughter: BTreeMap<EntityId, EntityId>,
    /// The herdable animals alive, by entity: the only things that change hands (item 5). Filled at seed and on
    /// spawn so the tick walks a handful rather than filtering the world every frame.
    herdable_ids: BTreeSet<EntityId>,
    /// How far each unit can see, squared, in raw units - `los` from the pack, resolved once at seed (item 5).
    sight: BTreeMap<u32, i64>,
    /// Rot per body (item 4) and the tick it must be gone by whatever happens (item 6). Fixed at death from the
    /// **living animal's** `decay_rate`, which is food per second - the body's own record carries a flat 1.0 that
    /// is not a rot rate at all.
    rots: BTreeMap<EntityId, Rot>,
    /// The same rate per animal unit, ids and families, resolved once at seed so a death never reads a table.
    animal_rot: BTreeMap<u32, i64>,
    /// How long a body of each animal lies on the map, in ticks: as long as its **full** carcass would take to rot
    /// (meat / decay rate). Derived from the pack, not invented (item 6).
    animal_span: BTreeMap<u32, u64>,
    /// Villager shares, by villager count: `(at_villagers, [(stockpile slot, percent)])`, the last row whose
    /// `at_villagers` the player has reached being the one that applies. This is the game's own AI shape - shares, not
    /// assignments - and the engine is what moves an idle villager to meet them (2026-10-29).
    shares: Vec<(usize, Vec<(usize, i32)>)>,
    /// When the shares were last enforced, so the pass costs a scan once a second rather than every tick.
    shares_scan_tick: u64,
    /// Nodes a share assignment failed to reach, and the tick it failed: retried after `share_retry_ticks` rather than
    /// every second (the `lastFailedGather` bookkeeping 0 A.D. keeps).
    share_failed: BTreeMap<EntityId, u64>,
    /// Nodes that have given their last wood: removed once no villager is working them, so the tile they stood on becomes
    /// ground a unit may cross (the owner's rule, 2026-10-30 - a spent tree disappears and frees its tile).
    spent_nodes: BTreeSet<EntityId>,
    /// A worker the shares have decided to move to another column, and the node waiting for it: the load in its hands is
    /// banked **first**, at its own drop site, and only then does it take up the new work (the owner's rule, 2026-10-30).
    pending_gather: BTreeMap<EntityId, EntityId>,
    /// Villagers the policy has already given an order to, and when: the guard that stops the pass re-issuing the same
    /// gather order - and paying for a path search - every second.
    share_moved: BTreeMap<EntityId, u64>,
    /// How long a node that could not be reached is left alone, in ticks (30 s at 20 Hz). A node does not get closer.
    share_retry_ticks: u64,
    /// How long a villager the policy has just given an order to is left alone, in ticks (2 s at 20 Hz). Short, because
    /// an order does not always stick - the node may be dead by the time the villager arrives - but not absent, because
    /// re-issuing the same order every second is a path search every second for no change.
    share_move_ticks: u64,
    /// Buildings with a construction in progress, by id. A maintained set rather than a scan: `settle_blind_sites`
    /// needs to know whether any site exists and used to walk the world to ask (item 6).
    foundations: BTreeSet<EntityId>,
    /// Which families require which family to have been built once, resolved from `[build] requires`,
    /// with the requirement's role name kept for the message.
    requires_built: BTreeMap<u32, (u32, String)>,
    /// Age requirements (`[age] requires`), keyed by tech id: how many buildings of the current age,
    /// which families count, and which families satisfy it alone (D65).
    age_needs: BTreeMap<u32, (u32, BTreeSet<u32>, BTreeSet<u32>)>,
    /// Who is in this match, set when a player's first entity lands and never cleared. Without it the
    /// world could not tell "the enemy has lost everything" from "nobody was ever here", and losing
    /// everything is exactly the case a match has to notice (D46).
    roster: [bool; MAX_PLAYERS],
    /// Decided at the end of the tick that took a seat away. Derived from `entities` and `roster`, so
    /// it is not hashed: two runs that agree on those agree on this.
    outcome: Option<MatchOutcome>,
    /// Where each building's finished units go, by building. A side table: most buildings have none, and
    /// `Entity` is walked every tick.
    rallies: BTreeMap<EntityId, RallyPoint>,
    /// The far end of a patrol, by unit: the end it is *not* walking towards. A side table because a
    /// patrol is one order on a few units, while `Task` is walked for every entity on the map.
    patrols: BTreeMap<EntityId, (Fixed, Fixed)>,
    /// The walk a fight interrupted, by unit. Restored when the fight ends, which is the whole point
    /// of attack-move: an order that survives contact (D47).
    resuming: BTreeMap<EntityId, Task>,
    /// Stance and post, by unit. Only units whose stance is not the default have an entry, so an
    /// aggressive world - which is every world that does not ask - carries nothing for this (D48).
    stances: BTreeMap<EntityId, (Stance, (Fixed, Fixed))>,
    /// The last order the engine gave *itself* - an animal reacting to a blow - and could not start, with
    /// the tick it was refused. A reaction is not a command from a player, so there is nobody to return
    /// the error to; dropping it on the floor is what `retaliate`'s `let _ =` did, and it hid a real defect
    /// for two sessions (D71). Diagnostic only: it changes nothing about the world, so it is not hashed.
    refusal: Option<(EntityId, SimError, u64)>,
    /// Where an animal was first struck, and the tick its interest runs out (D71). Absent means grazing.
    /// Both live in a side table for the same reason `stances` does: `Entity` is walked every tick, and
    /// the animals that ever chase anything are a handful.
    leash: BTreeMap<EntityId, Leash>,
    /// How long an animal stays interested after the last blow, in ticks. `None` means content said
    /// nothing, and then only the distance rule applies (D71).
    leash_ticks: Option<u64>,
    /// How far from where it was first struck an animal will chase, in engine units. `None` means it
    /// will follow to the map's edge, which is what a pack that says nothing gets.
    leash_tiles: Option<i64>,
    /// Per player, one bit per map tile, row-major: ground this player has ever looked at. Part of
    /// state and hashed, because fog *is* a fact about where a player has been - two runs that
    /// disagree about it have diverged, and only this table knows (D50).
    explored: [Vec<u64>; MAX_PLAYERS],
    /// The map's size in tiles, from the content the world was seeded with. Zero until seeding.
    map_size: (u16, u16),
    /// Unit ids (family roots) that block movement once built, from `[build] blocking_roles` in content.
    /// A wall is the only one this pack names, and the engine learns the word from the pack (D54).
    blocking: BTreeSet<u32>,
    /// Blast radius per unit id, in engine units: resolved once from the table, because a strike must
    /// not read the content table inside a tick. Only the 27 units that have a radius are here (D51).
    blast: BTreeMap<u32, i64>,
    /// How far a hunting unit looks for an enemy, in engine units. Zero means it walks past
    /// everything, which is what a pack that says nothing about hunting gets.
    aggro_radius: i64,
    /// How often a hunting unit looks around, in ticks. Zero means every tick.
    aggro_scan_ticks: u64,
    /// Units waiting behind the one a building is making, by building. A side table rather than a
    /// second field on `Entity`: the entity array is walked every tick, and a `Vec` in it costs every
    /// entity for the sake of a few buildings (docs/performance.md, pass seven).
    queued: BTreeMap<EntityId, Vec<Training>>,
    /// Per player, the techs already researched. Part of state: it decides what a later upgrade
    /// does and what a later order resolves to, so it is hashed.
    researched: BTreeMap<u8, BTreeSet<u32>>,
    /// The pack's resource table, when one has been loaded. Costs carry the reference pack's ids and
    /// nodes carry columns; `slot_for_dat_id` and `name` are the only places the two are related, so
    /// this is the field every cost and every gather order reads (D17).
    resources: Option<Resources>,
    sim_rng: Pcg32,
    map_rng: Pcg32,
}

/// The eight compass directions a scattered shot can land in, as raw multipliers of the scatter distance.
/// Diagonals are therefore a little farther than a `sqrt(2)` would make them: the scatter is a distance
/// error, not a geometry, and integer octants keep a replay exact (D43).
const OCTANTS: [(i64, i64); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

/// A small, well-mixing integer hash: the accuracy roll's only source of "randomness", so that a replay
/// and a re-run agree to the miss (D43).
fn splitmix64(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The integer square root, for the one place that needs a distance rather than a squared distance: a
/// shot's time of flight. Newton's method on integers, so it is exact and identical on every machine.
fn integer_sqrt(value: i64) -> i64 {
    if value <= 0 {
        return 0;
    }
    let mut guess = value;
    let mut next = (guess + 1) / 2;
    while next < guess {
        guess = next;
        next = (guess + value / guess) / 2;
    }
    guess
}

/// Gather the modifiers one effect has for a unit and an attribute into `chosen`.
///
/// A modifier matches on the unit id it names, on the unit's class, or on neither (-1 means
/// "everything"). Free rather than a method so that both the researched-tech loop and the civ-bonus loop
/// can share it without borrowing `self` twice.
fn collect_modifiers<'a>(
    effects: &'a ruage_data::Effects,
    effect_id: u32,
    unit: &Unit,
    attribute: &str,
    chosen: &mut Vec<&'a ruage_data::Modifier>,
) {
    let Some(effect) = effects.get(effect_id) else {
        return;
    };
    for modifier in &effect.modifiers {
        if modifier.attribute != attribute {
            continue;
        }
        if modifier.unit >= 0 && u32::try_from(modifier.unit).ok() != Some(unit.id) {
            continue;
        }
        if modifier.unit_class >= 0 && modifier.unit_class != unit.class {
            continue;
        }
        chosen.push(modifier);
    }
}

impl World {
    pub fn new(settings: &Settings, seed: u64) -> Result<Self, SimError> {
        Ok(World {
            tick: 0,
            tick_rate_hz: settings.tick_rate_hz,
            scale: settings.fixed_point_scale,
            max_entities: settings.max_entities,
            command_apply: settings.command_apply,
            entities: Vec::new(),
            queue: Vec::new(),
            units: None,
            economy: None,
            techs: None,
            effects: None,
            civs: None,
            civ_by_player: [None; MAX_PLAYERS],
            bonus_effects: std::array::from_fn(|_| Vec::new()),
            team_by_player: [1, 2, 3, 4, 5, 6, 7, 8],
            teams_named: false,
            rallies: BTreeMap::new(),
            patrols: BTreeMap::new(),
            resuming: BTreeMap::new(),
            stances: BTreeMap::new(),
            refusal: None,
            leash: BTreeMap::new(),
            leash_ticks: None,
            leash_tiles: None,
            explored: std::array::from_fn(|_| Vec::new()),
            map_size: (0, 0),
            blocking: BTreeSet::new(),
            blast: BTreeMap::new(),
            aggro_radius: 0,
            aggro_scan_ticks: 0,
            queued: BTreeMap::new(),
            gated_units: BTreeSet::new(),
            upgrade_tech: BTreeMap::new(),
            availability_techs: Vec::new(),
            resolving_availability: false,
            start_stock_applied: false,
            conquest_classes: BTreeSet::new(),
            built: BTreeSet::new(),
            build_queue: BTreeMap::new(),
            animals: BTreeSet::new(),
            herdable: BTreeSet::new(),
            carcasses: BTreeSet::new(),
            animal_bodies: BTreeMap::new(),
            butchers: BTreeSet::new(),
            armed_animals: BTreeSet::new(),
            slaughter: BTreeMap::new(),
            herdable_ids: BTreeSet::new(),
            sight: BTreeMap::new(),
            rots: BTreeMap::new(),
            animal_rot: BTreeMap::new(),
            animal_span: BTreeMap::new(),
            shares: Vec::new(),
            shares_scan_tick: 0,
            pending_gather: BTreeMap::new(),
            spent_nodes: BTreeSet::new(),
            share_failed: BTreeMap::new(),
            share_moved: BTreeMap::new(),
            share_retry_ticks: 600,
            share_move_ticks: 40,
            foundations: BTreeSet::new(),
            requires_built: BTreeMap::new(),
            age_needs: BTreeMap::new(),
            roster: [false; MAX_PLAYERS],
            outcome: None,
            researched: BTreeMap::new(),
            stock: [[0; MAX_RESOURCES]; MAX_PLAYERS],
            walk: None,
            damage_rules: None,
            combat_projectile_speed: None,
            projectile_step: 0,
            accuracy_scatter: 0,
            garrison_eject_at: Fixed::ZERO,
            paths: BTreeMap::new(),
            resources: None,
            sim_rng: Pcg32::new(seed, SIM_STREAM),
            map_rng: Pcg32::new(seed, MAP_STREAM),
        })
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn tick_rate_hz(&self) -> u32 {
        self.tick_rate_hz
    }

    /// The fixed-point scale every position and velocity in this world uses.
    pub fn scale(&self) -> i64 {
        self.scale
    }

    /// Entities in id order. The order is the invariant, not an implementation detail.
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// Install content's `[policy]` share table. **Not called by `load_resources`**, deliberately: the shares change what
    /// an idle villager does, and a test that loads the real pack to watch a villager's own behaviour must not silently
    /// get a second hand on it (six `gather.rs` tests said so, loudly, the first time this ran from the loader). The
    /// runners call this; a world that is not playing a match does not have to.
    pub fn apply_content_policy(&mut self) {
        self.install_shares();
    }

    /// Resolve content's `[policy]` rows into stockpile columns and install them. A name the pack cannot resolve is
    /// skipped rather than refused - the pack is authoritative about which resources exist, and a share for a resource
    /// that is not in the game is a share for nothing.
    fn install_shares(&mut self) {
        let Some(economy) = self.economy.as_ref() else {
            return;
        };
        if economy.policy.share.is_empty() {
            return;
        }
        let rows: Vec<(usize, Vec<(usize, i32)>)> = economy
            .policy
            .share
            .iter()
            .map(|row| {
                let shares = row
                    .per
                    .iter()
                    .filter_map(|(name, percent)| {
                        self.slot_of(name)
                            .map(|slot| (slot, i32::try_from(*percent).unwrap_or(0)))
                    })
                    .collect();
                (row.at_villagers, shares)
            })
            .collect();
        self.set_villager_shares(&rows);
    }

    /// Set the villager share table: `(at_villagers, [(stockpile slot, percent)])`, in ascending order. Content's
    /// `[policy]` is resolved into this at seed; a test sets it directly.
    ///
    /// The rows are *shares*, not orders: the engine moves a villager only when it is idle (or working a resource that
    /// the table says is over-manned), so a player's own orders are never fought over.
    pub fn set_villager_shares(&mut self, rows: &[(usize, Vec<(usize, i32)>)]) {
        let mut rows = rows.to_vec();
        rows.sort_by_key(|(at, _)| *at);
        self.shares = rows;
        // A new table is a new policy: the memo of what the *old* one moved and what it could not reach would otherwise
        // block the new one for its retry window - which is exactly how the first version of this test failed, with a
        // villager sitting idle beside a table that wanted it working.
        self.share_moved.clear();
        self.share_failed.clear();
    }

    /// Which stockpile column a gather node belongs to: a resource node's own, an animal's `[hunt]` resource, a farm's
    /// `[farm]` resource. `None` for anything else.
    fn node_slot(&self, node: EntityId) -> Option<usize> {
        let entity = self.find(node)?;
        match entity.kind {
            EntityKind::Resource(slot) => Some(slot),
            EntityKind::Unit => {
                let economy = self.economy.as_ref()?;
                economy
                    .hunt
                    .resource
                    .as_deref()
                    .and_then(|name| self.slot_of(name))
            }
            EntityKind::Building => {
                let economy = self.economy.as_ref()?;
                economy
                    .farm
                    .resource
                    .as_deref()
                    .and_then(|name| self.slot_of(name))
            }
            EntityKind::Projectile => None,
        }
    }

    /// Enforce the share table, once a second. One villager per pass - the table is a preference, not a teleport - and
    /// only idle villagers or ones on an over-manned resource are moved, so a player's own orders survive.
    ///
    /// Cost: one walk of the entities per call, gated on a table existing; the walk counts villagers and finds the
    /// nearest node of the chosen resource in a second pass over the resource nodes only.
    fn step_shares(&mut self) {
        if self.shares.is_empty() {
            return;
        }
        let every = u64::from(self.tick_rate_hz).max(1);
        if !self.tick.is_multiple_of(every) {
            return;
        }
        self.shares_scan_tick = self.tick;
        let villager_unit = match self.start_villager_unit() {
            Some(unit) => unit,
            None => return,
        };
        // Who is working what, and who is free.
        // `(id, player, which column it is working, and whether it is *free*)`. The flag is not `slot.is_none()`:
        // a villager laying a foundation or walking to one has no column either, and taking its build queue away
        // because the table wanted another lumberjack broke `build_queue.rs` the first time this pass ran.
        let mut workers: Vec<(EntityId, u8, Option<usize>, bool)> = Vec::new();
        let mut per_player: [usize; MAX_PLAYERS] = [0; MAX_PLAYERS];
        for entity in &self.entities {
            if entity.kind != EntityKind::Unit || entity.player == 0 {
                continue;
            }
            let is_villager = entity
                .unit
                .is_some_and(|unit| self.is_family(unit, villager_unit));
            if !is_villager {
                continue;
            }
            per_player[usize::from(entity.player) - 1] += 1;
            // A villager standing at a **dead** node with nothing in hand is idle in everything but the name of its
            // task - 0 A.D. gives that state its own name (`FINDINGNEWTARGET`) and lets it find the next resource. It is
            // the state this economy used to freeze in: the whole run stopped at 8 units with food in the stockpile,
            // because the workers *looked* employed. Counting them as idle is what lets the share table move them.
            let (slot, free) = match entity.task {
                Task::Gather { node, .. } => {
                    // Dead means "nothing left to take". **Not** `hp <= 0`: every resource node on the map is spawned with
                    // `hp = 0` (a tree is not a thing with health), so testing hp here made every gatherer look idle - and
                    // the share pass then re-issued a gather order to it once a second for the whole match, which is why
                    // villagers never settled on a tree. Only a *unit* has death by hp.
                    let dead = self.find(node).is_none_or(|target| {
                        target.amount.raw() <= 0
                            || (target.kind == EntityKind::Unit && target.hp <= 0)
                    });
                    if dead && entity.carry.raw() <= 0 {
                        (None, true)
                    } else {
                        (self.node_slot(node), false)
                    }
                }
                Task::Return { node, .. } => (self.node_slot(node), false),
                Task::Idle => (None, true),
                // **A builder is reserved.** This is the AI's `up-assign-builders` (Feudal Age.per assigns three to a
                // market, two to a blacksmith; Castle Age.per six to a town-centre foundation): once a villager is on a
                // site it is not free and it is not on a resource column, so neither the idle arm nor the pull arm can
                // take it. It stays that way until the site finishes, is cancelled, or the build path gives up on it.
                // Everything else - a walk, a fight - is work in progress and not the table's business either.
                _ => (None, false),
            };
            workers.push((entity.id, entity.player, slot, free));
        }
        for player in 1..=MAX_PLAYERS as u8 {
            let count = per_player[usize::from(player) - 1];
            if count == 0 {
                continue;
            }
            let Some((_, targets)) = self
                .shares
                .iter()
                .rev()
                .find(|(at, _)| *at <= count)
                .cloned()
            else {
                continue;
            };
            // Actual against target, per slot, in whole villagers. Every column counts, not only the listed ones: a
            // villager left on a resource the table gives 0% is over-manned and is what gets moved.
            let slots = self.resources.as_ref().map_or(0, |pack| pack.count());
            let percent_of = |slot: usize| {
                targets
                    .iter()
                    .find(|(known, _)| *known == slot)
                    .map_or(0, |(_, percent)| *percent)
            };
            let mut deficit: Vec<(i32, usize)> = (0..slots)
                .map(|slot| {
                    let percent = percent_of(slot);
                    let wanted = (i32::try_from(count).unwrap_or(0) * percent) / 100;
                    let actual = i32::try_from(
                        workers
                            .iter()
                            .filter(|(_, who, slot_of, _)| *who == player && slot_of == &Some(slot))
                            .count(),
                    )
                    .unwrap_or(0);
                    (wanted - actual, slot)
                })
                .collect();
            deficit.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            let Some((need, slot)) = deficit.first().copied().filter(|(need, _)| *need > 0) else {
                continue;
            };
            let _ = need;
            // **Only an idle villager is moved**, and never twice within the retry window. Moving somebody who is
            // already working means a fresh path search per second (measured: 29 ms each, 11.7 s over 400 game-seconds
            // against 319 ms without the pass), and re-issuing the same order every second is not a policy anyway: a
            // villager who has been told where to go is no longer the engine's business. Shares converge through the
            // villagers who *arrive* - one per pass - and through training.
            // The first free villager **that is also off the retry window**, with no site queued. Every part of that
            // belongs inside the search: asking for "the first free one" and then rejecting it spends the pass on the
            // same villager - which is exactly how this pass failed, with three idle villagers and the same one chosen
            // every second while the other two stood there.
            let free = workers
                .iter()
                .find(|(id, who, _, free)| {
                    *who == player
                        && *free
                        && !self.build_queue.get(id).is_some_and(|q| !q.is_empty())
                        && !self
                            .share_moved
                            .get(id)
                            .is_some_and(|at| self.tick.saturating_sub(*at) < self.share_move_ticks)
                })
                .map(|(id, _, _, _)| *id);
            // Nobody free: the table can still **rebalance**. Take one villager off the column that is over its share,
            // bounded by the same retry window. Without this the shares only ever describe the *first* assignment, and a
            // 32-villager economy sat on 35 wood for 1,000 game-seconds while fourteen villagers fed a granary that
            // nothing was spending.
            let poached = free.or_else(|| {
                let mut over: Vec<(i32, usize)> = (0..slots)
                    .filter_map(|other| {
                        let percent = percent_of(other);
                        let wanted = (i32::try_from(count).unwrap_or(0) * percent) / 100;
                        let actual = i32::try_from(
                            workers
                                .iter()
                                .filter(|(_, who, slot_of, _)| {
                                    *who == player && slot_of == &Some(other)
                                })
                                .count(),
                        )
                        .unwrap_or(0);
                        (actual > wanted).then_some((actual - wanted, other))
                    })
                    .collect();
                over.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
                let (_, from) = over.first().copied()?;
                workers
                    .iter()
                    .find(|(id, who, slot_of, _)| {
                        *who == player
                            && slot_of == &Some(from)
                            && !self.build_queue.get(id).is_some_and(|q| !q.is_empty())
                            && !self.share_moved.get(id).is_some_and(|at| {
                                self.tick.saturating_sub(*at) < self.share_move_ticks
                            })
                    })
                    .map(|(id, _, _, _)| *id)
            });
            let Some(worker) = poached else {
                continue;
            };
            let here = match self.find(worker) {
                Some(entity) => entity.position,
                None => continue,
            };
            // The worker may be carrying: then the load goes to a drop site **first** and the new order waits in
            // `pending_gather`. A villager pulled off wood with ten wood in its arms and sent straight to a farm has
            // thrown those ten away, which is not what a player sees.
            let node = match self.nodes_of_slot(here, slot, 3).into_iter().next() {
                Some(node) => node,
                None => continue,
            };
            self.share_moved.insert(worker, self.tick);
            if self
                .find(worker)
                .is_some_and(|entity| entity.carry.raw() > 0)
            {
                let task = self.find(worker).map(|entity| entity.task);
                if let Some(Task::Gather {
                    node: old,
                    rate_per_tick,
                    carry_cap,
                    ..
                }) = task
                    && let Some(returning) = self.return_task(
                        self.find(worker).expect("the worker"),
                        old,
                        rate_per_tick,
                        carry_cap,
                    )
                {
                    if let Some(entity) = self.find_mut(worker) {
                        entity.task = returning;
                    }
                    self.pending_gather.insert(worker, node);
                    let _ = need;
                    continue;
                }
            }
            if let Err(error) = self.start_gather(worker, node) {
                self.share_failed.insert(node, self.tick);
                self.refusal = Some((worker, error, self.tick));
            } else {
                self.share_failed.remove(&node);
            }
            let _ = need;
        }
    }

    /// The nearest nodes of a stockpile column, by straight-line distance - a **short list**, not one answer.
    ///
    /// **No reachability test here, deliberately.** `can_reach` is a full path search; asking it of six nodes once a
    /// second per player cost 37x the whole rest of the run (400 game-seconds: 319 ms without this pass, 11.7 s with it,
    /// 2026-10-29). `start_gather` plans the walk and is the one authority on whether a node can be worked, so this
    /// returns a few candidates in distance order and the caller stops at the first that starts. A node that fails is
    /// remembered (`share_failed`) instead of being retried every second - 0 A.D.'s `lastFailedGather`.
    fn nodes_of_slot(&self, from: (Fixed, Fixed), slot: usize, limit: usize) -> Vec<EntityId> {
        let mut nodes: Vec<(i64, u32, EntityId)> = Vec::new();
        for entity in &self.entities {
            if self.node_slot(entity.id) != Some(slot) {
                continue;
            }
            // A farm must be finished to be worked; a node with nothing left is no use; a body is not huntable.
            if entity.construction.is_some() || entity.amount.raw() <= 0 {
                continue;
            }
            if entity.kind == EntityKind::Unit && entity.hp <= 0 {
                continue;
            }
            if self
                .share_failed
                .get(&entity.id)
                .is_some_and(|at| self.tick.saturating_sub(*at) < self.share_retry_ticks)
            {
                continue;
            }
            // Somewhere to stand, or it is not a candidate at all: an interior tree costs a refusal and a `share_failed`
            // entry every 30 s for the rest of the match.
            if !self.node_workable(entity.id) {
                continue;
            }
            let dx = entity.position.0.raw() - from.0.raw();
            let dy = entity.position.1.raw() - from.1.raw();
            nodes.push((dx * dx + dy * dy, entity.id.0, entity.id));
        }
        nodes.sort_by_key(|(distance, id, _)| (*distance, *id));
        nodes.into_iter().take(limit).map(|(_, _, id)| id).collect()
    }

    /// The last reaction the engine could not start, as `(the entity that tried, why, the tick)`. A
    /// reaction has no caller to return an error to, so it is kept rather than dropped - the whole point
    /// of the field is that a silent refusal is undiagnosable.
    pub fn last_refusal(&self) -> Option<(EntityId, &SimError, u64)> {
        self.refusal
            .as_ref()
            .map(|(id, why, tick)| (*id, why, *tick))
    }

    pub fn position(&self, id: EntityId) -> Option<(Fixed, Fixed)> {
        self.find(id).map(|e| e.position)
    }

    pub fn entity(&self, id: EntityId) -> Option<&Entity> {
        self.find(id)
    }

    /// What a player has banked in a stockpile column. Player 0 (nobody) always reads zero.
    ///
    /// The column is the pack's slot, so a caller that wants "wood" asks the pack for the column
    /// (`slot_of`) rather than naming a resource here.
    pub fn stock(&self, player: u8, slot: usize) -> u32 {
        self.stock
            .get(usize::from(player).wrapping_sub(1))
            .and_then(|row| row.get(slot))
            .copied()
            .unwrap_or(0)
    }

    /// The stockpile column a cost resolves to, or a refusal.
    ///
    /// The pack owns the mapping (D17), in both of the namespaces a cost may use: a reference id
    /// (`[1, 25]`, what the generator emits) or a name (`{ resource = "timber" }`, what a pack of our
    /// own writes). Either becomes a column here and nothing above the pack layer learns what the
    /// cost meant. A refusal rather than a guess, because guessing is invisible: slot 0 is wood in
    /// our pack, so a wrong guess spends the wrong resource and every number still adds up.
    fn cost_slot(&self, cost: &Cost) -> Result<usize, SimError> {
        match &cost.resource {
            CostResource::ReferenceId(dat_id) => {
                self.resource_slot(u16::from(*dat_id))
                    .ok_or(SimError::UnknownResource {
                        dat_id: u16::from(*dat_id),
                    })
            }
            CostResource::Name(name) => self
                .slot_of(name)
                .ok_or_else(|| SimError::UnknownResourceName { name: name.clone() }),
        }
    }

    /// The pack's name for a stockpile column, for error messages only - a name never enters state.
    fn slot_name(&self, slot: usize) -> String {
        self.resources
            .as_ref()
            .and_then(|resources| resources.name(slot))
            .map_or_else(|| format!("slot {slot}"), |name| name.to_owned())
    }

    pub fn carry(&self, id: EntityId) -> Fixed {
        self.find(id).map(|e| e.carry).unwrap_or(Fixed::ZERO)
    }

    /// Submit a command, refusing one that cannot be applied *at all* before it is queued. A bad
    /// order should be an error in the caller's face, not a villager who idles for ten minutes.
    pub fn submit_checked(&mut self, command: Command) -> Result<(), SimError> {
        self.validate(&command)?;
        self.submit(command);
        Ok(())
    }

    fn validate(&self, command: &Command) -> Result<(), SimError> {
        // A finished match takes no more orders. The check is here as well as in `apply` so the
        // caller sees the refusal instead of a tick that silently does nothing.
        if self.outcome.is_some() {
            return Err(SimError::MatchOver);
        }
        match command {
            Command::Gather { villager, node } => {
                let worker = self
                    .find(*villager)
                    .ok_or(SimError::MissingEntity(*villager))?;
                if worker.kind != EntityKind::Unit {
                    return Err(SimError::NotAUnit { id: *villager });
                }
                let target = self.find(*node).ok_or(SimError::MissingEntity(*node))?;
                // What may be gathered from is content's answer, not the validator's: a node, or a
                // building the pack calls a farm. One helper, so the order cannot accept something
                // the validator refuses.
                self.gather_source(*node)?;
                // A walled-off node is refused here rather than discovered ten seconds later by a
                // villager walking into a wall and giving up.
                if let Some(position) = self.find(*villager).map(|e| e.position) {
                    self.plan_path(position, target.position)?;
                }
            }
            Command::Build { villager, unit, at } => {
                self.check_build(*villager, *unit, *at)?;
            }
            Command::Train { building, unit } => {
                self.check_train(*building, *unit)?;
            }
            Command::Research { building, tech } => {
                self.check_research(*building, *tech)?;
            }
            Command::Attack { attacker, target } => {
                self.check_attack(*attacker, *target)?;
            }
            // Stopping is always legal: there is nothing to be refused.
            Command::Rally { building, point } => {
                let entity = self
                    .find(*building)
                    .ok_or(SimError::MissingEntity(*building))?;
                if entity.kind != EntityKind::Building {
                    return Err(SimError::TrainNeedsABuilding { id: *building });
                }
                if let Some((x, y)) = point
                    && !self.on_map(*x, *y)
                {
                    return Err(SimError::OffMap { tile: (*x, *y) });
                }
            }
            Command::Garrison { unit, building } => {
                self.check_garrison(*unit, *building)?;
            }
            Command::Ungarrison { building } => {
                let entity = self
                    .find(*building)
                    .ok_or(SimError::MissingEntity(*building))?;
                if entity.kind != EntityKind::Building {
                    return Err(SimError::TrainNeedsABuilding { id: *building });
                }
            }
            Command::Stop { .. } => {}
            Command::Move { unit, to } => {
                self.check_order(*unit, *to)?;
            }
            Command::Stance { unit, .. } => {
                let entity = self.find(*unit).ok_or(SimError::MissingEntity(*unit))?;
                if entity.kind != EntityKind::Unit {
                    return Err(SimError::NotAUnit { id: *unit });
                }
            }
            Command::AttackMove { unit, to } | Command::Patrol { unit, to } => {
                self.check_order(*unit, *to)?;
            }
            Command::Delete { player, entity } => {
                let target = self.find(*entity).ok_or(SimError::MissingEntity(*entity))?;
                if target.player == 0 || target.player != *player {
                    return Err(SimError::NotYours {
                        id: *entity,
                        player: *player,
                    });
                }
                if !matches!(target.kind, EntityKind::Unit | EntityKind::Building) {
                    return Err(SimError::NotDeletable { id: *entity });
                }
            }
            // The bound belongs in the dry run too, or `submit_checked` accepts a ninth side and
            // the failure only appears a tick later inside the queue.
            Command::Spawn { player, .. } => {
                // 0 is neutral (no stockpile); the ceiling is what has to hold (D20).
                if usize::from(*player) > MAX_PLAYERS {
                    return Err(SimError::PlayerOutOfRange { player: *player });
                }
            }
        }
        Ok(())
    }

    fn find(&self, id: EntityId) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id)
    }

    fn find_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    /// Queue a command. With `command_apply = "immediate"` it takes effect at once; otherwise
    /// it is held until the next `step`, which is what makes a command log replayable.
    pub fn submit(&mut self, command: Command) {
        match self.command_apply {
            CommandApply::Immediate => {
                // The caller cannot observe the error here; `step` will not see these again.
                let _ = self.apply(command);
            }
            CommandApply::NextTick => self.queue.push(command),
        }
    }

    /// Spawn a nameless mover at the origin. Movement tests use this; a real entity comes from
    /// `seed_from`, because player, unit id and kind are not things a test should invent.
    /// Panics on a duplicate id or a full world: those are programming errors.
    /// A body that walks from the moment it exists, with no order - which is what the movement tests need
    /// and what the engine itself no longer produces: a placed thing starts at rest (D70).
    pub fn spawn_now(&mut self, id: EntityId, speed: Fixed, hp: i32) {
        self.spawn_now_checked(id, speed, hp)
            .unwrap_or_else(|err| panic!("{err}"));
        if let Some(entity) = self.find_mut(id) {
            entity.velocity = (speed, Fixed::ZERO);
        }
    }

    pub fn spawn_now_checked(
        &mut self,
        id: EntityId,
        speed: Fixed,
        hp: i32,
    ) -> Result<(), SimError> {
        self.apply(Command::Spawn {
            id,
            player: 0,
            kind: EntityKind::Unit,
            unit: None,
            position: (Fixed::ZERO, Fixed::ZERO),
            speed,
            hp,
            amount: 0,
        })
    }

    /// An animal's meat is the pack's `food` for that unit, in whole units - 100 for a sheep, 340 for a
    /// boar, 0 for anything that is not an animal (D69, D70).
    fn meat_of(&self, unit: Option<u32>) -> u32 {
        let Some(units) = self.units.as_ref() else {
            return 0;
        };
        unit.and_then(|unit| units.unit(unit))
            .map_or(0, |stats| stats.food)
    }

    /// Build the opening position: one town centre per player, `villagers_per_player` villagers
    /// around it, stats resolved from the unit table.
    ///
    /// Everything here is decided by the map and the config — no literal ids, no default
    /// speeds (D6). Tiles are placed at their origin, so a tile at (x, y) is `x * scale`.
    pub fn seed_from(
        &mut self,
        map: &Map,
        economy: &Economy,
        units: &Units,
    ) -> Result<(), SimError> {
        self.units = Some(units.clone());
        self.economy = Some(economy.clone());
        // Which building classes keep a player alive (D59b). The manual's list of things you need not
        // destroy is a list of *classes* - towers 52, walls 27, gates 39, farms 49, relics 1 - so the
        // whole rule is "class 3 counts", with no per-building list to keep in step and nothing to
        // forget when the pack gains a mill.
        // Which unit families are animals, so a gather order can be pointed at one (D70). By family, so a
        // carcass or an upgraded variant answers the same way.
        self.animals.clear();
        for role in &economy.hunt.animal_roles {
            if let Some(unit) = economy.role.get(role).copied() {
                // The id as well as the family: the tick asks "is this a living animal" per worker per tick and
                // must not walk the unit table to answer it (item 3).
                self.animals.insert(unit);
                self.animals.insert(units.root(unit));
            }
        }
        // Animals that can fight back. A sheep cannot, so being struck is not a refusal for it - it is simply an
        // animal that has no quarrel (item 3).
        self.armed_animals.clear();
        for role in &economy.hunt.animal_roles {
            if let Some(unit) = economy.role.get(role).copied()
                && units
                    .unit(unit)
                    .is_some_and(|stats| !stats.attacks.is_empty())
            {
                self.armed_animals.insert(unit);
                self.armed_animals.insert(units.root(unit));
            }
        }
        self.herdable.clear();
        for role in &economy.hunt.herdable_roles {
            if let Some(unit) = economy.role.get(role).copied() {
                // The id as well as the family: the tick asks "is this livestock" on the damage path and on the
                // conversion pass, and neither may walk the unit table (items 3 and 5).
                self.herdable.insert(unit);
                self.herdable.insert(units.root(unit));
            }
        }
        self.carcasses.clear();
        for role in &economy.hunt.carcass_roles {
            if let Some(unit) = economy.role.get(role).copied() {
                // Both the id and its family: the sweep runs per entity per tick and must not consult the
                // unit table to answer "is this a body" (D70).
                self.carcasses.insert(unit);
                self.carcasses.insert(units.root(unit));
            }
        }
        // What death leaves: each animal's `dead_unit`, and which roles make meat with their last blow. Both are
        // resolved here so the tick reads two maps and no table (item 2).
        self.animal_bodies.clear();
        for role in economy
            .hunt
            .animal_roles
            .iter()
            .chain(&economy.hunt.herdable_roles)
        {
            if let Some(unit) = economy.role.get(role).copied() {
                let body = units.unit(unit).and_then(|stats| stats.dead_unit);
                // A body the pack cannot name is a lookup that finds nothing, not an error: 640 records name a
                // body no civ has (D69). Such an animal is simply swept away as it was before.
                if let Some(body) = body.filter(|body| units.unit(*body).is_some()) {
                    self.animal_bodies.insert(unit, body);
                    self.animal_bodies.insert(units.root(unit), body);
                }
                // How fast its body rots: `decay_rate` is food per second (boar 0.4, sheep and deer 0.25 - the pack
                // agrees with two published accounts), turned into raw units per tick once, here. A rate the pack
                // leaves unresolved yields no rot rather than an invented one.
                if let Some(per_tick) = units
                    .unit(unit)
                    .and_then(|stats| stats.decay_rate.resolve(self.scale, "decay_rate").ok())
                    .map(|per_second| per_second.raw() / i64::from(self.tick_rate_hz))
                    .filter(|per_tick| *per_tick > 0)
                {
                    self.animal_rot.insert(unit, per_tick);
                    self.animal_rot.insert(units.root(unit), per_tick);
                    // How long its body lies there: its own meat divided by its own rot rate. A boar is 340 food at
                    // 0.4 a second, so 850 s; a sheep 100 at 0.25, so 400 s. Derived, so a pack whose animals change
                    // changes the lifetime with them.
                    let meat =
                        i64::from(units.unit(unit).map_or(0, |stats| stats.food)) * self.scale;
                    let span = u64::try_from((meat / per_tick).max(0)).unwrap_or(0);
                    if span > 0 {
                        self.animal_span.insert(unit, span);
                        self.animal_span.insert(units.root(unit), span);
                    }
                }
            }
        }
        self.butchers.clear();
        for role in &economy.hunt.butcher_roles {
            if let Some(unit) = economy.role.get(role).copied() {
                self.butchers.insert(unit);
                self.butchers.insert(units.root(unit));
            }
        }
        // How far a unit sees, squared and resolved once (item 5). `los` is a pack column; squaring at seed keeps
        // the per-tick comparison to a distance test with no root and no table read.
        self.sight.clear();
        for unit in units.units.values() {
            if let Ok(los) = unit.los.resolve(self.scale, "los") {
                let reach = los.raw();
                self.sight.insert(unit.id, reach * reach);
            }
        }
        // Which herdables are already standing here (a scenario may place them), and the same question for a body
        // that content names later: both sets are rebuilt here rather than guessed.
        self.herdable_ids.clear();
        for entity in &self.entities {
            if entity.kind == EntityKind::Unit
                && entity.player == 0
                && entity
                    .unit
                    .is_some_and(|unit| self.herdable.contains(&unit))
            {
                self.herdable_ids.insert(entity.id);
            }
        }
        // The leash (D71): content's two numbers, resolved once into ticks and engine units, so the tick
        // loop never divides and never touches a float. `None` means content said nothing, which is not
        // zero - a zero-second interest would make every animal give up on the tick it was struck.
        self.leash_ticks = economy
            .hunt
            .leash_seconds
            .map(|seconds| u64::from(seconds) * u64::from(self.tick_rate_hz));
        self.leash_tiles = economy
            .hunt
            .leash_tiles
            .map(|tiles| i64::from(tiles) * self.scale);
        self.conquest_classes.clear();
        for class in &economy.conquest.surviving_classes {
            self.conquest_classes.insert(*class);
        }
        // Building prerequisites (D63), resolved once to families: role -> the family that must have been
        // built first. A pair whose roles the pack does not have is skipped, like every other content name.
        self.requires_built.clear();
        if let Some(units) = self.units.as_ref() {
            for (role, need_role) in &economy.build.requires {
                if let (Some(unit), Some(needed)) =
                    (economy.role.get(role), economy.role.get(need_role))
                {
                    self.requires_built
                        .insert(units.root(*unit), (units.root(*needed), need_role.clone()));
                }
            }
        }
        // Age requirements (D65), resolved once to families and keyed by tech id, because that is what a
        // research order carries.
        self.age_needs.clear();
        if let Some(units) = self.units.as_ref() {
            for (role, requirement) in &economy.age.requires {
                let Some(tech) = economy.tech_role.get(role) else {
                    continue;
                };
                let families: BTreeSet<u32> = requirement
                    .of
                    .iter()
                    .filter_map(|name| economy.role.get(name))
                    .map(|unit| units.root(*unit))
                    .collect();
                let singles: BTreeSet<u32> = requirement
                    .single
                    .iter()
                    .filter_map(|name| economy.role.get(name))
                    .map(|unit| units.root(*unit))
                    .collect();
                if requirement.count > 0 {
                    self.age_needs
                        .insert(*tech, (requirement.count, families, singles));
                }
            }
        }
        // Blast radii, resolved once. The *radius* is the gate: `blast_level` is carried and not
        // consumed, because what its value means is not published and every unit that has one has a
        // radius with it (D51).
        self.blast.clear();
        for (id, unit) in &units.units {
            let Ok(radius) = unit.blast_radius.resolve(self.scale, "blast_radius") else {
                continue;
            };
            if radius.raw() > 0 {
                self.blast.insert(*id, radius.raw());
            }
        }
        self.walk = Some(Walk::from_map(map));
        self.blocking.clear();
        for role in &economy.build.blocking_roles {
            let unit = economy
                .role
                .get(role)
                .copied()
                .ok_or_else(|| SimError::NoRole { role: role.clone() })?;
            self.blocking.insert(units.root(unit));
        }
        self.map_size = (map.width, map.height);
        let words = (usize::from(map.width) * usize::from(map.height)).div_ceil(64);
        self.explored = std::array::from_fn(|_| vec![0u64; words]);
        // The start names roles; the economy's role table is what turns them into units, so this
        // function never learns that a town centre is 109 (D17).
        let tc_stats = self.role_unit_in(economy, &economy.start.town_centre)?;
        let villager_stats = self.role_unit_in(economy, &economy.start.villager)?;
        let town_centre = units
            .unit(tc_stats)
            .ok_or(SimError::MissingUnit { id: tc_stats })?;
        let villager = units
            .unit(villager_stats)
            .ok_or(SimError::MissingUnit { id: villager_stats })?;

        let resolve = |unit: &ruage_data::Unit, id: u32| -> Result<Fixed, SimError> {
            unit.speed
                .resolve(self.scale, "speed")
                .map_err(|err| SimError::Unresolved {
                    id,
                    detail: err.to_string(),
                })
        };
        let tc_speed = resolve(town_centre, tc_stats)?;
        let villager_speed = resolve(villager, villager_stats)?;

        let mut centres = map.town_centres.clone();
        centres.sort_by_key(|centre| centre.player);

        // Node tiles are taken before anyone is placed, so a villager never starts inside a
        // mine or a berry bush. The nodes themselves are inserted after the players for ids.
        let mut used: std::collections::BTreeSet<(u16, u16)> =
            map.resources.iter().map(|node| (node.x, node.y)).collect();
        let mut next_id = 1u32;
        for centre in centres {
            let here = (centre.x, centre.y);
            used.insert(here);
            self.insert(Entity {
                id: EntityId(next_id),
                player: centre.player,
                kind: EntityKind::Building,
                unit: Some(tc_stats),
                position: self.tile(here),
                speed: tc_speed,
                velocity: (Fixed::ZERO, Fixed::ZERO),
                hp: town_centre.hp,
                amount: Fixed::ZERO,
                task: Task::Idle,
                training: None,
                construction: None,
                researching: None,
                carry: Fixed::ZERO,
                variant: None,
            })?;
            next_id += 1;

            for _ in 0..economy.start.villagers_per_player {
                let spot = free_neighbour(map, here, &used).ok_or(SimError::NoSpace {
                    player: centre.player,
                })?;
                used.insert(spot);
                self.insert(Entity {
                    id: EntityId(next_id),
                    player: centre.player,
                    kind: EntityKind::Unit,
                    unit: Some(villager_stats),
                    position: self.tile(spot),
                    speed: villager_speed,
                    velocity: (Fixed::ZERO, Fixed::ZERO),
                    hp: villager.hp,
                    amount: Fixed::ZERO,
                    task: Task::Idle,
                    training: None,
                    construction: None,
                    researching: None,
                    carry: Fixed::ZERO,
                    variant: None,
                })?;
                next_id += 1;
            }
        }

        // Resource nodes are entities too: a villager targets one by id, and the amount has to
        // live in state because gathering spends it. Map order, not sorted, so two runs agree
        // for the same file.
        for node in &map.resources {
            self.insert(Entity {
                id: EntityId(next_id),
                player: 0,
                kind: EntityKind::Resource(node.slot),
                unit: None,
                position: self.tile((node.x, node.y)),
                speed: Fixed::ZERO,
                velocity: (Fixed::ZERO, Fixed::ZERO),
                // A tree has to be **felled** before it yields (the owner's rule, 2026-10-30): the dat gives it 20 hit
                // points (25, 30 for the variants) and a bush or a mine none.
                hp: node.hp,
                amount: Fixed::from_raw(i64::from(node.amount) * self.scale),
                task: Task::Idle,
                training: None,
                construction: None,
                researching: None,
                carry: Fixed::ZERO,
                variant: None,
            })?;
            next_id += 1;
        }
        // Teams: content may put players together, otherwise everyone is alone.
        let teams = economy.start.teams.clone();
        if !teams.is_empty() {
            self.teams_named = true;
            for index in 0..MAX_PLAYERS {
                let team = teams[index.min(teams.len() - 1)];
                self.team_by_player[index] = u8::try_from(team).unwrap_or(u8::MAX);
            }
        }

        // If the tables this needs are already loaded, the opening position belongs to these
        // entities; if they arrive later, the setters apply them then.
        self.apply_start_stock()?;
        self.grant_starting_techs()?;
        // The opening position is already watched: a player sees their own town centre from tick zero,
        // rather than from the first tick they happen to move something.
        self.stamp_all();
        Ok(())
    }

    /// A tile's origin as a world position. Tiles are the unit of the map; the scale is the unit
    /// of state, and this is the only place the two meet.
    /// Can a walker get from one place to another? Asked by the scenario runner when it picks which
    /// node to send a villager to, so the script chooses a node it can reach instead of ordering a
    /// trip the engine will refuse.
    pub fn can_reach(&self, from: (Fixed, Fixed), to: (Fixed, Fixed)) -> bool {
        self.plan_path(from, to).is_ok()
    }

    fn tile(&self, (x, y): (u16, u16)) -> (Fixed, Fixed) {
        (
            Fixed::from_raw(i64::from(x) * self.scale),
            Fixed::from_raw(i64::from(y) * self.scale),
        )
    }

    fn insert(&mut self, entity: Entity) -> Result<(), SimError> {
        if self.find(entity.id).is_some() {
            return Err(SimError::DuplicateEntity(entity.id));
        }
        if self.entities.len() >= self.max_entities as usize {
            return Err(SimError::TooManyEntities {
                max: self.max_entities,
            });
        }
        // A player is in the match from the moment their first entity exists, and stays in it even
        // after losing everything: "the enemy is gone" and "nobody was ever here" are different
        // facts, and only one of them ends a match (D46).
        if entity.player > 0 && entity.player as usize <= MAX_PLAYERS {
            self.roster[usize::from(entity.player) - 1] = true;
        }
        // A building that arrives finished - seeded by the opening, or placed by a scenario - has been
        // "built once" as surely as one a villager raised, so prerequisites treat them the same (D63).
        if entity.kind == EntityKind::Building
            && entity.construction.is_none()
            && let Some(unit) = entity.unit
        {
            let family = self.units.as_ref().map_or(unit, |units| units.root(unit));
            self.built.insert((entity.player, family));
        }
        let at = self
            .entities
            .iter()
            .position(|existing| existing.id > entity.id)
            .unwrap_or(self.entities.len());
        let id = entity.id;
        // A wall closes its tile from the moment it stands there - the tile is claimed by the insertion,
        // not by the completion, because a half-built wall is not walk-through either. The check reads the
        // entity being inserted rather than looking it up, which is the bug this line had first: an entity
        // cannot be found before it is in the list.
        if entity.kind == EntityKind::Building
            && entity.unit.is_some_and(|unit| {
                let root = self.units.as_ref().map_or(unit, |units| units.root(unit));
                self.blocking.contains(&root)
            })
        {
            let tile = self.tile_of(entity.position);
            if let Some(walk) = self.walk.as_mut() {
                walk.set_blocked(tile.0, tile.1, true);
            }
        }
        self.entities.insert(at, entity);
        // A thing that has just arrived is watched from the tick it arrives: fog follows insertion as
        // well as movement, and this is the one place both paths can be fed from.
        self.stamp_entity(id);
        Ok(())
    }

    /// The `Return` task that takes `entity` home from `node`, or `None` when the player has no
    /// building the variant accepts (in which case the villager stays where it is rather than
    /// walking to a drop site that would refuse it).
    fn return_task(
        &self,
        entity: &Entity,
        node: EntityId,
        rate_per_tick: Fixed,
        carry_cap: u32,
    ) -> Option<Task> {
        // The column comes from the same place the order got it. Carrying it in the task instead
        // would add 8 bytes to every entity for a value the switch needs once a trip.
        let (resource, _) = self.gather_source(node).ok()?;
        let slot = self.slot_of(&resource)?;
        let from = self.find(node)?.position;
        let drop = self.nearest_drop_site(entity.player, from, entity.variant)?;
        Some(Task::Return {
            node,
            drop,
            slot,
            rate_per_tick,
            carry_cap,
        })
    }

    /// The nearest building of `player`'s that the worker's own variant accepts as a drop site.
    /// Selection is by squared distance with ties broken by id, so two villagers sent home in the
    /// same tick agree on which building is closest.
    fn nearest_drop_site(
        &self,
        player: u8,
        from: (Fixed, Fixed),
        variant: Option<u32>,
    ) -> Option<EntityId> {
        let units = self.units.as_ref()?;
        let drop_sites = &units.unit(variant?)?.drop_sites;
        self.entities
            .iter()
            .filter(|entity| entity.kind == EntityKind::Building && entity.player == player)
            // By family, not by id: an upgraded town centre is unit 71, and the drop list names 109.
            .filter(|entity| {
                entity.unit.is_some_and(|unit| {
                    let root = units.root(unit);
                    drop_sites.contains(&unit) || drop_sites.contains(&root)
                })
            })
            .min_by_key(|entity| {
                let dx = entity.position.0.raw() - from.0.raw();
                let dy = entity.position.1.raw() - from.1.raw();
                (dx * dx + dy * dy, entity.id.0)
            })
            .map(|entity| entity.id)
    }

    /// The tile a world position sits on.
    fn tile_of(&self, position: (Fixed, Fixed)) -> (u16, u16) {
        (
            u16::try_from(position.0.raw() / self.scale).unwrap_or(0),
            u16::try_from(position.1.raw() / self.scale).unwrap_or(0),
        )
    }

    /// A route between two positions, or `NoPath` when the target is walled off. Computed from the
    /// terrain copy taken at seed time; the map is static for a match.
    fn plan_path(
        &self,
        from: (Fixed, Fixed),
        to: (Fixed, Fixed),
    ) -> Result<Vec<(u16, u16)>, SimError> {
        let walk = self.walk.as_ref().ok_or(SimError::NoTables)?;
        let (start, goal) = (self.tile_of(from), self.tile_of(to));
        find_path(walk, start, goal).ok_or(SimError::NoPath {
            from: start,
            to: goal,
        })
    }

    /// The next waypoint of a unit's route, as a world position.
    fn next_waypoint(&self, id: EntityId) -> Option<(Fixed, Fixed)> {
        let (x, y) = *self.paths.get(&id)?.first()?;
        Some(self.tile((x, y)))
    }

    /// How much a worker takes this tick: the rate, the room left under its cap, and what the
    /// node still holds - the smallest of the three.
    /// How much this worker takes **this tick**, in whole units at the engine scale, and the fraction it keeps.
    ///
    /// Whole units only. The rate is fractional (`0.39/s`), so it accumulates in `progress` and a unit is granted when a
    /// whole one is ready: `floor((progress + rate) / 1)`. Two villagers on the same node is a race over whole units, and
    /// the one whose unit is ready first takes it - the other is granted nothing and moves on, which is 0 A.D.'s rule
    /// (`TakeResources` returns what actually changed; an exhausted supply stops the gatherer and re-targets it).
    fn take_amount(&self, worker: &Entity) -> (i64, Fixed) {
        let Task::Gather {
            node,
            rate_per_tick,
            carry_cap,
            progress,
        } = worker.task
        else {
            return (0, Fixed::ZERO);
        };
        let room = i64::from(carry_cap) * self.scale - worker.carry.raw();
        // A full load takes nothing, and does not bank progress it cannot use.
        if room <= 0 {
            return (0, progress);
        }
        let Some(node_entity) = self.find(node) else {
            return (0, progress);
        };
        // **Meat implies death** (the owner's rule, D70): a live animal's `amount` is what its *body* will hold, not
        // something it is standing on, so the work while it walks is following and the take is nothing. Without this
        // the villager ate the sheep on the way home and the slaughter was theatre (item 3).
        if node_entity.hp > 0
            && node_entity
                .unit
                .is_some_and(|unit| self.animals.contains(&unit))
        {
            return (0, progress);
        }
        let accumulated = progress.raw() + rate_per_tick.raw();
        let whole = accumulated / self.scale;
        let kept = Fixed::from_raw(accumulated - whole * self.scale);
        if whole <= 0 {
            return (0, kept);
        }
        let left_in_node = node_entity.amount.raw();
        let take = (whole * self.scale).min(room).min(left_in_node).max(0);
        (take, kept)
    }

    /// Read the combat constants. Kept separate from `seed_from` so a world that never fights does
    /// not need them, and so an attack without them fails loudly instead of guessing a minimum.
    pub fn load_combat(&mut self, combat: &ruage_data::Combat) {
        let (minimum, missing_armour) = combat.damage_rules();
        self.damage_rules = Some(DamageRules {
            minimum,
            missing_armour,
        });
        self.combat_projectile_speed = combat
            .projectile_speed
            .resolve(self.scale, "combat.projectile_speed")
            .ok()
            .filter(|speed| speed.raw() > 0);
        self.projectile_step = self.projectile_speed();
        self.garrison_eject_at = combat
            .garrison_eject_fraction
            .resolve(self.scale, "combat.garrison_eject_fraction")
            .unwrap_or(Fixed::ZERO);
        self.accuracy_scatter = combat
            .accuracy_scatter
            .resolve(self.scale, "combat.accuracy_scatter")
            .ok()
            .filter(|scatter| scatter.raw() > 0)
            .map_or(0, |scatter| scatter.raw());
        self.aggro_radius = combat
            .aggro_radius
            .resolve(self.scale, "combat.aggro_radius")
            .ok()
            .filter(|radius| radius.raw() > 0)
            .map_or(0, |radius| radius.raw());
        self.aggro_scan_ticks = u64::from(combat.aggro_scan_ticks);
    }

    /// The checks an attack order has to survive.
    fn check_attack(
        &self,
        attacker: EntityId,
        target: EntityId,
    ) -> Result<(i32, u32, Fixed, Fixed), SimError> {
        let source = self
            .find(attacker)
            .ok_or(SimError::MissingEntity(attacker))?;
        let victim = self.find(target).ok_or(SimError::MissingEntity(target))?;
        let rules = self.damage_rules.ok_or(SimError::NoTables)?;
        if matches!(source.kind, EntityKind::Resource(_)) {
            return Err(SimError::Unarmed { id: attacker });
        }
        if matches!(source.task, Task::Garrisoned { .. }) {
            return Err(SimError::Garrisoned { id: attacker });
        }
        if matches!(
            victim.kind,
            EntityKind::Resource(_) | EntityKind::Projectile
        ) || matches!(victim.task, Task::Garrisoned { .. })
        {
            // A tree is not a target and neither is an arrow: one cannot bleed and the other is a
            // consequence, not a combatant. Nor is a unit inside a building - the building is the
            // target, and the unit goes with it.
            return Err(SimError::NotATarget { id: target });
        }
        if self.effective_stance(attacker) == Stance::NoAttack {
            return Err(SimError::StanceForbidsAttack { id: attacker });
        }
        // Your own livestock is not friendly fire: once a sheep is yours, your own shepherd has to be able to kill
        // it, or the slaughter cannot happen at all (D70 item 5 - the animal joins the player whose villager is
        // walking it, which is the same player). A teammate's sheep is a different matter: that is still a fight.
        let livestock = victim
            .unit
            .is_some_and(|unit| self.herdable.contains(&unit));
        let same_player = source.player == victim.player;
        if same_player && !livestock {
            return Err(SimError::FriendlyFire { id: target });
        }
        // Allies are not targets either: two players on one team do not fight each other. The check is skipped for
        // the livestock case above, where "the same side" is the *same player* and the sheep is its shepherd's to
        // kill; a **teammate's** sheep still needs an ally to be hostile, which it is not.
        if !same_player && self.team_of(source.player) == self.team_of(victim.player) {
            return Err(SimError::AlliedTarget { id: target });
        }
        let units = self.units.as_ref().ok_or(SimError::NoTables)?;
        // The **variant** is what the worker is while it works: a villager on a hunt task is VMHUN, and VMHUN carries a
        // bow (`range = 3.0`) where the base villager carries a knife (`range = 0.0`). Reading `unit` alone made every
        // hunter fight at arm's length, which is the owner's first correction (2026-10-29).
        let (attacker_stats, victim_stats) = match (source.variant.or(source.unit), victim.unit) {
            (Some(a), Some(b)) => (
                units.unit(a).ok_or(SimError::MissingUnit { id: a })?,
                units.unit(b).ok_or(SimError::MissingUnit { id: b })?,
            ),
            _ => return Err(SimError::Unarmed { id: attacker }),
        };
        if attacker_stats.attacks.is_empty() {
            return Err(SimError::Unarmed { id: attacker });
        }
        // Both sides' attacks and armours are adjusted by what their owners have researched: an
        // upgrade is resolved here, at order time, so the tick loop still never reads a table.
        let attacks = self.adjusted(source.player, attacker_stats, "attack");
        let armours = self.adjusted(victim.player, victim_stats, "armour");
        let damage = damage_between(&attacks, &armours, &rules);
        let reload_ticks = attacker_stats
            .reload
            .resolve(self.scale, "reload")
            .map(|reload| reload.raw() * i64::from(self.tick_rate_hz) / self.scale)
            .ok()
            .and_then(|ticks| u32::try_from(ticks).ok())
            .filter(|ticks| *ticks > 0)
            .ok_or(SimError::Unarmed { id: attacker })?;
        // Range is attribute 1, in tiles, and the upgrades that move it (Fletching and its line, +1
        // each) are kind-4 additions - so it resolves exactly like speed: base, then the owner's
        // modifiers at the engine scale. Zero stays zero: a melee unit has no reach to extend.
        let base = attacker_stats
            .range
            .resolve(self.scale, "range")
            .map_err(|err| SimError::Unresolved {
                id: attacker_stats.id,
                detail: err.to_string(),
            })?;
        let range = scale_fixed(
            base,
            self.scale,
            &self.modifiers_for(source.player, attacker_stats, "range"),
        );
        // The dead zone: a siege engine cannot shoot what stands next to it, and the number is the
        // unit's own (a mangonel 3 tiles, a bombard cannon 5, everything else 0), moved by any tech that
        // sets attribute 20 (D44).
        let min_base = attacker_stats
            .min_range
            .resolve(self.scale, "min_range")
            .map_err(|err| SimError::Unresolved {
                id: attacker_stats.id,
                detail: err.to_string(),
            })?;
        let min_range = scale_fixed(
            min_base,
            self.scale,
            &self.modifiers_for(source.player, attacker_stats, "minimum_range"),
        );
        Ok((damage, reload_ticks, range, min_range))
    }

    /// Resolve the numbers at order time and start swinging.
    fn start_attack(&mut self, attacker: EntityId, target: EntityId) -> Result<(), SimError> {
        let (damage, reload_ticks, range, min_range) = self.check_attack(attacker, target)?;
        let Some(source) = self.find(attacker) else {
            return Err(SimError::MissingEntity(attacker));
        };
        let position = source.position;
        let Some(victim) = self.find(target).map(|e| e.position) else {
            return Err(SimError::MissingEntity(target));
        };
        // A building does not walk: its reach is its range, and pathing from a blocked tile would
        // refuse every shot a town centre wants to take. The test is the kind, not the speed - a unit
        // with no speed still gets a route, which is what a stationary test fixture relies on.
        if source.kind != EntityKind::Building {
            let route = self.plan_path(position, victim)?;
            self.paths.insert(attacker, route);
        }
        if let Some(entity) = self.find_mut(attacker) {
            entity.task = Task::Attack {
                target,
                damage,
                reload_ticks,
                cooldown_ticks: 0,
                range,
                min_range,
            };
        }
        Ok(())
    }

    /// The checks a walk order has to survive: the unit is a unit, the tile is on the map, and there
    /// is a route to it. Returns the goal as a world position.
    fn check_order(&self, unit: EntityId, to: (u16, u16)) -> Result<(Fixed, Fixed), SimError> {
        let entity = self.find(unit).ok_or(SimError::MissingEntity(unit))?;
        if entity.kind != EntityKind::Unit {
            return Err(SimError::NotAUnit { id: unit });
        }
        if !self.on_map(to.0, to.1) {
            return Err(SimError::OffMap { tile: to });
        }
        let goal = self.tile(to);
        self.plan_path(entity.position, goal)?;
        Ok(goal)
    }

    /// Start a walk: an ordinary move, an attack-move, or a patrol. `patrol` only decides whether the
    /// tile the unit is standing on is remembered as the other end.
    ///
    /// The order is stored as `Task::Hunt` for the two fighting orders and `Task::Move` for a plain
    /// one, so a plain move stays exactly what it was: a walk that ignores everything on the way.
    fn start_walk(
        &mut self,
        unit: EntityId,
        to: (u16, u16),
        fighting: bool,
        patrol: bool,
    ) -> Result<(), SimError> {
        // A crowd ordered to one tile is a crowd ordered to a *place*: each unit takes a tile of its own
        // near the one that was clicked, in the order the orders arrive. Without this they all aim at one
        // tile, and with the last-step rule (D52) all but one stop a step short of it (D53).
        let already_heading = self
            .entities
            .iter()
            .filter(|entity| entity.id != unit)
            .filter(|entity| matches!(self.goal_of(entity), Some((tile, true)) if tile == to))
            .count();
        let to = self.slot_for(to, already_heading);
        let goal = self.check_order(unit, to)?;
        let entity = self.find(unit).ok_or(SimError::MissingEntity(unit))?;
        let route = self.plan_path(entity.position, goal)?;
        let here = self.tile_of(entity.position);
        self.paths.insert(unit, route);
        // Any earlier order is superseded: a new order is not a second one running alongside.
        self.resuming.remove(&unit);
        if patrol {
            self.patrols.insert(unit, self.tile(here));
        } else {
            self.patrols.remove(&unit);
        }
        if let Some(entity) = self.find_mut(unit) {
            entity.task = if fighting {
                Task::Hunt { to: goal }
            } else {
                Task::Move { to: goal }
            };
        }
        Ok(())
    }

    /// The stance an order will meet once the queue has landed.
    ///
    /// With `command_apply = "next_tick"` a validator runs against state where earlier orders in the
    /// same tick have not been applied, so a `Stance` followed by an `Attack` in one tick would be
    /// judged against the old stance - the same trap the training queue and the research guard each
    /// fell into once. The last pending stance for the unit wins, as it would when applied.
    fn effective_stance(&self, id: EntityId) -> Stance {
        let mut stance = self.stance_of(id);
        for command in &self.queue {
            if let Command::Stance {
                unit,
                stance: pending,
            } = command
                && *unit == id
            {
                stance = *pending;
            }
        }
        stance
    }

    /// This unit's stance. The default is aggressive, which is what every unit did before stances
    /// existed, so a world that never sets one behaves exactly as it used to.
    fn stance_of(&self, id: EntityId) -> Stance {
        self.stances
            .get(&id)
            .map_or(Stance::Aggressive, |(stance, _)| *stance)
    }

    /// A unit comes to rest when its order ends: no task, and no motion. `velocity` is the fixture
    /// hook `Command::Spawn` uses to make a nameless mover walk, so it must be cleared here or a unit
    /// that arrives somewhere keeps sliding east for the rest of the match.
    fn stand_down(&mut self, id: EntityId) {
        if let Some(entity) = self.find_mut(id) {
            entity.task = Task::Idle;
            entity.velocity = (Fixed::ZERO, Fixed::ZERO);
        }
    }

    /// The nearest enemy a hunting unit can see and may attack, by id order on a tie.
    ///
    /// A hunt scans on a schedule (`combat.aggro_scan_ticks`, staggered by id) because this is the one
    /// order-path cost that grows with the size of the world: a pass over every entity for every
    /// hunting unit. Four looks a second at 20 Hz is invisible to a player and a fifth of the cost.
    fn nearest_enemy(&self, hunter: &Entity) -> Option<EntityId> {
        if self.aggro_radius <= 0 {
            return None;
        }
        let scan = self.aggro_scan_ticks;
        if scan > 1 && !(self.tick + u64::from(hunter.id.0)).is_multiple_of(scan) {
            return None;
        }
        let radius_squared = self.aggro_radius * self.aggro_radius;
        let mut best: Option<(i64, EntityId)> = None;
        for other in &self.entities {
            if other.player == hunter.player || other.player == 0 || other.hp <= 0 {
                continue;
            }
            if matches!(other.kind, EntityKind::Resource(_) | EntityKind::Projectile) {
                continue;
            }
            if matches!(other.task, Task::Garrisoned { .. }) {
                continue;
            }
            if self.team_of(other.player) == self.team_of(hunter.player) {
                continue;
            }
            let dx = other.position.0.raw() - hunter.position.0.raw();
            let dy = other.position.1.raw() - hunter.position.1.raw();
            let squared = dx * dx + dy * dy;
            if squared > radius_squared {
                continue;
            }
            let closer = match best {
                None => true,
                Some((best_squared, best_id)) => {
                    squared < best_squared || (squared == best_squared && other.id < best_id)
                }
            };
            if closer {
                best = Some((squared, other.id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// The checks a training order has to survive. Readiness comes before trainability on
    /// purpose: a house that is still going up should be told it is not finished, not that it
    /// makes nothing.
    fn check_train(&self, building: EntityId, unit: u32) -> Result<TrainPlan, SimError> {
        let entity = self
            .find(building)
            .ok_or(SimError::MissingEntity(building))?;
        if entity.kind != EntityKind::Building {
            return Err(SimError::TrainNeedsABuilding { id: building });
        }
        let building_unit = entity
            .unit
            .ok_or(SimError::BuildingNotReady { id: building })?;
        if entity.construction.is_some() {
            return Err(SimError::BuildingNotReady { id: building });
        }
        // A building holds `queue_length` units at once: the one being made and those behind it. The
        // order is refused only when there is no room, because the queue is content's number, not the
        // engine's (D33).
        // Orders can be waiting to be applied (`command_apply = "next_tick"`), and a queue limit that
        // ignored them would accept five in one tick and then hold all five - the limit is about what
        // the building will hold, not about what has been applied so far.
        let pending = self
            .queue
            .iter()
            .filter(|command| matches!(command, Command::Train { building: queued, .. } if *queued == building))
            .count();
        let held = u32::from(entity.training.is_some())
            + u32::try_from(self.queued.get(&building).map_or(0, Vec::len)).unwrap_or(u32::MAX)
            + u32::try_from(pending).unwrap_or(u32::MAX);
        let limit = self
            .economy
            .as_ref()
            .map_or(1, |economy| economy.train.queue_length);
        if held >= limit {
            return Err(SimError::QueueFull {
                id: building,
                length: limit,
            });
        }
        let economy = self.economy.as_ref().ok_or(SimError::NoTables)?;
        // The same availability rule as building: a trebuchet needs the tech that makes it available,
        // and "no way to have it yet" outranks "not enough wood".
        if let Some(civ) = self.civ_barred_unit(entity.player, unit) {
            return Err(SimError::UnitBarred { unit, civ });
        }
        if !self.unit_available(entity.player, unit) {
            return Err(SimError::UnitUnavailable { unit });
        }
        // Population is not an ordering rule: a queue may hold more than the player can support, and
        // the *training* pauses at the ceiling instead (D34). Refusing here would make the ceiling a
        // wall you cannot queue behind, which is not how the game plays.
        // An upgrade target is not available on its own: a barracks trains pikemen only after the
        // Pikeman tech. Without this, `[train.entry]` naming the spear line would let a Dark Age
        // barracks train halberdiers.
        if let Some(tech) = self.upgrade_tech.get(&unit).copied()
            && !self.researched(entity.player).contains(&tech)
        {
            return Err(SimError::UnitNotUpgraded { unit, tech });
        }
        // A barracks upgraded by an age is unit 498, and `[train.entry]` names 12.
        let building_root = self
            .units
            .as_ref()
            .map_or(building_unit, |units| units.root(building_unit));
        // The line counts: `[train.entry]` says a barracks trains spearmen, and a barracks keeps doing
        // that as the line upgrades, so a pikeman matches through the family root.
        let unit_root = self.units.as_ref().map_or(unit, |units| units.root(unit));
        if !(economy.train.allows(building_unit, unit)
            || economy.train.allows(building_root, unit)
            || economy.train.allows(building_unit, unit_root)
            || economy.train.allows(building_root, unit_root))
        {
            return Err(SimError::NotTrainableHere {
                building: building_unit,
                unit,
            });
        }
        let units = self.units.as_ref().ok_or(SimError::NoTables)?;
        let stats = units.unit(unit).ok_or(SimError::MissingUnit { id: unit })?;
        let ticks = u32::try_from(stats.train_time)
            .ok()
            .and_then(|seconds| seconds.checked_mul(self.tick_rate_hz))
            .filter(|ticks| *ticks > 0)
            .ok_or(SimError::NotTrainableHere {
                building: building_unit,
                unit,
            })?;

        for cost in &stats.costs {
            let slot = self.cost_slot(cost)?;
            let need = u32::from(cost.amount);
            let have = self.stock(entity.player, slot);
            if have < need {
                return Err(SimError::NotEnough {
                    resource: self.slot_name(slot),
                    need,
                    have,
                });
            }
        }

        Ok(TrainPlan {
            costs: stats.costs.clone(),
            ticks,
            hp: stats.hp,
            speed: stats
                .speed
                .resolve(self.scale, "speed")
                .unwrap_or(Fixed::ZERO),
            player: entity.player,
        })
    }

    /// Pay for a unit and put it in the building's single production slot.
    fn start_train(&mut self, building: EntityId, unit: u32) -> Result<(), SimError> {
        let plan = self.check_train(building, unit)?;
        for cost in &plan.costs {
            let slot = self.cost_slot(cost)?;
            let index = usize::from(plan.player).wrapping_sub(1);
            let Some(row) = self.stock.get_mut(index) else {
                return Err(SimError::PlayerOutOfRange {
                    player: plan.player,
                });
            };
            row[slot] -= u32::from(cost.amount);
        }
        let job = Training {
            unit,
            remaining_ticks: plan.ticks,
            hp: plan.hp,
            speed: plan.speed,
            fresh: true,
        };
        let Some(entity) = self.find_mut(building) else {
            return Err(SimError::MissingEntity(building));
        };
        if entity.training.is_none() {
            entity.training = Some(job);
        } else {
            self.queued.entry(building).or_default().push(job);
        }
        Ok(())
    }

    /// Put a finished unit on the first free tile next to its building, in a fixed scan order so
    /// the result is deterministic. Returns false when there is nowhere to stand, in which case
    /// the unit waits inside the building rather than being lost.
    fn spawn_from_building(&mut self, building: EntityId) -> bool {
        let Some(job) = self.find(building).and_then(|e| e.training) else {
            return true;
        };
        let Some(source) = self.find(building) else {
            return true;
        };
        let player = source.player;
        // Health and speed are resolved when the unit appears, so a tech researched while it was in
        // the queue still counts.
        let hp = self.max_hp(player, job.unit).unwrap_or(job.hp);
        let speed = self.effective_speed(player, job.unit).unwrap_or(job.speed);
        let unit = job.unit;
        let scale = self.scale;
        let origin = (
            u16::try_from(source.position.0.raw() / scale).unwrap_or(0),
            u16::try_from(source.position.1.raw() / scale).unwrap_or(0),
        );
        let walk = match self.walk.as_ref() {
            Some(walk) => walk,
            None => return false,
        };
        let mut spot = None;
        'search: for dy in 0..=2i32 {
            for dx in 0..=2i32 {
                for (sx, sy) in [(dx, dy), (-dx, dy), (dx, -dy), (-dx, -dy)] {
                    let cand = (i32::from(origin.0) + sx, i32::from(origin.1) + sy);
                    if cand.0 < 0 || cand.1 < 0 {
                        continue;
                    }
                    let (cx, cy) = (
                        u16::try_from(cand.0).unwrap_or(u16::MAX),
                        u16::try_from(cand.1).unwrap_or(u16::MAX),
                    );
                    if !walk.contains(cx, cy) || walk.blocked(cx, cy) {
                        continue;
                    }
                    let position = self.tile((cx, cy));
                    if self.entities.iter().any(|e| e.position == position) {
                        continue;
                    }
                    spot = Some((cx, cy));
                    break 'search;
                }
            }
        }
        let Some(spot) = spot else {
            return false;
        };
        let position = self.tile(spot);
        let id = EntityId(self.entities.iter().map(|e| e.id.0).max().unwrap_or(0) + 1);
        let inserted = self.insert(Entity {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position,
            speed,
            velocity: (Fixed::ZERO, Fixed::ZERO),
            hp,
            amount: Fixed::ZERO,
            task: Task::Idle,
            training: None,
            construction: None,
            researching: None,
            carry: Fixed::ZERO,
            variant: None,
        });
        if inserted.is_err() {
            return false;
        }
        // Where the finished unit goes: the rally decides, and the default is where it stands.
        let rally = self.rallies.get(&building).copied();
        match rally {
            Some(RallyPoint::Inside) => {
                // Only a building that can hold units can keep one, and only while there is room.
                let room = self
                    .find(building)
                    .and_then(|entity| entity.unit)
                    .and_then(|unit| self.units.as_ref().and_then(|units| units.unit(unit)))
                    .is_some_and(|stats| stats.garrison > 0);
                if room
                    && self.check_garrison(id, building).is_ok()
                    && let Some(entity) = self.find_mut(id)
                {
                    entity.task = Task::Garrisoned { building };
                }
            }
            Some(RallyPoint::Tile(x, y)) => {
                let to = self.tile((x, y));
                // A rally point on a resource is a gathering order: the villager goes there and works
                // *that* node. Rallied onto gold with wood all round it, it gathers the gold or
                // nothing - never the nearest tree (D40).
                let node = self
                    .entities
                    .iter()
                    .find(|entity| {
                        matches!(entity.kind, EntityKind::Resource(_)) && entity.position == to
                    })
                    .map(|entity| entity.id);
                let gathering = node.is_some_and(|node| self.start_gather(id, node).is_ok());
                if !gathering {
                    // Nothing to work there, or this unit cannot work it (a spearman rallied onto a
                    // tree), or there is no way to reach it: go as close as the map allows.
                    self.walk_towards(id, to);
                }
            }
            None => {}
        }

        // The next unit in the queue starts where the finished one left off: its own create time was
        // priced when it was ordered, and it begins counting from that total.
        let next = self.queued.get_mut(&building).and_then(|waiting| {
            if waiting.is_empty() {
                None
            } else {
                Some(waiting.remove(0))
            }
        });
        if let Some(source) = self.find_mut(building) {
            source.training = next;
        }
        if self.queued.get(&building).is_some_and(Vec::is_empty) {
            self.queued.remove(&building);
        }
        true
    }

    /// One tick of swinging: count the reload down and land a blow when it reaches zero.
    ///
    /// The countdown happens before the check on purpose. Firing on the tick the counter *reaches*
    /// zero rather than the tick after makes a 2 s reload forty ticks at 20 Hz instead of
    /// forty-one, which only shows up when something is timed to the blow.
    fn swing(&mut self, attacker: EntityId) -> i32 {
        let Some(entity) = self.find_mut(attacker) else {
            return 0;
        };
        let Task::Attack {
            target,
            damage,
            reload_ticks,
            mut cooldown_ticks,
            range,
            min_range,
        } = entity.task
        else {
            return 0;
        };
        cooldown_ticks = cooldown_ticks.saturating_sub(1);
        let landed = if cooldown_ticks == 0 {
            cooldown_ticks = reload_ticks;
            damage
        } else {
            0
        };
        entity.task = Task::Attack {
            target,
            damage,
            reload_ticks,
            cooldown_ticks,
            range,
            min_range,
        };
        landed
    }

    /// Cancel a foundation of mine whose tile turns out to hold an enemy building that is already
    /// underway, once I can see that tile (D61).
    ///
    /// This is the second half of the blind-placement rule: you may lay a foundation in fog where
    /// something stands, because the player chose blind, and the game cancels the dead placement when
    /// your own sight reaches it - not at the click, and not when a builder finally arrives. The
    /// resources come back (D58a), because the player was never given a chance to know.
    ///
    /// An enemy *untouched* foundation is not a reason to cancel: two bare foundations are decided by
    /// whoever puts a builder to work first (`cancel_overbuilt`). Only something already started or
    /// finished is a settled fact that a late placement cannot undo.
    fn settle_blind_sites(&mut self) {
        // Nothing under construction: the pass costs one set lookup. It used to cost a walk of every entity to ask the
        // same question, every tick, in every match (item 6).
        if self.foundations.is_empty() {
            return;
        }
        // A maintained set drifts: this is the assertion that catches it, and it costs nothing in release.
        debug_assert_eq!(
            self.foundations.len(),
            self.entities
                .iter()
                .filter(|entity| entity.construction.is_some())
                .count(),
            "the foundation set disagrees with the world"
        );
        let sites: Vec<(EntityId, u8, (Fixed, Fixed))> = self
            .entities
            .iter()
            .filter(|entity| {
                entity.kind == EntityKind::Building
                    && entity.construction.is_some()
                    && entity.hp > 0
            })
            .map(|entity| (entity.id, entity.player, entity.position))
            .collect();
        let mut doomed = Vec::new();
        for (id, player, position) in sites {
            let tile = (
                u16::try_from((position.0.raw() / self.scale).max(0)).unwrap_or(u16::MAX),
                u16::try_from((position.1.raw() / self.scale).max(0)).unwrap_or(u16::MAX),
            );
            if !self.unit_sees(player, tile) {
                continue;
            }
            let taken = self.entities.iter().any(|other| {
                other.id != id
                    && other.player != 0
                    && other.player != player
                    && other.kind == EntityKind::Building
                    && other.position == position
                    && other
                        .construction
                        .is_none_or(|construction| construction.progress_ticks > 0)
            });
            if taken {
                doomed.push(id);
            }
        }
        for id in doomed {
            self.cancel_site(id);
        }
    }

    /// Cancel every other player's building sharing this site's tile (D58).
    ///
    /// Two buildings can only share a tile because one of them was placed blind, and this is where the
    /// blind one loses: the builder who starts working takes the ground. The victim is killed rather than
    /// deleted so the ordinary death sweep releases its bookkeeping and its tile - a cancelled wall must
    /// not keep blocking, and a cancelled site must not keep a `paths` entry.
    fn cancel_overbuilt(&mut self, site: EntityId) {
        let Some((position, player)) = self
            .find(site)
            .map(|entity| (entity.position, entity.player))
        else {
            return;
        };
        let theirs: Vec<(EntityId, bool)> = self
            .entities
            .iter()
            .filter(|entity| {
                entity.id != site
                    && entity.player != player
                    && entity.player != 0
                    && entity.position == position
                    && matches!(entity.kind, EntityKind::Building)
            })
            // "Underway" means started or finished: the owner's rule is that whoever starts building
            // first wins, so a site the other side has already put a builder to work on is not a
            // newcomer's to take (D61).
            .map(|entity| {
                (
                    entity.id,
                    entity
                        .construction
                        .is_none_or(|construction| construction.progress_ticks > 0),
                )
            })
            .collect();
        if theirs.iter().any(|(_, underway)| *underway) {
            self.cancel_site(site);
            return;
        }
        for (victim, _) in theirs {
            self.cancel_site(victim);
        }
    }

    /// Cancel a foundation and pay its owner back (D58a). The price is recomputed from content rather than
    /// carried on the entity: costs are not per-player yet, so what was paid and what is returned cannot
    /// drift - and the day a discount exists, this is the one place that has to learn about it.
    fn cancel_site(&mut self, site: EntityId) {
        let Some((player, unit, alive)) = self
            .find(site)
            .map(|entity| (entity.player, entity.unit, entity.hp > 0))
        else {
            return;
        };
        // Idempotent: a site cancelled earlier in the same tick is still in the list until the sweep,
        // and refunding it twice would pay its owner for a building that only cost once.
        if !alive {
            return;
        }
        if let Some(unit) = unit {
            self.refund_unit_cost(player, unit);
        }
        if let Some(entity) = self.find_mut(site) {
            entity.hp = 0;
        }
    }

    /// Give `player` back what one of `unit` costs. Saturating, because a refund is not a way to overflow a
    /// stockpile.
    fn refund_unit_cost(&mut self, player: u8, unit: u32) {
        self.refund_unit_cost_part(player, unit, 1, 1);
    }

    /// Give `player` back `left / total` of what one of `unit` costs - the fraction of a site that is not
    /// built yet. Integer arithmetic floors, so a nearly finished building returns almost nothing and never
    /// returns more than was paid.
    fn refund_unit_cost_part(&mut self, player: u8, unit: u32, left: u32, total: u32) {
        let Some(costs) = self
            .units
            .as_ref()
            .and_then(|units| units.unit(unit))
            .map(|stats| stats.costs.clone())
        else {
            return;
        };
        let total = total.max(1);
        let mut refunds = Vec::with_capacity(costs.len());
        for cost in &costs {
            if let Ok(slot) = self.cost_slot(cost) {
                let amount = u32::from(cost.amount).saturating_mul(left) / total;
                refunds.push((slot, amount));
            }
        }
        let Some(row) = self.stock.get_mut(usize::from(player).wrapping_sub(1)) else {
            return;
        };
        for (slot, amount) in refunds {
            if let Some(have) = row.get_mut(slot) {
                *have = have.saturating_add(amount);
            }
        }
    }

    /// One tick of work on a site. Returns true when this tick finished the building. HP is
    /// derived from progress rather than accumulated, so a mill's 17.14 HP/s cannot drift.
    fn work_one_tick(&mut self, site: EntityId) -> bool {
        // The first tick of work takes the tile (D58). A site that stands on an enemy's foundation can
        // only be there because it was placed blind, and the builder who starts working cancels the one
        // who did not get there first. Cancelled before the borrow below, which needs `self` whole.
        let first_tick = self
            .find(site)
            .and_then(|entity| entity.construction)
            .is_some_and(|construction| construction.progress_ticks == 0);
        if first_tick {
            self.cancel_overbuilt(site);
        }
        let Some(building) = self.find_mut(site) else {
            return false;
        };
        let Some(mut construction) = building.construction else {
            return false;
        };
        construction.progress_ticks = construction.progress_ticks.saturating_add(1);
        let total = construction.total_ticks.max(1);
        let hp = if construction.progress_ticks >= total {
            construction.max_hp
        } else {
            let scaled = i64::from(construction.max_hp) * i64::from(construction.progress_ticks)
                / i64::from(total);
            i32::try_from(scaled).unwrap_or(1).max(1)
        };
        building.hp = hp;
        let done = construction.progress_ticks >= total;
        building.construction = Some(construction);
        done
    }

    /// The checks a build order has to survive: is it a building, can the player pay, is the
    /// tile free. Returns what the order will cost and how long it will take.
    fn check_build(
        &self,
        villager: EntityId,
        unit_id: u32,
        at: (u16, u16),
    ) -> Result<BuildPlan, SimError> {
        let units = self.units.as_ref().ok_or(SimError::NoTables)?;
        let stats = units
            .unit(unit_id)
            .ok_or(SimError::MissingUnit { id: unit_id })?;
        let speed = stats
            .speed
            .resolve(self.scale, "speed")
            .unwrap_or(Fixed::ZERO);
        if speed.raw() != 0 || stats.train_time <= 0 {
            return Err(SimError::NotABuilding { unit: unit_id });
        }
        let total = u32::try_from(stats.train_time)
            .ok()
            .and_then(|seconds| seconds.checked_mul(self.tick_rate_hz))
            .ok_or(SimError::NotABuilding { unit: unit_id })?;
        let worker = self
            .find(villager)
            .ok_or(SimError::MissingEntity(villager))?;

        // Availability outranks the price: "you cannot have this yet" is the answer, not "you are poor".
        if let Some(civ) = self.civ_barred_unit(worker.player, unit_id) {
            return Err(SimError::UnitBarred { unit: unit_id, civ });
        }
        if !self.unit_available(worker.player, unit_id) {
            return Err(SimError::UnitUnavailable { unit: unit_id });
        }
        // A building prerequisite is about history, not the map (D63): the requirement is that this
        // player has finished one at some point, so a razed blacksmith does not take the workshop away.
        if let Some((need, name)) = self.requires_built.get(&units.root(unit_id))
            && !self.built.contains(&(worker.player, *need))
        {
            return Err(SimError::MissingBuilding { need: name.clone() });
        }
        for cost in &stats.costs {
            let slot = self.cost_slot(cost)?;
            let need = u32::from(cost.amount);
            let have = self.stock(worker.player, slot);
            if have < need {
                return Err(SimError::NotEnough {
                    resource: self.slot_name(slot),
                    need,
                    have,
                });
            }
        }
        let walk = self.walk.as_ref().ok_or(SimError::NoTables)?;
        if !walk.contains(at.0, at.1) || walk.blocked(at.0, at.1) {
            return Err(SimError::SiteBlocked { tile: at });
        }
        let position = self.tile(at);
        // One tile holds one building per player, and a building you cannot see does not block you
        // (D58): the player chooses blind, so the answer has to be the world *as that player sees it*.
        // Your own foundation always blocks - stacking your own is never what the click meant - and a
        // resource node blocks whoever is looking, because it is not a player's to lose.
        // Only a *foundation* can be overbuilt blind. A finished building blocks whoever is looking: the
        // rule is about a site someone has merely marked out, and letting a house cancel a town centre
        // through fog is not the game either version plays.
        let blind = self.vision(worker.player, at) != TileVision::Visible;
        // **Units do not block a site.** The game lets you drop a building on your own villagers and pushes them out;
        // refusing instead made a site beside a drop site impossible to place, because a villager now stands there
        // (deposits are made from a touching tile, 2026-10-29). Resources and buildings still block.
        let blocked = self.entities.iter().any(|entity| {
            entity.kind != EntityKind::Unit
                // A spent farm is not in the way: the whole point of building on it is to replace it.
                && !self.is_expired_farm(entity.id)
                && entity.position == position
                && (entity.player == worker.player || entity.player == 0 || !blind)
        });
        if blocked {
            return Err(SimError::SiteOccupied { tile: at });
        }
        // Reachability is part of "can this order run": paying for a site nobody can walk to would
        // leave a construction that never progresses and wood the player cannot get back.
        self.plan_path(worker.position, position)?;

        Ok(BuildPlan {
            total_ticks: total,
            // Derived, not copied: a building ordered after a health tech must be quoted the new
            // maximum, or it finishes at the old one.
            max_hp: self.max_hp(worker.player, unit_id).unwrap_or(stats.hp),
            costs: stats.costs.clone(),
            player: worker.player,
        })
    }

    /// A herdable changes hands by **line of sight** (D70, item 5): a *unit's* sight, not a building's, immediate,
    /// herdables only. Unowned animals join the first player who sees them; an owned one is stolen when another
    /// player sees it and **no unit of its owner's does**; a tie goes to the side with more units near it.
    ///
    /// Cost: one walk of the units times the herdables alive, and nothing at all when no herdable stands - which is
    /// the early return below. The counts are a fixed array per animal, so no allocation happens per player.
    fn step_conversion(&mut self) {
        if self.herdable_ids.is_empty() || self.sight.is_empty() {
            return;
        }
        // Copies out of the borrow: the decisions below change tasks and owners.
        let seers: Vec<(u8, (Fixed, Fixed), i64)> = self
            .entities
            .iter()
            .filter(|entity| entity.kind == EntityKind::Unit && entity.player > 0 && entity.hp > 0)
            .filter(|entity| !matches!(entity.task, Task::Garrisoned { .. }))
            // An animal is not a witness: a sheep has a line of sight of its own, and counting it would let every
            // owned animal keep itself - which is exactly what the first version did (seen = [1, 0, ...]).
            .filter(|entity| !entity.unit.is_some_and(|unit| self.animals.contains(&unit)))
            .filter_map(|entity| {
                let reach = entity.unit.and_then(|unit| self.sight.get(&unit))?;
                Some((entity.player, entity.position, *reach))
            })
            .collect();
        if seers.is_empty() {
            return;
        }
        let animals: Vec<(EntityId, u8, (Fixed, Fixed))> = self
            .herdable_ids
            .iter()
            .filter_map(|id| {
                self.find(*id)
                    .filter(|entity| entity.hp > 0)
                    .map(|entity| (entity.id, entity.player, entity.position))
            })
            .collect();
        // (animal, new owner, whether it was somebody's already): only a *steal* disturbs the herder, because an
        // unowned animal joining the player whose villager is already walking it is the ordinary case.
        let mut taken: Vec<(EntityId, u8, bool)> = Vec::new();
        for (id, owner, position) in animals {
            let mut seen: [u32; MAX_PLAYERS] = [0; MAX_PLAYERS];
            for (player, at, reach) in &seers {
                let dx = at.0.raw() - position.0.raw();
                let dy = at.1.raw() - position.1.raw();
                if dx * dx + dy * dy <= *reach {
                    seen[usize::from(*player) - 1] += 1;
                }
            }
            // The owner's own sight keeps the animal: that is the manual's second sentence.
            if owner > 0 && seen[usize::from(owner) - 1] > 0 {
                continue;
            }
            // Among the players who can see it, the one with most units near wins; a tie goes to the lower player
            // number, so two runs of the same match agree.
            let mut best: Option<(u8, u32)> = None;
            for (index, count) in seen.iter().enumerate() {
                if *count == 0 {
                    continue;
                }
                let player = u8::try_from(index + 1).unwrap_or(u8::MAX);
                if player == owner {
                    continue;
                }
                if best.is_none_or(|(_, best_count)| *count > best_count) {
                    best = Some((player, *count));
                }
            }
            if let Some((player, _)) = best {
                taken.push((id, player, owner > 0));
            }
        }
        for (id, player, stolen) in taken {
            if let Some(entity) = self.find_mut(id) {
                entity.player = player;
            }
            if stolen {
                // **Stolen**: it belongs to somebody else now, so it walks to nobody's drop site and the villager
                // that was herding it stops rather than following a stranger's sheep home. That window - the walk,
                // while the animal is alive - is the one D70 kept pointing at.
                self.paths.remove(&id);
                self.leash.remove(&id);
                let herders: Vec<EntityId> = self
                    .entities
                    .iter()
                    .filter(|entity| matches!(entity.task, Task::Gather { node, .. } if node == id))
                    .map(|entity| entity.id)
                    .collect();
                for herder in herders {
                    self.stand_down(herder);
                    self.slaughter.remove(&herder);
                }
            }
            self.stamp_entity(id);
        }
    }

    /// What a blow is worth against *this* victim. A **herdable is livestock, not an opponent**: a butcher's strike -
    /// the shepherd's knife or the hunter's arrow - takes it outright, so a 7-health sheep dies of one blow rather than
    /// of three (the owner's rule). A boar or a deer is fought, which is what makes a boar dangerous.
    fn blow_against(&self, attacker: EntityId, target: EntityId, damage: i32) -> i32 {
        let Some(victim) = self.find(target) else {
            return damage;
        };
        if !victim
            .unit
            .is_some_and(|unit| self.herdable.contains(&unit))
        {
            return damage;
        }
        let butcher = self
            .find(attacker)
            .and_then(|entity| entity.unit)
            .is_some_and(|unit| self.butchers.contains(&unit));
        if butcher {
            damage.max(victim.hp)
        } else {
            damage
        }
    }

    /// Death leaves a body: an animal at or below zero health becomes its `dead_unit` **in place**, and it holds
    /// its meat only if the last blow was a butcher's (D70, item 2).
    ///
    /// In place, and on the same id, because the gatherer's task names that id: a villager working a sheep carries
    /// on with the carcass without a re-target, which is the whole reason the slaughter transform was shaped this
    /// way. Health, speed, velocity and the variant all go - a body is not a unit that may move or be attacked.
    fn promote_dead_animals(
        &mut self,
        blows: &[(EntityId, EntityId)],
        new_rots: &mut Vec<(EntityId, Rot)>,
    ) {
        if self.animal_bodies.is_empty() || blows.is_empty() {
            return;
        }
        // Only this tick's victims can have died on this tick, so the candidates are a handful rather than the
        // whole entity list: a pass over every entity per tick is exactly the shape a review exists to catch.
        let mut seen: Vec<EntityId> = Vec::new();
        for (victim, _) in blows.iter().rev() {
            if seen.contains(victim) {
                continue;
            }
            seen.push(*victim);
            let Some(index) = self.entities.iter().position(|entity| entity.id == *victim) else {
                continue;
            };
            let (hp, unit, amount, kind) = {
                let entity = &self.entities[index];
                (entity.hp, entity.unit, entity.amount, entity.kind)
            };
            if kind != EntityKind::Unit || hp > 0 {
                continue;
            }
            let Some(body) = unit.and_then(|unit| self.animal_bodies.get(&unit).copied()) else {
                continue;
            };
            // The last blow decides: a soldier's or a tower's kill leaves a body with nothing in it.
            let killer = blows
                .iter()
                .rev()
                .find(|(id, _)| id == victim)
                .map(|(_, attacker)| *attacker);
            let butchered = killer.is_some_and(|killer| {
                self.find(killer)
                    .and_then(|entity| entity.unit)
                    .is_some_and(|unit| self.butchers.contains(&unit))
            });
            let id = *victim;
            let entity = &mut self.entities[index];
            entity.unit = Some(body);
            entity.amount = if butchered { amount } else { Fixed::ZERO };
            entity.hp = 0;
            entity.speed = Fixed::ZERO;
            entity.velocity = (Fixed::ZERO, Fixed::ZERO);
            entity.variant = None;
            entity.task = Task::Idle;
            // Bookkeeping goes with the animal: a body chases nothing and walks nowhere. A body that holds meat
            // starts rotting at its species' rate; an empty one is left to the sweep's business for now.
            self.paths.remove(&id);
            self.leash.remove(&id);
            self.rots.remove(&id);
            // Every body has a lifetime, meat or no meat: a soldier's kill leaves a carcass with nothing in it, and
            // before this it stood on the map for the rest of the match (item 6). A body that holds meat rots it
            // away and goes when it is empty; an empty one simply lies for its species' span and then goes.
            if let Some(unit) = unit {
                let rate = self.animal_rot.get(&unit).copied().unwrap_or(0);
                let span = self.animal_span.get(&unit).copied().unwrap_or(0);
                if span > 0 {
                    new_rots.push((
                        id,
                        Rot {
                            per_tick: if butchered { rate } else { 0 },
                            expires: self.tick.saturating_add(span),
                        },
                    ));
                }
            }
        }
    }

    /// Start a building: check the price, pay it, place the site and send the builder.
    ///
    /// "Is this a building?" is answered from the data rather than a hard-coded class list: a
    /// building has no speed and a positive creation time. A unit with a speed walks; a unit with
    /// no creation time cannot be put up.
    /// An animal takes being struck as an order: it turns on whoever hit it (D70). Nothing else does - a
    /// villager shot at keeps gathering, which is the game's rule and not an oversight here.
    fn retaliate(&mut self, victim: EntityId, attacker: EntityId) {
        if attacker == victim {
            return;
        }
        let Some(victim_unit) = self
            .find(victim)
            .filter(|entity| entity.hp > 0)
            .and_then(|entity| entity.unit)
        else {
            return;
        };
        let is_animal = self.units.as_ref().is_some_and(|units| {
            let root = units.root(victim_unit);
            self.animals.contains(&root)
        });
        if !is_animal {
            return;
        }
        // A sheep has nothing to answer with, and trying would record an `Unarmed` refusal for every slaughtered
        // animal - noise that would bury the refusals that matter (item 3).
        if !self.armed_animals.contains(&victim_unit) {
            return;
        }
        // Already fighting this one back: re-issuing would reset its cooldown on every blow.
        if matches!(
            self.find(victim).map(|entity| entity.task),
            Some(Task::Attack { target, .. }) if target == attacker
        ) {
            return;
        }
        // Interest, renewed by every blow (D71). This happens *before* the early return above, because
        // "already fighting this one" is exactly the case where the clock matters: a boar struck every
        // two seconds must never lose interest, and one struck again on its way home must turn round.
        let renewed = self.renew_interest(victim);
        if renewed {
            if let Err(error) = self.start_attack(victim, attacker) {
                self.refusal = Some((victim, error, self.tick));
            }
            return;
        }
        // A reaction has no caller to return an error to, and dropping it is how a real defect stayed
        // invisible for two sessions - the old `let _ =` printed only behind an env var nobody set, and
        // the instrumented `eprintln!` never appeared because this path was never reached at all (D71).
        if let Err(error) = self.start_attack(victim, attacker) {
            self.refusal = Some((victim, error, self.tick));
        }
    }

    /// Record - or renew - an animal's interest in its attacker. Returns whether this blow *renewed* a
    /// chase that had already been given up, which is what makes the owner's second arrow turn it round.
    fn renew_interest(&mut self, victim: EntityId) -> bool {
        if self.leash_ticks.is_none() && self.leash_tiles.is_none() {
            // A pack that states neither rule gets an animal that chases for ever, exactly as it did
            // before the leash existed - and pays nothing for it, because no entry is ever made.
            return false;
        }
        let until = self
            .leash_ticks
            .map_or(u64::MAX, |ticks| self.tick.saturating_add(ticks));
        let position = self.find(victim).map(|entity| entity.position);
        match self.leash.get_mut(&victim) {
            Some(entry) => {
                let was_returning = !entry.chasing;
                entry.until = until;
                entry.chasing = true;
                was_returning
            }
            None => {
                let home = position.unwrap_or((Fixed::ZERO, Fixed::ZERO));
                self.leash.insert(
                    victim,
                    Leash {
                        home,
                        until,
                        chasing: true,
                    },
                );
                false
            }
        }
    }

    /// The animal's interest, once per tick (D71). Three states in one entry: absent is grazing at home,
    /// an entry with `chasing` is hunting, and the same entry without it is walking back to where it was
    /// first struck. Costs one walk of the provoked animals and nothing at all in a world at peace.
    fn step_leashes(&mut self) {
        if self.leash.is_empty() {
            return;
        }
        let now = self.tick;
        let ids: Vec<EntityId> = self.leash.keys().copied().collect();
        for id in ids {
            let Some(entry) = self.leash.get(&id).copied() else {
                continue;
            };
            let Some(entity) = self.find(id) else {
                self.leash.remove(&id);
                continue;
            };
            // A body is not an animal any more: nothing chases anything from zero health.
            if entity.hp <= 0 {
                self.leash.remove(&id);
                continue;
            }
            if !entry.chasing {
                // Walking home is an ordinary `Move`; arriving is what ends the matter.
                if entity.task == Task::Idle {
                    self.leash.remove(&id);
                }
                continue;
            }
            let dx = (entity.position.0.raw() - entry.home.0.raw()).abs();
            let dy = (entity.position.1.raw() - entry.home.1.raw()).abs();
            let strayed = self.leash_tiles.is_some_and(|tiles| dx.max(dy) > tiles);
            let bored = now >= entry.until;
            if !strayed && !bored {
                continue;
            }
            // Out of interest. It stands still for as long as the clock allowed - which is the garrison
            // lure's window, where a boar that lost its target waits to be finished - and then walks
            // back to the place it was first struck.
            let position = entity.position;
            if let Some(entry) = self.leash.get_mut(&id) {
                entry.chasing = false;
            }
            self.stand_down(id);
            match self.plan_path(position, entry.home) {
                Ok(route) => {
                    self.paths.insert(id, route);
                    if let Some(entity) = self.find_mut(id) {
                        entity.task = Task::Move { to: entry.home };
                    }
                }
                Err(_) => {
                    // No way home: it stays where it stopped rather than walking through a wall. The
                    // entry goes, so a later blow starts a fresh chase from here.
                    self.leash.remove(&id);
                }
            }
        }
    }

    fn start_build(
        &mut self,
        villager: EntityId,
        unit_id: u32,
        at: (u16, u16),
    ) -> Result<(), SimError> {
        // Already building, or already waiting? Then this is the next one, not a replacement (D66). It is
        // checked now - a site that could never be built should be refused when it is ordered - and paid
        // for when it starts, so the queue cannot spend money a player has not got yet.
        let busy_building = self
            .find(villager)
            .is_some_and(|entity| matches!(entity.task, Task::Build { .. }));
        if busy_building || self.build_queue.contains_key(&villager) {
            self.check_build(villager, unit_id, at)?;
            self.build_queue
                .entry(villager)
                .or_default()
                .push_back((unit_id, at));
            return Ok(());
        }
        let BuildPlan {
            total_ticks,
            max_hp,
            costs,
            player,
        } = self.check_build(villager, unit_id, at)?;

        // Paid only after every check passed, so a refused order spends nothing.
        for cost in &costs {
            let slot = self.cost_slot(cost)?;
            let index = usize::from(player).wrapping_sub(1);
            let Some(row) = self.stock.get_mut(index) else {
                return Err(SimError::PlayerOutOfRange { player });
            };
            row[slot] -= u32::from(cost.amount);
        }

        let position = self.tile(at);
        // A farm is worth `food_per_farm` food the moment its plot is marked out, and it is
        // unworkable until finished - which is why the amount is set here and not on completion: the
        // plot is also the whole budget, and a plot destroyed half-built takes its food with it.
        let farm_food = self
            .economy
            .as_ref()
            .filter(|economy| {
                economy
                    .farm
                    .unit
                    .as_ref()
                    .and_then(|role| economy.role.get(role).copied())
                    == Some(unit_id)
            })
            .map(|_| Fixed::from_raw(self.farm_food_total(player) * self.scale))
            .unwrap_or(Fixed::ZERO);
        let id = EntityId(self.entities.iter().map(|e| e.id.0).max().unwrap_or(0) + 1);
        self.insert(Entity {
            id,
            player,
            kind: EntityKind::Building,
            unit: Some(unit_id),
            position,
            speed: Fixed::ZERO,
            velocity: (Fixed::ZERO, Fixed::ZERO),
            hp: 1,
            amount: farm_food,
            task: Task::Idle,
            training: None,
            construction: Some(Construction {
                progress_ticks: 0,
                total_ticks,
                max_hp,
            }),
            researching: None,
            carry: Fixed::ZERO,
            variant: None,
        })?;
        // A site that exists is a site the blind-placement pass has to look at, and the set is how it knows there is
        // one without walking the world every tick (item 6).
        self.foundations.insert(id);
        // A new farm on a spent one replaces it rather than standing beside it.
        let replaced: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|entity| entity.position == position && self.is_expired_farm(entity.id))
            .map(|entity| entity.id)
            .collect();
        self.entities
            .retain(|entity| !replaced.contains(&entity.id));
        self.push_units_off(at);

        let from = self
            .find(villager)
            .map(|entity| entity.position)
            .ok_or(SimError::MissingEntity(villager))?;
        // Beside the site, not on it: a builder standing inside the footprint has no free tile to
        // plan a route home from, and once finished the building would be under the villager.
        let route = match self.work_position(from, position) {
            Some(stand) => self.plan_path(from, stand)?,
            None => self.plan_path(from, position)?,
        };
        self.paths.insert(villager, route);
        if let Some(worker) = self.find_mut(villager) {
            worker.task = Task::Build { site: id };
        }
        Ok(())
    }

    /// The checks a research order has to survive: is this a finished building, is it the building
    /// the tech names, is the tech unknown / already had / already being researched here, and can
    /// the player pay. Returns the price and how many ticks it takes.
    /// The checks a garrison order has to survive: a unit, a finished building of the same player, room
    /// inside it, and the unit close enough to walk in - the capacity is the building's own data field.
    fn check_garrison(&self, unit: EntityId, building: EntityId) -> Result<(), SimError> {
        let source = self.find(unit).ok_or(SimError::MissingEntity(unit))?;
        if source.kind != EntityKind::Unit {
            return Err(SimError::NotAUnit { id: unit });
        }
        if matches!(source.task, Task::Garrisoned { .. }) {
            return Err(SimError::Garrisoned { id: unit });
        }
        let shelter = self
            .find(building)
            .ok_or(SimError::MissingEntity(building))?;
        if shelter.kind != EntityKind::Building {
            return Err(SimError::TrainNeedsABuilding { id: building });
        }
        if shelter.construction.is_some() {
            return Err(SimError::BuildingNotReady { id: building });
        }
        if shelter.player != source.player {
            return Err(SimError::FriendlyFire { id: building });
        }
        // Only a building that fights takes units in from outside. A production building - barracks,
        // archery range, stable, siege workshop, dock - holds nothing but what it has just trained, and
        // a unit that has left it can never come back. The data draws the line itself: exactly those
        // five carry a garrison capacity and no attack array, while the town centre, towers and castles
        // carry both (D38).
        let units_for_hp = self.units.as_ref().ok_or(SimError::NoTables)?;
        if let Some(stats) = shelter.unit.and_then(|built| units_for_hp.unit(built))
            && (stats.attacks.is_empty()
                || stats
                    .range
                    .resolve(self.scale, "range")
                    .map_or(0, |range| range.raw())
                    <= 0)
        {
            return Err(SimError::CannotGarrisonHere { id: building });
        }
        if let Some(stats) = shelter.unit.and_then(|built| units_for_hp.unit(built)) {
            let max = self.max_hp(shelter.player, stats.id).unwrap_or(stats.hp);
            let floor = (i64::from(max) * self.garrison_eject_at.raw()) / self.scale;
            if i64::from(shelter.hp) <= floor {
                return Err(SimError::GarrisonTooDamaged { id: building });
            }
        }
        let units = self.units.as_ref().ok_or(SimError::NoTables)?;
        let capacity = shelter
            .unit
            .and_then(|built| units.unit(built))
            .map_or(0, |stats| u32::from(stats.garrison));
        let inside = u32::try_from(
            self.entities
                .iter()
                .filter(|entity| {
                    entity.player == source.player
                        && entity.kind == EntityKind::Unit
                        && entity.task == (Task::Garrisoned { building })
                })
                .count(),
        )
        .unwrap_or(u32::MAX);
        if inside >= capacity {
            return Err(SimError::GarrisonFull {
                id: building,
                capacity,
            });
        }
        // Standing on the building's tile or beside it is close enough. A unit across the map is not
        // teleported in: move it first.
        let scale = self.scale;
        let here = (
            u16::try_from(source.position.0.raw() / scale).unwrap_or(0),
            u16::try_from(source.position.1.raw() / scale).unwrap_or(0),
        );
        let there = (
            u16::try_from(shelter.position.0.raw() / scale).unwrap_or(0),
            u16::try_from(shelter.position.1.raw() / scale).unwrap_or(0),
        );
        if here.0.abs_diff(there.0) > 1 || here.1.abs_diff(there.1) > 1 {
            return Err(SimError::TooFar { id: unit });
        }
        Ok(())
    }

    fn check_research(&self, building: EntityId, tech: u32) -> Result<ResearchPlan, SimError> {
        let entity = self
            .find(building)
            .ok_or(SimError::MissingEntity(building))?;
        if entity.kind != EntityKind::Building {
            return Err(SimError::TrainNeedsABuilding { id: building });
        }
        if entity.construction.is_some() {
            return Err(SimError::BuildingNotReady { id: building });
        }
        if entity.researching.is_some() {
            return Err(SimError::AlreadyResearching { id: building });
        }
        let techs = self.techs.as_ref().ok_or(SimError::NoTables)?;
        let stats = techs.get(tech).ok_or(SimError::MissingTech { id: tech })?;
        if self.researched(entity.player).contains(&tech) {
            return Err(SimError::AlreadyResearched { tech });
        }
        // One tech, one order: two buildings researching the same tech would both charge and the
        // second would apply an upgrade with nothing left to upgrade.
        if self.entities.iter().any(|other| {
            other.player == entity.player && other.researching.is_some_and(|job| job.tech == tech)
        }) {
            return Err(SimError::TechInProgress { tech });
        }
        // `location` is the unit that researches it (12 is a barracks, 87 an archery range). Zero
        // means "anywhere", which is how the data spells a tech with no home building.
        if let Ok(place) = u32::try_from(stats.location)
            && place != 0
            && entity
                .unit
                .map(|unit| self.units.as_ref().map_or(unit, |units| units.root(unit)))
                != Some(place)
        {
            return Err(SimError::CannotResearchHere {
                building: entity.unit.unwrap_or(0),
                tech,
            });
        }
        // An age needs buildings of the current age (D65). Checked before the price so the player is told
        // what is actually missing, and counted on finished buildings: a foundation is not an age.
        if let Some((need, families, singles)) = self.age_needs.get(&tech) {
            let singles_met = !singles.is_empty()
                && self.entities.iter().any(|other| {
                    other.player == entity.player
                        && other.kind == EntityKind::Building
                        && other.hp > 0
                        && other.construction.is_none()
                        && other.unit.is_some_and(|unit| {
                            let family = self.units.as_ref().map_or(unit, |units| units.root(unit));
                            singles.contains(&family)
                        })
                });
            let have = self.standing_buildings(entity.player, families);
            if !singles_met && have < *need {
                return Err(SimError::AgeNeedsBuildings { need: *need, have });
            }
        }
        // A civ's tech tree lists what it *cannot* have, and that outranks "not ready yet": the
        // British list carries the Hussar, the Eagle line and the Paladin, while no civ's list carries
        // Loom or an age - so these are restrictions, not permissions (see crates/data/src/civs.rs).
        // Checked before the prerequisites so a barred tech says so rather than asking for an age.
        if let Some(civ) = self.barred_by(entity.player, tech) {
            return Err(SimError::TechBarred { tech, civ });
        }
        // Prerequisites: every id a tech lists is a *tech* id (all 614 entries in the table resolve to
        // techs, even where the number also names a building), and the count matters: the dat's own
        // `min_required_techs` says how many of them are enough, and tech 120 ("Shadow Node for Age
        // Two") lists five buildings while needing two (D30).
        let missing: Vec<u32> = stats
            .required
            .iter()
            .filter_map(|required| u32::try_from(*required).ok())
            .filter(|required| !self.researched(entity.player).contains(required))
            .collect();
        let have = stats.required.len().saturating_sub(missing.len());
        if have < usize::from(stats.min_required)
            && let Some(missing) = missing.first().copied()
        {
            return Err(SimError::MissingPrerequisite {
                tech,
                missing,
                role: self.tech_role_name(missing).map(str::to_owned),
            });
        }
        let effects = self.effects.as_ref().ok_or(SimError::NoTables)?;
        if let Some(effect_id) = stats.effect_id
            && effects.get(effect_id).is_none()
        {
            return Err(SimError::MissingEffect { id: effect_id });
        }
        for cost in &stats.costs {
            let slot = self.cost_slot(cost)?;
            let need = u32::from(cost.amount);
            let have = self.stock(entity.player, slot);
            if have < need {
                return Err(SimError::NotEnough {
                    resource: self.slot_name(slot),
                    need,
                    have,
                });
            }
        }
        let ticks = u32::try_from(stats.research_time)
            .ok()
            .and_then(|seconds| seconds.checked_mul(self.tick_rate_hz))
            .ok_or(SimError::MissingTech { id: tech })?;
        Ok(ResearchPlan {
            costs: stats.costs.clone(),
            ticks,
            player: entity.player,
        })
    }

    /// Pay for a tech and start its timer. Paid only after every check passed, so a refused order
    /// spends nothing.
    fn start_research(&mut self, building: EntityId, tech: u32) -> Result<(), SimError> {
        let plan = self.check_research(building, tech)?;
        for cost in &plan.costs {
            let slot = self.cost_slot(cost)?;
            let index = usize::from(plan.player).wrapping_sub(1);
            let Some(row) = self.stock.get_mut(index) else {
                return Err(SimError::PlayerOutOfRange {
                    player: plan.player,
                });
            };
            row[slot] -= u32::from(cost.amount);
        }
        let Some(entity) = self.find_mut(building) else {
            return Err(SimError::MissingEntity(building));
        };
        entity.researching = Some(Research {
            tech,
            remaining_ticks: plan.ticks,
            fresh: true,
        });
        Ok(())
    }

    /// Every modifier of `attribute` that applies to `unit`, from the techs `player` has researched.
    fn modifiers_for(
        &self,
        player: u8,
        unit: &Unit,
        attribute: &str,
    ) -> Vec<&ruage_data::Modifier> {
        let Some(effects) = self.effects.as_ref() else {
            return Vec::new();
        };
        let mut chosen = Vec::new();
        if let Some(techs) = self.techs.as_ref() {
            for tech_id in self.researched(player) {
                let Some(effect_id) = techs.get(*tech_id).and_then(|tech| tech.effect_id) else {
                    continue;
                };
                collect_modifiers(effects, effect_id, unit, attribute, &mut chosen);
            }
        }
        // A civ's team bonus applies without being researched: it is a property of who the player is.
        for effect_id in &self.bonus_effects[usize::from(player).min(MAX_PLAYERS - 1)] {
            collect_modifiers(effects, *effect_id, unit, attribute, &mut chosen);
        }
        chosen
    }

    /// A unit's attacks or armours after every modifier its owner has researched.
    ///
    /// Additions are applied before multiplications, in researched-tech order, so the result cannot
    /// depend on the order techs happened to be researched in. A modifier matches on the unit id it
    /// names, on the unit's class, or on neither (-1 means "everything").
    fn adjusted(&self, player: u8, unit: &Unit, attribute: &str) -> Vec<(i16, i16)> {
        let base = if attribute == "attack" {
            &unit.attacks
        } else {
            &unit.armours
        };
        let chosen = self.modifiers_for(player, unit, attribute);

        let mut out = base.clone();
        for modifier in chosen.iter().filter(|m| m.op == ruage_data::ModOp::Add) {
            // An attack or armour change is a change *to a class*; one that names no class is
            // content we cannot place. Skipping it is inert today (an unlisted class loses to any
            // attack), but a later rule that lets a classless entry through would turn it into
            // "+99 to everything".
            if modifier.attribute_class < 0 {
                continue;
            }
            let amount = i16::try_from(modifier.value.round() as i64).unwrap_or(i16::MAX);
            add_to_class(&mut out, modifier.attribute_class, amount);
        }
        for modifier in chosen
            .iter()
            .filter(|m| m.op == ruage_data::ModOp::Multiply)
        {
            if modifier.attribute_class < 0 {
                continue;
            }
            multiply_class(&mut out, modifier.attribute_class, modifier.value);
        }
        out
    }

    /// A unit's maximum health for this player: the table's value with every hit-point modifier
    /// applied, additions before multiplications. Health is *state*, so this is what a spawn uses and
    /// what a research grant is measured against.
    fn max_hp(&self, player: u8, unit_id: u32) -> Result<i32, SimError> {
        let units = self.units.as_ref().ok_or(SimError::NoTables)?;
        let unit = units
            .unit(unit_id)
            .ok_or(SimError::MissingUnit { id: unit_id })?;
        Ok(scaled_health(
            unit,
            &self.modifiers_for(player, unit, "hit_points"),
        ))
    }

    /// A unit's speed for this player, at the engine scale.
    fn effective_speed(&self, player: u8, unit_id: u32) -> Result<Fixed, SimError> {
        let units = self.units.as_ref().ok_or(SimError::NoTables)?;
        let unit = units
            .unit(unit_id)
            .ok_or(SimError::MissingUnit { id: unit_id })?;
        let base = unit
            .speed
            .resolve(self.scale, "speed")
            .map_err(|err| SimError::Unresolved {
                id: unit_id,
                detail: err.to_string(),
            })?;
        let modifiers = self.modifiers_for(player, unit, "speed");
        Ok(scale_fixed(base, self.scale, &modifiers))
    }

    /// Apply a researched tech: mark it done and run its effect commands.
    ///
    /// Only **kind 3** is interpreted so far - an upgrade, "every unit `a` becomes unit `b`". Every
    /// other kind is left alone on purpose and counted in `docs/next-step.md`: applying half a tech
    /// silently would be worse than applying the half that is modelled, and the tests that use this
    /// path assert the effect they use contains nothing else.
    /// A tech has landed: apply it, then apply whatever availability it unlocked.
    fn finish_research(&mut self, player: u8, tech: u32) -> Result<(), SimError> {
        let food_before = self.farm_food_bonus(player);
        self.apply_tech(player, tech)?;
        self.grant_availability(player)?;
        // Farm upgrades are the one effect that changes ground already worked: the plot's food is a
        // budget, so what the tech adds goes onto every plot this player has that still has food in
        // it. A spent plot stays spent - its food is eaten, and an upgrade does not bring it back.
        let added = self.farm_food_bonus(player) - food_before;
        if added > 0 {
            self.grow_plots(player, added);
        }
        Ok(())
    }

    /// Add `food` to every plot of this player's that still holds some, in id order so the result is
    /// a function of state. Content decides which building is a plot, exactly as it does everywhere
    /// else; a pack with no farm yields nothing here.
    fn grow_plots(&mut self, player: u8, food: i64) {
        let Some(farm_unit) = self
            .economy
            .as_ref()
            .and_then(|economy| economy.farm.unit.as_ref())
            .and_then(|role| self.economy.as_ref()?.role.get(role).copied())
        else {
            return;
        };
        let scale = self.scale;
        for entity in &mut self.entities {
            if entity.player == player
                && entity.kind == EntityKind::Building
                && entity.unit == Some(farm_unit)
                && entity.amount.raw() > 0
            {
                entity.amount = Fixed::from_raw(entity.amount.raw() + food * scale);
            }
        }
    }

    /// Mark a tech researched and apply it. Does not chase availability techs; `finish_research` is the
    /// entry point that does.
    fn apply_tech(&mut self, player: u8, tech: u32) -> Result<(), SimError> {
        // What the player's units are worth *before* this tech, so the grant is the difference the
        // tech made rather than a full heal: a wounded unit keeps its wound.
        let before: Vec<(EntityId, u32, i32)> = self
            .entities
            .iter()
            .filter(|entity| entity.player == player)
            .filter_map(|entity| {
                let unit = entity.unit?;
                Some((
                    entity.id,
                    unit,
                    self.max_hp(player, unit).unwrap_or(entity.hp),
                ))
            })
            .collect();

        self.researched.entry(player).or_default().insert(tech);

        // Health and speed are state, so every unit the player already has gains them at once.
        for (id, unit_id, old_max) in before {
            let new_max = self.max_hp(player, unit_id)?;
            let speed = self.effective_speed(player, unit_id)?;
            let grant = new_max - old_max;
            if let Some(entity) = self.find_mut(id) {
                // A site that is still going up stores the maximum it will finish at, so it has to
                // hear about the new one too - otherwise a house whose build overlapped the tech
                // would finish at the health it was quoted when the order landed.
                if let Some(mut construction) = entity.construction {
                    construction.max_hp = new_max;
                    entity.construction = Some(construction);
                }
                entity.hp = entity.hp.saturating_add(grant).clamp(1, new_max);
                entity.speed = speed;
            }
        }

        let Some(effect_id) = self
            .techs
            .as_ref()
            .and_then(|techs| techs.get(tech))
            .and_then(|stats| stats.effect_id)
        else {
            return Ok(());
        };
        let commands: Vec<(i16, i16, i16)> = self
            .effects
            .as_ref()
            .and_then(|effects| effects.get(effect_id))
            .map(|effect| {
                effect
                    .commands
                    .iter()
                    .map(|command| (command.kind, command.a, command.b))
                    .collect()
            })
            .ok_or(SimError::MissingEffect { id: effect_id })?;

        for (kind, a, b) in commands {
            if kind != 3 {
                continue;
            }
            let (Ok(from), Ok(to)) = (u32::try_from(a), u32::try_from(b)) else {
                // A negative argument is "nothing", not an id: kind 3 always names both units.
                continue;
            };
            self.upgrade_units(player, from, to)?;
        }
        Ok(())
    }

    /// Replace every unit of `player`'s that is a `from` with a `to`, keeping the fraction of health
    /// it had: a spearman at half health becomes a swordsman at half health, not a healed one.
    fn upgrade_units(&mut self, player: u8, from: u32, to: u32) -> Result<(), SimError> {
        // Both caps go through the modifier table, so an upgrade does not undo a health tech.
        let old_max = self.max_hp(player, from).unwrap_or(0);
        let new_max = self.max_hp(player, to)?;
        let speed = self.effective_speed(player, to)?;

        let ids: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|entity| entity.player == player && entity.unit == Some(from))
            .map(|entity| entity.id)
            .collect();
        for id in ids {
            let Some(entity) = self.find_mut(id) else {
                continue;
            };
            let scaled = if old_max > 0 {
                // Ceiling division: a unit that had *any* health left keeps at least one point.
                let numerator = i64::from(entity.hp) * i64::from(new_max);
                i32::try_from((numerator + i64::from(old_max) - 1) / i64::from(old_max))
                    .unwrap_or(new_max)
            } else {
                new_max
            };
            entity.hp = scaled.clamp(1, new_max);
            entity.unit = Some(to);
            entity.speed = speed;
        }
        Ok(())
    }

    /// Turn a gather order into resolved state: the variant, the per-tick rate and the capacity
    /// are read from the tables here, once, and stored on the entity. Nothing in the tick loop
    /// looks at a table, and a replay cannot name its own numbers.
    /// How many arrows a building fires at once: its own, plus one for every whole multiple of its own
    /// damage per second that its garrison contributes. The published rule is
    /// `floor(sum of garrisoned units' dps / building dps)`, and only units that deal **pierce** damage
    /// from range count - so an archer adds, and a Throwing Axeman (melee damage at range) does not (D37).
    fn volley(&self, building: EntityId) -> u32 {
        let Some(entity) = self.find(building) else {
            return 1;
        };
        let Some(units) = self.units.as_ref() else {
            return 1;
        };
        let Some(stats) = entity.unit.and_then(|unit| units.unit(unit)) else {
            return 1;
        };
        let building_dps = self.pierce_dps(entity.player, stats);
        if building_dps <= 0 {
            return 1;
        }
        let mut garrison_dps = 0i64;
        for occupant in self.entities.iter() {
            if occupant.player != entity.player || occupant.kind != EntityKind::Unit {
                continue;
            }
            if occupant.task != (Task::Garrisoned { building }) {
                continue;
            }
            let Some(unit) = occupant.unit.and_then(|unit| units.unit(unit)) else {
                continue;
            };
            // Range plus pierce damage: a unit that fights at range but deals melee damage (a Mameluke,
            // a Throwing Axeman, a Gbeto) contributes nothing.
            if unit
                .range
                .resolve(self.scale, "range")
                .map_or(0, |range| range.raw())
                <= 0
            {
                continue;
            }
            garrison_dps += self.pierce_dps(occupant.player, unit);
        }
        let extra = u32::try_from(garrison_dps / building_dps).unwrap_or(0);
        1u32.saturating_add(extra)
    }

    /// A unit's pierce damage per second, with its owner's upgrades applied: the pierce (class 3)
    /// attack over the reload, both from the data.
    ///
    /// Stated in **millionths** of a damage per second, in integers (D67). The value is only ever used as
    /// a ratio against another one of these, so a common scale cancels - but the scale has to be fine, or
    /// truncating each side separately bends the ratio: a town centre with three archers in it came out at
    /// four arrows instead of three when this was computed at a coarser precision.
    fn pierce_dps(&self, player: u8, stats: &Unit) -> i64 {
        let pierce: i32 = self
            .adjusted(player, stats, "attack")
            .iter()
            .filter(|(class, _)| *class == 3)
            .map(|(_, amount)| i32::from(*amount))
            .sum();
        let reload = stats
            .reload
            .resolve(self.scale, "reload")
            .map_or(0, |r| r.raw());
        if pierce <= 0 || reload <= 0 {
            return 0;
        }
        i64::from(pierce) * DPS_ONE / reload
    }

    /// How fast a shot flies this tick, in engine units; zero when the pack shoots instantly.
    fn projectile_speed(&self) -> i64 {
        let Some(speed) = self.combat_projectile_speed else {
            return 0;
        };
        // Exactness is the D10 invariant: a step that does not divide would drift, so a pack whose
        // speed does not divide the tick rate gets no flight rather than an accumulating arrow.
        speed
            .div_exact(i64::from(self.tick_rate_hz))
            .map_or(0, |step| step.raw())
    }

    /// Put a shot in the air. Its ticks-to-land are not stored: the arrow moves by a fixed step every
    /// tick until it reaches the position it was aimed at, which is exact for any speed that divides
    /// the tick rate and needs no accumulation.
    fn spawn_projectile(&mut self, shot: Shot) -> Result<EntityId, SimError> {
        let Shot {
            player,
            shooter,
            from,
            aim,
            target,
            damage,
            blast,
        } = shot;
        let id = EntityId(self.entities.iter().map(|e| e.id.0).max().unwrap_or(0) + 1);
        self.insert(Entity {
            id,
            player,
            kind: EntityKind::Projectile,
            unit: None,
            position: from,
            speed: Fixed::ZERO,
            velocity: (Fixed::ZERO, Fixed::ZERO),
            hp: 1,
            amount: Fixed::ZERO,
            task: Task::Fly {
                target,
                shooter,
                damage,
                aim,
                blast,
            },
            training: None,
            construction: None,
            researching: None,
            carry: Fixed::ZERO,
            variant: None,
        })?;
        Ok(id)
    }

    /// How many units this player has. Every unit is worth one: the dat has no per-unit population
    /// cost, and neither does the game.
    pub fn population(&self, player: u8) -> u32 {
        u32::try_from(
            self.entities
                .iter()
                .filter(|entity| entity.player == player && entity.kind == EntityKind::Unit)
                .count(),
        )
        .unwrap_or(u32::MAX)
    }

    /// The ceiling on this player's population: what content allows, from finished buildings only - a
    /// house supports its five when it is finished, not when it is ordered.
    ///
    /// A civilisation whose tech tree takes the house away (the Huns) is exempt from house arithmetic
    /// altogether, or it would sit at zero for the whole match. That exemption is read from the same
    /// content the bar itself comes from, so a pack with no civs simply has no exemption (D31).
    pub fn population_cap(&self, player: u8) -> u32 {
        let Some(economy) = self.economy.as_ref() else {
            return u32::MAX;
        };
        let population = &economy.population;
        let house = economy.role.get("house").copied();
        if let Some(house) = house
            && self.civ_barred_unit(player, house).is_some()
        {
            return population.cap_default;
        }
        // Sum what each finished building supports rather than counting roles: a civilisation's team
        // bonus can make a building support population that content never listed for it (the Slavs'
        // military buildings, D41).
        let total = self
            .entities
            .iter()
            .filter(|entity| {
                entity.player == player
                    && entity.kind == EntityKind::Building
                    && entity.construction.is_none()
            })
            .filter_map(|entity| entity.unit)
            .fold(0u32, |sum, built| {
                sum.saturating_add(self.population_provided(player, built))
            });
        total.min(population.cap_default)
    }

    /// Let everyone out of a building, where they stand. Shared by `Ungarrison` and by a player
    /// demolishing their own building, so the two cannot drift apart.
    fn ungarrison(&mut self, building: EntityId) {
        let leaving: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|entity| entity.task == (Task::Garrisoned { building }))
            .map(|entity| entity.id)
            .collect();
        for id in leaving {
            if let Some(entity) = self.find_mut(id) {
                entity.task = Task::Idle;
            }
        }
    }

    /// Which tiles hold a unit right now, one bit per tile. Rebuilt on the ticks where something moves and
    /// only then, because a world where nothing moves (which is most of an idle probe, and most of a quiet
    /// match) has nothing to avoid (D52).
    fn occupancy(&self) -> Vec<u64> {
        let (width, height) = self.map_size;
        let mut grid = vec![0u64; (usize::from(width) * usize::from(height)).div_ceil(64)];
        if width == 0 {
            return grid;
        }
        for entity in &self.entities {
            if entity.kind != EntityKind::Unit || matches!(entity.task, Task::Garrisoned { .. }) {
                continue;
            }
            let (x, y) = self.tile_of_raw(entity.position);
            if x < 0 || y < 0 || x >= i64::from(width) || y >= i64::from(height) {
                continue;
            }
            let bit = usize::try_from(y * i64::from(width) + x).unwrap_or(0);
            if let Some(word) = grid.get_mut(bit / 64) {
                *word |= 1u64 << (bit % 64);
            }
        }
        grid
    }

    /// Soft avoidance: if this step would end on a tile another unit is standing on, try the two quarter
    /// turns; if those are taken too, stand still for this tick.
    ///
    /// Quarter turns rather than 45-degree ones because a rotation by ninety degrees preserves the step's
    /// length exactly in fixed point, where a diagonal nudge would need a square root. The occupancy grid is
    /// read as it was at the start of the tick, so two units ordered into one tile on the same tick still
    /// both arrive - the game allows that, and blocking it would make an order to a crowded place deadlock
    /// (D52).
    fn avoid(&self, occupied: &[u64], id: EntityId, dx: Fixed, dy: Fixed) -> (Fixed, Fixed) {
        if dx == Fixed::ZERO && dy == Fixed::ZERO {
            return (dx, dy);
        }
        let Some(entity) = self.find(id) else {
            return (dx, dy);
        };
        // What is avoided is the *last step onto a place somebody already stands*, not every tile on the
        // way: a unit must be able to walk past a friend, and a villager must be able to reach the tile
        // beside a tree that somebody is already working. The first version blocked any occupied tile and
        // stranded nine gather tests and every walk past a standing unit at once (D52a).
        let Some((goal, exclusive)) = self.goal_of(entity) else {
            return (dx, dy);
        };
        if !exclusive {
            return (dx, dy);
        }
        let (tx, ty) = (
            (entity.position.0.raw() + dx.raw()).div_euclid(self.scale),
            (entity.position.1.raw() + dy.raw()).div_euclid(self.scale),
        );
        if (tx, ty) != (i64::from(goal.0), i64::from(goal.1)) {
            return (dx, dy);
        }
        let (width, height) = self.map_size;
        if tx < 0 || ty < 0 || tx >= i64::from(width) || ty >= i64::from(height) {
            return (dx, dy);
        }
        let bit = usize::try_from(ty * i64::from(width) + tx).unwrap_or(0);
        let taken = occupied
            .get(bit / 64)
            .is_some_and(|word| word & (1u64 << (bit % 64)) != 0);
        if taken {
            // Stand where it is: the ground it was sent to is somebody else's for now, and a unit that
            // shuffled sideways for ever never arrives (which is what the second version did).
            (Fixed::ZERO, Fixed::ZERO)
        } else {
            (dx, dy)
        }
    }

    /// The `n`-th tile of a crowd's destination, counting the clicked tile as the first: the tile itself,
    /// then the ring around it, in a fixed order so two runs agree. Tiles that are not walkable are skipped;
    /// a target with no room left keeps the wanted tile, and the last-step rule keeps the crowd from
    /// stacking on it.
    fn slot_for(&self, wanted: (u16, u16), n: usize) -> (u16, u16) {
        if n == 0 {
            return wanted;
        }
        let mut seen = 0usize;
        for radius in 0..=6i32 {
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs() != radius && dy.abs() != radius {
                        continue;
                    }
                    let (Some(x), Some(y)) = (
                        u16::try_from(i32::from(wanted.0) + dx).ok(),
                        u16::try_from(i32::from(wanted.1) + dy).ok(),
                    ) else {
                        continue;
                    };
                    if radius == 0 {
                        // The clicked tile is the first slot, which the caller already took.
                        continue;
                    }
                    if !self.is_free_ground(x, y) {
                        continue;
                    }
                    // A tile already claimed by an earlier unit of the same crowd is not free either.
                    let claimed = self
                        .entities
                        .iter()
                        .any(|entity| matches!(self.goal_of(entity), Some((tile, true)) if tile == (x, y)));
                    if claimed {
                        continue;
                    }
                    seen += 1;
                    if seen == n {
                        return (x, y);
                    }
                }
            }
        }
        wanted
    }

    /// The tile an entity is trying to stand on, and whether that tile is *its own* - a place it goes to
    /// be at, rather than a thing it goes to work.
    ///
    /// Only the first kind is exclusive. Two villagers must be able to work one tree and two attackers must
    /// be able to reach one target, so a goal that is a body to be worked is not avoided; a walk to a place
    /// is, because that is the stacking a player sees.
    fn goal_of(&self, entity: &Entity) -> Option<((u16, u16), bool)> {
        let (goal, exclusive) = match entity.task {
            Task::Move { to } | Task::Hunt { to } => (to, true),
            // A drop site is worked, not occupied: two villagers must be able to hand in at one town
            // centre on the same tick, and a building's own tile is never free ground anyway.
            Task::Return { drop, .. } => (self.find(drop)?.position, false),
            Task::Build { site } => (self.find(site)?.position, false),
            Task::Gather { node, .. } => (self.find(node)?.position, false),
            Task::Attack { target, .. } => (self.find(target)?.position, false),
            Task::Idle | Task::Garrisoned { .. } | Task::Fly { .. } => return None,
        };
        Some((self.tile_of(goal), exclusive))
    }

    /// Everything a blast catches: units (not the aimed target, which the landing rule already dealt
    /// with, and not one sheltered inside a building), within `radius` of the landing point. Buildings
    /// take the aimed hit only - a mangonel's stone does not demolish a wall it passed near (D51).
    fn blast_victims(&self, aim: (Fixed, Fixed), radius: Fixed, aimed: EntityId) -> Vec<EntityId> {
        if radius.raw() <= 0 {
            return Vec::new();
        }
        let mut hit = Vec::new();
        for entity in &self.entities {
            if entity.id == aimed || entity.hp <= 0 || entity.kind != EntityKind::Unit {
                continue;
            }
            if matches!(entity.task, Task::Garrisoned { .. }) {
                continue;
            }
            let dx = entity.position.0.raw() - aim.0.raw();
            let dy = entity.position.1.raw() - aim.1.raw();
            if dx * dx + dy * dy <= radius.raw() * radius.raw() {
                hit.push(entity.id);
            }
        }
        hit
    }

    /// What this player knows about a tile: never seen, remembered, or watched right now.
    pub fn vision(&self, player: u8, tile: (u16, u16)) -> TileVision {
        if player == 0 || usize::from(player) > MAX_PLAYERS {
            return TileVision::Unseen;
        }
        if !self.explored_bit(player, tile) {
            return TileVision::Unseen;
        }
        if self
            .entities
            .iter()
            .any(|entity| entity.player == player && self.covers(entity, tile))
        {
            TileVision::Visible
        } else {
            TileVision::Explored
        }
    }

    /// Whether this player can see that entity: their own always, anybody else's only while it stands
    /// on ground an entity of theirs is watching.
    pub fn visible_to(&self, player: u8, entity: EntityId) -> bool {
        let Some(entity) = self.find(entity) else {
            return false;
        };
        if entity.player == player {
            return true;
        }
        let tile = self.tile_of(entity.position);
        self.vision(player, tile) == TileVision::Visible
    }

    /// The tiles this player sees *right now*, as one bit per tile in row-major order. Built from the
    /// union of the sight discs rather than by asking `vision` per tile, which is what a renderer
    /// needs: one pass per frame, not one pass per tile.
    pub fn visible_mask(&self, player: u8) -> Vec<u64> {
        let (width, height) = self.map_size;
        let mut mask = vec![0u64; (usize::from(width) * usize::from(height)).div_ceil(64)];
        if width == 0 || player == 0 || usize::from(player) > MAX_PLAYERS {
            return mask;
        }
        for entity in self.entities.iter().filter(|e| e.player == player) {
            let radius = self.sight_of(entity);
            if radius <= 0 {
                continue;
            }
            let (x, y) = self.tile_of_raw(entity.position);
            for dy in -radius..=radius {
                let ty = y + dy;
                if ty < 0 || ty >= i64::from(height) {
                    continue;
                }
                for dx in -radius..=radius {
                    if dx * dx + dy * dy > radius * radius {
                        continue;
                    }
                    let tx = x + dx;
                    if tx < 0 || tx >= i64::from(width) {
                        continue;
                    }
                    let bit = usize::try_from(ty * i64::from(width) + tx).unwrap_or(0);
                    if let Some(word) = mask.get_mut(bit / 64) {
                        *word |= 1u64 << (bit % 64);
                    }
                }
            }
        }
        mask
    }

    /// The sight radius of an entity in whole tiles: its own unit's `los`, rounded up. Zero for an
    /// entity with no unit (a handmade mover) or one whose table entry has no sight.
    fn sight_of(&self, entity: &Entity) -> i64 {
        let Some(unit) = entity.unit else { return 0 };
        let Some(units) = self.units.as_ref() else {
            return 0;
        };
        let Some(record) = units.unit(unit) else {
            return 0;
        };
        let Ok(los) = record.los.resolve(self.scale, "los") else {
            return 0;
        };
        let raw = los.raw();
        if raw <= 0 {
            return 0;
        }
        // `div_ceil` is unsigned-only on stable, and the raw value is positive by the guard above.
        (raw + self.scale - 1) / self.scale
    }

    /// Whether this entity's sight covers `tile`.
    fn covers(&self, entity: &Entity, tile: (u16, u16)) -> bool {
        if entity.player == 0
            || matches!(
                entity.kind,
                EntityKind::Projectile | EntityKind::Resource(_)
            )
            || matches!(entity.task, Task::Garrisoned { .. })
        {
            return false;
        }
        let radius = self.sight_of(entity);
        if radius <= 0 {
            return false;
        }
        let (x, y) = self.tile_of_raw(entity.position);
        let dx = x - i64::from(tile.0);
        let dy = y - i64::from(tile.1);
        dx * dx + dy * dy <= radius * radius
    }

    /// A tile coordinate from an engine position, without the `u16` clamp `tile_of` applies.
    fn tile_of_raw(&self, position: (Fixed, Fixed)) -> (i64, i64) {
        (
            position.0.raw().div_euclid(self.scale),
            position.1.raw().div_euclid(self.scale),
        )
    }

    fn explored_bit(&self, player: u8, tile: (u16, u16)) -> bool {
        let (width, height) = self.map_size;
        if tile.0 >= width || tile.1 >= height {
            return false;
        }
        let bit = usize::from(tile.1) * usize::from(width) + usize::from(tile.0);
        self.explored[usize::from(player) - 1]
            .get(bit / 64)
            .is_some_and(|word| word & (1u64 << (bit % 64)) != 0)
    }

    /// Mark what one entity can see into its player's explored grid.
    ///
    /// Called when the entity arrives in the world and again on a tick when it actually moves - never
    /// in a pass over the whole list. Fog is monotonic (a tile, once seen, stays seen), so an entity
    /// whose position did not change has nothing to add: the first version of this stamped every disc
    /// every tick and cost 19x the whole tick at 2,126 units (D50a). A stamp is now 50 ns on a unit
    /// that moved and zero on one that did not.
    fn stamp_entity(&mut self, id: EntityId) {
        let (width, height) = self.map_size;
        if width == 0 || height == 0 {
            return;
        }
        let Some(entity) = self.find(id) else { return };
        if entity.player == 0 || usize::from(entity.player) > MAX_PLAYERS {
            return;
        }
        if matches!(
            entity.kind,
            EntityKind::Projectile | EntityKind::Resource(_)
        ) {
            return;
        }
        // Inside a building a unit is not on the map and does not see out of it; the building's own
        // sight already covers what it can watch.
        if matches!(entity.task, Task::Garrisoned { .. }) {
            return;
        }
        let radius = self.sight_of(entity);
        if radius <= 0 {
            return;
        }
        let (x, y) = self.tile_of_raw(entity.position);
        let row = &mut self.explored[usize::from(entity.player) - 1];
        for dy in -radius..=radius {
            let ty = y + dy;
            if ty < 0 || ty >= i64::from(height) {
                continue;
            }
            for dx in -radius..=radius {
                if dx * dx + dy * dy > radius * radius {
                    continue;
                }
                let tx = x + dx;
                if tx < 0 || tx >= i64::from(width) {
                    continue;
                }
                let bit = usize::try_from(ty * i64::from(width) + tx).unwrap_or(0);
                if let Some(word) = row.get_mut(bit / 64) {
                    *word |= 1u64 << (bit % 64);
                }
            }
        }
    }

    /// Stamp every entity that is already in the world. Seeding and tests that hand-place entities use
    /// this; the tick never does, because it stamps the entities that moved as it moves them.
    fn stamp_all(&mut self) {
        let ids: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|entity| entity.player > 0)
            .map(|entity| entity.id)
            .collect();
        for id in ids {
            self.stamp_entity(id);
        }
    }

    /// How many entries the per-entity side tables hold, all told: routes, rallies, queues, patrol
    /// ends, interrupted walks and stances.
    ///
    /// Nothing in the tick loop needs this; it exists because each of those tables is keyed by an
    /// entity id and the state hash walks every one of them, so "a dead entity left something behind"
    /// is a leak that is invisible in play and visible here. The stability probe samples it.
    pub fn bookkeeping_entries(&self) -> usize {
        self.paths.len()
            + self.rallies.len()
            + self.queued.len()
            + self.patrols.len()
            + self.resuming.len()
            + self.stances.len()
            + self.leash.len()
            + self.slaughter.len()
            + self.rots.len()
    }

    /// Whether any side table still remembers `id`. The removal tests use it to prove a dead entity
    /// leaves nothing behind - and the overbuild rule uses it in a test to prove a *cancelled* one does
    /// not either. Nothing in the tick loop needs it, so it costs nothing there.
    pub fn bookkeeping_touches(&self, id: EntityId) -> bool {
        self.paths.contains_key(&id)
            || self.rallies.contains_key(&id)
            || self.queued.contains_key(&id)
            || self.resuming.contains_key(&id)
            || self.patrols.contains_key(&id)
            || self.stances.contains_key(&id)
            || self.build_queue.contains_key(&id)
            || self.leash.contains_key(&id)
            || self.slaughter.contains_key(&id)
            || self.slaughter.values().any(|animal| *animal == id)
            || self.rots.contains_key(&id)
    }

    /// How many units this building is making: the one in progress plus everything queued behind it.
    /// A HUD shows it and the death sweep proves it is released; nothing in the tick loop needs it.
    pub fn pending_orders(&self, building: EntityId) -> usize {
        let head = usize::from(
            self.find(building)
                .is_some_and(|entity| entity.training.is_some()),
        );
        head + self.queued.get(&building).map_or(0, Vec::len)
    }

    /// The most health this unit could have for this player, after every research that grants it. Public
    /// because a HUD and a health bar both need it, and both are outside the simulation (D55).
    pub fn max_hp_of(&self, player: u8, unit: u32) -> Option<i32> {
        self.max_hp(player, unit).ok()
    }

    /// Whether the match has been decided, and how. `None` while it is still being played.
    pub fn outcome(&self) -> Option<MatchOutcome> {
        self.outcome
    }

    /// How many **finished, standing** buildings of `player` are in `families` (D65). Foundations and
    /// rubble do not count: an age wants buildings, not holes in the ground with a villager in them.
    fn standing_buildings(&self, player: u8, families: &BTreeSet<u32>) -> u32 {
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        for entity in &self.entities {
            if entity.player != player
                || entity.kind != EntityKind::Building
                || entity.hp <= 0
                || entity.construction.is_some()
            {
                continue;
            }
            let Some(unit) = entity.unit else { continue };
            let family = self.units.as_ref().map_or(unit, |units| units.root(unit));
            if families.contains(&family) {
                seen.insert(entity.id.0);
            }
        }
        seen.len() as u32
    }

    /// Whether any **unit** of `player` can see `tile` - a foundation's own sight does not count.
    ///
    /// The distinction matters for the blind-placement rule (D61): a building projects sight as soon as
    /// it is placed, so asking `vision()` would have a foundation see the enemy building on its own tile
    /// and cancel itself on the tick it was laid, which would make placing blind impossible. What the
    /// rule means is *your units* looking at it - the villager you sent, or anybody else who walks by.
    fn unit_sees(&self, player: u8, tile: (u16, u16)) -> bool {
        let Some(units) = self.units.as_ref() else {
            return false;
        };
        self.entities
            .iter()
            .filter(|entity| entity.player == player && entity.kind == EntityKind::Unit)
            .any(|entity| {
                let los = entity
                    .unit
                    .and_then(|unit| units.unit(units.root(unit)))
                    .and_then(|stats| stats.los.resolve(self.scale, "los").ok())
                    .unwrap_or(Fixed::ZERO);
                let here = self.tile_of(entity.position);
                let dx = i64::from(here.0.abs_diff(tile.0));
                let dy = i64::from(here.1.abs_diff(tile.1));
                let reach = los.raw() / self.scale;
                let reach = reach.max(0);
                // A disc, like the vision layer: squared distance against squared radius.
                dx * dx + dy * dy <= reach * reach
            })
    }

    /// Whether this entity keeps its owner in the match (D59).
    ///
    /// A **unit** always counts - it can walk somewhere and make something happen. A **building** counts
    /// only if it can make a unit or take a resource in, so a house, a wall, a tower or a farm does not:
    /// those are the things a player can lose everything else and still be sitting on. Family rather than
    /// id, because an upgrade replaces an id (a town centre is 109 -> 71 -> 141).
    fn keeps_them_in(&self, entity: &Entity) -> bool {
        if entity.player == 0 || entity.hp <= 0 {
            return false;
        }
        match entity.kind {
            EntityKind::Unit => true,
            EntityKind::Building => entity.unit.is_some_and(|unit| {
                self.units
                    .as_ref()
                    .and_then(|units| units.unit(units.root(unit)).or_else(|| units.unit(unit)))
                    .is_some_and(|stats| self.conquest_classes.contains(&stats.class))
            }),
            // An arrow in the air and a resource node keep nobody alive.
            EntityKind::Projectile | EntityKind::Resource(_) => false,
        }
    }

    /// Whether this player still has anything that counts towards being in the match.
    fn still_standing(&self, player: u8) -> bool {
        self.entities
            .iter()
            .any(|entity| entity.player == player && self.keeps_them_in(entity))
    }

    /// Decide the match, once, from the state this tick left behind.
    ///
    /// One pass over the buildings, called only on a tick where something died: a seat cannot
    /// disappear any other way, so the idle tick pays nothing for it.
    fn decide_outcome(&mut self) {
        if self.outcome.is_some() {
            return;
        }
        let sides: Vec<u8> = (1..=MAX_PLAYERS as u8)
            .filter(|player| self.roster[usize::from(*player) - 1])
            .collect();
        if sides.len() < 2 {
            // A one-sided world is a test fixture or a sandbox, not a match.
            return;
        }
        let standing: Vec<u8> = sides
            .iter()
            .copied()
            .filter(|player| self.still_standing(*player))
            .collect();
        self.outcome = match standing.len() {
            0 => Some(MatchOutcome::Draw),
            1 => Some(MatchOutcome::Victory {
                winner: standing[0],
            }),
            // Two or more sides still stand: an eliminated side in a three-way match leaves the other
            // two fighting.
            _ => None,
        };
    }

    /// How much room two bodies take up between them, in engine units: the sum of their radii. Used to
    /// measure a dead zone from body to body rather than from centre to centre (D44).
    fn body_radii(&self, a: Option<u32>, b: Option<u32>) -> i64 {
        let Some(units) = self.units.as_ref() else {
            return 0;
        };
        let radius = |id: Option<u32>| -> i64 {
            id.and_then(|id| units.unit(units.root(id)))
                .and_then(|unit| unit.radius.first().copied())
                .and_then(|value| value.resolve(self.scale, "radius").ok())
                .map_or(0, |value| value.raw())
        };
        radius(a) + radius(b)
    }

    /// The point a shot is really aimed at, after the archer's accuracy has had its say.
    ///
    /// Accuracy is a percentage (attribute 11): the dat gives an archer 80, a hand cannoneer 65, every
    /// building that shoots 100, and Thumb Ring sets bowmen to 100. A shot aimed true goes exactly where
    /// the archer meant; one that is not lands `accuracy_scatter` away, which the landing rule then makes
    /// a miss. The roll is derived from the tick and the shooter's id rather than drawn from a generator,
    /// so a replay makes exactly the same misses (D43).
    fn true_aim(&self, shooter: EntityId, aim: (Fixed, Fixed)) -> (Fixed, Fixed) {
        if self.accuracy_scatter <= 0 {
            return aim;
        }
        let (Some(units), Some(entity)) = (self.units.as_ref(), self.find(shooter)) else {
            return aim;
        };
        let Some(unit) = entity.unit.and_then(|unit| units.unit(units.root(unit))) else {
            return aim;
        };
        let accuracy = self.accuracy_of(entity.player, unit);
        if accuracy >= 100 {
            return aim;
        }
        let roll = splitmix64(
            self.tick()
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                .wrapping_add(u64::from(entity.id.0)),
        );
        if accuracy > 0 && roll % 100 < u64::from(accuracy) {
            return aim;
        }
        // Off the mark: one of eight directions, because the scatter is a distance and a direction needs
        // no trigonometry to be deterministic.
        let (dx, dy) = OCTANTS[usize::try_from((roll >> 8) % 8).unwrap_or(0)];
        (
            Fixed::from_raw(aim.0.raw() + dx * self.accuracy_scatter),
            Fixed::from_raw(aim.1.raw() + dy * self.accuracy_scatter),
        )
    }

    /// This player's accuracy with this unit, in percent: what the dat gives it, unless a tech sets
    /// attribute 11 to something else (Thumb Ring sets 100, City Rights 87).
    fn accuracy_of(&self, player: u8, unit: &Unit) -> u16 {
        let mut accuracy = unit.accuracy;
        for modifier in self.modifiers_for(player, unit, "accuracy") {
            if modifier.op == ruage_data::ModOp::Set {
                accuracy = u16::try_from(modifier.value.clamp(0.0, 100.0) as i64).unwrap_or(0);
            }
        }
        accuracy
    }

    /// Does this shooter lead its shots? Attribute 19 is the ballistics flag, and content sets it on the
    /// units a university's Ballistics teaches. Any positive set value counts (D42).
    fn has_ballistics(&self, shooter: EntityId) -> bool {
        let (Some(units), Some(entity)) = (self.units.as_ref(), self.find(shooter)) else {
            return false;
        };
        let Some(unit) = entity.unit.and_then(|unit| units.unit(units.root(unit))) else {
            return false;
        };
        self.modifiers_for(entity.player, unit, "ballistics")
            .iter()
            .any(|modifier| modifier.value > 0.0)
    }

    /// Where to aim so that a shot meets a moving target: iterate the time of flight, which depends on the
    /// distance to the aim, which depends on the time of flight. Three passes settle it for any speed the
    /// game has, and every step is integer arithmetic on engine units, so two runs aim at the same point.
    ///
    /// The target's motion for this tick is read off its own movement rather than stored: `velocity` is a
    /// spawn-time field, and a unit's real direction is the step it is about to take.
    fn lead_shot(
        &self,
        from: (Fixed, Fixed),
        victim: (Fixed, Fixed),
        target: EntityId,
        projectile_step: i64,
    ) -> (Fixed, Fixed) {
        let (step_x, step_y) = self.motion_of(target);
        if (step_x, step_y) == (0, 0) || projectile_step <= 0 {
            return victim;
        }
        let mut aim = victim;
        for _ in 0..3 {
            let dx = aim.0.raw() - from.0.raw();
            let dy = aim.1.raw() - from.1.raw();
            let distance = integer_sqrt(dx * dx + dy * dy);
            let ticks = distance / projectile_step;
            aim = (
                Fixed::from_raw(victim.0.raw() + step_x * ticks),
                Fixed::from_raw(victim.1.raw() + step_y * ticks),
            );
        }
        aim
    }

    /// How far this entity moves per tick, as a raw-unit delta: the step its own task would take now.
    fn motion_of(&self, id: EntityId) -> (i64, i64) {
        let Some(entity) = self.find(id) else {
            return (0, 0);
        };
        let reach = match entity.task {
            Task::Move { to } | Task::Hunt { to } => Some(to),
            Task::Attack { .. }
            | Task::Gather { .. }
            | Task::Return { .. }
            | Task::Build { .. } => self.next_waypoint(id),
            Task::Idle | Task::Garrisoned { .. } | Task::Fly { .. } => None,
        };
        let Some(reach) = reach else {
            return (0, 0);
        };
        let Ok(delta) = entity.speed.div_exact(i64::from(self.tick_rate_hz)) else {
            return (0, 0);
        };
        step_towards(entity.position, reach, delta.raw())
    }

    /// How much population one finished building supports: what content gives its role, unless a
    /// modifier *sets* it to something else.
    ///
    /// Population is attribute 21 and arrives as a kind-0 `set`, which is a different shape from the
    /// additive attack and armour modifiers: "set" means the building supports this many units, not this
    /// many more. The vertical is what makes the Slavs' team bonus work (military buildings support five)
    /// and the Chinese town centre (ten) without either being written into content by hand (D41).
    fn population_provided(&self, player: u8, built: u32) -> u32 {
        let Some(economy) = self.economy.as_ref() else {
            return 0;
        };
        let units = self.units.as_ref();
        let root = units.map_or(built, |units| units.root(built));
        let population = &economy.population;
        let base = if economy.role.get("house") == Some(&root) {
            population.house_population
        } else if economy.role.get("town_centre") == Some(&root) {
            population.town_centre_population
        } else {
            0
        };
        let Some(stats) = units.and_then(|units| units.unit(root)) else {
            return base;
        };
        let mut provided = base;
        for modifier in self.modifiers_for(player, stats, "population") {
            if modifier.op == ruage_data::ModOp::Set {
                provided = u32::try_from(modifier.value.max(0.0) as i64).unwrap_or(0);
            }
        }
        provided
    }

    /// How much food a new farm plot of this player's is worth: what content gives a farm, plus what
    /// the player's techs have added to the dat's farm food resource.
    fn farm_food_total(&self, player: u8) -> i64 {
        let base = self
            .economy
            .as_ref()
            .map_or(0, |economy| i64::from(economy.farm.food_per_farm));
        base + self.farm_food_bonus(player)
    }

    /// The farm food a player's researched techs have added, in whole food.
    fn farm_food_bonus(&self, player: u8) -> i64 {
        let (Some(economy), Some(effects), Some(techs)) = (
            self.economy.as_ref(),
            self.effects.as_ref(),
            self.techs.as_ref(),
        ) else {
            return 0;
        };
        let Some(resource) = economy.farm.food_resource else {
            return 0;
        };
        let Some(researched) = self.researched.get(&player) else {
            return 0;
        };
        researched
            .iter()
            .filter_map(|tech| techs.get(*tech))
            .filter_map(|tech| tech.effect_id)
            .filter_map(|effect| effects.get(effect))
            .flat_map(|effect| &effect.resource_modifiers)
            .filter(|modifier| modifier.resource == resource && modifier.mode == 1)
            .map(|modifier| modifier.value.round() as i64)
            .sum()
    }

    /// Where a worker stands to work a building: the first neighbouring tile that is both free and
    /// reachable from `from`, in a fixed scan order, so the choice is a function of the map rather
    /// than of iteration order. `None` when there is none, and the caller keeps its old behaviour of
    /// walking at the building itself.
    fn work_position(
        &self,
        from: (Fixed, Fixed),
        target: (Fixed, Fixed),
    ) -> Option<(Fixed, Fixed)> {
        let scale = self.scale;
        let x = u16::try_from(target.0.raw() / scale).ok()?;
        let y = u16::try_from(target.1.raw() / scale).ok()?;
        const RING: [(i32, i32); 8] = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];
        RING.iter().find_map(|(dx, dy)| {
            let tile = (
                u16::try_from(i32::from(x) + dx).ok()?,
                u16::try_from(i32::from(y) + dy).ok()?,
            );
            if !self.is_free_ground(tile.0, tile.1) {
                return None;
            }
            let position = self.tile(tile);
            // Free is not the same as reachable: a free tile inside a wall is no place to stand.
            self.plan_path(from, position).ok().map(|_| position)
        })
    }

    /// Walk a unit towards a place, stopping as close to it as the map allows: the place itself first,
    /// then the nearest tile on each ring around it that a route exists to. A rally point inside a walled
    /// pocket is therefore an instruction to stand at the pocket's edge, not to stay home (D40).
    fn walk_towards(&mut self, unit: EntityId, to: (Fixed, Fixed)) {
        let Some(from) = self.find(unit).map(|entity| entity.position) else {
            return;
        };
        let mut target = None;
        if let Ok(route) = self.plan_path(from, to) {
            self.paths.insert(unit, route);
            target = Some(to);
        } else {
            let scale = self.scale;
            let x = i32::from(u16::try_from(to.0.raw() / scale).unwrap_or(0));
            let y = i32::from(u16::try_from(to.1.raw() / scale).unwrap_or(0));
            'rings: for radius in 1..=10i32 {
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        // Only the ring itself: the interior has already been tried.
                        if dx.abs() != radius && dy.abs() != radius {
                            continue;
                        }
                        let (Some(cx), Some(cy)) =
                            (u16::try_from(x + dx).ok(), u16::try_from(y + dy).ok())
                        else {
                            continue;
                        };
                        if !self.is_free_ground(cx, cy) {
                            continue;
                        }
                        let candidate = self.tile((cx, cy));
                        if let Ok(route) = self.plan_path(from, candidate) {
                            self.paths.insert(unit, route);
                            target = Some(candidate);
                            break 'rings;
                        }
                    }
                }
            }
        }
        if let Some(target) = target
            && let Some(entity) = self.find_mut(unit)
        {
            entity.task = Task::Move { to: target };
        }
    }

    /// Is this tile on the map at all? Free ground is the stricter question, and a rally point may
    /// legitimately be somewhere with a tree on it.
    fn on_map(&self, x: u16, y: u16) -> bool {
        self.walk.as_ref().is_some_and(|walk| walk.contains(x, y))
    }

    /// Can this node be worked at all? A node is worked by standing **on** it (a tree) or **beside** it (a mine, a farm),
    /// so it is workable when its own tile is free ground or one of the eight tiles around it is.
    ///
    /// This is a **tile** test, not a path search - which is the whole point: the middle of a forest block is not
    /// unreachable because a route exists, it is unreachable because there is nowhere to stand. Choosing by distance alone
    /// put villagers on interior trees, where `start_gather` refused with *"the target is walled off"* (measured at 703 s
    /// of the acceptance match) and the wood column churned through failures for the rest of the run.
    #[must_use]
    pub fn node_workable(&self, id: EntityId) -> bool {
        let Some(position) = self.position(id) else {
            return false;
        };
        self.work_position(position, position).is_some()
    }

    /// Remove every emptied node that nobody is working any more, and free its tile.
    ///
    /// A node lingers for a tick or two after its last unit, so the villager that emptied it can still be told where to go
    /// next (`nearest_like` needs the old node's *kind*), and the ground opens only once the village has moved on.
    fn step_spent_nodes(&mut self) {
        if self.spent_nodes.is_empty() || !self.tick.is_multiple_of(4) {
            return;
        }
        let busy: BTreeSet<EntityId> = self
            .entities
            .iter()
            .filter_map(|entity| match entity.task {
                Task::Gather { node, .. } | Task::Return { node, .. } => Some(node),
                _ => None,
            })
            .collect();
        let done: Vec<EntityId> = self
            .spent_nodes
            .iter()
            .copied()
            .filter(|node| !busy.contains(node))
            .collect();
        for node in done {
            if let Some(entity) = self.find(node)
                && matches!(entity.kind, EntityKind::Resource(_))
            {
                let tile = self.tile_of(entity.position);
                if let Some(walk) = self.walk.as_mut() {
                    walk.set_blocked(tile.0, tile.1, false);
                }
            }
            self.entities.retain(|entity| entity.id != node);
            self.spent_nodes.remove(&node);
        }
    }

    /// The actor of the command that just failed, for the refusal record. A command is not always about one entity, so a
    /// placeholder is used when the world cannot name one.
    fn entity_of_last_command(&self) -> EntityId {
        EntityId(0)
    }

    /// A farm whose food is gone. It **stays on the map** - walkable, worth nothing - until a new farm replaces it, and the
    /// replacement costs the full 60 wood (the owner's rule, 2026-10-30). Content names the farm role; the engine only
    /// knows that a building with nothing left in it and no construction in progress is not in the way.
    #[must_use]
    pub fn is_expired_farm(&self, id: EntityId) -> bool {
        let Some(entity) = self.find(id) else {
            return false;
        };
        if !self.is_farm_role(entity) {
            return false;
        }
        entity.construction.is_none() && entity.amount.raw() <= 0
    }

    /// Is this entity a farm, by the role content calls `[farm] unit`? By family, so an upgraded farm counts.
    fn is_farm_role(&self, entity: &Entity) -> bool {
        if entity.kind != EntityKind::Building {
            return false;
        }
        let Some(unit) = entity.unit else {
            return false;
        };
        let Some(economy) = self.economy.as_ref() else {
            return false;
        };
        let Some(role) = economy.farm.unit.as_deref() else {
            return false;
        };
        economy
            .role
            .get(role)
            .is_some_and(|farm| self.is_family(unit, *farm))
    }

    /// Move any unit standing on a tile that has just been built on to the nearest free tile beside it, and forget its
    /// route: the game pushes units out of the way of a new building, and a player who drops a house on a villager means
    /// the house, not the villager.
    fn push_units_off(&mut self, at: (u16, u16)) {
        let position = self.tile(at);
        let movers: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|entity| {
                entity.kind == EntityKind::Unit && entity.position == position && entity.player != 0
            })
            .map(|entity| entity.id)
            .collect();
        for id in movers {
            let Some(spot) = self.work_position(position, position) else {
                continue;
            };
            if let Some(entity) = self.find_mut(id) {
                entity.position = spot;
            }
            self.paths.remove(&id);
        }
    }

    /// How many waypoints are left on this entity's route: 0 means it is standing on its goal (or has none).
    #[must_use]
    pub fn path_length(&self, id: EntityId) -> usize {
        self.paths.get(&id).map_or(0, Vec::len)
    }

    /// Whether a unit may stand on this tile: terrain, walls and buildings, as the walk grid holds them. Exposed because
    /// "can this tile be walked on" is a question a test (and a future UI) asks of the world directly.
    #[must_use]
    pub fn tile_walkable(&self, x: u16, y: u16) -> bool {
        self.walk.as_ref().is_some_and(|walk| !walk.blocked(x, y))
    }

    /// Whether an entity is a **living animal**, and if it is, whether it is herdable. `None` for a tree, a building, a
    /// carcass or a dead animal: only a live animal is something a villager hunts or herds.
    fn animal_at(&self, id: EntityId) -> Option<bool> {
        let entity = self.find(id)?;
        if entity.kind != EntityKind::Unit || entity.hp <= 0 {
            return None;
        }
        let unit = entity.unit?;
        if !self.animals.contains(&unit) {
            return None;
        }
        Some(self.herdable.contains(&unit))
    }

    /// The nearest **other** node of the same kind: the same resource column for a tree or a mine, the same family for
    /// an animal (the next sheep, not a boar), the same role for a farm with food left in it. Reachability is part of
    /// the choice, because a node behind a wall is not somewhere a worker can be sent (item 9's rule).
    fn nearest_like(
        &self,
        from: (Fixed, Fixed),
        old: EntityId,
        worker: EntityId,
    ) -> Option<EntityId> {
        let source = self.find(old)?;
        let kind = source.kind;
        let unit = source.unit;
        let family = unit.and_then(|unit| self.units.as_ref().map(|units| units.root(unit)));
        // A farm is a **building** that fills the same column a bush does, so "the next source of food" has to cross the
        // two kinds. Without this a forager whose berries are gone has nowhere to go: eighteen of them stood on the empty
        // berry patch at 2,400 s and the match never spent another stick of wood.
        let farm_slot = self
            .economy
            .as_ref()
            .and_then(|economy| economy.farm.resource.as_deref())
            .and_then(|name| self.slot_of(name));
        let mut best: Option<(i64, u32, EntityId)> = None;
        for entity in &self.entities {
            if entity.id == old || entity.id == worker {
                continue;
            }
            if matches!(entity.kind, EntityKind::Resource(_)) && !self.node_workable(entity.id) {
                continue;
            }
            let same = match (kind, entity.kind) {
                (EntityKind::Resource(slot), EntityKind::Resource(other)) => slot == other,
                // ... and a finished farm, for the column a farm fills.
                (EntityKind::Resource(slot), EntityKind::Building) => {
                    farm_slot == Some(slot)
                        && entity.construction.is_none()
                        && entity.amount.raw() > 0
                }
                (EntityKind::Unit, EntityKind::Unit) => {
                    // Another animal of the same family, and still alive: a carcass is not something to hunt.
                    entity.hp > 0
                        && unit.is_some_and(|unit| {
                            entity.unit.is_some_and(|other| {
                                other == unit
                                    || family.is_some_and(|family| {
                                        self.units
                                            .as_ref()
                                            .is_some_and(|units| units.root(other) == family)
                                    })
                            })
                        })
                }
                (EntityKind::Building, EntityKind::Building) => {
                    // Another finished farm with food left in it.
                    entity.construction.is_none()
                        && entity.amount.raw() > 0
                        && unit.is_some_and(|unit| {
                            entity.unit.is_some_and(|other| self.is_family(other, unit))
                        })
                }
                _ => false,
            };
            // Reachability is part of the choice, and a node's own tile is usually **blocked** (a tree, a mine, a
            // farm), so the question is whether a worker can reach somewhere it can *work* it - the same tile
            // `work_position` picks when the gather order is given. Asking `can_reach(node.position)` refused every
            // tree, which is how the first version sent every lumberjack to Idle.
            let reachable = self.can_reach(from, entity.position)
                || self
                    .work_position(from, entity.position)
                    .is_some_and(|stand| self.can_reach(from, stand));
            if !same || !reachable {
                continue;
            }
            let dx = entity.position.0.raw() - from.0.raw();
            let dy = entity.position.1.raw() - from.1.raw();
            let key = (dx * dx + dy * dy, entity.id.0, entity.id);
            if best
                .is_none_or(|(best_distance, best_id, _)| (key.0, key.1) < (best_distance, best_id))
            {
                best = Some(key);
            }
        }
        best.map(|(_, _, id)| id)
    }

    /// What may be gathered from an entity, and whether the thing being worked sets the pace: a farm
    /// does (its own work rate), a tree does not (the villager's variant does).
    ///
    /// The farm is content's idea (D27). The engine knows two kinds of gatherable thing only because
    /// `[farm]` names a building and the resource it yields.
    fn gather_source(&self, node: EntityId) -> Result<(String, Option<MaybeFixed>), SimError> {
        let entity = self
            .find(node)
            .cloned()
            .ok_or(SimError::MissingEntity(node))?;
        if let EntityKind::Resource(slot) = entity.kind {
            let name = self
                .resources
                .as_ref()
                .and_then(|resources| resources.name(slot))
                .ok_or_else(|| SimError::NoVariant {
                    task: format!("slot {slot}"),
                    detail: "the content pack names no resource in this column".to_owned(),
                })?;
            return Ok((name.to_owned(), None));
        }
        let economy = self.economy.as_ref().ok_or(SimError::NoTables)?;
        // A live animal is a source of food, and its meat is what it carries (D70). Content names which
        // units are animals, so the engine never learns what a sheep is. A body is not a source: only a
        // living animal yields, and what it leaves is the carcass rule's business (level 3 of this slice).
        if entity.kind == EntityKind::Unit
            && let Some(unit) = entity.unit
            && self.units.as_ref().is_some_and(|units| {
                let root = units.root(unit);
                // Alive, it is an animal; dead, it is a body - and a body is gathered from with no health at
                // all, which is why the carcass list is content rather than an inference (D70).
                self.animals.contains(&root) && entity.hp > 0 || self.carcasses.contains(&root)
            })
        {
            let resource = economy
                .hunt
                .resource
                .clone()
                .ok_or_else(|| SimError::NoVariant {
                    task: "hunt".to_owned(),
                    detail: "[hunt] names no resource for an animal to yield".to_owned(),
                })?;
            return Ok((resource, None));
        }
        let farm_unit = economy
            .farm
            .unit
            .as_ref()
            .and_then(|role| economy.role.get(role).copied());
        if farm_unit.is_none() || entity.kind != EntityKind::Building || entity.unit != farm_unit {
            return Err(SimError::NotAResource { id: node });
        }
        if entity.construction.is_some() {
            // A plot holds its food from the moment it is marked out, but nothing comes out of it
            // until it is finished.
            return Err(SimError::BuildingNotReady { id: node });
        }
        let resource = economy
            .farm
            .resource
            .clone()
            .ok_or_else(|| SimError::NoVariant {
                task: "farm".to_owned(),
                detail: "[farm] names no resource for the farm to yield".to_owned(),
            })?;
        // `use_villager_rate` picks who sets the pace: the building's own work rate, or the
        // variant's. It is a data question, so content holds the answer.
        let rate = (!economy.farm.use_villager_rate).then_some(economy.farm.rate);
        Ok((resource, rate))
    }

    fn start_gather(&mut self, villager: EntityId, node: EntityId) -> Result<(), SimError> {
        // A deliberate order supersedes a share reassignment that was waiting on a deposit: without this, a villager told
        // to do something else would still be yanked to the column it was pulled towards, once its load went in.
        self.pending_gather.remove(&villager);
        // Whose techs apply: the villager's owner's (a rate modifier is the worker's, not the node's).
        let worker_player = self
            .find(villager)
            .map(|entity| entity.player)
            .ok_or(SimError::MissingEntity(villager))?;
        let (variant_id, rate_per_tick, carry_cap) = {
            let units = self.units.as_ref().ok_or(SimError::NoTables)?;
            let economy = self.economy.as_ref().ok_or(SimError::NoTables)?;

            let (resource, farm_rate) = self.gather_source(node)?;
            // The load has a column to land in, and that is checked here rather than discovered at
            // the deposit: an order naming a resource the stockpile has no column for is refused now.
            self.slot_of(&resource)
                .ok_or_else(|| SimError::UnknownResourceName {
                    name: resource.clone(),
                })?;
            // Which task the node presents. An **animal is hunted** (the hunter's bow: VMHUN carries `range = 3.0`)
            // and a **herdable is herded** (the shepherd's knife), which is the owner's rule and is why the pack names
            // both tasks. Everything else comes from content's resource -> task map, and a pack that names no hunt or
            // herd task falls back to that map rather than refusing the order.
            let animal_task = match self.find(node).map(|entity| (entity.kind, entity.unit)) {
                Some((EntityKind::Unit, Some(unit))) if self.herdable.contains(&unit) => {
                    Some("herd")
                }
                Some((EntityKind::Unit, Some(unit))) if self.animals.contains(&unit) => {
                    Some("hunt")
                }
                _ => None,
            };
            let task_name = animal_task
                .filter(|task| economy.gather.variant.contains_key(*task))
                .map(str::to_owned)
                .or_else(|| economy.gather.resource_task.get(&resource).cloned())
                .ok_or_else(|| SimError::NoVariant {
                    task: resource.clone(),
                    detail: "no [gather.resource_task] entry for this resource".to_owned(),
                })?;
            let variant_id =
                *economy
                    .gather
                    .variant
                    .get(&task_name)
                    .ok_or_else(|| SimError::NoVariant {
                        task: task_name.clone(),
                        detail: "no such task in [gather.variant]".to_owned(),
                    })?;
            let variant = units.unit(variant_id).ok_or_else(|| SimError::NoVariant {
                task: task_name.clone(),
                detail: format!("unit {variant_id} is not in the unit table"),
            })?;
            if variant.carry == 0 {
                return Err(SimError::NoVariant {
                    task: task_name.clone(),
                    detail: "a variant that cannot carry anything cannot gather".to_owned(),
                });
            }
            let rate = match farm_rate {
                Some(rate) => {
                    rate.resolve(self.scale, "farm.rate")
                        .map_err(|err| SimError::NoVariant {
                            task: "farm".to_owned(),
                            detail: err.to_string(),
                        })?
                }
                None => variant
                    .work_rate
                    .resolve(self.scale, "work_rate")
                    .map_err(|err| SimError::NoVariant {
                        task: task_name.clone(),
                        detail: err.to_string(),
                    })?,
            };
            // Gather rate and carry capacity are resolved *here*, when the order lands, from the
            // variant and the owner's modifiers: a load already in hand keeps the numbers it started
            // with, which is what "the resources have to be gathered first" means.
            let rate = scale_fixed(
                rate,
                self.scale,
                &self.modifiers_for(worker_player, variant, "gather_rate"),
            );
            // The carry cap is content arithmetic on a whole number, so it goes through the same
            // fixed-point helper as speed and gather rate rather than through `apply_scalar` (D67).
            let carry = scale_fixed(
                Fixed::from_raw(i64::from(variant.carry) * self.scale),
                self.scale,
                &self.modifiers_for(worker_player, variant, "carry"),
            );
            // Rounded, not truncated: 10 x 1.25 is 12.5 and the game's Wheelbarrow gives 13. A
            // truncating division here would quietly take a villager's load down by one.
            let carry_cap = u32::try_from((carry.raw() + self.scale / 2) / self.scale)
                .unwrap_or(u32::from(variant.carry));
            // Exactness is the D10 invariant: a rate that does not split into whole per-tick
            // amounts would drift, so it is an error rather than a rounded accumulation.
            let rate_per_tick = rate
                .div_exact(i64::from(self.tick_rate_hz))
                .map_err(|err| SimError::NoVariant {
                    task: task_name.clone(),
                    detail: err.to_string(),
                })?;
            (variant_id, rate_per_tick, carry_cap)
        };

        let from = self
            .find(villager)
            .map(|entity| entity.position)
            .ok_or(SimError::MissingEntity(villager))?;
        let to = self
            .find(node)
            .map(|entity| entity.position)
            .ok_or(SimError::MissingEntity(node))?;
        // A building's own tile is blocked, so a worker stands *beside* it. Planning to the centre
        // would walk the villager into the building, and the walk home would then start from a
        // blocked tile and never find a route.
        let route = if self.find(node).map(|entity| entity.kind) == Some(EntityKind::Building) {
            match self.work_position(from, to) {
                Some(stand) => self.plan_path(from, stand)?,
                None => self.plan_path(from, to)?,
            }
        } else {
            self.plan_path(from, to)?
        };
        self.paths.insert(villager, route);

        let worker = self
            .find_mut(villager)
            .ok_or(SimError::MissingEntity(villager))?;
        worker.variant = Some(variant_id);
        worker.task = Task::Gather {
            node,
            rate_per_tick,
            carry_cap,
            progress: Fixed::ZERO,
        };
        // The load is the villager's, not the node's: re-tasking onto the next tree (or the next sheep) must not spill
        // what is already in its hands. It used to be zeroed here, which was invisible until a worker could move on by
        // itself.
        Ok(())
    }

    fn apply(&mut self, command: Command) -> Result<(), SimError> {
        if self.outcome.is_some() {
            return Err(SimError::MatchOver);
        }
        match command {
            Command::Spawn {
                id,
                player,
                kind,
                unit,
                position,
                speed,
                hp,
                amount,
            } => {
                // Spawn is the only command that names a player directly, so the bound is enforced
                // here too: an entity for player 9 would otherwise sit in the world until something
                // tried to hand it a stockpile, and then panic on the array.
                // Player 0 is neutral - resource nodes live there and it has no stockpile - so the
                // bound is only the ceiling. The movement hook in `spawn_now` uses player 0.
                if usize::from(player) > MAX_PLAYERS {
                    return Err(SimError::PlayerOutOfRange { player });
                }
                if kind == EntityKind::Unit
                    && unit.is_some_and(|unit| {
                        self.herdable.contains(&unit)
                            || self
                                .units
                                .as_ref()
                                .is_some_and(|units| self.herdable.contains(&units.root(unit)))
                    })
                {
                    self.herdable_ids.insert(id);
                }
                self.insert(Entity {
                    id,
                    player,
                    kind,
                    unit,
                    position,
                    speed,
                    // A thing that is placed is not a thing that is walking. This was `(speed, ZERO)`: a
                    // spawned body drifted east at its own speed for ever with `Task::Idle`, taking one
                    // step of 0.025 tile per tick - which is what the "creep" was. Every other way into
                    // the world starts at rest, and this one now does too (D64, D70).
                    velocity: (Fixed::ZERO, Fixed::ZERO),
                    hp,
                    // An animal placed with no amount is worth what the pack says it is worth: a sheep
                    // carries its 100 food, not zero (D70).
                    amount: Fixed::from_raw(
                        i64::from(if amount == 0 {
                            self.meat_of(unit)
                        } else {
                            amount
                        }) * self.scale,
                    ),
                    task: Task::Idle,
                    training: None,
                    construction: None,
                    researching: None,
                    carry: Fixed::ZERO,
                    variant: None,
                })
            }
            Command::Gather { villager, node } => self.start_gather(villager, node),
            Command::Build { villager, unit, at } => self.start_build(villager, unit, at),
            Command::Train { building, unit } => self.start_train(building, unit),
            Command::Research { building, tech } => self.start_research(building, tech),
            Command::Attack { attacker, target } => self.start_attack(attacker, target),
            Command::Rally { building, point } => {
                match point {
                    None => {
                        self.rallies.remove(&building);
                    }
                    Some(tile) => {
                        // A rally on the building's own tile means "stay inside". It is the same tile
                        // the player would click on the building, so no second command is needed.
                        let inside = self
                            .find(building)
                            .map(|entity| self.tile_of(entity.position) == tile)
                            .unwrap_or(false);
                        let rally = if inside {
                            RallyPoint::Inside
                        } else {
                            RallyPoint::Tile(tile.0, tile.1)
                        };
                        self.rallies.insert(building, rally);
                    }
                }
                Ok(())
            }
            Command::Garrison { unit, building } => {
                // Inside, the unit keeps its position and its load: it is sheltered, not teleported.
                if let Some(entity) = self.find_mut(unit) {
                    entity.task = Task::Garrisoned { building };
                }
                self.paths.remove(&unit);
                Ok(())
            }
            Command::Ungarrison { building } => {
                self.ungarrison(building);
                Ok(())
            }
            Command::Stop { unit } => {
                self.paths.remove(&unit);
                self.patrols.remove(&unit);
                self.resuming.remove(&unit);
                self.stand_down(unit);
                Ok(())
            }
            Command::Stance { unit, stance } => {
                let Some(post) = self.find(unit).map(|entity| entity.position) else {
                    return Err(SimError::MissingEntity(unit));
                };
                if stance == Stance::Aggressive {
                    // The default is the absence of an entry, so an aggressive unit costs nothing.
                    self.stances.remove(&unit);
                } else {
                    self.stances.insert(unit, (stance, post));
                }
                // A unit told to stop fighting drops the fight it is in; a unit told to stand its
                // ground keeps the fight, because it may still be in reach.
                if stance == Stance::NoAttack
                    && matches!(
                        self.find(unit).map(|entity| entity.task),
                        Some(Task::Attack { .. })
                    )
                {
                    self.stand_down(unit);
                }
                Ok(())
            }
            Command::Move { unit, to } => self.start_walk(unit, to, false, false),
            Command::AttackMove { unit, to } => self.start_walk(unit, to, true, false),
            Command::Patrol { unit, to } => self.start_walk(unit, to, true, true),
            Command::Delete { entity, .. } => {
                // A deletion is a death and takes the death path: health to zero, and the end of the
                // tick does the rest - the route goes, the population frees. Removing the entity
                // here would run that path from a place where the entity list is supposed to be
                // fixed for the length of the tick.
                //
                // One deliberate difference from being razed by an enemy: whoever is inside comes
                // out alive. Demolishing your own town centre is how a player pulls villagers out of
                // it, and losing them would make the command a trap; an enemy's siege still kills
                // the garrison, which is what the death sweep does on its own (D46a).
                let is_building = self
                    .find(entity)
                    .is_some_and(|target| target.kind == EntityKind::Building);
                // Demolishing work in progress gives back the part you have not built (D60). Verified
                // against the game: an untouched foundation refunds in full, a half-built building
                // refunds about half, a finished one refunds nothing - you lose what you invested.
                let part = self.find(entity).and_then(|target| {
                    target
                        .construction
                        .map(|construction| (target.player, target.unit, construction))
                });
                if let Some((player, Some(unit), construction)) = part {
                    let total = construction.total_ticks;
                    if total > 0 {
                        let left = total.saturating_sub(construction.progress_ticks.min(total));
                        self.refund_unit_cost_part(player, unit, left, total);
                    }
                }
                if let Some(target) = self.find_mut(entity) {
                    target.hp = 0;
                }
                if is_building {
                    self.ungarrison(entity);
                }
                self.paths.remove(&entity);
                Ok(())
            }
        }
    }

    /// Advance exactly one tick: apply queued commands, integrate, increment the counter.
    ///
    /// Movement is integrated in id order with an exact per-tick delta, so a unit's position
    /// after N ticks is `N * speed / tick_rate` with no accumulated error.
    pub fn step(&mut self) -> Result<(), SimError> {
        let queued = std::mem::take(&mut self.queue);
        for command in queued {
            // A command that became impossible between being accepted and landing is **refused, not fatal**: the tile
            // somebody else took this tick, the tree that fell, the price that went up. The match must not end because two
            // orders in the same tick wanted the same square - which is exactly how the acceptance run died at tick 16,008
            // (`something already stands at (18, 17)`, a market and a blacksmith both choosing it).
            if let Err(why) = self.apply(command) {
                self.refusal = Some((self.entity_of_last_command(), why, self.tick));
            }
        }

        let tick_rate = i64::from(self.tick_rate_hz);

        // Decide this tick's work in id order, then apply it. Deciding first keeps the second
        // pass free of lookups, so an entity cannot be moved *and* read from in the same tick in
        // an order-dependent way.
        // (index, id, dx, dy): the index is the fast path and the id is the check. `insert` keeps
        // the list sorted by id, so anything spawned later in the same tick shifts every index after
        // it - the id is what makes that detectable instead of silently moving the wrong entities.
        let mut deltas: Vec<(usize, EntityId, Fixed, Fixed)> =
            Vec::with_capacity(self.entities.len());
        let mut takes: Vec<(EntityId, i64, Fixed)> = Vec::new();
        // Workers chopping a standing tree, and the hit points they take off it this tick.
        let mut chops: Vec<EntityId> = Vec::new();
        let mut finished: Vec<EntityId> = Vec::new();
        let mut switches: Vec<(EntityId, Task)> = Vec::new();
        // A villager working a herdable animal, and the animal: herded after the tick's own movement.
        let mut herds: Vec<(EntityId, EntityId)> = Vec::new();
        let mut deposits: Vec<EntityId> = Vec::new();
        let mut builders: Vec<(EntityId, EntityId)> = Vec::new();
        let mut strikes: Vec<(EntityId, EntityId)> = Vec::new();
        // Workers whose node ran dry and who have somewhere else to go, decided in the pass and started after it.
        let mut retasks: Vec<(EntityId, EntityId)> = Vec::new();
        // (hunter, animal): a huntable animal in front of a villager, which is an attack rather than a take.
        let mut hunts: Vec<(EntityId, EntityId)> = Vec::new();
        // Who hit whom this tick, in order: the last blow decides the meat, and the blow that kills is always one
        // of this tick's (a victim at or below zero health is promoted before the tick ends). Nothing is carried
        // between ticks, nothing is hashed, and it is empty in any world where nobody is fighting (item 2).
        let mut blows: Vec<(EntityId, EntityId)> = Vec::new();
        // Arrows that landed this tick. They are entities, so they leave the list rather than being
        // turned idle: an idle arrow would sit in the world forever.
        let mut landed_arrows: Vec<EntityId> = Vec::new();
        // (target, damage, where the arrow was aimed): the aim point decides whether the shot is a hit
        // or a miss, so it has to survive the flight (D42).
        let mut landings: Vec<Landing> = Vec::new();
        let mut shots: Vec<Shot> = Vec::new();
        // Whoever was hit this tick. A building among them may have crossed the eject threshold, and
        // that is where its garrison comes out (D37).
        let mut ejected: Vec<EntityId> = Vec::new();
        // (hunter, target): a hunt that found somebody this tick. Started after the pass, because
        // `start_attack` plans a route and the pass holds the entity list still.
        let mut engaging: Vec<(EntityId, EntityId)> = Vec::new();
        // Patrols that reached their far end and have to turn round.
        let mut turning: Vec<EntityId> = Vec::new();
        // Defensive units whose fight left their ground: they go back to the post.
        let mut homing: Vec<EntityId> = Vec::new();
        let projectile_step = self.projectile_step;

        // Indexed, not id-collected: resolving each id afterwards is a linear scan per entity,
        // which makes the whole tick O(n^2). The entity list does not change during this loop, so
        // an index stays valid and the second pass touches each entity once.
        for (index, entity) in self.entities.iter().enumerate() {
            let (dx, dy) = match entity.task {
                Task::Attack {
                    target,
                    range,
                    min_range,
                    ..
                } => {
                    let Some((victim, victim_unit)) = self
                        .find(target)
                        // A tree cannot bleed, and a **body** cannot be fought: a carcass keeps the entity's
                        // id, so "the target is gone" stopped being how a fight with an animal ends (item 2).
                        // Without this an army swings at a corpse for the rest of the match.
                        .filter(|v| !matches!(v.kind, EntityKind::Resource(_)) && v.hp > 0)
                        .map(|v| (v.position, v.unit))
                    else {
                        // The target is gone - or is now a body. The attacker stands down on the next tick.
                        finished.push(entity.id);
                        continue;
                    };
                    // A melee attacker strikes from the tile next to its target; anything with reach
                    // strikes from as far as that reach goes, and walks until it is inside it. Both
                    // are measured on the same squared distance, in engine units, so the rule is one
                    // comparison and no square root.
                    let here = self.tile_of(entity.position);
                    let there = self.tile_of(victim);
                    let adjacent = here.0.abs_diff(there.0) <= 1 && here.1.abs_diff(there.1) <= 1;
                    let dx = victim.0.raw() - entity.position.0.raw();
                    let dy = victim.1.raw() - entity.position.1.raw();
                    let squared = dx * dx + dy * dy;
                    let in_reach = if range.raw() > 0 {
                        squared <= range.raw() * range.raw()
                    } else {
                        // A melee attacker keeps the rule it always had: the next tile, diagonals
                        // included. Comparing distance to one tile would exclude the diagonal.
                        adjacent
                    };
                    // A siege engine - and a skirmisher - has a dead zone: inside it the shot is not
                    // possible however close the target is, so the shooter backs away instead of
                    // standing there. The distance is measured **to the target's edge**, because two
                    // bodies cannot share a tile: a skirmisher's one-tile dead zone would otherwise
                    // never bite on a tile grid, where the closest a neighbour can stand is one tile
                    // from centre to centre (D44).
                    let padding = if min_range.raw() > 0 {
                        self.body_radii(entity.unit, victim_unit)
                    } else {
                        0
                    };
                    let dead = min_range.raw() + padding;
                    let too_close = min_range.raw() > 0 && squared < dead * dead;
                    // A stance decides how much of this fight is the unit's own business: it can refuse
                    // the chase (stand ground), refuse to leave its post (defensive), or refuse the
                    // fight altogether (no attack, which the validator already keeps out of here).
                    let stance = self.stance_of(entity.id);
                    let off_ground = stance == Stance::Defensive
                        && self.stances.get(&entity.id).is_some_and(|(_, post)| {
                            let dx = victim.0.raw() - post.0.raw();
                            let dy = victim.1.raw() - post.1.raw();
                            dx * dx + dy * dy > self.aggro_radius * self.aggro_radius
                        });
                    if stance == Stance::NoAttack || off_ground {
                        if off_ground {
                            homing.push(entity.id);
                        }
                        (Fixed::ZERO, Fixed::ZERO)
                    } else if too_close && stance == Stance::StandGround {
                        // It cannot shoot what is against it, and it may not step back either.
                        (Fixed::ZERO, Fixed::ZERO)
                    } else if too_close {
                        let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                            SimError::Displacement {
                                id: entity.id,
                                detail: err.to_string(),
                            }
                        })?;
                        // Away from the target, along the line it already lies on: the point reflected
                        // through the attacker's own position.
                        let away = (
                            Fixed::from_raw(entity.position.0.raw() * 2 - victim.0.raw()),
                            Fixed::from_raw(entity.position.1.raw() * 2 - victim.1.raw()),
                        );
                        let (dx, dy) = step_towards(entity.position, away, delta.raw());
                        (Fixed::from_raw(dx), Fixed::from_raw(dy))
                    } else if in_reach {
                        strikes.push((entity.id, target));
                        (Fixed::ZERO, Fixed::ZERO)
                    } else if stance == Stance::StandGround {
                        // In reach or not, it does not walk to the fight.
                        (Fixed::ZERO, Fixed::ZERO)
                    } else {
                        let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                            SimError::Displacement {
                                id: entity.id,
                                detail: err.to_string(),
                            }
                        })?;
                        let next = self.next_waypoint(entity.id).unwrap_or(victim);
                        let (dx, dy) = step_towards(entity.position, next, delta.raw());
                        (Fixed::from_raw(dx), Fixed::from_raw(dy))
                    }
                }
                Task::Build { site } => {
                    let Some(target) = self.find(site).map(|s| s.position) else {
                        finished.push(entity.id);
                        continue;
                    };
                    // Adjacent counts: the site's own tile is where the building will be, so a
                    // builder works from the tile beside it.
                    let here = self.tile_of(entity.position);
                    let there = self.tile_of(target);
                    let adjacent = here.0.abs_diff(there.0) <= 1 && here.1.abs_diff(there.1) <= 1;
                    if adjacent {
                        builders.push((entity.id, site));
                        (Fixed::ZERO, Fixed::ZERO)
                    } else {
                        let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                            SimError::Displacement {
                                id: entity.id,
                                detail: err.to_string(),
                            }
                        })?;
                        let next = self.next_waypoint(entity.id).unwrap_or(target);
                        let (dx, dy) = step_towards(entity.position, next, delta.raw());
                        (Fixed::from_raw(dx), Fixed::from_raw(dy))
                    }
                }
                Task::Return { drop, .. } => {
                    let Some(target) = self.find(drop).map(|n| n.position) else {
                        finished.push(entity.id);
                        continue;
                    };
                    let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                        SimError::Displacement {
                            id: entity.id,
                            detail: err.to_string(),
                        }
                    })?;
                    // **Touching the drop site is enough** - any side, any corner (the owner's rule, 2026-10-29). Waiting
                    // for the final step *onto* the building's own tile is what parked twenty-seven villagers beside a
                    // mill and five woodcutters beside a lumber camp for a thousand game-seconds, each holding a load
                    // and each with one waypoint left that they never took.
                    let here = self.tile_of(entity.position);
                    let there = self.tile_of(target);
                    let touching = here.0.abs_diff(there.0) <= 1 && here.1.abs_diff(there.1) <= 1;
                    if touching {
                        deposits.push(entity.id);
                        (Fixed::ZERO, Fixed::ZERO)
                    } else {
                        let next = self.next_waypoint(entity.id).unwrap_or(target);
                        let (dx, dy) = step_towards(entity.position, next, delta.raw());
                        (Fixed::from_raw(dx), Fixed::from_raw(dy))
                    }
                }
                Task::Gather {
                    node,
                    rate_per_tick,
                    carry_cap,
                    progress,
                } => {
                    let _ = progress;
                    let Some((target, target_is_building, node_unit)) = self
                        .find(node)
                        .map(|n| (n.position, n.kind == EntityKind::Building, n.unit))
                    else {
                        // The node is gone (a later increment may delete emptied nodes).
                        finished.push(entity.id);
                        continue;
                    };
                    let here = self.tile_of(entity.position);
                    let there = self.tile_of(target);
                    // A node is stood on; a building is worked from the tile beside it, because its
                    // own tile is blocked and a worker that insisted on it would walk forever.
                    // **Standing beside a node you cannot stand on counts as working it.** A tree in the forest sits on
                    // blocked terrain, so a villager that walks up to it stops on the adjacent tile with one waypoint
                    // left and never arrives: `at_node` was false for ever, `take_amount` never ran, and - because the
                    // villager was carrying a partial load - the "find the next node" branch did not fire either. Four
                    // woodcutters did this for a thousand game-seconds beside two spent trees (2026-10-29), which is
                    // why the wood column produced nothing while the food column produced 1,994.
                    let beside = target_is_building || !self.tile_walkable(there.0, there.1);
                    let at_node = entity.position == target
                        || (beside
                            && here.0.abs_diff(there.0) <= 1
                            && here.1.abs_diff(there.1) <= 1);

                    // A herdable animal is walked to the drop site by the villager working it (D70). The
                    // node was already read above, so this costs one family lookup and no second scan of the
                    // world - the first version re-found the node per gatherer per tick, which is O(units).
                    if !self.herdable.is_empty()
                        && node_unit.is_some_and(|unit| {
                            self.units
                                .as_ref()
                                .is_some_and(|units| self.herdable.contains(&units.root(unit)))
                        })
                    {
                        herds.push((entity.id, node));
                    }

                    // A **huntable** animal is shot, not gathered from, and the shot is ordered **before** the walk:
                    // a hunter that first walked onto the boar would be inside knife range at the first swing, and
                    // the bow (VMHUN, `range = 3.0`) would never matter. A herdable is walked home first and killed at
                    // the drop site, which is the herding pass's business.
                    if self.animal_at(node) == Some(false) {
                        hunts.push((entity.id, node));
                    }

                    // Standing still at the thing being worked is what entitles a worker to take
                    // from it.
                    if at_node {
                        // **A standing tree is chopped down first.** Until its hit points are gone it yields nothing;
                        // when they are, it falls where it stands and is then cut for wood (the owner's rule). A bush or
                        // a mine has no hit points, so this branch never runs for them.
                        // Only a **resource node** is felled. A farm is a building with hit points of its own, and
                        // chopping it was how the first version of this left every farmer standing in a field.
                        let standing = self
                            .find(node)
                            .is_some_and(|n| matches!(n.kind, EntityKind::Resource(_)) && n.hp > 0);
                        if standing {
                            chops.push(entity.id);
                            (Fixed::ZERO, Fixed::ZERO)
                        } else {
                            let (take, kept) = self.take_amount(entity);
                            let node_left = self.find(node).map(|n| n.amount.raw()).unwrap_or(0);
                            takes.push((entity.id, take, kept));

                            // Two reasons to head home: the load is full, or the node just gave its
                            // last. The second matters because a partial load on a dead tree never
                            // reaches the cap - the villager would stand there for the rest of the
                            // match holding wood nobody can spend.
                            let in_hand = entity.carry.raw() + take;
                            let empty_node = node_left - take <= 0;
                            let full = in_hand >= i64::from(carry_cap) * self.scale;
                            // A **full load always goes home**, whether or not anything was taken this tick. Without the
                            // first clause a villager whose cap was reached by a re-task (a load carried onto the next
                            // node) stood at that node with no room to take anything, for the rest of the match.
                            let brimming = entity.carry.raw() >= i64::from(carry_cap) * self.scale;
                            if brimming
                                || (take > 0 && (full || empty_node))
                                || (take == 0 && empty_node && in_hand > 0)
                            {
                                switches.extend(
                                    self.return_task(entity, node, rate_per_tick, carry_cap)
                                        .map(|task| (entity.id, task)),
                                );
                            }
                            if take == 0 && empty_node {
                                // Nothing in hand and nothing left here: look for the next node of the same kind before
                                // standing still. A tree holds 100 wood, so without this a lumberjack works one tree and
                                // idles for the rest of the match - which is what stopped the acceptance match at 139
                                // wood (the owner's rule: "move to next tree, next bush, next pile of gold").
                                let here = entity.position;
                                match self.nearest_like(here, node, entity.id) {
                                    Some(next) => retasks.push((entity.id, next)),
                                    None => finished.push(entity.id),
                                }
                            }
                            (Fixed::ZERO, Fixed::ZERO)
                        }
                    } else {
                        let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                            SimError::Displacement {
                                id: entity.id,
                                detail: err.to_string(),
                            }
                        })?;
                        let next = self.next_waypoint(entity.id).unwrap_or(target);
                        let (dx, dy) = step_towards(entity.position, next, delta.raw());
                        (Fixed::from_raw(dx), Fixed::from_raw(dy))
                    }
                }
                Task::Fly {
                    target,
                    shooter,
                    damage,
                    aim,
                    blast,
                } => {
                    // The step is resolved once per tick, outside the borrow: `step_towards` needs no
                    // table, so the only thing to avoid is touching `self` inside the loop.
                    let (dx, dy) = step_towards(entity.position, aim, projectile_step);
                    if dx == 0 && dy == 0 {
                        // Landing is the only thing an arrow does. Whether it hurts anything is
                        // decided after the loop, against the target as it stands then: an arrow
                        // whose target died on the way finds nothing and does not pick another.
                        landings.push(Landing {
                            shooter,
                            target,
                            damage,
                            aim,
                            blast,
                        });
                        landed_arrows.push(entity.id);
                    }
                    (Fixed::from_raw(dx), Fixed::from_raw(dy))
                }
                // Inside a building: no movement, no work, and no cooldown runs - a sheltered unit is
                // simply not on the map.
                Task::Move { to } => {
                    #[allow(clippy::let_and_return)]
                    let (dx, dy) = {
                        let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                            SimError::Displacement {
                                id: entity.id,
                                detail: err.to_string(),
                            }
                        })?;
                        let next = self.next_waypoint(entity.id).unwrap_or(to);
                        step_towards(entity.position, next, delta.raw())
                    };
                    if dx == 0 && dy == 0 && entity.position == to {
                        finished.push(entity.id);
                    }
                    (Fixed::from_raw(dx), Fixed::from_raw(dy))
                }
                Task::Hunt { to } => {
                    // Looking for a fight first: an enemy in reach is the order's business, and the
                    // walk is only how the unit got here. Two stances never look: one cannot fight,
                    // the other does not walk to a fight it was not given.
                    let may_engage = matches!(
                        self.stance_of(entity.id),
                        Stance::Aggressive | Stance::Defensive
                    );
                    if let Some(enemy) = may_engage.then(|| self.nearest_enemy(entity)).flatten() {
                        engaging.push((entity.id, enemy));
                        (Fixed::ZERO, Fixed::ZERO)
                    } else {
                        let delta = entity.speed.div_exact(tick_rate).map_err(|err| {
                            SimError::Displacement {
                                id: entity.id,
                                detail: err.to_string(),
                            }
                        })?;
                        let next = self.next_waypoint(entity.id).unwrap_or(to);
                        let (dx, dy) = step_towards(entity.position, next, delta.raw());
                        if dx == 0 && dy == 0 && entity.position == to {
                            // Arrival: a patrol turns round, an attack-move stops.
                            if self.patrols.contains_key(&entity.id) {
                                turning.push(entity.id);
                            } else {
                                finished.push(entity.id);
                            }
                        }
                        (Fixed::from_raw(dx), Fixed::from_raw(dy))
                    }
                }
                Task::Garrisoned { .. } => (Fixed::ZERO, Fixed::ZERO),
                Task::Idle => {
                    let dx = entity.velocity.0.div_exact(tick_rate).map_err(|err| {
                        SimError::Displacement {
                            id: entity.id,
                            detail: err.to_string(),
                        }
                    })?;
                    let dy = entity.velocity.1.div_exact(tick_rate).map_err(|err| {
                        SimError::Displacement {
                            id: entity.id,
                            detail: err.to_string(),
                        }
                    })?;
                    (dx, dy)
                }
            };
            deltas.push((index, entity.id, dx, dy));
        }

        // A hunt that found somebody starts the fight and remembers the walk it interrupted.
        for (hunter, target) in engaging {
            let Some(walk) = self.find(hunter).map(|entity| entity.task) else {
                continue;
            };
            if self.start_attack(hunter, target).is_ok() {
                self.resuming.insert(hunter, walk);
            }
        }
        // A patrol at its far end swaps ends: the tile it just reached becomes the one to come back
        // to, and the tile it came from becomes the goal.
        for id in turning {
            let Some(Task::Hunt { to }) = self.find(id).map(|entity| entity.task) else {
                continue;
            };
            let Some(other) = self.patrols.insert(id, to) else {
                continue;
            };
            let route = self
                .find(id)
                .map(|entity| entity.position)
                .and_then(|from| self.plan_path(from, other).ok());
            match route {
                Some(route) => {
                    self.paths.insert(id, route);
                    if let Some(entity) = self.find_mut(id) {
                        entity.task = Task::Hunt { to: other };
                    }
                }
                // An end it cannot get back to is an order that has run out, not a unit that should
                // stand there forever pretending to patrol.
                None => {
                    self.patrols.remove(&id);
                    if let Some(entity) = self.find_mut(id) {
                        entity.task = Task::Idle;
                    }
                }
            }
        }

        // A defensive unit whose fight walked off its ground goes home; that is the difference
        // between holding a chokepoint and chasing an army across the map.
        for id in homing {
            let Some(post) = self.stances.get(&id).map(|(_, post)| *post) else {
                continue;
            };
            let route = self
                .find(id)
                .map(|entity| entity.position)
                .and_then(|from| self.plan_path(from, post).ok());
            match route {
                Some(route) => {
                    self.paths.insert(id, route);
                    if let Some(entity) = self.find_mut(id) {
                        entity.task = Task::Move { to: post };
                    }
                }
                None => self.stand_down(id),
            }
        }

        // Grant takes in id order, each bounded by what the node still holds *at that moment*.
        // Reading the amount before the loop would let two villagers on a one-wood tree take one
        // wood each: the node would clamp at zero and the pair would keep two.
        for worker in chops {
            let node = match self.find(worker).map(|e| e.task) {
                Some(Task::Gather { node, .. }) => node,
                _ => continue,
            };
            let per_tick = self.economy.as_ref().map_or(0, |economy| {
                economy.gather.fell_hp_per_tick(self.tick_rate_hz)
            });
            if let Some(target) = self.find_mut(node) {
                // Accumulate in whole points so a slow chop still lands: a tree falls when its hit points reach zero and
                // **stays where it stands** until the wood is gone.
                let loss = per_tick;
                target.hp = (target.hp - loss).max(0);
            }
        }
        for (worker, take, kept) in takes {
            let node = match self.find(worker).map(|e| e.task) {
                Some(Task::Gather { node, .. }) => node,
                _ => continue,
            };
            if let Some(entity) = self.find_mut(worker)
                && let Task::Gather { progress, .. } = &mut entity.task
            {
                *progress = kept;
            }
            let granted = match self.find_mut(node) {
                Some(target) => {
                    let available = target.amount.raw();
                    let granted = take.min(available);
                    target.amount = Fixed::from_raw(available - granted);
                    // Only a **resource node** disappears when it is emptied. A farm is a building that content refills
                    // (Horse Collar and friends), and removing it here deleted the crop instead of the node.
                    if available - granted <= 0
                        && self
                            .find(node)
                            .is_some_and(|entity| matches!(entity.kind, EntityKind::Resource(_)))
                    {
                        self.spent_nodes.insert(node);
                    }
                    granted
                }
                None => 0,
            };
            if let Some(entity) = self.find_mut(worker) {
                entity.carry = Fixed::from_raw(entity.carry.raw() + granted);
            }
        }
        // A hunt: the villager attacks the animal, and the gather order is remembered so it resumes on the body.
        for (hunter, animal) in hunts {
            self.slaughter.entry(hunter).or_insert(animal);
            let already = matches!(
                self.find(hunter).map(|entity| entity.task),
                Some(Task::Attack { target, .. }) if target == animal
            );
            if !already && let Err(error) = self.start_attack(hunter, animal) {
                self.refusal = Some((hunter, error, self.tick));
            }
        }
        // The next node of the same kind, for a worker whose own ran out. An ordinary gather order: the load in hand is
        // kept, and the walk is planned by the same code a player's click uses.
        for (worker, node) in retasks {
            if let Err(error) = self.start_gather(worker, node) {
                self.refusal = Some((worker, error, self.tick));
            }
        }
        for (id, task) in switches {
            // A worker heading home needs a route to the drop site, not just the final position.
            if let Task::Return { drop, .. } = task {
                let ends = (
                    self.find(id).map(|entity| entity.position),
                    self.find(drop).map(|entity| entity.position),
                );
                let route = match ends {
                    (Some(from), Some(to)) => self.plan_path(from, to).ok(),
                    _ => None,
                };
                if let Some(route) = route {
                    self.paths.insert(id, route);
                }
            } else {
                self.paths.remove(&id);
            }
            if let Some(entity) = self.find_mut(id) {
                entity.task = task;
            }
        }
        // Herding (D70): a villager working a herdable animal walks it toward the **nearest drop site of
        // that villager's player** - not toward itself - and follows it. The animal's route is planned here
        // rather than every tick, and the villager's cached route is dropped so it steps at where the animal
        // *is* rather than at where it was when the order was given.
        for (villager, animal) in herds {
            let animal_pos = self.find(animal).map(|entity| entity.position);
            let player = self.find(villager).map(|entity| entity.player);
            let variant = self.find(villager).and_then(|entity| entity.unit);
            let (Some(animal_pos), Some(player)) = (animal_pos, player) else {
                continue;
            };
            let Some(drop) = self.nearest_drop_site(player, animal_pos, variant) else {
                continue;
            };
            let Some(drop_pos) = self.find(drop).map(|entity| entity.position) else {
                continue;
            };
            // Arrived: the shepherd **strikes** it where it stopped (D70, the owner's correction: *"shepherd has
            // to kill the animal. It doesn't change to corpse itself."*). The kill is an ordinary attack, so the
            // blow carries its attacker and the last-blow rule decides the meat; death promotes the animal to its
            // body in place, and the gather order is resumed on that body below. Arriving is not killing.
            if self.tile_of(animal_pos) == self.tile_of(drop_pos) {
                self.slaughter.insert(villager, animal);
                let already = matches!(
                    self.find(villager).map(|entity| entity.task),
                    Some(Task::Attack { target, .. }) if target == animal
                );
                if !already && let Err(error) = self.start_attack(villager, animal) {
                    self.refusal = Some((villager, error, self.tick));
                }
                continue;
            }
            let heading = matches!(
                self.find(animal).map(|entity| entity.task),
                Some(Task::Move { to }) if to == drop_pos
            );
            if !heading && self.tile_of(animal_pos) != self.tile_of(drop_pos) {
                self.walk_towards(animal, drop_pos);
                if let Some(entity) = self.find_mut(animal) {
                    entity.task = Task::Move { to: drop_pos };
                }
            }
            self.paths.remove(&villager);
        }
        // A shepherd whose blow has landed goes back to the work it was sent to do: the animal it was gathering is
        // now a body, and the gather order resumes on the same id (D70, item 3).
        if !self.slaughter.is_empty() {
            let pairs: Vec<(EntityId, EntityId)> = self
                .slaughter
                .iter()
                .map(|(one, other)| (*one, *other))
                .collect();
            for (villager, animal) in pairs {
                let idle = matches!(
                    self.find(villager).map(|entity| entity.task),
                    Some(Task::Idle)
                );
                if !idle {
                    continue;
                }
                match self.find(animal).map(|entity| entity.hp) {
                    // Dead: the body is waiting to be worked.
                    Some(hp) if hp <= 0 => {
                        if let Err(error) = self.start_gather(villager, animal) {
                            self.refusal = Some((villager, error, self.tick));
                        }
                        self.slaughter.remove(&villager);
                    }
                    // Still alive and no longer swinging: try again, and give up on the entry if it will not start
                    // (a refusal is recorded; re-issuing it every tick would only bury it).
                    Some(_) => {
                        if let Err(error) = self.start_attack(villager, animal) {
                            self.refusal = Some((villager, error, self.tick));
                            self.slaughter.remove(&villager);
                        }
                    }
                    None => {
                        self.slaughter.remove(&villager);
                    }
                }
            }
        }
        // Deposit whole resources; the fraction stays in hand. Dropping the remainder instead
        // would quietly destroy a slice of every trip.
        for id in deposits {
            let Some((player, task)) = self.find(id).map(|e| (e.player, e.task)) else {
                continue;
            };
            let Task::Return {
                node,
                slot,
                rate_per_tick,
                carry_cap,
                ..
            } = task
            else {
                continue;
            };
            let raw = self.find(id).map(|e| e.carry.raw()).unwrap_or(0);
            let whole = raw / self.scale;
            let slot_player = usize::from(player).wrapping_sub(1);
            if whole > 0 && slot_player < self.stock.len() {
                let slot = &mut self.stock[slot_player][slot];
                *slot = slot.saturating_add(u32::try_from(whole).unwrap_or(u32::MAX));
            }
            let node_left = self.find(node).map(|n| n.amount.raw()).unwrap_or(0);
            let scale = self.scale;
            // Going back to work needs a fresh route from the drop site: without one the villager
            // walks at the node in a straight line and straight through whatever it went around.
            if node_left > 0 {
                let ends = (
                    self.find(id).map(|entity| entity.position),
                    self.find(node).map(|entity| entity.position),
                );
                let route = match ends {
                    (Some(here), Some(there)) => self.plan_path(here, there).ok(),
                    _ => None,
                };
                if let Some(route) = route {
                    self.paths.insert(id, route);
                }
            }
            // The shares moved this worker while it was carrying: the bank has it now, so it takes up the new column.
            if let Some(next) = self.pending_gather.remove(&id) {
                if let Err(error) = self.start_gather(id, next) {
                    self.refusal = Some((id, error, self.tick));
                }
                continue;
            }
            let mut spent = false;
            if let Some(entity) = self.find_mut(id) {
                entity.carry = Fixed::from_raw(raw - whole * scale);
                if node_left > 0 {
                    entity.task = Task::Gather {
                        node,
                        rate_per_tick,
                        carry_cap,
                        progress: Fixed::ZERO,
                    };
                } else {
                    entity.task = Task::Idle;
                    spent = true;
                }
            }
            if spent {
                // The node is finished: go to the next one of the same kind rather than standing down (the owner's
                // rule). This is where a lumberjack used to stop for the rest of the match after one tree.
                let here = self.find(id).map(|entity| entity.position);
                if let Some(next) = here.and_then(|here| self.nearest_like(here, node, id))
                    && let Err(error) = self.start_gather(id, next)
                {
                    self.refusal = Some((id, error, self.tick));
                }
            }
        }
        // Construction advances one builder-tick per tick of work, so the finish lands exactly on
        // `create_time x tick_rate` even where HP per second is not a whole number (a mill is
        // 600 HP over 35 s).
        for (builder, site) in builders {
            if self.work_one_tick(site) {
                let finished = self
                    .find(site)
                    .map(|building| (building.player, building.unit));
                if let Some(building) = self.find_mut(site) {
                    building.construction = None;
                }
                self.foundations.remove(&site);
                // Finished once is finished for good, for the purposes of prerequisites (D63).
                if let Some((player, Some(unit))) = finished {
                    let family = self.units.as_ref().map_or(unit, |units| units.root(unit));
                    self.built.insert((player, family));
                }
                // The builder comes to rest *and forgets its route* (D64). Setting the task to idle was
                // not enough: the waypoints stayed, and a villager that finished a house on its doorstep
                // kept walking into the building at its own speed for as long as the match ran.
                self.stand_down(builder);
                self.paths.remove(&builder);
                // The next queued site starts on the tick the last one finished, so a villager with a
                // queue never stands idle between buildings (D66).
                let next = self
                    .build_queue
                    .get_mut(&builder)
                    .and_then(|queue| queue.pop_front());
                if self
                    .build_queue
                    .get(&builder)
                    .is_some_and(|queue| queue.is_empty())
                {
                    self.build_queue.remove(&builder);
                }
                if let Some((unit, at)) = next {
                    let _ = self.start_build(builder, unit, at);
                }
            }
        }
        // Melee: a blow lands on the tick the attacker is in position, then every `reload_ticks`.
        for (attacker, target) in strikes {
            let swung = self.swing(attacker);
            let landed = self.blow_against(attacker, target, swung);
            if landed <= 0 {
                continue;
            }
            let flight = self.projectile_speed();
            let Some(source) = self.find(attacker).map(|entity| {
                (
                    entity.position,
                    entity.player,
                    entity.kind,
                    entity.unit,
                    entity.task,
                )
            }) else {
                continue;
            };
            let blast = source
                .3
                .and_then(|unit| self.blast.get(&unit).copied())
                .unwrap_or(0);
            let Some(victim_position) = self.find(target).map(|entity| entity.position) else {
                continue;
            };
            // An arrow flies to a point, so the point is the whole of the aim: at the target's feet
            // without Ballistics, at where it will be with it (D42).
            let aim = if flight > 0 && self.has_ballistics(attacker) {
                self.lead_shot(source.0, victim_position, target, flight)
            } else {
                victim_position
            };
            // ... and accuracy decides whether the archer aims *true* at all. A shot that is not aimed
            // true lands the content's scatter away, which the landing rule turns into a miss (D43).
            let aim = self.true_aim(attacker, aim);
            // Only a *ranged* blow flies. The pack's shot speed used to be applied to every blow, so a
            // knife became an arrow in the air: it landed in the `landings` loop, which carried no
            // attacker, and melee damage was therefore unattributable (D71). A melee blow lands here,
            // on the tick it is swung, where `(attacker, target)` is known.
            let ranged = matches!(source.4, Task::Attack { range, .. } if range > Fixed::ZERO);
            if ranged && flight > 0 && source.0 != aim {
                // The shot exists now and lands later. It carries the damage resolved when the order
                // was given, so an upgrade that arrives mid-flight changes nothing about it. It is
                // *recorded* here and created below, because the entity list is index-stable for the
                // length of this tick and inserting into it mid-pass would shift every later index.
                // A building looses a volley rather than a shot: its own arrow plus whatever its
                // garrison adds. Each arrow carries the *building's* damage for now - the published
                // rule has extra arrows transmit the unit's attack, and that difference is recorded
                // in docs/decisions.md rather than guessed at here (D37).
                let arrows = if source.2 == EntityKind::Building {
                    self.volley(attacker)
                } else {
                    1
                };
                for _ in 0..arrows {
                    shots.push(Shot {
                        player: source.1,
                        shooter: attacker,
                        from: source.0,
                        aim,
                        target,
                        damage: landed,
                        blast: Fixed::from_raw(blast),
                    });
                }
                continue;
            }
            // A pack whose shots are instant: the blow lands where it was aimed, and a blast still
            // splashes around that point rather than hitting one unit (D51).
            if let Some(victim) = self.find_mut(target) {
                victim.hp -= landed;
            }
            blows.push((target, attacker));
            // A blow is an order to an animal (D70): it turns on whoever struck it.
            self.retaliate(target, attacker);
            let blast = source
                .3
                .and_then(|unit| self.blast.get(&unit).copied())
                .unwrap_or(0);
            for victim in self.blast_victims(aim, Fixed::from_raw(blast), target) {
                if let Some(entity) = self.find_mut(victim) {
                    entity.hp -= landed;
                }
                ejected.push(victim);
            }
            ejected.push(target);
        }
        // Production: one job per building, finishing exactly on its create time. A unit with
        // nowhere to stand waits inside the building rather than being lost.
        // Production and research: one pass over the entities, because both timers are the same
        // shape and the second `collect` used to cost a whole extra scan of the world every tick.
        let mut trainers: Vec<EntityId> = Vec::new();
        // Who is at their ceiling, worked out once per player that has a building in production rather
        // than once per building per tick. A paused job keeps its remaining ticks: the queue waits for
        // a unit to die or a house to finish, which is the rule (D34).
        let mut capped: [bool; MAX_PLAYERS] = [false; MAX_PLAYERS];
        let mut researchers: Vec<(EntityId, u8, u32)> = Vec::new();
        for entity in &self.entities {
            if entity.construction.is_some() {
                continue;
            }
            if entity.training.is_some() {
                trainers.push(entity.id);
            }
            if let Some(job) = entity.researching {
                researchers.push((entity.id, entity.player, job.tech));
            }
        }
        {
            let mut seen: [bool; MAX_PLAYERS] = [false; MAX_PLAYERS];
            for id in &trainers {
                if let Some(player) = self.find(*id).map(|entity| entity.player) {
                    let index = usize::from(player).wrapping_sub(1);
                    if index < MAX_PLAYERS && !seen[index] {
                        seen[index] = true;
                        capped[index] = self.population(player) >= self.population_cap(player);
                    }
                }
            }
        }
        for id in trainers {
            if self.find(id).is_some_and(|entity| {
                capped[usize::from(entity.player)
                    .wrapping_sub(1)
                    .min(MAX_PLAYERS - 1)]
            }) {
                continue;
            }
            let ready = match self.find_mut(id) {
                Some(entity) => match entity.training {
                    Some(mut job) => {
                        if job.fresh {
                            job.fresh = false;
                            entity.training = Some(job);
                            false
                        } else {
                            job.remaining_ticks = job.remaining_ticks.saturating_sub(1);
                            let done = job.remaining_ticks == 0;
                            entity.training = Some(job);
                            done
                        }
                    }
                    None => false,
                },
                None => false,
            };
            if ready {
                self.spawn_from_building(id);
            }
        }
        // Research: the same timer, and the effect lands when it reaches zero.
        for (id, player, tech) in researchers {
            let landed = match self.find_mut(id) {
                Some(entity) => match entity.researching {
                    Some(mut job) => {
                        if job.fresh {
                            job.fresh = false;
                            entity.researching = Some(job);
                            false
                        } else {
                            job.remaining_ticks = job.remaining_ticks.saturating_sub(1);
                            let done = job.remaining_ticks == 0;
                            if done {
                                entity.researching = None;
                            } else {
                                entity.researching = Some(job);
                            }
                            done
                        }
                    }
                    None => false,
                },
                None => false,
            };
            if landed {
                self.finish_research(player, tech)?;
            }
        }
        let mut resumed: Vec<EntityId> = Vec::new();
        for id in finished {
            if self.resuming.contains_key(&id) {
                // The fight ended and the unit still has a place to be: attack-move's whole point.
                resumed.push(id);
            } else {
                self.stand_down(id);
            }
        }
        for id in resumed {
            let Some(task @ Task::Hunt { to }) = self.resuming.remove(&id) else {
                continue;
            };
            let route = self
                .find(id)
                .map(|entity| entity.position)
                .and_then(|from| self.plan_path(from, to).ok());
            match route {
                Some(route) => {
                    self.paths.insert(id, route);
                    if let Some(entity) = self.find_mut(id) {
                        entity.task = task;
                    }
                }
                None => {
                    if let Some(entity) = self.find_mut(id) {
                        entity.task = Task::Idle;
                    }
                }
            }
        }

        let mut reached: Vec<(EntityId, (Fixed, Fixed))> = Vec::new();
        // Whatever actually moved this tick is what can see something new (D50a).
        let mut movers: Vec<EntityId> = Vec::new();
        // The occupancy grid is only worth building on a tick where something moves; an idle world pays
        // one `any` over the delta list, which is empty of movement in the common case (D52).
        let something_moves = deltas
            .iter()
            .any(|(_, _, dx, dy)| *dx != Fixed::ZERO || *dy != Fixed::ZERO);
        let occupied = if something_moves {
            self.occupancy()
        } else {
            Vec::new()
        };
        for (index, id, dx, dy) in deltas {
            let (dx, dy) = self.avoid(&occupied, id, dx, dy);
            if dx != Fixed::ZERO || dy != Fixed::ZERO {
                movers.push(id);
            }
            // Fast path: the entity at this index is still the one we decided for. If a spawn moved
            // it, fall back to a lookup by id - slower, but correct, and only on ticks where the
            // world changed shape.
            if self.entities.get(index).map(|entity| entity.id) != Some(id) {
                let position = self
                    .find(id)
                    .expect("entity list is stable within a tick")
                    .position;
                let moved = (
                    position
                        .0
                        .checked_add(dx)
                        .map_err(|_| SimError::Overflow(id))?,
                    position
                        .1
                        .checked_add(dy)
                        .map_err(|_| SimError::Overflow(id))?,
                );
                let entity = self.find_mut(id).expect("just found");
                entity.position = moved;
                reached.push((id, moved));
                continue;
            }
            let entity = &mut self.entities[index];
            entity.position.0 = entity
                .position
                .0
                .checked_add(dx)
                .map_err(|_| SimError::Overflow(id))?;
            entity.position.1 = entity
                .position
                .1
                .checked_add(dy)
                .map_err(|_| SimError::Overflow(id))?;
            reached.push((id, entity.position));
        }
        for id in movers {
            self.stamp_entity(id);
        }

        // A waypoint reached is a waypoint spent. Done after the move, so a unit that lands on it
        // this tick starts toward the next one on the next tick.
        let scale = self.scale;
        let spent: Vec<EntityId> = reached
            .iter()
            .filter(|(id, position)| {
                self.paths
                    .get(id)
                    .and_then(|path| path.first())
                    .map(|&(x, y)| {
                        *position
                            == (
                                Fixed::from_raw(i64::from(x) * scale),
                                Fixed::from_raw(i64::from(y) * scale),
                            )
                    })
                    .unwrap_or(false)
            })
            .map(|(id, _)| *id)
            .collect();
        for id in spent {
            if let Some(path) = self.paths.get_mut(&id) {
                path.remove(0);
            }
        }

        // Shots go up, land, and leave the list, all after the movement pass: the entity list is
        // index-stable for the length of that pass, and every one of these changes its shape.
        for shot in shots {
            // A full world loses the shot rather than the tick: a battle must not be stopped by the
            // entity ceiling.
            let _ = self.spawn_projectile(shot);
        }
        for Landing {
            shooter,
            target,
            damage,
            aim,
            blast,
        } in landings
        {
            // The arrow lands where it was aimed. It hurts the target only if the target is still
            // there - within half a tile of the landing point, which is the space a unit occupies.
            // Without this an arrow could not miss, and Ballistics would have nothing to change.
            let tolerance = self.scale / 2;
            let reachable = self.find(target).is_some_and(|victim| {
                let dx = victim.position.0.raw() - aim.0.raw();
                let dy = victim.position.1.raw() - aim.1.raw();
                dx * dx + dy * dy <= tolerance * tolerance
            });
            let mut struck = false;
            // A hunter's arrow is a villager's blow too, so the same slaughter rule applies to it (the owner's second
            // correction): an arrow that kills a herdable leaves the carcass, exactly as the knife does.
            let damage = self.blow_against(shooter, target, damage);
            if let Some(victim) = self.find_mut(target)
                && victim.hp > 0
                && reachable
            {
                victim.hp -= damage;
                blows.push((target, shooter));
                struck = true;
            }
            // A shot is an order to an animal just as a knife is: whoever loosed it is now its quarry.
            if struck {
                self.retaliate(target, shooter);
            }
            // ... and a shot with a blast hurts what stands around the landing point, its owner's own
            // units included: a mangonel that could not hit its own line would not need protecting.
            for victim in self.blast_victims(aim, blast, target) {
                if let Some(entity) = self.find_mut(victim) {
                    entity.hp -= damage;
                }
                blows.push((victim, shooter));
                self.retaliate(victim, shooter);
                ejected.push(victim);
            }
            ejected.push(target);
        }
        for id in landed_arrows {
            self.entities.retain(|entity| entity.id != id);
        }
        // Animals react to the blows this tick landed, so the leash is judged after them (D71).
        self.step_leashes();
        // ... and a herdable changes hands as soon as somebody's unit can see it (item 5).
        self.step_conversion();
        // Shares: idle villagers follow the table content gives (the game's own AI shape).
        self.step_shares();
        self.step_spent_nodes();
        // A building at or below its eject fraction throws its garrison out. They are unharmed - the
        // published rule - and they land where they already stand.
        for id in ejected {
            // Copy what is needed out of the borrow before touching anything mutable.
            let Some((kind, hp, player, unit)) = self
                .find(id)
                .map(|building| (building.kind, building.hp, building.player, building.unit))
            else {
                continue;
            };
            if kind != EntityKind::Building || hp <= 0 {
                continue;
            }
            let Some(unit) = unit else { continue };
            let max = self.max_hp(player, unit).unwrap_or(hp);
            let floor = (i64::from(max) * self.garrison_eject_at.raw()) / self.scale;
            if i64::from(hp) > floor {
                continue;
            }
            let inside: Vec<EntityId> = self
                .entities
                .iter()
                .filter(|entity| entity.task == (Task::Garrisoned { building: id }))
                .map(|entity| entity.id)
                .collect();
            for unit in inside {
                if let Some(entity) = self.find_mut(unit) {
                    entity.task = Task::Idle;
                }
            }
        }

        // The dead leave the world only at the end of the tick, once every phase has finished with
        // them: an entity that dies mid-tick still has a position to update and waypoints to spend.
        // Resource nodes are exempt - their HP is zero by design and their amount is what matters.
        // Route bookkeeping dies with its owner. Without this a unit that is killed mid-walk keeps
        // its remaining waypoints forever - a long match accumulates routes nobody walks, and the
        // state hash walks them, so a dead unit's path was part of the hash.
        // A foundation placed blind over something that is already being built is cancelled the moment
        // its owner can see the tile (D61). Done before the sweep so the cancellation dies this tick.
        self.settle_blind_sites();
        let mut buried: Vec<EntityId> = Vec::new();
        // A wall's tile, remembered before the sweep: after `retain` the entity is gone and there is no
        // position left to free.
        let wall_tiles: Vec<(EntityId, (u16, u16))> = if self.blocking.is_empty() {
            Vec::new()
        } else {
            self.entities
                .iter()
                .filter(|entity| entity.kind == EntityKind::Building)
                .filter(|entity| {
                    entity.unit.is_some_and(|unit| {
                        let root = self.units.as_ref().map_or(unit, |units| units.root(unit));
                        self.blocking.contains(&root)
                    })
                })
                .map(|entity| (entity.id, self.tile_of(entity.position)))
                .collect()
        };
        // An animal that died of a blow leaves its body where it stood, and the body keeps the entity's id so the
        // gatherer's task stays valid across the blow (D70). Done before the sweep, which then keeps it because its
        // unit is a named carcass (item 2).
        // Bodies made this tick start rotting on the *next* one: a corpse that lost a tick of food as it appeared
        // would make every arithmetic assertion about meat off by exactly one tick of rot.
        let mut new_rots: Vec<(EntityId, Rot)> = Vec::new();
        self.promote_dead_animals(&blows, &mut new_rots);
        // A body is a unit with no health, and it must survive the pass that removes the dead (D70). The set
        // holds ids as well as families, so this costs a lookup and no table walk: the first version cloned
        // the whole unit table here, once per tick.
        let bodies = self.carcasses.clone();
        // Rot happens here rather than in a pass of its own: the sweep already touches every entity once per tick,
        // and a second walk of the world to take four food off a carcass would be the same mistake the leash review
        // caught. `rots` is taken out so the closure does not borrow `self` twice.
        let mut rots = std::mem::take(&mut self.rots);
        let mut rotted: Vec<EntityId> = Vec::new();
        let now = self.tick;
        self.entities.retain_mut(|entity| {
            if let Some(rot) = rots.get(&entity.id).copied()
                && rot.per_tick > 0
            {
                entity.amount = Fixed::from_raw((entity.amount.raw() - rot.per_tick).max(0));
            }
            let is_body = entity.kind == EntityKind::Unit
                && entity.unit.is_some_and(|unit| bodies.contains(&unit));
            // A body with nothing left in it is gone: nothing eats an empty carcass, and leaving it standing is the
            // "an emptied body stays for ever" hole this closes.
            // Spent: empty meat that was rotting away, or a body that has lain for its species' span - which is the
            // only lifetime an empty carcass has, since there is nothing in it to rot (item 6).
            let spent = is_body
                && rots.get(&entity.id).is_some_and(|rot| {
                    (rot.per_tick > 0 && entity.amount.raw() <= 0) || now >= rot.expires
                });
            let keep = (!matches!(entity.kind, EntityKind::Unit | EntityKind::Building)
                || entity.hp > 0
                || is_body)
                && !spent;
            if !keep {
                if spent {
                    rotted.push(entity.id);
                }
                buried.push(entity.id);
            }
            keep
        });
        for id in rotted {
            rots.remove(&id);
        }
        self.rots = rots;
        for (id, rot) in new_rots {
            self.rots.insert(id, rot);
        }
        // A garrison goes down with its shelter: a unit inside a building that dies has nowhere to be,
        // and the alternative - ejecting it onto the rubble - would let a razed town centre disgorge
        // fifteen villagers. Their ids join `buried`, because they have bookkeeping of their own.
        let razed: BTreeSet<EntityId> = buried.iter().copied().collect();
        let inside: BTreeSet<EntityId> = self
            .entities
            .iter()
            .filter(|entity| {
                matches!(entity.task, Task::Garrisoned { building } if razed.contains(&building))
            })
            .map(|entity| entity.id)
            .collect();
        self.entities.retain(|entity| !inside.contains(&entity.id));
        buried.extend(inside);
        let something_died = !buried.is_empty();
        // Everything keyed by an entity id goes with it. The route was the first of these (a dead
        // unit kept its waypoints forever); the queue, the rally, the stance, the patrol's far end and
        // the interrupted walk are the rest, and the state hash walks all of them.
        for (id, tile) in wall_tiles {
            if buried.contains(&id)
                && let Some(walk) = self.walk.as_mut()
            {
                walk.set_blocked(tile.0, tile.1, false);
            }
        }
        for id in buried {
            self.paths.remove(&id);
            self.rallies.remove(&id);
            self.queued.remove(&id);
            self.resuming.remove(&id);
            self.patrols.remove(&id);
            self.stances.remove(&id);
            self.build_queue.remove(&id);
            self.leash.remove(&id);
            self.slaughter.remove(&id);
            self.foundations.remove(&id);
            self.slaughter.retain(|_, animal| *animal != id);
            self.herdable_ids.remove(&id);
        }

        // A seat can only disappear by dying, so the match is only ever decided on a tick where
        // something did. The idle tick pays one boolean for it.
        if something_died {
            self.decide_outcome();
        }

        self.tick += 1;
        Ok(())
    }

    /// Hand the world the tech and effect tables. Refused when either is empty, because an empty
    /// table is a content mistake that would otherwise surface as "every tech is missing".
    pub fn load_research(
        &mut self,
        techs: &ruage_data::Techs,
        effects: &Effects,
    ) -> Result<(), SimError> {
        if techs.techs.is_empty() {
            return Err(SimError::MissingTech { id: 0 });
        }
        if effects.effects.is_empty() {
            return Err(SimError::MissingEffect { id: 0 });
        }
        self.techs = Some(techs.clone());
        self.effects = Some(effects.clone());

        // What availability looks like in this pack: which units some tech can make available, and which
        // techs exist only to do that. Both are read once, from content.
        self.gated_units.clear();
        self.availability_techs.clear();
        for effect in effects.effects.values() {
            for command in &effect.commands {
                if command.kind == 2
                    && command.b == 1
                    && let Ok(unit) = u32::try_from(command.a)
                {
                    self.gated_units.insert(unit);
                }
            }
        }
        // Which tech upgrades into which unit. The lowest tech id wins when two effects could produce
        // the same unit, so the answer does not depend on iteration order.
        self.upgrade_tech.clear();
        for tech in techs.techs.values() {
            let Some(effect_id) = tech.effect_id else {
                continue;
            };
            let Some(effect) = effects.effects.get(&effect_id) else {
                continue;
            };
            for command in &effect.commands {
                if command.kind != 3 {
                    continue;
                }
                let (Ok(from), Ok(to)) = (u32::try_from(command.a), u32::try_from(command.b))
                else {
                    continue;
                };
                if from == to || to == 0 {
                    continue;
                }
                self.upgrade_tech
                    .entry(to)
                    .and_modify(|existing| *existing = (*existing).min(tech.id))
                    .or_insert(tech.id);
            }
        }
        for tech in techs.techs.values() {
            if tech.research_time != 0 || !tech.costs.is_empty() || tech.location >= 0 {
                continue;
            }
            // Bookkeeping techs are the ones with nothing to click: no price, no time, no building,
            // and an effect that either does nothing at all or only makes units available. The dat
            // spells both kinds of marker this way - "ARRG (make avail)" enables a building, and
            // "Shadow Barracks -- Age Two" *is* the statement that a barracks exists (tech 122, no
            // effect, required by 667, which the archery range requires). Without the second kind the
            // whole Feudal building tree is unreachable: 147 waits on 667, 667 waits on 122, and
            // nothing would ever grant it (D29).
            let bookkeeping = match tech.effect_id {
                None => true,
                Some(effect_id) => match effects.effects.get(&effect_id) {
                    None => true,
                    Some(effect) => {
                        effect.commands.is_empty()
                            || effect
                                .commands
                                .iter()
                                .all(|command| command.kind == 2 && command.b == 1)
                    }
                },
            };
            if bookkeeping {
                self.availability_techs.push(tech.id);
            }
        }

        self.grant_starting_techs()
    }

    /// Load the civ table and resolve the civ content plays as. Refused by name when the pack names a
    /// civ the table does not have: a match would otherwise run with no restrictions at all and give
    /// the player units their civilisation cannot build.
    pub fn load_civs(&mut self, civs: &ruage_data::Civs) -> Result<(), SimError> {
        let wanted = self
            .economy
            .as_ref()
            .map(|economy| economy.start.civs.clone())
            .unwrap_or_default();
        if !wanted.is_empty() {
            self.civ_by_player = [None; MAX_PLAYERS];
            for index in 0..MAX_PLAYERS {
                // The last entry covers everyone past the end of the list: a pack that names one civ
                // means "everyone plays this one".
                let name = &wanted[index.min(wanted.len() - 1)];
                let civ = civs
                    .by_name(name)
                    .ok_or_else(|| SimError::UnknownCiv { name: name.clone() })?;
                self.civ_by_player[index] = Some(civ.id);
            }
        }
        // A civ's team bonus is an effect, not a tech: nothing researches it, and every player of that
        // civ has it from the first tick. Content that names no team bonus simply has an empty list.
        self.bonus_effects = std::array::from_fn(|_| Vec::new());
        for index in 0..MAX_PLAYERS {
            let Some(id) = self.civ_by_player[index] else {
                continue;
            };
            if let Some(bonus) = civs.by_id(id).and_then(|civ| civ.team_bonus_effect) {
                self.bonus_effects[index].push(bonus);
            }
        }
        self.civs = Some(civs.clone());
        Ok(())
    }

    /// May this player have this unit at all? A unit is *gated* when some tech's effect makes it
    /// available (kind 2, positive flag), and gated units need one of those techs researched. A unit
    /// nothing gates - a villager, a house, a spearman - is available always.
    fn unit_available(&self, player: u8, unit: u32) -> bool {
        if !self.gated_units.contains(&unit) {
            return true;
        }
        let (Some(effects), Some(techs)) = (self.effects.as_ref(), self.techs.as_ref()) else {
            return true;
        };
        self.researched(player).iter().any(|tech_id| {
            let Some(effect_id) = techs.get(*tech_id).and_then(|tech| tech.effect_id) else {
                return false;
            };
            let Some(effect) = effects.get(effect_id) else {
                return false;
            };
            effect.commands.iter().any(|command| {
                command.kind == 2
                    && command.b == 1
                    && u32::try_from(command.a).is_ok_and(|id| id == unit)
            })
        })
    }

    /// Is this unit taken away by the player's civilisation? Returns the civ's name so a refusal can say
    /// whose rule it is.
    ///
    /// Derived on the spot rather than cached: the set would have been one more field on `World`, and a
    /// field on `World` costs more than the lookup does (`docs/performance.md`, pass seven and thirteen).
    /// The scan is one effect's command list, and only when an order is placed.
    fn civ_barred_unit(&self, player: u8, unit: u32) -> Option<String> {
        let civs = self.civs.as_ref()?;
        let civ = civs.get(self.civ_of(player)?)?;
        let effects = self.effects.as_ref()?;
        let effect = effects.get(civ.tech_tree_effect?)?;
        let barred = effect.commands.iter().any(|command| {
            // Zero is the data's "take it away": one row in the whole table, the Huns' house.
            command.kind == 2
                && command.b == 0
                && u32::try_from(command.a).is_ok_and(|id| id == unit)
        });
        barred.then(|| civ.name.clone())
    }

    /// Apply the availability techs whose prerequisites the player now meets, and keep going while
    /// granting one unlocks another. Idempotent, and guarded against re-entry because granting a tech
    /// runs `finish_research`, which calls this again.
    fn grant_availability(&mut self, player: u8) -> Result<(), SimError> {
        if self.resolving_availability {
            return Ok(());
        }
        self.resolving_availability = true;
        let result = (|| -> Result<(), SimError> {
            loop {
                let mut granted = false;
                for tech in self.availability_techs.clone() {
                    if self.researched(player).contains(&tech) {
                        continue;
                    }
                    let Some(stats) = self.techs.as_ref().and_then(|techs| techs.get(tech)) else {
                        continue;
                    };
                    // The same counting rule as an order: "at least min_required of them".
                    let have = stats
                        .required
                        .iter()
                        .filter_map(|required| u32::try_from(*required).ok())
                        .filter(|required| self.researched(player).contains(required))
                        .count();
                    let ready = have >= usize::from(stats.min_required);
                    if ready {
                        self.finish_research(player, tech)?;
                        granted = true;
                    }
                }
                if !granted {
                    return Ok(());
                }
            }
        })();
        self.resolving_availability = false;
        result
    }

    /// The team a player is on. A player with no team in content is alone on a team of their own, so
    /// "same team" never accidentally means "everybody".
    fn team_of(&self, player: u8) -> u8 {
        let index = usize::from(player).checked_sub(1);
        index
            .and_then(|index| self.team_by_player.get(index).copied())
            .unwrap_or(player)
    }

    /// Which civilisation a player is, once the table is loaded and content has named one.
    fn civ_of(&self, player: u8) -> Option<u32> {
        let index = usize::from(player).checked_sub(1)?;
        self.civ_by_player.get(index).copied().flatten()
    }

    /// Is this tech on the player's civilisation's restriction list? Returns the civ's name, so a
    /// refusal can say *whose* rule it is.
    fn barred_by(&self, player: u8, tech: u32) -> Option<String> {
        let civs = self.civs.as_ref()?;
        let effects = self.effects.as_ref()?;
        let civ = civs.get(self.civ_of(player)?)?;
        let effect = effects.get(civ.tech_tree_effect?)?;
        // Kind 102 carries the barred id in `value` and leaves the four selectors at -1: the effect
        // table says "this id is in the tree", with no state field to read a direction from, which is
        // why the direction had to be settled from the data (see docs/decisions.md D23).
        let barred = effect.commands.iter().any(|command| {
            command.kind == 102
                && command
                    .value
                    .resolve(self.scale, "effect.value")
                    .is_ok_and(|value| value.raw() == i64::from(tech) * self.scale)
        });
        barred.then(|| civ.name.clone())
    }

    /// Give each player the resources content says a match opens with (`[start] stock`).
    ///
    /// Called from `seed_from` and from `load_resources` for the same reason the starting techs are
    /// granted in two places: a runner may seed before it loads the pack or after. It runs once -
    /// `start_stock_applied` says so - because the alternative is a second `load_resources` silently
    /// wiping whatever the player had gathered by then.
    fn apply_start_stock(&mut self) -> Result<(), SimError> {
        if self.start_stock_applied {
            return Ok(());
        }
        let (Some(economy), Some(resources)) = (self.economy.as_ref(), self.resources.as_ref())
        else {
            return Ok(());
        };
        let mut wanted: Vec<(usize, u32)> = Vec::new();
        for (name, amount) in &economy.start.stock {
            let slot = resources
                .slot_of(name)
                .ok_or_else(|| SimError::UnknownResourceName { name: name.clone() })?;
            wanted.push((slot, *amount));
        }
        for player in 1..=u8::try_from(MAX_PLAYERS).unwrap_or(1) {
            if !self.entities.iter().any(|entity| entity.player == player) {
                continue;
            }
            let index = usize::from(player).wrapping_sub(1);
            let Some(row) = self.stock.get_mut(index) else {
                continue;
            };
            for (slot, amount) in &wanted {
                if let Some(cell) = row.get_mut(*slot) {
                    *cell = *amount;
                }
            }
        }
        self.start_stock_applied = true;
        Ok(())
    }

    /// Grant the techs content says a match opens with - the pack names the Dark Age, which every other
    /// tech lists as a prerequisite, so a world that skipped it could research nothing at all.
    ///
    /// Both `seed_from` and `load_research` call this: a runner may load the tables before it seeds or
    /// after, and the opening position must not depend on that order. Granting twice is harmless - a
    /// researched tech is a set insert, the health delta is zero, and a kind-3 upgrade finds no unit
    /// still carrying the old id.
    fn grant_starting_techs(&mut self) -> Result<(), SimError> {
        if self.techs.is_none() {
            return Ok(());
        }
        let Some(economy) = self.economy.clone() else {
            return Ok(());
        };
        let starting: Vec<u32> = economy
            .start
            .techs
            .iter()
            .filter_map(|role| economy.tech_role.get(role).copied())
            .collect();
        if starting.is_empty() {
            return Ok(());
        }
        for player in 1..=u8::try_from(MAX_PLAYERS).unwrap_or(1) {
            if !self.entities.iter().any(|entity| entity.player == player) {
                continue;
            }
            for tech in &starting {
                self.grant_tech(player, *tech)?;
            }
            self.grant_availability(player)?;
        }
        Ok(())
    }

    /// The techs a player has researched. Empty for a player who has done nothing, and for an id
    /// outside `1..=MAX_PLAYERS`.
    #[must_use]
    pub fn researched(&self, player: u8) -> &BTreeSet<u32> {
        static NONE: BTreeSet<u32> = BTreeSet::new();
        self.researched.get(&player).unwrap_or(&NONE)
    }

    /// Hand the world its resource table. Without one, a cost cannot name a stockpile slot and
    /// spending has to fail rather than guess - which is the point of taking this in explicitly.
    ///
    /// The pack also decides *how many* resources exist, and a stockpile row is a fixed-size array,
    /// so a pack wider than the row is refused here: one refusal at load instead of a bounds check
    /// on every cost site, and no order can ever name a column that is not there.
    pub fn load_resources(&mut self, resources: &Resources) -> Result<(), SimError> {
        if resources.count() > MAX_RESOURCES {
            return Err(SimError::TooManyResources {
                count: resources.count(),
                max: MAX_RESOURCES,
            });
        }
        self.resources = Some(resources.clone());
        // The pack is what turns a name in `[start] stock` into a column, so this is the earliest
        // point the opening stockpile can be applied when the pack is loaded after seeding.
        self.apply_start_stock()?;
        Ok(())
    }

    /// The unit id behind a role name, from the loaded economy (`[role]` in economy.toml). This is
    /// how content names a unit without quoting the dat's numbering (D17).
    #[must_use]
    pub fn role_unit(&self, role: &str) -> Option<u32> {
        self.economy
            .as_ref()
            .and_then(|economy| economy.role.get(role).copied())
    }

    /// Is this unit the same *building* as the role names? True for the unit itself and for any
    /// variant an age upgraded it into, so a scenario that says `building = "barracks"` finds the
    /// feudal barracks (498) as well as the dark age one (12).
    pub fn is_family(&self, unit: u32, wanted: u32) -> bool {
        match self.units.as_ref() {
            Some(units) => units.root(unit) == units.root(wanted),
            None => unit == wanted,
        }
    }

    /// The name a tech is known by in the pack, when it has one. Used where a refusal has to name a
    /// tech: an id alone tells a player nothing, and the pack already has the word (D17).
    pub fn tech_role_name(&self, tech: u32) -> Option<&str> {
        self.economy.as_ref().and_then(|economy| {
            economy
                .tech_role
                .iter()
                .find(|(_, id)| **id == tech)
                .map(|(name, _)| name.as_str())
        })
    }

    /// Mark a tech as already researched, applying it exactly as a completed research would: the
    /// seeded state of a world that begins in a later age, and how a test that is about something else
    /// gets a prerequisite out of the way.
    pub fn grant_tech(&mut self, player: u8, tech: u32) -> Result<(), SimError> {
        let techs = self.techs.as_ref().ok_or(SimError::NoTables)?;
        if techs.get(tech).is_none() {
            return Err(SimError::MissingTech { id: tech });
        }
        self.finish_research(player, tech)
    }

    /// The tech a `[tech_role]` name stands for. Techs have no name in the dat, so this is the only
    /// way content - the scenario format, a UI - can name one without quoting an id.
    pub fn tech_role(&self, role: &str) -> Option<u32> {
        self.economy
            .as_ref()
            .and_then(|economy| economy.tech_role.get(role).copied())
    }

    fn role_unit_in(&self, economy: &Economy, role: &str) -> Result<u32, SimError> {
        economy
            .role
            .get(role)
            .copied()
            .ok_or_else(|| SimError::NoRole {
                role: role.to_owned(),
            })
    }

    /// The pack's resource table, when one has been loaded. A caller that wants to show what a
    /// player has asks the pack for its names rather than hardcoding them (D17).
    #[must_use]
    pub fn resources(&self) -> Option<&Resources> {
        self.resources.as_ref()
    }

    /// The stockpile slot for a resource's name, as the pack spells it. This is how a test, a
    /// scenario or a UI asks for "wood" without the engine knowing that word.
    #[must_use]
    pub fn slot_of(&self, name: &str) -> Option<usize> {
        self.resources
            .as_ref()
            .and_then(|resources| resources.slot_of(name))
    }

    /// The stockpile slot for a cost's resource id, resolved by the pack.
    ///
    /// `None` means this pack does not carry that id - a content mistake the caller must refuse,
    /// not a slot to guess at.
    #[must_use]
    pub fn resource_slot(&self, dat_id: u16) -> Option<usize> {
        self.resources
            .as_ref()
            .and_then(|resources| resources.slot_for_dat_id(dat_id))
    }

    /// How many waypoints remain for this entity, or `None` when it has no route. Exposed so tests
    /// can assert that bookkeeping is released when an entity dies.
    #[must_use]
    pub fn path_len(&self, id: EntityId) -> Option<usize> {
        self.paths.get(&id).map(Vec::len)
    }

    /// Canonical hash over the state. Encoding is explicit little-endian: no `usize`, no
    /// platform-dependent layout, so the number means the same thing everywhere.
    pub fn state_hash(&self) -> u64 {
        let mut hash = FNV_OFFSET_BASIS;
        fnv_bytes(&mut hash, &self.tick.to_le_bytes());
        // The civilisation is hashed because it changes what the world will allow: two runs that
        // differ in it refuse different techs, and a replay that lost it would diverge silently. Only
        // when set, so a world that names no civ hashes exactly as it did before civs existed.
        // What a player has ever built changes what the world will allow, so it is state (D63).
        for (id, queue) in &self.build_queue {
            fnv_bytes(&mut hash, b"build-queue");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            for (unit, at) in queue {
                fnv_bytes(&mut hash, &unit.to_le_bytes());
                fnv_bytes(
                    &mut hash,
                    &[at.0 as u8, (at.0 >> 8) as u8, at.1 as u8, (at.1 >> 8) as u8],
                );
            }
        }
        for (player, family) in &self.built {
            fnv_bytes(&mut hash, &[*player]);
            fnv_bytes(&mut hash, &family.to_le_bytes());
        }
        for (id, rally) in &self.rallies {
            fnv_bytes(&mut hash, b"rally");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            match rally {
                RallyPoint::Inside => fnv_bytes(&mut hash, &[1u8]),
                RallyPoint::Tile(x, y) => {
                    fnv_bytes(&mut hash, &[2u8]);
                    fnv_bytes(&mut hash, &x.to_le_bytes());
                    fnv_bytes(&mut hash, &y.to_le_bytes());
                }
            }
        }
        for (id, other) in &self.patrols {
            fnv_bytes(&mut hash, b"patrol");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            fnv_bytes(&mut hash, &other.0.raw().to_le_bytes());
            fnv_bytes(&mut hash, &other.1.raw().to_le_bytes());
        }
        for (villager, animal) in &self.slaughter {
            fnv_bytes(&mut hash, b"slaughter");
            fnv_bytes(&mut hash, &villager.0.to_le_bytes());
            fnv_bytes(&mut hash, &animal.0.to_le_bytes());
        }
        for (id, leash) in &self.leash {
            fnv_bytes(&mut hash, b"leash");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            fnv_bytes(&mut hash, &leash.home.0.raw().to_le_bytes());
            fnv_bytes(&mut hash, &leash.home.1.raw().to_le_bytes());
            fnv_bytes(&mut hash, &leash.until.to_le_bytes());
            fnv_bytes(&mut hash, &[u8::from(leash.chasing)]);
        }
        for (id, (stance, post)) in &self.stances {
            fnv_bytes(&mut hash, b"stance");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            fnv_bytes(
                &mut hash,
                &[match stance {
                    Stance::Aggressive => 0u8,
                    Stance::Defensive => 1,
                    Stance::StandGround => 2,
                    Stance::NoAttack => 3,
                }],
            );
            fnv_bytes(&mut hash, &post.0.raw().to_le_bytes());
            fnv_bytes(&mut hash, &post.1.raw().to_le_bytes());
        }
        for (id, task) in &self.resuming {
            fnv_bytes(&mut hash, b"resume");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            if let Task::Hunt { to } = task {
                fnv_bytes(&mut hash, &to.0.raw().to_le_bytes());
                fnv_bytes(&mut hash, &to.1.raw().to_le_bytes());
            }
        }
        for (id, waiting) in &self.queued {
            fnv_bytes(&mut hash, b"queued");
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            fnv_bytes(
                &mut hash,
                &u32::try_from(waiting.len())
                    .unwrap_or(u32::MAX)
                    .to_le_bytes(),
            );
            for job in waiting {
                fnv_bytes(&mut hash, &job.unit.to_le_bytes());
                fnv_bytes(&mut hash, &job.remaining_ticks.to_le_bytes());
            }
        }
        if self.teams_named {
            fnv_bytes(&mut hash, b"teams");
            fnv_bytes(&mut hash, &self.team_by_player);
        }
        // Who is in the match, hashed only when somebody is: a world built by hand for a movement
        // test has no players and must hash exactly as it did before rosters existed.
        // Fog is state: a player who has seen a tile knows it forever, so two runs that disagree
        // about the map they have uncovered have diverged. Hashed only when something has been seen,
        // so a world built by hand for a movement test hashes exactly as before.
        for (index, grid) in self.explored.iter().enumerate() {
            if grid.iter().all(|word| *word == 0) {
                continue;
            }
            fnv_bytes(&mut hash, b"explored");
            fnv_bytes(&mut hash, &[u8::try_from(index).unwrap_or(u8::MAX)]);
            for word in grid {
                fnv_bytes(&mut hash, &word.to_le_bytes());
            }
        }
        if self.roster.iter().any(|present| *present) {
            fnv_bytes(&mut hash, b"roster");
            for present in self.roster {
                fnv_bytes(&mut hash, &[u8::from(present)]);
            }
        }
        for (index, civ) in self.civ_by_player.iter().enumerate() {
            if let Some(civ) = civ {
                fnv_bytes(&mut hash, b"civ");
                fnv_bytes(&mut hash, &[u8::try_from(index).unwrap_or(u8::MAX)]);
                fnv_bytes(&mut hash, &civ.to_le_bytes());
            }
        }
        fnv_bytes(&mut hash, &(self.entities.len() as u32).to_le_bytes());
        for row in &self.stock {
            for value in row {
                fnv_bytes(&mut hash, &value.to_le_bytes());
            }
        }
        for (player, techs) in &self.researched {
            fnv_bytes(&mut hash, &[*player]);
            fnv_bytes(&mut hash, &(techs.len() as u32).to_le_bytes());
            for tech in techs {
                fnv_bytes(&mut hash, &tech.to_le_bytes());
            }
        }
        for (id, path) in &self.paths {
            fnv_bytes(&mut hash, &id.0.to_le_bytes());
            fnv_bytes(&mut hash, &(path.len() as u32).to_le_bytes());
            for (x, y) in path {
                fnv_bytes(&mut hash, &x.to_le_bytes());
                fnv_bytes(&mut hash, &y.to_le_bytes());
            }
        }
        for entity in &self.entities {
            fnv_bytes(&mut hash, &entity.id.0.to_le_bytes());
            fnv_bytes(&mut hash, &entity.player.to_le_bytes());
            let (tag, resource) = match entity.kind {
                EntityKind::Projectile => (b'p', 0u8),
                EntityKind::Building => (b'b', 0u8),
                EntityKind::Unit => (b'u', 0),
                // The column, not a kind: a pack that renumbers its resources renumbers this tag
                // too, which is what "the state changed" means for a node.
                EntityKind::Resource(slot) => (b'r', u8::try_from(slot).unwrap_or(u8::MAX)),
            };
            fnv_bytes(&mut hash, &[tag, resource]);
            fnv_bytes(&mut hash, &entity.amount.raw().to_le_bytes());
            fnv_bytes(&mut hash, &entity.carry.raw().to_le_bytes());
            fnv_bytes(&mut hash, &entity.speed.raw().to_le_bytes());
            match entity.researching {
                Some(job) => {
                    fnv_bytes(&mut hash, &[1u8]);
                    fnv_bytes(&mut hash, &job.tech.to_le_bytes());
                    fnv_bytes(&mut hash, &job.remaining_ticks.to_le_bytes());
                    fnv_bytes(&mut hash, &[u8::from(job.fresh)]);
                }
                None => fnv_bytes(&mut hash, &[0u8]),
            }
            match entity.training {
                Some(job) => {
                    fnv_bytes(&mut hash, &[1u8]);
                    fnv_bytes(&mut hash, &job.unit.to_le_bytes());
                    fnv_bytes(&mut hash, &job.remaining_ticks.to_le_bytes());
                    fnv_bytes(&mut hash, &job.hp.to_le_bytes());
                    fnv_bytes(&mut hash, &job.speed.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &[u8::from(job.fresh)]);
                }
                None => fnv_bytes(&mut hash, &[0u8]),
            }
            match entity.construction {
                Some(construction) => {
                    fnv_bytes(&mut hash, &[1u8]);
                    fnv_bytes(&mut hash, &construction.progress_ticks.to_le_bytes());
                    fnv_bytes(&mut hash, &construction.total_ticks.to_le_bytes());
                    fnv_bytes(&mut hash, &construction.max_hp.to_le_bytes());
                }
                None => fnv_bytes(&mut hash, &[0u8]),
            }
            match entity.task {
                Task::Idle => fnv_bytes(&mut hash, &[0u8]),
                Task::Move { to } => {
                    fnv_bytes(&mut hash, &[7u8]);
                    fnv_bytes(&mut hash, &to.0.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &to.1.raw().to_le_bytes());
                }
                Task::Hunt { to } => {
                    // Its own tag, not `Move`'s: two units walking to the same tile in the same
                    // position are doing the same thing only if one of them is not looking for a fight.
                    fnv_bytes(&mut hash, &[8u8]);
                    fnv_bytes(&mut hash, &to.0.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &to.1.raw().to_le_bytes());
                }
                Task::Garrisoned { building } => {
                    fnv_bytes(&mut hash, &[6u8]);
                    fnv_bytes(&mut hash, &building.0.to_le_bytes());
                }
                Task::Fly {
                    target,
                    shooter,
                    damage,
                    aim,
                    blast,
                } => {
                    fnv_bytes(&mut hash, &[5u8]);
                    fnv_bytes(&mut hash, &target.0.to_le_bytes());
                    fnv_bytes(&mut hash, &shooter.0.to_le_bytes());
                    fnv_bytes(&mut hash, &damage.to_le_bytes());
                    fnv_bytes(&mut hash, &aim.0.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &aim.1.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &blast.raw().to_le_bytes());
                }
                Task::Attack {
                    target,
                    damage,
                    reload_ticks,
                    cooldown_ticks,
                    range,
                    min_range,
                } => {
                    fnv_bytes(&mut hash, &[4u8]);
                    fnv_bytes(&mut hash, &target.0.to_le_bytes());
                    fnv_bytes(&mut hash, &damage.to_le_bytes());
                    fnv_bytes(&mut hash, &reload_ticks.to_le_bytes());
                    fnv_bytes(&mut hash, &cooldown_ticks.to_le_bytes());
                    fnv_bytes(&mut hash, &range.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &min_range.raw().to_le_bytes());
                }
                Task::Build { site } => {
                    fnv_bytes(&mut hash, &[3u8]);
                    fnv_bytes(&mut hash, &site.0.to_le_bytes());
                }
                Task::Return {
                    node,
                    drop,
                    slot,
                    rate_per_tick,
                    carry_cap,
                } => {
                    fnv_bytes(&mut hash, &[2u8]);
                    fnv_bytes(&mut hash, &node.0.to_le_bytes());
                    fnv_bytes(&mut hash, &drop.0.to_le_bytes());
                    fnv_bytes(&mut hash, &[u8::try_from(slot).unwrap_or(u8::MAX)]);
                    fnv_bytes(&mut hash, &rate_per_tick.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &carry_cap.to_le_bytes());
                }
                Task::Gather {
                    node,
                    rate_per_tick,
                    carry_cap,
                    progress,
                } => {
                    fnv_bytes(&mut hash, &[1u8]);
                    fnv_bytes(&mut hash, &node.0.to_le_bytes());
                    fnv_bytes(&mut hash, &rate_per_tick.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &carry_cap.to_le_bytes());
                    fnv_bytes(&mut hash, &progress.raw().to_le_bytes());
                }
            }
            match entity.variant {
                Some(variant) => {
                    fnv_bytes(&mut hash, &[1u8]);
                    fnv_bytes(&mut hash, &variant.to_le_bytes());
                }
                None => fnv_bytes(&mut hash, &[0u8]),
            }
            match entity.unit {
                Some(unit) => {
                    fnv_bytes(&mut hash, &[1u8]);
                    fnv_bytes(&mut hash, &unit.to_le_bytes());
                }
                None => fnv_bytes(&mut hash, &[0u8]),
            }
            fnv_bytes(&mut hash, &entity.position.0.raw().to_le_bytes());
            fnv_bytes(&mut hash, &entity.position.1.raw().to_le_bytes());
            fnv_bytes(&mut hash, &entity.velocity.0.raw().to_le_bytes());
            fnv_bytes(&mut hash, &entity.velocity.1.raw().to_le_bytes());
            fnv_bytes(&mut hash, &entity.hp.to_le_bytes());
        }
        hash
    }

    /// Draw from the simulation stream (accuracy, dispersion, conversion).
    pub fn sim_roll(&mut self) -> u32 {
        self.sim_rng.next_u32()
    }

    /// Draw from the map-generation stream. Separate state, so it cannot perturb `sim_roll`.
    pub fn map_roll(&mut self) -> u32 {
        self.map_rng.next_u32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruage_fixed::Fixed;

    const S: i64 = 20_000;

    const FIXTURE: &str = r#"
schema = 1
[sim]
tick_rate_hz = @TICK@
fixed_point_scale = 20000
dat_value_decimals = 4
per_tick_decimals = 3
max_entities = 4
state_hash_interval_ticks = 20
command_apply = "@APPLY@"
[determinism]
floats_in_sim_state = false
iteration_order = "entity_id"
rng_streams = ["sim", "map"]
[rng]
algorithm = "pcg32"
seed_source = "explicit"
[paths]
gamedata = "gamedata"
overrides = "overrides.toml"
"#;

    fn settings(tick_rate: u32, apply: &str) -> ruage_data::Sim {
        let text = FIXTURE
            .replace("@TICK@", &tick_rate.to_string())
            .replace("@APPLY@", apply);
        ruage_data::load_sim_str(&text, "fixture.toml").expect("test fixture must load")
    }

    #[test]
    fn one_second_is_tick_rate_ticks() {
        let mut w = World::new(&settings(20, "next_tick"), 1).unwrap();
        assert_eq!(w.tick(), 0);
        for _ in 0..20 {
            w.step().unwrap();
        }
        assert_eq!(w.tick(), 20);
    }

    #[test]
    fn movement_is_a_whole_number_of_units_per_tick() {
        // 1.35 tiles/s at 20 Hz is 1350 units per tick at scale 20000, and it must stay exact
        // for the whole second - no accumulated rounding.
        let mut w = World::new(&settings(20, "next_tick"), 1).unwrap();
        w.spawn_now(EntityId(1), Fixed::parse("1.35", S).unwrap(), 100);
        for _ in 0..20 {
            w.step().unwrap();
        }
        assert_eq!(w.position(EntityId(1)).unwrap().0.raw(), 27_000);
    }

    #[test]
    fn entities_are_ordered_by_id_not_insertion() {
        let mut w = World::new(&settings(20, "next_tick"), 1).unwrap();
        for id in [3, 1, 2] {
            w.spawn_now(EntityId(id), Fixed::ZERO, 10);
        }
        let ids: Vec<u32> = w.entities().iter().map(|e| e.id.0).collect();
        assert_eq!(ids, vec![1, 2, 3], "iteration order must be entity id");
    }

    #[test]
    fn state_hash_ignores_spawn_order() {
        let mk = |order: [u32; 3]| {
            let mut w = World::new(&settings(20, "next_tick"), 7).unwrap();
            for id in order {
                w.spawn_now(EntityId(id), Fixed::parse("0.96", S).unwrap(), 25);
            }
            for _ in 0..10 {
                w.step().unwrap();
            }
            w.state_hash()
        };
        assert_eq!(mk([1, 2, 3]), mk([3, 1, 2]));
        assert_eq!(mk([1, 2, 3]), mk([2, 3, 1]));
    }

    #[test]
    fn state_hash_notices_a_change() {
        let mut a = World::new(&settings(20, "next_tick"), 7).unwrap();
        let mut b = World::new(&settings(20, "next_tick"), 7).unwrap();
        for w in [&mut a, &mut b] {
            w.spawn_now(EntityId(1), Fixed::parse("1.35", S).unwrap(), 100);
        }
        b.spawn_now(EntityId(2), Fixed::ZERO, 10);
        assert_ne!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn same_inputs_give_the_same_trajectory() {
        let mut a = World::new(&settings(20, "next_tick"), 99).unwrap();
        let mut b = World::new(&settings(20, "next_tick"), 99).unwrap();
        for w in [&mut a, &mut b] {
            w.spawn_now(EntityId(1), Fixed::parse("1.54", S).unwrap(), 100);
            w.submit(Command::Spawn {
                id: EntityId(2),
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(83),
                position: (Fixed::ZERO, Fixed::ZERO),
                speed: Fixed::parse("0.8", S).unwrap(),
                hp: 25,
                amount: 0,
            });
        }
        let mut hashes = Vec::new();
        for _ in 0..100 {
            a.step().unwrap();
            b.step().unwrap();
            assert_eq!(
                a.state_hash(),
                b.state_hash(),
                "diverged at tick {}",
                a.tick()
            );
            hashes.push(a.state_hash());
        }
        assert!(
            hashes.windows(2).any(|w| w[0] != w[1]),
            "the hash must change as time passes"
        );
    }

    #[test]
    fn the_two_rng_streams_are_independent() {
        // Drawing map random numbers must not shift the simulation stream: that is the whole
        // point of separate streams.
        let mut a = World::new(&settings(20, "next_tick"), 5).unwrap();
        let mut b = World::new(&settings(20, "next_tick"), 5).unwrap();
        for _ in 0..50 {
            a.map_roll();
        }
        assert_eq!(a.sim_roll(), b.sim_roll());
        assert_ne!(
            a.sim_roll(),
            a.map_roll(),
            "the streams must not be the same sequence"
        );
    }

    #[test]
    fn commands_take_effect_on_the_next_tick() {
        let mut w = World::new(&settings(20, "next_tick"), 1).unwrap();
        w.step().unwrap();
        w.submit(Command::Spawn {
            id: EntityId(1),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(83),
            position: (Fixed::ZERO, Fixed::ZERO),
            speed: Fixed::ZERO,
            hp: 10,
            amount: 0,
        });
        assert!(
            w.entities().is_empty(),
            "the command must not be applied mid-tick"
        );
        w.step().unwrap();
        assert_eq!(w.entities().len(), 1);
    }

    #[test]
    fn immediate_commands_apply_on_submission() {
        let mut w = World::new(&settings(20, "immediate"), 1).unwrap();
        w.submit(Command::Spawn {
            id: EntityId(1),
            player: 1,
            kind: EntityKind::Unit,
            unit: Some(83),
            position: (Fixed::ZERO, Fixed::ZERO),
            speed: Fixed::ZERO,
            hp: 10,
            amount: 0,
        });
        assert_eq!(
            w.entities().len(),
            1,
            "command_apply = immediate acts at once"
        );
    }

    #[test]
    fn entities_beyond_max_are_refused() {
        let mut w = World::new(&settings(20, "next_tick"), 1).unwrap();
        for id in 1..=4 {
            w.spawn_now(EntityId(id), Fixed::ZERO, 10);
        }
        let err = w
            .spawn_now_checked(EntityId(5), Fixed::ZERO, 10)
            .expect_err("max_entities = 4 must be enforced");
        assert!(err.to_string().contains("max_entities"), "got: {err}");
    }

    #[test]
    fn a_spawn_in_the_same_tick_does_not_move_the_wrong_entities() {
        // `insert` keeps the list sorted by id, so a unit spawned during a tick shifts the index of
        // every entity after it. Applying this tick's movements by index alone would then move the
        // wrong units; the tick carries the id as a check for that reason. The whole suite passed
        // without this test, because nothing else both moves and spawns in one tick.
        // 20 Hz with the fixture's 20,000 scale: a raw speed of 100 is exactly 5 units per tick.
        let mut world = World::new(&settings(20, "next_tick"), 5).expect("world");
        world.spawn_now(EntityId(1), Fixed::from_raw(100), 10);
        world.spawn_now(EntityId(2), Fixed::from_raw(100), 10);
        let before = (
            world.entity(EntityId(1)).expect("1").position.0.raw(),
            world.entity(EntityId(2)).expect("2").position.0.raw(),
        );

        // A spawn with an id below both, queued for the same tick the two movers advance in.
        world
            .submit_checked(Command::Spawn {
                id: EntityId(3),
                player: 1,
                kind: EntityKind::Unit,
                unit: Some(83),
                position: (Fixed::ZERO, Fixed::ZERO),
                speed: Fixed::ZERO,
                hp: 25,
                amount: 0,
            })
            .expect("spawn");
        world.step().expect("step");

        for (id, was) in [(EntityId(1), before.0), (EntityId(2), before.1)] {
            let now = world.entity(id).expect("still there").position.0.raw();
            assert_eq!(
                now - was,
                5,
                "entity {} must move by its own delta, not by a neighbour's",
                id.0
            );
        }
    }

    #[test]
    fn the_world_resolves_resource_slots_through_the_pack() {
        // D17's boundary, one step closer: the world does not decide what slot wood is, the pack
        // does. Until this existed the mapping lived next to the engine's own enum.
        let mut world = World::new(&settings(20, "next_tick"), 3).expect("world");
        assert_eq!(
            world.resource_slot(1),
            None,
            "with no table loaded, nothing may be guessed"
        );

        let resources = ruage_data::load_resources_file(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/resources.toml"),
        )
        .expect("the project's resource table");
        world
            .load_resources(&resources)
            .expect("the pack fits the stockpile");
        assert_eq!(world.resource_slot(1), Some(0), "dat 1 is wood, slot 0");
        assert_eq!(world.resource_slot(0), Some(1), "dat 0 is food, slot 1");
        assert_eq!(
            world.resource_slot(99),
            None,
            "an id the pack does not carry"
        );
    }
}
