use crate::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;

fn now(value: &str) -> DateTime<Utc> {
    value.parse().unwrap()
}
fn settings() -> Settings {
    Settings {
        level: "Beginner".into(),
        time_zone: "America/Chicago".into(),
        daily_goal: 10,
        theme: "system".into(),
        reduced_motion: false,
        text_scale: 100,
    }
}
fn at() -> DateTime<Utc> {
    now("2026-10-14T12:00:00Z")
}
fn lesson(approved: bool) -> Lesson {
    let mut value = development_lesson().unwrap();
    // Test-only approval exercises awarding rules, not an editorial approval.
    if approved {
        value.editorial_status = "approved".into();
    }
    value
}
fn store(approved: bool) -> Store {
    let mut store = Store::memory(vec![lesson(approved)]).unwrap();
    store.save_settings(settings(), at()).unwrap();
    store
}
fn submit(store: &mut Store, session: &SessionView, answer: Answer) -> SessionView {
    store
        .submit(
            &session.id,
            &session.exercise.as_ref().unwrap().id,
            answer,
            &Uuid::new_v4().to_string(),
            at(),
        )
        .unwrap()
}
fn finish(store: &mut Store, time: DateTime<Utc>) -> SessionView {
    let mut session = store.start("dev.a1.cafe", time).unwrap();
    while session.status == "active" {
        let exercise_id = session.exercise.as_ref().unwrap().id.clone();
        store.reveal(&session.id, &exercise_id, time).unwrap();
        session = store.advance(&session.id, &exercise_id, time).unwrap();
    }
    session
}

#[test]
fn normalization_preserves_meaningful_accents_and_apostrophes() {
    assert_eq!(normalize("Caffè ! !  "), "caffè");
    assert_eq!(normalize("  CAFFE\u{300}  !  "), "caffè");
    assert_eq!(normalize("L’acqua."), normalize("l'acqua"));
    assert_ne!(normalize("e"), normalize("è"));
    assert_ne!(normalize("lacqua"), normalize("l'acqua"));
    assert_ne!(normalize("pero"), normalize("però"));
}

#[test]
fn draft_saving_accepts_incomplete_whitespace_without_grading_it() {
    let mut store = store(false);
    let session = store.start("dev.a1.cafe", at()).unwrap();
    store.reveal(&session.id, "dev.cafe.choice", at()).unwrap();
    store.advance(&session.id, "dev.cafe.choice", at()).unwrap();
    store
        .save_draft(
            &session.id,
            "dev.cafe.translation",
            Answer::Text("  ".into()),
        )
        .unwrap();
    let restored = store.session(&session.id).unwrap();
    assert_eq!(
        restored.exercise.as_ref().unwrap().draft,
        Some(Answer::Text("  ".into()))
    );
    assert!(restored.exercise.unwrap().feedback.is_none());
    assert!(store
        .submit(
            &session.id,
            "dev.cafe.translation",
            Answer::Text("  ".into()),
            &Uuid::new_v4().to_string(),
            at()
        )
        .is_err());
}

#[test]
fn future_historical_study_date_does_not_hide_current_streak_after_zone_change() {
    let mut store = store(true);
    let mut selected = settings();
    selected.time_zone = "Pacific/Kiritimati".into();
    let time = now("2026-10-14T12:00:00Z"); // October 15 at UTC+14
    store.save_settings(selected.clone(), time).unwrap();
    finish(&mut store, time);
    selected.time_zone = "Pacific/Honolulu".into(); // October 14 at UTC-10
    store.save_settings(selected, time).unwrap();
    finish(&mut store, time);
    let progress = store.progress(time).unwrap();
    assert_eq!(progress.current_streak, 1);
    assert_eq!(progress.days.len(), 2);
    assert_eq!(progress.days[1].date, "2026-10-15");
}

#[test]
fn retries_hints_and_duplicate_submissions_preserve_first_evidence() {
    let mut store = store(false);
    let session = store.start("dev.a1.cafe", at()).unwrap();
    let exercise = session.exercise.as_ref().unwrap().id.clone();
    let request = Uuid::new_v4().to_string();
    let first = store
        .submit(
            &session.id,
            &exercise,
            Answer::Option("b".into()),
            &request,
            at(),
        )
        .unwrap();
    assert!(!first.exercise.unwrap().ready_to_continue);
    store.hint(&session.id, &exercise).unwrap();
    let corrected = submit(&mut store, &session, Answer::Option("a".into()));
    assert_eq!(corrected.first_correct, 0);
    assert_eq!(corrected.independent_correct, 0);
    assert_eq!(corrected.hinted_items, 1);
    assert!(corrected.exercise.unwrap().ready_to_continue);
    let repeated = store
        .submit(
            &session.id,
            &exercise,
            Answer::Option("b".into()),
            &request,
            at(),
        )
        .unwrap();
    assert_eq!(repeated.first_correct, 0);
    let next = store.advance(&session.id, &exercise, at()).unwrap();
    assert_eq!(next.position, 1);
    assert_eq!(
        store
            .advance(&session.id, &exercise, at())
            .unwrap()
            .position,
        1
    );
}

