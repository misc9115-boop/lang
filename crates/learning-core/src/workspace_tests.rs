use super::*;
use crate::{content::*, Answer, BackupController, Settings};
use std::{collections::HashSet, fs};

const PACK: &str = "dev.pack.cafe";
#[test]
fn a_profile_lock_excludes_a_second_owner_and_releases_on_drop() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("profile.lock");
    let owner = lock_profile(&path).unwrap();
    assert!(lock_profile(&path).is_err());
    drop(owner);
    assert!(lock_profile(&path).is_ok());
}
fn fixture() -> tempfile::TempDir {
    let root = tempfile::Builder::new()
        .prefix("desktop caffè ")
        .tempdir()
        .unwrap();
    fs::write(
        root.path().join("pack.json"),
        include_bytes!("../../../content/development/pack.json"),
    )
    .unwrap();
    fs::write(
        root.path().join("cafe.json"),
        include_bytes!("../../../content/development/cafe.json"),
    )
    .unwrap();
    root
}
fn workspace(root: &Path, trust: ContentTrust) -> StudyWorkspace {
    let store = Store::open(
        &root.join("learner.sqlite3"),
        vec![crate::development_lesson().unwrap()],
        true,
    )
    .unwrap();
    let library = PackLibrary::open(&root.join("packs.sqlite3"), trust, LibraryMode::Development);
    StudyWorkspace::new(store, library)
}
fn install(workspace: &mut StudyWorkspace, source: &Path) {
    let ticket = workspace.prepare_pack(source).unwrap();
    workspace.install_pack(&ticket.token).unwrap();
}
fn settings(workspace: &mut StudyWorkspace) {
    workspace
        .save_settings(
            Settings {
                level: "A1".into(),
                time_zone: "America/Chicago".into(),
                daily_goal: 10,
                theme: "light".into(),
                reduced_motion: false,
                text_scale: 100,
            },
            Utc::now(),
        )
        .unwrap();
}
fn update(root: &Path, change: impl FnOnce(&mut PackManifest)) {
    let mut pack: PackManifest =
        serde_json::from_slice(&fs::read(root.join("pack.json")).unwrap()).unwrap();
    change(&mut pack);
    fs::write(
        root.join("pack.json"),
        serde_json::to_vec_pretty(&pack).unwrap(),
    )
    .unwrap();
}
fn checksum(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn previews_are_cancelled_single_use_and_pinned_to_exact_source() {
    let source = fixture();
    let root = fixture();
    let mut w = workspace(root.path(), ContentTrust::default());
    let ticket = w.prepare_pack(source.path()).unwrap();
    w.cancel_pack();
    assert!(w.install_pack(&ticket.token).is_err());
    let ticket = w.prepare_pack(source.path()).unwrap();
    update(source.path(), |m| m.title.push_str(" changed"));
    assert!(w
        .install_pack(&ticket.token)
        .unwrap_err()
        .contains("changed after preview"));
    assert!(w.pack_state().entries.is_empty());
    let ticket = w.prepare_pack(source.path()).unwrap();
    assert!(w.install_pack("wrong-token").is_err());
    assert!(w.install_pack(&ticket.token).is_err());
    let ticket = w.prepare_pack(source.path()).unwrap();
    w.install_pack(&ticket.token).unwrap();
    assert!(w.install_pack(&ticket.token).is_err());
}

#[test]
fn references_round_trip_require_matching_local_packs_and_invalidate_old_preview() {
    let source = fixture();
    let root = fixture();
    let mut w = workspace(root.path(), ContentTrust::default());
    settings(&mut w);
    let now = Utc::now();
    let legacy = w.export_backup(now).unwrap();
    let mut controller = BackupController::default();
    let old = controller.prepare(&w, legacy.clone(), now).unwrap();
    install(&mut w, source.path());
    assert!(controller
        .restore(&mut w, &old.token, &root.path().join("recovery"), now)
        .is_err());
    let bytes = w.export_backup(now).unwrap();
    let preview = w.inspect_backup(&bytes, now).unwrap();
    assert_eq!(preview.pack_references.len(), 1);
    assert_eq!(preview.pack_references[0].pack_id, PACK);
    let other = fixture();
    let mut target = workspace(other.path(), ContentTrust::default());
    settings(&mut target);
    assert!(target
        .inspect_backup(&bytes, now)
        .unwrap_err()
        .contains("matching verified pack"));
    install(&mut target, source.path());
    let fingerprint = target.backup_fingerprint().unwrap();
    target
        .restore_backup(&bytes, &fingerprint, &other.path().join("recovery"), now)
        .unwrap();
    assert_eq!(
        target
            .inspect_backup(&target.export_backup(now).unwrap(), now)
            .unwrap()
            .pack_references,
        preview.pack_references
    );
    assert!(target.inspect_backup(&legacy, now).is_ok());
}

#[test]
fn corrupted_active_content_is_reported_without_silently_using_fixture() {
    let source = fixture();
    let root = fixture();
    let mut w = workspace(root.path(), ContentTrust::default());
    install(&mut w, source.path());
    w.change_pack("activate", PACK, 1).unwrap();
    let db = rusqlite::Connection::open(root.path().join("packs.sqlite3")).unwrap();
    db.execute("UPDATE pack_files SET bytes=x'00'", []).unwrap();
    let state = w.state(Utc::now()).unwrap();
    assert!(state.lessons.is_empty());
    assert!(!state.content_issues.is_empty());
    assert!(w.change_pack("remove", PACK, 1).is_err());
    install(&mut w, source.path());
    assert_eq!(w.state(Utc::now()).unwrap().lessons.len(), 1);
    assert!(w.state(Utc::now()).unwrap().content_issues.is_empty());
}

#[test]
fn unknown_catalogue_does_not_block_learner_state_or_destroy_it() {
    let root = fixture();
    let store = Store::open(&root.path().join("learner.sqlite3"), vec![], true).unwrap();
    let mut w = StudyWorkspace::new(store, Err("Unsupported catalogue schema".into()));
    settings(&mut w);
    let state = w.state(Utc::now()).unwrap();
    assert!(state.settings.is_some());
    assert_eq!(state.content_issues.len(), 1);
    assert!(w.prepare_pack(root.path()).is_err());
    assert!(w.export_backup(Utc::now()).is_ok());
}

#[test]
fn explicit_preview_can_cache_unknown_historical_version_without_switching() {
    let source = fixture();
    let root = fixture();
    let mut w = workspace(root.path(), ContentTrust::default());
    update(source.path(), |m| m.version = 2);
    install(&mut w, source.path());
    w.change_pack("activate", PACK, 2).unwrap();
    update(source.path(), |m| m.version = 1);
    install(&mut w, source.path());
    let state = w.pack_state();
    assert!(
        state
            .entries
            .iter()
            .find(|e| e.entry.version == 2)
            .unwrap()
            .entry
            .active
    );
    update(source.path(), |m| {
        m.version = 3;
        m.title.push_str(" higher");
    });
    install(&mut w, source.path());
    update(source.path(), |m| {
        m.version = 1;
        m.title.push_str(" reused");
    });
    let ticket = w.prepare_pack(source.path()).unwrap();
    assert!(w.install_pack(&ticket.token).is_err());
}

#[test]
fn historical_reviewed_snapshot_can_restore_after_update_and_is_protected_from_removal() {
    let source = fixture();
    let root = fixture();
    let mut lesson: Lesson =
        serde_json::from_slice(&fs::read(source.path().join("cafe.json")).unwrap()).unwrap();
    lesson.editorial_status = "approved".into();
    let bytes = serde_json::to_vec_pretty(&lesson).unwrap();
    fs::write(source.path().join("cafe.json"), &bytes).unwrap();
    update(source.path(), |m| {
        m.editorial_status = EditorialStatus::Approved;
        m.scope = PackScope::Course;
        m.lessons[0].estimated_minutes = Some(7); // Synthetic pacing only.
        m.lessons[0].file.bytes = bytes.len() as u64;
        m.lessons[0].file.sha256 = checksum(&bytes);
        m.reviews = vec![EditorialReview {
            reviewer: "Synthetic test record — no human review occurred".into(),
            reviewed_at: Utc::now(),
            areas: vec![
                ReviewArea::Linguistic,
                ReviewArea::Pedagogical,
                ReviewArea::AnswerVariants,
                ReviewArea::Licensing,
                ReviewArea::Pacing,
            ],
        }];
    });
    let old_manifest = fs::read(source.path().join("pack.json")).unwrap();
    let old_lesson = bytes;
    lesson.version = 2;
    lesson.title = "Synthetic reviewed update".into();
    let new_lesson = serde_json::to_vec_pretty(&lesson).unwrap();
    update(source.path(), |m| {
        m.version = 2;
        m.lessons[0].file.bytes = new_lesson.len() as u64;
        m.lessons[0].file.sha256 = checksum(&new_lesson);
    });
    let new_manifest = fs::read(source.path().join("pack.json")).unwrap();
    let digests = HashSet::from([checksum(&old_manifest), checksum(&new_manifest)]);
    let trust = ContentTrust {
        reviewed_manifest_sha256: digests.clone(),
    };
    fs::write(source.path().join("pack.json"), &old_manifest).unwrap();
    fs::write(source.path().join("cafe.json"), &old_lesson).unwrap();
    let mut w = workspace(root.path(), trust);
    settings(&mut w);
    install(&mut w, source.path());
    w.change_pack("activate", PACK, 1).unwrap();
    let now = Utc::now();
    let session = w.start("dev.a1.cafe", now).unwrap();
    w.save_draft(&session.id, "dev.cafe.choice", Answer::Option("a".into()))
        .unwrap();
    let backup = w.export_backup(now).unwrap();
    fs::write(source.path().join("pack.json"), new_manifest).unwrap();
    fs::write(source.path().join("cafe.json"), new_lesson).unwrap();
    install(&mut w, source.path());
    w.change_pack("activate", PACK, 2).unwrap();
    assert!(w
        .change_pack("remove", PACK, 1)
        .unwrap_err()
        .contains("history needs this version"));
    assert!(w.export_backup(now).is_ok());
    assert!(w.inspect_backup(&backup, now).is_ok());
    let fingerprint = w.backup_fingerprint().unwrap();
    w.restore_backup(&backup, &fingerprint, &root.path().join("recovery"), now)
        .unwrap();
    assert_eq!(
        w.state(now).unwrap().lessons[0].title,
        "Synthetic reviewed update"
    );
    assert_eq!(w.session(&session.id).unwrap().lesson.title, "At the café");
    assert_eq!(
        w.session(&session.id).unwrap().exercise.unwrap().draft,
        Some(Answer::Option("a".into()))
    );
    drop(w);
    let mut reopened = workspace(
        root.path(),
        ContentTrust {
            reviewed_manifest_sha256: digests,
        },
    );
    assert!(reopened.export_backup(now).is_ok());
    assert_eq!(
        reopened.state(now).unwrap().lessons[0].title,
        "Synthetic reviewed update"
    );
    drop(reopened);
    let mut revoked = workspace(root.path(), ContentTrust::default());
    assert!(revoked.reveal(&session.id, "dev.cafe.choice", now).is_err());
    assert!(revoked
        .advance(&session.id, "dev.cafe.choice", now)
        .is_err());
    assert!(revoked.export_backup(now).is_err());
    assert!(revoked.inspect_backup(&backup, now).is_err());
    assert!(!revoked.state(now).unwrap().content_issues.is_empty());
}
