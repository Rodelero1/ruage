//! A scenario: timed `Command`s that name *what*, not *which id*.
//!
//! This is the replay format for step 5 and the scripted opponent in one. Times are seconds (D16),
//! converted with the match's own tick rate, so the same script means the same match at any rate.
//!
//! Steps resolve selectors against the world when they are due — "the first villager", "the nearest
//! tree", "beside my town centre" — because ids belong to seeding order and would rot the moment the
//! map changed. A step that cannot be resolved is reported, not silently skipped.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{Command, EntityId, EntityKind, World};

/// The scenario format's own version. It is not a config file: it describes a match, so it moves
/// at its own pace and does not share `config/`'s schema constant.
///
/// Version 2: a step names **roles** (`role = "house"`), not the dat's unit ids. The economy's
/// `[role]` table is the vocabulary, so a scenario for a pack of our own is writable at all (D17).
pub const SCENARIO_SCHEMA: u32 = 2;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub schema: u32,
    pub duration_seconds: f64,
    /// Steps are optional: a scenario written entirely as rules has none, and a rule that trains villagers is a whole
    /// opening on its own.
    #[serde(default)]
    pub step: Vec<Step>,
    /// Rules are **additive**: a v2 file without any means exactly what it did, which is why they do not bump the
    /// schema. A timestamp says *when*; a rule says *while* - which is how the game's own AI writes a build order
    /// (`docs/ai-of-the-game.md`), and the reason a rule cannot go stale when the economy's speed changes.
    #[serde(default)]
    pub rule: Vec<Rule>,
}

/// One rule: a player, a condition, one action, and a cooldown so a satisfied condition does not spam orders.
#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    /// Not before this many game seconds. Default 0.
    #[serde(default)]
    pub at_seconds: f64,
    /// How often the rule may fire, in game seconds. Default 5: long enough that a train rule is a queue of one, short
    /// enough that a building rule reacts while the wood is still in hand.
    #[serde(default = "default_cooldown")]
    pub cooldown_seconds: f64,
    pub player: u8,
    #[serde(default)]
    pub when: When,
    #[serde(flatten)]
    pub action: RuleAction,
}

fn default_cooldown() -> f64 {
    5.0
}

/// Conditions, all of which must hold. Every field is optional; an empty `When` is always true.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct When {
    #[serde(default)]
    pub villagers_at_least: Option<usize>,
    #[serde(default)]
    pub villagers_at_most: Option<usize>,
    /// None of this role exists for the player - a **foundation counts**, so "no market" cannot fire twice while the
    /// first one is being built.
    #[serde(default)]
    pub no_building: Option<String>,
    /// Every one of these roles must exist for the player. A list, because TOML's inline tables cannot repeat a key and
    /// "market *and* blacksmith" is the shape a build order needs.
    #[serde(default)]
    pub has_building: Vec<String>,
    /// A `[tech_role]` that must be researched (an age, for a build order).
    #[serde(default)]
    pub age: Option<String>,
    /// A `[tech_role]` that must **not** be researched yet - "Loom, once" needs it, because a research rule with only a
    /// villager threshold would ask for Loom every cooldown for the rest of the match.
    #[serde(default)]
    pub no_tech: Option<String>,
    /// A role the player must have at least one **unit** of: "send the mangonel" waits for the mangonel.
    #[serde(default)]
    pub has_unit: Option<String>,
    /// Free population before the cap (`cap - population`). A house rule is "somewhere under three", not "no house".
    #[serde(default)]
    pub housing_at_most: Option<u32>,
    #[serde(default)]
    pub stock: Vec<StockWhen>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StockWhen {
    pub name: String,
    #[serde(default)]
    pub at_least: Option<u32>,
    #[serde(default)]
    pub at_most: Option<u32>,
}

