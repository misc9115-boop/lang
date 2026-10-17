//! Review history persistence and provenance. No review-session XP is awarded here.
use crate::{
    backup::hash,
    pack_library::{ContentContext, PackReference},
    review::{ReviewEvidence, ReviewHistory, ReviewKey, ReviewObservation, ReviewSchedule},
    Feedback, Lesson, Store,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

pub const MAX_STORED_REVIEW_BYTES: usize = 8 * 1024 * 1024;
type Result<T> = std::result::Result<T, String>;
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn encode<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(error)
}

/// Built by the host from exact, independently reviewed pack bytes. A source is
/// not authority merely because it appears in a saved record or an archive.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewSource {
    pub pack: PackReference,
    pub lesson_id: String,
    pub lesson_sha256: String,
    pub exercise_id: String,
    pub key: ReviewKey,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoredReview {
    history: ReviewHistory,
    sources: Vec<ReviewSource>,
}
impl StoredReview {
    pub fn history(&self) -> &ReviewHistory {
        &self.history
    }
    pub fn sources(&self) -> &[ReviewSource] {
        &self.sources
    }
    pub fn schedule(&self) -> Result<ReviewSchedule> {
        self.history
            .schedule()?
            .ok_or_else(|| "Review history is empty.".into())
    }
    pub(crate) fn from_json(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_STORED_REVIEW_BYTES {
            return Err("Stored review exceeds the input limit.".into());
        }
        let value: Self = serde_json::from_slice(bytes).map_err(error)?;
        value.validate_structure()?;
        Ok(value)
    }
    pub(crate) fn validate_structure(&self) -> Result<()> {
        self.history
            .schedule()?
            .ok_or("A stored review must contain an observation.")?;
        if self.sources.len() != self.history.observations().len()
            || self.sources.iter().any(|s| &s.key != self.history.key())
        {
            return Err("Review history and exercise sources do not agree.".into());
        }
        Ok(())
    }
    pub(crate) fn validate_authority(&self, context: &ContentContext) -> Result<()> {
        if self
            .sources
            .iter()
            .any(|source| !context.review_sources.contains(source))
        {
            return Err("Matching independently reviewed exercise content is unavailable. Install or repair the referenced pack before using this review history.".into());
        }
        Ok(())
    }
}

impl Store {
    pub fn schema_version(&self) -> Result<u32> {
        schema_version(&self.connection)
    }

    /// Reading retained history does not grant permission to study revoked content.
    pub fn review_history(&self, key: &ReviewKey) -> Result<Option<StoredReview>> {
        read_one(&self.connection, key)
    }

    pub fn review_histories(&self) -> Result<Vec<StoredReview>> {
        read_all(&self.connection)
    }
}

pub(crate) fn schema_version(connection: &Connection) -> Result<u32> {
    connection
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(error)
}
pub(crate) fn read_one(connection: &Connection, key: &ReviewKey) -> Result<Option<StoredReview>> {
    if !matches!(schema_version(connection)?, 2 | 3) {
        return Err("Upgrade the learner database before using review history.".into());
    }
    let key = encode(key)?;
    let length: Option<i64> = connection
        .query_row(
            "SELECT length(CAST(record AS BLOB)) FROM review_histories WHERE review_key=?1",
            [&key],
            |r| r.get(0),
        )
        .optional()
        .map_err(error)?;
    if length.is_some_and(|n| n > MAX_STORED_REVIEW_BYTES as i64) {
        return Err("Stored review exceeds the input limit.".into());
    }
    let data: Option<String> = connection
        .query_row(
            "SELECT record FROM review_histories WHERE review_key=?1",
            [&key],
            |r| r.get(0),
        )
        .optional()
        .map_err(error)?;
    data.map(|data| {
        let value = StoredReview::from_json(data.as_bytes())?;
        if encode(value.history.key())? != key {
            return Err("Stored review key does not match its history.".into());
        }
        Ok(value)
    })
    .transpose()
}
pub(crate) fn read_all(connection: &Connection) -> Result<Vec<StoredReview>> {
    match schema_version(connection)? {
        1 => return Ok(vec![]),
        2 | 3 => {}
        _ => return Err("Unsupported learner schema; review history was not read.".into()),
    }
    let (count, total, largest, key_length): (i64, i64, i64, i64) = connection.query_row("SELECT COUNT(*), COALESCE(SUM(length(CAST(record AS BLOB))),0), COALESCE(MAX(length(CAST(record AS BLOB))),0), COALESCE(MAX(length(CAST(review_key AS BLOB))),0) FROM review_histories", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).map_err(error)?;
    if count > 100_000
        || total > crate::MAX_BACKUP_BYTES as i64
        || largest > MAX_STORED_REVIEW_BYTES as i64
        || key_length > 1024
    {
        return Err("Stored reviews exceed supported backup limits.".into());
    }
    let mut statement = connection
        .prepare("SELECT review_key,record FROM review_histories ORDER BY review_key")
        .map_err(error)?;
    let rows = statement
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(error)?;
    rows.map(|row| {
        let (key, data) = row.map_err(error)?;
        let record = StoredReview::from_json(data.as_bytes())?;
        if key != encode(record.history.key())? {
            return Err("Stored review key does not match its history.".into());
        }
        Ok(record)
    })
    .collect()
}
pub(crate) fn insert_record(connection: &Connection, record: &StoredReview) -> Result<()> {
    let data = encode(record)?;
    if data.len() > MAX_STORED_REVIEW_BYTES {
        return Err("Stored review exceeds the input limit.".into());
    }
    connection.execute("INSERT INTO review_histories(review_key,record) VALUES(?1,?2) ON CONFLICT(review_key) DO UPDATE SET record=excluded.record", params![encode(record.history.key())?,data]).map_err(error)?;
    Ok(())
}

