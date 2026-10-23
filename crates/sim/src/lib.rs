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

use ruage_data::{CommandApply, Sim as Settings};
use ruage_fixed::Fixed;

/// Stable handle for an entity. Ordered, so iteration by id is just sorting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u32);

/// The subset of an entity the tick loop touches today. Fixed-point only (D9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entity {
    pub id: EntityId,
    pub position: (Fixed, Fixed),
    pub velocity: (Fixed, Fixed),
    pub hp: i32,
}

/// Inputs to the simulation. A replay is a list of these with the ticks they land on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Spawn { id: EntityId, speed: Fixed, hp: i32 },
}

#[derive(Debug)]
pub enum SimError {
    TooManyEntities { max: u32 },
    DuplicateEntity(EntityId),
    Displacement { id: EntityId, detail: String },
    Overflow(EntityId),
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

pub struct World {
    tick: u64,
    tick_rate_hz: u32,
    max_entities: u32,
    command_apply: CommandApply,
    entities: Vec<Entity>,
    queue: Vec<Command>,
    sim_rng: Pcg32,
    map_rng: Pcg32,
}

impl World {
    pub fn new(settings: &Settings, seed: u64) -> Result<Self, SimError> {
        Ok(World {
            tick: 0,
            tick_rate_hz: settings.tick_rate_hz,
            max_entities: settings.max_entities,
            command_apply: settings.command_apply,
            entities: Vec::new(),
            queue: Vec::new(),
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

    /// Entities in id order. The order is the invariant, not an implementation detail.
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    pub fn position(&self, id: EntityId) -> Option<(Fixed, Fixed)> {
        self.find(id).map(|e| e.position)
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

    /// Spawn outside the tick loop. Tests and tooling use this; the sim itself spawns through
    /// commands. Panics on a duplicate id or a full world: those are programming errors.
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
        self.apply(Command::Spawn { id, speed, hp })
    }

    fn apply(&mut self, command: Command) -> Result<(), SimError> {
        match command {
            Command::Spawn { id, speed, hp } => {
                if self.find(id).is_some() {
                    return Err(SimError::DuplicateEntity(id));
                }
                if self.entities.len() >= self.max_entities as usize {
                    return Err(SimError::TooManyEntities {
                        max: self.max_entities,
                    });
                }
                let entity = Entity {
                    id,
                    position: (Fixed::ZERO, Fixed::ZERO),
                    velocity: (speed, Fixed::ZERO),
                    hp,
                };
                // Insert by id: the vector is the iteration order.
                let at = self
                    .entities
                    .iter()
                    .position(|e| e.id > id)
                    .unwrap_or(self.entities.len());
                self.entities.insert(at, entity);
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
            self.apply(command)?;
        }

        let tick_rate = i64::from(self.tick_rate_hz);
        let mut deltas: Vec<(EntityId, Fixed, Fixed)> = Vec::with_capacity(self.entities.len());
        for entity in &self.entities {
            let dx =
                entity
                    .velocity
                    .0
                    .div_exact(tick_rate)
                    .map_err(|err| SimError::Displacement {
                        id: entity.id,
                        detail: err.to_string(),
                    })?;
            let dy =
                entity
                    .velocity
                    .1
                    .div_exact(tick_rate)
                    .map_err(|err| SimError::Displacement {
                        id: entity.id,
                        detail: err.to_string(),
                    })?;
            deltas.push((entity.id, dx, dy));
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
        for entity in &self.entities {
            fnv_bytes(&mut hash, &entity.id.0.to_le_bytes());
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
                speed: Fixed::parse("0.8", S).unwrap(),
                hp: 25,
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
            speed: Fixed::ZERO,
            hp: 10,
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
            speed: Fixed::ZERO,
            hp: 10,
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