/// What a rule does when it fires. The action picks its own worker (an idle villager) rather than naming one, because a
/// rule is re-evaluated and the villager it would have named has moved by the second firing.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case")]
pub enum RuleAction {
    Train {
        building: String,
        role: String,
    },
    Research {
        building: String,
        tech: String,
    },
    Build {
        role: String,
        #[serde(default, rename = "where")]
        where_: Option<String>,
    },
    Gather {
        resource: String,
        #[serde(default)]
        node: Option<String>,
    },
    Attack {
        attacker: String,
        target: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct Step {
    pub at_seconds: f64,
    pub player: u8,
    #[serde(flatten)]
    pub kind: StepKind,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepKind {
    /// Put one of this player's villagers on a resource, named as the content pack names it.
    Gather {
        villager: usize,
        resource: String,
        /// Which of the nearest nodes of that kind: 0 is nearest. Three villagers told 0, 1 and 2
        /// work three different trees instead of queueing on one and idling when it runs out.
        #[serde(default)]
        nth: usize,
        /// Work a **building** instead of a resource node: a role (`farm`), resolved to this player's nearest finished
        /// building of that family. Without it a script cannot use a farm at all - `nearest_resource` only finds
        /// `EntityKind::Resource`, and a farm is a building - which is exactly the food engine a 30-minute opening
        /// needs once the berries are gone.
        #[serde(default)]
        node: Option<String>,
    },
    /// Put up a building: a villager, the building's role, and **where** it goes.
    ///
    /// `where` is a resource name (the building lands on the free tile nearest the nearest such node) or
    /// `"near_town_centre"`, which is the default. The field was parsed and **ignored** before this: `where` was
    /// written in `scenario.toml`, `deny_unknown_fields` is not set on a step, and nothing read the key - so a script
    /// that asked for a lumber camp by the wood got a building beside the town centre and no complaint.
    Build {
        villager: usize,
        role: String,
        #[serde(default, rename = "where")]
        where_: Option<String>,
    },
    /// Train a unit at a named building. Both are roles (`[role]` in economy.toml).
    Train { building: String, role: String },
    /// Research a tech at a named building. The building is a role; the tech is a `[tech_role]`
    /// entry, because a tech has no name in the dat at all (D17).
    Research { building: String, tech: String },
    /// Send this player's newest unit of a role at something. `target` is a selector: `"nearest_enemy"` (the first
    /// enemy unit), `"enemy_building"` (their first building), or `"enemy_town_centre"` (the building of the start's
    /// town-centre family, which is what a mangonel is aimed at).
    Attack { attacker: String, target: String },
    /// Stand a villager down.
    Stop { villager: usize },
}

/// What happened to a step, kept so the run can be reported rather than guessed at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The step ran and the simulation accepted the order.
    Ran,
    /// The step was due but the world could not satisfy it. Carries the reason.
    Refused(String),
    /// The step has not come due yet.
    Pending,
}

/// Runs a scenario against a world, one tick at a time.
pub struct Runner {
    steps: Vec<(u64, u8, StepKind)>,
    next: usize,
    /// Rules with their resolved timings: not before, cooldown, and the tick each last fired (`u64::MAX` = never).
    rules: Vec<(u64, u64, Rule)>,
    /// The tick each rule last fired, by index.
    fired: BTreeMap<usize, u64>,
    /// A rule that could not run, and why: `(game seconds, reason)`. Separate from `outcomes`, because a rule retries and
    /// a refusal at 8 seconds is a schedule fact rather than a broken script - a *step* that is refused is the script's
    /// fault, and that distinction is what makes a refusal list readable.
    pub rule_refusals: Vec<(f64, String)>,
    /// How many times each rule has fired, refusals included: a rule's condition is a question about the world, and a
    /// count of firings is the only way a run can show that a rule is thrashing or asleep.
    pub fired_times: Vec<usize>,
    pub outcomes: Vec<(f64, Outcome)>,
}

impl Runner {
    pub fn new(scenario: &Scenario, tick_rate_hz: u32) -> Result<Self, String> {
        if scenario.schema != SCENARIO_SCHEMA {
            return Err(format!(
                "the scenario is schema {}, this build reads {SCENARIO_SCHEMA}",
                scenario.schema
            ));
        }
        let mut steps = Vec::with_capacity(scenario.step.len());
        for step in &scenario.step {
            if !step.at_seconds.is_finite() || step.at_seconds < 0.0 {
                return Err(format!(
                    "a step is due at {} seconds, which is not a time",
                    step.at_seconds
                ));
            }
            let tick = (step.at_seconds * f64::from(tick_rate_hz)).round();
            let tick = u64::try_from(tick as i64)
                .map_err(|_| format!("a step at {} s does not fit in a tick", step.at_seconds))?;
            steps.push((tick, step.player, step.kind.clone()));
        }
        steps.sort_by_key(|(tick, player, _)| (*tick, *player));
        let mut rules = Vec::with_capacity(scenario.rule.len());
        for rule in &scenario.rule {
            if !rule.at_seconds.is_finite() || rule.at_seconds < 0.0 {
                return Err(format!(
                    "a rule is not due until {} seconds, which is not a time",
                    rule.at_seconds
                ));
            }
            if !rule.cooldown_seconds.is_finite() || rule.cooldown_seconds <= 0.0 {
                return Err(format!(
                    "a rule has a cooldown of {} seconds, which is not a cooldown",
                    rule.cooldown_seconds
                ));
            }
            let at = ((rule.at_seconds * f64::from(tick_rate_hz)).round() as i64).max(0) as u64;
            let cool =
                ((rule.cooldown_seconds * f64::from(tick_rate_hz)).round() as i64).max(1) as u64;
            rules.push((at, cool, rule.clone()));
        }
        let rules_len = rules.len();
        Ok(Self {
            steps,
            next: 0,
            rules,
            fired: BTreeMap::new(),
            fired_times: vec![0; rules_len],
            rule_refusals: Vec::new(),
            outcomes: Vec::new(),
        })
    }

    pub fn pending(&self) -> bool {
        self.next < self.steps.len()
    }

    /// Run every step that is due at or before the current tick, then every rule whose condition holds and whose
    /// cooldown has expired.
    pub fn due(&mut self, world: &mut World) {
        self.due_steps(world);
        self.due_rules(world);
    }

    /// Fire the rules. A rule that fires is *reported* when it could not run, so a build order cannot silently stall.
    fn due_rules(&mut self, world: &mut World) {
        let now = world.tick();
        let second = f64::from(world.tick_rate_hz());
        let mut refusals: Vec<(f64, String)> = Vec::new();
        for index in 0..self.rules.len() {
            let (at, cooldown) = (self.rules[index].0, self.rules[index].1);
            if now < at {
                continue;
            }
            if let Some(last) = self.fired.get(&index)
                && now.saturating_sub(*last) < cooldown
            {
                continue;
            }
            // The condition and the order it resolves to, under one immutable borrow; the borrow ends before the
            // world is touched mutably.
            let resolved = {
                let (_, _, rule) = &self.rules[index];
                if when_holds(world, rule) {
                    Some(resolve_rule(world, rule))
                } else {
                    None
                }
            };
            let Some(resolved) = resolved else {
                continue;
            };
            self.fired.insert(index, now);
            if let Some(count) = self.fired_times.get_mut(index) {
                *count += 1;
            }
            match resolved {
                Ok(command) => {
                    if let Err(err) = world.submit_checked(command) {
                        refusals.push((now as f64 / second, err.to_string()));
                    }
                }
                Err(reason) => refusals.push((now as f64 / second, reason)),
            }
        }
        self.rule_refusals.extend(refusals);
    }

    fn due_steps(&mut self, world: &mut World) {
        while let Some((at, player, kind)) = self.steps.get(self.next).cloned() {
            if world.tick() < at {
                return;
            }
            let outcome = match resolve(world, player, &kind) {
                Ok(command) => match world.submit_checked(command) {
                    Ok(()) => Outcome::Ran,
                    Err(err) => Outcome::Refused(err.to_string()),
                },
                Err(reason) => Outcome::Refused(reason),
            };
            self.outcomes
                .push((at as f64 / f64::from(world.tick_rate_hz()), outcome));
            self.next += 1;
        }
    }
}

fn resolve(world: &World, player: u8, kind: &StepKind) -> Result<Command, String> {
    match kind {
        StepKind::Gather {
            villager,
            resource,
            nth,
            node,
        } => {
            let unit = nth_villager(world, player, *villager)
                .ok_or_else(|| format!("player {player} has no villager {villager}"))?;
            let slot = world.slot_of(resource).ok_or_else(|| {
                format!(
                    "the content pack defines no resource called {resource:?}, so this step names \
                     nothing a villager could gather"
                )
            })?;
            let node = match node {
                None => nearest_resource(world, unit, slot, *nth).ok_or_else(|| {
                    format!("player {player} has no {resource} node number {nth} to gather")
                })?,
                Some(role) => {
                    // A building being worked (a farm): the nearest finished one of that role.
                    let wanted = role_of(world, role)?;
                    let here = world.position(unit);
                    world
                        .entities()
                        .iter()
                        .filter(|entity| {
                            entity.player == player
                                && entity.kind == EntityKind::Building
                                && entity.construction.is_none()
                                && entity
                                    .unit
                                    .is_some_and(|unit| world.is_family(unit, wanted))
                        })
                        .min_by_key(|entity| match here {
                            Some(from) => {
                                let dx = entity.position.0.raw() - from.0.raw();
                                let dy = entity.position.1.raw() - from.1.raw();
                                (dx * dx + dy * dy, entity.id.0)
                            }
                            None => (0, entity.id.0),
                        })
                        .map(|entity| entity.id)
                        .ok_or_else(|| format!("player {player} has no finished {role} to work"))?
                }
            };
            Ok(Command::Gather {
                villager: unit,
                node,
            })
        }
        StepKind::Build {
            villager,
            role,
            where_,
        } => {
            let worker = nth_villager(world, player, *villager)
                .ok_or_else(|| format!("player {player} has no villager {villager}"))?;
            let unit = role_of(world, role)?;
            let at = match where_.as_deref() {
                None | Some("near_town_centre") => free_tile_near(world, player),
                Some(resource) => {
                    // A building placed for a resource is a building placed *at* that resource: a lumber camp beside
                    // the wood is the whole point of a lumber camp, and one beside the town centre is wood spent on a
                    // longer walk.
                    let slot = world.slot_of(resource).ok_or_else(|| {
                        format!(
                            "where = {resource:?} names no resource in the pack, so there is nothing to build beside"
                        )
                    })?;
                    let centre = town_centre_of(world, player).ok_or_else(|| {
                        format!("player {player} has no town centre to measure a walk from")
                    })?;
                    let node = nearest_resource(world, centre, slot, 0).ok_or_else(|| {
                        format!("player {player} has no {resource} node to build beside")
                    })?;
                    free_tile_near_node(world, node)
                }
            };
            let at =
                at.ok_or_else(|| format!("player {player} has no free ground for a {role}"))?;
            Ok(Command::Build {
                villager: worker,
                unit,
                at,
            })
        }
        StepKind::Train { building, role } => {
            let wanted = role_of(world, building)?;
            let unit = role_of(world, role)?;
            let site = world
                .entities()
                .iter()
                .find(|entity| {
                    entity.player == player
                        && entity.kind == EntityKind::Building
                        && entity
                            .unit
                            .is_some_and(|unit| world.is_family(unit, wanted))
                        && entity.construction.is_none()
                })
                .map(|entity| entity.id)
                .ok_or_else(|| format!("player {player} has no finished {building}"))?;
            Ok(Command::Train {
                building: site,
                unit,
            })
        }
        StepKind::Research { building, tech } => {
            let wanted = role_of(world, building)?;
            let tech_id = world.tech_role(tech).ok_or_else(|| {
                format!(
                    "{tech:?} is not a tech role in economy.toml's [tech_role] table, so no tech can \
                     be named by it"
                )
            })?;
            let site = world
                .entities()
                .iter()
                .find(|entity| {
                    entity.player == player
                        && entity.kind == EntityKind::Building
                        && entity
                            .unit
                            .is_some_and(|unit| world.is_family(unit, wanted))
                        && entity.construction.is_none()
                })
                .map(|entity| entity.id)
                .ok_or_else(|| format!("player {player} has no finished {building}"))?;
            Ok(Command::Research {
                building: site,
                tech: tech_id,
            })
        }
        StepKind::Attack { attacker, target } => {
            let attacker_unit = role_of(world, attacker)?;
            let unit = world
                .entities()
                .iter()
                .filter(|entity| {
                    entity.player == player
                        && entity.kind == EntityKind::Unit
                        && entity.unit == Some(attacker_unit)
                })
                .map(|entity| entity.id)
                .max()
                .ok_or_else(|| format!("player {player} has no unit {attacker}"))?;
            // Reachability is part of the choice, exactly as it is for a resource node: an enemy inside a wood cluster
            // cannot be walked to, and a step that names it fails the *run* at apply time rather than picking the next
            // one. This became visible when villagers learned to spread through the trees (item 9), which put an enemy
            // villager behind one.
            let enemy = choose_target(world, unit, player, target)?;
            Ok(Command::Attack {
                attacker: unit,
                target: enemy,
            })
        }
        StepKind::Stop { villager } => {
            let unit = nth_villager(world, player, *villager)
                .ok_or_else(|| format!("player {player} has no villager {villager}"))?;
            Ok(Command::Stop { unit })
        }
    }
}

/// A role name resolved through the content pack's economy, or a refusal that names it.
fn role_of(world: &World, role: &str) -> Result<u32, String> {
    world.role_unit(role).ok_or_else(|| {
        format!(
            "{role:?} is not a role in economy.toml's [role] table, so no unit can be named by it"
        )
    })
}

fn nth_villager(world: &World, player: u8, index: usize) -> Option<EntityId> {
    let villager_unit = world.start_villager_unit()?;
    world
        .entities()
        .iter()
        .filter(|entity| {
            entity.player == player
                && entity.kind == EntityKind::Unit
                && entity.unit == Some(villager_unit)
        })
        .map(|entity| entity.id)
        .nth(index)
}

/// The `nth` nearest node of a kind that the villager can actually walk to, ordered by
/// (distance, id) so the choice is deterministic and two villagers told `nth = 0` and `nth = 1` land
/// on different nodes.
///
/// Reachability is part of the choice, not a check afterwards: a tree behind a wall is not a tree
/// this villager can be sent to, and the script should pick the next one rather than refuse.
fn nearest_resource(world: &World, from: EntityId, slot: usize, nth: usize) -> Option<EntityId> {
    let here = world.position(from)?;
    let mut nodes: Vec<(i64, u32, EntityId)> = world
        .entities()
        .iter()
        .filter(|entity| entity.kind == EntityKind::Resource(slot))
        .filter(|entity| world.can_reach(here, entity.position))
        .map(|entity| {
            let dx = entity.position.0.raw() - here.0.raw();
            let dy = entity.position.1.raw() - here.1.raw();
            (dx * dx + dy * dy, entity.id.0, entity.id)
        })
        .collect();
    nodes.sort_unstable();
    nodes.get(nth).map(|(_, _, id)| *id)
}

/// The first free tile within two of this player's town centre, in a fixed scan order.
/// The player's own town centre, by the start's role rather than by an id.
fn town_centre_of(world: &World, player: u8) -> Option<EntityId> {
    let centre_unit = world.start_town_centre_unit()?;
    world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == player
                && entity.kind == EntityKind::Building
                && entity
                    .unit
                    .is_some_and(|unit| world.is_family(unit, centre_unit))
        })
        .map(|entity| entity.id)
}

