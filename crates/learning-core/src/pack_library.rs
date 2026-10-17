//! Offline text-pack storage. This database is separate from learner progress.
use crate::{
    content::{self, ContentTrust, EditorialStatus, ValidatedPack},
    Lesson,
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, path::Path};

type Result<T> = std::result::Result<T, String>;
const APPLICATION_ID: u32 = 0x4c504b31;
const MAX_VERSIONS: u32 = 1000;
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// A host chooses this explicitly. Development packs never become reviewed.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LibraryMode {
    ReviewedOnly,
    Development,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackEntry {
    pub pack_id: String,
    pub version: u32,
    pub manifest_sha256: String,
    /// Stored is a catalogue state, not a fresh integrity/approval assertion.
    pub stored: bool,
    pub active: bool,
    pub payload_bytes: u64,
    pub installed_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackReference {
    pub pack_id: String,
    pub version: u32,
    pub manifest_sha256: String,
}

#[derive(Clone, Default)]
pub(crate) struct ContentContext {
    pub managed: bool,
    pub references: Vec<PackReference>,
    pub reviewed_snapshots: HashSet<String>,
    pub review_sources: HashSet<crate::review_storage::ReviewSource>,
}

pub struct PackLibrary {
    connection: Connection,
    trust: ContentTrust,
    mode: LibraryMode,
}

impl PackLibrary {
    pub fn open(path: &Path, trust: ContentTrust, mode: LibraryMode) -> Result<Self> {
        let mut connection = Connection::open(path).map_err(error)?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000; PRAGMA synchronous=FULL;",
            )
            .map_err(error)?;
        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(error)?;
        let application: u32 = connection
            .query_row("PRAGMA application_id", [], |r| r.get(0))
            .map_err(error)?;
        if version == 0 && application == 0 {
            let tables: u32 = connection
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
                    [],
                    |r| r.get(0),
                )
                .map_err(error)?;
            if tables != 0 {
                return Err(
                    "The selected database is not an empty pack catalogue. It was not changed."
                        .into(),
                );
            }
            let tx = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(error)?;
            tx.execute_batch(include_str!("pack_schema.sql"))
                .map_err(error)?;
            tx.commit().map_err(error)?;
        } else if version != 1 || application != APPLICATION_ID {
            return Err("Unsupported pack catalogue. Its contents were not changed.".into());
        }
        Ok(Self {
            connection,
            trust,
            mode,
        })
    }

    /// Entries include removed versions: their identity history prevents reuse.
    /// Call verify for current integrity; active_lessons always revalidates.
    pub fn entries(&self) -> Result<Vec<PackEntry>> {
        entries(&self.connection)
    }

    /// Validate and copy only declared, verified bytes, in one SQLite transaction.
    /// Installation caches a version; activation is a separate explicit operation.
    pub fn install_directory(&mut self, root: &Path) -> Result<PackEntry> {
        let pack = content::validate_pack(root, &self.trust)?;
        self.check_mode(&pack)?;
        self.install_verified(pack)
    }

    pub fn install_previewed(&mut self, root: &Path, expected_digest: &str) -> Result<PackEntry> {
        let pack = content::validate_pack(root, &self.trust)?;
        self.check_mode(&pack)?;
        if pack.report.manifest_sha256 != expected_digest {
            return Err("The pack changed after preview. Choose its folder again.".into());
        }
        self.install_verified_inner(pack, true)
    }

    pub fn preview_directory(&self, root: &Path) -> Result<ValidatedPack> {
        let pack = content::validate_pack(root, &self.trust)?;
        self.check_mode(&pack)?;
        Ok(pack)
    }

    fn check_mode(&self, pack: &ValidatedPack) -> Result<()> {
        if self.mode == LibraryMode::ReviewedOnly
            && pack.manifest.editorial_status != EditorialStatus::Approved
        {
            return Err("Unreviewed packs can only be installed in a development library.".into());
        }
        Ok(())
    }

    fn install_verified(&mut self, pack: ValidatedPack) -> Result<PackEntry> {
        self.install_verified_inner(pack, false)
    }

    fn install_verified_inner(
        &mut self,
        pack: ValidatedPack,
        allow_historical: bool,
    ) -> Result<PackEntry> {
        let id = &pack.manifest.id;
        let version = pack.manifest.version;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;
        let previous: Option<String> = tx
            .query_row(
                "SELECT digest FROM pack_versions WHERE pack_id=?1 AND version=?2",
                params![id, version],
                |r| r.get(0),
            )
            .optional()
            .map_err(error)?;
        if let Some(previous) = &previous {
            if previous != &pack.report.manifest_sha256 {
                return Err("This pack ID/version already identifies different bytes. Increase the version.".into());
            }
        } else {
            let versions: u32 = tx
                .query_row("SELECT count(*) FROM pack_versions", [], |r| r.get(0))
                .map_err(error)?;
            if versions >= MAX_VERSIONS {
                return Err("This text library has reached its 1000-version history limit.".into());
            }
            let highest: u32 = tx
                .query_row(
                    "SELECT coalesce(max(version),0) FROM pack_versions WHERE pack_id=?1",
                    [id],
                    |r| r.get(0),
                )
                .map_err(error)?;
            let historical = allow_historical && version < highest;
            if version <= highest && !historical {
                return Err("A new pack version must exceed its recorded history. Use activate for an installed rollback.".into());
            }
            // Keep semantic identity history even if the user removes payloads.
            for lesson in &pack.lessons {
                let record = pack
                    .manifest
                    .lessons
                    .iter()
                    .find(|r| r.lesson_id == lesson.id)
                    .ok_or("Missing lesson record.")?;
                record_identity(
                    &tx,
                    id,
                    "lesson",
                    &lesson.id,
                    lesson.version,
                    &record.file.sha256,
                    historical,
                )?;
            }
            let curriculum = &pack.manifest.curriculum;
            let bytes = serde_json::to_vec(curriculum).map_err(error)?;
            record_identity(
                &tx,
                id,
                "curriculum",
                &curriculum.id,
                curriculum.version,
                &hash(&bytes),
                historical,
            )?;
            tx.execute(
                "INSERT INTO pack_versions(pack_id,version,digest,manifest,stored,payload_bytes,installed_at) VALUES(?1,?2,?3,?4,0,?5,?6)",
                params![id, version, pack.report.manifest_sha256, pack.manifest_bytes, pack.report.payload_bytes as u32, Utc::now().to_rfc3339()],
            ).map_err(error)?;
        }
        // Exact-version reinstall repairs missing/corrupt cached payloads. The
        // original digest stays immutable, including after payload removal.
        tx.execute(
            "DELETE FROM pack_files WHERE pack_id=?1 AND version=?2",
            params![id, version],
        )
        .map_err(error)?;
        for (path, bytes) in &pack.files {
            tx.execute(
                "INSERT INTO pack_files(pack_id,version,path,bytes) VALUES(?1,?2,?3,?4)",
                params![id, version, path, bytes],
            )
            .map_err(error)?;
        }
        tx.execute(
            "UPDATE pack_versions SET manifest=?3,stored=1 WHERE pack_id=?1 AND version=?2",
            params![id, version, pack.manifest_bytes],
        )
        .map_err(error)?;
        tx.commit().map_err(error)?;
        self.entries()?
            .into_iter()
            .find(|e| e.pack_id == *id && e.version == version)
            .ok_or("Installed version is missing.".into())
    }

    pub fn verify(&self, id: &str, version: u32) -> Result<ValidatedPack> {
        let pack = read_pack(&self.connection, id, version, &self.trust)?;
        self.check_mode(&pack)?;
        Ok(pack)
    }

    /// This explicit action also permits rollback to an intact retained version.
    /// The old active pointer survives every validation or commit failure.
    pub fn activate(&mut self, id: &str, version: u32) -> Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;
        let pack = read_pack(&tx, id, version, &self.trust)?;
        if self.mode == LibraryMode::ReviewedOnly
            && pack.manifest.editorial_status != EditorialStatus::Approved
        {
            return Err("Unreviewed packs cannot be activated in a reviewed-only library.".into());
        }
        let mut ids = HashSet::new();
        record_active_ids(&pack, &mut ids)?;
        drop(pack);
        for (other, current) in active_versions(&tx)? {
            if other != id {
                let other_pack = read_pack(&tx, &other, current, &self.trust)?;
                if self.mode == LibraryMode::ReviewedOnly
                    && other_pack.manifest.editorial_status != EditorialStatus::Approved
                {
                    return Err("Another active pack is unreviewed. Deactivate it first.".into());
                }
                record_active_ids(&other_pack, &mut ids)?;
            }
        }
        tx.execute(
            "INSERT INTO active_packs(pack_id,version) VALUES(?1,?2) ON CONFLICT(pack_id) DO UPDATE SET version=excluded.version",
            params![id, version],
        ).map_err(error)?;
        tx.commit().map_err(error)
    }

    pub fn deactivate(&mut self, id: &str) -> Result<()> {
        self.connection
            .execute("DELETE FROM active_packs WHERE pack_id=?1", [id])
            .map_err(error)?;
        Ok(())
    }

    /// Active payloads require explicit deactivation first. Identity metadata
    /// remains; removing content never deletes learner sessions or progress.
    pub fn remove_version(&mut self, id: &str, version: u32) -> Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(error)?;
        let active: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM active_packs WHERE pack_id=?1 AND version=?2)",
                params![id, version],
                |r| r.get(0),
            )
            .map_err(error)?;
        if active {
            return Err("Deactivate this version before removing its payloads.".into());
        }
        let changed = tx
            .execute(
                "UPDATE pack_versions SET stored=0 WHERE pack_id=?1 AND version=?2",
                params![id, version],
            )
            .map_err(error)?;
        if changed == 0 {
            return Err("This pack version is not recorded.".into());
        }
        tx.execute(
            "DELETE FROM pack_files WHERE pack_id=?1 AND version=?2",
            params![id, version],
        )
        .map_err(error)?;
        tx.commit().map_err(error)
    }

    /// No cached approval is trusted: recheck current host trust and all bytes.
    /// A corrupt active pack produces an error rather than silently dropping it.
    pub fn active_lessons(&self) -> Result<Vec<Lesson>> {
        let mut lessons = vec![];
        let mut ids = HashSet::new();
        for (id, version) in active_versions(&self.connection)? {
            let pack = self.verify(&id, version)?;
            record_active_ids(&pack, &mut ids)?;
            // Release raw snapshots after each pack rather than retaining every
            // installed payload alongside the parsed course in memory.
            lessons.extend(pack.lessons);
        }
        Ok(lessons)
    }
}

