use super::*;

fn now() -> DateTime<Utc> {
    "2026-10-14T12:00:00Z".parse().unwrap()
}
fn settings() -> Settings {
    Settings {
        level: "A1".into(),
        time_zone: "America/Chicago".into(),
        daily_goal: 10,
        theme: "system".into(),
        reduced_motion: false,
        text_scale: 100,
    }
}
fn store(approved: bool) -> Store {
    let mut lesson = development_lesson().unwrap();
    if approved {
        lesson.editorial_status = "approved".into();
    }
    let mut store = Store::memory(vec![lesson]).unwrap();
    store.save_settings(settings(), now()).unwrap();
    store
}
fn source() -> Store {
    let mut store = store(false);
    let session = store.start("dev.a1.cafe", now()).unwrap();
    store
        .submit(
            &session.id,
            "dev.cafe.choice",
            Answer::Option("b".into()),
            &Uuid::new_v4().to_string(),
            now(),
        )
        .unwrap();
    store.hint(&session.id, "dev.cafe.choice").unwrap();
    store
        .submit(
            &session.id,
            "dev.cafe.choice",
            Answer::Option("a".into()),
            &Uuid::new_v4().to_string(),
            now(),
        )
        .unwrap();
    store
        .advance(&session.id, "dev.cafe.choice", now())
        .unwrap();
    store
        .save_draft(
            &session.id,
            "dev.cafe.translation",
            Answer::Text("Un ca".into()),
        )
        .unwrap();
    store
        .report_answer(
            &session.id,
            "dev.cafe.choice",
            "Test-only local editorial report.",
            now(),
        )
        .unwrap();
    store
}
fn modified(bytes: &[u8], change: impl FnOnce(&mut Archive)) -> Vec<u8> {
    let mut archive: Archive = serde_json::from_slice(bytes).unwrap();
    change(&mut archive);
    archive.sha256 = hash(&archive.body).unwrap();
    serde_json::to_vec(&archive).unwrap()
}

#[test]
fn round_trip_preserves_drafts_evidence_and_recovery_history() {
    let source = source();
    let archive = source.export_backup(now()).unwrap();
    let mut target = store(false);
    let mut changed = settings();
    changed.level = "C2".into();
    target.save_settings(changed, now()).unwrap();
    let before = target.backup_fingerprint().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let preview = target.inspect_backup(&archive, now()).unwrap();
    assert_eq!(preview.level.as_deref(), Some("A1"));
    assert_eq!(preview.session_count, 1);
    let result = target
        .restore_backup(&archive, &before, dir.path(), now())
        .unwrap();
    assert_eq!(
        source.backup_fingerprint().unwrap(),
        target.backup_fingerprint().unwrap()
    );
    let active = result.state.active_session.unwrap();
    assert_eq!(active.first_correct, 0);
    assert_eq!(active.hinted_items, 1);
    assert_eq!(
        active.exercise.unwrap().draft,
        Some(Answer::Text("Un ca".into()))
    );
    let recovery = read_backup_file(&result.recovery_path).unwrap();
    let current = target.backup_fingerprint().unwrap();
    target
        .restore_backup(&recovery, &current, dir.path(), now())
        .unwrap();
    assert_eq!(target.backup_fingerprint().unwrap(), before);
}

#[test]
fn earned_xp_days_and_achievements_survive_backup() {
    let mut source = store(true);
    let mut session = source.start("dev.a1.cafe", now()).unwrap();
    while session.status == "active" {
        let exercise = session.exercise.as_ref().unwrap().id.clone();
        source.reveal(&session.id, &exercise, now()).unwrap();
        session = source.advance(&session.id, &exercise, now()).unwrap();
    }
    let bytes = source.export_backup(now()).unwrap();
    let mut target = store(true);
    let before = target.backup_fingerprint().unwrap();
    let dir = tempfile::tempdir().unwrap();
    target
        .restore_backup(&bytes, &before, dir.path(), now())
        .unwrap();
    let progress = target.progress(now()).unwrap();
    assert_eq!(progress.total_xp, 10);
    assert_eq!(progress.current_streak, 1);
    assert_eq!(progress.achievements, vec!["first_lesson"]);
    assert_eq!(
        source.backup_fingerprint().unwrap(),
        target.backup_fingerprint().unwrap()
    );
}

