//! **What a refused order says** (D145): the error type and its messages, out of `lib.rs`.
//!
//! `SimError` is the public shape of every refusal - the tick's, the loader's and the validator's - and the messages are
//! read by scripts, tests and the CLI, so they live together rather than 800 lines apart.

use std::fmt;

use crate::{EntityId, MAX_PLAYERS, max_scale_for};

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
    /// **The pack's scale is too fine for its map** (D128): squared distances are computed in `i64`, and beyond this the
    /// product would overflow - in debug a panic, in release a wrong distance and a wrong decision. Refused at load rather
    /// than discovered in a match.
    GeometryOverflow {
        scale: i64,
        tiles: u32,
    },
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
    /// A wall drag with no tiles in it (D155): a line of nothing is not an order.
    EmptyLine,
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
    /// **Not somebody's unfinished site** (D186 addendum 3): a join order naming a building that is finished, or is another
    /// player's, or is not a building at all.
    NotAFoundation {
        site: EntityId,
    },
    SiteOccupied {
        tile: (u16, u16),
    },
    /// The chosen tile is not open ground.
    SiteBlocked {
        tile: (u16, u16),
    },
    /// **The ground has never been looked at** (D183 addendum): a foundation may be placed blind in fog, but not in the
    /// black - an area the player has never seen is not a place they can have chosen.
    SiteUnscouted {
        tile: (u16, u16),
    },
    /// **Construction cannot start while somebody stands on the site** (D183 §4): every unit of every player, and the
    /// animals, must be off the footprint first. A refusal rather than a silent wait, because a site that never starts
    /// with no reason given is undiagnosable.
    SiteNotClear {
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

impl std::fmt::Display for SimError {
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
            SimError::GeometryOverflow { scale, tiles } => write!(
                f,
                "a {tiles}-tile map at scale {scale} overflows squared distances in i64; \\
                 the largest scale that fits is {}",
                max_scale_for(u64::from(*tiles))
            ),
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
            SimError::EmptyLine => write!(f, "a wall drag named no tiles"),
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
            SimError::NotAFoundation { site } => {
                write!(f, "{site:?} is not an unfinished site of yours to join")
            }
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
            SimError::SiteUnscouted { tile } => {
                write!(
                    f,
                    "{tile:?} is ground this player has never looked at: a foundation may go in the fog, never in the black"
                )
            }
            SimError::SiteNotClear { tile } => {
                write!(
                    f,
                    "the site at {tile:?} cannot be started while a unit stands on it - every unit must leave first"
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

/// **Which kind of failure an error is** (D193). Fifty-nine variants in one enum is right for a simulation - each says exactly what
/// went wrong and where - but a caller cannot tell "the player cannot do that" from "the engine is wrong" without matching all of
/// them, so every UI path either swallows errors or prints internals.
///
/// Three classes, decided by **who is at fault**:
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    /// **The player may not do that** - now, here, or with what he has. A legal command that the world refuses: no route, not
    /// enough wood, a site that is not clear, a match already over. The view shows it and moves on; nothing is broken.
    Refused,
    /// **The command is malformed or names something that is not there** - an entity that does not exist, another player's unit, a
    /// scenario line that does not parse. The caller made a mistake; the view should not have offered the action.
    BadRequest,
    /// **The engine cannot answer for itself** - fixed-point overflow, a placement with no legal spot. This is a bug in this crate,
    /// not in the player's click.
    Internal,
}

impl SimError {
    /// The class of this error.
    ///
    /// **The match has no wildcard arm on purpose**: a new variant does not compile until it is classified, which is the only
    /// guarantee that stays true as the enum grows. `crates/sim/tests/error_class.rs` pins one variant of each class.
    pub fn class(&self) -> ErrorClass {
        match self {
            // **Refused** - legal orders the world will not take.
            Self::NoPath { .. }
            | Self::NoSpace { .. }
            | Self::NoVariant { .. }
            | Self::FriendlyFire { .. }
            | Self::NotATarget { .. }
            | Self::Unarmed { .. }
            | Self::CannotResearchHere { .. }
            | Self::AlreadyResearching { .. }
            | Self::AlreadyResearched { .. }
            | Self::TechInProgress { .. }
            | Self::QueueFull { .. }
            | Self::Garrisoned { .. }
            | Self::TooFar { .. }
            | Self::CannotGarrisonHere { .. }
            | Self::GarrisonTooDamaged { .. }
            | Self::GarrisonFull { .. }
            | Self::AlliedTarget { .. }
            | Self::UnitBarred { .. }
            | Self::UnitNotUpgraded { .. }
            | Self::UnitUnavailable { .. }
            | Self::MissingBuilding { .. }
            | Self::AgeNeedsBuildings { .. }
            | Self::TechBarred { .. }
            | Self::MissingPrerequisite { .. }
            | Self::TooManyResources { .. }
            | Self::NotEnough { .. }
            | Self::NotAFoundation { .. }
            | Self::SiteOccupied { .. }
            | Self::SiteBlocked { .. }
            | Self::SiteUnscouted { .. }
            | Self::SiteNotClear { .. }
            | Self::NotTrainableHere { .. }
            | Self::BuildingNotReady { .. }
            | Self::TrainNeedsABuilding { .. }
            | Self::NotABuilding { .. }
            | Self::NotDeletable { .. }
            | Self::StanceForbidsAttack { .. }
            | Self::MatchOver => ErrorClass::Refused,

            // **BadRequest** - the command itself is wrong.
            Self::TooManyEntities { .. }
            | Self::DuplicateEntity(_)
            | Self::MissingUnit { .. }
            | Self::MissingEntity(_)
            | Self::Unresolved { .. }
            | Self::EmptyLine
            | Self::NotAResource { .. }
            | Self::NotAUnit { .. }
            | Self::NoTables
            | Self::PlayerOutOfRange { .. }
            | Self::UnknownResource { .. }
            | Self::UnknownResourceName { .. }
            | Self::NoRole { .. }
            | Self::MissingTech { .. }
            | Self::MissingEffect { .. }
            | Self::UnknownCiv { .. }
            | Self::OffMap { .. }
            | Self::NotYours { .. } => ErrorClass::BadRequest,

            // **Internal** - the engine's own arithmetic.
            Self::Displacement { .. } | Self::Overflow(_) | Self::GeometryOverflow { .. } => {
                ErrorClass::Internal
            }
        }
    }
}
