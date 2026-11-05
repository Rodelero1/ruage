//! **What a worker is called while he works** (D121).
//!
//! The owner's ask (2026-11-04): *"For Villager: renamed to match the task he is doing"* - a villager on berries is a
//! **Forager**, on a farm a **Farmer**, on a sheep a **Shepherd**. The engine knows the task and the target; the words are
//! content, so they live in `config/jobs.toml` and a mod renames them without touching code (D17).
//!
//! **Two keys.** The target's **role** in `economy.toml` when it has one (`farm` -> Farmer), and otherwise the **column** the
//! target holds (`wood` -> Lumberjack). The reference pack's 40 roles cover buildings, animals and soldiers but no tree, bush
//! or mine, so a role-only key could not name a woodcutter - that is not a guess, it is what its role table contains. A role
//! wins when both match, so a pack that adds a role for its trees overrides the column entry without moving anything.
//!
//! An entry with neither key is refused at load: a rule that can never match is a rule the author meant to be a rule.

use serde::Deserialize;

use crate::ConfigError;

/// The schema this build reads.
pub const SUPPORTED_SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobDef {
    /// The target's role name in `economy.toml`, when the pack gives it one. Checked first.
    pub target: Option<String>,
    /// The name of the resource column the target holds. The fallback.
    pub column: Option<String>,
    /// What the worker is called: `"Forager"`.
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Jobs {
    pub entries: Vec<JobDef>,
}

impl Jobs {
    /// The word for a worker whose target is `role` (if any) and holds `column` (if any), or `None` when no entry matches -
    /// and then the caller keeps the pack's own name for the unit, which is what a villager with nothing to do is.
    #[must_use]
    pub fn name_for(&self, role: Option<&str>, column: Option<&str>) -> Option<&str> {
        // A role beats a column: it is the more specific statement, and it is the one a mod adds when it wants its own word.
        if let Some(role) = role
            && let Some(job) = self
                .entries
                .iter()
                .find(|job| job.target.as_deref() == Some(role))
        {
            return Some(&job.name);
        }
        let column = column?;
        self.entries
            .iter()
            .find(|job| job.column.as_deref() == Some(column))
            .map(|job| job.name.as_str())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    schema: u32,
    #[serde(default)]
    job: Vec<RawJob>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawJob {
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    column: Option<String>,
    name: String,
}

pub fn load_jobs_str(text: &str, path: &str) -> Result<Jobs, ConfigError> {
    let raw: RawFile = toml::from_str(text).map_err(|err| ConfigError::Parse {
        path: path.to_owned(),
        detail: err.to_string(),
    })?;
    if raw.schema != SUPPORTED_SCHEMA {
        return Err(ConfigError::Invalid {
            key: "schema".to_owned(),
            detail: format!(
                "{} is schema {}, this build reads {}",
                path, raw.schema, SUPPORTED_SCHEMA
            ),
        });
    }
    let mut entries = Vec::with_capacity(raw.job.len());
    for (index, job) in raw.job.into_iter().enumerate() {
        if job.target.is_none() && job.column.is_none() {
            return Err(ConfigError::Invalid {
                key: format!("job[{index}]"),
                detail: "an entry needs `target` (a role) or `column` (a resource), or it can never match".to_owned(),
            });
        }
        if job.name.trim().is_empty() {
            return Err(ConfigError::Invalid {
                key: format!("job[{index}].name"),
                detail: "a job name cannot be empty".to_owned(),
            });
        }
        entries.push(JobDef {
            target: job.target,
            column: job.column,
            name: job.name,
        });
    }
    Ok(Jobs { entries })
}

pub fn load_jobs_file(path: &std::path::Path) -> Result<Jobs, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|err| ConfigError::Io {
        path: path.display().to_string(),
        detail: err.to_string(),
    })?;
    load_jobs_str(&text, &path.display().to_string())
}
