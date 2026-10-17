use super::*;
use crate::{content::PackManifest, Answer, Settings, Store};
use std::fs;

const PACK: &str = "dev.pack.cafe";
fn fixture() -> tempfile::TempDir {
    let root = tempfile::Builder::new()
        .prefix("caffè library ")
        .tempdir()
        .unwrap();
    fs::write(
        root.path().join("pack.json"),
        include_bytes!("../../../../content/development/pack.json"),
    )
    .unwrap();
    fs::write(
        root.path().join("cafe.json"),
        include_bytes!("../../../../content/development/cafe.json"),
    )
    .unwrap();
    root
}
fn library(root: &Path) -> PackLibrary {
    PackLibrary::open(
        &root.join("packs.sqlite3"),
        ContentTrust::default(),
        LibraryMode::Development,
    )
    .unwrap()
}
fn manifest(root: &Path) -> PackManifest {
    serde_json::from_slice(&fs::read(root.join("pack.json")).unwrap()).unwrap()
}
fn update(root: &Path, change: impl FnOnce(&mut PackManifest)) {
    let mut m = manifest(root);
    change(&mut m);
    fs::write(
        root.join("pack.json"),
        serde_json::to_vec_pretty(&m).unwrap(),
    )
    .unwrap();
}
fn update_lesson(root: &Path, version: u32) {
    let mut lesson: Lesson =
        serde_json::from_slice(&fs::read(root.join("cafe.json")).unwrap()).unwrap();
    lesson.version = version;
    lesson.title = format!("Synthetic updated lesson v{version}");
    let bytes = serde_json::to_vec_pretty(&lesson).unwrap();
    fs::write(root.join("cafe.json"), &bytes).unwrap();
    update(root, |m| {
        m.lessons[0].file.bytes = bytes.len() as u64;
        m.lessons[0].file.sha256 = hash(&bytes);
    });
}
fn install_active(lib: &mut PackLibrary, source: &Path) {
    let pack = lib.install_directory(source).unwrap();
    lib.activate(&pack.pack_id, pack.version).unwrap();
}