/// The free tile nearest a resource node, for a building that exists to shorten a walk to it.
fn free_tile_near_node(world: &World, node: EntityId) -> Option<(u16, u16)> {
    let position = world.position(node)?;
    let scale = world.scale();
    let origin = (
        u16::try_from(position.0.raw() / scale).ok()?,
        u16::try_from(position.1.raw() / scale).ok()?,
    );
    for radius in 1..=4u16 {
        for dy in 0..=radius {
            for dx in 0..=radius {
                for (x, y) in [
                    (origin.0 + dx, origin.1 + dy),
                    (origin.0 + dx, origin.1.wrapping_sub(dy)),
                    (origin.0.wrapping_sub(dx), origin.1 + dy),
                    (origin.0.wrapping_sub(dx), origin.1.wrapping_sub(dy)),
                ] {
                    if world.is_free_ground(x, y) {
                        return Some((x, y));
                    }
                }
            }
        }
    }
    None
}

fn free_tile_near(world: &World, player: u8) -> Option<(u16, u16)> {
    // The start's own town-centre role, not the word: a pack of our own may call it "keep" (D17).
    let centre_unit = world.start_town_centre_unit()?;
    // **The family, not the id.** A town centre that has been upgraded to its Feudal variant (463) is not unit 109 any
    // more, and matching the id exactly made every placement beside it fail: *"no free tile for a market"*, 267 times,
    // with the town centre standing right there.
    let centre = world.entities().iter().find(|entity| {
        entity.player == player
            && entity.kind == EntityKind::Building
            && entity
                .unit
                .is_some_and(|unit| world.is_family(unit, centre_unit))
    })?;
    let scale = world.scale();
    let origin = (
        u16::try_from(centre.position.0.raw() / scale).ok()?,
        u16::try_from(centre.position.1.raw() / scale).ok()?,
    );
    // **Wide enough to still find ground.** Six tiles around a town centre is a base: seven houses, a mill, a lumber camp
    // and a dozen farms fill it, and then every later building is refused with *"no free tile"* - 267 times each for the
    // market and the blacksmith in the last full run, with 3,848 wood in the bank. The ring grows to twelve.
    for radius in 2..=12u16 {
        for dy in 0..=radius {
            for dx in 0..=radius {
                for (x, y) in [
                    (origin.0 + dx, origin.1 + dy),
                    (origin.0 + dx, origin.1.wrapping_sub(dy)),
                    (origin.0.wrapping_sub(dx), origin.1 + dy),
                    (origin.0.wrapping_sub(dx), origin.1.wrapping_sub(dy)),
                ] {
                    if world.is_free_ground(x, y) {
                        return Some((x, y));
                    }
                }
            }
        }
    }
    None
}

