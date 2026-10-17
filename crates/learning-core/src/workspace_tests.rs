use super::*;
use crate::{content::*, Answer, BackupController, Settings};
use sha2::{Digest, Sha256};
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

fn reviewed_fixture(single: bool) -> (tempfile::TempDir, ContentTrust) {
    let source = fixture();
    let mut lesson = crate::development_lesson().unwrap();
    lesson.editorial_status = "approved".into();
    if single {
        lesson.exercises.truncate(1);
    }
    let bytes = serde_json::to_vec_pretty(&lesson).unwrap();
    fs::write(source.path().join("cafe.json"), &bytes).unwrap();
    update(source.path(), |m| {
        m.editorial_status = EditorialStatus::Approved;
        m.scope = PackScope::Course;
        m.lessons[0].estimated_minutes = Some(7);
        m.lessons[0].file.bytes = bytes.len() as u64;
        m.lessons[0].file.sha256 = checksum(&bytes);
        if single {
            m.lessons[0].exercises.truncate(1);
        }
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
    let digest = checksum(&fs::read(source.path().join("pack.json")).unwrap());
    (
        source,
        ContentTrust {
            reviewed_manifest_sha256: HashSet::from([digest]),
        },
    )
}

fn reviewed_workspace(single: bool) -> (tempfile::TempDir, tempfile::TempDir, StudyWorkspace) {
    let (source, trust) = reviewed_fixture(single);
    let root = fixture();
    let mut w = workspace(root.path(), trust);
    settings(&mut w);
    install(&mut w, source.path());
    w.change_pack("activate", PACK, 1).unwrap();
    (source, root, w)
}

#[test]
fn reviewed_lesson_seeds_mapped_skill_histories_once_per_session_and_preserves_evidence() {
    use crate::{content::Skill, review::Rating};
    let (_source, _root, mut w) = reviewed_workspace(false);
    let now = Utc::now();
    let session = w.start("dev.a1.cafe", now).unwrap();
    assert!(w.review_histories().unwrap().is_empty());
    w.submit(
        &session.id,
        "dev.cafe.choice",
        Answer::Option("b".into()),
        &uuid::Uuid::new_v4().to_string(),
        now,
    )
    .unwrap();
    w.submit(
        &session.id,
        "dev.cafe.choice",
        Answer::Option("a".into()),
        &uuid::Uuid::new_v4().to_string(),
        now,
    )
    .unwrap();
    w.advance(&session.id, "dev.cafe.choice", now).unwrap();
    let first = w.review_histories().unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].schedule().unwrap().last_rating, Rating::Hard);
    assert!(!first[0].history().observations()[0].evidence.first_correct);
    w.advance(&session.id, "dev.cafe.choice", now).unwrap();
    assert_eq!(
        w.review_histories().unwrap()[0]
            .history()
            .observations()
            .len(),
        1
    );
    let mut active = w.session(&session.id).unwrap();
    while active.status == "active" {
        let exercise = active.exercise.unwrap();
        w.reveal(&session.id, &exercise.id, now).unwrap();
        active = w.advance(&session.id, &exercise.id, now).unwrap();
    }
    let records = w.review_histories().unwrap();
    assert_eq!(records.len(), 2);
    for record in &records {
        assert_eq!(record.history().observations().len(), 1);
    }
    assert_eq!(
        records
            .iter()
            .find(|r| r.history().key().skill == Skill::Reading)
            .unwrap()
            .schedule()
            .unwrap()
            .last_rating,
        Rating::Hard
    );
    assert_eq!(
        records
            .iter()
            .find(|r| r.history().key().skill == Skill::Writing)
            .unwrap()
            .schedule()
            .unwrap()
            .last_rating,
        Rating::Again
    );
    assert_eq!(w.state(now).unwrap().progress.total_xp, 10); // Lesson credit only.
    assert_eq!(
        w.inspect_backup(&w.export_backup(now).unwrap(), now)
            .unwrap()
            .review_item_count,
        2
    );
}

