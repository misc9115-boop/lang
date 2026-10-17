use super::*;

fn now() -> DateTime<Utc> {
    "2026-10-17T12:00:00Z".parse().unwrap()
}
fn lessons() -> Vec<Lesson> {
    vec![development_lesson().unwrap()]
}
fn legacy(path: &Path) -> Store {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(include_str!("schema.sql"))
        .unwrap();
    let mut store = Store::initialize(connection, lessons(), true).unwrap();
    store
        .save_settings(
            Settings {
                level: "A1".into(),
                time_zone: "America/Chicago".into(),
                daily_goal: 10,
                theme: "dark".into(),
                reduced_motion: true,
                text_scale: 100,
            },
            now(),
        )
        .unwrap();
    let session = store.start("dev.a1.cafe", now()).unwrap();
    store
        .save_draft(&session.id, "dev.cafe.choice", Answer::Option("a".into()))
        .unwrap();
    store
}
fn table_exists(store: &Store) -> bool {
    store
        .connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='review_histories')",
            [],
            |r| r.get(0),
        )
        .unwrap()
}

#[test]
fn migration_preserves_legacy_history_and_verified_recovery_across_reopen() {
    let root = tempfile::Builder::new()
        .prefix("migration caffè ")
        .tempdir()
        .unwrap();
    let path = root.path().join("learner.sqlite3");
    let mut store = legacy(&path);
    let old = store.export_backup(now()).unwrap();
    assert_eq!(parse_archive(&old, true).unwrap().body.version, 1);
    assert!(!String::from_utf8(old.clone())
        .unwrap()
        .contains("reviewRecords"));
    let recovery = store
        .migrate_schema(&root.path().join("recovery"), now())
        .unwrap()
        .unwrap();
    assert_eq!(read_backup_file(&recovery).unwrap(), old);
    assert_eq!(store.schema_version().unwrap(), 2);
    assert!(table_exists(&store));
    assert!(store.review_histories().unwrap().is_empty());
    let upgraded = store.export_backup(now()).unwrap();
    let archive = parse_archive(&upgraded, true).unwrap();
    assert_eq!(archive.body.version, 2);
    assert_eq!(archive.body.data.review_records.unwrap().len(), 0);
    assert!(store.inspect_backup(&old, now()).is_ok());
    assert!(store
        .migrate_schema(&root.path().join("recovery"), now())
        .unwrap()
        .is_none());
    drop(store);
    let mut reopened = Store::open(&path, lessons(), true).unwrap();
    assert_eq!(reopened.export_backup(now()).unwrap(), upgraded);
    assert!(reopened
        .migrate_schema(&root.path().join("recovery"), now())
        .unwrap()
        .is_none());
    assert_eq!(
        fs::read_dir(root.path().join("recovery")).unwrap().count(),
        1
    );
    let fingerprint = reopened.backup_fingerprint().unwrap();
    reopened
        .restore_backup(
            &old,
            &fingerprint,
            &root.path().join("restore-recovery"),
            now(),
        )
        .unwrap();
    assert_eq!(reopened.state(now()).unwrap().settings.unwrap().level, "A1");
    assert_eq!(
        reopened
            .state(now())
            .unwrap()
            .active_session
            .unwrap()
            .exercise
            .unwrap()
            .draft,
        Some(Answer::Option("a".into()))
    );
}

#[test]
fn failed_recovery_write_prevents_schema_change() {
    let root = tempfile::tempdir().unwrap();
    let mut store = legacy(&root.path().join("learner.sqlite3"));
    let before = store.export_backup(now()).unwrap();
    let blocked = root.path().join("blocked");
    fs::write(&blocked, b"keep this file").unwrap();
    assert!(store.migrate_schema(&blocked, now()).is_err());
    assert_eq!(store.schema_version().unwrap(), 1);
    assert!(!table_exists(&store));
    assert_eq!(store.export_backup(now()).unwrap(), before);
    assert_eq!(fs::read(blocked).unwrap(), b"keep this file");
}

