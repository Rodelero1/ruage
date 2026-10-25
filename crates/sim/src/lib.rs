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

use std::fmt;

use ruage_data::{CommandApply, Economy, Map, ResourceKind, Sim as Settings, Units};
use ruage_fixed::Fixed;

/// Stable handle for an entity. Ordered, so iteration by id is just sorting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u32);

/// A building is a place; a unit is a thing that moves. Splitting them now keeps "villagers
/// gather into the town centre" from being a special case of "entities with zero speed".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Building,
    Unit,
    /// A tree, a berry bush or a mine: a place that holds a quantity and can be worked down.
    Resource(ResourceKind),
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
    pub velocity: (Fixed, Fixed),
    pub hp: i32,
    /// What is left in a resource node, in fixed point so a take can be fractional. Zero for
    /// everything that is not a node.
    pub amount: Fixed,
    /// What the unit is doing. Buildings and nodes are always idle.
    pub task: Task,
    /// What the unit is carrying, in fixed point; whole resources are taken on deposit.
    pub carry: Fixed,
    /// The villager variant currently in use (the task's unit id), if any.
    pub variant: Option<u32>,
}

/// Unit work in progress. A state machine small enough to hash and to explain: walking to a node
/// and taking from it. Returning to a drop site is the next increment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Task {
    #[default]
    Idle,
    Gather {
        node: EntityId,
        /// Resource per tick, resolved from the variant's `work_rate` at the engine scale.
        rate_per_tick: Fixed,
        /// The variant's carry, in whole resources.
        carry_cap: u32,
    },
    /// Carrying a full load to somewhere that will take it. The node is remembered so the
    /// villager can go back to work once the load is dropped.
    Return {
        node: EntityId,
        drop: EntityId,
        kind: ResourceKind,
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
    /// Per player, per resource. Index is `player - 1`; player 0 has no stockpile.
    stock: [[u32; ResourceKind::COUNT]; 2],
    sim_rng: Pcg32,
    map_rng: Pcg32,
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
            stock: [[0; ResourceKind::COUNT]; 2],
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

    pub fn position(&self, id: EntityId) -> Option<(Fixed, Fixed)> {
        self.find(id).map(|e| e.position)
    }

    pub fn entity(&self, id: EntityId) -> Option<&Entity> {
        self.find(id)
    }

    /// What a player has banked. Player 0 (nobody) always reads zero.
    pub fn stock(&self, player: u8, kind: ResourceKind) -> u32 {
        self.stock
            .get(usize::from(player).wrapping_sub(1))
            .map(|row| row[kind.index()])
            .unwrap_or(0)
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
        match command {
            Command::Gather { villager, node } => {
                let worker = self
                    .find(*villager)
                    .ok_or(SimError::MissingEntity(*villager))?;
                if worker.kind != EntityKind::Unit {
                    return Err(SimError::NotAUnit { id: *villager });
                }
                let target = self.find(*node).ok_or(SimError::MissingEntity(*node))?;
                if !matches!(target.kind, EntityKind::Resource(_)) {
                    return Err(SimError::NotAResource { id: *node });
                }
            }
            Command::Spawn { .. } => {}
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
    pub fn spawn_now(&mut self, id: EntityId, speed: Fixed, hp: i32) {
        self.spawn_now_checked(id, speed, hp)
            .unwrap_or_else(|err| panic!("{err}"));
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
        let tc_stats = economy.start.town_centre;
        let villager_stats = economy.start.villager;
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
                velocity: (tc_speed, Fixed::ZERO),
                hp: town_centre.hp,
                amount: Fixed::ZERO,
                task: Task::Idle,
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
                    velocity: (villager_speed, Fixed::ZERO),
                    hp: villager.hp,
                    amount: Fixed::ZERO,
                    task: Task::Idle,
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
                kind: EntityKind::Resource(node.kind),
                unit: None,
                position: self.tile((node.x, node.y)),
                velocity: (Fixed::ZERO, Fixed::ZERO),
                hp: 0,
                amount: Fixed::from_raw(i64::from(node.amount) * self.scale),
                task: Task::Idle,
                carry: Fixed::ZERO,
                variant: None,
            })?;
            next_id += 1;
        }
        Ok(())
    }

    /// A tile's origin as a world position. Tiles are the unit of the map; the scale is the unit
    /// of state, and this is the only place the two meet.
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
        let at = self
            .entities
            .iter()
            .position(|existing| existing.id > entity.id)
            .unwrap_or(self.entities.len());
        self.entities.insert(at, entity);
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
        let kind = match self.find(node).map(|n| n.kind) {
            Some(EntityKind::Resource(kind)) => kind,
            _ => return None,
        };
        let from = self.find(node)?.position;
        let drop = self.nearest_drop_site(entity.player, from, entity.variant)?;
        Some(Task::Return {
            node,
            drop,
            kind,
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
            .filter(|entity| entity.unit.is_some_and(|unit| drop_sites.contains(&unit)))
            .min_by_key(|entity| {
                let dx = entity.position.0.raw() - from.0.raw();
                let dy = entity.position.1.raw() - from.1.raw();
                (dx * dx + dy * dy, entity.id.0)
            })
            .map(|entity| entity.id)
    }

    /// How much a worker takes this tick: the rate, the room left under its cap, and what the
    /// node still holds - the smallest of the three.
    fn take_amount(&self, worker: &Entity) -> i64 {
        let Task::Gather {
            node,
            rate_per_tick,
            carry_cap,
        } = worker.task
        else {
            return 0;
        };
        let room = i64::from(carry_cap) * self.scale - worker.carry.raw();
        // `worker` is the villager; the quantity to bound against belongs to a different entity.
        let left_in_node = self.find(node).map(|n| n.amount.raw()).unwrap_or(0);
        rate_per_tick.raw().min(room).min(left_in_node).max(0)
    }

    /// Turn a gather order into resolved state: the variant, the per-tick rate and the capacity
    /// are read from the tables here, once, and stored on the entity. Nothing in the tick loop
    /// looks at a table, and a replay cannot name its own numbers.
    fn start_gather(&mut self, villager: EntityId, node: EntityId) -> Result<(), SimError> {
        let (variant_id, rate_per_tick, carry_cap) = {
            let units = self.units.as_ref().ok_or(SimError::NoTables)?;
            let economy = self.economy.as_ref().ok_or(SimError::NoTables)?;

            let kind = match self.find(node).map(|entity| entity.kind) {
                Some(EntityKind::Resource(kind)) => kind,
                Some(_) => return Err(SimError::NotAResource { id: node }),
                None => return Err(SimError::MissingEntity(node)),
            };
            let task_name = economy.gather.resource_task.for_kind(kind).to_owned();
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
            let rate = variant
                .work_rate
                .resolve(self.scale, "work_rate")
                .map_err(|err| SimError::NoVariant {
                    task: task_name.clone(),
                    detail: err.to_string(),
                })?;
            // Exactness is the D10 invariant: a rate that does not split into whole per-tick
            // amounts would drift, so it is an error rather than a rounded accumulation.
            let rate_per_tick = rate
                .div_exact(i64::from(self.tick_rate_hz))
                .map_err(|err| SimError::NoVariant {
                    task: task_name.clone(),
                    detail: err.to_string(),
                })?;
            (variant_id, rate_per_tick, u32::from(variant.carry))
        };

        let worker = self
            .find_mut(villager)
            .ok_or(SimError::MissingEntity(villager))?;
        worker.variant = Some(variant_id);
        worker.task = Task::Gather {
            node,
            rate_per_tick,
            carry_cap,
        };
        worker.carry = Fixed::ZERO;
        Ok(())
    }

    fn apply(&mut self, command: Command) -> Result<(), SimError> {
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
            } => self.insert(Entity {
                id,
                player,
                kind,
                unit,
                position,
                velocity: (speed, Fixed::ZERO),
                hp,
                amount: Fixed::from_raw(i64::from(amount) * self.scale),
                task: Task::Idle,
                carry: Fixed::ZERO,
                variant: None,
            }),
            Command::Gather { villager, node } => self.start_gather(villager, node),
        }
    }

    /// Advance exactly one tick: apply queued commands, integrate, increment the counter.
    ///
    /// Movement is integrated in id order with an exact per-tick delta, so a unit's position
    /// after N ticks is `N * speed / tick_rate` with no accumulated error.
    pub fn step(&mut self) -> Result<(), SimError> {
        let queued = std::mem::take(&mut self.queue);
        for command in queued {
            self.apply(command)?;
        }

        let tick_rate = i64::from(self.tick_rate_hz);

        // Decide this tick's work in id order, then apply it. Deciding first keeps the second
        // pass free of lookups, so an entity cannot be moved *and* read from in the same tick in
        // an order-dependent way.
        let mut deltas: Vec<(EntityId, Fixed, Fixed)> = Vec::with_capacity(self.entities.len());
        let mut takes: Vec<(EntityId, i64)> = Vec::new();
        let mut finished: Vec<EntityId> = Vec::new();
        let mut switches: Vec<(EntityId, Task)> = Vec::new();
        let mut deposits: Vec<EntityId> = Vec::new();

        for entity in &self.entities {
            let (dx, dy) = match entity.task {
                Task::Return { drop, .. } => {
                    let Some(target) = self.find(drop).map(|n| n.position) else {
                        finished.push(entity.id);
                        continue;
                    };
                    let delta = entity.velocity.0.div_exact(tick_rate).map_err(|err| {
                        SimError::Displacement {
                            id: entity.id,
                            detail: err.to_string(),
                        }
                    })?;
                    let (dx, dy) = step_towards(entity.position, target, delta.raw());
                    if dx == 0 && dy == 0 {
                        deposits.push(entity.id);
                    }
                    (Fixed::from_raw(dx), Fixed::from_raw(dy))
                }
                Task::Gather {
                    node,
                    rate_per_tick,
                    carry_cap,
                } => {
                    let Some(target) = self.find(node).map(|n| n.position) else {
                        // The node is gone (a later increment may delete emptied nodes).
                        finished.push(entity.id);
                        continue;
                    };
                    let delta = entity.velocity.0.div_exact(tick_rate).map_err(|err| {
                        SimError::Displacement {
                            id: entity.id,
                            detail: err.to_string(),
                        }
                    })?;
                    let (dx, dy) = step_towards(entity.position, target, delta.raw());

                    // Standing still on the node is what entitles a worker to take from it.
                    if dx == 0 && dy == 0 {
                        let take = self.take_amount(entity);
                        let node_left = self.find(node).map(|n| n.amount.raw()).unwrap_or(0);
                        if take > 0 {
                            takes.push((entity.id, take));
                        }

                        // Two reasons to head home: the load is full, or the node just gave its
                        // last. The second matters because a partial load on a dead tree never
                        // reaches the cap - the villager would stand there for the rest of the
                        // match holding wood nobody can spend.
                        let in_hand = entity.carry.raw() + take;
                        let empty_node = node_left - take <= 0;
                        let full = in_hand >= i64::from(carry_cap) * self.scale;
                        if (take > 0 && (full || empty_node))
                            || (take == 0 && empty_node && in_hand > 0)
                        {
                            switches.extend(
                                self.return_task(entity, node, rate_per_tick, carry_cap)
                                    .map(|task| (entity.id, task)),
                            );
                        }
                        if take == 0 && empty_node && in_hand <= 0 {
                            finished.push(entity.id);
                        }
                    }
                    (Fixed::from_raw(dx), Fixed::from_raw(dy))
                }
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
            deltas.push((entity.id, dx, dy));
        }

        // Grant takes in id order, each bounded by what the node still holds *at that moment*.
        // Reading the amount before the loop would let two villagers on a one-wood tree take one
        // wood each: the node would clamp at zero and the pair would keep two.
        for (worker, take) in takes {
            let node = match self.find(worker).map(|e| e.task) {
                Some(Task::Gather { node, .. }) => node,
                _ => continue,
            };
            let granted = match self.find_mut(node) {
                Some(target) => {
                    let available = target.amount.raw();
                    let granted = take.min(available);
                    target.amount = Fixed::from_raw(available - granted);
                    granted
                }
                None => 0,
            };
            if let Some(entity) = self.find_mut(worker) {
                entity.carry = Fixed::from_raw(entity.carry.raw() + granted);
            }
        }
        for (id, task) in switches {
            if let Some(entity) = self.find_mut(id) {
                entity.task = task;
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
                kind,
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
                let slot = &mut self.stock[slot_player][kind.index()];
                *slot = slot.saturating_add(u32::try_from(whole).unwrap_or(u32::MAX));
            }
            let node_left = self.find(node).map(|n| n.amount.raw()).unwrap_or(0);
            let scale = self.scale;
            if let Some(entity) = self.find_mut(id) {
                entity.carry = Fixed::from_raw(raw - whole * scale);
                entity.task = if node_left > 0 {
                    Task::Gather {
                        node,
                        rate_per_tick,
                        carry_cap,
                    }
                } else {
                    Task::Idle
                };
            }
        }
        for id in finished {
            if let Some(entity) = self.find_mut(id) {
                entity.task = Task::Idle;
            }
        }

        for (id, dx, dy) in deltas {
            let entity = self
                .find_mut(id)
                .expect("entity list is stable within a tick");
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
        }

        self.tick += 1;
        Ok(())
    }

    /// Canonical hash over the state. Encoding is explicit little-endian: no `usize`, no
    /// platform-dependent layout, so the number means the same thing everywhere.
    pub fn state_hash(&self) -> u64 {
        let mut hash = FNV_OFFSET_BASIS;
        fnv_bytes(&mut hash, &self.tick.to_le_bytes());
        fnv_bytes(&mut hash, &(self.entities.len() as u32).to_le_bytes());
        for row in &self.stock {
            for value in row {
                fnv_bytes(&mut hash, &value.to_le_bytes());
            }
        }
        for entity in &self.entities {
            fnv_bytes(&mut hash, &entity.id.0.to_le_bytes());
            fnv_bytes(&mut hash, &entity.player.to_le_bytes());
            let (tag, resource) = match entity.kind {
                EntityKind::Building => (b'b', 0u8),
                EntityKind::Unit => (b'u', 0),
                EntityKind::Resource(kind) => (
                    b'r',
                    match kind {
                        ResourceKind::Wood => 1,
                        ResourceKind::Food => 2,
                        ResourceKind::Gold => 3,
                        ResourceKind::Stone => 4,
                    },
                ),
            };
            fnv_bytes(&mut hash, &[tag, resource]);
            fnv_bytes(&mut hash, &entity.amount.raw().to_le_bytes());
            fnv_bytes(&mut hash, &entity.carry.raw().to_le_bytes());
            match entity.task {
                Task::Idle => fnv_bytes(&mut hash, &[0u8]),
                Task::Return {
                    node,
                    drop,
                    kind,
                    rate_per_tick,
                    carry_cap,
                } => {
                    fnv_bytes(&mut hash, &[2u8]);
                    fnv_bytes(&mut hash, &node.0.to_le_bytes());
                    fnv_bytes(&mut hash, &drop.0.to_le_bytes());
                    fnv_bytes(&mut hash, &[kind.index() as u8]);
                    fnv_bytes(&mut hash, &rate_per_tick.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &carry_cap.to_le_bytes());
                }
                Task::Gather {
                    node,
                    rate_per_tick,
                    carry_cap,
                } => {
                    fnv_bytes(&mut hash, &[1u8]);
                    fnv_bytes(&mut hash, &node.0.to_le_bytes());
                    fnv_bytes(&mut hash, &rate_per_tick.raw().to_le_bytes());
                    fnv_bytes(&mut hash, &carry_cap.to_le_bytes());
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
}
