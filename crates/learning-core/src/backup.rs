use crate::pack_library::{ContentContext, PackReference};
use crate::{
    store::{validate_lesson, validate_settings},
    *,
};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const MAX_BACKUP_BYTES: usize = 64 * 1024 * 1024;
const MAX_RECORDS: usize = 100_000;
type Result<T> = std::result::Result<T, String>;
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn encode<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(error)
}
fn decode<T: serde::de::DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(value).map_err(error)
}
pub(crate) fn hash<T: Serialize>(value: &T) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).map_err(error)?)
    ))
}
fn uuid(id: &str) -> Result<()> {
    if Uuid::parse_str(id).is_ok_and(|value| value.to_string() == id) {
        Ok(())
    } else {
        Err("An archive record has an invalid identifier.".into())
    }
}
fn date(value: &str) -> Result<()> {
    if NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok_and(|date| date.to_string() == value) {
        Ok(())
    } else {
        Err("An archive study date is invalid.".into())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Archive {
    body: ArchiveBody,
    sha256: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ArchiveBody {
    format: String,
    version: u32,
    schema_version: u32,
    created_at: DateTime<Utc>,
    development: bool,
    data: LearnerData,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LearnerData {
    settings: Option<Settings>,
    sessions: Vec<SavedSession>,
    items: Vec<SavedItem>,
    attempts: Vec<SavedAttempt>,
    xp: Vec<SavedXp>,
    days: Vec<SavedDay>,
    achievements: Vec<Grant>,
    reports: Vec<Report>,
    installed_pack_references: Vec<PackReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    review_records: Option<Vec<crate::review_storage::StoredReview>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    review_sessions: Option<Vec<crate::review_session::SavedReviewSession>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    review_xp: Option<Vec<crate::review_session::ReviewCredit>>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedSession {
    id: String,
    lesson: Lesson,
    position: u32,
    status: String,
    practice_only: bool,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    xp_earned: u32,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedItem {
    session_id: String,
    position: u32,
    draft: Option<Answer>,
    first_correct: Option<bool>,
    first_independent: Option<bool>,
    hinted: bool,
    revealed: bool,
    resolved: bool,
    feedback: Option<Feedback>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedAttempt {
    sequence: i64,
    request_id: String,
    session_id: String,
    position: u32,
    correct: bool,
    hinted: bool,
    revealed: bool,
    created_at: DateTime<Utc>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedXp {
    id: String,
    session_id: String,
    lesson_id: String,
    study_date: String,
    amount: u32,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedDay {
    date: String,
    goal: u32,
    time_zone: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Grant {
    id: String,
    granted_at: DateTime<Utc>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Report {
    id: String,
    session_id: String,
    exercise_id: String,
    note: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPreview {
    pub created_at: DateTime<Utc>,
    pub level: Option<String>,
    pub total_xp: u32,
    pub completed_lessons: u32,
    pub session_count: usize,
    pub active_lesson: Option<String>,
    pub development: bool,
    pub fingerprint: String,
    pub pack_references: Vec<PackReference>,
    pub review_item_count: usize,
    pub review_session_count: usize,
    pub active_review: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub state: AppState,
    pub recovery_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupTicket {
    pub token: String,
    pub preview: BackupPreview,
}

#[derive(Default)]
pub struct BackupController {
    pending: Option<PreparedImport>,
}
struct PreparedImport {
    token: String,
    bytes: Vec<u8>,
    expected_current: String,
}

impl BackupController {
    pub fn prepare(
        &mut self,
        store: &Store,
        bytes: Vec<u8>,
        now: DateTime<Utc>,
    ) -> Result<BackupTicket> {
        self.pending = None;
        let preview = store.inspect_backup(&bytes, now)?;
        let expected_current = store.backup_fingerprint()?;
        let token = Uuid::new_v4().to_string();
        self.pending = Some(PreparedImport {
            token: token.clone(),
            bytes,
            expected_current,
        });
        Ok(BackupTicket { token, preview })
    }
    pub fn cancel(&mut self) {
        self.pending = None;
    }
    pub fn restore(
        &mut self,
        store: &mut Store,
        token: &str,
        recovery_directory: &Path,
        now: DateTime<Utc>,
    ) -> Result<RestoreResult> {
        if self.pending.as_ref().map(|p| p.token.as_str()) != Some(token) {
            return Err("This import preview is no longer active. Choose the backup again.".into());
        }
        let pending = self.pending.take().ok_or("No import preview is active.")?;
        store.restore_backup(
            &pending.bytes,
            &pending.expected_current,
            recovery_directory,
            now,
        )
    }
}

impl Store {
    /// Upgrade only after the host has supplied current installed-content trust.
    /// Keep a write reservation from the snapshot through the atomic schema commit.
    pub fn migrate_schema(
        &mut self,
        directory: &Path,
        now: DateTime<Utc>,
    ) -> Result<Option<PathBuf>> {
        self.migrate_schema_with(directory, now, |_| Ok(()))
    }

    fn migrate_schema_with(
        &mut self,
        directory: &Path,
        now: DateTime<Utc>,
        after_schema: impl FnOnce(&Connection) -> Result<()>,
    ) -> Result<Option<PathBuf>> {
        let version = self.schema_version()?;
        match version {
            3 => return Ok(None),
            1 | 2 => {}
            _ => {
                return Err(
                    "Unsupported learner schema; migration did not change your data.".into(),
                )
            }
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;
        if crate::review_storage::schema_version(&tx)? != version {
            return Err("Learner schema changed before migration. Reopen the profile.".into());
        }
        let data = read_data(&tx, &self.content.references)?;
        let bytes = archive_bytes(data, self.development_build, now, version)?;
        let archive = parse_archive(&bytes, self.development_build)?;
        validate_data(
            &archive.body.data,
            self.development_build,
            &self.lessons,
            &self.content,
            now,
        )?;
        fs::create_dir_all(directory).map_err(error)?;
        let path = directory.join(format!(
            "pre-migrate-v{version}-to-v3-{}-{}.langbackup",
            now.format("%Y%m%dT%H%M%SZ"),
            Uuid::new_v4()
        ));
        write_new_backup(&path, &bytes)?;
        let saved = read_backup_file(&path)?;
        let checked = parse_archive(&saved, self.development_build)?;
        if checked.sha256 != archive.sha256 {
            return Err(
                "Pre-migration recovery verification failed; no schema was changed.".into(),
            );
        }
        validate_data(
            &checked.body.data,
            self.development_build,
            &self.lessons,
            &self.content,
            now,
        )?;
        // Both table creation and user_version roll back together on failure/crash.
        if version == 1 {
            tx.execute_batch(include_str!("review_schema.sql"))
                .map_err(error)?;
        }
        tx.execute_batch(include_str!("review_session_schema.sql"))
            .map_err(error)?;
        after_schema(&tx)?;
        tx.commit().map_err(error)?;
        Ok(Some(path))
    }

    pub fn export_backup(&self, now: DateTime<Utc>) -> Result<Vec<u8>> {
        let tx = self.connection.unchecked_transaction().map_err(error)?;
        let data = read_data(&tx, &self.content.references)?;
        let bytes = archive_bytes(data, self.development_build, now, self.schema_version()?)?;
        // Validate what we are about to call a backup, not just its serialization.
        self.inspect_backup(&bytes, now)?;
        tx.commit().map_err(error)?;
        Ok(bytes)
    }

    pub fn backup_fingerprint(&self) -> Result<String> {
        let tx = self.connection.unchecked_transaction().map_err(error)?;
        let fingerprint = hash(&read_data(&tx, &self.content.references)?)?;
        tx.commit().map_err(error)?;
        Ok(fingerprint)
    }

    pub fn inspect_backup(&self, bytes: &[u8], now: DateTime<Utc>) -> Result<BackupPreview> {
        let archive = parse_archive(bytes, self.development_build)?;
        let candidate = validate_data(
            &archive.body.data,
            self.development_build,
            &self.lessons,
            &self.content,
            now,
        )?;
        let state = candidate.state(now)?;
        Ok(BackupPreview {
            created_at: archive.body.created_at,
            level: state.settings.map(|s| s.level),
            total_xp: state.progress.total_xp,
            completed_lessons: state.progress.completed_lessons,
            session_count: archive.body.data.sessions.len(),
            active_lesson: state.active_session.map(|s| s.lesson.title),
            development: archive.body.development,
            fingerprint: archive.sha256,
            pack_references: archive.body.data.installed_pack_references,
            review_session_count: archive
                .body
                .data
                .review_sessions
                .as_ref()
                .map_or(0, Vec::len),
            active_review: archive
                .body
                .data
                .review_sessions
                .as_ref()
                .is_some_and(|ss| ss.iter().any(|s| s.status == "active")),
            review_item_count: archive
                .body
                .data
                .review_records
                .as_ref()
                .map_or(0, Vec::len),
        })
    }

    pub fn restore_backup(
        &mut self,
        bytes: &[u8],
        expected_current: &str,
        recovery_directory: &Path,
        now: DateTime<Utc>,
    ) -> Result<RestoreResult> {
        if self.schema_version()? != 3 {
            return Err("Upgrade the learner database before restoring a backup.".into());
        }
        // This candidate never touches the live database, even for a valid checksum.
        let archive = parse_archive(bytes, self.development_build)?;
        let candidate = validate_data(
            &archive.body.data,
            self.development_build,
            &self.lessons,
            &self.content,
            now,
        )?;
        let state = candidate.state(now)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;
        let current = read_data(&tx, &self.content.references)?;
        if hash(&current)? != expected_current {
            return Err(
                "Study data changed after the preview. Choose the backup again before restoring."
                    .into(),
            );
        }
        // Hold the write lock so the verified recovery snapshot is the exact state
        // being replaced, including changes from another app process.
        let recovery = archive_bytes(current, self.development_build, now, 3)?;
        let checked = parse_archive(&recovery, self.development_build)?;
        validate_data(
            &checked.body.data,
            self.development_build,
            &self.lessons,
            &self.content,
            now,
        )?;
        fs::create_dir_all(recovery_directory).map_err(error)?;
        let recovery_path = recovery_directory.join(format!(
            "pre-import-{}-{}.langbackup",
            now.format("%Y%m%dT%H%M%SZ"),
            Uuid::new_v4()
        ));
        write_new_backup(&recovery_path, &recovery)?;
        let saved = read_backup_file(&recovery_path)?;
        let verified = parse_archive(&saved, self.development_build)?;
        if verified.sha256 != checked.sha256 {
            return Err(
                "The recovery backup failed verification. Existing study data was not replaced."
                    .into(),
            );
        }
        // Only hard-coded, parameterized statements ever reach the live DB.
        tx.execute_batch("DELETE FROM review_xp; DELETE FROM review_sessions; DELETE FROM review_histories; DELETE FROM answer_reports; DELETE FROM attempts; DELETE FROM session_items; DELETE FROM xp_ledger; DELETE FROM sessions; DELETE FROM achievements; DELETE FROM study_days; DELETE FROM profile;").map_err(error)?;
        insert_data(&tx, &archive.body.data)?;
        tx.commit().map_err(error)?;
        Ok(RestoreResult {
            state: AppState {
                content_issues: self.content_issues.clone(),
                ..state
            },
            recovery_path,
        })
    }
}

pub fn read_backup_file(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(error)?
        .take(MAX_BACKUP_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(error)?;
    if bytes.len() > MAX_BACKUP_BYTES {
        return Err("The backup exceeds this version's 64 MiB limit. No data was changed.".into());
    }
    Ok(bytes)
}

pub fn write_new_backup(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or("Choose a backup filename in an existing folder.")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(error)?;
    temporary.write_all(bytes).map_err(error)?;
    temporary.as_file().sync_all().map_err(error)?;
    // persist_noclobber is atomic and refuses to replace an existing file.
    temporary.persist_noclobber(path).map_err(|_|"The backup could not be saved. Choose a new filename; existing files are never overwritten.".to_string())?;
    #[cfg(unix)]
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(error)?;
    Ok(())
}

fn archive_bytes(
    data: LearnerData,
    development: bool,
    now: DateTime<Utc>,
    version: u32,
) -> Result<Vec<u8>> {
    let body = ArchiveBody {
        format: "lang-learner-backup".into(),
        version,
        schema_version: version,
        created_at: now,
        development,
        data,
    };
    let sha256 = hash(&body)?;
    let bytes = serde_json::to_vec(&Archive { body, sha256 }).map_err(error)?;
    if bytes.len() > MAX_BACKUP_BYTES {
        return Err(
            "The backup exceeds this version's 64 MiB limit. No history was truncated.".into(),
        );
    }
    Ok(bytes)
}
fn parse_archive(bytes: &[u8], development: bool) -> Result<Archive> {
    if bytes.len() > MAX_BACKUP_BYTES {
        return Err("The backup exceeds this version's 64 MiB limit.".into());
    }
    let archive: Archive = serde_json::from_slice(bytes)
        .map_err(|_| "This file is not a supported learner backup.".to_string())?;
    if archive.body.format != "lang-learner-backup"
        || !matches!(
            (archive.body.version, archive.body.schema_version),
            (1, 1) | (2, 2) | (3, 3)
        )
        || (archive.body.version == 1 && archive.body.data.review_records.is_some())
        || (archive.body.version >= 2 && archive.body.data.review_records.is_none())
        || (archive.body.version == 3) != archive.body.data.review_sessions.is_some()
        || (archive.body.version == 3) != archive.body.data.review_xp.is_some()
    {
        return Err(
            "This backup's format or schema version is not supported. No data was changed.".into(),
        );
    }
    if archive.body.development != development {
        return Err("Development and learner backups cannot be mixed.".into());
    }
    if archive.sha256 != hash(&archive.body)? {
        return Err("Backup checksum verification failed. No data was changed.".into());
    }
    Ok(archive)
}

fn read_data(connection: &Connection, references: &[PackReference]) -> Result<LearnerData> {
    let version = crate::review_storage::schema_version(connection)?;
    let (review_rows, review_bytes): (i64, i64) = if version >= 2 {
        connection.query_row("SELECT COUNT(*), COALESCE(SUM(length(CAST(record AS BLOB))),0) FROM review_histories", [], |r| Ok((r.get(0)?,r.get(1)?))).map_err(error)?
    } else {
        (0, 0)
    };
    let count: i64 = connection.query_row("SELECT (SELECT COUNT(*) FROM sessions)+(SELECT COUNT(*) FROM session_items)+(SELECT COUNT(*) FROM attempts)+(SELECT COUNT(*) FROM xp_ledger)+(SELECT COUNT(*) FROM study_days)+(SELECT COUNT(*) FROM achievements)+(SELECT COUNT(*) FROM answer_reports)",[],|r|r.get(0)).map_err(error)?;
    let text_bytes: i64=connection.query_row("SELECT COALESCE((SELECT SUM(length(CAST(lesson_snapshot AS BLOB))) FROM sessions),0)+COALESCE((SELECT SUM(COALESCE(length(CAST(draft AS BLOB)),0)+COALESCE(length(CAST(feedback AS BLOB)),0)) FROM session_items),0)+COALESCE((SELECT SUM(length(CAST(note AS BLOB))) FROM answer_reports),0)",[],|r|r.get(0)).map_err(error)?;
    if count + review_rows > MAX_RECORDS as i64
        || text_bytes + review_bytes > MAX_BACKUP_BYTES as i64
    {
        return Err(
            "The learner history exceeds this version's backup limits. No history was truncated."
                .into(),
        );
    }
    let review_records = if version >= 2 {
        Some(crate::review_storage::read_all(connection)?)
    } else {
        None
    };
    let review_sessions = if version >= 3 {
        Some(crate::review_session::read_all(connection)?)
    } else {
        None
    };
    let review_xp = if version >= 3 {
        Some(crate::review_session::credits(connection)?)
    } else {
        None
    };
    let settings = connection
        .query_row("SELECT settings FROM profile WHERE id=1", [], |r| {
            r.get::<_, String>(0)
        })
        .optional()
        .map_err(error)?
        .map(|v| decode(&v))
        .transpose()?;
    let mut statement=connection.prepare("SELECT id,lesson_snapshot,position,status,practice_only,started_at,completed_at,xp_earned FROM sessions ORDER BY id").map_err(error)?;
    let sessions = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, u32>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, bool>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, u32>(7)?,
            ))
        })
        .map_err(error)?
        .map(|row| {
            let (id, snapshot, position, status, practice_only, started, completed, xp_earned) =
                row.map_err(error)?;
            Ok(SavedSession {
                id,
                lesson: decode(&snapshot)?,
                position,
                status,
                practice_only,
                started_at: started.parse().map_err(error)?,
                completed_at: completed.map(|v| v.parse().map_err(error)).transpose()?,
                xp_earned,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut statement=connection.prepare("SELECT session_id,position,draft,first_correct,first_independent,hinted,revealed,resolved,feedback FROM session_items ORDER BY session_id,position").map_err(error)?;
    let items = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, u32>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<bool>>(3)?,
                r.get::<_, Option<bool>>(4)?,
                r.get::<_, bool>(5)?,
                r.get::<_, bool>(6)?,
                r.get::<_, bool>(7)?,
                r.get::<_, Option<String>>(8)?,
            ))
        })
        .map_err(error)?
        .map(|row| {
            let (
                session_id,
                position,
                draft,
                first_correct,
                first_independent,
                hinted,
                revealed,
                resolved,
                feedback,
            ) = row.map_err(error)?;
            Ok(SavedItem {
                session_id,
                position,
                draft: draft.map(|v| decode(&v)).transpose()?,
                first_correct,
                first_independent,
                hinted,
                revealed,
                resolved,
                feedback: feedback.map(|v| decode(&v)).transpose()?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut statement=connection.prepare("SELECT rowid,request_id,session_id,position,correct,hinted,revealed,created_at FROM attempts ORDER BY rowid").map_err(error)?;
    let attempts = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, u32>(3)?,
                r.get::<_, bool>(4)?,
                r.get::<_, bool>(5)?,
                r.get::<_, bool>(6)?,
                r.get::<_, String>(7)?,
            ))
        })
        .map_err(error)?
        .map(|row| {
            let (sequence, request_id, session_id, position, correct, hinted, revealed, created) =
                row.map_err(error)?;
            Ok(SavedAttempt {
                sequence,
                request_id,
                session_id,
                position,
                correct,
                hinted,
                revealed,
                created_at: created.parse().map_err(error)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut statement = connection
        .prepare("SELECT id,session_id,lesson_id,study_date,amount FROM xp_ledger ORDER BY id")
        .map_err(error)?;
    let xp = statement
        .query_map([], |r| {
            Ok(SavedXp {
                id: r.get(0)?,
                session_id: r.get(1)?,
                lesson_id: r.get(2)?,
                study_date: r.get(3)?,
                amount: r.get(4)?,
            })
        })
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error)?;
    let mut statement = connection
        .prepare("SELECT date,goal,time_zone FROM study_days ORDER BY date")
        .map_err(error)?;
    let days = statement
        .query_map([], |r| {
            Ok(SavedDay {
                date: r.get(0)?,
                goal: r.get(1)?,
                time_zone: r.get(2)?,
            })
        })
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error)?;
    let mut statement = connection
        .prepare("SELECT id,granted_at FROM achievements ORDER BY id")
        .map_err(error)?;
    let achievements = statement
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(error)?
        .map(|row| {
            let (id, created) = row.map_err(error)?;
            Ok(Grant {
                id,
                granted_at: created.parse().map_err(error)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut statement = connection
        .prepare("SELECT id,session_id,exercise_id,note,created_at FROM answer_reports ORDER BY id")
        .map_err(error)?;
    let reports = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .map_err(error)?
        .map(|row| {
            let (id, session_id, exercise_id, note, created) = row.map_err(error)?;
            Ok(Report {
                id,
                session_id,
                exercise_id,
                note,
                created_at: created.parse().map_err(error)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(LearnerData {
        settings,
        sessions,
        items,
        attempts,
        xp,
        days,
        achievements,
        reports,
        installed_pack_references: references.to_vec(),
        review_records,
        review_sessions,
        review_xp,
    })
}

fn insert_data(connection: &Connection, data: &LearnerData) -> Result<()> {
    if let Some(settings) = &data.settings {
        connection
            .execute(
                "INSERT INTO profile(id,settings) VALUES(1,?1)",
                [encode(settings)?],
            )
            .map_err(error)?;
    }
    for s in &data.sessions {
        connection.execute("INSERT INTO sessions(id,lesson_id,lesson_version,lesson_snapshot,position,status,practice_only,started_at,completed_at,xp_earned) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![s.id,s.lesson.id,s.lesson.version,encode(&s.lesson)?,s.position,s.status,s.practice_only,s.started_at.to_rfc3339(),s.completed_at.map(|v|v.to_rfc3339()),s.xp_earned]).map_err(error)?;
    }
    for i in &data.items {
        connection.execute("INSERT INTO session_items(session_id,position,draft,first_correct,first_independent,hinted,revealed,resolved,feedback) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![i.session_id,i.position,i.draft.as_ref().map(encode).transpose()?,i.first_correct,i.first_independent,i.hinted,i.revealed,i.resolved,i.feedback.as_ref().map(encode).transpose()?]).map_err(error)?;
    }
    for a in &data.attempts {
        connection.execute("INSERT INTO attempts(rowid,request_id,session_id,position,correct,hinted,revealed,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![a.sequence,a.request_id,a.session_id,a.position,a.correct,a.hinted,a.revealed,a.created_at.to_rfc3339()]).map_err(error)?;
    }
    for x in &data.xp {
        connection.execute("INSERT INTO xp_ledger(id,session_id,lesson_id,study_date,amount) VALUES(?1,?2,?3,?4,?5)",params![x.id,x.session_id,x.lesson_id,x.study_date,x.amount]).map_err(error)?;
    }
    for d in &data.days {
        connection
            .execute(
                "INSERT INTO study_days(date,time_zone,goal) VALUES(?1,?2,?3)",
                params![d.date, d.time_zone, d.goal],
            )
            .map_err(error)?;
    }
    for g in &data.achievements {
        connection
            .execute(
                "INSERT INTO achievements(id,granted_at) VALUES(?1,?2)",
                params![g.id, g.granted_at.to_rfc3339()],
            )
            .map_err(error)?;
    }
    for r in &data.reports {
        connection.execute("INSERT INTO answer_reports(id,session_id,exercise_id,note,created_at) VALUES(?1,?2,?3,?4,?5)",params![r.id,r.session_id,r.exercise_id,r.note,r.created_at.to_rfc3339()]).map_err(error)?;
    }
    for record in data.review_records.as_deref().unwrap_or_default() {
        crate::review_storage::insert_record(connection, record)?;
    }
    for session in data.review_sessions.as_deref().unwrap_or_default() {
        crate::review_session::save(connection, session)?;
    }
    for credit in data.review_xp.as_deref().unwrap_or_default() {
        connection
            .execute(
                "INSERT INTO review_xp(study_date,session_id,amount) VALUES(?1,?2,?3)",
                params![
                    credit.study_date,
                    credit.session_id.to_string(),
                    credit.amount
                ],
            )
            .map_err(error)?;
    }
    Ok(())
}

fn validate_data(
    data: &LearnerData,
    development: bool,
    lessons: &[Lesson],
    context: &ContentContext,
    now: DateTime<Utc>,
) -> Result<Store> {
    let count = data.sessions.len()
        + data.items.len()
        + data.attempts.len()
        + data.xp.len()
        + data.days.len()
        + data.achievements.len()
        + data.reports.len()
        + data
            .review_records
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|r| 1 + 2 * r.history().observations().len())
            .sum::<usize>();
    let count = count
        + data
            .review_sessions
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|s| 1 + s.items.iter().map(|i| 1 + i.attempts.len()).sum::<usize>())
            .sum::<usize>()
        + data.review_xp.as_ref().map_or(0, Vec::len);
    if count > MAX_RECORDS {
        return Err("The backup exceeds this version's 100,000 record limit.".into());
    }
    if data.installed_pack_references.len() > 1000 {
        return Err("Too many pack references in backup.".into());
    }
    let mut references = HashSet::new();
    for reference in &data.installed_pack_references {
        if !references.insert((&reference.pack_id, reference.version))
            || !context.references.contains(reference)
        {
            return Err("A matching verified pack version is missing. Install the exact referenced content before restoring this backup.".into());
        }
    }
    if let Some(settings) = &data.settings {
        validate_settings(settings)?;
    } else if count > 0 {
        return Err("Learner records require profile settings.".into());
    }
    let mut approved_snapshots = lessons
        .iter()
        .filter(|lesson| lesson.editorial_status == "approved")
        .map(hash)
        .collect::<Result<HashSet<_>>>()?;
    approved_snapshots.extend(context.reviewed_snapshots.iter().cloned());
    let mut sessions = HashMap::new();
    let mut session_hashes = HashMap::new();
    for s in &data.sessions {
        uuid(&s.id)?;
        validate_lesson(&s.lesson)?;
        let snapshot_hash = hash(&s.lesson)?;
        if !s.practice_only && !approved_snapshots.contains(&snapshot_hash) {
            return Err("A reviewed lesson snapshot is not available in the installed catalogue. Install the matching reviewed content before restoring this backup.".into());
        }
        session_hashes.insert(s.id.as_str(), snapshot_hash);
        if s.lesson.exercises.len() > 1000
            || s.lesson.id.len() > 256
            || sessions.insert(s.id.as_str(), s).is_some()
            || s.position as usize >= s.lesson.exercises.len()
            || s.practice_only != (s.lesson.editorial_status != "approved")
            || (!development && s.practice_only)
            || !["active", "completed"].contains(&s.status.as_str())
            || (s.status == "completed") != s.completed_at.is_some()
            || (s.status == "completed" && s.position as usize != s.lesson.exercises.len() - 1)
            || (s.status == "active" && s.xp_earned != 0)
        {
            return Err("Invalid session or lesson snapshot in backup.".into());
        }
        for e in &s.lesson.exercises {
            if e.id.len() > 256
                || e.options.len() > 1000
                || e.accepted.len() > 1000
                || e.prompt.len() > 1_000_000
                || e.context.as_ref().is_some_and(|v| v.len() > 1_000_000)
            {
                return Err("An exercise snapshot exceeds supported limits.".into());
            }
        }
    }
    let mut items = HashMap::new();
    for i in &data.items {
        let s = sessions
            .get(i.session_id.as_str())
            .ok_or("Orphan exercise evidence in backup.")?;
        let e = s
            .lesson
            .exercises
            .get(i.position as usize)
            .ok_or("Invalid exercise position in backup.")?;
        if items
            .insert((i.session_id.as_str(), i.position), i)
            .is_some()
        {
            return Err("Duplicate exercise evidence in backup.".into());
        }
        if let Some(draft) = &i.draft {
            match (&e.kind, draft) {
                (ExerciseKind::Translation | ExerciseKind::Cloze, Answer::Text(text))
                    if text.len() <= 4000 => {}
                (ExerciseKind::Ordering, Answer::Tiles(ids)) if ids.is_empty() => {}
                _ => {
                    crate::scoring::answer_text(e, draft)?;
                }
            }
        }
        if (i.position < s.position || s.status == "completed") && !i.resolved {
            return Err("Completed exercises contain unresolved evidence.".into());
        }
        if i.position > s.position
            && (i.first_correct.is_some()
                || i.first_independent.is_some()
                || i.hinted
                || i.revealed
                || i.resolved
                || i.feedback.is_some()
                || i.draft.is_some())
        {
            return Err("Unvisited exercises contain assessment evidence.".into());
        }
    }
    for s in &data.sessions {
        for p in 0..s.lesson.exercises.len() {
            if !items.contains_key(&(s.id.as_str(), p as u32)) {
                return Err("A session is missing exercise evidence.".into());
            }
        }
    }
    let mut attempts: HashMap<(&str, u32), Vec<&SavedAttempt>> = HashMap::new();
    let mut sequences = HashSet::new();
    let mut requests = HashSet::new();
    for a in &data.attempts {
        uuid(&a.request_id)?;
        let item = items
            .get(&(a.session_id.as_str(), a.position))
            .ok_or("Orphan attempt in backup.")?;
        if a.sequence <= 0
            || !sequences.insert(a.sequence)
            || !requests.insert(&a.request_id)
            || (a.hinted && !item.hinted)
            || (a.revealed && !item.revealed)
            || (a.correct && a.revealed)
        {
            return Err("Invalid attempt evidence in backup.".into());
        }
        attempts
            .entry((a.session_id.as_str(), a.position))
            .or_default()
            .push(a);
    }
    let mut last_attempts = HashMap::new();
    for ((id, position), i) in &items {
        let mut history = attempts.remove(&(*id, *position)).unwrap_or_default();
        history.sort_by_key(|a| a.sequence);
        if let (Some(first), Some(last)) = (history.first(), history.last()) {
            last_attempts.insert((*id, *position), *last);
            if history.windows(2).any(|pair| {
                pair[0].correct || pair[0].revealed || (pair[0].hinted && !pair[1].hinted)
            }) || i.revealed != last.revealed
            {
                return Err("Attempt ordering is inconsistent with recorded corrections.".into());
            }
            let e = &sessions[id].lesson.exercises[*position as usize];
            let expected = Feedback {
                correct: last.correct,
                revealed: last.revealed,
                expected: e.accepted[0].clone(),
                explanation: e.explanation.clone(),
            };
            if i.first_correct != Some(first.correct)
                || i.first_independent != Some(first.correct && !first.hinted && !first.revealed)
                || i.resolved != (last.correct || last.revealed)
                || i.feedback.as_ref().map(encode).transpose()? != Some(encode(&expected)?)
            {
                return Err("First-attempt or feedback evidence is inconsistent.".into());
            }
        } else if i.first_correct.is_some()
            || i.first_independent.is_some()
            || i.resolved
            || i.revealed
            || i.feedback.is_some()
        {
            return Err("Assessment evidence has no corresponding attempt.".into());
        }
    }
    let mut saved_reviews = HashMap::new();
    let mut all_requests: HashSet<Uuid> = data
        .attempts
        .iter()
        .map(|a| Uuid::parse_str(&a.request_id).map_err(error))
        .collect::<Result<_>>()?;
    let mut expected_observations = HashSet::new();
    let mut active_reviews = 0;
    for s in data.review_sessions.as_deref().unwrap_or_default() {
        s.validate(context, now)?;
        if sessions.contains_key(s.id.to_string().as_str())
            || saved_reviews.insert(s.id, s).is_some()
        {
            return Err("Duplicate lesson/review session identifier.".into());
        }
        if s.status == "active" {
            active_reviews += 1;
        }
        for i in &s.items {
            if !data.installed_pack_references.contains(&i.source.pack) {
                return Err("Saved review pack reference is missing.".into());
            }
            for a in &i.attempts {
                if !all_requests.insert(a.request_id) {
                    return Err("Duplicate review answer identifier.".into());
                }
            }
            if let Some(o) = &i.observation {
                expected_observations.insert((s.id, i.source.key.clone(), o.request_id));
            }
        }
    }
    if active_reviews
        + data
            .sessions
            .iter()
            .filter(|s| s.status == "active")
            .count()
        > 1
    {
        return Err("Only one lesson or review can be active.".into());
    }
    let mut days = HashSet::new();
    for d in &data.days {
        date(&d.date)?;
        d.time_zone.parse::<chrono_tz::Tz>().map_err(error)?;
        if d.goal == 0 || !days.insert(d.date.as_str()) {
            return Err("Invalid or duplicate study day in backup.".into());
        }
    }
    let mut credited = HashMap::new();
    let mut credit_days = HashSet::new();
    for x in &data.xp {
        uuid(&x.id)?;
        date(&x.study_date)?;
        let s = sessions
            .get(x.session_id.as_str())
            .ok_or("Orphan XP entry in backup.")?;
        if x.amount != 10
            || s.status != "completed"
            || s.practice_only
            || s.lesson.id != x.lesson_id
            || !days.contains(x.study_date.as_str())
            || credited.insert(x.session_id.as_str(), x.amount).is_some()
        {
            return Err("Invalid XP credit in backup.".into());
        }
        credit_days.insert(x.study_date.as_str());
    }
    let mut review_dates = HashSet::new();
    let mut review_credited = HashSet::new();
    for x in data.review_xp.as_deref().unwrap_or_default() {
        date(&x.study_date)?;
        let s = saved_reviews
            .get(&x.session_id)
            .ok_or("Orphan review credit.")?;
        if x.amount != 5
            || s.status != "completed"
            || s.study_date.as_ref() != Some(&x.study_date)
            || s.xp_earned != 5
            || !days.contains(x.study_date.as_str())
            || !review_dates.insert(x.study_date.as_str())
            || !review_credited.insert(x.session_id)
        {
            return Err("Invalid review credit in backup.".into());
        }
        credit_days.insert(x.study_date.as_str());
    }
    for s in saved_reviews.values() {
        if s.xp_earned
            != if review_credited.contains(&s.id) {
                5
            } else {
                0
            }
        {
            return Err("Review XP differs from its ledger.".into());
        }
        if let Some(d) = &s.study_date {
            date(d)?;
            if !review_dates.contains(d.as_str()) {
                return Err("Completed review date has no qualifying daily credit.".into());
            }
            credit_days.insert(d.as_str());
        }
    }
    if days != credit_days {
        return Err("Study days and qualifying credit do not agree.".into());
    }
    for s in &data.sessions {
        if s.xp_earned != *credited.get(s.id.as_str()).unwrap_or(&0) {
            return Err("Session XP does not agree with its ledger.".into());
        }
    }
    for r in &data.reports {
        uuid(&r.id)?;
        if r.note.trim().is_empty()
            || r.note.len() > 4000
            || !sessions
                .get(r.session_id.as_str())
                .is_some_and(|s| s.lesson.exercises.iter().any(|e| e.id == r.exercise_id))
        {
            return Err("Invalid local answer report in backup.".into());
        }
    }
    let mut review_keys = HashSet::new();
    for record in data.review_records.as_deref().unwrap_or_default() {
        record.validate_structure()?;
        record.validate_authority(context)?;
        if !review_keys.insert(record.history().key()) {
            return Err("Duplicate review item history in backup.".into());
        }
        for (observation, source) in record.history().observations().iter().zip(record.sources()) {
            let id = observation.session_id.to_string();
            if let Some(session) = saved_reviews.get(&observation.session_id) {
                let item = session
                    .items
                    .iter()
                    .find(|i| {
                        i.source == *source
                            && i.observation.as_ref().is_some_and(|o| o == observation)
                    })
                    .ok_or("Review observation does not match its saved review interaction.")?;
                if !expected_observations.remove(&(
                    session.id,
                    item.source.key.clone(),
                    observation.request_id,
                )) {
                    return Err("Repeated review observation.".into());
                }
                continue;
            }
            let session = sessions
                .get(id.as_str())
                .ok_or("Review observation has no saved session.")?;
            let position = session
                .lesson
                .exercises
                .iter()
                .position(|e| e.id == source.exercise_id)
                .ok_or("Review source exercise is missing.")? as u32;
            let item = items
                .get(&(id.as_str(), position))
                .ok_or("Review source evidence is missing.")?;
            let feedback = item
                .feedback
                .as_ref()
                .ok_or("Review source feedback is missing.")?;
            let evidence = crate::review::ReviewEvidence {
                first_correct: item.first_correct.unwrap_or(false),
                final_correct: feedback.correct,
                hinted: item.hinted,
                revealed: item.revealed,
            };
            let last = last_attempts
                .get(&(id.as_str(), position))
                .ok_or("Review source attempt is missing.")?;
            if session.practice_only
                || !item.resolved
                || source.lesson_id != session.lesson.id
                || session_hashes.get(id.as_str()) != Some(&source.lesson_sha256)
                || !data.installed_pack_references.contains(&source.pack)
                || observation.evidence != evidence
                || observation.explicit_easy
                || observation.reviewed_at < session.started_at
                || observation.reviewed_at > now
                || session
                    .completed_at
                    .is_some_and(|end| observation.reviewed_at > end)
                || last.request_id != observation.request_id.to_string()
                || last.created_at > observation.reviewed_at
            {
                return Err("Review observation does not match its graded lesson evidence.".into());
            }
        }
    }
    if !expected_observations.is_empty() {
        return Err("Saved review rating is absent from its FSRS history.".into());
    }
    for g in &data.achievements {
        if !["first_lesson", "first_review", "100_xp", "7_day_streak"].contains(&g.id.as_str()) {
            return Err("This archive contains an unsupported achievement.".into());
        }
    }
    let candidate = Store::initialize(
        Connection::open_in_memory().map_err(error)?,
        lessons.to_vec(),
        development,
    )?;
    insert_data(&candidate.connection, data)?;
    let progress = candidate.progress(now)?;
    for (id, earned) in [
        ("first_lesson", progress.completed_lessons > 0),
        (
            "first_review",
            saved_reviews.values().any(|s| s.status == "completed"),
        ),
        ("100_xp", progress.total_xp >= 100),
        ("7_day_streak", progress.best_streak >= 7),
    ] {
        if data.achievements.iter().any(|g| g.id == id) != earned {
            return Err("Achievement grants do not agree with study evidence.".into());
        }
    }
    candidate.state(now)?;
    Ok(candidate)
}

#[cfg(test)]
#[path = "migration_tests.rs"]
mod migration_tests;
#[cfg(test)]
#[path = "backup_tests.rs"]
mod tests;
