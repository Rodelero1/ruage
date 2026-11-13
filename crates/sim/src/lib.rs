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

use ruage_data::{
    CommandApply, Cost, CostResource, Economy, Effects, Map, Policy, Resources, Sim as Settings,
    Unit, Units,
};

mod metrics;
mod path;
mod roles;
mod short;
pub mod tiles;
mod world;
pub use metrics::Metrics;
pub use path::{
    Walk, find_path, find_path_avoiding, line_clear, line_clear_avoiding, nearest_reachable,
};
use ruage_fixed::Fixed;
pub use short::short_range;

/// The largest number of players a match can have: eight, so four-versus-four (D20). It is a
/// constant rather than a runtime number because stockpiles are fixed-size arrays, and it is eight
/// rather than two because the cost of widening is paid once and the cost of a later re-refactor is
/// paid in every state hash.
/// The most routes one tick may re-plan because bodies are in the way (D98a). A **performance budget**, not a game
/// value - the same kind of constant as `MAX_PLAYERS`' neighbour `max_entities`: a crowd in which every unit is blocked
/// would otherwise buy one path search per unit per tick, and measurably turned a 40,000-tick fixture into minutes.
/// **The finest scale a map of `tiles` can use before squared distances overflow `i64`** (D128). The arithmetic is
/// `2 * ((tiles - 1) * scale)^2 <= i64::MAX`, solved for `scale` with integer square roots so the answer is exact.
#[must_use]
pub fn max_scale_for(tiles: u64) -> i64 {
    let span = tiles.saturating_sub(1).max(1);
    // scale^2 <= i64::MAX / 2 / span^2
    let limit = (i64::MAX as i128) / 2 / (i128::from(span) * i128::from(span));
    if limit <= 0 {
        return 0;
    }
    // **Integer square root, no float**: `no_float` is right about this crate, and a float here would be the first one
    // back into it since D9.
    let mut root = integer_sqrt_i128(limit);
    while (root + 1) * (root + 1) <= limit {
        root += 1;
    }
    while root * root > limit {
        root -= 1;
    }
    i64::try_from(root).unwrap_or(i64::MAX)
}

/// **How many times one route may be re-planned because its ground went stale** (D127). Two is enough for a house built across
/// a lane and one more obstruction; more turns a hopeless goal into a search every tick.
pub(crate) const MAX_GROUND_REPLANS: u8 = 2;

/// A unit the budget leaves out simply waits this tick, which is what a blocked unit does anyway, and tries again next.
pub(crate) const MAX_REPATHS_PER_TICK: usize = 4;

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

/// How near an existing drop site makes another pointless, when content says nothing: the reference implementation's
/// `kCovered`, which mirrors the original AI's own placement rule.
const BUILD_SPACING_DEFAULT: u32 = 5;

/// One damage per second, in the integer millionths `pierce_dps` counts in (D67).
const DPS_ONE: i64 = 1_000_000;

/// What a command-card button would do: make a unit, or research a tech.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardTarget {
    Unit(u32),
    Tech(u32),
}

/// **One button on the command card.** It is a query over the same tables an order is validated against, so a button can
/// never be offered for an order the engine would refuse: the price comes from the unit or tech, the name from the pack's
/// own vocabulary (`[role]`, `[tech_role]`), and `refusal` is `check_train`/`check_research`'s own answer. That is what
/// "moddable" means for a UI (the owner's D1): a pack with a new building and a new unit gets a new button without the
/// engine learning either word.
#[derive(Debug, Clone, PartialEq)]
pub struct CardItem {
    pub target: CardTarget,
    /// The pack's word for it, or `"unit 83"` when the pack names no role - a button with no label is worse than a button
    /// with an id.
    pub name: String,
    /// `(stock column, amount)`, in the pack's column order. Empty when the thing is free.
    pub costs: Vec<(usize, u32)>,
    /// `None` when the order would be accepted right now; otherwise why not, in the engine's words.
    pub refusal: Option<String>,
}

/// A villager's waiting sites: unit id and tile, oldest first (D66).
type BuildQueue = BTreeMap<EntityId, VecDeque<(u32, (u16, u16))>>;