pub fn load_scenario_str(text: &str, path: &str) -> Result<Scenario, String> {
    toml::from_str(text).map_err(|err| format!("{path}: {err}"))
}

pub fn load_scenario_file(path: &std::path::Path) -> Result<Scenario, String> {
    let text = std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    load_scenario_str(&text, &path.display().to_string())
}

/// A step that could not run at all, for tests that want to know why.
pub fn describe(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Ran => "ran".to_owned(),
        Outcome::Refused(reason) => format!("refused: {reason}"),
        Outcome::Pending => "pending".to_owned(),
    }
}

impl World {
    /// The unit id a player's villagers use: the `start.villager` role, resolved through the
    /// economy's role table.
    pub fn start_villager_unit(&self) -> Option<u32> {
        let economy = self.economy.as_ref()?;
        economy.role.get(&economy.start.villager).copied()
    }

    /// The unit id of the town centre a match starts from: the `start.town_centre` role. Scripted
    /// steps place buildings beside it, and they must do that by role rather than by the word.
    pub fn start_town_centre_unit(&self) -> Option<u32> {
        let economy = self.economy.as_ref()?;
        economy.role.get(&economy.start.town_centre).copied()
    }

    /// Is this tile inside the map, walkable, and empty?
    pub fn is_free_ground(&self, x: u16, y: u16) -> bool {
        let Some(walk) = self.walk.as_ref() else {
            return false;
        };
        if !walk.contains(x, y) || walk.blocked(x, y) {
            return false;
        }
        let position = (
            ruage_fixed::Fixed::from_raw(i64::from(x) * self.scale),
            ruage_fixed::Fixed::from_raw(i64::from(y) * self.scale),
        );
        // **A unit does not make ground occupied.** Terrain and what stands on it permanently do; a villager is a person
        // who is somewhere for a moment, and treating people as obstacles left a player unable to place a market beside
        // their own town centre - 177 refusals in a row - once villagers started standing there to deposit.
        !self
            .entities()
            .iter()
            .any(|entity| entity.position == position && entity.kind != EntityKind::Unit)
    }
}