#[test]
fn a_hint_before_correct_answer_is_not_independent_evidence() {
    let mut store = store(false);
    let session = store.start("dev.a1.cafe", at()).unwrap();
    store
        .hint(&session.id, &session.exercise.as_ref().unwrap().id)
        .unwrap();
    let result = submit(&mut store, &session, Answer::Option("a".into()));
    assert_eq!(result.first_correct, 1);
    assert_eq!(result.independent_correct, 0);
}

#[test]
fn development_fixture_cannot_award_progress() {
    let mut store = store(false);
    let session = finish(&mut store, at());
    assert!(session.practice_only);
    assert_eq!(session.xp_earned, 0);
    let progress = store.progress(at()).unwrap();
    assert_eq!(progress.total_xp, 0);
    assert_eq!(progress.completed_lessons, 0);
    assert!(progress.days.is_empty());
    assert!(progress.achievements.is_empty());
}

#[test]
fn daily_xp_caps_and_completion_are_transactional_and_idempotent() {
    let mut store = store(true);
    let first = finish(&mut store, at());
    assert_eq!(first.xp_earned, 10);
    let duplicate = store.advance(&first.id, "dev.cafe.reading", at()).unwrap();
    assert_eq!(duplicate.xp_earned, 10);
    assert_eq!(finish(&mut store, at()).xp_earned, 0);
    let next_day = now("2026-10-15T12:00:00Z");
    assert_eq!(finish(&mut store, next_day).xp_earned, 10);
    let progress = store.progress(next_day).unwrap();
    assert_eq!(progress.total_xp, 20);
    assert_eq!(progress.today_xp, 10);
    assert_eq!(progress.completed_lessons, 1);
    assert_eq!(progress.current_streak, 2);
    assert_eq!(progress.achievements, vec!["first_lesson"]);
}

#[test]
fn study_zone_handles_midnight_dst_and_historical_goal_changes() {
    let mut store = store(true);
    finish(&mut store, now("2026-11-01T05:30:00Z")); // 00:30 CDT
    assert_eq!(finish(&mut store, now("2026-11-01T07:30:00Z")).xp_earned, 0); // 01:30 CST
    finish(&mut store, now("2026-11-02T06:01:00Z")); // 00:01 CST
    let mut changed = settings();
    changed.daily_goal = 25;
    store
        .save_settings(changed.clone(), now("2026-11-02T07:00:00Z"))
        .unwrap();
    let progress = store.progress(now("2026-11-02T07:00:00Z")).unwrap();
    assert_eq!(progress.days[0].goal, 10);
    assert_eq!(progress.days[1].goal, 25);
    changed.time_zone = "Asia/Tokyo".into();
    store
        .save_settings(changed, now("2026-11-03T07:00:00Z"))
        .unwrap();
    assert_eq!(
        store.progress(now("2026-11-03T07:00:00Z")).unwrap().days[0].time_zone,
        "America/Chicago"
    );
}

#[test]
fn gaps_restart_streak_and_grants_are_not_duplicated() {
    let mut store = store(true);
    for day in 1..=10 {
        finish(&mut store, now(&format!("2026-10-{day:02}T12:00:00Z")));
    }
    let progress = store.progress(now("2026-10-10T12:00:00Z")).unwrap();
    assert_eq!(progress.current_streak, 10);
    assert_eq!(
        progress.achievements,
        vec!["100_xp", "7_day_streak", "first_lesson"]
    );
    assert_eq!(
        store
            .progress(now("2026-10-12T12:00:00Z"))
            .unwrap()
            .current_streak,
        0
    );
    finish(&mut store, now("2026-10-13T12:00:00Z"));
    let progress = store.progress(now("2026-10-13T12:00:00Z")).unwrap();
    assert_eq!(progress.current_streak, 1);
    assert_eq!(progress.best_streak, 10);
    assert_eq!(progress.achievements.len(), 3);
}

