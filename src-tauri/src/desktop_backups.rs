use crate::Database;
use chrono::{DateTime, Utc};
use learning_core::{
    read_backup_file, write_new_backup, BackupController, BackupTicket, RestoreResult,
};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

type Result<T> = std::result::Result<T, String>;
pub struct BackupServices {
    pub controller: Mutex<BackupController>,
    pub recovery_directory: PathBuf,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryEntry {
    id: String,
    saved_at: DateTime<Utc>,
}

async fn background<T: Send + 'static>(
    action: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    tauri::async_runtime::spawn_blocking(action)
        .await
        .map_err(|_| "The backup operation could not finish.".to_string())?
}
fn prepare(app: &AppHandle, path: &Path) -> Result<BackupTicket> {
    let bytes = read_backup_file(path)?;
    let services = app.state::<BackupServices>();
    let database = app.state::<Database>();
    let mut controller = services
        .controller
        .lock()
        .map_err(|_| "Backup preview is unavailable.")?;
    let store = database
        .0
        .lock()
        .map_err(|_| "Local storage is unavailable.")?;
    controller.prepare(&store, bytes, Utc::now())
}

#[tauri::command]
pub async fn export_learner_backup(app: AppHandle) -> Result<Option<String>> {
    background(move || {
        let suggested = format!(
            "italian-learning-{}.langbackup",
            Utc::now().format("%Y%m%d-%H%M%S")
        );
        let Some(file) = app
            .dialog()
            .file()
            .add_filter("Learner backup", &["langbackup"])
            .set_file_name(&suggested)
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let path = file
            .into_path()
            .map_err(|_| "Choose a local backup file.")?;
        if !path.extension().is_some_and(|extension| {
            extension
                .to_string_lossy()
                .eq_ignore_ascii_case("langbackup")
        }) {
            return Err("Use the .langbackup file extension.".into());
        }
        let database = app.state::<Database>();
        let store = database
            .0
            .lock()
            .map_err(|_| "Local storage is unavailable.")?;
        let bytes = store.export_backup(Utc::now())?;
        write_new_backup(&path, &bytes)?;
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
}

#[tauri::command]
pub async fn choose_learner_backup(app: AppHandle) -> Result<Option<BackupTicket>> {
    background(move || {
        app.state::<BackupServices>()
            .controller
            .lock()
            .map_err(|_| "Backup preview is unavailable.")?
            .cancel();
        let Some(file) = app
            .dialog()
            .file()
            .add_filter("Learner backup", &["langbackup"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = file
            .into_path()
            .map_err(|_| "Choose a local backup file.")?;
        prepare(&app, &path).map(Some)
    })
    .await
}

#[tauri::command]
pub async fn restore_learner_backup(app: AppHandle, token: String) -> Result<RestoreResult> {
    background(move || {
        let services = app.state::<BackupServices>();
        let database = app.state::<Database>();
        let mut controller = services
            .controller
            .lock()
            .map_err(|_| "Backup preview is unavailable.")?;
        let mut store = database
            .0
            .lock()
            .map_err(|_| "Local storage is unavailable.")?;
        controller.restore(&mut store, &token, &services.recovery_directory, Utc::now())
    })
    .await
}

#[tauri::command]
pub fn cancel_backup_import(app: AppHandle) -> Result<()> {
    app.state::<BackupServices>()
        .controller
        .lock()
        .map_err(|_| "Backup preview is unavailable.")?
        .cancel();
    Ok(())
}

#[tauri::command]
pub async fn list_recovery_backups(app: AppHandle) -> Result<Vec<RecoveryEntry>> {
    background(move || {
        let services = app.state::<BackupServices>();
        let directory = &services.recovery_directory;
        if !directory.exists() {
            return Ok(vec![]);
        }
        let mut entries = vec![];
        for entry in fs::read_dir(directory).map_err(|_| "Recovery backups could not be listed.")? {
            let entry = entry.map_err(|_| "A recovery backup could not be read.")?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if valid_recovery_name(&id) && entry.file_type().map_err(|e| e.to_string())?.is_file() {
                let saved_at = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .map_err(|e| e.to_string())?;
                entries.push(RecoveryEntry {
                    id,
                    saved_at: saved_at.into(),
                });
            }
        }
        entries.sort_by(|a, b| b.saved_at.cmp(&a.saved_at));
        entries.truncate(50);
        Ok(entries)
    })
    .await
}
fn valid_recovery_name(id: &str) -> bool {
    id.starts_with("pre-import-")
        && id.ends_with(".langbackup")
        && id.len() < 150
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

#[tauri::command]
pub async fn preview_recovery_backup(app: AppHandle, id: String) -> Result<BackupTicket> {
    background(move || {
        app.state::<BackupServices>()
            .controller
            .lock()
            .map_err(|_| "Backup preview is unavailable.")?
            .cancel();
        if !valid_recovery_name(&id) {
            return Err("Choose a listed recovery backup.".into());
        }
        let services = app.state::<BackupServices>();
        let directory = &services.recovery_directory;
        let path = directory.join(id);
        let root = directory
            .canonicalize()
            .map_err(|_| "Recovery storage is unavailable.")?;
        let selected = path
            .canonicalize()
            .map_err(|_| "The recovery backup is unavailable.")?;
        if !selected.starts_with(root) {
            return Err("The recovery backup is outside application storage.".into());
        }
        prepare(&app, &selected)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovery_selection_rejects_paths_and_nonbackup_names() {
        for name in [
            "../pre-import-x.langbackup",
            "pre-import-../../file.langbackup",
            "pre-import-C:\\file.langbackup",
            "pre-import-x.langbackup/child",
            "learner.sqlite3",
        ] {
            assert!(!valid_recovery_name(name));
        }
        assert!(valid_recovery_name(
            "pre-import-20261014T120000Z-00e31037-56d2-4578-a1f0-3c909ba0716e.langbackup"
        ));
    }
}