/// Does a rule's condition hold? Every field that is present must hold; an empty condition always does.
fn when_holds(world: &World, rule: &Rule) -> bool {
    let when = &rule.when;
    let player = rule.player;
    let count = villagers_of(world, player).len();
    if let Some(least) = when.villagers_at_least
        && count < least
    {
        return false;
    }
    if let Some(most) = when.villagers_at_most
        && count > most
    {
        return false;
    }
    if let Some(role) = when.no_building.as_deref() {
        match building_count(world, player, role) {
            // A role the pack cannot name is a bug in the content, not a condition that can be true.
            None => return false,
            // The condition: the player has none of them.
            Some(0) => {}
            // One stands already. This branch was missing, and it cost a match: the mill rule fired 170 times, the
            // lumber camp 140 and the barracks 65 in 1,000 game-seconds - each paying for a building it already had, and
            // each refusal eating the wood the *next* rule needed. `no_building` was only ever checked for "is this a
            // role at all".
            Some(_) => return false,
        }
    }
    for role in &when.has_building {
        if building_count(world, player, role).unwrap_or(0) == 0 {
            return false;
        }
    }
    if let Some(age) = when.age.as_deref() {
        let Some(tech) = world.tech_role(age) else {
            return false;
        };
        if !world.researched(player).contains(&tech) {
            return false;
        }
    }
    if let Some(tech) = when.no_tech.as_deref() {
        match world.tech_role(tech) {
            Some(tech) if world.researched(player).contains(&tech) => return false,
            None => return false,
            _ => {}
        }
    }
    if let Some(role) = when.has_unit.as_deref() {
        let Some(wanted) = world.role_unit(role) else {
            return false;
        };
        let found = world.entities().iter().any(|entity| {
            entity.player == player
                && entity.kind == EntityKind::Unit
                && entity.unit == Some(wanted)
        });
        if !found {
            return false;
        }
    }
    if let Some(most) = when.housing_at_most {
        let headroom = world
            .population_cap(player)
            .saturating_sub(world.population(player));
        if headroom > most {
            return false;
        }
    }
    for stock in &when.stock {
        let Some(slot) = world.slot_of(&stock.name) else {
            return false;
        };
        let amount = world.stock(player, slot);
        if let Some(least) = stock.at_least
            && amount < least
        {
            return false;
        }
        if let Some(most) = stock.at_most
            && amount > most
        {
            return false;
        }
    }
    true
}