#[test]
fn review_write_lesson_completion_xp_and_achievements_commit_or_roll_back_together() {
    let (_source, _root, mut w) = reviewed_workspace(true);
    let now = Utc::now();
    let session = w.start("dev.a1.cafe", now).unwrap();
    w.submit(
        &session.id,
        "dev.cafe.choice",
        Answer::Option("a".into()),
        &uuid::Uuid::new_v4().to_string(),
        now,
    )
    .unwrap();
    let before = w.backup_fingerprint().unwrap();
    w.connection.execute_batch("CREATE TRIGGER fail_review_completion BEFORE UPDATE OF status ON sessions WHEN NEW.status='completed' BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    assert!(w.advance(&session.id, "dev.cafe.choice", now).is_err());
    assert_eq!(w.backup_fingerprint().unwrap(), before);
    assert!(w.review_histories().unwrap().is_empty());
    assert_eq!(w.session(&session.id).unwrap().status, "active");
    assert_eq!(w.state(now).unwrap().progress.total_xp, 0);
    w.connection
        .execute_batch("DROP TRIGGER fail_review_completion")
        .unwrap();
    w.advance(&session.id, "dev.cafe.choice", now).unwrap();
    assert_eq!(w.review_histories().unwrap().len(), 1);
    assert_eq!(w.state(now).unwrap().progress.total_xp, 10);
}

#[test]
fn review_history_survives_restart_and_restore_and_protects_referenced_pack() {
    let (source, root, mut w) = reviewed_workspace(true);
    let now = Utc::now();
    let session = w.start("dev.a1.cafe", now).unwrap();
    w.reveal(&session.id, "dev.cafe.choice", now).unwrap();
    w.advance(&session.id, "dev.cafe.choice", now).unwrap();
    let archive = w.export_backup(now).unwrap();
    assert!(w
        .change_pack("remove", PACK, 1)
        .unwrap_err()
        .contains("review history"));
    let before = serde_json::to_value(w.review_histories().unwrap()).unwrap();
    let digests = ContentTrust {
        reviewed_manifest_sha256: HashSet::from([checksum(
            &fs::read(source.path().join("pack.json")).unwrap(),
        )]),
    };
    drop(w);
    let mut w = workspace(root.path(), digests);
    assert_eq!(
        serde_json::to_value(w.review_histories().unwrap()).unwrap(),
        before
    );
    let next = w.start("dev.a1.cafe", now).unwrap();
    w.reveal(&next.id, "dev.cafe.choice", now).unwrap();
    w.advance(&next.id, "dev.cafe.choice", now).unwrap();
    assert_eq!(
        w.review_histories().unwrap()[0]
            .history()
            .observations()
            .len(),
        2
    );
    assert_eq!(w.state(now).unwrap().progress.total_xp, 10);
    let fingerprint = w.backup_fingerprint().unwrap();
    w.restore_backup(&archive, &fingerprint, &root.path().join("recovery"), now)
        .unwrap();
    assert_eq!(
        serde_json::to_value(w.review_histories().unwrap()).unwrap(),
        before
    );
    drop(w);
    let revoked = workspace(root.path(), ContentTrust::default());
    assert_eq!(
        serde_json::to_value(revoked.review_histories().unwrap()).unwrap(),
        before
    );
    assert!(revoked.inspect_backup(&archive, now).is_err());
    assert!(revoked.export_backup(now).is_err());
}

#[test]
fn draft_lessons_never_seed_review_history() {
    let source = fixture();
    let root = fixture();
    let mut w = workspace(root.path(), ContentTrust::default());
    settings(&mut w);
    install(&mut w, source.path());
    w.change_pack("activate", PACK, 1).unwrap();
    let now = Utc::now();
    let session = w.start("dev.a1.cafe", now).unwrap();
    w.reveal(&session.id, "dev.cafe.choice", now).unwrap();
    w.advance(&session.id, "dev.cafe.choice", now).unwrap();
    assert!(w.review_histories().unwrap().is_empty());
}

#[test]
fn review_observations_keep_historical_sources_after_pack_update_and_restore() {
    let (source, root, mut w) = reviewed_workspace(true);
    let now = Utc::now();
    let first = w.start("dev.a1.cafe", now).unwrap();
    w.reveal(&first.id, "dev.cafe.choice", now).unwrap();
    w.advance(&first.id, "dev.cafe.choice", now).unwrap();
    let old = w.export_backup(now).unwrap();
    let old_digest = checksum(&fs::read(source.path().join("pack.json")).unwrap());
    drop(w);
    let mut lesson: Lesson =
        serde_json::from_slice(&fs::read(source.path().join("cafe.json")).unwrap()).unwrap();
    lesson.version = 2;
    lesson.title = "Synthetic version 2 — no human review occurred".into();
    let bytes = serde_json::to_vec_pretty(&lesson).unwrap();
    fs::write(source.path().join("cafe.json"), &bytes).unwrap();
    update(source.path(), |m| {
        m.version = 2;
        m.lessons[0].file.bytes = bytes.len() as u64;
        m.lessons[0].file.sha256 = checksum(&bytes);
    });
    let new_digest = checksum(&fs::read(source.path().join("pack.json")).unwrap());
    let mut w = workspace(
        root.path(),
        ContentTrust {
            reviewed_manifest_sha256: HashSet::from([old_digest, new_digest]),
        },
    );
    install(&mut w, source.path());
    w.change_pack("activate", PACK, 2).unwrap();
    let second = w.start("dev.a1.cafe", now).unwrap();
    w.reveal(&second.id, "dev.cafe.choice", now).unwrap();
    w.advance(&second.id, "dev.cafe.choice", now).unwrap();
    let history = w.review_histories().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].history().observations().len(), 2);
    assert_eq!(
        history[0]
            .sources()
            .iter()
            .map(|s| s.pack.version)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert!(w.export_backup(now).is_ok());
    assert!(w.inspect_backup(&old, now).is_ok());
    let fingerprint = w.backup_fingerprint().unwrap();
    w.restore_backup(&old, &fingerprint, &root.path().join("recovery"), now)
        .unwrap();
    assert_eq!(
        w.review_histories().unwrap()[0].sources()[0].pack.version,
        1
    );
    assert_eq!(w.state(now).unwrap().lessons[0].title, lesson.title);
    assert!(w.change_pack("remove", PACK, 1).is_err());
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