#[test]
fn reopen_preserves_current_position_draft_and_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("learner.db");
    let id;
    {
        let mut store = Store::open(&path, vec![lesson(false)], true).unwrap();
        store.save_settings(settings(), at()).unwrap();
        let session = store.start("dev.a1.cafe", at()).unwrap();
        id = session.id.clone();
        submit(&mut store, &session, Answer::Option("b".into()));
        store.reveal(&id, "dev.cafe.choice", at()).unwrap();
        store.advance(&id, "dev.cafe.choice", at()).unwrap();
        store
            .save_draft(&id, "dev.cafe.translation", Answer::Text("Un ca".into()))
            .unwrap();
    }
    let store = Store::open(&path, vec![lesson(false)], true).unwrap();
    let session = store.state(at()).unwrap().active_session.unwrap();
    assert_eq!(session.id, id);
    assert_eq!(session.position, 1);
    assert_eq!(session.first_correct, 0);
    assert_eq!(session.revealed_items, 1);
    assert_eq!(
        session.exercise.unwrap().draft,
        Some(Answer::Text("Un ca".into()))
    );
}

#[test]
fn invalid_input_cannot_advance_mutate_settings_or_earn_credit() {
    let mut store = store(true);
    let session = store.start("dev.a1.cafe", at()).unwrap();
    assert!(store.advance(&session.id, "dev.cafe.choice", at()).is_err());
    assert!(store
        .submit(
            &session.id,
            "dev.cafe.choice",
            Answer::Option("unknown".into()),
            &Uuid::new_v4().to_string(),
            at()
        )
        .is_err());
    let mut changed = settings();
    changed.daily_goal = 0;
    assert!(store.save_settings(changed, at()).is_err());
    assert_eq!(store.settings().unwrap().unwrap().daily_goal, 10);
    assert_eq!(store.progress(at()).unwrap().total_xp, 0);
}

#[test]
fn duplicate_word_tiles_require_distinct_identifiers() {
    let mut exercise = lesson(false).exercises[2].clone();
    exercise.options = vec![
        OptionItem {
            id: "a".into(),
            text: "piano".into(),
        },
        OptionItem {
            id: "b".into(),
            text: "piano".into(),
        },
    ];
    exercise.accepted = vec!["piano piano".into()];
    assert!(
        crate::scoring::evaluate(&exercise, &Answer::Tiles(vec!["a".into(), "b".into()])).unwrap()
    );
    assert!(
        crate::scoring::evaluate(&exercise, &Answer::Tiles(vec!["a".into(), "a".into()])).is_err()
    );
}

#[test]
fn unknown_database_version_is_refused_without_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("future.db");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("CREATE TABLE future_data(value TEXT); INSERT INTO future_data VALUES('preserve me'); PRAGMA user_version=99;").unwrap();
    drop(connection);
    assert!(Store::open(&path, vec![], false).is_err());
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT value FROM future_data", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "preserve me"
    );
}

#[test]
fn failed_award_rolls_back_completion_day_and_achievement_together() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("atomic.db");
    let mut store = Store::open(&path, vec![lesson(true)], true).unwrap();
    store.save_settings(settings(), at()).unwrap();
    let mut session = store.start("dev.a1.cafe", at()).unwrap();
    while session.position < session.lesson.exercise_count - 1 {
        let exercise = session.exercise.as_ref().unwrap().id.clone();
        store.reveal(&session.id, &exercise, at()).unwrap();
        session = store.advance(&session.id, &exercise, at()).unwrap();
    }
    let exercise = session.exercise.as_ref().unwrap().id.clone();
    store.reveal(&session.id, &exercise, at()).unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_credit BEFORE INSERT ON xp_ledger BEGIN SELECT RAISE(ABORT, 'injected disk-write failure'); END;").unwrap();
    assert!(store.advance(&session.id, &exercise, at()).is_err());
    assert_eq!(store.session(&session.id).unwrap().status, "active");
    let progress = store.progress(at()).unwrap();
    assert_eq!(progress.total_xp, 0);
    assert_eq!(progress.completed_lessons, 0);
    assert!(progress.days.is_empty());
    assert!(progress.achievements.is_empty());
    connection
        .execute_batch("DROP TRIGGER reject_credit")
        .unwrap();
    assert_eq!(
        store
            .advance(&session.id, &exercise, at())
            .unwrap()
            .xp_earned,
        10
    );
}

#[test]
fn content_updates_do_not_change_a_resumable_lesson_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("snapshot.db");
    {
        let mut store = Store::open(&path, vec![lesson(false)], true).unwrap();
        store.save_settings(settings(), at()).unwrap();
        store.start("dev.a1.cafe", at()).unwrap();
    }
    let mut update = lesson(false);
    update.version = 2;
    update.title = "Changed test lesson".into();
    update.exercises[0].prompt = "Updated test prompt".into();
    let store = Store::open(&path, vec![update], true).unwrap();
    let state = store.state(at()).unwrap();
    assert_eq!(state.lessons[0].title, "Changed test lesson");
    let active = state.active_session.unwrap();
    assert_eq!(active.lesson.title, "At the café");
    assert_eq!(active.exercise.unwrap().prompt, "Un caffè, per favore.");
}
