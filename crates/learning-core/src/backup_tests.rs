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
fn future_versions_modes_and_missing_pack_references_are_rejected() {
    let bytes = source().export_backup(now()).unwrap();
    let target = store(false);
    for bad in [
        modified(&bytes, |a| a.body.version = 99),
        modified(&bytes, |a| a.body.schema_version = 99),
        modified(&bytes, |a| a.body.development = false),
        modified(&bytes, |a| {
            a.body.data.installed_pack_references.push(PackReference {
                pack_id: "future".into(),
                version: 1,
                manifest_sha256: "0".repeat(64),
            })
        }),
    ] {
        assert!(target.inspect_backup(&bad, now()).is_err());
    }
    let release = Store::initialize(Connection::open_in_memory().unwrap(), vec![], false).unwrap();
    assert!(release.inspect_backup(&bytes, now()).is_err());
}

#[test]
fn pack_reference_digest_and_uniqueness_are_checked_even_with_a_valid_archive_checksum() {
    let reference = PackReference {
        pack_id: "dev.synthetic.pack".into(),
        version: 1,
        manifest_sha256: "a".repeat(64),
    };
    let mut source = source();
    source.content.references = vec![reference.clone()];
    let bytes = source.export_backup(now()).unwrap();
    let mut target = store(false);
    target.content.references = vec![reference.clone()];
    assert_eq!(
        target
            .inspect_backup(&bytes, now())
            .unwrap()
            .pack_references,
        vec![reference]
    );
    let before = target.backup_fingerprint().unwrap();
    for bad in [
        modified(&bytes, |a| {
            a.body.data.installed_pack_references[0].manifest_sha256 = "b".repeat(64)
        }),
        modified(&bytes, |a| {
            let reference = a.body.data.installed_pack_references[0].clone();
            a.body.data.installed_pack_references.push(reference);
        }),
    ] {
        assert!(target.inspect_backup(&bad, now()).is_err());
        let root = tempfile::tempdir().unwrap();
        assert!(target
            .restore_backup(&bad, &before, root.path(), now())
            .is_err());
        assert_eq!(target.backup_fingerprint().unwrap(), before);
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
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

fn reviewed_history_source() -> Store {
    // Synthetic host authority for storage tests only. No human review occurred.
    let mut store = store(true);
    let lesson = store.lessons[0].clone();
    let source = crate::review_storage::ReviewSource {
        pack: PackReference {
            pack_id: "synthetic.storage-test".into(),
            version: 1,
            manifest_sha256: "a".repeat(64),
        },
        lesson_id: lesson.id.clone(),
        lesson_sha256: hash(&lesson).unwrap(),
        exercise_id: lesson.exercises[0].id.clone(),
        key: crate::review::ReviewKey {
            curriculum_id: "synthetic.test-curriculum".into(),
            target_kind: crate::review::ReviewTarget::Objective,
            target_id: "synthetic.reading".into(),
            skill: crate::content::Skill::Reading,
        },
    };
    store.content.references.push(source.pack.clone());
    store.content.review_sources.insert(source);
    let session = store.start(&lesson.id, now()).unwrap();
    store
        .submit(
            &session.id,
            &lesson.exercises[0].id,
            Answer::Option("a".into()),
            &Uuid::new_v4().to_string(),
            now(),
        )
        .unwrap();
    store
        .advance(&session.id, &lesson.exercises[0].id, now())
        .unwrap();
    store
}

#[test]
fn checksummed_review_changes_cannot_forge_exercise_authority_or_observations() {
    let store = reviewed_history_source();
    let bytes = store.export_backup(now()).unwrap();
    let changes = [
        ("evidence", serde_json::json!(false)),
        ("easy", serde_json::json!(true)),
        ("request", serde_json::json!(Uuid::new_v4().to_string())),
        (
            "time",
            serde_json::json!((now() + chrono::Duration::days(1)).to_rfc3339()),
        ),
        ("digest", serde_json::json!("b".repeat(64))),
        ("source", serde_json::json!("dev.cafe.reading")),
        ("engine", serde_json::json!("future-engine")),
    ];
    for (change, replacement) in changes {
        let forged = modified(&bytes, |archive| {
            let record = &mut archive.body.data.review_records.as_mut().unwrap()[0];
            let mut value = serde_json::to_value(&record).unwrap();
            match change {
                "evidence" => {
                    value["history"]["observations"][0]["evidence"]["firstCorrect"] = replacement
                }
                "easy" => value["history"]["observations"][0]["explicitEasy"] = replacement,
                "request" => value["history"]["observations"][0]["requestId"] = replacement,
                "time" => value["history"]["observations"][0]["reviewedAt"] = replacement,
                "digest" => value["sources"][0]["pack"]["manifestSha256"] = replacement,
                "source" => value["sources"][0]["exerciseId"] = replacement,
                "engine" => value["history"]["schedulerVersion"] = replacement,
                _ => unreachable!(),
            }
            *record = serde_json::from_value(value).unwrap();
        });
        assert!(
            store.inspect_backup(&forged, now()).is_err(),
            "forgery accepted: {change}"
        );
    }
    let duplicate = modified(&bytes, |archive| {
        let records = archive.body.data.review_records.as_mut().unwrap();
        records.push(records[0].clone());
    });
    assert!(store
        .inspect_backup(&duplicate, now())
        .unwrap_err()
        .contains("Duplicate review"));
}

#[test]
fn legacy_restore_replaces_reviews_and_v2_recovery_can_restore_them_back() {
    let mut store = reviewed_history_source();
    let bytes = store.export_backup(now()).unwrap();
    assert_eq!(
        store
            .inspect_backup(&bytes, now())
            .unwrap()
            .review_item_count,
        1
    );
    let legacy = modified(&bytes, |archive| {
        archive.body.version = 1;
        archive.body.schema_version = 1;
        archive.body.data.review_records = None;
        archive.body.data.review_sessions = None;
        archive.body.data.review_xp = None;
    });
    assert_eq!(
        store
            .inspect_backup(&legacy, now())
            .unwrap()
            .review_item_count,
        0
    );
    let fingerprint = store.backup_fingerprint().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let result = store
        .restore_backup(&legacy, &fingerprint, directory.path(), now())
        .unwrap();
    assert!(store.review_histories().unwrap().is_empty());
    let recovery = read_backup_file(&result.recovery_path).unwrap();
    assert_eq!(parse_archive(&recovery, true).unwrap().body.version, 3);
    assert_eq!(
        store
            .inspect_backup(&recovery, now())
            .unwrap()
            .review_item_count,
        1
    );
    let fingerprint = store.backup_fingerprint().unwrap();
    store
        .restore_backup(&recovery, &fingerprint, directory.path(), now())
        .unwrap();
    assert_eq!(store.review_histories().unwrap().len(), 1);
}

#[test]
fn corrupt_review_rows_and_oversized_payloads_are_not_exported_or_silently_dropped() {
    let mut store = reviewed_history_source();
    let before = store.export_backup(now()).unwrap();
    store
        .connection
        .execute("UPDATE review_histories SET review_key='mismatched'", [])
        .unwrap();
    assert!(store.review_histories().is_err());
    assert!(store.export_backup(now()).is_err());
    store
        .connection
        .execute("DELETE FROM review_histories", [])
        .unwrap();
    let fingerprint = store.backup_fingerprint().unwrap();
    store
        .restore_backup(
            &before,
            &fingerprint,
            tempfile::tempdir().unwrap().path(),
            now(),
        )
        .unwrap();
    store
        .connection
        .execute(
            "UPDATE review_histories SET record=zeroblob(?1)",
            [crate::review_storage::MAX_STORED_REVIEW_BYTES as i64 + 1],
        )
        .unwrap();
    assert!(store.review_histories().is_err());
    assert!(store.export_backup(now()).is_err());
}

#[test]
fn changing_review_history_invalidates_an_existing_restore_preview() {
    let mut store = reviewed_history_source();
    let archive = store.export_backup(now()).unwrap();
    let mut controller = BackupController::default();
    let ticket = controller.prepare(&store, archive, now()).unwrap();
    store
        .connection
        .execute("DELETE FROM review_histories", [])
        .unwrap();
    assert!(controller
        .restore(
            &mut store,
            &ticket.token,
            tempfile::tempdir().unwrap().path(),
            now()
        )
        .unwrap_err()
        .contains("changed after"));
    assert!(store.review_histories().unwrap().is_empty());
}

fn review_ready() -> Store {
    let mut s = reviewed_history_source();
    loop {
        let Some(active) = s.state(now()).unwrap().active_session else {
            break;
        };
        let e = active.exercise.unwrap();
        s.reveal(&active.id, &e.id, now()).unwrap();
        s.advance(&active.id, &e.id, now()).unwrap();
    }
    s
}
fn finish_review(
    s: &mut Store,
    at: DateTime<Utc>,
    easy: bool,
) -> crate::review_session::ReviewSessionView {
    let r = s.start_review(true, at).unwrap();
    let item = r.exercise.unwrap().id;
    s.review_submit(
        &r.id,
        &item,
        Answer::Option("a".into()),
        &Uuid::new_v4().to_string(),
        at,
    )
    .unwrap();
    s.advance_review(&r.id, &item, easy, at).unwrap()
}
#[test]
fn review_completion_daily_credit_practice_and_streak_are_idempotent() {
    use crate::review::Rating;
    let mut s = review_ready();
    assert_eq!(s.review_overview(now()).unwrap().due_count, 0);
    assert_eq!(s.review_overview(now()).unwrap().available_count, 1);
    assert!(s.start_review(false, now()).is_err());
    let r = finish_review(&mut s, now(), true);
    assert_eq!(r.status, "completed");
    assert_eq!(r.xp_earned, 5);
    assert_eq!(s.progress(now()).unwrap().total_xp, 15);
    assert!(s
        .progress(now())
        .unwrap()
        .achievements
        .contains(&"first_review".into()));
    assert_eq!(
        s.review_histories().unwrap()[0]
            .schedule()
            .unwrap()
            .last_rating,
        Rating::Easy
    );
    assert_eq!(
        s.advance_review(&r.id, "old-item", false, now())
            .unwrap()
            .xp_earned,
        5
    );
    assert_eq!(finish_review(&mut s, now(), false).xp_earned, 0);
    assert_eq!(s.progress(now()).unwrap().total_xp, 15);
    for day in 1..7 {
        let at = now() + chrono::Duration::days(day);
        assert_eq!(finish_review(&mut s, at, false).xp_earned, 5);
    }
    let at = now() + chrono::Duration::days(6);
    assert_eq!(s.progress(at).unwrap().current_streak, 7);
    assert!(s
        .progress(at)
        .unwrap()
        .achievements
        .contains(&"7_day_streak".into()));
    assert!(s.export_backup(at).is_ok());
}
#[test]
fn review_correction_hints_reveal_and_explicit_easy_preserve_first_evidence() {
    use crate::review::Rating;
    for mode in ["correction", "hint", "reveal"] {
        let mut s = review_ready();
        let r = s.start_review(true, now()).unwrap();
        let item = r.exercise.unwrap().id;
        assert!(s.advance_review(&r.id, &item, false, now()).is_err());
        if mode == "reveal" {
            s.review_reveal(&r.id, &item, now()).unwrap();
        } else {
            if mode == "hint" {
                s.review_hint(&r.id, &item, now()).unwrap();
            } else {
                s.review_submit(
                    &r.id,
                    &item,
                    Answer::Option("b".into()),
                    &Uuid::new_v4().to_string(),
                    now(),
                )
                .unwrap();
            }
            let request = Uuid::new_v4().to_string();
            s.review_submit(&r.id, &item, Answer::Option("a".into()), &request, now())
                .unwrap();
            s.review_submit(&r.id, &item, Answer::Option("a".into()), &request, now())
                .unwrap();
            assert!(s
                .review_submit(&r.id, &item, Answer::Option("b".into()), &request, now())
                .is_err());
        }
        assert!(!s.review_session(&r.id).unwrap().can_rate_easy);
        let before = s.backup_fingerprint().unwrap();
        assert!(s.advance_review(&r.id, &item, true, now()).is_err());
        assert_eq!(before, s.backup_fingerprint().unwrap());
        s.advance_review(&r.id, &item, false, now()).unwrap();
        let records = s.review_histories().unwrap();
        assert_eq!(
            records[0].schedule().unwrap().last_rating,
            if mode == "reveal" {
                Rating::Again
            } else {
                Rating::Hard
            }
        );
        assert!(s.export_backup(now()).is_ok());
    }
}
#[test]
fn review_resume_from_sqlite_and_backup_keeps_drafts_hints_and_wrong_attempts() {
    let source = review_ready();
    let content = source.content.clone();
    let lessons = source.lessons.clone();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("review.sqlite3");
    let mut s = Store::open(&path, lessons.clone(), true).unwrap();
    s.content = content.clone();
    let fp = s.backup_fingerprint().unwrap();
    s.restore_backup(
        &source.export_backup(now()).unwrap(),
        &fp,
        &root.path().join("recovery"),
        now(),
    )
    .unwrap();
    let r = s.start_review(true, now()).unwrap();
    let item = r.exercise.unwrap().id;
    s.review_submit(
        &r.id,
        &item,
        Answer::Option("b".into()),
        &Uuid::new_v4().to_string(),
        now(),
    )
    .unwrap();
    s.review_hint(&r.id, &item, now()).unwrap();
    s.review_draft(&r.id, &item, Answer::Option("a".into()), now())
        .unwrap();
    let bytes = s.export_backup(now()).unwrap();
    drop(s);
    let mut s = Store::open(&path, lessons, true).unwrap();
    s.content = content;
    let resumed = s.state(now()).unwrap().active_review.unwrap();
    assert_eq!(resumed.id, r.id);
    let e = resumed.exercise.unwrap();
    assert_eq!(e.draft, Some(Answer::Option("a".into())));
    assert!(e.hint.is_some());
    assert!(!e.feedback.unwrap().correct);
    assert_eq!(s.export_backup(now()).unwrap(), bytes);
    let fp = s.backup_fingerprint().unwrap();
    s.abort_review(&r.id, now()).unwrap();
    assert!(s.state(now()).unwrap().active_review.is_none());
    let fp_after = s.backup_fingerprint().unwrap();
    assert_ne!(fp, fp_after);
    s.restore_backup(&bytes, &fp_after, &root.path().join("recovery"), now())
        .unwrap();
    assert_eq!(s.state(now()).unwrap().active_review.unwrap().id, r.id);
}
#[test]
fn unfinished_reviews_award_nothing_and_block_parallel_lessons() {
    let mut s = review_ready();
    let r = s.start_review(true, now()).unwrap();
    let item = r.exercise.unwrap().id;
    assert_eq!(s.start_review(true, now()).unwrap().id, r.id);
    assert!(s.start("dev.a1.cafe", now()).is_err());
    s.review_reveal(&r.id, &item, now()).unwrap();
    s.abort_review(&r.id, now()).unwrap();
    assert_eq!(s.progress(now()).unwrap().total_xp, 10);
    assert!(!s
        .progress(now())
        .unwrap()
        .achievements
        .contains(&"first_review".into()));
    assert_eq!(
        s.review_histories().unwrap()[0]
            .history()
            .observations()
            .len(),
        1
    );
    assert!(s.export_backup(now()).is_ok());
    let lesson = s.start("dev.a1.cafe", now()).unwrap();
    assert!(s.start_review(true, now()).is_err());
    assert_eq!(
        s.state(now()).unwrap().active_session.unwrap().id,
        lesson.id
    );
}
#[test]
fn review_award_failure_rolls_back_rating_completion_and_progress() {
    let mut s = review_ready();
    let r = s.start_review(true, now()).unwrap();
    let item = r.exercise.unwrap().id;
    s.review_reveal(&r.id, &item, now()).unwrap();
    let before = s.export_backup(now()).unwrap();
    s.connection.execute_batch("CREATE TRIGGER fail_review BEFORE INSERT ON review_xp BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    assert!(s.advance_review(&r.id, &item, false, now()).is_err());
    assert_eq!(before, s.export_backup(now()).unwrap());
    s.connection
        .execute_batch("DROP TRIGGER fail_review")
        .unwrap();
    assert_eq!(
        s.advance_review(&r.id, &item, false, now())
            .unwrap()
            .xp_earned,
        5
    );
}
#[test]
fn checksummed_review_session_forgery_is_rejected_before_live_restore() {
    let mut s = review_ready();
    finish_review(&mut s, now(), true);
    let bytes = s.export_backup(now()).unwrap();
    for field in [
        "answer",
        "correct",
        "time",
        "source",
        "position",
        "rating",
        "credit",
        "missing-history",
        "missing-session",
        "duplicate-request",
    ] {
        let forged = modified(&bytes, |a| {
            let session = &mut a.body.data.review_sessions.as_mut().unwrap()[0];
            match field {
                "answer" => session.items[0].attempts[0].answer = Some(Answer::Option("b".into())),
                "correct" => session.items[0].attempts[0].correct = false,
                "time" => session.items[0].attempts[0].at = now() + chrono::Duration::days(1),
                "source" => session.items[0].source.lesson_sha256 = "b".repeat(64),
                "position" => session.position = 0,
                "rating" => session.items[0].observation.as_mut().unwrap().explicit_easy = false,
                "credit" => a.body.data.review_xp.as_mut().unwrap()[0].amount = 10,
                "missing-history" => a.body.data.review_records = Some(vec![]),
                "missing-session" => a.body.data.review_sessions = Some(vec![]),
                "duplicate-request" => {
                    session.items[0].attempts[0].request_id =
                        Uuid::parse_str(&a.body.data.attempts[0].request_id).unwrap()
                }
                _ => unreachable!(),
            }
        });
        let fp = s.backup_fingerprint().unwrap();
        let root = tempfile::tempdir().unwrap();
        assert!(
            s.restore_backup(&forged, &fp, root.path(), now()).is_err(),
            "accepted {field}"
        );
        assert_eq!(fp, s.backup_fingerprint().unwrap());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
#[test]
fn v2_restore_clears_review_sessions_and_v3_recovery_can_restore_them() {
    let mut s = review_ready();
    let base = s.export_backup(now()).unwrap();
    let v2 = modified(&base, |a| {
        a.body.version = 2;
        a.body.schema_version = 2;
        a.body.data.review_sessions = None;
        a.body.data.review_xp = None;
    });
    let r = s.start_review(true, now()).unwrap();
    let item = r.exercise.unwrap().id;
    s.review_hint(&r.id, &item, now()).unwrap();
    let fp = s.backup_fingerprint().unwrap();
    let root = tempfile::tempdir().unwrap();
    let result = s.restore_backup(&v2, &fp, root.path(), now()).unwrap();
    assert!(s.state(now()).unwrap().active_review.is_none());
    let recovery = read_backup_file(&result.recovery_path).unwrap();
    let preview = s.inspect_backup(&recovery, now()).unwrap();
    assert_eq!(preview.review_session_count, 1);
    assert!(preview.active_review);
    let fp = s.backup_fingerprint().unwrap();
    s.restore_backup(&recovery, &fp, root.path(), now())
        .unwrap();
    assert_eq!(s.state(now()).unwrap().active_review.unwrap().id, r.id);
}

#[test]
fn review_credit_uses_saved_zone_and_goal_history_without_rewriting_old_days() {
    let mut s = review_ready();
    finish_review(&mut s, now(), false);
    let mut changed = settings();
    changed.time_zone = "Pacific/Kiritimati".into();
    changed.daily_goal = 7;
    s.save_settings(changed, now()).unwrap();
    finish_review(&mut s, now(), false);
    let p = s.progress(now()).unwrap();
    assert_eq!(p.total_xp, 20);
    assert_eq!(p.today_xp, 5);
    assert_eq!(p.days[0].date, "2026-10-14");
    assert_eq!(p.days[0].goal, 10);
    assert_eq!(p.days[0].time_zone, "America/Chicago");
    assert_eq!(p.days[1].date, "2026-10-15");
    assert_eq!(p.days[1].goal, 7);
    assert_eq!(p.days[1].time_zone, "Pacific/Kiritimati");
    assert!(s.export_backup(now()).is_ok());
}
#[test]
fn mixed_lesson_and_review_credits_grant_100_xp_once() {
    let mut s = review_ready();
    for day in 0..18 {
        assert_eq!(
            finish_review(&mut s, now() + chrono::Duration::days(day), false).xp_earned,
            5
        );
    }
    let at = now() + chrono::Duration::days(17);
    let p = s.progress(at).unwrap();
    assert_eq!(p.total_xp, 100);
    assert_eq!(
        p.achievements
            .iter()
            .filter(|id| id.as_str() == "100_xp")
            .count(),
        1
    );
    assert!(s.export_backup(at).is_ok());
}
#[test]
fn real_v2_review_history_upgrades_with_identical_v2_recovery() {
    let mut s = review_ready();
    s.connection
        .execute_batch("DROP TABLE review_xp; DROP TABLE review_sessions; PRAGMA user_version=2;")
        .unwrap();
    let before = s.export_backup(now()).unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = s.migrate_schema(root.path(), now()).unwrap().unwrap();
    assert_eq!(read_backup_file(&path).unwrap(), before);
    assert_eq!(s.schema_version().unwrap(), 3);
    assert_eq!(s.review_histories().unwrap().len(), 1);
    assert_eq!(s.review_overview(now()).unwrap().available_count, 1);
    assert!(s.export_backup(now()).is_ok());
}
