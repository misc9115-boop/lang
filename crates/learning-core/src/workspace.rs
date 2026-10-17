//! Coordinates installed content with learner snapshots; never grants review trust.
use crate::{content::EditorialStatus, pack_library::*, AppState, Lesson, Store};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};
use uuid::Uuid;
type Result<T> = std::result::Result<T, String>;

/// Hold the returned handle for the entire desktop profile lifetime.
pub fn lock_profile(path: &Path) -> Result<std::fs::File> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.try_lock().map_err(|e|format!("This study profile could not be locked. It may already be open in another app window: {e}"))?;
    Ok(file)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackRecord {
    pub entry: PackEntry,
    pub title: Option<String>,
    pub editorial_status: Option<EditorialStatus>,
    pub error: Option<String>,
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PackState {
    pub entries: Vec<PackRecord>,
    pub issues: Vec<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackTicket {
    pub token: String,
    pub pack_id: String,
    pub version: u32,
    pub title: String,
    pub payload_bytes: u64,
    pub lesson_count: usize,
    pub editorial_status: EditorialStatus,
    pub license: String,
    pub attribution: String,
}
struct Pending {
    token: String,
    root: PathBuf,
    digest: String,
}
pub struct StudyWorkspace {
    store: Store,
    library: Result<PackLibrary>,
    fallback: Vec<Lesson>,
    pending: Option<Pending>,
}
impl Deref for StudyWorkspace {
    type Target = Store;
    fn deref(&self) -> &Store {
        &self.store
    }
}
impl DerefMut for StudyWorkspace {
    fn deref_mut(&mut self) -> &mut Store {
        &mut self.store
    }
}
impl StudyWorkspace {
    pub fn new(store: Store, library: Result<PackLibrary>) -> Self {
        let fallback = store.lessons.clone();
        let mut workspace = Self {
            store,
            library,
            fallback,
            pending: None,
        };
        workspace.pack_state();
        workspace
    }
    pub fn state(&mut self, now: DateTime<Utc>) -> Result<AppState> {
        self.pack_state();
        self.store.state(now)
    }
    pub fn pack_state(&mut self) -> PackState {
        let mut state = PackState::default();
        let mut context = ContentContext {
            managed: true,
            ..Default::default()
        };
        let mut lessons = self.fallback.clone();
        match &self.library {
            Err(error) => state
                .issues
                .push(format!("Pack catalogue unavailable: {error}")),
            Ok(library) => match library.entries() {
                Err(error) => state
                    .issues
                    .push(format!("Pack catalogue could not be read: {error}")),
                Ok(entries) => {
                    let any_active = entries.iter().any(|e| e.active);
                    for entry in entries {
                        let mut record = PackRecord {
                            entry,
                            title: None,
                            editorial_status: None,
                            error: None,
                        };
                        if record.entry.stored {
                            match library.verify(&record.entry.pack_id, record.entry.version) {
                                Err(error) => {
                                    state.issues.push(format!(
                                        "{} v{}: {error}",
                                        record.entry.pack_id, record.entry.version
                                    ));
                                    record.error = Some(error);
                                }
                                Ok(pack) => {
                                    record.title = Some(pack.manifest.title);
                                    record.editorial_status = Some(pack.manifest.editorial_status);
                                    context.references.push(PackReference {
                                        pack_id: record.entry.pack_id.clone(),
                                        version: record.entry.version,
                                        manifest_sha256: record.entry.manifest_sha256.clone(),
                                    });
                                    if pack.report.trusted_review {
                                        for lesson in pack.lessons {
                                            if let Ok(bytes) = serde_json::to_vec(&lesson) {
                                                context
                                                    .reviewed_snapshots
                                                    .insert(format!("{:x}", Sha256::digest(bytes)));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        state.entries.push(record);
                    }
                    if any_active {
                        match library.active_lessons() {
                            Ok(active) => lessons = active,
                            Err(error) => {
                                lessons.clear();
                                state
                                    .issues
                                    .push(format!("Installed lessons unavailable: {error}"));
                            }
                        }
                    }
                }
            },
        }
        self.store.lessons = lessons;
        self.store.content = context;
        self.store.content_issues = state.issues.clone();
        state
    }
    pub fn prepare_pack(&mut self, root: &Path) -> Result<PackTicket> {
        self.pending = None;
        let pack = self
            .library
            .as_ref()
            .map_err(Clone::clone)?
            .preview_directory(root)?;
        let token = Uuid::new_v4().to_string();
        let ticket = PackTicket {
            token: token.clone(),
            pack_id: pack.manifest.id,
            version: pack.manifest.version,
            title: pack.manifest.title,
            payload_bytes: pack.report.payload_bytes,
            lesson_count: pack.lessons.len(),
            editorial_status: pack.manifest.editorial_status,
            license: pack.manifest.provenance.license,
            attribution: pack.manifest.provenance.attribution,
        };
        self.pending = Some(Pending {
            token,
            root: root.to_path_buf(),
            digest: pack.report.manifest_sha256,
        });
        Ok(ticket)
    }
    pub fn cancel_pack(&mut self) {
        self.pending = None;
    }
    pub fn install_pack(&mut self, token: &str) -> Result<PackState> {
        let pending = self
            .pending
            .take()
            .ok_or("Choose a pack folder again before installing.")?;
        if pending.token != token {
            return Err("This pack preview is no longer available.".into());
        }
        self.library
            .as_mut()
            .map_err(|e| e.clone())?
            .install_previewed(&pending.root, &pending.digest)?;
        Ok(self.pack_state())
    }
    pub fn change_pack(&mut self, action: &str, id: &str, version: u32) -> Result<PackState> {
        let library = self.library.as_mut().map_err(|e| e.clone())?;
        match action {
            "activate" => library.activate(id, version)?,
            "deactivate" => library.deactivate(id)?,
            "remove" => {
                // Conservatively retain every version whose reviewed lesson ID/
                // version is referenced by saved history. No learner row is deleted.
                if let Ok(pack) = library.verify(id, version) {
                    for lesson in pack
                        .lessons
                        .iter()
                        .filter(|l| l.editorial_status == "approved")
                    {
                        let used:bool=self.store.connection.query_row("SELECT EXISTS(SELECT 1 FROM sessions WHERE lesson_id=?1 AND lesson_version=?2 AND practice_only=0)",rusqlite::params![lesson.id,lesson.version],|r|r.get(0)).map_err(|e|e.to_string())?;
                        if used {
                            return Err("Saved reviewed lesson history needs this version. Keep its payloads for backup validation; you can deactivate it.".into());
                        }
                    }
                } else {
                    // An unreadable version cannot safely be classified as unused.
                    return Err("Repair this pack before removing it so saved-history references can be checked.".into());
                }
                library.remove_version(id, version)?;
            }
            _ => return Err("Unsupported pack action.".into()),
        }
        Ok(self.pack_state())
    }
}