#[test]
fn install_is_offline_and_explicit_activation_survives_reopen_without_source() {
    let source = fixture();
    fs::write(source.path().join("unlisted-secret"), "not content").unwrap();
    let root = fixture();
    let mut lib = library(root.path());
    let entry = lib.install_directory(source.path()).unwrap();
    assert!(entry.stored);
    assert!(!entry.active);
    assert!(lib.active_lessons().unwrap().is_empty());
    lib.activate(PACK, 1).unwrap();
    assert_eq!(
        lib.connection
            .query_row("SELECT count(*) FROM pack_files", [], |r| r
                .get::<_, u32>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        lib.verify(PACK, 1).unwrap().manifest_bytes,
        fs::read(source.path().join("pack.json")).unwrap()
    );
    drop(source);
    drop(lib);
    let lib = library(root.path());
    assert!(lib.entries().unwrap()[0].active);
    assert_eq!(lib.active_lessons().unwrap()[0].id, "dev.a1.cafe");
}

#[test]
fn validated_bytes_are_installed_without_reopening_changed_source() {
    let source = fixture();
    let root = fixture();
    let mut lib = library(root.path());
    let pack = content::validate_pack(source.path(), &ContentTrust::default()).unwrap();
    let expected = pack.files["cafe.json"].clone();
    fs::write(source.path().join("cafe.json"), "corrupt after preflight").unwrap();
    fs::write(source.path().join("pack.json"), "corrupt after preflight").unwrap();
    lib.install_verified(pack).unwrap();
    assert_eq!(lib.verify(PACK, 1).unwrap().files["cafe.json"], expected);
}

#[test]
fn update_preserves_old_active_until_explicit_switch_and_allows_rollback() {
    let source = fixture();
    let root = fixture();
    let mut lib = library(root.path());
    install_active(&mut lib, source.path());
    update(source.path(), |m| m.version = 2);
    update_lesson(source.path(), 2);
    lib.install_directory(source.path()).unwrap();
    assert_eq!(lib.active_lessons().unwrap()[0].version, 1);
    lib.activate(PACK, 2).unwrap();
    assert_eq!(lib.active_lessons().unwrap()[0].version, 2);
    lib.activate(PACK, 1).unwrap();
    assert_eq!(lib.active_lessons().unwrap()[0].version, 1);
    assert_eq!(lib.entries().unwrap().len(), 2);
}

#[test]
fn corrupt_input_and_reused_pack_or_lesson_versions_leave_active_unchanged() {
    let source = fixture();
    let root = fixture();
    let mut lib = library(root.path());
    install_active(&mut lib, source.path());
    update(source.path(), |m| m.title.push_str(" changed"));
    assert!(lib
        .install_directory(source.path())
        .unwrap_err()
        .contains("different bytes"));
    update(source.path(), |m| m.version = 2);
    update_lesson(source.path(), 1);
    assert!(lib
        .install_directory(source.path())
        .unwrap_err()
        .contains("without increasing"));
    fs::write(source.path().join("cafe.json"), "truncated").unwrap();
    assert!(lib.install_directory(source.path()).is_err());
    assert_eq!(lib.entries().unwrap().len(), 1);
    assert_eq!(lib.active_lessons().unwrap()[0].version, 1);
}

#[test]
fn curriculum_changes_also_require_version_bumps() {
    let source = fixture();
    let root = fixture();
    let mut lib = library(root.path());
    install_active(&mut lib, source.path());
    update(source.path(), |m| {
        m.version = 2;
        m.curriculum.objectives[0].description.push_str(" changed");
    });
    assert!(lib
        .install_directory(source.path())
        .unwrap_err()
        .contains("curriculum"));
    update(source.path(), |m| m.curriculum.version = 2);
    lib.install_directory(source.path()).unwrap();
}

#[test]
fn removed_payloads_keep_history_and_exact_reinstall_repairs_them() {
    let source = fixture();
    let root = fixture();
    let mut lib = library(root.path());
    install_active(&mut lib, source.path());
    assert!(lib.remove_version(PACK, 1).is_err());
    lib.deactivate(PACK).unwrap();
    lib.remove_version(PACK, 1).unwrap();
    assert!(!lib.entries().unwrap()[0].stored);
    assert!(lib.activate(PACK, 1).is_err());
    assert_eq!(
        lib.connection
            .query_row("SELECT count(*) FROM pack_files", [], |r| r
                .get::<_, u32>(0))
            .unwrap(),
        0
    );
    update(source.path(), |m| m.title.push_str(" changed"));
    assert!(lib.install_directory(source.path()).is_err());
    let original = fixture();
    lib.install_directory(original.path()).unwrap();
    lib.activate(PACK, 1).unwrap();
    assert_eq!(lib.active_lessons().unwrap().len(), 1);
    assert!(lib.remove_version("missing", 1).is_err());
}

#[test]
fn corruption_is_reported_then_exact_reinstall_repairs_manifest_and_payloads() {
    let source = fixture();
    let root = fixture();
    let mut lib = library(root.path());
    install_active(&mut lib, source.path());
    lib.connection
        .execute("UPDATE pack_files SET bytes=x'00'", [])
        .unwrap();
    assert!(lib.active_lessons().is_err());
    lib.install_directory(source.path()).unwrap();
    assert_eq!(lib.active_lessons().unwrap().len(), 1);
    lib.connection
        .execute("DELETE FROM pack_files", [])
        .unwrap();
    assert!(lib.verify(PACK, 1).is_err());
    lib.install_directory(source.path()).unwrap();
    lib.connection
        .execute("UPDATE pack_versions SET manifest=x'00'", [])
        .unwrap();
    assert!(lib.verify(PACK, 1).is_err());
    lib.install_directory(source.path()).unwrap();
    assert_eq!(lib.active_lessons().unwrap().len(), 1);
}

#[test]
fn failed_multirow_install_and_failed_activation_roll_back() {
    let source = fixture();
    let root = fixture();
    let mut lib = library(root.path());
    install_active(&mut lib, source.path());
    update(source.path(), |m| m.version = 2);
    update_lesson(source.path(), 2);
    lib.connection.execute_batch("CREATE TRIGGER fail_files BEFORE INSERT ON pack_files BEGIN SELECT RAISE(ABORT,'injected disk write failure'); END;").unwrap();
    assert!(lib.install_directory(source.path()).is_err());
    assert_eq!(lib.entries().unwrap().len(), 1);
    assert_eq!(
        lib.connection
            .query_row(
                "SELECT max(version) FROM identities WHERE kind='lesson'",
                [],
                |r| r.get::<_, u32>(0)
            )
            .unwrap(),
        1
    );
    lib.connection
        .execute_batch("DROP TRIGGER fail_files;")
        .unwrap();
    lib.install_directory(source.path()).unwrap();
    lib.connection.execute_batch("CREATE TRIGGER fail_activation BEFORE UPDATE ON active_packs BEGIN SELECT RAISE(ABORT,'injected activation failure'); END;").unwrap();
    assert!(lib.activate(PACK, 2).is_err());
    assert_eq!(lib.active_lessons().unwrap()[0].version, 1);
}

#[test]
fn active_pack_id_conflicts_do_not_replace_current_course() {
    let source = fixture();
    let root = fixture();
    let mut lib = library(root.path());
    install_active(&mut lib, source.path());
    update(source.path(), |m| m.id = "dev.other.pack".into());
    lib.install_directory(source.path()).unwrap();
    assert!(lib
        .activate("dev.other.pack", 1)
        .unwrap_err()
        .contains("conflict"));
    assert_eq!(lib.active_lessons().unwrap().len(), 1);
    lib.deactivate(PACK).unwrap();
    lib.activate("dev.other.pack", 1).unwrap();
    assert_eq!(lib.active_lessons().unwrap().len(), 1);
}

#[test]
fn reviewed_only_mode_rejects_drafts_including_previously_cached_ones() {
    let source = fixture();
    let root = fixture();
    let mut production = PackLibrary::open(
        &root.path().join("packs.sqlite3"),
        ContentTrust::default(),
        LibraryMode::ReviewedOnly,
    )
    .unwrap();
    assert!(production.install_directory(source.path()).is_err());
    drop(production);
    let mut dev = library(root.path());
    install_active(&mut dev, source.path());
    drop(dev);
    let mut production = PackLibrary::open(
        &root.path().join("packs.sqlite3"),
        ContentTrust::default(),
        LibraryMode::ReviewedOnly,
    )
    .unwrap();
    assert!(production.active_lessons().is_err());
    assert!(production.activate(PACK, 1).is_err());
    assert!(production.verify(PACK, 1).is_err());
}

#[test]
fn forged_approval_is_rejected_even_in_development_mode() {
    let source = fixture();
    let root = fixture();
    let mut lib = library(root.path());
    update(source.path(), |m| {
        m.editorial_status = EditorialStatus::Approved
    });
    assert!(lib
        .install_directory(source.path())
        .unwrap_err()
        .contains("editorial authority"));
    assert!(lib.entries().unwrap().is_empty());
}

#[test]
fn unknown_catalogue_schema_and_learner_database_are_preserved() {
    let root = fixture();
    let path = root.path().join("unknown.sqlite3");
    let db = Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE marker(value TEXT); INSERT INTO marker VALUES('preserve'); PRAGMA user_version=99;").unwrap();
    assert!(PackLibrary::open(&path, ContentTrust::default(), LibraryMode::Development).is_err());
    assert_eq!(
        db.query_row("SELECT value FROM marker", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "preserve"
    );
    db.execute_batch("PRAGMA user_version=0;").unwrap();
    assert!(PackLibrary::open(&path, ContentTrust::default(), LibraryMode::Development).is_err());
    let learner = root.path().join("learner.sqlite3");
    let store = Store::open(&learner, vec![], true).unwrap();
    assert!(
        PackLibrary::open(&learner, ContentTrust::default(), LibraryMode::Development).is_err()
    );
    assert!(store.settings().unwrap().is_none());
}

#[test]
fn existing_sessions_and_backups_survive_pack_changes_and_removal() {
    let source = fixture();
    let root = fixture();
    let mut lib = library(root.path());
    install_active(&mut lib, source.path());
    let learner = root.path().join("learner.sqlite3");
    let now = Utc::now();
    let mut store = Store::open(&learner, lib.active_lessons().unwrap(), true).unwrap();
    store
        .save_settings(
            Settings {
                level: "A1".into(),
                time_zone: "America/Chicago".into(),
                daily_goal: 10,
                theme: "light".into(),
                reduced_motion: false,
                text_scale: 100,
            },
            now,
        )
        .unwrap();
    let session = store.start("dev.a1.cafe", now).unwrap();
    let original_title = session.lesson.title.clone();
    let exercise = session.exercise.unwrap().id;
    store
        .save_draft(&session.id, &exercise, Answer::Option("a".into()))
        .unwrap();
    let before = store.export_backup(now).unwrap();
    update(source.path(), |m| m.version = 2);
    update_lesson(source.path(), 2);
    lib.install_directory(source.path()).unwrap();
    lib.activate(PACK, 2).unwrap();
    drop(store);
    let store = Store::open(&learner, lib.active_lessons().unwrap(), true).unwrap();
    assert_eq!(
        store.session(&session.id).unwrap().lesson.title,
        original_title
    );
    assert_eq!(store.export_backup(now).unwrap(), before);
    lib.deactivate(PACK).unwrap();
    lib.remove_version(PACK, 1).unwrap();
    lib.remove_version(PACK, 2).unwrap();
    drop(store);
    let store = Store::open(&learner, lib.active_lessons().unwrap(), true).unwrap();
    assert_eq!(
        store.session(&session.id).unwrap().exercise.unwrap().draft,
        Some(Answer::Option("a".into()))
    );
    assert_eq!(store.state(now).unwrap().progress.total_xp, 0);
}

// The child exits without unwinding, leaving SQLite's journal recovery to reopen.
#[test]
#[ignore = "subprocess helper invoked by interrupted_writer_recovers_previous_active_version"]
fn interrupted_writer_child() {
    let Some(path) = std::env::var_os("LANG_PACK_CRASH_TEST_DB") else {
        return;
    };
    let db = Connection::open(path).unwrap();
    db.execute_batch("PRAGMA synchronous=FULL; BEGIN IMMEDIATE; DELETE FROM active_packs; UPDATE pack_files SET bytes=zeroblob(2000000);").unwrap();
    std::process::exit(17);
}

#[test]
fn interrupted_writer_recovers_previous_active_version() {
    let source = fixture();
    let root = fixture();
    let mut lib = library(root.path());
    install_active(&mut lib, source.path());
    drop(lib);
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "pack_library::tests::interrupted_writer_child",
            "--ignored",
        ])
        .env("LANG_PACK_CRASH_TEST_DB", root.path().join("packs.sqlite3"))
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(17));
    let lib = library(root.path());
    assert!(lib.entries().unwrap()[0].active);
    assert_eq!(lib.active_lessons().unwrap()[0].version, 1);
}

