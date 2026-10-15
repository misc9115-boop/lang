use super::*;

fn fixture() -> tempfile::TempDir {
    let root = tempfile::Builder::new()
        .prefix("pacchetto caffè ")
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
fn manifest(root: &Path) -> PackManifest {
    serde_json::from_slice(&fs::read(root.join("pack.json")).unwrap()).unwrap()
}
fn write_manifest(root: &Path, value: &PackManifest) {
    fs::write(
        root.join("pack.json"),
        serde_json::to_vec_pretty(value).unwrap(),
    )
    .unwrap();
}
fn update(root: &Path, change: impl FnOnce(&mut PackManifest)) {
    let mut value = manifest(root);
    change(&mut value);
    write_manifest(root, &value);
}
fn lesson(root: &Path) -> Lesson {
    serde_json::from_slice(&fs::read(root.join("cafe.json")).unwrap()).unwrap()
}
fn write_lesson(root: &Path, value: &Lesson) {
    let bytes = serde_json::to_vec_pretty(value).unwrap();
    fs::write(root.join("cafe.json"), &bytes).unwrap();
    update(root, |m| {
        m.lessons[0].file.bytes = bytes.len() as u64;
        m.lessons[0].file.sha256 = sha256(&bytes);
    });
}
fn check(root: &Path) -> Result<ValidatedPack> {
    validate_pack(root, &ContentTrust::default())
}
fn reviewed_fixture(root: &Path) -> ContentTrust {
    let mut value = lesson(root);
    value.editorial_status = "approved".into();
    write_lesson(root, &value);
    update(root, |m| {
        m.scope = PackScope::Course;
        m.editorial_status = EditorialStatus::Approved;
        m.lessons[0].estimated_minutes = Some(7); // Synthetic test estimate, not measured pacing.
        m.reviews = vec![EditorialReview {
            reviewer: "Synthetic test record — no human review occurred".into(),
            reviewed_at: "2026-10-15T12:00:00Z".parse().unwrap(),
            areas: vec![
                ReviewArea::Linguistic,
                ReviewArea::Pedagogical,
                ReviewArea::AnswerVariants,
                ReviewArea::Licensing,
                ReviewArea::Pacing,
            ],
        }];
    });
    ContentTrust {
        reviewed_manifest_sha256: HashSet::from([sha256(
            &fs::read(root.join("pack.json")).unwrap(),
        )]),
    }
}

#[test]
fn fixture_reports_draft_coverage_gaps_without_mutating_files() {
    let root = fixture();
    let before = fs::read(root.path().join("pack.json")).unwrap();
    let pack = check(root.path()).unwrap();
    assert_eq!(pack.lessons.len(), 1);
    assert_eq!(pack.report.coverage.len(), 48);
    assert!(!pack.report.trusted_review);
    assert!(pack
        .report
        .coverage
        .iter()
        .all(|c| c.reviewed_objectives == 0));
    assert!(pack
        .report
        .coverage
        .iter()
        .filter(|c| c.level != "A1")
        .all(|c| c.required_objectives == 0));
    assert!(pack
        .report
        .coverage
        .iter()
        .any(|c| c.strand == Strand::Listening && c.represented_objectives == 0));
    assert_eq!(pack.report.unmeasured_pacing, vec!["dev.a1.cafe"]);
    assert_eq!(
        pack.report.missing_translation_directions,
        vec![TranslationDirection::ItalianToEnglish]
    );
    assert_eq!(before, fs::read(root.path().join("pack.json")).unwrap());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}

#[test]
fn changed_truncated_and_missing_payloads_fail_integrity_checks() {
    for mode in 0..3 {
        let root = fixture();
        let path = root.path().join("cafe.json");
        match mode {
            0 => {
                let mut bytes = fs::read(&path).unwrap();
                bytes[20] ^= 1;
                fs::write(&path, bytes).unwrap();
            }
            1 => fs::write(&path, b"{}").unwrap(),
            _ => fs::remove_file(&path).unwrap(),
        }
        assert!(check(root.path()).is_err());
    }
}

#[test]
fn unsupported_versions_are_rejected() {
    let variants: [fn(&mut PackManifest); 7] = [
        |m| m.format_version = 2,
        |m| m.lesson_schema_version = 2,
        |m| m.curriculum.schema_version = 2,
        |m| m.min_reader_version = 2,
        |m| m.max_reader_version = 0,
        |m| m.version = 0,
        |m| m.curriculum.version = 0,
    ];
    for mutate in variants {
        let root = fixture();
        update(root.path(), mutate);
        assert!(check(root.path()).is_err());
    }
}

#[test]
fn unknown_fields_are_not_silently_dropped() {
    let root = fixture();
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(root.path().join("pack.json")).unwrap()).unwrap();
    value["mediaFiles"] = serde_json::json!([]);
    fs::write(
        root.path().join("pack.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    assert!(check(root.path()).is_err());
    let root = fixture();
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(root.path().join("cafe.json")).unwrap()).unwrap();
    value["runtimeScript"] = serde_json::json!("never execute");
    let bytes = serde_json::to_vec(&value).unwrap();
    fs::write(root.path().join("cafe.json"), &bytes).unwrap();
    update(root.path(), |m| {
        m.lessons[0].file.bytes = bytes.len() as u64;
        m.lessons[0].file.sha256 = sha256(&bytes);
    });
    assert!(check(root.path()).is_err());
}

#[test]
fn portable_paths_reject_traversal_absolute_devices_and_windows_aliases() {
    for path in [
        "../cafe.json",
        "/cafe.json",
        "C:/cafe.json",
        "cafe\\file.json",
        "a//b.json",
        "./cafe.json",
        "folder/../cafe.json",
        "CON.json",
        "aux.txt",
        "folder/LPT9.json",
        "cafe.json.",
        "cafe .json",
        "cafe.json:stream",
        "café.json",
    ] {
        let root = fixture();
        update(root.path(), |m| m.lessons[0].file.path = path.into());
        assert!(check(root.path()).is_err(), "accepted unsafe path {path}");
    }
    assert!(portable_path("lessons/a1-cafe.v1.json").is_ok());
}

#[test]
fn case_collisions_and_duplicate_unit_membership_are_rejected() {
    let root = fixture();
    update(root.path(), |m| {
        let mut duplicate = m.lessons[0].clone();
        duplicate.lesson_id = "other.lesson".into();
        duplicate.file.path = "CAFE.JSON".into();
        m.curriculum.units[0]
            .lesson_ids
            .push(duplicate.lesson_id.clone());
        m.lessons.push(duplicate);
    });
    assert!(check(root.path()).err().unwrap().contains("case-colliding"));
    let root = fixture();
    update(root.path(), |m| {
        m.curriculum.units[0]
            .lesson_ids
            .push(m.lessons[0].lesson_id.clone())
    });
    assert!(check(root.path()).is_err());
}

#[test]
fn dangling_objectives_levels_targets_and_unit_references_are_rejected() {
    let variants: [fn(&mut PackManifest); 8] = [
        |m| m.lessons[0].objective_ids[0] = "unknown.objective".into(),
        |m| m.curriculum.objectives[0].level = "B1".into(),
        |m| m.curriculum.units[0].level = "Beginner".into(),
        |m| m.lessons[0].vocabulary_sense_ids[0] = "unknown.sense".into(),
        |m| {
            m.lessons[0]
                .grammar_target_ids
                .push(m.curriculum.targets[0].id.clone())
        },
        |m| m.lessons[0].unit_id = "missing.unit".into(),
        |m| {
            m.curriculum.units[0]
                .lesson_ids
                .push("missing.lesson".into())
        },
        |m| {
            m.lessons[0]
                .recommended_prior_knowledge
                .push("missing.objective".into())
        },
    ];
    for mutate in variants {
        let root = fixture();
        update(root.path(), mutate);
        assert!(check(root.path()).is_err());
    }
}

#[test]
fn prerequisite_cycles_and_unknown_nodes_fail_without_recursive_traversal() {
    let root = fixture();
    update(root.path(), |m| {
        let prior = m.curriculum.objectives[1].id.clone();
        m.curriculum.objectives[0]
            .recommended_prerequisites
            .push(prior)
    });
    assert!(check(root.path()).err().unwrap().contains("cycle"));
    let root = fixture();
    update(root.path(), |m| {
        m.curriculum.objectives[0]
            .recommended_prerequisites
            .push("unknown".into())
    });
    assert!(check(root.path()).is_err());
}

#[test]
fn missing_evidence_is_reported_without_inventing_course_coverage() {
    let root = fixture();
    update(root.path(), |m| {
        m.curriculum.objectives.push(Objective {
            id: "unrepresented.listening".into(),
            level: "C2".into(),
            description: "Synthetic unmet objective, not authored course content".into(),
            strands: vec![Strand::Listening],
            skills: vec![Skill::Listening],
            required: true,
            recommended_prerequisites: vec![],
            provenance: None,
        });
    });
    let pack = check(root.path()).unwrap();
    assert_eq!(pack.report.missing_required_evidence.len(), 1);
    let cell = pack
        .report
        .coverage
        .iter()
        .find(|c| c.level == "C2" && c.strand == Strand::Listening)
        .unwrap();
    assert_eq!(
        (
            cell.required_objectives,
            cell.represented_objectives,
            cell.reviewed_objectives
        ),
        (1, 0, 0)
    );
}

#[test]
fn written_exercises_cannot_claim_listening_or_speaking_evidence() {
    for skill in [Skill::Listening, Skill::Speaking] {
        let root = fixture();
        update(root.path(), |m| {
            m.curriculum.objectives[0].skills.push(skill);
            m.lessons[0].exercises[0].objectives[0].tested_skills = vec![skill];
        });
        assert!(check(root.path()).err().unwrap().contains("cannot supply"));
    }
}

#[test]
fn translation_directions_and_italian_evidence_are_explicit() {
    let root = fixture();
    update(root.path(), |m| {
        m.lessons[0].exercises[1].translation_direction = None
    });
    assert!(check(root.path()).is_err());
    let root = fixture();
    update(root.path(), |m| {
        m.lessons[0].exercises[0].translation_direction =
            Some(TranslationDirection::ItalianToEnglish)
    });
    assert!(check(root.path()).is_err());
    let root = fixture();
    update(root.path(), |m| {
        m.lessons[0].exercises[1].translation_direction =
            Some(TranslationDirection::ItalianToEnglish)
    });
    assert!(check(root.path()).err().unwrap().contains("cannot supply"));
}

#[test]
fn every_exercise_requires_exactly_one_mapping_and_stable_identity() {
    let root = fixture();
    update(root.path(), |m| {
        m.lessons[0].exercises.pop();
    });
    assert!(check(root.path()).is_err());
    let root = fixture();
    update(root.path(), |m| {
        m.lessons[0].exercises[0].exercise_id = "unknown.exercise".into();
    });
    assert!(check(root.path()).is_err());
    let root = fixture();
    update(root.path(), |m| {
        let duplicate = m.lessons[0].exercises[0].clone();
        m.lessons[0].exercises.push(duplicate);
    });
    assert!(check(root.path()).is_err());
    let root = fixture();
    let mut value = lesson(root.path());
    value.id = "changed.id".into();
    write_lesson(root.path(), &value);
    assert!(check(root.path()).is_err());
}

#[test]
fn approved_labels_and_self_reported_reviews_are_not_trusted() {
    let root = fixture();
    let trust = reviewed_fixture(root.path());
    assert!(check(root.path())
        .err()
        .unwrap()
        .contains("not editorial authority"));
    let pack = validate_pack(root.path(), &trust).unwrap();
    assert!(pack.report.trusted_review);
    update(root.path(), |m| m.title.push_str(" edited after review"));
    assert!(validate_pack(root.path(), &trust).is_err());
}

#[test]
fn trusted_digest_does_not_replace_complete_review_records() {
    let root = fixture();
    reviewed_fixture(root.path());
    update(root.path(), |m| {
        m.reviews[0].areas.pop();
    });
    let trust = ContentTrust {
        reviewed_manifest_sha256: HashSet::from([sha256(
            &fs::read(root.path().join("pack.json")).unwrap(),
        )]),
    };
    assert!(validate_pack(root.path(), &trust)
        .err()
        .unwrap()
        .contains("review area"));
}

#[test]
fn draft_pack_cannot_smuggle_approved_lessons_and_external_sources_need_citations() {
    let root = fixture();
    let mut value = lesson(root.path());
    value.editorial_status = "approved".into();
    write_lesson(root.path(), &value);
    assert!(check(root.path()).is_err());
    let root = fixture();
    update(root.path(), |m| {
        m.lessons[0].provenance.origin = Origin::External
    });
    assert!(check(root.path()).is_err());
}

#[test]
fn input_bounds_fail_before_loading_large_payloads() {
    let root = fixture();
    update(root.path(), |m| {
        m.lessons[0].file.bytes = MAX_LESSON_BYTES + 1
    });
    assert!(check(root.path()).is_err());
    let root = fixture();
    fs::write(
        root.path().join("pack.json"),
        vec![b' '; MAX_MANIFEST_BYTES as usize + 1],
    )
    .unwrap();
    assert!(check(root.path()).is_err());
}

#[test]
fn pacing_target_is_reported_without_becoming_a_hard_lesson_time_limit() {
    let root = fixture();
    update(root.path(), |m| m.lessons[0].estimated_minutes = Some(12));
    assert_eq!(
        check(root.path()).unwrap().report.pacing_outside_target,
        vec!["dev.a1.cafe"]
    );
    let root = fixture();
    update(root.path(), |m| m.lessons[0].estimated_minutes = Some(0));
    assert!(check(root.path()).is_err());
}

#[test]
fn language_contract_and_empty_assessment_controls_are_rejected() {
    let root = fixture();
    update(root.path(), |m| m.language = "fr".into());
    assert!(check(root.path()).is_err());
    let root = fixture();
    update(root.path(), |m| m.instruction_language = "it".into());
    assert!(check(root.path()).is_err());
    let root = fixture();
    let mut value = lesson(root.path());
    value.exercises[0].hint.clear();
    write_lesson(root.path(), &value);
    assert!(check(root.path()).is_err());
    let root = fixture();
    let mut value = lesson(root.path());
    value.exercises[0].options[1].text.clear();
    write_lesson(root.path(), &value);
    assert!(check(root.path()).is_err());
}

#[test]
fn item_specific_external_material_requires_its_own_citation() {
    let root = fixture();
    update(root.path(), |m| {
        m.lessons[0].exercises[0].provenance = Some(Provenance {
            origin: Origin::External,
            license: "Synthetic test license".into(),
            attribution: "Synthetic test source".into(),
            source: None,
        })
    });
    assert!(check(root.path())
        .err()
        .unwrap()
        .contains("source citation"));
    update(root.path(), |m| {
        m.lessons[0].exercises[0]
            .provenance
            .as_mut()
            .unwrap()
            .source =
            Some("Synthetic citation for schema validation; not external source material".into())
    });
    assert!(check(root.path()).is_ok());
}

#[test]
fn declared_unit_order_is_used_and_cross_lesson_exercise_ids_cannot_collide() {
    let root = fixture();
    let mut second = lesson(root.path());
    second.id = "dev.a1.cafe.second".into();
    for exercise in &mut second.exercises {
        exercise.id.push_str(".second");
    }
    let bytes = serde_json::to_vec(&second).unwrap();
    fs::write(root.path().join("second.json"), &bytes).unwrap();
    update(root.path(), |m| {
        let mut record = m.lessons[0].clone();
        record.lesson_id = second.id.clone();
        record.file = FileRecord {
            path: "second.json".into(),
            bytes: bytes.len() as u64,
            sha256: sha256(&bytes),
        };
        for mapping in &mut record.exercises {
            mapping.exercise_id.push_str(".second");
        }
        m.curriculum.units[0]
            .lesson_ids
            .insert(0, second.id.clone());
        m.lessons.push(record);
    });
    assert_eq!(check(root.path()).unwrap().lessons[0].id, second.id);
    second.exercises[0].id = "dev.cafe.choice".into();
    let bytes = serde_json::to_vec(&second).unwrap();
    fs::write(root.path().join("second.json"), &bytes).unwrap();
    update(root.path(), |m| {
        m.lessons[1].file.bytes = bytes.len() as u64;
        m.lessons[1].file.sha256 = sha256(&bytes);
        m.lessons[1].exercises[0].exercise_id = second.exercises[0].id.clone();
    });
    assert!(check(root.path())
        .err()
        .unwrap()
        .contains("unique across the pack"));
}

#[cfg(unix)]
#[test]
fn symlinked_payloads_and_directories_are_rejected() {
    use std::os::unix::fs::symlink;
    let root = fixture();
    let outside = tempfile::tempdir().unwrap();
    fs::write(
        outside.path().join("cafe.json"),
        fs::read(root.path().join("cafe.json")).unwrap(),
    )
    .unwrap();
    fs::remove_file(root.path().join("cafe.json")).unwrap();
    symlink(
        outside.path().join("cafe.json"),
        root.path().join("cafe.json"),
    )
    .unwrap();
    assert!(check(root.path()).is_err());
    symlink(outside.path(), root.path().join("linked")).unwrap();
    update(root.path(), |m| {
        m.lessons[0].file.path = "linked/cafe.json".into()
    });
    assert!(check(root.path()).is_err());
}