/// A tile found by a nearest-first search: its sort key - squared distance, then the tile, so the answer is
/// deterministic - and the tile itself.
type NearKey = ((u32, u16, u16), (u16, u16));

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
    /// **What the movement pass decided for this body on its last tick** (D196). Instrumentation: not part of the state hash, and
    /// written by the pass itself so reading it costs a lookup only when something asks (a test, a dump, a bug report).
    pub last_step: StepDecision,
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
pub(crate) fn scale_fixed(base: Fixed, scale: i64, modifiers: &[&ruage_data::Modifier]) -> Fixed {
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
pub(crate) fn scaled_health(unit: &Unit, modifiers: &[&ruage_data::Modifier]) -> i32 {
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
pub(crate) fn add_to_class(list: &mut Vec<(i16, i16)>, class: i16, amount: i16) {
    match list.iter_mut().find(|(entry, _)| *entry == class) {
        Some((_, value)) => *value = value.saturating_add(amount),
        None => list.push((class, amount)),
    }
}

/// Multiply a class's value, rounded half away from zero: a multiplier on a class the unit does not
/// have is a no-op, because there is nothing to scale.
pub(crate) fn multiply_class(list: &mut [(i16, i16)], class: i16, factor: f64) {
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

/// Why a building's production queue is stopped (D85).
///
/// Engine state, never words: the view owns the wording (`docs/ui.md`), so a mod changes what the player reads without
/// touching the simulation. It is a fact about *now* rather than an event - set while the reason holds, cleared the
/// tick it stops holding, which is what lets the panel draw it for exactly as long as it is true.
///
/// `Unaffordable` is in `docs/ui.md`'s plan and is **not** here: the queue pays when the order is placed, so no code
/// path can produce it today, and a variant no test can drive is a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainingPause {
    /// Nowhere to stand beside the building and no room inside it to shelter the recruit.
    NoRoom,
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

/// What one player last observed of one unit type: how many were in sight, and the tick that sighting happened.
///
/// This is a **memory**, not a view (D92): the count is what was there when it was seen, and the stamp does not move
/// until something is seen again, so a rule can decide for itself how stale is too stale. Nothing reads the world's
/// real state through it, which is what makes an unscouted army genuinely unknown rather than merely distant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sighting {
    pub count: u16,
    pub at: u64,
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
pub(crate) struct Landing {
    shooter: EntityId,
    target: EntityId,
    damage: i32,
    aim: (Fixed, Fixed),
    blast: Fixed,
}

/// A shot leaving the world this tick: who loosed it, from where, at what, and for how much. Recorded
/// rather than spawned in place, because the entity list is index-stable for the length of the pass.
pub(crate) struct Shot {
    player: u8,
    shooter: EntityId,
    from: (Fixed, Fixed),
    aim: (Fixed, Fixed),
    target: EntityId,
    damage: i32,
    blast: Fixed,
}

/// A training order that passed every check: what it costs, how long it takes, and what comes out.
pub(crate) struct TrainPlan {
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

/// **One job in a building's queue** (D157): a unit to train, or a tech to research. Exactly one runs at a time and the rest
/// wait in the order they were ordered - the owner's rule: *"Units and tech can be queued in the building, but they are
/// performed one after another not in parallel."*
#[derive(Debug, Clone)]
pub(crate) enum Job {
    Train(Training),
    Research { tech: u32 },
}

/// What a building is making, as the interface asks for it (D157): the job in progress with its progress in ticks, and the
/// ones waiting behind it, in order.
///
/// One call rather than three, because a HUD that asked `training`, `researching` and the queue separately would draw three
/// pictures that can disagree.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Production {
    /// The running job, how many of its ticks are done, and how many there are in total.
    pub running: Option<(QueuedItem, u32, u32)>,
    /// The jobs behind it, in the order they will run.
    pub waiting: Vec<QueuedItem>,
}

/// A unit or a tech, named the way the queue holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueuedItem {
    Unit(u32),
    Tech(u32),
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
///
/// **Not `Copy`** since D155: a wall drag carries its tiles, and a line of a hundred segments should be handed over once rather
/// than copied on every pass. `Clone` stays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Take from a resource node. The rate, the capacity and the variant come from the tables,
    /// not from the command: a replay must not be able to name its own numbers.
    Gather {
        villager: EntityId,
        node: EntityId,
    },
    /// Stand a unit down: drop its path and return it to idle.
    Stop {
        unit: EntityId,
    },
    /// Raze something of your own. The player is named because ownership is the only thing there is
    /// to check: every other command inherits its owner from the actor, and a deletion has no actor.
    Delete {
        player: u8,
        entity: EntityId,
    },
    /// Walk to a tile and stop. The ordinary order, and the one the rally point uses.
    /// Walk to a **point**, not to a tile's centre (A). The original's movement is free of the grid - its waypoints and
    /// goal points are floats - while tiles are for terrain, resource and building placement. A `Move` names a tile and
    /// means its centre; this names a place.
    WalkTo {
        unit: EntityId,
        to: (Fixed, Fixed),
    },
    Move {
        unit: EntityId,
        to: (u16, u16),
    },
    /// Walk to a tile, fighting whatever enemy it meets on the way.
    AttackMove {
        unit: EntityId,
        to: (u16, u16),
    },
    /// Walk to a tile, then back to where the order was given, forever - fighting on the way.
    Patrol {
        unit: EntityId,
        to: (u16, u16),
    },
    /// How much of a fight this unit picks on its own. The ground it stands on becomes its post, which
    /// is what a defensive stance returns to.
    Stance {
        unit: EntityId,
        stance: Stance,
    },
    /// Attack another player's unit or building.
    Attack {
        attacker: EntityId,
        target: EntityId,
    },
    /// Produce a unit at a building. Cost and duration come from the unit table; whether this
    /// building may make this unit comes from `economy.toml` (the dat does not carry that edge).
    Train {
        building: EntityId,
        unit: u32,
    },
    /// Put up a **line** of buildings - walls, in practice (D155): the tiles a drag covered, in order.
    ///
    /// The list, not a start-and-end pair, because the view already has to compute the line to draw its preview and its red
    /// ineligible tiles - and two implementations of "where does the wall go" would disagree. [`World::wall_line`] is that
    /// one implementation, and the view calls it.
    ///
    /// A tile that cannot be built on is **skipped**, not fatal: the rest of the drag still stands (the owner's answer B).
    BuildLine {
        villager: EntityId,
        unit: u32,
        tiles: Vec<(u16, u16)>,
    },
    /// Put up a building at a tile. Cost and duration come from the unit table.
    Build {
        villager: EntityId,
        unit: u32,
        at: (u16, u16),
    },
    /// **Walk a villager onto a foundation that already stands** (D186 addendum 3): the gesture the original makes when a
    /// crowd is clicked onto a half-built site, kept as its own command because `Build` also means *place*, and a placement
    /// over an existing site must stay refused.
    JoinBuild {
        villager: EntityId,
        site: EntityId,
    },
    /// Research a tech at a building. Price, duration, where it may happen and what it applies all
    /// come from the tech and effect tables - a replay carries the id, never the numbers.
    Research {
        building: EntityId,
        tech: u32,
    },
    /// Where a building's finished units go. `Some(tile)` walks them there; `Some` of the building's own
    /// tile keeps them inside it, which is the only way a production building holds anybody (D38);
    /// `None` clears the rally and leaves them beside the building.
    Rally {
        building: EntityId,
        point: Option<(u16, u16)>,
    },
    /// Put a unit inside a building. The capacity is the building's own `garrison` field, so the only
    /// numbers involved are the data's.
    Garrison {
        unit: EntityId,
        building: EntityId,
    },
    /// Let everyone out of a building.
    Ungarrison {
        building: EntityId,
    },
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

mod error;
pub use error::{ErrorClass, SimError};

/// **What the movement pass decided for one body, this tick** (D193). `collide` decides in nine ordered rules and, before this, its
/// answer was a pair of deltas - so "why is this man standing still" had no value to print, test or compare, and answering it took two
/// hours of printing from inside the function, twice in one week.
///
/// Instrumentation, not state: nothing reads it in the tick and it is not hashed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StepDecision {
    /// Nothing was asked of this body.
    Idle,
    /// The step was taken: ground clear, no body in the way.
    Take,
    /// The step was taken over a body the mover is already inside (D52).
    TakeOverlapping,
    /// The step was taken into a body at rest, whose repath routes round it from the next tick (rule 8).
    TakeAtRest,
    /// The step was slid along one clear axis: ground or a body on the planned line.
    Slide,
    /// The step was made by nudging once per route (rule 6).
    MakeRoom,
    /// The step was taken because the blocker refused one at this body on the same tick (rule 9).
    GiveWay,
    /// The same, when the blocker stood on the mover's own tile (rule 4).
    GiveWayOwnTile,
    /// Refused: the route's waypoint is blocked by ground, and a re-plan was asked for (D127).
    WaitGround,
    /// Refused: the blocker is the body the order was given to (rule 2).
    WaitTarget,
    /// Refused: a **hostile** body stands on the tile the order asked for (rule 2, D131/D192).
    WaitGoal,
    /// Refused: a moving body stands on the mover's own tile (rule 4).
    WaitOwnTile,
    /// Refused: anything else in the way (rule 9).
    WaitBlocked,
}

impl StepDecision {
    /// Every variant, in declaration order: the counter array is indexed by this.
    pub const ALL: [Self; 13] = [
        Self::Idle,
        Self::Take,
        Self::TakeOverlapping,
        Self::TakeAtRest,
        Self::Slide,
        Self::MakeRoom,
        Self::GiveWay,
        Self::GiveWayOwnTile,
        Self::WaitGround,
        Self::WaitTarget,
        Self::WaitGoal,
        Self::WaitOwnTile,
        Self::WaitBlocked,
    ];

    /// This variant's slot in the counter array.
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|other| *other == self)
            .unwrap_or(0)
    }

    /// Whether the body was told to stand still this tick.
    pub fn is_a_wait(self) -> bool {
        matches!(
            self,
            Self::WaitGround
                | Self::WaitTarget
                | Self::WaitGoal
                | Self::WaitOwnTile
                | Self::WaitBlocked
        )
    }

    /// Whether the refusal was "somebody stands on the tile you were sent to" - the clause D192 made hostile-only.
    pub fn waited_for_a_goal_tile(self) -> bool {
        self == Self::WaitGoal
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

/// One tick of movement from `from` toward `to`, in raw fixed units on each axis: **a step along the line**.
///
/// The old shape clamped each axis to its own remaining gap, which walks every heading that is not a multiple of 45
/// degrees as a 45-degree leg followed by a cardinal one - the owner's report, three times over, the third with a
/// screenshot ("the path is not a straight line but instead 45 degree walk followed by horizontal walk"). The step is now
/// the same fraction of both gaps, so the unit's positions lie on the segment it was given, and the walk is as long as
/// the line rather than as long as the line's Manhattan projection (a diagonal used to be sqrt(2) faster).
///
/// Two details are load-bearing:
/// 1. **When the remainder is within one step the unit lands exactly on the point.** Every arrival rule in this engine -
///    `position == waypoint` to spend a waypoint, `position == target` for a node - is an equality, and a proportional
///    step that stopped a rounding unit short would stall all of them.
/// 2. It is all integer arithmetic (`integer_sqrt`, an `i128` product), because a float in the tick is the desync the
///    project forbids and the talk's own warning about pathing precision.
pub(crate) fn step_towards(from: (Fixed, Fixed), to: (Fixed, Fixed), delta: i64) -> (i64, i64) {
    let gap_x = to.0.raw() - from.0.raw();
    let gap_y = to.1.raw() - from.1.raw();
    // **Nothing moves on a zero step**, including the one-raw-unit floor below: a unit the pack gives no speed would
    // otherwise creep a raw unit a tick for the rest of the match (found by `a_unit_with_no_speed_stays_where_it_is`).
    if (gap_x == 0 && gap_y == 0) || delta <= 0 {
        return (0, 0);
    }
    let squared = gap_x * gap_x + gap_y * gap_y;
    let length = integer_sqrt(squared);
    if length <= delta.max(0) {
        return (gap_x, gap_y);
    }
    // **Rounded, not truncated, and never past the point.** Truncation is what froze a villager against a house: its line
    // carried a 20-raw-unit remainder on one axis, every step rounded that axis to zero, and a purely axis-aligned step
    // whose landing tile was blocked then had no slide to fall back on (`a_villager_walks_round_a_house_rather_than_through_it`,
    // 2026-11-04). Rounding keeps both axes moving, which is also what gives the slide in `collide` something to work with.
    // The clamp to the gap means no axis can overshoot, so a point is still landed on exactly.
    let component = |gap: i64| -> i64 {
        if gap == 0 {
            return 0;
        }
        let magnitude = i128::from(gap).abs() * i128::from(delta);
        let rounded = (magnitude + i128::from(length / 2)) / i128::from(length);
        // **No axis is ever abandoned.** A component that rounds to zero leaves the unit with a purely axis-aligned step,
        // and an axis-aligned step into a wall has no slide to fall back on - that is what froze a villager a tenth of a
        // raw unit short of the row its goal was in. One raw unit per tick closes any remainder, and the cap keeps it
        // from passing the point it aims at.
        let capped = rounded.max(1).min(i128::from(gap.unsigned_abs()));
        i64::try_from(capped)
            .unwrap_or_else(|_| i64::try_from(gap.unsigned_abs()).unwrap_or(i64::MAX))
            * gap.signum()
    };
    let (step_x, step_y) = (component(gap_x), component(gap_y));
    // A step that rounded to nothing on both axes would freeze the unit short of its point.
    if step_x == 0 && step_y == 0 {
        return (gap_x.signum(), gap_y.signum());
    }
    (step_x, step_y)
}

/// The nearest free walkable tile around `from`, searched in ring order so the choice is
/// deterministic and independent of any hash iteration.
pub(crate) fn free_neighbour(
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
pub(crate) struct Rot {
    per_tick: i64,
    expires: u64,
}

/// A unit's remaining route, and the bodies it is planned around (D98a).
///
/// The avoid set lives **with** the route rather than in a table keyed by unit, so a new order cannot inherit the old
/// plan's blind spots: replacing the route replaces the set. A collision with a unit in this set is *past* - the plan
/// already accounts for it - which is exactly what stops a blocked unit from repathing every tick.
/// A repath collected in the movement pass and applied after it (D98a): the unit, the body that was in the way, and the
/// tile that body occupies. The blocked tile is the body's own, so it is not stored twice.
pub(crate) struct Repath {
    pub(crate) unit: EntityId,
    /// The body the route must now avoid, or `None` when the obstruction is **ground** - a stale route whose waypoints
    /// have turned into blocked tiles (D127). Nothing is added to the avoid set in that case.
    pub(crate) body: Option<EntityId>,
    pub(crate) goal: (u16, u16),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Route {
    /// Remaining waypoints, **as points**: the geometry's corners (D113), or the centres of the tiles a search returned when
    /// the geometry declined. The last point is the goal itself, so an order between tiles lands where it pointed (A).
    pub(crate) points: Vec<(Fixed, Fixed)>,
    /// Units this route was planned around. Not hashed on its own: it is part of the route, and the route is hashed.
    pub(crate) avoiding: BTreeSet<EntityId>,
    /// This route has already **made room** once (C1b). A nudge is a way past one obstruction, not a way of life: the
    /// second obstruction on the same route is answered by refusing the step and re-planning, or a crowded base becomes
    /// villagers stepping aside for each other and never arriving.
    pub(crate) gave_way: bool,
    /// How many times this route has been re-planned because its **ground** went stale. Capped: a route whose goal is buried
    /// behind new walls would otherwise search again on every tick, which is the cost D98a's "past" rule exists to avoid.
    pub(crate) stalls: u8,
}

/// What one tick of the movement pass carries between steps: the bodies that can be in the way, who has a step to take,
/// and the reactions it collects for **after** the loop (a repath plans a route and must not be run while the entity list
/// is being walked; a nudge writes a flag the pass reads).
pub(crate) struct Passing<'a> {
    pub(crate) bodies: &'a [(i64, i64, EntityId, usize, i64)],
    /// Who has something to do this tick. A body at rest cannot move aside for anybody, which is the difference between
    /// waiting a tick and waiting for ever.
    pub(crate) moving: &'a BTreeSet<EntityId>,
    pub(crate) repaths: Vec<Repath>,
    pub(crate) made_room: Vec<EntityId>,
    /// `(mover, blocker)` for every refused step this tick, in entity order: a pair that refused at each other is a jam.
    pub(crate) refused: Vec<(EntityId, EntityId)>,
}

/// **What a unit's card reads** (2026-11-04): the numbers the bottom-central panel puts beside a portrait.
///
/// Fractional quantities are integer **hundredths** - a speed of 96 is 0.96 tiles a second, a range of 400 is four tiles -
/// so no float crosses this boundary. Attack and armour are hundredths too, for one shape rather than two. Hit points and
/// capacity are counts, because that is what they are.
/// **What a worker's target is**, for the panel's job name (D121): the pack's role for it, and the resource column it
/// holds. Either may be absent - a tree has a column and no role, a building site has neither - and the content decides
/// which word that means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkTarget {
    /// The target's role name in the pack's economy table (`"farm"`, `"sheep"`), when it has one.
    pub role: Option<String>,
    /// The resource column the target holds, when it holds one.
    pub column: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitStats {
    pub max_hp: i32,
    /// The larger of class 3 (pierce) and class 4 (melee); a bonus against a unit class is not the card's number.
    pub attack: i32,
    /// Class 4: the `0` before the slash.
    pub armour_melee: i32,
    /// Class 3: the `0` after it.
    pub armour_pierce: i32,
    pub range: i32,
    pub speed: i32,
    pub los: i32,
    pub work_rate: i32,
    pub carry: i32,
}

pub struct World {
    pub(crate) tick: u64,
    pub(crate) tick_rate_hz: u32,
    pub(crate) scale: i64,
    max_entities: u32,
    command_apply: CommandApply,
    pub(crate) entities: Vec<Entity>,
    queue: Vec<Command>,
    /// Cloned at seed time so a command can be resolved without threading references through the
    /// tick loop. A replay carries ids, never numbers.
    pub(crate) units: Option<Units>,
    pub(crate) economy: Option<Economy>,
    /// Per player, per resource. Index is `player - 1`; player 0 has no stockpile and neither does
    /// anything above `MAX_PLAYERS`. The width is a constant because the row is an array, and the
    /// pack is checked against it when it is loaded - so a slot is always inside the row.
    stock: [[u32; MAX_RESOURCES]; MAX_PLAYERS],
    /// Terrain, copied once at seed time. Static for a match.
    pub(crate) walk: Option<Walk>,
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
    /// A route and the bodies it was planned around (D98a).
    pub(crate) paths: BTreeMap<EntityId, Route>,
    /// **How many consecutive ticks a body has been refused by another body** (D200). Not hashed: it is a count of waiting,
    /// not a position, and a route that is stalled for ever shows up in behaviour rather than in the state hash.
    pub(crate) stalled: BTreeMap<EntityId, u32>,
    /// How many times a blocked unit has re-planned its route. **Instrumentation, not state**: nothing reads it and it
    /// is not hashed; a test counts it to prove a known obstacle is not re-planned every tick (D98a).
    repath_count: u64,
    /// The **last movement decision** for each body, and how many of each have been made (D193). Instrumentation, like the
    /// counters above: never hashed, never read by the tick. `step_trace` was the review's name for it.
    /// Counted per decision, in a flat array: a `BTreeMap` here cost **1.8 µs a tick** at 128 bodies (6.7 -> 8.5), which is a third
    /// of the tick for instrumentation. The per-body trace lives on the entity (`Entity::last_step`) for the same reason.
    step_counts: [u64; 13],
    /// **Whether the movement trace is being written** (D196). Off by default: the per-body record and the counters cost **0.4 us of a
    /// 6.7 us tick** (about 6%), which is a price the tick should not pay for a diagnostic nobody asked for. A test, a dump or a bug
    /// report turns it on and pays it for that run.
    tracing_steps: bool,
    /// How many of `repath_count` were forced by a **body** rather than by ground (D193 measurement). Instrumentation, like
    /// the total: never hashed, never read by the tick. It exists because D192 met a 70% rise in repaths and could say only
    /// that it had happened.
    repath_by_body: u64,
    /// How many were forced by ground: a route whose waypoint is now blocked by terrain, a gate or a fresh building.
    repath_by_ground: u64,
    /// How many ground re-plans **failed**. A counter, like `repath_count`, and not state: it exists so a test can say
    /// "this unit tried twice and stopped" instead of "this unit searched all match" (D135).
    repath_failures: u64,
    /// How many times a step made room instead of being refused (C1b). A counter, like `repath_count`, because it is a
    /// reaction the engine gives itself and has no caller to report to.
    made_room_count: u64,
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
    pub(crate) animals: BTreeSet<u32>,
    /// The smaller list: animals that walk to a drop site when a villager works them, from
    /// `[hunt] herdable_roles`. A boar is food but not herdable (D70).
    pub(crate) herdable: BTreeSet<u32>,
    /// Bodies: what a slaughtered animal becomes in place, from `[hunt] carcass_roles`. A carcass is gathered
    /// from, and it is a named unit rather than an inference from having no health (D70).
    pub(crate) carcasses: BTreeSet<u32>,
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
    /// **Empty farms, and when each one emptied** (the owner, 2026-11-04: *"vanishing takes some time, husk first"*). A husk
    /// blocks nothing, holds nothing and cannot be worked; it is kept for `husk_ticks` so a player can see the spent plot and
    /// click a villager onto it to plant a new farm, and then removed.
    husks: BTreeMap<EntityId, u64>,
    /// How long a husk stays, in ticks. Zero for a pack with no farms.
    husk_ticks: u64,
    /// **The chopped remainder, in thousandths of a hit point, per tree** (D129). A chop of 1.5 hit points a second at
    /// 20 Hz is 75 thousandths a tick: without a carry the tick could only take whole points and every tree would fall in
    /// one blow. State, because it decides when the next point comes off.
    fell_carry: BTreeMap<EntityId, i64>,
    /// A worker the shares have decided to move to another column, and the node waiting for it: the load in its hands is
    /// banked **first**, at its own drop site, and only then does it take up the new work (the owner's rule, 2026-10-30).
    pub(crate) pending_gather: BTreeMap<EntityId, EntityId>,
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
    /// **A man's place on his node's ring** (D186 addendum 5): worker -> (node, slot). The settle used to *choose* a place
    /// every tick, so two men walking to the same side of a bush swapped places and ended up on top of each other - measured
    /// **0.001**, **0.20** and **0.243** tiles apart in three different fixtures. A place chosen once and kept is a place
    /// nobody else can take, and a man walks to it rather than to whatever is nearest this tick.
    ///
    /// Bookkeeping, and not hashed: it is re-derived from the tasks every time it is read (`place_is_live`), so a re-tasked
    /// man's place dies with the order rather than needing a sweep.
    gather_place: BTreeMap<EntityId, (EntityId, (i64, i64))>,
    /// **Sites that could not start because somebody was standing on them** (D183 §4). Bookkeeping, not simulation state:
    /// it exists so the refusal goes out **once per state change** rather than once a tick, and like `foundations` and
    /// `open_gates` it is derived from the world every tick, so it is not hashed. A site leaves the set the moment its
    /// ground is clear - which is what makes a second stall a second message.
    stalled_sites: BTreeSet<EntityId>,
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
    /// **Units that were rallied into a building but appeared too far away to enter it**, and the building each is walking
    /// to. A garrison order is refused for a unit that is not at the door, so a rally to "inside" used to shelter only a
    /// unit that happened to be born touching the building and silently idled the rest - found when the owner's opening
    /// scout made the town centre's free tiles crowded enough for a finished villager to appear two tiles out.
    entering: BTreeMap<EntityId, EntityId>,
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
    pub(crate) explored: [Vec<u64>; MAX_PLAYERS],
    /// Per player, what they have watched of other players' units, by unit id (D92). `BTreeMap` because iteration
    /// order is hashed and a hash map would make the state hash depend on insertion order.
    pub(crate) memory: [BTreeMap<u32, Sighting>; MAX_PLAYERS],
    /// Which players keep a memory at all. **Installed by the match, not by the loader** - a bare world stays bare, so
    /// a test that never asks for a memory pays nothing for it; the rules of a scenario switch it on for the players
    /// they belong to.
    pub(crate) observers: [bool; MAX_PLAYERS],
    /// The map's size in tiles, from the content the world was seeded with. Zero until seeding.
    pub(crate) map_size: (u16, u16),
    /// Unit ids (family roots) that block movement once built, from `[build] blocking_roles` in content.
    /// A wall is the only one this pack names, and the engine learns the word from the pack (D54).
    blocking: BTreeSet<u32>,
    /// Family roots that are **gates** (D154), from `[build] gate_roles`: a building whose tiles are walkable while it is
    /// open. Content names them, so the engine learns no dat class.
    gates: BTreeSet<u32>,
    /// Which gates are open right now, so the walk grid is unblocked exactly once per opening and re-blocked once per
    /// closing - `Walk` counts occupants, and an unbalanced pair leaves a tile blocked for ever (or free for ever).
    open_gates: BTreeSet<EntityId>,
    /// Family root → the solid cells of that building's footprint (D76). A role with no entry blocks its own tile.
    footprints: BTreeMap<u32, ruage_data::Footprint>,
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
    /// **One queue per building, jobs in order** (D157): units and techs together, so a tech ordered after two units runs
    /// third. The running job is not in here - it lives on the entity as `training` or `researching`, because those are the
    /// timers the tick loop reads.
    queued: BTreeMap<EntityId, Vec<Job>>,
    /// Why a building's production is stopped, by building (D85). A side table for the same reason as `queued`: it is
    /// a fact about a handful of buildings, not a field every entity pays for. Cleared on the tick production can
    /// continue, so a stale reason can never outlive its cause.
    training_pauses: BTreeMap<EntityId, TrainingPause>,
    /// Per player, the techs already researched. Part of state: it decides what a later upgrade
    /// does and what a later order resolves to, so it is hashed.
    researched: BTreeMap<u8, BTreeSet<u32>>,
    /// The pack's resource table, when one has been loaded. Costs carry the reference pack's ids and
    /// nodes carry columns; `slot_for_dat_id` and `name` are the only places the two are related, so
    /// this is the field every cost and every gather order reads (D17).
    resources: Option<Resources>,
    /// Each resource's node half-extent in raw units, resolved at load from the pack's `radius` (D169).
    node_radii: Vec<i64>,
    sim_rng: Pcg32,
    map_rng: Pcg32,
}

/// The eight compass directions a scattered shot can land in, as raw multipliers of the scatter distance.
/// Diagonals are therefore a little farther than a `sqrt(2)` would make them: the scatter is a distance
/// error, not a geometry, and integer octants keep a replay exact (D43).
pub(crate) const OCTANTS: [(i64, i64); 8] = [
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
pub(crate) fn splitmix64(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Whether `here` is close enough to `goal` to call it arrived: within one step's length, so the check cannot be stricter than
/// the movement it judges. The last step of a walk is often clipped by the ground or by a body, and an exact-equality test left
/// a unit pacing at its own doorstep for ever - which is how a barracks was never finished (2026-11-04).
pub(crate) fn reaches(here: (Fixed, Fixed), goal: (Fixed, Fixed), delta: i64) -> bool {
    let (dx, dy) = (goal.0.raw() - here.0.raw(), goal.1.raw() - here.1.raw());
    let reach = delta.max(1);
    dx * dx + dy * dy <= reach * reach
}

/// The integer square root, for the one place that needs a distance rather than a squared distance: a
/// shot's time of flight. Newton's method on integers, so it is exact and identical on every machine.
/// Integer square root of a non-negative `i128`, by Newton's method: the largest `r` with `r * r <= value`.
pub(crate) fn integer_sqrt_i128(value: i128) -> i128 {
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
pub(crate) fn collect_modifiers<'a>(
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

/// What `standoff_point` found: a place to stand, or **no room on the ring** (D182) - the owner's *"if there is no space at half diameter from
/// another villager and the resource try another nearest resource of the same kind"*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Standoff {
    /// A point on the ring: the two bodies touch there and no other body holds it.
    Point((Fixed, Fixed)),
    /// Touching is possible, but every side is held by another body or cannot be walked to.
    NoRoom,
}

/// **The gather ring's radius**, in thousandths of a tile (D186) - measured, not derived.
///
/// The owner's second save (`owner-test2.aoe2spgame`) carries the standing positions of six woodcutters round one tree:
/// **1.105, 1.142, 1.160, 1.170, 1.170, 1.189** tiles from the source square's centre, mean **1.156**, spread **0.084**.
/// The old standoff was `radius[0] + node radius` = 0.2 + 0.5 = **0.70** (D169), which is nearly half a tile inside the
/// game's front row - the difference the owner could see between his screenshot and our render.
pub(crate) const GATHER_RING_MILLI: i64 = 1156;

/// **Places on the ring: one every two degrees** (D188). There is no cap and no table of places any more: the owner's rule is that
/// a man works wherever **his circle fits**, so the ring is sampled finely and each sample is tested against every other man. 180
/// samples sit **0.020** tiles apart on the 1.156 ring - twenty times finer than the 0.40 a body needs - so a man is never refused
/// a place that exists, and how many fit falls out of the circumference instead of a constant.
pub const GATHER_PLACES: u16 = 180;

/// **How far round the ring a man will walk** before taking another source instead, in thousandths of a tile (D188). The owner:
/// *"take next tree instead"* - a place on the far side costs two tiles of walking round the tree, and a man who does that is a
/// man not gathering. The test is the *extra* walk over walking straight at the source.
pub(crate) const GATHER_WALK_SLACK_MILLI: i64 = 1500;

/// **How many men a node's tiles hold**: the touching places of every tile it occupies, for a body of `radius`, spaced
/// [`GATHER_SPACING_MILLI`] apart. Derived from the geometry (D191/D192) - seven for a villager on one tile - so there is no
/// cap in a table and none to keep in step.
#[must_use]
pub fn touching_place_count(points: &[(u16, u16)], radius: i64, scale: i64) -> usize {
    points
        .iter()
        .map(|(x, y)| {
            crate::tiles::touching_places(
                (i64::from(*x), i64::from(*y)),
                radius,
                spacing(scale),
                scale,
            )
            .len()
        })
        .sum()
}

/// **How far apart two working men stand**, in thousandths of a tile: `0.660`.
///
/// Not one body (0.400). The owner's own saves measure **0.661-0.663** between seven men round one node, and the gap is the
/// whole reason a crowd is deliverable: with friends blocking (D200), a place exactly one body from the next one is a place
/// nobody can walk into, because the man walking to it is stopped at the reach. Seven places on the touching envelope is
/// also what those saves show (`docs/reviews/2026-11-13-the-standoff-is-measured.md`, D199/D202).
pub(crate) const GATHER_SPACING_MILLI: i64 = 660;

/// [`GATHER_SPACING_MILLI`] in raw units, at this world's scale.
pub(crate) fn spacing(scale: i64) -> i64 {
    (scale * GATHER_SPACING_MILLI / 1000).max(1)
}

/// The **sine of 0, 2, 4 ... 90 degrees**, in millionths: the quarter table the ring is drawn from.
///
/// Integer tables rather than `f64`: the simulation is deterministic and hashed, and a platform's `sin` is not the same number
/// twice. Every other angle on the ring is this table read backwards or negated.
const SIN: [i64; 46] = [
    0, 34899, 69756, 104528, 139173, 173648, 207912, 241922, 275637, 309017, 342020, 374607,
    406737, 438371, 469472, 500000, 529919, 559193, 587785, 615661, 642788, 669131, 694658, 719340,
    743145, 766044, 788011, 809017, 829038, 848048, 866025, 882948, 898794, 913545, 927184, 939693,
    951057, 961262, 970296, 978148, 984808, 990268, 994522, 997564, 999391, 1000000,
];

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
            entering: BTreeMap::new(),
            patrols: BTreeMap::new(),
            resuming: BTreeMap::new(),
            stances: BTreeMap::new(),
            refusal: None,
            leash: BTreeMap::new(),
            leash_ticks: None,
            leash_tiles: None,
            explored: std::array::from_fn(|_| Vec::new()),
            memory: std::array::from_fn(|_| BTreeMap::new()),
            observers: [false; MAX_PLAYERS],
            map_size: (0, 0),
            blocking: BTreeSet::new(),
            gates: BTreeSet::new(),
            open_gates: BTreeSet::new(),
            footprints: BTreeMap::new(),
            blast: BTreeMap::new(),
            aggro_radius: 0,
            aggro_scan_ticks: 0,
            queued: BTreeMap::new(),
            training_pauses: BTreeMap::new(),
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
            husks: BTreeMap::new(),
            husk_ticks: 0,
            fell_carry: BTreeMap::new(),
            share_failed: BTreeMap::new(),
            share_moved: BTreeMap::new(),
            share_retry_ticks: 600,
            share_move_ticks: 40,
            foundations: BTreeSet::new(),
            stalled_sites: BTreeSet::new(),
            gather_place: BTreeMap::new(),
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
            stalled: BTreeMap::new(),
            repath_count: 0,
            step_counts: [0; 13],
            tracing_steps: false,
            repath_by_body: 0,
            repath_by_ground: 0,
            repath_failures: 0,
            made_room_count: 0,
            resources: None,
            node_radii: Vec::new(),
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

    /// Install the **match's** villager-share table - called with the scenario's, which carries the personality's
    /// (D94). **Not called by `load_resources`**, deliberately: the shares change what an idle villager does, and a test
    /// that loads the real pack to watch a villager's own behaviour must not silently get a second hand on it (six
    /// `gather.rs` tests said so, loudly, the first time this ran from the loader). The runners call this; a world that
    /// is not playing a match does not have to, and a match whose AI writes no `[policy]` has none.
    pub fn apply_policy(&mut self, policy: &Policy) {
        self.install_policy(policy);
    }

    /// Resolve the share rows into stockpile columns and install them. A name the pack cannot resolve is
    /// skipped rather than refused - the pack is authoritative about which resources exist, and a share for a resource
    /// that is not in the game is a share for nothing.
    fn install_policy(&mut self, policy: &Policy) {
        if self.economy.is_none() || policy.share.is_empty() {
            return;
        }
        let rows: Vec<(usize, Vec<(usize, i32)>)> = policy
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

    /// The nearest nodes of a stockpile column, by straight-line distance - a **short list**, not one answer.
    ///
    /// **No reachability test here, deliberately.** `can_reach` is a full path search; asking it of six nodes once a
    /// second per player cost 37x the whole rest of the run (400 game-seconds: 319 ms without this pass, 11.7 s with it,
    /// 2026-10-29). `start_gather` plans the walk and is the one authority on whether a node can be worked, so this
    /// returns a few candidates in distance order and the caller stops at the first that starts. A node that fails is
    /// remembered (`share_failed`) instead of being retried every second - 0 A.D.'s `lastFailedGather`.
    /// **How far a worker looks for its next node**: its own line of sight plus five tiles (D149, the owner's number).
    ///
    /// The five is a look-around, not a sense: a villager whose tree runs out walks to the nearest thing *it can notice*,
    /// and a patch it scouted twenty tiles ago is not that. D141 tried a bound without the fog filter and starved a match;
    /// this one only ever removes candidates the player already knows, so the failure mode is a villager standing idle, not
    /// an empty base.
    /// **How far this worker looks for the next source**, in raw units (D175). Content's `search_radius` when the pack carries
    /// it, otherwise its sight plus a look-around - and public because a test has to be able to ask what the rule is.
    #[must_use]
    pub fn gather_reach(&self, worker: EntityId) -> i64 {
        // **The unit's own `search_radius` when content carries it** (D175). The dat gives every unit one - a villager **4** tiles, a
        // scout 4, a cog 6, a fishing ship 12, a trebuchet 17-18, and 21 at the widest - and that is the number the original looks
        // for its next source within. Packs generated before this field existed carry none, and fall back to sight plus a look-around.
        let unit_search = self
            .find(worker)
            .and_then(|entity| entity.unit)
            .and_then(|unit| {
                self.units
                    .as_ref()
                    .and_then(|units| units.unit(units.root(unit)))
            })
            .and_then(|unit| unit.search_radius.value());
        if let Some(tiles) = unit_search
            && tiles > 0.0
        {
            #[allow(clippy::cast_possible_truncation)]
            let tiles = tiles.round() as i64;
            return tiles * self.scale;
        }
        const LOOK_AROUND_TILES: i64 = 5;
        // `sight_of` answers in **whole tiles**, so the sum is tiles and the scale is applied once here. Mixing the two -
        // tiles plus a raw offset - gives a reach of five tiles and looks like a rule that refuses everything.
        let tiles = match self.find(worker) {
            Some(entity) => self.sight_of(entity),
            None => 0,
        };
        (tiles + LOOK_AROUND_TILES) * self.scale
    }

    /// **`reach` is the worker's own look-around, or `None` for an order** (D149): "nearby" governs what a worker
    /// chooses for itself; a player or a script pointing at a resource is not guessing what the villager can notice.
    /// **Public for the scenario language** (D198): a rule that names a resource slot asks the engine which nodes are in it, and the
    /// runner lives outside this crate now. Nothing in the tick calls it from here.
    pub fn nodes_of_slot(
        &self,
        worker: EntityId,
        slot: usize,
        limit: usize,
        reach: Option<i64>,
    ) -> Vec<EntityId> {
        let Some(from) = self.position(worker) else {
            return Vec::new();
        };
        let Some(player) = self.find(worker).map(|entity| entity.player) else {
            return Vec::new();
        };
        let mut nodes: Vec<(i64, u32, EntityId)> = Vec::new();
        for entity in &self.entities {
            if self.node_slot(entity.id) != Some(slot) {
                continue;
            }
            // **A worker only knows what has been scouted** (D148): distance over every node in the world sent villagers
            // across the map to berries nobody had found. A node the player has never seen is not a candidate at all, and
            // with nothing left to work the worker goes idle - the owner's rule.
            if !self.knows(player, entity) {
                continue;
            }
            let (dx, dy) = (
                entity.position.0.raw() - from.0.raw(),
                entity.position.1.raw() - from.1.raw(),
            );
            if reach.is_some_and(|reach| dx * dx + dy * dy > reach * reach) {
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

    /// How many routes the engine has re-planned because a body was in the way. **Instrumentation, not state**: the tick
    /// never reads it and it is not hashed; a test counts it to prove a known obstacle is not re-planned every tick.
    pub fn repaths(&self) -> u64 {
        self.repath_count
    }

    /// **What the movement pass decided for this body on the last tick it was asked** (D196): `None` if there is no such entity.
    pub fn last_step(&self, id: EntityId) -> Option<StepDecision> {
        self.find(id).map(|entity| entity.last_step)
    }

    /// How many times this decision has been made, over the whole match. Instrumentation: a slice that changes how bodies answer each
    /// other is judged by these the way it is judged by the bank.
    pub fn step_count(&self, step: StepDecision) -> u64 {
        self.step_counts[step.index()]
    }

    /// **Start writing the movement trace** (D196). Off by default and free when off; call this before the ticks you want to
    /// explain.
    pub fn trace_steps(&mut self) {
        self.tracing_steps = true;
    }

    /// Whether the movement trace is being written.
    pub fn is_tracing_steps(&self) -> bool {
        self.tracing_steps
    }

    /// Record one decision. Called once per body per tick, from the movement pass.
    pub(crate) fn record_step(&mut self, index: usize, id: EntityId, step: StepDecision) {
        if !self.tracing_steps {
            return;
        }
        self.step_counts[step.index()] += 1;
        // **O(1), through the index the pass already has**: a `BTreeMap` keyed by id cost 1.8 µs a tick at 128 bodies (6.7 -> 8.5),
        // which was a third of the tick for instrumentation. The fallback by id only runs on ticks where the list moved.
        if let Some(entity) = self.entities.get_mut(index)
            && entity.id == id
        {
            entity.last_step = step;
        } else if let Some(entity) = self.find_mut(id) {
            entity.last_step = step;
        }
    }

    /// How many of `repaths()` a **body** forced, and how many **ground** did. Two counters rather than one because the
    /// question "did this slice make bodies more work or is there simply more traffic?" cannot be answered from a total.
    pub fn repaths_by_body(&self) -> u64 {
        self.repath_by_body
    }

    /// See `repaths_by_body`.
    pub fn repaths_by_ground(&self) -> u64 {
        self.repath_by_ground
    }

    /// How many ground re-plans failed: the measure of a unit caught against ground it cannot answer for (D135).
    pub fn repath_failures(&self) -> u64 {
        self.repath_failures
    }

    /// How many steps have made room for a body in the way (C1b). Exposed so a test can say a nudge happened **once**.
    #[must_use]
    pub fn made_room(&self) -> u64 {
        self.made_room_count
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

    /// Can a walker get from one place to another? Asked by the scenario runner when it picks which
    /// node to send a villager to, so the script chooses a node it can reach instead of ordering a
    /// trip the engine will refuse.
    pub fn can_reach(&self, from: (Fixed, Fixed), to: (Fixed, Fixed)) -> bool {
        self.plan_path(from, to, 0).is_ok()
    }

    /// A tile's origin as a world position. Tiles are the unit of the map; the scale is the unit
    /// of state, and this is the only place the two meet.
    /// **Public for the scenario language** (D198): a rule's "can he reach this tile?" needs a tile's centre in engine units, and the
    /// runner lives outside this crate now.
    pub fn tile(&self, (x, y): (u16, u16)) -> (Fixed, Fixed) {
        (
            Fixed::from_raw(i64::from(x) * self.scale),
            Fixed::from_raw(i64::from(y) * self.scale),
        )
    }

    /// **The middle of a tile, which is where a body sent to that tile stands** (D114).
    ///
    /// `tile` is a tile's *corner*: the anchor a building's footprint and a spawn position are placed from. It was also the
    /// point every walk aimed at, so a tile order ended where four cells meet - half a tile off the middle of the tile the
    /// player clicked, and exactly on a wall's corner when one of those four cells was solid. Two tests say so directly
    /// (`tile_goal.rs`): a walk to a tile stopped 10,000 raw units from its middle, and a walk to a tile beside a house
    /// stopped **0** units from the house.
    fn tile_centre(&self, (x, y): (u16, u16)) -> (Fixed, Fixed) {
        let half = self.scale / 2;
        (
            Fixed::from_raw(i64::from(x) * self.scale + half),
            Fixed::from_raw(i64::from(y) * self.scale + half),
        )
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

    /// The tile a position falls in. Truncating division, because every position the engine stores is positive - a
    /// position outside the map is clamped to tile 0 rather than panicking, which is what a test fixture needs.
    /// **The tile a resource occupies.** Every node in this engine is **one tile** - a map that writes `tiles = [[x,y], ...]` for a
    /// wood becomes one node per tile at load - which is also the game's own model: his saves show **24 of 49** men standing in the
    /// notch between two adjacent tiles, so a cluster is a honeycomb of single tiles and never a block with an outline.
    #[must_use]
    pub fn resource_tiles(&self, node: EntityId) -> Vec<(u16, u16)> {
        self.find(node)
            .filter(|entity| matches!(entity.kind, EntityKind::Resource(_)))
            .map(|entity| vec![self.tile_of(entity.position)])
            .unwrap_or_default()
    }

    pub(crate) fn tile_of(&self, position: (Fixed, Fixed)) -> (u16, u16) {
        (
            u16::try_from(position.0.raw() / self.scale).unwrap_or(0),
            u16::try_from(position.1.raw() / self.scale).unwrap_or(0),
        )
    }

    /// A route between two positions, as **points to walk through**, or `NoPath` when the target is walled off.
    ///
    /// Two solvers, in this order, and the second is why a walk bends where the ground does (D113):
    /// 1. **The short-range pathfinder** (`short.rs`): the goal is within its window and reachable, so it answers the whole
    ///    question geometrically - obstacles expanded by the body's radius, corners of the merged rectangles, shortest path
    ///    through them. This is the route a player watches.
    /// 2. **A\* over the tile grid**, pulled to the corners an obstacle forces, when the geometry declines (a window too big
    ///    for its budget, or a goal outside it). Those are straight lines between **tile corners** - better than a staircase,
    ///    worse than the geometry, and cheap.
    ///
    /// `radius` is the body's own footprint: a route for a scout is not a route for a house. A caller that only asks whether
    /// a place is reachable passes `0`, because the question is about walls, not clearance.
    pub(crate) fn plan_path(
        &self,
        from: (Fixed, Fixed),
        to: (Fixed, Fixed),
        radius: i64,
    ) -> Result<Vec<(Fixed, Fixed)>, SimError> {
        let walk = self.walk.as_ref().ok_or(SimError::NoTables)?;
        let (start, goal) = (self.tile_of(from), self.tile_of(to));
        if let Some(points) = short_range(walk, self.scale, from, to, radius, &BTreeSet::new()) {
            return Ok(points);
        }
        // The geometry declined: the tile search answers instead, and its corners become the route.
        // A blocked goal is allowed here for the same reason it always was - a node's own tile holds the thing being
        // worked, and "come as close as you can" is what an order on a wall means.
        let steps = if start == goal || line_clear(walk, start, goal) {
            vec![goal]
        } else {
            find_path(walk, start, goal).ok_or(SimError::NoPath {
                from: start,
                to: goal,
            })?
        };
        Ok(self.corners(walk, start, steps, to))
    }

    /// The route a **body** would walk: the same plan, with its own footprint as the clearance.
    fn plan_path_for(
        &self,
        id: EntityId,
        from: (Fixed, Fixed),
        to: (Fixed, Fixed),
    ) -> Result<Vec<(Fixed, Fixed)>, SimError> {
        let radius = self.body_radius(self.find(id).and_then(|entity| entity.unit));
        self.plan_path(from, to, radius)
    }

    /// A pulled tile chain as points: each corner is the **centre of its tile**, and the last point is the goal itself, so a
    /// click between tiles lands where it was clicked.
    fn corners(
        &self,
        walk: &Walk,
        start: (u16, u16),
        steps: Vec<(u16, u16)>,
        goal: (Fixed, Fixed),
    ) -> Vec<(Fixed, Fixed)> {
        let pulled = Self::pull_route(walk, start, steps);
        let half = self.scale / 2;
        let mut points: Vec<(Fixed, Fixed)> = pulled
            .iter()
            .map(|(x, y)| {
                (
                    Fixed::from_raw(i64::from(*x) * self.scale + half),
                    Fixed::from_raw(i64::from(*y) * self.scale + half),
                )
            })
            .collect();
        if let Some(last) = points.last_mut() {
            *last = goal;
        }
        points
    }

    /// **B**: spend the waypoints a clear line lets the unit skip.
    ///
    /// The pather's route is a polyline through tile centres and it carries slack - measured at **4.94% of walking
    /// distance over 200 routes** on the project map - because A\* minimises the sum of 10/14 steps rather than the length
    /// of the line the unit actually walks. From each node this jumps to the furthest node `line_clear` reaches, so the
    /// corners that remain are the ones an obstacle forces. Cheap: it runs when a route is planned, never per tick.
    fn pull_route(walk: &Walk, start: (u16, u16), steps: Vec<(u16, u16)>) -> Vec<(u16, u16)> {
        Self::pull_route_avoiding(walk, start, steps, &BTreeSet::new())
    }

    /// The same pull, obeying the tiles a repath's search went round (D98a). Without this the **repathed** route is the
    /// search's own answer - a node per tile - and the unit walks it tile centre to tile centre: the staircase, in the one
    /// place the owner actually watches a base full of villagers, because most long walks there are repaths.
    fn pull_route_avoiding(
        walk: &Walk,
        start: (u16, u16),
        steps: Vec<(u16, u16)>,
        avoid: &BTreeSet<(u16, u16)>,
    ) -> Vec<(u16, u16)> {
        if steps.len() < 2 {
            return steps;
        }
        let mut nodes = Vec::with_capacity(steps.len() + 1);
        nodes.push(start);
        nodes.extend(steps);
        let mut pulled: Vec<(u16, u16)> = Vec::with_capacity(nodes.len());
        let last = nodes.len() - 1;
        let mut at = 0usize;
        while at + 1 < nodes.len() {
            let mut jump = last;
            // Every intermediate node the search returns is free ground (only the goal may be blocked), so the pull needs
            // no rule of its own about the end: `line_clear_avoiding` applies the same endpoint exemption the search does.
            while jump > at + 1 && !line_clear_avoiding(walk, nodes[at], nodes[jump], avoid) {
                jump -= 1;
            }
            pulled.push(nodes[jump]);
            at = jump;
        }
        pulled
    }

    /// The next waypoint of a unit's route, as a world position.
    fn next_waypoint(&self, id: EntityId) -> Option<(Fixed, Fixed)> {
        // **A point, not a tile.** The route already carries the corner the walk should bend at and the exact place the
        // order pointed (D113); `step_towards` lands on it exactly, so the waypoint-spent test is still an equality.
        self.paths.get(&id)?.points.first().copied()
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

    /// The checks a walk order has to survive: the unit is a unit, the tile is on the map, and there
    /// is a route to it. Returns the goal as a world position.
    /// The checks a **point** order has to survive (A): the unit is a unit, the point is on the map, and there is a route
    /// to it. Returns the point itself, not the centre of the tile it falls in - that is the whole point of the variant.
    fn check_order_at(
        &self,
        unit: EntityId,
        to: (Fixed, Fixed),
    ) -> Result<(Fixed, Fixed), SimError> {
        let entity = self.find(unit).ok_or(SimError::MissingEntity(unit))?;
        if entity.kind != EntityKind::Unit {
            return Err(SimError::NotAUnit { id: unit });
        }
        let (x, y) = self.tile_of_raw(to);
        let (Ok(tx), Ok(ty)) = (u16::try_from(x), u16::try_from(y)) else {
            return Err(SimError::OffMap { tile: (0, 0) });
        };
        if !self.on_map(tx, ty) {
            return Err(SimError::OffMap { tile: (tx, ty) });
        }
        self.plan_path_for(unit, entity.position, to)?;
        Ok(to)
    }

    fn check_order(&self, unit: EntityId, to: (u16, u16)) -> Result<(Fixed, Fixed), SimError> {
        let entity = self.find(unit).ok_or(SimError::MissingEntity(unit))?;
        if entity.kind != EntityKind::Unit {
            return Err(SimError::NotAUnit { id: unit });
        }
        if !self.on_map(to.0, to.1) {
            return Err(SimError::OffMap { tile: to });
        }
        self.walk_goal(unit, entity.position, self.tile_centre(to))
    }

    /// **The nearest legal standing point** to a walk order's target (D134; the owner's answer to *"close as it can"*:
    /// *"nearest legal standing point"*), already planned: `Ok` means a route to it exists.
    ///
    /// A walk order names a **place**. A place on blocked ground - the middle of a wood, a wall, a building's footprint -
    /// is a request to come as close to it as the ground allows, and a place no route reaches is the same request. So the
    /// order's own goal becomes a tile a body can stand on, decided once, when the order is given: no later re-plan, crowd
    /// slot, stall check or repath ever aims at a point nobody can occupy.
    ///
    /// **Only walk orders come here.** A gather, build or attack order names a *thing*, and "unreachable" is the answer
    /// those have to keep - a tree behind a wall is a refusal, not a walk to the fence (item 9's rule).
    fn walk_goal(
        &self,
        unit: EntityId,
        from: (Fixed, Fixed),
        to: (Fixed, Fixed),
    ) -> Result<(Fixed, Fixed), SimError> {
        match self.plan_path_for(unit, from, to) {
            Ok(_) => Ok(to),
            Err(SimError::NoPath { .. }) => {
                let Some(walk) = self.walk.as_ref() else {
                    return Err(SimError::NoPath {
                        from: self.tile_of(from),
                        to: self.tile_of(to),
                    });
                };
                let (start, goal) = (self.tile_of(from), self.tile_of(to));
                let (tile, _) = nearest_reachable(walk, start, goal).ok_or(SimError::NoPath {
                    from: start,
                    to: goal,
                })?;
                let stand = self.tile_centre(tile);
                self.plan_path_for(unit, from, stand)?;
                Ok(stand)
            }
            Err(other) => Err(other),
        }
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

    /// Whether this building can shelter one more of its owner's units. The capacity is the building's own `garrison`
    /// field - the same number its garrison orders are checked against - and the count is what it already holds. A
    /// building whose capacity is zero (a market, a smith, a house) shelters nobody, whatever it can produce.
    fn garrison_room(&self, building: EntityId, player: u8) -> bool {
        let Some(shelter) = self.find(building) else {
            return false;
        };
        let Some(stats) = shelter
            .unit
            .and_then(|unit| self.units.as_ref().and_then(|units| units.unit(unit)))
        else {
            return false;
        };
        let capacity = u32::from(stats.garrison);
        if capacity == 0 {
            return false;
        }
        // A building at or below its eject threshold has already turned its garrison out, so sheltering a new recruit
        // in it would put one back where the rule just took them out (D37). The arithmetic is the garrison order's.
        let max = self.max_hp(shelter.player, stats.id).unwrap_or(stats.hp);
        let floor = (i64::from(max) * self.garrison_eject_at.raw()) / self.scale;
        if i64::from(shelter.hp) <= floor {
            return false;
        }
        let inside = u32::try_from(
            self.entities
                .iter()
                .filter(|entity| {
                    entity.player == player
                        && entity.kind == EntityKind::Unit
                        && entity.task == (Task::Garrisoned { building })
                })
                .count(),
        )
        .unwrap_or(u32::MAX);
        inside < capacity
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
            match self.plan_path(position, entry.home, 0) {
                Ok(route) => {
                    self.paths.insert(
                        id,
                        Route {
                            points: route,
                            ..Route::default()
                        },
                    );
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

    /// **A wall drag** (D155): every tile of the line, in the order the drag was made.
    ///
    /// A tile that cannot be built on is **skipped** - the owner's answer B, "skip it, keep going" - so the rest of the wall
    /// still stands; the whole command fails only when **no** tile could be built, and then with the first refusal, which is
    /// the one worth reporting. `start_build` prices each segment, so running out of wood mid-line stops the line where the
    /// money ran out rather than building the far end first.
    fn start_build_line(
        &mut self,
        villager: EntityId,
        unit_id: u32,
        tiles: &[(u16, u16)],
    ) -> Result<(), SimError> {
        let mut refused: Option<SimError> = None;
        let mut built = 0usize;
        for tile in tiles {
            match self.start_build(villager, unit_id, *tile) {
                Ok(()) => built += 1,
                Err(why) => refused = refused.or(Some(why)),
            }
        }
        if built == 0 {
            return Err(refused.unwrap_or(SimError::EmptyLine));
        }
        Ok(())
    }

    /// **The tiles a wall drag covers** (D155): one bend at most, a 45 degree leg first, then the straight one.
    ///
    /// Public because the view draws its preview from it: there must be one answer to "where does the wall go", and it is
    /// this one (`world::lines::wall_line`, tested without a world).
    #[must_use]
    pub fn wall_line(from: (u16, u16), to: (u16, u16)) -> Vec<(u16, u16)> {
        world::lines::wall_line(from, to)
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
        self.start_build_site(villager, unit_id, at)
    }

    /// **Start one site now**, with no regard for a queue (D155).
    ///
    /// The queue's own advance path used to call `start_build`, which asks "is there a queue? then queue this instead" -
    /// so popping a site off a queue of three put it straight back at the back, and a villager that finished a building
    /// with two more queued **never started either**. Splitting the immediate path out fixes that, and it is what a wall
    /// line needs: the drag starts its first segment and queues the rest.
    pub(crate) fn start_build_site(
        &mut self,
        villager: EntityId,
        unit_id: u32,
        at: (u16, u16),
    ) -> Result<(), SimError> {
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
            last_step: StepDecision::Idle,
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
        // **Our own and allied units step aside by walking** (D183 §1), never by being placed somewhere. The builder is
        // excluded: its task is about to become the build order, with a route to a standoff outside the footprint.
        self.evict_units_off(id, villager);

        let from = self
            .find(villager)
            .map(|entity| entity.position)
            .ok_or(SimError::MissingEntity(villager))?;
        // Beside the site, not on it: a builder standing inside the footprint has no free tile to
        // plan a route home from, and once finished the building would be under the villager.
        //
        // **Outside the whole footprint** (D183 §3): a ghost is walkable, so the anchor's own neighbours are no longer
        // reliably outside it, and a builder standing on his own site would be waiting for himself to leave.
        let footprint = self.footprint_area_of(id);
        let route = match self.approach_footprint(from, &footprint) {
            Some(stand) => self.plan_path_for(villager, from, stand)?,
            None => self.plan_path_for(villager, from, position)?,
        };
        self.paths.insert(
            villager,
            Route {
                points: route,
                ..Route::default()
            },
        );
        if let Some(worker) = self.find_mut(villager) {
            worker.task = Task::Build { site: id };
        }
        Ok(())
    }

    /// The checks a **join** order has to survive: the villager is a unit, and the site is that player's own unfinished
    /// foundation. Nothing is paid - the man who placed it paid.
    pub(crate) fn check_join(&self, villager: EntityId, site: EntityId) -> Result<(), SimError> {
        let worker = self
            .find(villager)
            .ok_or(SimError::MissingEntity(villager))?;
        if worker.kind != EntityKind::Unit {
            return Err(SimError::NotAUnit { id: villager });
        }
        let building = self.find(site).ok_or(SimError::MissingEntity(site))?;
        let own = building.player == worker.player;
        let building_kind = matches!(building.kind, EntityKind::Building);
        if !own || !building_kind || building.construction.is_none() {
            return Err(SimError::NotAFoundation { site });
        }
        Ok(())
    }

    /// **Take a join order**: validated, then the walk and the task.
    pub(crate) fn start_join(
        &mut self,
        villager: EntityId,
        site: EntityId,
    ) -> Result<(), SimError> {
        self.check_join(villager, site)?;
        self.join_foundation(villager, site)
    }

    /// **Walk a villager out to a foundation that already stands and put him on it** (D186 addendum 3): the same route the
    /// first builder took - outside the whole footprint, never onto it - and the same task, so the site's own gate and the
    /// builder's adjacency apply to him exactly as they apply to the man who placed it.
    fn join_foundation(&mut self, villager: EntityId, site: EntityId) -> Result<(), SimError> {
        let from = self
            .find(villager)
            .map(|entity| entity.position)
            .ok_or(SimError::MissingEntity(villager))?;
        let position = self
            .find(site)
            .map(|entity| entity.position)
            .ok_or(SimError::MissingEntity(site))?;
        let footprint = self.footprint_area_of(site);
        let route = match self.approach_footprint(from, &footprint) {
            Some(stand) => self.plan_path_for(villager, from, stand)?,
            None => self.plan_path_for(villager, from, position)?,
        };
        self.paths.insert(
            villager,
            Route {
                points: route,
                ..Route::default()
            },
        );
        if let Some(worker) = self.find_mut(villager) {
            worker.task = Task::Build { site };
        }
        Ok(())
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

    /// How many **military** units `player` has, by content's `[metrics] military_roles`.
    ///
    /// The same list the balance harness counts (`Metrics`), so a rule that waits for an army and a metric that reports
    /// one cannot disagree about what an army is. **Family-matched**, because a man-at-arms is the militia's upgrade and
    /// both answer the question "how many soldiers" - the roots are computed once per call, not once per entity.
    ///
    /// A dead soldier is not an army: `hp > 0` filters the corpses that the death sweep has not collected yet.
    /// The family roots content calls military, computed once per call. A world with no unit table cannot family-match,
    /// so it compares ids: the role *is* its own root, which is the only statement available. Returning nothing here
    /// instead was a bug that made an army of three read as zero in every fixture that seeds without a table.
    fn military_roots(&self) -> Vec<u32> {
        let Some(economy) = self.economy.as_ref() else {
            return Vec::new();
        };
        economy
            .metrics
            .military_roles
            .iter()
            .filter_map(|role| economy.role.get(role).copied())
            .map(|unit| self.units.as_ref().map_or(unit, |units| units.root(unit)))
            .collect()
    }

    /// Is this unit a **soldier**, by content's `[metrics] military_roles`? The view asks it to choose a silhouette, and
    /// `army` asks it to count - so a picture and a metric cannot disagree about what a soldier is.
    #[must_use]
    pub fn is_military(&self, unit: u32) -> bool {
        let root = self.units.as_ref().map_or(unit, |units| units.root(unit));
        self.military_roots().contains(&root)
    }

    /// Is this unit a **worker** - the start villager's family, so an upgrade still counts?
    #[must_use]
    pub fn is_worker(&self, unit: u32) -> bool {
        self.start_villager_unit()
            .is_some_and(|villager| self.is_family(unit, villager))
    }

    /// The **mark** content gives this unit's role (item 6), for the view to draw on a building's footprint. The name is
    /// the view's vocabulary ("ring", "sword", ...); content only says which role carries which mark.
    #[must_use]
    pub fn mark_name(&self, unit: u32) -> Option<&str> {
        let economy = self.economy.as_ref()?;
        let root = self.units.as_ref().map_or(unit, |units| units.root(unit));
        let role = economy
            .role
            .iter()
            .find(|(_, id)| **id == unit)
            .or_else(|| {
                economy.role.iter().find(|(_, id)| {
                    self.units.as_ref().map_or(**id, |units| units.root(**id)) == root
                })
            })
            .map(|(name, _)| name)?;
        economy.mark.get(role).map(String::as_str)
    }

    #[must_use]
    pub fn army(&self, player: u8) -> usize {
        let roots = self.military_roots();
        if roots.is_empty() {
            return 0;
        }
        self.entities
            .iter()
            .filter(|entity| {
                entity.player == player && entity.kind == EntityKind::Unit && entity.hp > 0
            })
            .filter(|entity| {
                entity.unit.is_some_and(|unit| {
                    roots.contains(&self.units.as_ref().map_or(unit, |units| units.root(unit)))
                })
            })
            .count()
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

    /// The bodies that can be in the way this tick, **sorted by tile then id**:
    /// `(tile x, tile y, id, index into the entity list, its own radius)`.
    ///
    /// A flat sorted slice rather than a map of tile to list: the map allocated a `Vec` per occupied tile on every
    /// moving tick, which is 150 allocations a tick at the acceptance match's size, and the sorted slice answers the
    /// same nine-tile query with `partition_point` and one allocation. The index is an index into the entity list, not
    /// a copied position: a body that steps during the tick has to be seen where it now is, and a map of copies left a
    /// pair standing 500 raw units inside each other (measured in C's first build).
    fn body_tiles(&self) -> Vec<(i64, i64, EntityId, usize, i64)> {
        let mut bodies: Vec<(i64, i64, EntityId, usize, i64)> =
            Vec::with_capacity(self.entities.len());
        for (index, entity) in self.entities.iter().enumerate() {
            if entity.kind != EntityKind::Unit
                || matches!(entity.task, Task::Garrisoned { .. })
                || entity.hp <= 0
            {
                continue;
            }
            let (x, y) = self.tile_of_raw(entity.position);
            // The footprint is resolved **once per body per tick**, not once per pair it is asked about: at 150 bodies
            // and five queries a step that was the difference between 20.7 s and 18.4 s on the acceptance run.
            bodies.push((x, y, entity.id, index, self.body_radius(entity.unit)));
        }
        bodies.sort_unstable_by_key(|(x, y, id, _, _)| (*x, *y, id.0));
        bodies
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
    pub(crate) fn goal_of(&self, entity: &Entity) -> Option<((u16, u16), bool)> {
        let (goal, exclusive) = match entity.task {
            Task::Move { to } | Task::Hunt { to } => (to, true),
            // A drop site is worked, not occupied: two villagers must be able to hand in at one town
            // centre on the same tick, and a building's own tile is never free ground anyway.
            // **A carrier aims at the yard, not at the building's own ground** (D200). The drop's anchor tile is blocked -
            // it is a building - so every search that ends there fails, the route's stalls run out, and the man waits for
            // the match: measured, a full-handed carrier at `(11.850,21.139)` in `WaitGround` for 400,000 ticks while the
            // bank held 790 of 900. `approach_tile` names the walkable tile beside the footprint.
            Task::Return { drop, .. } => {
                let at = self.find(drop)?.position;
                let tile = self.tile_of(at);
                if self.tile_walkable(tile.0, tile.1) {
                    (at, false)
                } else {
                    (
                        self.approach_tile(self.tile_of(entity.position), drop)
                            .map_or(at, |yard| self.tile_centre(yard)),
                        false,
                    )
                }
            }
            Task::Build { site } => (self.find(site)?.position, false),
            Task::Gather { node, .. } => (self.find(node)?.position, false),
            Task::Attack { target, .. } => (self.find(target)?.position, false),
            Task::Idle | Task::Garrisoned { .. } | Task::Fly { .. } => return None,
        };
        Some((self.tile_of(goal), exclusive))
    }

    /// A tile coordinate from an engine position, without the `u16` clamp `tile_of` applies.
    pub(crate) fn tile_of_raw(&self, position: (Fixed, Fixed)) -> (i64, i64) {
        (
            position.0.raw().div_euclid(self.scale),
            position.1.raw().div_euclid(self.scale),
        )
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
            || self.stalled_sites.contains(&id)
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

    /// **What this building is making** (D157), for the interface: the running job with its progress in ticks, and the jobs
    /// waiting behind it **in order**. One call, so a HUD cannot draw three pictures that disagree.
    #[must_use]
    pub fn production(&self, building: EntityId) -> Production {
        let entity = self.find(building);
        let running = entity.and_then(|entity| {
            if let Some(job) = entity.training {
                // `remaining_ticks` counts down from the total resolved when the order landed, so the total is what the pack
                // priced and the done is the difference. A `fresh` job has spent no work yet.
                let total = self
                    .train_total_ticks(job.unit)
                    .unwrap_or(job.remaining_ticks);
                Some((
                    QueuedItem::Unit(job.unit),
                    total.saturating_sub(job.remaining_ticks),
                    total,
                ))
            } else {
                entity.researching.map(|job| {
                    let total = self
                        .research_total_ticks(job.tech)
                        .unwrap_or(job.remaining_ticks);
                    (
                        QueuedItem::Tech(job.tech),
                        total.saturating_sub(job.remaining_ticks),
                        total,
                    )
                })
            }
        });
        let waiting = self
            .queued
            .get(&building)
            .map(|jobs| {
                jobs.iter()
                    .map(|job| match job {
                        Job::Train(training) => QueuedItem::Unit(training.unit),
                        Job::Research { tech } => QueuedItem::Tech(*tech),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Production { running, waiting }
    }

    /// A unit's create time in ticks, as the pack prices it.
    fn train_total_ticks(&self, unit: u32) -> Option<u32> {
        let stats = self.units.as_ref()?.unit(unit)?;
        u32::try_from(stats.train_time)
            .ok()
            .and_then(|seconds| seconds.checked_mul(self.tick_rate_hz))
    }

    /// A tech's research time in ticks, as the pack prices it.
    fn research_total_ticks(&self, tech: u32) -> Option<u32> {
        let stats = self.techs.as_ref()?.get(tech)?;
        u32::try_from(stats.research_time)
            .ok()
            .and_then(|seconds| seconds.checked_mul(self.tick_rate_hz))
    }

    /// Why this building's production is stopped, if it is (D85). The view draws the reason it is given; the
    /// simulation never says it in words.
    pub fn training_pause(&self, building: EntityId) -> Option<TrainingPause> {
        self.training_pauses.get(&building).copied()
    }

    /// The most health this unit could have for this player, after every research that grants it. Public
    /// because a HUD and a health bar both need it, and both are outside the simulation (D55).
    /// **How much larger than the body the selection ring is drawn** (D125), from content. View-side only: nothing in the tick
    /// reads it, so it cannot desync anything.
    /// The same number in **hundredths**, so no float enters the simulation (D9's `no_float` guard is the reason).
    #[must_use]
    pub fn ring_scale_percent(&self) -> i32 {
        self.economy
            .as_ref()
            .map_or(150, |economy| economy.ring.scale_percent)
    }

    pub fn max_hp_of(&self, player: u8, unit: u32) -> Option<i32> {
        self.max_hp(player, unit).ok()
    }

    /// **What a worker is working on** (D121), as the panel needs it: the target's **role name** when the pack gives it one
    /// (`"farm"`, `"sheep"`) and the **resource column** the target holds when it holds one. The words are content's;
    /// this is the engine's answer to *which* target, and nothing here knows that a woodcutter is called a Lumberjack.
    ///
    /// `None` when the unit is not working on anything - idle, walking, fighting - and then the caller keeps the pack's own
    /// name for the unit, which is what a villager with nothing to do is.
    pub fn work_target(&self, id: EntityId) -> Option<WorkTarget> {
        let entity = self.entity(id)?;
        let target = match entity.task {
            Task::Gather { node, .. } | Task::Return { node, .. } => node,
            _ => return None,
        };
        let target_entity = self.entity(target)?;
        Some(WorkTarget {
            role: target_entity.unit.and_then(|unit| self.role_name(unit)),
            column: self.held_slot(target),
        })
    }

    /// The tiles a unit occupies, from the pack's own radii. Public for the same reason as `max_hp_of`: a
    /// renderer draws a building's silhouette and must not carry a table of sizes to do it (D17).
    pub fn footprint_of(&self, unit: u32) -> Option<(u32, u32)> {
        self.units.as_ref()?.footprint(unit)
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
        self.body_radius(a) + self.body_radius(b)
    }

    /// One body's own footprint, in engine units: `radius[0]` of its variant, or zero when the content carries none
    /// (D98a's `Pos=(...) Radius=`). Hoisted out of the pair calculation because the movement pass asks it for every body
    /// it can see on every step, and resolving it per pair cost 2.3 s of a 36,000-tick acceptance run (measured).
    pub(crate) fn body_radius(&self, id: Option<u32>) -> i64 {
        let Some(units) = self.units.as_ref() else {
            return 0;
        };
        id.and_then(|id| units.unit(units.root(id)))
            .and_then(|unit| unit.radius.first().copied())
            .and_then(|value| value.resolve(self.scale, "radius").ok())
            .map_or(0, |value| value.raw())
    }

    /// The standoff two bodies keep from each other: the sum of their footprints' radii, or **one whole tile** when the
    /// content gives them no radius at all.
    ///
    /// One number for two rules (C): `collide` refuses a step that ends inside it, and the arrival predicates accept work
    /// at it, so "too close to stand" and "close enough to work" cannot drift apart. A target whose own tile is blocked
    /// ground - a tree, a mine, a town centre - is worked from the ring around it, which the predicates express in the
    /// same units.
    #[must_use]
    pub fn reach_between(&self, a: EntityId, b: EntityId) -> Fixed {
        let unit = |id: EntityId| self.find(id).and_then(|entity| entity.unit);
        Fixed::from_raw(self.reach_raw(unit(a), unit(b)))
    }

    fn reach_raw(&self, a: Option<u32>, b: Option<u32>) -> i64 {
        match self.body_radii(a, b) {
            0 => self.scale,
            sum => sum,
        }
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
        } else if economy.role.get("town_center") == Some(&root) {
            population.town_center_population
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

    pub(crate) fn work_position(
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
            self.plan_path(from, position, 0).ok().map(|_| position)
        })
    }

    /// **Is there any ring at all?** True when at least one of the nine slots round a source square is walkable, free
    /// ground (D186). It decides *where* a man works: on the ring when there is one, but from the tile beside the source when
    /// the ring is buried - a tree deep in a forest has its whole ring on blocked tiles, and the men who cut it stand on the
    /// open edge beside it, which is the one case the ring cannot describe. Nine walkability lookups, no path search.
    pub(crate) fn ring_open(&self, centre: (Fixed, Fixed)) -> bool {
        // Every tenth degree: enough to tell a buried ring from an open one, and 36 lookups rather than 180 on the path where a
        // gatherer decides every tick.
        (0..GATHER_PLACES).step_by(5).any(|place| {
            let point = self.ring_point(centre, place);
            let (Ok(x), Ok(y)) = (
                u16::try_from(point.0.raw().div_euclid(self.scale)),
                u16::try_from(point.1.raw().div_euclid(self.scale)),
            ) else {
                return false;
            };
            self.tile_walkable(x, y) && self.is_free_ground(x, y)
        })
    }

    /// **Where a worker stands to work a node** (D192, superseding D186/D188's ring): **with his circle touching the node's tile**.
    /// The owner: *"their circles are touching the square ... center stays at least half the diameter of the circle"*. His saves
    /// measure it: men at **0.70** from a resource tile's centre (`0.5 + 0.2`) and **0.99** at a corner, not the 1.156 ring this
    /// engine used - which was the row *behind* the working one, and the reason the front row never worked.
    ///
    /// The places come from `tiles::touching_places`, one tile at a time: every node in this engine is a single tile, and a cluster
    /// is a honeycomb of them, so the notch between two adjacent tiles is walkable ground - 24 of the owner's 49 men stand in one.
    /// How many fit falls out of the geometry (eleven for a villager), and there is no cap and no table.
    ///
    /// A candidate must pass: **walkable ground**; **not on a foundation**; **one diameter clear** of every body and every reserved
    /// place; and **no further than `GATHER_WALK_SLACK_MILLI` beyond walking straight at the node**, because the owner's answer to a
    /// place on the far side was *"take next tree instead"*. `NoRoom` is an answer, not a failure: the caller sends the man on.
    pub(crate) fn standoff_point(
        &self,
        from: (Fixed, Fixed),
        target: (Fixed, Fixed),
        node: EntityId,
        worker: EntityId,
        taken: &[(EntityId, (Fixed, Fixed))],
        check_path: bool,
    ) -> Option<Standoff> {
        let scale = self.scale;
        let worker_radius = self
            .find(worker)
            .and_then(|entity| entity.unit)
            .map_or(0, |unit| self.body_radius(Some(unit)));
        let (dx, dy) = (from.0.raw() - target.0.raw(), from.1.raw() - target.1.raw());
        if dx == 0 && dy == 0 {
            return None;
        }
        // **Two bodies, a gap apart** (D202): the owner's saves measure 0.661-0.663 between working men, not 0.400. With
        // friends blocking, a place exactly one body from its neighbour is a place nobody can walk into.
        let too_close = spacing(scale);
        let centre = self.tile_centre(self.tile_of(target));
        let slack = scale * GATHER_WALK_SLACK_MILLI / 1000;
        let straight = integer_sqrt(
            (centre.0.raw() - from.0.raw()).pow(2) + (centre.1.raw() - from.1.raw()).pow(2),
        );
        let mut candidates: Vec<(Fixed, Fixed)> = Vec::new();
        for tile in self.resource_tiles(node) {
            candidates.extend(crate::tiles::touching_places(
                (i64::from(tile.0), i64::from(tile.1)),
                worker_radius,
                too_close,
                scale,
            ));
        }
        let mut best: Option<(i64, (Fixed, Fixed))> = None;
        for point in candidates {
            let (px, py) = (point.0.raw(), point.1.raw());
            let away = integer_sqrt((px - from.0.raw()).pow(2) + (py - from.1.raw()).pow(2));
            if best.as_ref().is_some_and(|(closest, _)| away >= *closest) {
                continue;
            }
            if away > straight + slack {
                continue;
            }
            let (Ok(x), Ok(y)) = (
                u16::try_from(px.div_euclid(scale)),
                u16::try_from(py.div_euclid(scale)),
            ) else {
                continue;
            };
            if !self.tile_walkable(x, y) {
                continue;
            }
            let on_site = self.foundations.iter().any(|site| {
                self.footprint_rect(*site)
                    .is_some_and(|(ax, ay, wide, tall)| {
                        x >= ax
                            && x < ax.saturating_add(wide)
                            && y >= ay
                            && y < ay.saturating_add(tall)
                    })
            });
            if on_site {
                continue;
            }
            let near = |point: (Fixed, Fixed)| -> bool {
                let (bx, by) = (point.0.raw() - px, point.1.raw() - py);
                bx * bx + by * by < too_close * too_close
            };
            if self
                .entities
                .iter()
                .any(|other| other.id != worker && near(other.position))
            {
                continue;
            }
            if taken
                .iter()
                .any(|(other, point)| *other != worker && near(*point))
            {
                continue;
            }
            if self.gather_place.iter().any(|(other, (at, held))| {
                *other != worker
                    && *at == node
                    && near((Fixed::from_raw(held.0), Fixed::from_raw(held.1)))
                    && self.place_is_live(*other, node)
            }) {
                continue;
            }
            if check_path && !self.can_reach(from, point) {
                continue;
            }
            best = Some((away, point));
        }
        Some(best.map_or(Standoff::NoRoom, |(_, point)| Standoff::Point(point)))
    }

    /// **A point on the ring**, two degrees per place: `place` 0 is due east and the index grows anticlockwise. The quarter table
    /// `SIN` holds 0..90 degrees, so each quadrant reads it forwards or backwards with one or both signs flipped.
    pub(crate) fn ring_point(&self, centre: (Fixed, Fixed), place: u16) -> (Fixed, Fixed) {
        let ring = self.scale * GATHER_RING_MILLI / 1000;
        let index = usize::from(place % GATHER_PLACES);
        let within = index % 45;
        let (sin, cos) = match index / 45 {
            0 => (SIN[within], SIN[45 - within]),
            1 => (SIN[45 - within], -SIN[within]),
            2 => (-SIN[within], -SIN[45 - within]),
            _ => (-SIN[45 - within], SIN[within]),
        };
        (
            Fixed::from_raw(centre.0.raw() + ring * cos / 1_000_000),
            Fixed::from_raw(centre.1.raw() + ring * sin / 1_000_000),
        )
    }

    /// **Is this man still gathering that node?** A place belongs to an order, so the moment the order changes the place is gone -
    /// asked when the place is read rather than swept every tick, which keeps the tick free of a map-shaped cost.
    fn place_is_live(&self, worker: EntityId, node: EntityId) -> bool {
        self.find(worker).is_some_and(|entity| {
            matches!(
                entity.task,
                Task::Gather { node: at, .. } | Task::Return { node: at, .. } if at == node
            )
        })
    }

    /// **Every place held on a node**, for a probe or a test: `(worker, place)`. Reading it from outside is what tells a
    /// reservation defect apart from a movement one.
    #[must_use]
    pub fn gather_places(&self) -> Vec<(EntityId, (i64, i64))> {
        self.gather_place
            .iter()
            .map(|(worker, (_, (x, y)))| (*worker, (*x, *y)))
            .collect()
    }

    /// **Let a place go**: the man is being moved by something other than his order - a foundation under his feet, a site
    /// clearing - so the place he had must not pull him back.
    pub(crate) fn release_gather_place(&mut self, worker: EntityId) {
        self.gather_place.remove(&worker);
    }

    /// **Take the place a man has walked to, and keep it** (D186 addendum 5). Stored as the point itself rather than an index into
    /// a table that no longer exists: a place is a position, and `place_stands_clear` re-checks the ground it stands on.
    pub(crate) fn reserve_gather_place(
        &mut self,
        worker: EntityId,
        node: EntityId,
        point: (Fixed, Fixed),
    ) {
        self.gather_place
            .insert(worker, (node, (point.0.raw(), point.1.raw())));
    }

    /// **The place this man already owns**, if its ground is still ground. `is_free_ground` is deliberately not asked: it answers
    /// "may a body stand here", and the body asking is the man himself.
    pub(crate) fn gather_place_of(
        &self,
        worker: EntityId,
        node: EntityId,
    ) -> Option<(Fixed, Fixed)> {
        let (at, (x, y)) = *self.gather_place.get(&worker)?;
        if at != node || !self.place_is_live(worker, node) {
            return None;
        }
        let point = (Fixed::from_raw(x), Fixed::from_raw(y));
        let (Ok(tx), Ok(ty)) = (
            u16::try_from(x.div_euclid(self.scale)),
            u16::try_from(y.div_euclid(self.scale)),
        ) else {
            return None;
        };
        if !self.tile_walkable(tx, ty) {
            return None;
        }
        let on_site = self.foundations.iter().any(|site| {
            self.footprint_rect(*site)
                .is_some_and(|(ax, ay, wide, tall)| {
                    tx >= ax
                        && tx < ax.saturating_add(wide)
                        && ty >= ay
                        && ty < ay.saturating_add(tall)
                })
        });
        (!on_site).then_some(point)
    }

    /// **How much room a body has**, in raw units: the distance from `position` to the nearest **blocked tile**, minus `radius`.
    /// Zero is "touching the wall", negative is "inside it" - and the movement rule of D176 is that it never goes negative.
    ///
    /// This is the original's *"shrink the unit to a point, expand every other obstruction"* statement of the same thing: a disc of radius
    /// `r` against a square is the square grown by `r`, so the test is one distance, not a tile lookup.
    #[must_use]
    pub fn clearance_at(&self, position: (Fixed, Fixed), radius: i64) -> i64 {
        let scale = self.scale;
        let Some(walk) = self.walk.as_ref() else {
            return i64::MAX;
        };
        let (x, y) = (position.0.raw(), position.1.raw());
        let (tx, ty) = (x.div_euclid(scale), y.div_euclid(scale));
        let mut nearest = i64::MAX;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (cx, cy) = (tx + dx, ty + dy);
                let (Ok(cx16), Ok(cy16)) = (u16::try_from(cx), u16::try_from(cy)) else {
                    continue;
                };
                if walk.contains(cx16, cy16) && self.tile_walkable(cx16, cy16) {
                    continue;
                }
                // Distance from the point to this tile's square, axis by axis, then the hypotenuse.
                let left = cx * scale;
                let top = cy * scale;
                let gx = if x < left {
                    left - x
                } else if x > left + scale {
                    x - (left + scale)
                } else {
                    0
                };
                let gy = if y < top {
                    top - y
                } else if y > top + scale {
                    y - (top + scale)
                } else {
                    0
                };
                let distance = (gx * gx + gy * gy).isqrt();
                nearest = nearest.min(distance);
            }
        }
        nearest.saturating_sub(radius)
    }

    /// **A unit's clearance**, using its own body radius (D176).
    #[must_use]
    pub fn body_clearance(&self, id: EntityId) -> i64 {
        let Some(entity) = self.find(id) else {
            return i64::MAX;
        };
        let radius = self.body_radius(entity.unit);
        self.clearance_at(entity.position, radius)
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

    /// **Husks go** (D122): an empty farm is a picture with a lifetime, and this is the clock.
    ///
    /// The first pass records a farm that has just been emptied; the second removes the ones whose while is over. Both run
    /// every fourth tick, like the spent-node pass beside it: a husk is a second-scale thing and this is not the tick loop's
    /// hot path. **Nothing here frees a tile** - a farm's footprint is a floor and a husk blocks nothing, which is what the
    /// owner asked for ("it obstructs nothing as if it was not there"); the entity that goes is a drawing.
    fn step_husks(&mut self) {
        if !self.tick.is_multiple_of(4) {
            return;
        }
        if self.husk_ticks > 0 {
            let now = self.tick;
            let empty: Vec<EntityId> = self
                .entities
                .iter()
                // **One pass, no lookups.** The first version asked `is_expired_farm(id)` per entity, which does a linear
                // `find` for an entity the loop already holds - O(n^2) every fourth tick, and it showed: the tick went
                // 5.9 -> 7.4 us (+23%) when this pass landed. The check here is on the entity in hand.
                .filter(|entity| {
                    !self.husks.contains_key(&entity.id) && self.is_expired_farm_ref(entity)
                })
                .map(|entity| entity.id)
                .collect();
            for id in empty {
                self.husks.insert(id, now);
            }
        }
        if self.husks.is_empty() || self.husk_ticks == 0 {
            return;
        }
        let now = self.tick;
        let gone: Vec<EntityId> = self
            .husks
            .iter()
            .filter(|(_, since)| now.saturating_sub(**since) >= self.husk_ticks)
            .map(|(id, _)| *id)
            .collect();
        for id in gone {
            self.husks.remove(&id);
            self.paths.remove(&id);
            self.entities.retain(|entity| entity.id != id);
        }
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
                    walk.unblock(tile.0, tile.1);
                }
            }
            self.entities.retain(|entity| entity.id != node);
            self.spent_nodes.remove(&node);
        }
    }

    /// Whether this worker can **work** that node: somewhere to stand that is free *and reachable*. A node's own tile is
    /// blocked - a tree, a bush, a mine - so `can_reach(from, node.position)` is false for every one of them, and a
    /// chooser that asks it rejects the whole map (the trap: `nearest_resource` did, and a test helper spun 400,000
    /// ticks re-issuing an order that could never start).
    #[must_use]
    pub fn can_work(&self, from: EntityId, node: EntityId) -> bool {
        let (Some(here), Some(there)) = (self.position(from), self.position(node)) else {
            return false;
        };
        self.work_position(here, there).is_some()
    }

    /// How many of this player's villagers are **working** the resource in `slot`: a gather task whose node fills that
    /// column, or a trip home carrying what that node gave. A farm is a building, so the column comes from
    /// `gather_source`, not from the entity's kind - the same rule the trip home uses.
    #[must_use]
    pub fn workers_on(&self, player: u8, slot: usize) -> usize {
        self.entities
            .iter()
            .filter(|entity| entity.player == player && entity.kind == EntityKind::Unit)
            .filter(|entity| {
                let node = match entity.task {
                    Task::Gather { node, .. } | Task::Return { node, .. } => Some(node),
                    _ => None,
                };
                node.and_then(|node| self.gather_source(node).ok())
                    .and_then(|(resource, _)| self.slot_of(&resource))
                    == Some(slot)
            })
            .count()
    }

    /// The entity a command acts through: the villager, the building, the attacker, or the id a spawn names. A refusal
    /// that cannot name its actor is a refusal nobody can diagnose, and this used to return `EntityId(0)` for every
    /// command - so every recorded refusal read `entity 0`. The actor is read **before** `apply`, which takes the command
    /// by value.
    fn command_entity(command: &Command) -> EntityId {
        match command {
            Command::Gather { villager, .. }
            | Command::Build { villager, .. }
            | Command::JoinBuild { villager, .. }
            | Command::BuildLine { villager, .. } => *villager,
            Command::Stop { unit }
            | Command::WalkTo { unit, .. }
            | Command::Move { unit, .. }
            | Command::AttackMove { unit, .. }
            | Command::Patrol { unit, .. }
            | Command::Stance { unit, .. }
            | Command::Garrison { unit, .. } => *unit,
            Command::Delete { entity, .. } => *entity,
            Command::Attack { attacker, .. } => *attacker,
            Command::Train { building, .. }
            | Command::Research { building, .. }
            | Command::Rally { building, .. }
            | Command::Ungarrison { building } => *building,
            Command::Spawn { id, .. } => *id,
        }
    }

    /// A farm whose food is gone. It **stays on the map** - walkable, worth nothing - until a new farm replaces it, and the
    /// replacement costs the full 60 wood (the owner's rule, 2026-10-30). Content names the farm role; the engine only
    /// knows that a building with nothing left in it and no construction in progress is not in the way.
    #[must_use]
    pub fn is_expired_farm(&self, id: EntityId) -> bool {
        self.find(id)
            .is_some_and(|entity| self.is_expired_farm_ref(entity))
    }

    /// The same answer for an entity the caller **already holds** - the snapshot asks it once per entity per frame, and asking
    /// by id there was a linear `find` per entity, which is O(n^2) on the frame path.
    #[must_use]
    pub fn is_expired_farm_ref(&self, entity: &Entity) -> bool {
        self.is_farm_role(entity) && entity.construction.is_none() && entity.amount.raw() <= 0
    }

    /// Is this entity a farm, by the role content calls `[farm] unit`? By family, so an upgraded farm counts.
    pub(crate) fn is_farm_role(&self, entity: &Entity) -> bool {
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
    /// **Where a unit dispossessed by a building goes**: the nearest walkable tile that is not part of the building,
    /// measured on the grid as it stands *before* the building is inserted. Nearest is by squared tile distance, with the
    /// tile itself as the tie-break so the answer is deterministic.
    fn escape_from(&self, tiles: &[(u16, u16)], from: (u16, u16)) -> Option<(Fixed, Fixed)> {
        let (wide, tall) = self.walk.as_ref().map_or((0, 0), Walk::size);
        let mut best: Option<NearKey> = None;
        // **The ring, not the square.** A square scan up to the map's size is a quarter of a million tile tests per
        // dispossessed unit, and it cost 60 s on the suite when I first wrote it this way; a ring costs O(radius) and the
        // loop stops as soon as no further ring can hold anything nearer than what it has.
        for radius in 1..=wide.max(tall).max(1) {
            if let Some((key, _)) = best
                && key.0 <= u32::from(radius) * u32::from(radius)
            {
                break;
            }
            for at @ (x, y) in Self::ring(from, radius) {
                if tiles.contains(&at) || !self.tile_walkable(x, y) {
                    continue;
                }
                let across = u32::from(x.abs_diff(from.0));
                let down = u32::from(y.abs_diff(from.1));
                let key = (across * across + down * down, x, y);
                if best.as_ref().is_none_or(|(seen, _)| key < *seen) {
                    best = Some((key, at));
                }
            }
        }
        best.map(|(_, at)| self.tile_centre(at))
    }

    /// The nearest of `candidates` to `from`, by squared tile distance with the tile as the tie-break.
    fn nearest_of(
        &self,
        from: (u16, u16),
        candidates: impl Iterator<Item = (u16, u16)>,
    ) -> Option<(u16, u16)> {
        let mut best: Option<NearKey> = None;
        for at @ (x, y) in candidates {
            let across = u32::from(x.abs_diff(from.0));
            let down = u32::from(y.abs_diff(from.1));
            let key = (across * across + down * down, x, y);
            if best.as_ref().is_none_or(|(seen, _)| key < *seen) {
                best = Some((key, at));
            }
        }
        best.map(|(_, at)| at)
    }

    /// **Whether a unit is on or beside a building's ground.** The anchor is a blocked tile in the middle of the diamond,
    /// so "how far from the anchor" answers a question nobody asked: a villager handing in from a town centre's yard is
    /// two tiles from the anchor and stands *on* the building. Without this, widening the approach (see `approach_tile`)
    /// walked carriers into the yard and then refused to bank them there - the economy lost a third of its size and the
    /// acceptance match stopped being won (measured: 176 entities won at HEAD, 127 entities drawn with the approach
    /// alone).
    #[must_use]
    pub fn touches_footprint(&self, position: (Fixed, Fixed), building: EntityId) -> bool {
        let Some(entity) = self.find(building) else {
            return false;
        };
        let Some((across, down)) = entity.unit.and_then(|unit| self.footprint_of(unit)) else {
            return false;
        };
        let anchor = self.tile_of(entity.position);
        let here = self.tile_of(position);
        // One tile of slack on every side: a crowd handing in at one building stands a little deep, and that is what the
        // "touching is enough" rule (C2) is about.
        let (wide, tall) = (across as u16, down as u16);
        here.0.saturating_add(1) >= anchor.0
            && here.1.saturating_add(1) >= anchor.1
            && here.0 <= anchor.0.saturating_add(wide)
            && here.1 <= anchor.1.saturating_add(tall)
    }

    /// **The ground a worker stands on to work a place.** A building's own tile is blocked, so a route aimed at it does
    /// not exist: the walkable tiles of the footprint come first (a town centre's yard), then the ring touching the
    /// footprint. Without this, a carrier standing on the far side of its own barracks was told there was no route home
    /// and stood holding its load for the rest of the match - every acceptance run's last refusal, and six red tests.
    fn approach_tile(&self, from: (u16, u16), building: EntityId) -> Option<(u16, u16)> {
        let entity = self.find(building)?;
        let tiles = self.build_site_tiles(entity.unit.unwrap_or(0), self.tile_of(entity.position));
        let yard = self.nearest_of(
            from,
            tiles
                .iter()
                .copied()
                .filter(|&(x, y)| self.tile_walkable(x, y)),
        );
        yard.or_else(|| {
            let ring: Vec<(u16, u16)> = tiles.iter().flat_map(|at| Self::ring(*at, 1)).collect();
            self.nearest_of(
                from,
                ring.into_iter()
                    .filter(|at| !tiles.contains(at) && self.tile_walkable(at.0, at.1)),
            )
        })
    }

    /// Every tile exactly `radius` tiles from `at`, in a stable order, without leaving the map.
    fn ring(at: (u16, u16), radius: u16) -> Vec<(u16, u16)> {
        let (x0, y0) = (at.0.saturating_sub(radius), at.1.saturating_sub(radius));
        let (x1, y1) = (at.0.saturating_add(radius), at.1.saturating_add(radius));
        let mut out = Vec::with_capacity(radius as usize * 8);
        for x in x0..=x1 {
            out.push((x, y0));
            if y1 != y0 {
                out.push((x, y1));
            }
        }
        for y in (y0 + 1)..y1 {
            out.push((x0, y));
            if x1 != x0 {
                out.push((x1, y));
            }
        }
        out
    }

    /// **A body's own footprint, in engine units**, as the movement rules read it (`radius[0]` of its variant, or zero
    /// when the content carries none). Public because a test - and later a renderer - asks how much room a unit keeps, and
    /// a second copy of that number is how two answers to one question start.
    #[must_use]
    pub fn body_radius_of(&self, id: EntityId) -> i64 {
        self.body_radius(self.find(id).and_then(|entity| entity.unit))
    }

    /// **Which resource column this entity holds, if it holds one.** The panel asks so it can print a number *and say whose
    /// number it is*: a node knows its column from its kind, while a **farm is a building** whose column only content knows
    /// (`economy.farm.resource`). `None` for a villager, a house, an arrow.
    #[must_use]
    pub fn held_slot(&self, id: EntityId) -> Option<usize> {
        let entity = self.find(id)?;
        match entity.kind {
            // A node's own kind is its column: the dat's wood line, resolved by the pack.
            EntityKind::Resource(slot) => Some(slot),
            // A **farm is a building**, and the only thing that knows which column a farm fills is content: `[farm]
            // resource` in `economy.toml`. Asking the same table the gather rules ask keeps one answer to "what does a
            // farm yield" instead of a second one in the panel.
            EntityKind::Building => {
                let farm = self.farm_unit()?;
                let unit = entity.unit?;
                let root = |unit: u32| self.units.as_ref().map_or(unit, |units| units.root(unit));
                if root(unit) != root(farm) {
                    return None;
                }
                self.economy
                    .as_ref()?
                    .farm
                    .resource
                    .as_deref()
                    .and_then(|name| self.slot_of(name))
            }
            // A villager's load and a building's queue are not a resource node's holdings.
            EntityKind::Unit | EntityKind::Projectile => None,
        }
    }

    /// How many waypoints are left on this entity's route: 0 means it is standing on its goal (or has none).
    #[must_use]
    pub fn path_length(&self, id: EntityId) -> usize {
        self.paths.get(&id).map_or(0, |route| route.points.len())
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
        // **The node's half-extent, in raw units, resolved once** (D169). `radius` is the pack's own f64 - the dat's
        // `radius_x` - and this is the one place it is turned into engine units, so nothing in a tick ever touches a float.
        let scale = self.scale;
        self.node_radii = resources
            .entries
            .iter()
            .map(|entry| (entry.radius * scale as f64).round() as i64)
            .collect();
        self.resources = Some(resources.clone());
        // The pack is what turns a name in `[start] stock` into a column, so this is the earliest
        // point the opening stockpile can be applied when the pack is loaded after seeding.
        self.apply_start_stock()?;
        Ok(())
    }

    /// **What the selected building offers, as the buttons a player would see.** Empty for a unit, for another player's
    /// building, and for a foundation: none of those has a card.
    ///
    /// Every entry is checked with the same function the order goes through, so the card and the order cannot disagree -
    /// a lesson paid for twice in this tree (a HUD that showed what nothing drew, a selection nobody called).
    /// **The age this player has reached**, as the pack names it: the last name in `[age] order` whose tech is researched,
    /// or the first name when none is. `None` when the pack gives no order - a pack without one must not be guessed at,
    /// and the alphabetically last key is not "the latest age".
    #[must_use]
    pub fn age_name(&self, player: u8) -> Option<String> {
        let economy = self.economy.as_ref()?;
        let first = economy.age.order.first()?.clone();
        let done = self.researched(player);
        let mut reached = first;
        for role in &economy.age.order {
            let Some(id) = economy.tech_role.get(role) else {
                continue;
            };
            if done.contains(id) {
                reached = role.clone();
            }
        }
        Some(reached)
    }

    /// The role name a unit id carries in the pack, if any: the inverse of `role_unit`, so a card can name a unit without
    /// the window holding a second copy of the vocabulary.
    #[must_use]
    pub fn role_name(&self, unit: u32) -> Option<String> {
        let economy = self.economy.as_ref()?;
        // **Through the family.** An age-up replaces a building with its variant (the town centre 109 becomes 71, a house 70
        // becomes 463), and a lookup on the exact id found nothing - so the owner's selected town centre was called "unit 71"
        // the moment the Feudal Age arrived ("buildings get reamed upon move to the next age", 2026-11-04).
        let root = self.units.as_ref().map_or(unit, |units| units.root(unit));
        // **Its own role first.** A unit the pack names is that unit: 74 is `militia`, 448 is `scout_cavalry`.
        if let Some((name, _)) = economy.role.iter().find(|(_, id)| **id == unit) {
            return Some(name.clone());
        }
        // **Then the family's base role**, which is what the fallback was added for: an age-up replaces a building's id
        // (the town centre 109 becomes 71) and the panel still has to call it a town centre.
        if let Some((name, _)) = economy.role.iter().find(|(_, id)| **id == root) {
            return Some(name.clone());
        }
        // **Then any role in the family, lowest id first.** The order is the tie-break, because a map ordered by name made
        // "whichever comes first alphabetically" the answer - and for an upgrade line that is the *top* tier: a Dark Age
        // militia read as `champion` and the opening scout read as `hussar` (the owner's report, 2026-11-04).
        economy
            .role
            .iter()
            .filter(|(_, id)| {
                self.units
                    .as_ref()
                    .is_some_and(|units| units.root(**id) == root)
            })
            .min_by_key(|(_, id)| **id)
            .map(|(name, _)| name.clone())
    }

    /// A price as `(stock column, amount)`. A cost the pack cannot place in a column is dropped: a button that quotes a
    /// number for a resource the bank does not hold would be quoting a column that does not exist.
    fn cost_slots(&self, costs: &[ruage_data::Cost]) -> Vec<(usize, u32)> {
        costs
            .iter()
            .filter_map(|cost| {
                let slot = match &cost.resource {
                    ruage_data::CostResource::ReferenceId(id) => self.resource_slot(u16::from(*id)),
                    ruage_data::CostResource::Name(name) => self.slot_of(name),
                }?;
                Some((slot, u32::from(cost.amount)))
            })
            .collect()
    }

    /// **Whether an existing drop site that accepts this worker's load is already within `tiles` of `at`.** The game AI's
    /// own rule for a camp: "a drop site already this near a forest, mine or forage makes another there pointless" - the
    /// reference implementation's `kCovered` is **5 tiles**, and its camp may be at most `sn-camp-max-distance` (25) from
    /// home. `town` is the reference's own flag: what a camp would serve, the town centre does not already serve ("the Town
    /// Center counting only if `town`"), which is what lets a mill stand at the berries the town centre sits beside.
    #[must_use]
    pub fn drop_site_within(
        &self,
        player: u8,
        variant: Option<u32>,
        at: (u16, u16),
        tiles: u32,
        town: bool,
    ) -> bool {
        let (Some(units), Some(variant)) = (self.units.as_ref(), variant) else {
            return false;
        };
        let Some(accepted) = units.unit(variant).map(|stats| &stats.drop_sites) else {
            return false;
        };
        let here = self.tile(at);
        let reach = i64::from(tiles) * self.scale;
        self.entities.iter().any(|entity| {
            if entity.kind != EntityKind::Building || entity.player != player {
                return false;
            }
            if !town && self.is_town_centre(entity.unit) {
                return false;
            }
            let accepts = entity.unit.is_some_and(|unit| {
                let root = units.root(unit);
                accepted.contains(&unit) || accepted.contains(&root)
            });
            if !accepts {
                return false;
            }
            let dx = entity.position.0.raw() - here.0.raw();
            let dy = entity.position.1.raw() - here.1.raw();
            dx * dx + dy * dy <= reach * reach
        })
    }

    /// **Whether a refusal is the age talking rather than a price or a building the player could put up now.** The owner's
    /// note (2026-11-04): "techs, upgrades and units should be shown based on age the player is in. Imperial Age upgrades
    /// should not show in Feudal Age." So an item whose answer is "not available", "not upgraded", "your civ bars it", or "a
    /// *later age* is missing" is hidden, while "you need two dark age buildings" or "not enough food" stays on the card with
    /// the engine's sentence: those are things the player can act on in this age.
    #[must_use]
    pub fn is_age_gate(&self, err: &SimError) -> bool {
        match err {
            SimError::UnitUnavailable { .. }
            | SimError::UnitNotUpgraded { .. }
            | SimError::UnitBarred { .. }
            | SimError::TechBarred { .. }
            | SimError::AlreadyResearched { .. } => true,
            SimError::MissingPrerequisite { missing, .. } => self.is_age_tech(*missing),
            _ => false,
        }
    }

    /// **Whether a tech is one of the pack's ages** (`[age] order`): the question "is this gap an age, or something the
    /// player can close now" cannot be answered by the dat's numbering (dark 104, feudal 101, castle 102, imperial 103).
    #[must_use]
    pub fn is_age_tech(&self, tech: u32) -> bool {
        self.economy.as_ref().is_some_and(|economy| {
            economy
                .age
                .order
                .iter()
                .filter_map(|role| economy.tech_role.get(role).copied())
                .any(|id| id == tech)
        })
    }

    /// **The unit a farm plot is**, from content (`[build] farm`): `None` when the pack has no farms, which means no rings
    /// are reserved and every placement rule behaves as it did before the owner's note (2026-11-04).
    #[must_use]
    pub fn farm_unit(&self) -> Option<u32> {
        self.economy
            .as_ref()
            .and_then(|economy| economy.build.farm.as_deref())
            .and_then(|role| self.role_unit(role))
    }

    /// **How far the farm ring reaches from the town centre**, in tiles: `[build] farm_spacing` farm widths plus one tile of
    /// clearance. Farms and drop sites may stand inside it; a house may not, which is the owner's "houses should be
    /// constructed in a way to allow such farm placement".
    #[must_use]
    pub fn farm_ring_end(&self) -> u32 {
        let Some(farm) = self.farm_unit() else {
            return 0;
        };
        let width = self
            .units
            .as_ref()
            .and_then(|units| units.footprint(farm))
            .map_or(3, |(width, _)| width);
        let spacing = self
            .economy
            .as_ref()
            .map_or(2, |economy| economy.build.farm_spacing);
        spacing * width + 1
    }

    /// **Whether a role must not be built where a drop site already serves the resource** - content's `[build]
    /// spaced_roles`, so the owner's lumber-camp rule is a word in the pack rather than a name in the engine.
    #[must_use]
    pub fn build_spaces_drop_sites(&self, role: &str) -> bool {
        self.economy
            .as_ref()
            .is_some_and(|economy| economy.build.spaced_roles.iter().any(|named| named == role))
    }

    /// The worker variant the pack uses for a resource (`[gather.variant]`): content's own answer to "which unit carries
    /// this", already used to pick what a villager becomes when it is sent to a node.
    #[must_use]
    pub fn gather_variant(&self, resource: &str) -> Option<u32> {
        self.economy
            .as_ref()
            .and_then(|economy| economy.gather.variant.get(resource).copied())
    }

    /// **The drop sites a worker of this variant may hand a load in at**, in the order the buildings stand: the same edge the
    /// path to a drop site follows (`drop_sites` on the worker's row), so a farm ring is drawn around the buildings that
    /// actually take food rather than around every camp on the map.
    #[must_use]
    pub fn drop_sites_of(&self, player: u8, variant: Option<u32>) -> Vec<EntityId> {
        let (Some(units), Some(variant)) = (self.units.as_ref(), variant) else {
            return Vec::new();
        };
        let Some(accepted) = units.unit(variant).map(|stats| &stats.drop_sites) else {
            return Vec::new();
        };
        self.entities
            .iter()
            .filter(|entity| entity.kind == EntityKind::Building && entity.player == player)
            .filter(|entity| {
                entity.unit.is_some_and(|unit| {
                    accepted.contains(&unit) || accepted.contains(&units.root(unit))
                })
            })
            .map(|entity| entity.id)
            .collect()
    }

    /// **Whether a building is a drop site at all**: some worker in the pack lists it among the places it can hand a load
    /// in. That is the dat's own edge (`drop_sites` per worker), so nothing here learns a role name - and a pack with a new
    /// camp needs no change to this.
    #[must_use]
    pub fn is_drop_site(&self, unit: u32) -> bool {
        let Some(units) = self.units.as_ref() else {
            return false;
        };
        let root = units.root(unit);
        units.units.values().any(|worker| {
            worker
                .drop_sites
                .iter()
                .any(|site| *site == unit || *site == root)
        })
    }

    /// Whether a unit id is the match's **town centre**, by the start's own role rather than by a number: the reference
    /// implementation's placement asks the same question with `train_loc`.
    #[must_use]
    pub fn is_town_centre(&self, unit: Option<u32>) -> bool {
        let (Some(unit), Some(centre)) = (unit, self.start_town_center_unit()) else {
            return false;
        };
        self.units
            .as_ref()
            .is_some_and(|units| units.root(unit) == units.root(centre))
    }

    /// The tile a position falls in, for a caller comparing tiles - the same truncating division `tile_of` does, exposed
    /// because "which tile is this" is a question content code (and a future UI) asks of the world.
    #[must_use]
    pub fn tile_of_position(&self, at: (Fixed, Fixed)) -> (u16, u16) {
        self.tile_of(at)
    }

    /// **How far from an existing drop site a new camp may still be worth it**, from content (`[build] drop_site_spacing`),
    /// defaulting to the reference implementation's 5 tiles.
    #[must_use]
    pub fn drop_site_spacing(&self) -> u32 {
        self.economy
            .as_ref()
            .map_or(BUILD_SPACING_DEFAULT, |economy| {
                economy.build.drop_site_spacing
            })
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
        self.paths.get(&id).map(|route| route.points.len())
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