#[test]
fn historical_version_rules_survive_removal_and_downgrade_attempts() {
    let source = fixture();
    let root = fixture();
    let mut lib = library(root.path());
    update(source.path(), |m| m.version = 3);
    update_lesson(source.path(), 3);
    lib.install_directory(source.path()).unwrap();
    lib.remove_version(PACK, 3).unwrap();
    update(source.path(), |m| m.version = 2);
    assert!(lib
        .install_directory(source.path())
        .unwrap_err()
        .contains("recorded history"));
    update(source.path(), |m| m.version = 4);
    update_lesson(source.path(), 2);
    assert!(lib
        .install_directory(source.path())
        .unwrap_err()
        .contains("regresses"));
    assert_eq!(lib.entries().unwrap().len(), 1);
}

#[test]
fn catalogue_bounds_fail_before_loading_oversized_stored_bytes() {
    let source = fixture();
    let root = fixture();
    let mut lib = library(root.path());
    install_active(&mut lib, source.path());
    lib.connection
        .execute("UPDATE pack_versions SET manifest=zeroblob(1048577)", [])
        .unwrap();
    assert!(lib.verify(PACK, 1).err().unwrap().contains("limit"));
    lib.install_directory(source.path()).unwrap();
    lib.connection
        .execute("UPDATE pack_files SET bytes=zeroblob(8388609)", [])
        .unwrap();
    assert!(lib.verify(PACK, 1).err().unwrap().contains("declared size"));
}