/// How many buildings of a role a player has, **foundations included**: `None` when the pack cannot name the role,
/// `Some(0)` when it can and the player has none.
fn building_count(world: &World, player: u8, role: &str) -> Option<usize> {
    let wanted = world.role_unit(role)?;
    Some(
        world
            .entities()
            .iter()
            .filter(|entity| {
                entity.player == player
                    && entity.kind == EntityKind::Building
                    && entity
                        .unit
                        .is_some_and(|unit| world.is_family(unit, wanted))
            })
            .count(),
    )
}

/// This player's villagers, in seeding order.
fn villagers_of(world: &World, player: u8) -> Vec<EntityId> {
    let Some(villager) = world.start_villager_unit() else {
        return Vec::new();
    };
    world
        .entities()
        .iter()
        .filter(|entity| {
            entity.player == player
                && entity.kind == EntityKind::Unit
                && entity
                    .unit
                    .is_some_and(|unit| world.is_family(unit, villager))
        })
        .map(|entity| entity.id)
        .collect()
}

/// An idle villager of this player, or the one that has been idle longest - a rule names no villager because the
/// villager it would name has moved by the time the rule fires again.
fn idle_villager(world: &World, player: u8) -> Option<EntityId> {
    let all = villagers_of(world, player);
    // An idle villager first, then anybody who is not already building. The fallback is not a convenience: the share
    // table keeps every idle villager working within a second, so "find an idle villager" succeeds only at the start of
    // the game - which is how the acceptance run reached 35 villagers and 349 wood and then never built its market,
    // with every build rule refused for want of a free villager. The game's own answer is the same shape:
    // `up-assign-builders` takes builders out of the gathering pool.
    all.iter()
        .copied()
        .find(|id| {
            world
                .entity(*id)
                .is_some_and(|entity| matches!(entity.task, crate::Task::Idle))
        })
        .or_else(|| {
            all.iter().copied().find(|id| {
                world
                    .entity(*id)
                    .is_some_and(|entity| !matches!(entity.task, crate::Task::Build { .. }))
            })
        })
}