fn entries(connection: &Connection) -> Result<Vec<PackEntry>> {
    let mut statement = connection.prepare(
        "SELECT p.pack_id,p.version,p.digest,p.stored,p.payload_bytes,p.installed_at,EXISTS(SELECT 1 FROM active_packs a WHERE a.pack_id=p.pack_id AND a.version=p.version) FROM pack_versions p ORDER BY p.pack_id,p.version"
    ).map_err(error)?;
    let rows = statement
        .query_map([], |r| {
            Ok(PackEntry {
                pack_id: r.get(0)?,
                version: r.get(1)?,
                manifest_sha256: r.get(2)?,
                stored: r.get(3)?,
                payload_bytes: u64::from(r.get::<_, u32>(4)?),
                installed_at: r.get(5)?,
                active: r.get(6)?,
            })
        })
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error)?;
    Ok(rows)
}

fn active_versions(connection: &Connection) -> Result<Vec<(String, u32)>> {
    let mut statement = connection
        .prepare("SELECT pack_id,version FROM active_packs ORDER BY pack_id")
        .map_err(error)?;
    let rows = statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error)?;
    Ok(rows)
}

fn read_pack(
    connection: &Connection,
    id: &str,
    version: u32,
    trust: &ContentTrust,
) -> Result<ValidatedPack> {
    let (digest, size, stored): (String,u32,bool) = connection.query_row(
        "SELECT digest,length(manifest),stored FROM pack_versions WHERE pack_id=?1 AND version=?2", params![id,version], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
    ).optional().map_err(error)?.ok_or("This pack version is not recorded.")?;
    if !stored {
        return Err(
            "This version's payloads have been removed. Reinstall the exact original pack.".into(),
        );
    }
    if u64::from(size) > content::MAX_MANIFEST_BYTES {
        return Err("Stored manifest exceeds the reader's limit.".into());
    }
    let manifest: Vec<u8> = connection
        .query_row(
            "SELECT manifest FROM pack_versions WHERE pack_id=?1 AND version=?2",
            params![id, version],
            |r| r.get(0),
        )
        .map_err(error)?;
    if hash(&manifest) != digest {
        return Err(
            "Stored manifest integrity check failed. Reinstall the exact original pack.".into(),
        );
    }
    let pack = content::validate_bytes(manifest, trust, |path, limit| {
        let size: u32 = connection
            .query_row(
                "SELECT length(bytes) FROM pack_files WHERE pack_id=?1 AND version=?2 AND path=?3",
                params![id, version, path],
                |r| r.get(0),
            )
            .optional()
            .map_err(error)?
            .ok_or("An installed payload is missing. Reinstall the pack.")?;
        if u64::from(size) > limit {
            return Err("Installed payload exceeds its declared size.".into());
        }
        connection
            .query_row(
                "SELECT bytes FROM pack_files WHERE pack_id=?1 AND version=?2 AND path=?3",
                params![id, version, path],
                |r| r.get(0),
            )
            .map_err(error)
    })?;
    if pack.manifest.id != id || pack.manifest.version != version {
        return Err("Stored pack identity disagrees with the catalogue.".into());
    }
    Ok(pack)
}

