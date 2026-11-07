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

use ruage_data::Policy;

use ruage_fixed::Fixed;

use crate::{Command, EntityId, EntityKind, Task, World};

/// The scenario format's own version. It is not a config file: it describes a match, so it moves
/// at its own pace and does not share `config/`'s schema constant.
///
/// Version 2: a step names **roles** (`role = "house"`), not the dat's unit ids. The economy's
/// `[role]` table is the vocabulary, so a scenario for a pack of our own is writable at all (D17).
pub const SCENARIO_SCHEMA: u32 = 2;

/// A personality's own version. It moves at its own pace like the scenario's, and for the same reason: `config/ai/` is
/// behaviour, and a rule's shape may change without a match's format changing.
pub const PERSONALITY_SCHEMA: u32 = 1;

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
    /// The **personality** this match plays (D94): a name resolved beside the scenario, as `ai/<name>.toml`. Behaviour
    /// lives there, not here - a scenario keeps what is scripted for this match, and one opening serves many matches.
    /// A scenario may use `ai` or carry its own `[[rule]]` blocks, never both: two sources for one AI is a file nobody
    /// can read.
    #[serde(default)]
    pub ai: Option<String>,
    /// The villager-share table this match plays with (D94). Empty means "no policy": the engine then never moves a
    /// villager on its own. It arrives from the personality, or from a scenario that writes its own rules - and the
    /// loader refuses a scenario that names a personality *and* carries a table, for the same reason it refuses two
    /// rule sources.
    #[serde(default)]
    pub policy: Policy,
}