#[test]
fn corruption_and_unknown_fields_preserve_existing_data() {
    let bytes = source().export_backup(now()).unwrap();
    let mut target = store(false);
    let before = target.backup_fingerprint().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    json["body"]["data"]["settings"]["level"] = serde_json::json!("C2");
    assert!(target
        .restore_backup(
            &serde_json::to_vec(&json).unwrap(),
            &before,
            dir.path(),
            now()
        )
        .is_err());
    let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    json["body"]["data"]["futureLearnerData"] = serde_json::json!({"value":1});
    assert!(target
        .inspect_backup(&serde_json::to_vec(&json).unwrap(), now())
        .is_err());
    assert!(target
        .restore_backup(b"{broken", &before, dir.path(), now())
        .is_err());
    assert_eq!(target.backup_fingerprint().unwrap(), before);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn checksummed_invalid_evidence_is_rejected_before_replacement() {
    let bytes = source().export_backup(now()).unwrap();
    let mut target = store(false);
    let before = target.backup_fingerprint().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let bad = modified(&bytes, |a| a.body.data.items[0].first_correct = Some(true));
    assert!(target
        .restore_backup(&bad, &before, dir.path(), now())
        .is_err());
    let bad = modified(&bytes, |a| {
        a.body.data.items.pop();
    });
    assert!(target.inspect_backup(&bad, now()).is_err());
    let bad = modified(&bytes, |a| {
        a.body.data.attempts[0].session_id = Uuid::new_v4().to_string()
    });
    assert!(target.inspect_backup(&bad, now()).is_err());
    let bad = modified(&bytes, |a| a.body.data.sessions[0].practice_only = false);
    assert!(target.inspect_backup(&bad, now()).is_err());
    assert_eq!(target.backup_fingerprint().unwrap(), before);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn preview_is_invalidated_by_later_study_changes() {
    let bytes = source().export_backup(now()).unwrap();
    let mut target = store(false);
    let before = target.backup_fingerprint().unwrap();
    let mut changed = settings();
    changed.daily_goal = 20;
    target.save_settings(changed, now()).unwrap();
    let actual = target.backup_fingerprint().unwrap();
    let dir = tempfile::tempdir().unwrap();
    assert!(target
        .restore_backup(&bytes, &before, dir.path(), now())
        .unwrap_err()
        .contains("changed after"));
    assert_eq!(target.backup_fingerprint().unwrap(), actual);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn recovery_write_failure_preserves_live_data() {
    let bytes = source().export_backup(now()).unwrap();
    let mut target = store(false);
    let before = target.backup_fingerprint().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let not_directory = dir.path().join("file");
    fs::write(&not_directory, b"keep").unwrap();
    assert!(target
        .restore_backup(&bytes, &before, &not_directory, now())
        .is_err());
    assert_eq!(target.backup_fingerprint().unwrap(), before);
    assert_eq!(fs::read(&not_directory).unwrap(), b"keep");
}

#[test]
fn failed_import_rolls_back_and_keeps_verified_recovery() {
    let bytes = source().export_backup(now()).unwrap();
    let mut target = store(false);
    let before = target.backup_fingerprint().unwrap();
    let dir = tempfile::tempdir().unwrap();
    target.connection.execute_batch("CREATE TRIGGER reject_restore BEFORE INSERT ON profile BEGIN SELECT RAISE(ABORT,'injected restore write failure'); END;").unwrap();
    assert!(target
        .restore_backup(&bytes, &before, dir.path(), now())
        .is_err());
    assert_eq!(target.backup_fingerprint().unwrap(), before);
    let files = fs::read_dir(dir.path())
        .unwrap()
        .map(|f| f.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 1);
    let recovery = read_backup_file(&files[0]).unwrap();
    let recovered = parse_archive(&recovery, true).unwrap();
    assert_eq!(hash(&recovered.body.data).unwrap(), before);
}

#[test]
fn export_never_overwrites_existing_file() {
    let bytes = source().export_backup(now()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("save.langbackup");
    write_new_backup(&path, &bytes).unwrap();
    assert!(write_new_backup(&path, b"different").is_err());
    assert_eq!(read_backup_file(&path).unwrap(), bytes);
}

#[test]
fn future_versions_modes_and_pack_references_are_rejected() {
    let bytes = source().export_backup(now()).unwrap();
    let target = store(false);
    for bad in [
        modified(&bytes, |a| a.body.version = 2),
        modified(&bytes, |a| a.body.schema_version = 2),
        modified(&bytes, |a| a.body.development = false),
        modified(&bytes, |a| {
            a.body
                .data
                .installed_pack_references
                .push(serde_json::json!({"packId":"future"}))
        }),
    ] {
        assert!(target.inspect_backup(&bad, now()).is_err());
    }
    let release = Store::initialize(Connection::open_in_memory().unwrap(), vec![], false).unwrap();
    assert!(release.inspect_backup(&bytes, now()).is_err());
}

#[test]
fn report_text_is_data_and_unrelated_tables_are_not_exported() {
    let mut source = source();
    let session = source.state(now()).unwrap().active_session.unwrap();
    source
        .report_answer(
            &session.id,
            "dev.cafe.choice",
            "DROP TABLE sessions; -- literal report text",
            now(),
        )
        .unwrap();
    source.connection.execute_batch("CREATE TABLE unrelated_private_data(secret TEXT); INSERT INTO unrelated_private_data VALUES('must not export');").unwrap();
    let bytes = source.export_backup(now()).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("must not export"));
    let mut target = store(false);
    let dir = tempfile::tempdir().unwrap();
    let before = target.backup_fingerprint().unwrap();
    target
        .restore_backup(&bytes, &before, dir.path(), now())
        .unwrap();
    assert!(target.state(now()).unwrap().active_session.is_some());
    let count: u32 = target
        .connection
        .query_row("SELECT COUNT(*) FROM answer_reports", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn oversized_backup_is_rejected_without_parsing() {
    let bytes = vec![b' '; MAX_BACKUP_BYTES + 1];
    assert!(store(false)
        .inspect_backup(&bytes, now())
        .unwrap_err()
        .contains("64 MiB"));
}

#[test]
fn empty_profile_can_be_backed_up_and_restored() {
    let empty = Store::memory(vec![development_lesson().unwrap()]).unwrap();
    let bytes = empty.export_backup(now()).unwrap();
    let mut target = source();
    let before = target.backup_fingerprint().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let result = target
        .restore_backup(&bytes, &before, dir.path(), now())
        .unwrap();
    assert!(result.state.settings.is_none());
    assert!(result.state.active_session.is_none());
    assert_eq!(result.state.progress.total_xp, 0);
}

#[test]
fn controller_cancellation_and_consumed_tokens_cannot_restore() {
    let bytes = source().export_backup(now()).unwrap();
    let mut target = store(false);
    let before = target.backup_fingerprint().unwrap();
    let mut controller = BackupController::default();
    let dir = tempfile::tempdir().unwrap();
    let ticket = controller.prepare(&target, bytes.clone(), now()).unwrap();
    controller.cancel();
    assert!(controller
        .restore(&mut target, &ticket.token, dir.path(), now())
        .is_err());
    assert_eq!(target.backup_fingerprint().unwrap(), before);
    let ticket = controller.prepare(&target, bytes, now()).unwrap();
    assert!(controller
        .restore(&mut target, "wrong-token", dir.path(), now())
        .is_err());
    controller
        .restore(&mut target, &ticket.token, dir.path(), now())
        .unwrap();
    let actual = target.backup_fingerprint().unwrap();
    assert!(controller
        .restore(&mut target, &ticket.token, dir.path(), now())
        .is_err());
    assert_eq!(target.backup_fingerprint().unwrap(), actual);
}

#[test]
fn a_replaced_preview_invalidates_the_previous_token() {
    let bytes = source().export_backup(now()).unwrap();
    let mut target = store(false);
    let mut controller = BackupController::default();
    let dir = tempfile::tempdir().unwrap();
    let old = controller.prepare(&target, bytes.clone(), now()).unwrap();
    let current = controller.prepare(&target, bytes, now()).unwrap();
    assert!(controller
        .restore(&mut target, &old.token, dir.path(), now())
        .is_err());
    controller
        .restore(&mut target, &current.token, dir.path(), now())
        .unwrap();
}

#[test]
fn editing_an_archive_cannot_approve_unreviewed_content() {
    let bytes = source().export_backup(now()).unwrap();
    let forged = modified(&bytes, |a| {
        a.body.data.sessions[0].practice_only = false;
        a.body.data.sessions[0].lesson.editorial_status = "approved".into();
    });
    assert!(store(false)
        .inspect_backup(&forged, now())
        .unwrap_err()
        .contains("reviewed lesson snapshot"));
}