fn record_identity(
    connection: &Connection,
    pack: &str,
    kind: &str,
    id: &str,
    version: u32,
    digest: &str,
    historical: bool,
) -> Result<()> {
    let highest: u32 = connection.query_row("SELECT coalesce(max(version),0) FROM identities WHERE pack_id=?1 AND kind=?2 AND content_id=?3", params![pack,kind,id], |r| r.get(0)).map_err(error)?;
    if version < highest && !historical {
        return Err(format!("{kind} {id} regresses its recorded version."));
    }
    let previous: Option<String> = connection.query_row("SELECT digest FROM identities WHERE pack_id=?1 AND kind=?2 AND content_id=?3 AND version=?4", params![pack,kind,id,version], |r| r.get(0)).optional().map_err(error)?;
    if previous.as_deref().is_some_and(|p| p != digest) {
        return Err(format!(
            "{kind} {id} changed without increasing its version."
        ));
    }
    connection.execute("INSERT OR IGNORE INTO identities(pack_id,kind,content_id,version,digest) VALUES(?1,?2,?3,?4,?5)", params![pack,kind,id,version,digest]).map_err(error)?;
    Ok(())
}

fn record_active_ids(
    pack: &ValidatedPack,
    ids: &mut HashSet<(&'static str, String)>,
) -> Result<()> {
    for (kind, id) in pack
        .lessons
        .iter()
        .map(|l| ("lesson", l.id.as_str()))
        .chain(
            pack.lessons
                .iter()
                .flat_map(|l| l.exercises.iter().map(|e| ("exercise", e.id.as_str()))),
        )
        .chain(
            pack.manifest
                .curriculum
                .objectives
                .iter()
                .map(|o| ("objective", o.id.as_str())),
        )
        .chain(
            pack.manifest
                .curriculum
                .targets
                .iter()
                .map(|t| ("target", t.id.as_str())),
        )
    {
        if !ids.insert((kind, id.to_string())) {
            return Err(format!(
                "Active packs conflict on {kind} ID {id}. Deactivate the conflicting pack first."
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
