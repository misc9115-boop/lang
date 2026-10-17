//! Deterministic offline review scheduling. This module grants no content trust,
//! mastery or XP and does not yet write to the learner database.
use crate::content::Skill;
use chrono::{DateTime, Duration, Utc};
use fsrs::{MemoryState, FSRS};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

pub const SCHEDULER_VERSION: &str = "fsrs6-fsrs-rs-6.6.2-default-v1";
pub const RATING_MAPPING_VERSION: u32 = 1;
pub const DESIRED_RETENTION: f32 = 0.9;
pub const MAX_HISTORY_RECORDS: usize = 10_000;
pub const MAX_HISTORY_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_CANDIDATES: usize = 10_000;
type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ReviewTarget {
    VocabularySense,
    Grammar,
    Objective,
}

/// Identity survives pack versions. Different curricula, target kinds and tested
/// skills never share a memory state. The host must resolve current reviewed items.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewKey {
    pub curriculum_id: String,
    pub target_kind: ReviewTarget,
    pub target_id: String,
    pub skill: Skill,
}

impl ReviewKey {
    fn validate(&self) -> Result<()> {
        for id in [&self.curriculum_id, &self.target_id] {
            if id.is_empty()
                || id.len() > 128
                || !id.as_bytes()[0].is_ascii_alphanumeric()
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
            {
                return Err("Review keys require stable content identifiers.".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Rating {
    Again,
    Hard,
    Good,
    Easy,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewEvidence {
    pub first_correct: bool,
    pub final_correct: bool,
    pub hinted: bool,
    pub revealed: bool,
}

impl ReviewEvidence {
    /// Easy requires a separate explicit action after an independent success.
    /// Correction never overwrites the first attempt. Speed is not an input.
    pub fn rating(self, explicit_easy: bool) -> Result<Rating> {
        if self.first_correct && !self.final_correct {
            return Err("Review evidence contradicts the first correct answer.".into());
        }
        let rating = if self.revealed || !self.final_correct {
            Rating::Again
        } else if self.hinted || !self.first_correct {
            Rating::Hard
        } else {
            Rating::Good
        };
        if explicit_easy {
            if rating != Rating::Good {
                return Err(
                    "Easy requires an independent correct answer without hints or reveal.".into(),
                );
            }
            Ok(Rating::Easy)
        } else {
            Ok(rating)
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewObservation {
    pub request_id: Uuid,
    pub session_id: Uuid,
    pub reviewed_at: DateTime<Utc>,
    pub evidence: ReviewEvidence,
    pub explicit_easy: bool,
}

impl ReviewObservation {
    fn validate(&self) -> Result<Rating> {
        if self.request_id.is_nil() || self.session_id.is_nil() {
            return Err("Review request and session identifiers must not be nil.".into());
        }
        self.evidence.rating(self.explicit_easy)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSchedule {
    pub key: ReviewKey,
    pub scheduler_version: &'static str,
    pub rating_mapping_version: u32,
    pub desired_retention: f32,
    pub stability: f32,
    pub difficulty: f32,
    pub last_reviewed_at: DateTime<Utc>,
    pub due_at: DateTime<Utc>,
    pub interval_days: u32,
    pub repetitions: u32,
    pub last_rating: Rating,
}

/// Store observations, not caller-supplied derived memory state. All deserialized
/// histories are checked/replayed before use. A format/engine upgrade needs an
/// explicit migration; silently applying new defaults would change due dates.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewHistory {
    format_version: u32,
    scheduler_version: String,
    rating_mapping_version: u32,
    key: ReviewKey,
    observations: Vec<ReviewObservation>,
}

impl ReviewHistory {
    pub fn new(key: ReviewKey) -> Result<Self> {
        key.validate()?;
        Ok(Self {
            format_version: 1,
            scheduler_version: SCHEDULER_VERSION.into(),
            rating_mapping_version: RATING_MAPPING_VERSION,
            key,
            observations: vec![],
        })
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_HISTORY_BYTES {
            return Err("Review history exceeds the input size limit.".into());
        }
        let history: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        history.schedule()?;
        Ok(history)
    }

    pub fn observations(&self) -> &[ReviewObservation] {
        &self.observations
    }

    pub fn key(&self) -> &ReviewKey {
        &self.key
    }

    pub fn schedule(&self) -> Result<Option<ReviewSchedule>> {
        self.key.validate()?;
        if self.format_version != 1
            || self.scheduler_version != SCHEDULER_VERSION
            || self.rating_mapping_version != RATING_MAPPING_VERSION
        {
            return Err("Unsupported review history version; migration is required.".into());
        }
        if self.observations.len() > MAX_HISTORY_RECORDS {
            return Err("Review history exceeds the record limit.".into());
        }
        let mut requests = HashSet::new();
        let mut sessions = HashSet::new();
        let mut schedule = None;
        let engine = FSRS::default();
        for observation in &self.observations {
            if !requests.insert(observation.request_id) || !sessions.insert(observation.session_id)
            {
                return Err("A review item may be rated only once in each session.".into());
            }
            schedule = Some(next_schedule(
                &engine,
                &self.key,
                schedule.as_ref(),
                observation,
            )?);
        }
        Ok(schedule)
    }

    /// Exact retries return the current schedule without adding another rating.
    /// A reused request with changed evidence or a second session rating fails.
    /// Mutation happens only after every check and FSRS calculation succeeds.
    pub fn record(&mut self, observation: ReviewObservation) -> Result<ReviewSchedule> {
        let previous = self.schedule()?;
        observation.validate()?;
        if let Some(existing) = self
            .observations
            .iter()
            .find(|r| r.request_id == observation.request_id)
        {
            if existing == &observation {
                return previous.ok_or_else(|| "Review retry has no schedule.".into());
            }
            return Err(
                "Review request identifier was already used with different evidence.".into(),
            );
        }
        if self
            .observations
            .iter()
            .any(|r| r.session_id == observation.session_id)
        {
            return Err("A review item may be rated only once in each session.".into());
        }
        if self.observations.len() == MAX_HISTORY_RECORDS {
            return Err("Review history is full; no rating was recorded.".into());
        }
        let schedule = next_schedule(&FSRS::default(), &self.key, previous.as_ref(), &observation)?;
        self.observations.push(observation);
        Ok(schedule)
    }

    pub fn candidate(&self) -> Result<ReviewCandidate> {
        Ok(ReviewCandidate {
            key: self.key.clone(),
            due_at: self.schedule()?.map(|s| s.due_at),
        })
    }
}

fn next_schedule(
    engine: &FSRS,
    key: &ReviewKey,
    previous: Option<&ReviewSchedule>,
    observation: &ReviewObservation,
) -> Result<ReviewSchedule> {
    let rating = observation.validate()?;
    let elapsed = if let Some(previous) = previous {
        let elapsed = observation
            .reviewed_at
            .signed_duration_since(previous.last_reviewed_at);
        if elapsed < Duration::zero() {
            return Err("Review time precedes the last rating; no rating was recorded.".into());
        }
        u32::try_from(elapsed.num_days()).map_err(|_| "Review interval is too large.")?
    } else {
        0
    };
    let memory = previous.map(|s| MemoryState {
        stability: s.stability,
        difficulty: s.difficulty,
    });
    let states = engine
        .next_states(memory, DESIRED_RETENTION, elapsed)
        .map_err(|e| e.to_string())?;
    let next = match rating {
        Rating::Again => states.again,
        Rating::Hard => states.hard,
        Rating::Good => states.good,
        Rating::Easy => states.easy,
    };
    // Use upstream's day-granular example: round to the nearest day, minimum 1.
    // No randomized fuzz, minute-level relearning steps or parameter training.
    let rounded = next.interval.round().max(1.0);
    if !next.interval.is_finite()
        || !next.memory.stability.is_finite()
        || next.memory.stability <= 0.0
        || !next.memory.difficulty.is_finite()
        || !(1.0..=10.0).contains(&next.memory.difficulty)
        || rounded as f64 > u32::MAX as f64
    {
        return Err("FSRS returned an invalid schedule.".into());
    }
    let interval_days = rounded as u32;
    let due_at = observation
        .reviewed_at
        .checked_add_signed(Duration::days(interval_days as i64))
        .ok_or("Review due date exceeds the supported calendar.")?;
    Ok(ReviewSchedule {
        key: key.clone(),
        scheduler_version: SCHEDULER_VERSION,
        rating_mapping_version: RATING_MAPPING_VERSION,
        desired_retention: DESIRED_RETENTION,
        stability: next.memory.stability,
        difficulty: next.memory.difficulty,
        last_reviewed_at: observation.reviewed_at,
        due_at,
        interval_days,
        repetitions: previous.map_or(1, |s| s.repetitions + 1),
        last_rating: rating,
    })
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewCandidate {
    pub key: ReviewKey,
    pub due_at: Option<DateTime<Utc>>,
}

/// The host supplies only currently available, verified, reviewed candidates.
/// Due items come first, oldest due date first. Explicit practice can also select
/// new and not-yet-due items. Stable identity breaks ties, independent of input order.
pub fn select_reviews(
    candidates: &[ReviewCandidate],
    now: DateTime<Utc>,
    limit: usize,
    include_practice: bool,
) -> Result<Vec<ReviewCandidate>> {
    if candidates.len() > MAX_CANDIDATES || limit == 0 || limit > MAX_CANDIDATES {
        return Err("Review selection exceeds the supported limits.".into());
    }
    let mut keys = HashSet::new();
    for candidate in candidates {
        candidate.key.validate()?;
        if !keys.insert(&candidate.key) {
            return Err("Review selection contains a duplicate item.".into());
        }
    }
    let mut selected: Vec<_> = candidates
        .iter()
        .filter(|c| include_practice || c.due_at.is_some_and(|due| due <= now))
        .cloned()
        .collect();
    selected.sort_by_key(|c| {
        let group = match c.due_at {
            Some(due) if due <= now => 0,
            None => 1,
            Some(_) => 2,
        };
        (group, c.due_at, c.key.clone())
    });
    selected.truncate(limit);
    Ok(selected)
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;