/// Turn a rule's action into a command. The rules share the steps' resolvers, so a role means the same thing in both.
fn resolve_rule(world: &World, rule: &Rule) -> Result<Command, String> {
    let player = rule.player;
    match &rule.action {
        RuleAction::Train { building, role } => {
            let at = finished_building(world, player, building)
                .ok_or_else(|| format!("player {player} has no finished {building}"))?;
            let unit = role_of(world, role)?;
            Ok(Command::Train { building: at, unit })
        }
        RuleAction::Research { building, tech } => {
            let at = finished_building(world, player, building)
                .ok_or_else(|| format!("player {player} has no finished {building}"))?;
            let tech = world
                .tech_role(tech)
                .ok_or_else(|| format!("the content pack has no tech role {tech:?}"))?;
            Ok(Command::Research { building: at, tech })
        }
        RuleAction::Build { role, where_ } => {
            let worker = idle_villager(world, player)
                .ok_or_else(|| format!("player {player} has no idle villager to build with"))?;
            let unit = role_of(world, role)?;
            let at = match where_.as_deref() {
                None | Some("near_town_centre") => free_tile_near(world, player),
                Some(resource) => {
                    let slot = world.slot_of(resource).ok_or_else(|| {
                        format!("the content pack defines no resource called {resource:?}")
                    })?;
                    let centre = town_centre_of(world, player).ok_or_else(|| {
                        format!("player {player} has no town centre to measure a walk from")
                    })?;
                    let node = nearest_resource(world, centre, slot, 0).ok_or_else(|| {
                        format!("player {player} has no {resource} node to build beside")
                    })?;
                    free_tile_near_node(world, node)
                }
            };
            let at = at.ok_or_else(|| format!("no free tile for a {role}"))?;
            Ok(Command::Build {
                villager: worker,
                unit,
                at,
            })
        }
        RuleAction::Gather { resource, node } => {
            let worker = idle_villager(world, player)
                .or_else(|| villagers_of(world, player).first().copied())
                .ok_or_else(|| format!("player {player} has no villager"))?;
            let target = match node {
                Some(role) => {
                    let wanted = role_of(world, role)?;
                    let here = world.position(worker);
                    world
                        .entities()
                        .iter()
                        .filter(|entity| {
                            entity.player == player
                                && entity.kind == EntityKind::Building
                                && entity.construction.is_none()
                                && entity
                                    .unit
                                    .is_some_and(|unit| world.is_family(unit, wanted))
                        })
                        .min_by_key(|entity| match here {
                            Some(from) => {
                                let dx = entity.position.0.raw() - from.0.raw();
                                let dy = entity.position.1.raw() - from.1.raw();
                                (dx * dx + dy * dy, entity.id.0)
                            }
                            None => (0, entity.id.0),
                        })
                        .map(|entity| entity.id)
                        .ok_or_else(|| format!("player {player} has no finished {role} to work"))?
                }
                None => {
                    let slot = world.slot_of(resource).ok_or_else(|| {
                        format!("the content pack defines no resource called {resource:?}")
                    })?;
                    nearest_resource(world, worker, slot, 0).ok_or_else(|| {
                        format!("player {player} has no {resource} node to gather")
                    })?
                }
            };
            Ok(Command::Gather {
                villager: worker,
                node: target,
            })
        }
        RuleAction::Attack { attacker, target } => {
            let attacker_unit = role_of(world, attacker)?;
            let unit = world
                .entities()
                .iter()
                .filter(|entity| {
                    entity.player == player
                        && entity.kind == EntityKind::Unit
                        && entity.unit == Some(attacker_unit)
                })
                .map(|entity| entity.id)
                .max()
                .ok_or_else(|| format!("player {player} has no unit {attacker}"))?;
            let aim = choose_target(world, unit, player, target)?;
            Ok(Command::Attack {
                attacker: unit,
                target: aim,
            })
        }
    }
}

