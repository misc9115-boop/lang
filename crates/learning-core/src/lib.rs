mod backup;
pub mod content;
mod model;
pub mod pack_library;
pub mod review;
pub mod review_session;
pub mod review_storage;
mod scoring;
mod store;
mod workspace;
pub use workspace::{lock_profile, PackRecord, PackState, PackTicket, StudyWorkspace};

pub use backup::{
    read_backup_file, write_new_backup, BackupController, BackupPreview, BackupTicket,
    RestoreResult, MAX_BACKUP_BYTES,
};
pub use model::*;
pub use scoring::normalize;
pub use store::Store;

pub const LEVELS: [&str; 7] = ["Beginner", "A1", "A2", "B1", "B2", "C1", "C2"];

#[cfg(any(debug_assertions, test))]
pub fn development_lesson() -> Result<Lesson, String> {
    serde_json::from_str(include_str!("../../../content/development/cafe.json"))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;