/// Only called inside the transaction advancing a resolved, reviewed lesson.
/// Objective/skill mappings exist today; target-level mappings must be authored
/// before vocabulary/grammar-specific items can be seeded. No invented mappings.
pub(crate) fn seed_lesson_reviews(
    connection: &Connection,
    context: &ContentContext,
    lesson: &Lesson,
    session_id: &str,
    position: usize,
    now: DateTime<Utc>,
) -> Result<()> {
    let snapshot = hash(lesson)?;
    let mut sources: Vec<_> = context
        .review_sources
        .iter()
        .filter(|s| s.lesson_sha256 == snapshot && s.exercise_id == lesson.exercises[position].id)
        .collect();
    // If identical content is cached in multiple packs/versions, bind deterministically.
    sources.sort_by_key(|s| (&s.pack.pack_id, s.pack.version, &s.key));
    let mut keys = HashSet::new();
    for source in sources {
        if !keys.insert(&source.key) {
            continue;
        }
        let mut record = read_one(connection, &source.key)?.unwrap_or(StoredReview {
            history: ReviewHistory::new(source.key.clone())?,
            sources: vec![],
        });
        record.validate_authority(context)?;
        let session = Uuid::parse_str(session_id).map_err(error)?;
        // Several exercises may test one item in a lesson. Preserve its first
        // resolved observation instead of repeatedly rating it in that session.
        if record
            .history
            .observations()
            .iter()
            .any(|o| o.session_id == session)
        {
            continue;
        }
        let (request, created): (String, String) = connection.query_row("SELECT request_id,created_at FROM attempts WHERE session_id=?1 AND position=?2 ORDER BY rowid DESC LIMIT 1", params![session_id, position as u32], |r| Ok((r.get(0)?, r.get(1)?))).map_err(error)?;
        let (first, hinted, revealed, feedback): (bool, bool, bool, String) = connection.query_row("SELECT first_correct,hinted,revealed,feedback FROM session_items WHERE session_id=?1 AND position=?2 AND resolved=1", params![session_id, position as u32], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(error)?;
        let feedback: Feedback = serde_json::from_str(&feedback).map_err(error)?;
        if now < created.parse::<DateTime<Utc>>().map_err(error)? {
            return Err("Review time precedes its recorded answer.".into());
        }
        let observation = ReviewObservation {
            request_id: Uuid::parse_str(&request).map_err(error)?,
            session_id: session,
            reviewed_at: now,
            evidence: ReviewEvidence {
                first_correct: first,
                final_correct: feedback.correct,
                hinted,
                revealed,
            },
            explicit_easy: false,
        };
        record.history.record(observation)?;
        record.sources.push(source.clone());
        insert_record(connection, &record)?;
    }
    Ok(())
}

/// Commit a resolved review rating using a host-verified source.
pub(crate) fn record_review(
    connection: &Connection,
    context: &ContentContext,
    source: &ReviewSource,
    observation: ReviewObservation,
) -> Result<()> {
    if !context.review_sources.contains(source) {
        return Err("Review content is no longer verified.".into());
    }
    let mut record = read_one(connection, &source.key)?
        .ok_or("Introduce this item through a reviewed lesson first.")?;
    record.validate_authority(context)?;
    record.history.record(observation)?;
    record.sources.push(source.clone());
    insert_record(connection, &record)
}