#[test]
fn second_writer_cannot_redefine_a_version_prepared_before_first_writer_commits() {
    let source = fixture();
    let root = fixture();
    let mut first = library(root.path());
    let prepared = content::validate_pack(source.path(), &ContentTrust::default()).unwrap();
    let mut second = library(root.path());
    update(source.path(), |m| m.title.push_str(" changed"));
    second.install_directory(source.path()).unwrap();
    assert!(first
        .install_verified(prepared)
        .unwrap_err()
        .contains("different bytes"));
    assert_eq!(second.entries().unwrap().len(), 1);
}

#[test]
fn approval_requires_current_external_trust_each_time_content_is_loaded() {
    use crate::content::{EditorialReview, PackScope, ReviewArea};
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
        m.lessons[0].file.bytes = bytes.len() as u64;
        m.lessons[0].file.sha256 = hash(&bytes);
        m.lessons[0].estimated_minutes = Some(7); // Synthetic test estimate only.
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
    let trust = ContentTrust {
        reviewed_manifest_sha256: HashSet::from([hash(
            &fs::read(source.path().join("pack.json")).unwrap(),
        )]),
    };
    let mut lib = PackLibrary::open(
        &root.path().join("packs.sqlite3"),
        trust,
        LibraryMode::ReviewedOnly,
    )
    .unwrap();
    install_active(&mut lib, source.path());
    assert_eq!(
        lib.active_lessons().unwrap()[0].editorial_status,
        "approved"
    );
    drop(lib);
    let mut revoked = PackLibrary::open(
        &root.path().join("packs.sqlite3"),
        ContentTrust::default(),
        LibraryMode::ReviewedOnly,
    )
    .unwrap();
    assert!(revoked.active_lessons().is_err());
    assert!(revoked.activate(PACK, 1).is_err());
    assert!(revoked.install_directory(source.path()).is_err());
    assert!(revoked.entries().unwrap()[0].active); // Pointer is retained, never mislabelled as trusted.
}
