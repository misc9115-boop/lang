use super::*;
use serde_json::{json, Value};

fn now() -> DateTime<Utc> {
    "2026-10-17T12:00:00Z".parse().unwrap()
}
fn key(id: &str) -> ReviewKey {
    ReviewKey {
        curriculum_id: "test-curriculum".into(),
        target_kind: ReviewTarget::VocabularySense,
        target_id: id.into(),
        skill: Skill::Reading,
    }
}
fn evidence() -> ReviewEvidence {
    ReviewEvidence {
        first_correct: true,
        final_correct: true,
        hinted: false,
        revealed: false,
    }
}
fn observation(n: u128, days: i64) -> ReviewObservation {
    ReviewObservation {
        request_id: Uuid::from_u128(n),
        session_id: Uuid::from_u128(n + 100_000),
        reviewed_at: now() + Duration::days(days),
        evidence: evidence(),
        explicit_easy: false,
    }
}
fn bytes(history: &ReviewHistory) -> Vec<u8> {
    serde_json::to_vec(history).unwrap()
}
fn serialized_schedule(history: &ReviewHistory) -> Value {
    serde_json::to_value(history.schedule().unwrap()).unwrap()
}

#[test]
fn rating_mapping_preserves_correction_hints_reveal_and_explicit_easy() {
    assert_eq!(evidence().rating(false).unwrap(), Rating::Good);
    assert_eq!(evidence().rating(true).unwrap(), Rating::Easy);
    let cases = [
        (
            ReviewEvidence {
                first_correct: false,
                final_correct: false,
                ..evidence()
            },
            Rating::Again,
        ),
        (
            ReviewEvidence {
                revealed: true,
                ..evidence()
            },
            Rating::Again,
        ),
        (
            ReviewEvidence {
                first_correct: false,
                ..evidence()
            },
            Rating::Hard,
        ),
        (
            ReviewEvidence {
                hinted: true,
                ..evidence()
            },
            Rating::Hard,
        ),
        (
            ReviewEvidence {
                first_correct: false,
                hinted: true,
                ..evidence()
            },
            Rating::Hard,
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(input.rating(false).unwrap(), expected);
        assert!(input.rating(true).is_err());
    }
    assert!(ReviewEvidence {
        final_correct: false,
        ..evidence()
    }
    .rating(false)
    .is_err());
}

#[test]
fn published_upstream_initial_states_match_all_four_ratings() {
    // fsrs-rs 6.6.2 inference.rs next_states documentation example.
    let cases = [
        (
            ReviewEvidence {
                first_correct: false,
                final_correct: false,
                ..evidence()
            },
            false,
            Rating::Again,
            0.212,
            6.4133,
            1,
        ),
        (
            ReviewEvidence {
                hinted: true,
                ..evidence()
            },
            false,
            Rating::Hard,
            1.2931,
            5.112_170_7,
            1,
        ),
        (evidence(), false, Rating::Good, 2.3065, 2.118_104, 2),
        (evidence(), true, Rating::Easy, 8.2956, 1.0, 8),
    ];
    for (evidence, explicit_easy, rating, stability, difficulty, interval) in cases {
        let mut history = ReviewHistory::new(key("test-item")).unwrap();
        let result = history
            .record(ReviewObservation {
                evidence,
                explicit_easy,
                ..observation(1, 0)
            })
            .unwrap();
        assert_eq!(result.last_rating, rating);
        assert!((result.stability - stability).abs() < 0.00001);
        assert!((result.difficulty - difficulty).abs() < 0.00001);
        assert_eq!(result.interval_days, interval);
        assert_eq!(result.due_at, now() + Duration::days(interval as i64));
        assert_eq!(result.desired_retention, 0.9);
        assert_eq!(result.scheduler_version, SCHEDULER_VERSION);
        assert_eq!(result.rating_mapping_version, 1);
        assert_eq!(result.repetitions, 1);
    }
}

#[test]
fn persisted_history_replays_after_same_day_review_lapse_and_reopen() {
    let mut history = ReviewHistory::new(key("test-item")).unwrap();
    assert!(history.schedule().unwrap().is_none());
    for (n, day) in [(1, 0), (2, 0), (3, 7), (4, 30), (5, 31)] {
        let mut input = observation(n, day);
        if n == 4 {
            input.evidence = ReviewEvidence {
                first_correct: false,
                final_correct: false,
                ..evidence()
            };
        }
        history.record(input).unwrap();
        let reopened = ReviewHistory::from_json(&bytes(&history)).unwrap();
        assert_eq!(
            serialized_schedule(&history),
            serialized_schedule(&reopened)
        );
        assert_eq!(history.observations(), reopened.observations());
    }
    assert!(!history.observations()[3].evidence.first_correct);
    assert_eq!(history.schedule().unwrap().unwrap().repetitions, 5);
}

#[test]
fn exact_and_old_retries_do_not_repeat_or_roll_back_ratings() {
    let mut history = ReviewHistory::new(key("test-item")).unwrap();
    history.record(observation(1, 0)).unwrap();
    history.record(observation(2, 4)).unwrap();
    let before = bytes(&history);
    let schedule = serialized_schedule(&history);
    let retried = history.record(observation(1, 0)).unwrap();
    assert_eq!(bytes(&history), before);
    assert_eq!(serde_json::to_value(retried).unwrap(), schedule);
    let changed = ReviewObservation {
        explicit_easy: true,
        ..observation(1, 0)
    };
    assert!(history
        .record(changed)
        .unwrap_err()
        .contains("different evidence"));
    assert_eq!(bytes(&history), before);
}

#[test]
fn a_session_cannot_rate_the_same_item_twice_but_can_rate_different_skills() {
    let mut reading = ReviewHistory::new(key("test-item")).unwrap();
    reading.record(observation(1, 0)).unwrap();
    let before = bytes(&reading);
    assert!(reading
        .record(ReviewObservation {
            request_id: Uuid::from_u128(2),
            ..observation(1, 0)
        })
        .is_err());
    assert_eq!(bytes(&reading), before);
    let mut writing_key = key("test-item");
    writing_key.skill = Skill::Writing;
    let mut writing = ReviewHistory::new(writing_key).unwrap();
    writing.record(observation(1, 0)).unwrap();
    assert_ne!(writing.key(), reading.key());
}

#[test]
fn clock_rollback_and_calendar_overflow_leave_history_unchanged() {
    let mut history = ReviewHistory::new(key("test-item")).unwrap();
    history.record(observation(1, 0)).unwrap();
    let before = bytes(&history);
    assert!(history.record(observation(2, -1)).is_err());
    assert!(history
        .record(ReviewObservation {
            reviewed_at: DateTime::<Utc>::MAX_UTC,
            ..observation(2, 0)
        })
        .is_err());
    assert_eq!(bytes(&history), before);
}

#[test]
fn utc_elapsed_days_do_not_depend_on_study_time_zone_or_dst() {
    let mut a = ReviewHistory::new(key("test-item")).unwrap();
    let mut b = a.clone();
    let first = observation(1, 0);
    a.record(first.clone()).unwrap();
    b.record(first).unwrap();
    let mut next_a = observation(2, 1);
    next_a.reviewed_at = "2026-10-18T07:00:00-05:00".parse().unwrap();
    let mut next_b = next_a.clone();
    next_b.reviewed_at = "2026-10-18T14:00:00+02:00".parse().unwrap();
    a.record(next_a).unwrap();
    b.record(next_b).unwrap();
    assert_eq!(serialized_schedule(&a), serialized_schedule(&b));
    // 23 hours counts as zero elapsed whole days, including across DST.
    let prior = ReviewHistory::new(key("other")).unwrap();
    let mut short = prior.clone();
    let mut same = prior;
    short.record(observation(1, 0)).unwrap();
    same.record(observation(1, 0)).unwrap();
    let short_state = short
        .record(ReviewObservation {
            reviewed_at: now() + Duration::hours(23),
            ..observation(2, 0)
        })
        .unwrap();
    let same_state = same.record(observation(2, 0)).unwrap();
    assert_eq!(short_state.stability, same_state.stability);
    assert_eq!(short_state.difficulty, same_state.difficulty);
}

#[test]
fn unknown_versions_fields_and_invalid_evidence_are_rejected_on_load() {
    let mut history = ReviewHistory::new(key("test-item")).unwrap();
    history.record(observation(1, 0)).unwrap();
    let original = serde_json::to_value(&history).unwrap();
    for (field, replacement) in [
        ("formatVersion", json!(2)),
        ("schedulerVersion", json!("future-engine")),
        ("ratingMappingVersion", json!(2)),
        ("stability", json!(999)),
    ] {
        let mut value = original.clone();
        value[field] = replacement;
        assert!(ReviewHistory::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    let mut value = original.clone();
    value["observations"][0]["evidence"]["finalCorrect"] = json!(false);
    assert!(ReviewHistory::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut value = original.clone();
    value["observations"][0]["requestId"] = json!(Uuid::nil());
    assert!(ReviewHistory::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut value = original;
    let duplicate = value["observations"][0].clone();
    value["observations"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(ReviewHistory::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn malformed_or_oversized_history_is_rejected() {
    assert!(ReviewHistory::from_json(b"{}").is_err());
    assert!(ReviewHistory::from_json(&vec![b' '; MAX_HISTORY_BYTES + 1]).is_err());
    let mut history = ReviewHistory::new(key("test-item")).unwrap();
    history.observations = (1..=MAX_HISTORY_RECORDS + 1)
        .map(|n| observation(n as u128, 0))
        .collect();
    assert!(history.schedule().unwrap_err().contains("record limit"));
}

#[test]
fn keys_keep_curricula_target_kinds_and_tested_skills_separate() {
    let base = key("test-item");
    let mut curriculum = base.clone();
    curriculum.curriculum_id = "other-curriculum".into();
    let mut grammar = base.clone();
    grammar.target_kind = ReviewTarget::Grammar;
    let mut writing = base.clone();
    writing.skill = Skill::Writing;
    assert_eq!(
        [base, curriculum, grammar, writing]
            .into_iter()
            .collect::<HashSet<_>>()
            .len(),
        4
    );
    for invalid in ["", "../escape", "UPPERCASE", "é", &"x".repeat(129)] {
        assert!(ReviewHistory::new(key(invalid)).is_err());
    }
}

fn candidate(id: &str, days: Option<i64>) -> ReviewCandidate {
    ReviewCandidate {
        key: key(id),
        due_at: days.map(|d| now() + Duration::days(d)),
    }
}

#[test]
fn selection_prioritizes_due_items_with_stable_ties_and_no_duplicates() {
    let mut candidates = vec![
        candidate("future", Some(3)),
        candidate("new", None),
        candidate("b", Some(-2)),
        candidate("oldest", Some(-4)),
        candidate("a", Some(-2)),
        candidate("today", Some(0)),
    ];
    let selected = select_reviews(&candidates, now(), 3, false).unwrap();
    assert_eq!(
        selected
            .iter()
            .map(|c| c.key.target_id.as_str())
            .collect::<Vec<_>>(),
        ["oldest", "a", "b"]
    );
    candidates.reverse();
    let reversed = select_reviews(&candidates, now(), 3, false).unwrap();
    assert_eq!(
        serde_json::to_value(selected).unwrap(),
        serde_json::to_value(reversed).unwrap()
    );
    let all = select_reviews(&candidates, now(), 10, true).unwrap();
    assert_eq!(
        all.iter()
            .map(|c| c.key.target_id.as_str())
            .collect::<Vec<_>>(),
        ["oldest", "a", "b", "today", "new", "future"]
    );
    candidates.push(candidate("a", None));
    assert!(select_reviews(&candidates, now(), 3, true).is_err());
}

#[test]
fn explicit_practice_remains_available_when_no_items_are_due() {
    let candidates = [candidate("future", Some(3)), candidate("new", None)];
    assert!(select_reviews(&candidates, now(), 10, false)
        .unwrap()
        .is_empty());
    assert_eq!(
        select_reviews(&candidates, now(), 10, true).unwrap().len(),
        2
    );
    assert!(select_reviews(&[], now(), 1, true).unwrap().is_empty());
    assert!(select_reviews(&candidates, now(), 0, true).is_err());
    assert!(select_reviews(&candidates, now(), MAX_CANDIDATES + 1, true).is_err());
    assert!(select_reviews(
        &vec![candidate("same", None); MAX_CANDIDATES + 1],
        now(),
        1,
        true
    )
    .is_err());
}

#[test]
fn candidate_due_date_comes_from_validated_history() {
    let mut history = ReviewHistory::new(key("test-item")).unwrap();
    assert!(history.candidate().unwrap().due_at.is_none());
    let schedule = history.record(observation(1, 0)).unwrap();
    assert_eq!(history.candidate().unwrap().due_at, Some(schedule.due_at));
    history.scheduler_version = "unrecognized".into();
    assert!(history.candidate().is_err());
}

#[test]
fn dst_transitions_use_elapsed_utc_time_instead_of_local_calendar_days() {
    for (first, next, elapsed_days) in [
        ("2026-11-01T00:30:00-05:00", "2026-11-01T23:30:00-06:00", 1),
        ("2026-03-08T00:30:00-06:00", "2026-03-08T23:30:00-05:00", 0),
    ] {
        let mut history = ReviewHistory::new(key("test-item")).unwrap();
        let first_at = first.parse::<DateTime<Utc>>().unwrap();
        history
            .record(ReviewObservation {
                reviewed_at: first_at,
                ..observation(1, 0)
            })
            .unwrap();
        let mut expected = history.clone();
        let result = history
            .record(ReviewObservation {
                reviewed_at: next.parse().unwrap(),
                ..observation(2, 0)
            })
            .unwrap();
        let reference = expected
            .record(ReviewObservation {
                reviewed_at: first_at + Duration::days(elapsed_days),
                ..observation(2, 0)
            })
            .unwrap();
        assert_eq!(result.stability, reference.stability);
        assert_eq!(result.difficulty, reference.difficulty);
        assert_eq!(result.interval_days, reference.interval_days);
    }
}

#[test]
fn full_history_allows_exact_retry_but_rejects_new_ratings_without_mutation() {
    let mut history = ReviewHistory::new(key("test-item")).unwrap();
    history.observations = (1..=MAX_HISTORY_RECORDS)
        .map(|n| observation(n as u128, 0))
        .collect();
    let before = bytes(&history);
    assert!(before.len() < MAX_HISTORY_BYTES);
    let expected = serialized_schedule(&history);
    let retry = history.record(observation(1, 0)).unwrap();
    assert_eq!(serde_json::to_value(retry).unwrap(), expected);
    assert!(history
        .record(observation(MAX_HISTORY_RECORDS as u128 + 1, 1))
        .unwrap_err()
        .contains("full"));
    assert_eq!(bytes(&history), before);
}

#[test]
fn failed_evidence_and_nil_identifiers_do_not_modify_an_existing_history() {
    let mut history = ReviewHistory::new(key("test-item")).unwrap();
    history.record(observation(1, 0)).unwrap();
    let before = bytes(&history);
    for input in [
        ReviewObservation {
            request_id: Uuid::nil(),
            ..observation(2, 1)
        },
        ReviewObservation {
            session_id: Uuid::nil(),
            ..observation(2, 1)
        },
        ReviewObservation {
            explicit_easy: true,
            evidence: ReviewEvidence {
                hinted: true,
                ..evidence()
            },
            ..observation(2, 1)
        },
    ] {
        assert!(history.record(input).is_err());
        assert_eq!(bytes(&history), before);
    }
}