/// The player's first **finished** building of a role.
fn finished_building(world: &World, player: u8, role: &str) -> Option<EntityId> {
    let wanted = role_of(world, role).ok()?;
    world
        .entities()
        .iter()
        .find(|entity| {
            entity.player == player
                && entity.kind == EntityKind::Building
                && entity.construction.is_none()
                && entity
                    .unit
                    .is_some_and(|unit| world.is_family(unit, wanted))
        })
        .map(|entity| entity.id)
}

/// Which enemy an attack step or rule means: a selector, resolved against the world, with reachability part of the
/// choice so a target behind a wood cluster cannot fail the run at apply time.
fn choose_target(
    world: &World,
    unit: EntityId,
    player: u8,
    target: &str,
) -> Result<EntityId, String> {
    let here = world.position(unit);
    let reachable = |entity: &crate::Entity| match here {
        Some(from) => world.can_reach(from, entity.position),
        None => true,
    };
    let enemy = match target {
        "nearest_enemy" => world
            .entities()
            .iter()
            .filter(|entity| {
                entity.player != player && entity.kind == EntityKind::Unit && entity.player != 0
            })
            .filter(|entity| reachable(entity))
            .map(|entity| entity.id)
            .next(),
        "enemy_building" => world
            .entities()
            .iter()
            .filter(|entity| {
                entity.player != player && entity.kind == EntityKind::Building && entity.player != 0
            })
            .map(|entity| entity.id)
            .next(),
        "enemy_town_centre" => {
            let centre = world.start_town_centre_unit().ok_or_else(|| {
                "the pack names no town centre, so there is none to aim at".to_owned()
            })?;
            world
                .entities()
                .iter()
                .filter(|entity| {
                    entity.player != player
                        && entity.kind == EntityKind::Building
                        && entity
                            .unit
                            .is_some_and(|unit| world.is_family(unit, centre))
                })
                .map(|entity| entity.id)
                .next()
        }
        other => return Err(format!("unknown target selector {other:?}")),
    };
    enemy.ok_or_else(|| format!("no enemy matches {target:?}"))
}