#[test]
fn failed_ddl_and_post_ddl_failure_roll_back_with_valid_prior_recovery() {
    for ddl_conflict in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut store = legacy(&root.path().join("learner.sqlite3"));
        if ddl_conflict {
            store.connection.execute_batch("CREATE TABLE review_histories(external TEXT); INSERT INTO review_histories VALUES('preserve me');").unwrap();
        }
        let before = store.export_backup(now()).unwrap();
        let directory = root.path().join("recovery");
        assert!(store
            .migrate_schema_with(&directory, now(), |_| Err("injected failure".into()))
            .is_err());
        assert_eq!(store.schema_version().unwrap(), 1);
        assert_eq!(store.export_backup(now()).unwrap(), before);
        let path = fs::read_dir(directory)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(read_backup_file(&path).unwrap(), before);
        if ddl_conflict {
            assert_eq!(
                store
                    .connection
                    .query_row("SELECT external FROM review_histories", [], |r| r
                        .get::<_, String>(0))
                    .unwrap(),
                "preserve me"
            );
        } else {
            assert!(!table_exists(&store));
        }
    }
}

#[test]
fn invalid_legacy_state_is_not_migrated_or_called_a_backup() {
    let root = tempfile::tempdir().unwrap();
    let mut store = legacy(&root.path().join("learner.sqlite3"));
    store
        .connection
        .execute("UPDATE profile SET settings='{}'", [])
        .unwrap();
    let directory = root.path().join("recovery");
    assert!(store.migrate_schema(&directory, now()).is_err());
    assert_eq!(store.schema_version().unwrap(), 1);
    assert!(!directory.exists());
    assert_eq!(
        store
            .connection
            .query_row("SELECT settings FROM profile", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "{}"
    );
}

#[test]
fn migration_write_reservation_blocks_another_writer_until_commit() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("learner.sqlite3");
    let mut store = legacy(&path);
    let before = store.export_backup(now()).unwrap();
    let other = Connection::open(&path).unwrap();
    other.busy_timeout(std::time::Duration::ZERO).unwrap();
    let recovery = store
        .migrate_schema_with(&root.path().join("recovery"), now(), |_| {
            assert!(other.execute("DELETE FROM profile", []).is_err());
            Ok(())
        })
        .unwrap()
        .unwrap();
    assert_eq!(read_backup_file(&recovery).unwrap(), before);
    assert_eq!(store.state(now()).unwrap().settings.unwrap().level, "A1");
}

#[test]
#[ignore = "subprocess helper invoked by interrupted_migration_preserves_v1_and_recovery"]
fn interrupted_migration_child() {
    let Some(path) = std::env::var_os("LANG_MIGRATION_CRASH_DB") else {
        return;
    };
    let path = PathBuf::from(path);
    let mut store = Store::open(&path, lessons(), true).unwrap();
    store
        .migrate_schema_with(&path.parent().unwrap().join("recovery"), now(), |_| {
            std::process::exit(23)
        })
        .unwrap();
}

#[test]
fn interrupted_migration_preserves_v1_and_recovery() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("learner.sqlite3");
    let store = legacy(&path);
    let before = store.export_backup(now()).unwrap();
    drop(store);
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "backup::migration_tests::interrupted_migration_child",
            "--ignored",
        ])
        .env("LANG_MIGRATION_CRASH_DB", &path)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(23));
    let mut reopened = Store::open(&path, lessons(), true).unwrap();
    assert_eq!(reopened.schema_version().unwrap(), 1);
    assert!(!table_exists(&reopened));
    assert_eq!(reopened.export_backup(now()).unwrap(), before);
    let recovery = fs::read_dir(root.path().join("recovery"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(read_backup_file(&recovery).unwrap(), before);
    assert!(reopened
        .migrate_schema(&root.path().join("recovery"), now())
        .unwrap()
        .is_some());
    assert_eq!(reopened.schema_version().unwrap(), 2);
}

#[test]
fn schema_changes_after_open_and_future_archive_pairs_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let mut store = legacy(&root.path().join("learner.sqlite3"));
    let old = store.export_backup(now()).unwrap();
    store
        .connection
        .execute_batch("PRAGMA user_version=99")
        .unwrap();
    assert!(store
        .migrate_schema(&root.path().join("recovery"), now())
        .is_err());
    assert!(!root.path().join("recovery").exists());
    let target = Store::memory(lessons()).unwrap();
    for (version, schema, reviews) in [
        (1, 2, None),
        (2, 1, None),
        (2, 2, None),
        (1, 1, Some(vec![])),
    ] {
        let mut archive: Archive = serde_json::from_slice(&old).unwrap();
        archive.body.version = version;
        archive.body.schema_version = schema;
        archive.body.data.review_records = reviews;
        archive.sha256 = hash(&archive.body).unwrap();
        assert!(target
            .inspect_backup(&serde_json::to_vec(&archive).unwrap(), now())
            .is_err());
    }
}