/// A personality: the rules that decide what a player *does* (D94). Its own file, its own schema, because it is
/// behaviour and the rest of `config/` is the world.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Personality {
    pub schema: u32,
    #[serde(default)]
    pub rule: Vec<Rule>,
    /// Where the villagers go, per villager count (`[policy]`, D75's shape). Shares are behaviour - "how many on wood"
    /// is exactly what a personality decides - so the table lives here and not in `economy.toml` (D94).
    #[serde(default)]
    pub policy: Policy,
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
    /// How many **military** units the player must have, by `[metrics] military_roles`: "gather an army, then attack"
    /// (the owner, 2026-11-03). A count, not `has_unit`, because one soldier is not an army; and `army_at_most` so a
    /// training rule can stop at a size instead of spending the whole larder.
    #[serde(default)]
    pub army_at_least: Option<usize>,
    #[serde(default)]
    pub army_at_most: Option<usize>,
    /// Free population before the cap (`cap - population`). A house rule is "somewhere under three", not "no house".
    #[serde(default)]
    pub housing_at_most: Option<u32>,
    #[serde(default)]
    pub stock: Vec<StockWhen>,
    /// How many villagers **work a resource**, which is what a drop-site rule has to see: a mining camp beside the gold
    /// waits for a villager on the gold, not for the bank to be low (item 4). Counted from the gather and trip-home tasks'
    /// nodes, so a worker carrying its load home still counts.
    #[serde(default)]
    pub working: Vec<WorkingWhen>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkingWhen {
    pub resource: String,
    #[serde(default)]
    pub at_least: Option<usize>,
    #[serde(default)]
    pub at_most: Option<usize>,
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
    /// **Send every idle unit of a role at the nearest ground the player has never seen.** The owner's note (2026-11-04):
    /// "player should start with scout cavalry, villagers shouldn't scout". The role is the whole guard - a villager is not
    /// listed, so a villager cannot be sent scouting by this rule, and a pack that wants its villagers to explore says so.
    Explore {
        role: String,
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
    /// `"near_town_center"`, which is the default. The field was parsed and **ignored** before this: `where` was
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
    /// enemy unit), `"enemy_building"` (their first building), or `"enemy_town_center"` (the building of the start's
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
    /// How long the scenario runs for, resolved once from its duration and the tick rate.
    duration_ticks: u64,
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

/// **A time becomes a tick, or the run is refused** (second review, `script.rs:287` and `:307`).
///
/// `f64 as i64` **saturates**, so the old `u64::try_from(tick as i64)` could never fail and its "does not fit in a tick"
/// message was dead code: a step at 1e300 seconds was accepted as 9.2e18 ticks, which is a run that can never reach it. The
/// check has to happen **before** the cast, in the same words the refusal already used.
fn tick_at(seconds: f64, tick_rate_hz: u32, at_least: u64, what: &str) -> Result<u64, String> {
    let ticks = (seconds * f64::from(tick_rate_hz)).round();
    if !ticks.is_finite() || ticks < 0.0 || ticks > i64::MAX as f64 {
        return Err(format!("{what} at {seconds} s does not fit in a tick"));
    }
    Ok(((ticks as i64) as u64).max(at_least))
}

impl Scenario {
    /// **How many ticks this scenario runs for**, bounded to a real tick count (second review [74] [173] [184]).
    ///
    /// One definition, used by both runners and by `Runner::new`: the inline `(duration * rate).round() as i64` that the
    /// two binaries each carried turned a huge duration into `i64::MAX` and a `NaN` into zero without saying anything.
    pub fn ticks(&self, tick_rate_hz: u32) -> Result<u64, String> {
        tick_at(self.duration_seconds, tick_rate_hz, 0, "duration_seconds")
    }
}

impl Runner {
    pub fn new(scenario: &Scenario, tick_rate_hz: u32) -> Result<Self, String> {
        let duration_ticks = scenario.ticks(tick_rate_hz)?;
        // **A runner needs a clock** (second review, `script.rs:271`): at zero ticks a second every time rounds to tick 0,
        // and the reporting path divides by the rate. A runner that cannot keep time is a broken run, not a quiet one.
        if tick_rate_hz == 0 {
            return Err(
                "a runner needs a tick rate above zero: nothing can happen at zero ticks a second"
                    .to_owned(),
            );
        }
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
            let tick = tick_at(step.at_seconds, tick_rate_hz, 0, "a step")?;
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
            let at = tick_at(rule.at_seconds, tick_rate_hz, 0, "a rule")?;
            let cool = tick_at(rule.cooldown_seconds, tick_rate_hz, 1, "a cooldown")?;
            rules.push((at, cool, rule.clone()));
        }
        let rules_len = rules.len();
        Ok(Self {
            steps,
            next: 0,
            duration_ticks,
            rules,
            fired: BTreeMap::new(),
            fired_times: vec![0; rules_len],
            rule_refusals: Vec::new(),
            outcomes: Vec::new(),
        })
    }

    /// How many ticks this scenario runs for - the number the runners used to compute for themselves.
    #[must_use]
    pub fn duration_ticks(&self) -> u64 {
        self.duration_ticks
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
                Ok(commands) => {
                    for command in commands {
                        if let Err(err) = world.submit_checked(command) {
                            refusals.push((now as f64 / second, err.to_string()));
                        }
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
            // **A farm goes in the ring**, not beside the berries: the plots belong around the town centre and then around the
            // mill, and `where = "food"` on a farm rule is the pack saying what it feeds on, not where it stands. The step and
            // the rule resolvers share this branch, because a role means the same thing in both.
            let at = if world.farm_unit() == Some(unit) {
                farm_site(world, player, unit, food_variant(world), Some(worker))
            } else {
                match where_.as_deref() {
                    None | Some("near_town_center") => {
                        free_tile_near(world, player, unit, Some(worker))
                    }
                    Some(resource) => {
                        // A building placed for a resource is a building placed *at* that resource: a lumber camp beside
                        // the wood is the whole point of a lumber camp, and one beside the town centre is wood spent on a
                        // longer walk.
                        let slot = world.slot_of(resource).ok_or_else(|| {
                        format!(
                            "where = {resource:?} names no resource in the pack, so there is nothing to build beside"
                        )
                    })?;
                        let centre = town_center_of(world, player).ok_or_else(|| {
                            format!("player {player} has no town centre to measure a walk from")
                        })?;
                        // **A camp where a drop site already serves the resource is a wasted camp.** The reference
                        // implementation's rule (`kCovered = 5` tiles, the town centre not counting) skips nodes an existing
                        // drop site already serves and takes the next; farms are not drop sites, so their near-the-mill rule is
                        // untouched by this.
                        let variant = world.entity(worker).and_then(|entity| entity.variant);
                        let spacing = world.drop_site_spacing();
                        let camp = world.build_spaces_drop_sites(role);
                        let mut chosen = None;
                        let here = world.position(centre);
                        let candidates = match here {
                            Some(here) => nearest_nodes(world, here, slot, 16),
                            None => Vec::new(),
                        };
                        for candidate in candidates {
                            let covered = camp
                                && world.position(candidate).is_some_and(|position| {
                                    world.drop_site_within(
                                        player,
                                        variant,
                                        world.tile_of_position(position),
                                        spacing,
                                        true,
                                    )
                                });
                            if covered {
                                continue;
                            }
                            // The walk is the expensive question, so it is asked last, and only of a node that survived every cheap test.
                            if here.is_none_or(|here| {
                                world
                                    .position(candidate)
                                    .is_some_and(|at| world.can_reach(here, at))
                            }) {
                                chosen = Some(candidate);
                                break;
                            }
                        }
                        let node = chosen.ok_or_else(|| {
                        format!(
                            "player {player} has no {resource} node that a drop site does not already serve"
                        )
                    })?;
                        free_tile_near_node(world, node, unit, player, Some(worker))
                    }
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
                    // The **family**: a step that names militia after the upgrade must find the man-at-arms (second
                    // review, `script.rs:583`), or the step refuses with "no unit militia" while the unit stands there.
                    entity.player == player
                        && entity.kind == EntityKind::Unit
                        && entity
                            .unit
                            .is_some_and(|unit| world.is_family(unit, attacker_unit))
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

/// **The nearest nodes of a kind, sorted by (distance, id), with no reachability test at all.** Reachability is a path
/// search per node, so a caller that walks a list of candidates must not pay it for nodes it will never choose: the camp
/// rule used to call `nearest_resource` sixteen times, each of which sorted every node on the map *and* path-searched to
/// each - 840 ms per firing, every five seconds, in the late match (measured with `RUAGE_PROFILE_RULES`).
fn nearest_nodes(world: &World, from: (Fixed, Fixed), slot: usize, limit: usize) -> Vec<EntityId> {
    let mut nodes: Vec<(i64, u32, EntityId)> = world
        .entities()
        .iter()
        .filter(|entity| entity.kind == EntityKind::Resource(slot))
        .map(|entity| {
            let dx = entity.position.0.raw() - from.0.raw();
            let dy = entity.position.1.raw() - from.1.raw();
            (dx * dx + dy * dy, entity.id.0, entity.id)
        })
        .collect();
    nodes.sort_unstable();
    nodes.truncate(limit);
    nodes.into_iter().map(|(_, _, id)| id).collect()
}

/// The `nth` nearest node of a resource, **by distance alone** (D138, the owner's choice: *"B"*).
///
/// Ordered by (distance, id), so the choice is deterministic and two villagers told `nth = 0` and `nth = 1` land on
/// different nodes.
///
/// The engine's own picker is `World::nodes_of_slot`, and this asks it rather than keeping a second opinion: distance
/// order, **no reachability test** - `can_reach` is a full path search, and asking it of every candidate node once a
/// second per player cost **37x the whole run** (400 game-seconds: 319 ms -> 11.9 s, 2026-10-29) - with nodes that failed
/// remembered in `share_failed` and skipped until `share_retry_ticks` has passed. `start_gather` is the one authority on
/// whether a node can be worked, and it is the only place a search is spent.
///
/// The copy this replaces did ask `can_reach` of every node of the resource on every call, and it asked it of the node's
/// **own** position, which is blocked ground for a tree - so the question was the expensive one *and* the wrong one.
fn nearest_resource(world: &World, from: EntityId, slot: usize, nth: usize) -> Option<EntityId> {
    // The picker asks **for the worker**, not for the world (D148): a rule may not use what its player has not scouted,
    // and it looks no further than the worker's own reach (D149).
    world.nodes_of_slot(from, slot, nth + 1).get(nth).copied()
}

/// The first free tile within two of this player's town centre, in a fixed scan order.
/// The player's own town centre, by the start's role rather than by an id.
fn town_center_of(world: &World, player: u8) -> Option<EntityId> {
    let centre_unit = world.start_town_center_unit()?;
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
///
/// **And one the builder can walk to.** A site that fits and leaves a lane can still be inside a wood pocket: the camp is
/// ordered, paid for, and refused at apply with *"no walkable route ... the target is walled off"* - measured, the first
/// gold mining camp this personality ever ordered. Reachability is the last test because it is the expensive one.
fn free_tile_near_node(
    world: &World,
    node: EntityId,
    unit: u32,
    player: u8,
    builder: Option<EntityId>,
) -> Option<(u16, u16)> {
    let position = world.position(node)?;
    let scale = world.scale();
    let origin = (
        u16::try_from(position.0.raw() / scale).ok()?,
        u16::try_from(position.1.raw() / scale).ok()?,
    );
    // **Wide enough for a footprint, not for a tile.** The search used to span four tiles, which is plenty for a point and
    // too little for a farm: a 3x3 plot needs its whole footprint free, and around a bush among trees a four-tile ring
    // found a corner at best - 46 refusals of "no free tile for a farm" in one acceptance run.
    let ring = Ring::of(world, player, unit);
    for radius in 1..=8u16 {
        for dy in 0..=radius {
            for dx in 0..=radius {
                for (x, y) in [
                    (origin.0 + dx, origin.1 + dy),
                    (origin.0 + dx, origin.1.wrapping_sub(dy)),
                    (origin.0.wrapping_sub(dx), origin.1 + dy),
                    (origin.0.wrapping_sub(dx), origin.1.wrapping_sub(dy)),
                ] {
                    if world.is_free_ground(x, y)
                        && world.site_fits(player, unit, (x, y))
                        && world.site_has_lane(player, unit, (x, y))
                        && ring.allows((x, y))
                        && reachable(world, builder, (x, y))
                    {
                        return Some((x, y));
                    }
                }
            }
        }
    }
    None
}

/// **Where a building may stand relative to the town centre**, in the farm ring's own terms (the owner's note, 2026-11-04:
/// "houses and other buildings should be constructed in a way to allow such farm placement").
///
/// A farm fills the ring and may stand anywhere; a drop site may stand in its outer half - the mill is *two farm widths out*,
/// which is the whole point of the ring; anything else stands outside it. A pack with no `[build] farm` has no ring and every
/// building is free, which is what this returned before the rule existed.
///
/// Built **once per placement**, not once per candidate tile: `is_drop_site` walks the whole unit table, and asking it for
/// 289 candidate tiles per site search was the difference between a 36-second acceptance run and a 104-second one.
struct Ring {
    centre: Option<(i64, i64)>,
    farm: Option<u32>,
    unit: u32,
    drop: bool,
    far: i64,
    near: i64,
}

impl Ring {
    fn of(world: &World, player: u8, unit: u32) -> Self {
        let far = i64::from(world.farm_ring_end());
        let scale = world.scale();
        Ring {
            centre: town_center_of(world, player)
                .and_then(|id| world.position(id))
                .map(|at| (at.0.raw() / scale, at.1.raw() / scale)),
            farm: world.farm_unit(),
            unit,
            drop: far > 0 && world.is_drop_site(unit),
            far,
            near: far.saturating_sub(2),
        }
    }

    /// Whether a building of this kind may stand here.
    fn allows(&self, site: (u16, u16)) -> bool {
        if self.far == 0 || self.farm == Some(self.unit) {
            return true;
        }
        let Some((cx, cy)) = self.centre else {
            return true;
        };
        let (dx, dy) = (i64::from(site.0) - cx, i64::from(site.1) - cy);
        let distance = dx * dx + dy * dy;
        // Squared, so the comparison is exact and no rounding decides a site.
        if self.drop {
            distance >= self.near * self.near
        } else {
            distance >= self.far * self.far
        }
    }
}

/// The worker variant that carries **food**, from the pack's own gather table (`[gather.variant]`): the farm ring is drawn
/// around the buildings that variant may hand a load in at, which is the pack saying "where does this go" rather than the
/// engine assuming a mill.
fn food_variant(world: &World) -> Option<u32> {
    world.gather_variant("food")
}

/// **A farm site: in the ring around the town centre first, then around the mill.** The owner's picture - farms around the
/// centre, and the next ring around the mill right next to them - so the search is by distance from the centre outward, and a
/// mill is tried only when the centre's ring is full.
fn farm_site(
    world: &World,
    player: u8,
    unit: u32,
    variant: Option<u32>,
    builder: Option<EntityId>,
) -> Option<(u16, u16)> {
    let mut origins: Vec<(u16, u16)> = Vec::new();
    let scale = world.scale();
    // **The town centre and the buildings that take food** - not every camp on the map. A ring drawn around a lumber camp put
    // farms at the far side of the base with a long walk to any drop site; the food worker's own `drop_sites` list is the
    // pack's answer to "where does this load go" and it is already the edge the return path follows.
    if let Some(centre) = town_center_of(world, player).and_then(|id| world.position(id)) {
        origins.push((
            u16::try_from(centre.0.raw() / scale).ok()?,
            u16::try_from(centre.1.raw() / scale).ok()?,
        ));
    }
    for site in world.drop_sites_of(player, variant) {
        let Some(at) = world.position(site) else {
            continue;
        };
        if let (Ok(x), Ok(y)) = (
            u16::try_from(at.0.raw() / scale),
            u16::try_from(at.1.raw() / scale),
        ) {
            origins.push((x, y));
        }
    }
    // **Gather first, walk last.** `reachable` is a path search, so asking it per candidate turned one farm order into seconds:
    // 256 candidates times an A* is how a 3-second rule became a five-minute match. Geometry is cheap and can be checked for
    // every tile; the builder's walk is the expensive question and belongs on the handful of sites that survived.
    let mut candidates: Vec<(u16, u16)> = Vec::new();
    for origin in origins {
        for radius in 1..=10u16 {
            for dy in 0..=radius {
                for dx in 0..=radius {
                    for (x, y) in [
                        (origin.0 + dx, origin.1 + dy),
                        (origin.0 + dx, origin.1.wrapping_sub(dy)),
                        (origin.0.wrapping_sub(dx), origin.1 + dy),
                        (origin.0.wrapping_sub(dx), origin.1.wrapping_sub(dy)),
                    ] {
                        if world.is_free_ground(x, y)
                            && world.site_fits(player, unit, (x, y))
                            && world.site_has_lane(player, unit, (x, y))
                        {
                            candidates.push((x, y));
                        }
                        if candidates.len() >= 8 {
                            break;
                        }
                    }
                }
            }
            if candidates.len() >= 8 {
                break;
            }
        }
        if candidates.len() >= 8 {
            break;
        }
    }
    candidates
        .into_iter()
        .find(|site| reachable(world, builder, *site))
}

/// Can `builder` walk to that tile? A site elsewhere on the map is a site nobody can raise.
fn reachable(world: &World, builder: Option<EntityId>, at: (u16, u16)) -> bool {
    let Some(builder) = builder else {
        return true;
    };
    world
        .position(builder)
        .is_some_and(|from| world.can_reach(from, world.tile(at)))
}

fn free_tile_near(
    world: &World,
    player: u8,
    unit: u32,
    builder: Option<EntityId>,
) -> Option<(u16, u16)> {
    // The start's own town-centre role, not the word: a pack of our own may call it "keep" (D17).
    let centre_unit = world.start_town_center_unit()?;
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
    // **Leave the ground near the town centre for farms**, which is the manual's advice for houses - *"you should not
    // build your first house too close to the Town Center; make sure there will be enough room for farms later"* - and
    // the reason a base that packs itself against the centre has nowhere left to put a plot. Radius 2-3 is tried only if
    // nothing further out fits.
    let ring = Ring::of(world, player, unit);
    for radius in 4..=12u16 {
        for dy in 0..=radius {
            for dx in 0..=radius {
                for (x, y) in [
                    (origin.0 + dx, origin.1 + dy),
                    (origin.0 + dx, origin.1.wrapping_sub(dy)),
                    (origin.0.wrapping_sub(dx), origin.1 + dy),
                    (origin.0.wrapping_sub(dx), origin.1.wrapping_sub(dy)),
                ] {
                    if world.is_free_ground(x, y)
                        && world.site_fits(player, unit, (x, y))
                        && world.site_has_lane(player, unit, (x, y))
                        && ring.allows((x, y))
                        && reachable(world, builder, (x, y))
                    {
                        return Some((x, y));
                    }
                }
            }
        }
    }
    // Nothing at arm's length: come back in close, because a building is better than none.
    for radius in 2..=3u16 {
        for dy in 0..=radius {
            for dx in 0..=radius {
                for (x, y) in [
                    (origin.0 + dx, origin.1 + dy),
                    (origin.0 + dx, origin.1.wrapping_sub(dy)),
                    (origin.0.wrapping_sub(dx), origin.1 + dy),
                    (origin.0.wrapping_sub(dx), origin.1.wrapping_sub(dy)),
                ] {
                    if world.is_free_ground(x, y)
                        && world.site_fits(player, unit, (x, y))
                        && world.site_has_lane(player, unit, (x, y))
                        && ring.allows((x, y))
                        && reachable(world, builder, (x, y))
                    {
                        return Some((x, y));
                    }
                }
            }
        }
    }
    None
}

pub fn load_scenario_str(text: &str, path: &str) -> Result<Scenario, String> {
    let scenario: Scenario = toml::from_str(text).map_err(|err| format!("{path}: {err}"))?;
    // **A scenario's length is a length** (second review [74] [173] [184]): `f64 as i64` saturates, so a `NaN` duration
    // became a *zero-tick match* - it played nothing, exited 0 and reported success - and `1e300` became `i64::MAX`
    // ticks, a match that never ends. The product is bounded where the rate is known (`Scenario::ticks`); the two things
    // that can be judged without a clock are judged here, and the refusal names the field.
    if !scenario.duration_seconds.is_finite() || scenario.duration_seconds < 0.0 {
        return Err(format!(
            "{path}: duration_seconds must be a finite, non-negative number of seconds, not {}",
            scenario.duration_seconds
        ));
    }
    // `ai` names a file beside the scenario, so a scenario held only as text cannot honour it. Refusing is the honest
    // answer: silently playing with no rules is an AI that does nothing and a file that says it should do something.
    if let Some(name) = &scenario.ai {
        return Err(format!(
            "{path}: names personality {name:?}, which is resolved beside the file - load it with load_scenario_file"
        ));
    }
    Ok(scenario)
}

/// A personality's name, resolved inside `ai/`. It must therefore *be* a name: lowercase letters, digits, `-` and `_`.
///
/// This is not decoration. `Path::join` honours `..` and replaces the base for an absolute path, which was checked with
/// `rustc` against the real implementation: `"../economy"` gave `config/ai/../economy.toml` and `"/etc/passwd"` gave
/// `/etc/passwd.toml`. Neither is a personality, and a scenario that names one should hear that rather than get a
/// confusing "cannot be read" from a file it was never meant to reach.
fn personality_path(dir: &std::path::Path, name: &str) -> Result<std::path::PathBuf, String> {
    let valid = !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_');
    if !valid {
        return Err(format!(
            "{name:?} is not a personality name: use lowercase letters, digits, '-' and '_' (it is resolved inside ai/)"
        ));
    }
    Ok(dir.join("ai").join(format!("{name}.toml")))
}

/// Read a personality from its own file.
///
/// Public because a caller that installs an AI without a scenario - the frame-cost probe, or a tool that wants the base
/// AI's shares - must use the *same* parse as the scenario loader. One path, so a second caller cannot drift from it.
/// A personality with neither rules nor shares is refused: it would play the match doing nothing.
pub fn load_personality_file(path: &std::path::Path) -> Result<Personality, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("{} cannot be read: {err}", path.display()))?;
    let personality: Personality =
        toml::from_str(&text).map_err(|err| format!("personality {}: {err}", path.display()))?;
    if personality.schema != PERSONALITY_SCHEMA {
        return Err(format!(
            "personality {} is schema {}, this build reads {PERSONALITY_SCHEMA}",
            path.display(),
            personality.schema
        ));
    }
    if personality.rule.is_empty() && personality.policy.share.is_empty() {
        return Err(format!(
            "personality {} has no [[rule]] blocks and no [policy], so it would play the match doing nothing",
            path.display()
        ));
    }
    Ok(personality)
}

pub fn load_scenario_file(path: &std::path::Path) -> Result<Scenario, String> {
    let text = std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    let mut scenario: Scenario =
        toml::from_str(&text).map_err(|err| format!("{}: {err}", path.display()))?;
    let Some(name) = scenario.ai.clone() else {
        return Ok(scenario);
    };
    // One source for one AI: a scenario that names a personality supplies neither rules nor shares of its own. The
    // refusal says *which* behaviour it found, so the author does not have to guess what to delete.
    let own_behaviour = if !scenario.rule.is_empty() {
        Some("[[rule]] blocks")
    } else if !scenario.policy.share.is_empty() {
        Some("a [policy] table")
    } else {
        None
    };
    if let Some(what) = own_behaviour {
        return Err(format!(
            "{}: carries {what} *and* names personality {name:?} - the behaviour belongs in one place",
            path.display()
        ));
    }
    let dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let file = personality_path(dir, &name).map_err(|why| format!("{}: {why}", path.display()))?;
    let personality = load_personality_file(&file)
        .map_err(|why| format!("{}: names personality {name:?}, but {why}", path.display()))?;
    scenario.rule = personality.rule;
    scenario.policy = personality.policy;
    Ok(scenario)
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

    /// The unit id of the town centre a match starts from: the `start.town_center` role. Scripted
    /// steps place buildings beside it, and they must do that by role rather than by the word.
    pub fn start_town_center_unit(&self) -> Option<u32> {
        let economy = self.economy.as_ref()?;
        economy.role.get(&economy.start.town_center).copied()
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
        //
        // **And a spent farm is not ground that stands on itself** (D77): its husk is a floor, and the next farm is built
        // on it. This predicate used to say otherwise while `site_refusal` said the opposite, so the placement search
        // never offered a husk's tile and the farm rule refused with *"no free tile for a farm"* 105 times in one
        // acceptance run, with nine husks in its own base.
        !self.entities().iter().any(|entity| {
            entity.position == position
                && entity.kind != EntityKind::Unit
                && !self.is_expired_farm(entity.id)
        })
    }
}

/// Does a rule's condition hold? Every field that is present must hold; an empty condition always does.
fn when_holds(world: &World, rule: &Rule) -> bool {
    let when = &rule.when;
    let player = rule.player;
    let count = villagers_of(world, player).len();
    if let Some(least) = when.army_at_least {
        let army = world.army(player);
        if army < least {
            return false;
        }
    }
    if let Some(most) = when.army_at_most {
        let army = world.army(player);
        if army > most {
            return false;
        }
    }
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
        // **The family, not one id** (second review, `script.rs:1162`): `has_unit = "militia"` is still true when the
        // militia has been upgraded, which is the question the author asked. Exact-id matching made the condition go
        // silently false the moment the upgrade landed.
        let found = world.entities().iter().any(|entity| {
            entity.player == player
                && entity.kind == EntityKind::Unit
                && entity
                    .unit
                    .is_some_and(|unit| world.is_family(unit, wanted))
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
    for working in &when.working {
        let Some(slot) = world.slot_of(&working.resource) else {
            return false;
        };

        let count = world.workers_on(player, slot);
        if let Some(least) = working.at_least
            && count < least
        {
            return false;
        }
        if let Some(most) = working.at_most
            && count > most
        {
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

/// Turn a rule's action into the commands it orders. The rules share the steps' resolvers, so a role means the same
/// thing in both. **A list, not one command:** an attack is a body of soldiers, and ordering only the newest one left an
/// army standing at its barracks while a single mangonel fought (`an_attack_rule_sends_the_whole_army`).
fn resolve_rule(world: &World, rule: &Rule) -> Result<Vec<Command>, String> {
    let player = rule.player;
    match &rule.action {
        RuleAction::Train { building, role } => {
            let at = finished_building(world, player, building)
                .ok_or_else(|| format!("player {player} has no finished {building}"))?;
            let unit = role_of(world, role)?;
            Ok(vec![Command::Train { building: at, unit }])
        }
        RuleAction::Research { building, tech } => {
            let at = finished_building(world, player, building)
                .ok_or_else(|| format!("player {player} has no finished {building}"))?;
            let tech = world
                .tech_role(tech)
                .ok_or_else(|| format!("the content pack has no tech role {tech:?}"))?;
            Ok(vec![Command::Research { building: at, tech }])
        }
        RuleAction::Build { role, where_ } => {
            let worker = idle_villager(world, player)
                .ok_or_else(|| format!("player {player} has no idle villager to build with"))?;
            let unit = role_of(world, role)?;
            // **A farm goes in the ring**, not beside the berries: the plots belong around the town centre and then around the
            // mill, and `where = "food"` on a farm rule is the pack saying what it feeds on, not where it stands. The step and
            // the rule resolvers share this branch, because a role means the same thing in both.
            let at = if world.farm_unit() == Some(unit) {
                farm_site(world, player, unit, food_variant(world), Some(worker))
            } else {
                match where_.as_deref() {
                    None | Some("near_town_center") => {
                        free_tile_near(world, player, unit, Some(worker))
                    }
                    Some(resource) => {
                        let slot = world.slot_of(resource).ok_or_else(|| {
                            format!("the content pack defines no resource called {resource:?}")
                        })?;
                        let centre = town_center_of(world, player).ok_or_else(|| {
                            format!("player {player} has no town centre to measure a walk from")
                        })?;
                        // **A camp where a drop site already serves the resource is a wasted camp.** The reference
                        // implementation's rule (`kCovered = 5` tiles, the town centre not counting) skips nodes an existing
                        // drop site already serves and takes the next; farms are not drop sites, so their near-the-mill rule is
                        // untouched by this.
                        let variant = world.entity(worker).and_then(|entity| entity.variant);
                        let spacing = world.drop_site_spacing();
                        let camp = world.build_spaces_drop_sites(role);
                        let mut chosen = None;
                        let here = world.position(centre);
                        let candidates = match here {
                            Some(here) => nearest_nodes(world, here, slot, 16),
                            None => Vec::new(),
                        };
                        for candidate in candidates {
                            let covered = camp
                                && world.position(candidate).is_some_and(|position| {
                                    world.drop_site_within(
                                        player,
                                        variant,
                                        world.tile_of_position(position),
                                        spacing,
                                        true,
                                    )
                                });
                            if covered {
                                continue;
                            }
                            // The walk is the expensive question, so it is asked last, and only of a node that survived every cheap test.
                            if here.is_none_or(|here| {
                                world
                                    .position(candidate)
                                    .is_some_and(|at| world.can_reach(here, at))
                            }) {
                                chosen = Some(candidate);
                                break;
                            }
                        }
                        let node = chosen.ok_or_else(|| {
                        format!(
                            "player {player} has no {resource} node that a drop site does not already serve"
                        )
                    })?;
                        free_tile_near_node(world, node, unit, player, Some(worker))
                    }
                }
            };
            let at = at.ok_or_else(|| format!("no free tile for a {role}"))?;
            Ok(vec![Command::Build {
                villager: worker,
                unit,
                at,
            }])
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
            Ok(vec![Command::Gather {
                villager: worker,
                node: target,
            }])
        }
        RuleAction::Explore { role } => {
            let scout_unit = role_of(world, role)?;
            let mut commands = Vec::new();
            for entity in world.entities() {
                if entity.player != player || entity.kind != EntityKind::Unit {
                    continue;
                }
                if !entity
                    .unit
                    .is_some_and(|unit| world.is_family(unit, scout_unit))
                {
                    continue;
                }
                // Only an idle scout is sent: one already walking somewhere was given that order by a rule with a better
                // idea, and re-aiming it every cooldown would make it wander in place.
                if !matches!(entity.task, Task::Idle) {
                    continue;
                }
                let here = world.tile_of_position(entity.position);
                if let Some(to) = world.nearest_unseen(player, here, 26) {
                    commands.push(Command::Move {
                        unit: entity.id,
                        to,
                    });
                }
            }
            // **Nothing to do is not a refusal.** A scout that is already walking was sent somewhere by this same rule a
            // moment ago; reporting "no idle scout" every four seconds filled the run's refusal list with 389 lines that said
            // the rule was working. A role the pack does not have is still an error, and `role_of` says so above.
            Ok(commands)
        }
        RuleAction::Attack { attacker, target } => {
            let attacker_unit = role_of(world, attacker)?;
            // Family-matched: an upgrade is the same soldier. And **every** one of them, not the newest - the first
            // shape ordered `max()` by id, which sent a single unit while the rest of the army stood where it spawned.
            let army: Vec<EntityId> = world
                .entities()
                .iter()
                .filter(|entity| {
                    entity.player == player
                        && entity.kind == EntityKind::Unit
                        && entity
                            .unit
                            .is_some_and(|unit| world.is_family(unit, attacker_unit))
                })
                .map(|entity| entity.id)
                .collect();
            if army.is_empty() {
                return Err(format!("player {player} has no unit {attacker}"));
            }
            let mut commands = Vec::with_capacity(army.len());
            let mut first_refusal: Option<String> = None;
            for unit in army {
                // A target is chosen per soldier because reachability is part of the choice, exactly as it is for a
                // resource node: one soldier inside a wood cluster must not fail the whole order.
                match choose_target(world, unit, player, target) {
                    Ok(aim) => commands.push(Command::Attack {
                        attacker: unit,
                        target: aim,
                    }),
                    Err(reason) => {
                        first_refusal.get_or_insert(reason);
                    }
                }
            }
            if commands.is_empty() {
                return Err(first_refusal.unwrap_or_else(|| format!("no enemy matches {target:?}")));
            }
            Ok(commands)
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
        "enemy_town_center" => {
            let centre = world.start_town_center_unit().ok_or_else(|| {
                "the pack names no town centre, so there is none to aim at".to_owned()
            })?;
            // The town centre first; then **any** enemy building, then the nearest enemy unit. A mangonel that has already
            // razed the town centre leaves the rule with nothing to aim at - *"no enemy matches enemy_town_center"*, 64
            // times in the last run, while the enemy's villagers were still alive and the match was still on. The
            // selector now degrades instead of failing.
            let buildings = |family: Option<u32>| -> Option<EntityId> {
                world
                    .entities()
                    .iter()
                    .filter(|entity| {
                        entity.player != player
                            && entity.kind == EntityKind::Building
                            && entity.hp > 0
                            && family.is_none_or(|centre| {
                                entity
                                    .unit
                                    .is_some_and(|unit| world.is_family(unit, centre))
                            })
                    })
                    .map(|entity| entity.id)
                    .next()
            };
            buildings(Some(centre)).or_else(|| buildings(None))
        }
        other => return Err(format!("unknown target selector {other:?}")),
    };
    enemy
        .or_else(|| {
            // Last resort: the nearest enemy unit. A side with no buildings but a villager in a corner is still in the
            // match, and a rule that says "attack" must have something to attack.
            let here = world.position(unit);
            world
                .entities()
                .iter()
                .filter(|entity| {
                    entity.player != player && entity.player != 0 && entity.kind == EntityKind::Unit
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
        })
        .ok_or_else(|| format!("no enemy matches {target:?}"))
}
