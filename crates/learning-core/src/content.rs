//! Offline authoring preflight. Validation does not install packs or award credit.
use crate::{store::validate_lesson, ExerciseKind, Lesson};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_LESSON_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PACK_BYTES: u64 = 64 * 1024 * 1024;
const MAX_LESSONS: usize = 1000;
const READER_VERSION: u32 = 1;
const LEVELS: [&str; 6] = ["A1", "A2", "B1", "B2", "C1", "C2"];
type Result<T> = std::result::Result<T, String>;

macro_rules! contract_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}
contract_enum!(Strand {
    Vocabulary,
    Grammar,
    Reading,
    Writing,
    Listening,
    Speaking,
    CulturalContext,
    Dialogues
});
contract_enum!(Skill {
    Reading,
    Writing,
    Listening,
    Speaking
});
contract_enum!(EditorialStatus { Draft, Approved });
contract_enum!(PackScope {
    DevelopmentFixture,
    Course
});
contract_enum!(Origin {
    OriginalAiDraft,
    OriginalHuman,
    External
});
contract_enum!(TargetKind {
    VocabularySense,
    Grammar
});
contract_enum!(TranslationDirection {
    ItalianToEnglish,
    EnglishToItalian
});
contract_enum!(ReviewArea {
    Linguistic,
    Pedagogical,
    AnswerVariants,
    Licensing,
    Pacing
});

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Provenance {
    pub origin: Origin,
    pub license: String,
    pub attribution: String,
    pub source: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorialReview {
    pub reviewer: String,
    pub reviewed_at: DateTime<Utc>,
    pub areas: Vec<ReviewArea>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Objective {
    pub id: String,
    pub level: String,
    pub description: String,
    pub strands: Vec<Strand>,
    pub skills: Vec<Skill>,
    pub required: bool,
    pub recommended_prerequisites: Vec<String>,
    pub provenance: Option<Provenance>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Target {
    pub id: String,
    pub kind: TargetKind,
    pub description: String,
    pub provenance: Option<Provenance>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Unit {
    pub id: String,
    pub level: String,
    pub title: String,
    pub lesson_ids: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObjectiveManifest {
    pub schema_version: u32,
    pub id: String,
    pub version: u32,
    pub objectives: Vec<Objective>,
    pub targets: Vec<Target>,
    pub units: Vec<Unit>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileRecord {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObjectiveEvidence {
    pub objective_id: String,
    pub tested_skills: Vec<Skill>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExerciseMapping {
    pub exercise_id: String,
    pub objectives: Vec<ObjectiveEvidence>,
    pub translation_direction: Option<TranslationDirection>,
    pub provenance: Option<Provenance>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LessonRecord {
    pub lesson_id: String,
    pub unit_id: String,
    pub file: FileRecord,
    pub provenance: Provenance,
    pub objective_ids: Vec<String>,
    pub vocabulary_sense_ids: Vec<String>,
    pub grammar_target_ids: Vec<String>,
    pub recommended_prior_knowledge: Vec<String>,
    pub estimated_minutes: Option<u16>,
    pub exercises: Vec<ExerciseMapping>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackManifest {
    pub format: String,
    pub language: String,
    pub instruction_language: String,
    pub format_version: u32,
    pub lesson_schema_version: u32,
    pub min_reader_version: u32,
    pub max_reader_version: u32,
    pub id: String,
    pub version: u32,
    pub title: String,
    pub scope: PackScope,
    pub editorial_status: EditorialStatus,
    pub provenance: Provenance,
    pub reviews: Vec<EditorialReview>,
    pub curriculum: ObjectiveManifest,
    pub lessons: Vec<LessonRecord>,
}

/// Supplied by a trusted host after real editorial review, never by pack data.
#[derive(Default)]
pub struct ContentTrust {
    pub reviewed_manifest_sha256: HashSet<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageCell {
    pub level: String,
    pub strand: Strand,
    pub required_objectives: usize,
    pub represented_objectives: usize,
    pub reviewed_objectives: usize,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingEvidence {
    pub objective_id: String,
    pub skills: Vec<Skill>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackReport {
    pub pack_id: String,
    pub version: u32,
    pub manifest_sha256: String,
    pub editorial_status: EditorialStatus,
    pub trusted_review: bool,
    pub lesson_count: usize,
    pub payload_bytes: u64,
    pub coverage: Vec<CoverageCell>,
    pub missing_required_evidence: Vec<MissingEvidence>,
    pub missing_translation_directions: Vec<TranslationDirection>,
    pub unmeasured_pacing: Vec<String>,
    pub pacing_outside_target: Vec<String>,
}
pub struct ValidatedPack {
    pub manifest: PackManifest,
    pub lessons: Vec<Lesson>,
    pub report: PackReport,
}

fn text(value: &str, name: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > 4000 {
        return Err(format!(
            "{name} must contain 1–4000 bytes of nonblank text."
        ));
    }
    Ok(())
}
fn id(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        || !value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
    {
        return Err(format!("Invalid stable content ID: {value:?}"));
    }
    Ok(())
}
fn unique<T: Eq + std::hash::Hash>(values: &[T], name: &str) -> Result<()> {
    if values.iter().collect::<HashSet<_>>().len() != values.len() {
        return Err(format!("Duplicate {name}."));
    }
    Ok(())
}
fn level(value: &str) -> Result<()> {
    if !LEVELS.contains(&value) {
        return Err("Curriculum levels must be A1–C2; Beginner is an entry route into A1.".into());
    }
    Ok(())
}
fn provenance(value: &Provenance) -> Result<()> {
    text(&value.license, "License")?;
    text(&value.attribution, "Attribution")?;
    if let Some(source) = &value.source {
        text(source, "Source citation")?;
    }
    if value.origin == Origin::External && value.source.is_none() {
        return Err("External material requires a source citation.".into());
    }
    Ok(())
}
fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn portable_path(value: &str) -> Result<PathBuf> {
    if value.is_empty() || value.len() > 240 {
        return Err("A pack path is empty or too long.".into());
    }
    for segment in value.split('/') {
        let stem = segment.split('.').next().unwrap_or("").to_ascii_uppercase();
        let device = ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && (b'1'..=b'9').contains(&stem.as_bytes()[3]));
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.ends_with('.')
            || device
            || !segment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err(format!(
                "Pack path must be a portable relative path: {value:?}"
            ));
        }
    }
    Ok(value.split('/').collect())
}
fn file_path(root: &Path, name: &str) -> Result<PathBuf> {
    let relative = portable_path(name)?;
    let mut selected = root.to_path_buf();
    for segment in relative.components() {
        selected.push(segment);
        let metadata = fs::symlink_metadata(&selected).map_err(|e| format!("{name}: {e}"))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "Pack files and folders cannot be symbolic links: {name}"
            ));
        }
    }
    let canonical = selected
        .canonicalize()
        .map_err(|e| format!("{name}: {e}"))?;
    if !canonical.starts_with(root) {
        return Err(format!("Pack file is outside the selected folder: {name}"));
    }
    Ok(canonical)
}
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    // Reject devices/FIFOs before opening: opening a FIFO can itself block.
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err("Pack payload must be a regular file within the reader's size limit.".into());
    }
    let file = File::open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err("Pack payload must be a regular file within the reader's size limit.".into());
    }
    let mut bytes = vec![];
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("Pack payload exceeds the reader's size limit.".into());
    }
    Ok(bytes)
}

/// Checks a directory read-only. Only the exact bytes verified here are parsed.
/// It is not an installer or a defence against concurrent hostile filesystem edits.
pub fn validate_pack(root: &Path, trust: &ContentTrust) -> Result<ValidatedPack> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let manifest_bytes = read_bounded(&file_path(&root, "pack.json")?, MAX_MANIFEST_BYTES)?;
    let manifest: PackManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| format!("Invalid pack manifest: {e}"))?;
    let digest = sha256(&manifest_bytes);
    if manifest.language != "it" || manifest.instruction_language != "en" {
        return Err("This reader supports Italian content with English instructions.".into());
    }
    if manifest.format != "lang-curriculum-pack"
        || manifest.format_version != 1
        || manifest.lesson_schema_version != 1
        || manifest.curriculum.schema_version != 1
        || manifest.min_reader_version > READER_VERSION
        || manifest.max_reader_version < READER_VERSION
        || manifest.min_reader_version == 0
        || manifest.min_reader_version > manifest.max_reader_version
    {
        return Err("Unsupported pack, objective, lesson or reader version.".into());
    }
    id(&manifest.id)?;
    id(&manifest.curriculum.id)?;
    text(&manifest.title, "Pack title")?;
    provenance(&manifest.provenance)?;
    if manifest.version == 0
        || manifest.curriculum.version == 0
        || manifest.lessons.is_empty()
        || manifest.lessons.len() > MAX_LESSONS
    {
        return Err("Pack versions must be positive and contain 1–1000 lessons.".into());
    }
    let trusted_review = manifest.editorial_status == EditorialStatus::Approved;
    for review in &manifest.reviews {
        text(&review.reviewer, "Reviewer attribution")?;
        if review.areas.is_empty() {
            return Err("Review records need explicit review areas.".into());
        }
        unique(&review.areas, "review areas")?;
    }
    if trusted_review {
        if manifest.scope != PackScope::Course || !trust.reviewed_manifest_sha256.contains(&digest)
        {
            return Err("An approval label is not editorial authority. This exact manifest requires trusted external review provenance.".into());
        }
        let reviewed = manifest
            .reviews
            .iter()
            .flat_map(|r| r.areas.iter())
            .collect::<HashSet<_>>();
        for area in [
            ReviewArea::Linguistic,
            ReviewArea::Pedagogical,
            ReviewArea::AnswerVariants,
            ReviewArea::Licensing,
            ReviewArea::Pacing,
        ] {
            if !reviewed.contains(&area) {
                return Err("Approved content is missing a required editorial review area.".into());
            }
        }
    }
    let curriculum = &manifest.curriculum;
    let mut objectives = HashMap::new();
    for objective in &curriculum.objectives {
        id(&objective.id)?;
        level(&objective.level)?;
        text(&objective.description, "Objective description")?;
        if let Some(source) = &objective.provenance {
            provenance(source)?;
        }
        if objective.strands.is_empty()
            || objective.skills.is_empty()
            || objectives
                .insert(objective.id.as_str(), objective)
                .is_some()
        {
            return Err("Objectives need unique IDs, strands and tested skills.".into());
        }
        unique(&objective.strands, "objective strands")?;
        unique(&objective.skills, "objective skills")?;
        for (strand, skill) in [
            (Strand::Reading, Skill::Reading),
            (Strand::Writing, Skill::Writing),
            (Strand::Listening, Skill::Listening),
            (Strand::Speaking, Skill::Speaking),
        ] {
            if objective.strands.contains(&strand) && !objective.skills.contains(&skill) {
                return Err(
                    "A language-skill strand must require its corresponding tested skill.".into(),
                );
            }
        }
    }
    if objectives.is_empty() {
        return Err("The objective manifest is empty.".into());
    }
    validate_prerequisites(&objectives)?;
    let mut targets = HashMap::new();
    for target in &curriculum.targets {
        id(&target.id)?;
        text(&target.description, "Target description")?;
        if let Some(source) = &target.provenance {
            provenance(source)?;
        }
        if targets.insert(target.id.as_str(), target.kind).is_some() {
            return Err("Duplicate vocabulary/grammar target ID.".into());
        }
    }
    let mut units = HashMap::new();
    let mut membership = HashSet::new();
    for unit in &curriculum.units {
        id(&unit.id)?;
        level(&unit.level)?;
        text(&unit.title, "Unit title")?;
        if unit.lesson_ids.is_empty() || units.insert(unit.id.as_str(), unit).is_some() {
            return Err("Units need unique IDs and at least one lesson.".into());
        }
        for lesson in &unit.lesson_ids {
            if !membership.insert(lesson.as_str()) {
                return Err("A lesson occurs more than once in unit ordering.".into());
            }
        }
    }
    let mut files = HashSet::from(["pack.json".to_string()]);
    let mut lesson_ids = HashSet::new();
    let mut exercise_ids = HashSet::new();
    let mut evidence: BTreeMap<&str, BTreeSet<Skill>> = BTreeMap::new();
    let mut directions = BTreeSet::new();
    let mut lessons = vec![];
    let mut payload_bytes = 0u64;
    let mut unmeasured_pacing = vec![];
    let mut pacing_outside_target = vec![];
    for record in &manifest.lessons {
        id(&record.lesson_id)?;
        if !lesson_ids.insert(record.lesson_id.as_str()) {
            return Err("Duplicate lesson ID.".into());
        }
        let unit = units
            .get(record.unit_id.as_str())
            .ok_or("Lesson references an unknown unit.")?;
        if !unit.lesson_ids.contains(&record.lesson_id) {
            return Err("Lesson and unit ordering do not agree.".into());
        }
        portable_path(&record.file.path)?;
        if !files.insert(record.file.path.to_ascii_lowercase()) {
            return Err("Duplicate or case-colliding pack file path.".into());
        }
        payload_bytes = payload_bytes
            .checked_add(record.file.bytes)
            .ok_or("Pack size overflow.")?;
        if record.file.bytes == 0
            || record.file.bytes > MAX_LESSON_BYTES
            || payload_bytes > MAX_PACK_BYTES
        {
            return Err("Text-pack payload exceeds this reader's size limits.".into());
        }
        if record.file.sha256.len() != 64
            || !record
                .file
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("Pack SHA-256 values must be 64 lowercase hexadecimal characters.".into());
        }
        provenance(&record.provenance)?;
        let bytes = read_bounded(&file_path(&root, &record.file.path)?, record.file.bytes)?;
        if bytes.len() as u64 != record.file.bytes || sha256(&bytes) != record.file.sha256 {
            return Err(format!(
                "Pack integrity verification failed: {}",
                record.file.path
            ));
        }
        let lesson: Lesson = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Invalid lesson {}: {e}", record.lesson_id))?;
        if lesson.exercises.len() > 1000
            || lesson
                .exercises
                .iter()
                .any(|e| e.options.len() > 1000 || e.accepted.len() > 1000)
        {
            return Err("A lesson exceeds the supported exercise/answer count limits.".into());
        }
        validate_lesson(&lesson)?;
        level(&lesson.level)?;
        let expected_status = if trusted_review { "approved" } else { "draft" };
        if lesson.id != record.lesson_id
            || lesson.level != unit.level
            || lesson.editorial_status != expected_status
            || lesson.license != record.provenance.license
        {
            return Err(
                "Lesson identity, level, editorial status or license disagrees with its manifest."
                    .into(),
            );
        }
        if record.objective_ids.is_empty() {
            return Err("Lessons must declare communicative objectives.".into());
        }
        unique(&record.objective_ids, "lesson objectives")?;
        for objective_id in &record.objective_ids {
            if objectives
                .get(objective_id.as_str())
                .is_none_or(|o| o.level != lesson.level)
            {
                return Err("Lesson objective is missing or belongs to another level.".into());
            }
        }
        unique(
            &record.recommended_prior_knowledge,
            "recommended prior knowledge",
        )?;
        if record
            .recommended_prior_knowledge
            .iter()
            .any(|p| !objectives.contains_key(p.as_str()))
        {
            return Err("Unknown recommended prior knowledge.".into());
        }
        for (ids, expected) in [
            (&record.vocabulary_sense_ids, TargetKind::VocabularySense),
            (&record.grammar_target_ids, TargetKind::Grammar),
        ] {
            unique(ids, "lesson targets")?;
            if ids
                .iter()
                .any(|id| targets.get(id.as_str()) != Some(&expected))
            {
                return Err("Unknown or incorrectly typed lesson target.".into());
            }
        }
        match record.estimated_minutes {
            Some(0) => return Err("Estimated lesson duration must be positive.".into()),
            Some(minutes) if !(5..=10).contains(&minutes) => {
                pacing_outside_target.push(record.lesson_id.clone())
            }
            None => {
                if trusted_review {
                    return Err("Reviewed lessons require a reviewed pacing estimate.".into());
                }
                unmeasured_pacing.push(record.lesson_id.clone());
            }
            _ => {}
        }
        let mut mappings = HashMap::new();
        for mapping in &record.exercises {
            if let Some(source) = &mapping.provenance {
                provenance(source)?;
            }
            if mappings
                .insert(mapping.exercise_id.as_str(), mapping)
                .is_some()
            {
                return Err("Duplicate exercise mapping.".into());
            }
        }
        for exercise in &lesson.exercises {
            id(&exercise.id)?;
            text(&exercise.hint, "Exercise hint")?;
            for option in &exercise.options {
                id(&option.id)?;
                text(&option.text, "Option/tile text")?;
            }
            if !exercise_ids.insert(exercise.id.clone()) {
                return Err("Exercise IDs must be unique across the pack.".into());
            }
            let mapping = mappings
                .remove(exercise.id.as_str())
                .ok_or("Exercise is missing its objective/skill mapping.")?;
            let allowed = match (&exercise.kind, mapping.translation_direction) {
                (ExerciseKind::Translation, Some(TranslationDirection::EnglishToItalian)) => {
                    Skill::Writing
                }
                (ExerciseKind::Translation, Some(TranslationDirection::ItalianToEnglish)) => {
                    Skill::Reading
                }
                (ExerciseKind::Cloze, None) => Skill::Writing,
                (ExerciseKind::Choice | ExerciseKind::Reading | ExerciseKind::Ordering, None) => {
                    Skill::Reading
                }
                _ => {
                    return Err(
                        "Only translation exercises declare an explicit translation direction."
                            .into(),
                    )
                }
            };
            if let Some(direction) = mapping.translation_direction {
                directions.insert(direction);
            }
            if mapping.objectives.is_empty() {
                return Err("Exercise evidence requires an objective.".into());
            }
            let mut mapped_objectives = HashSet::new();
            for mapped in &mapping.objectives {
                if !record.objective_ids.contains(&mapped.objective_id)
                    || !mapped_objectives.insert(mapped.objective_id.as_str())
                {
                    return Err("Exercise objective is undeclared or duplicated.".into());
                }
                let objective = objectives[mapped.objective_id.as_str()];
                unique(&mapped.tested_skills, "tested skills")?;
                if mapped.tested_skills.is_empty()
                    || mapped
                        .tested_skills
                        .iter()
                        .any(|s| *s != allowed || !objective.skills.contains(s))
                {
                    return Err(
                        "Exercise format cannot supply the claimed Italian skill evidence.".into(),
                    );
                }
                evidence
                    .entry(mapped.objective_id.as_str())
                    .or_default()
                    .extend(&mapped.tested_skills);
            }
        }
        if !mappings.is_empty() {
            return Err("Mapping references an exercise absent from the lesson.".into());
        }
        lessons.push(lesson);
    }
    if membership != lesson_ids {
        return Err("Unit ordering and pack lessons must contain exactly the same IDs.".into());
    }
    let mut by_id = lessons
        .into_iter()
        .map(|lesson| (lesson.id.clone(), lesson))
        .collect::<HashMap<_, _>>();
    let mut lessons = vec![];
    for unit in &curriculum.units {
        for lesson_id in &unit.lesson_ids {
            lessons.push(
                by_id
                    .remove(lesson_id)
                    .ok_or("Unit ordering references a missing lesson.")?,
            );
        }
    }
    let mut coverage = vec![];
    let mut missing_required_evidence = vec![];
    for objective in &curriculum.objectives {
        let missing = objective
            .skills
            .iter()
            .filter(|s| {
                !evidence
                    .get(objective.id.as_str())
                    .is_some_and(|e| e.contains(s))
            })
            .copied()
            .collect::<Vec<_>>();
        if objective.required && !missing.is_empty() {
            missing_required_evidence.push(MissingEvidence {
                objective_id: objective.id.clone(),
                skills: missing,
            });
        }
    }
    for level in LEVELS {
        for strand in [
            Strand::Vocabulary,
            Strand::Grammar,
            Strand::Reading,
            Strand::Writing,
            Strand::Listening,
            Strand::Speaking,
            Strand::CulturalContext,
            Strand::Dialogues,
        ] {
            let required = curriculum
                .objectives
                .iter()
                .filter(|o| o.required && o.level == level && o.strands.contains(&strand))
                .collect::<Vec<_>>();
            let represented = required
                .iter()
                .filter(|o| {
                    o.skills
                        .iter()
                        .all(|s| evidence.get(o.id.as_str()).is_some_and(|e| e.contains(s)))
                })
                .count();
            coverage.push(CoverageCell {
                level: level.into(),
                strand,
                required_objectives: required.len(),
                represented_objectives: represented,
                reviewed_objectives: if trusted_review { represented } else { 0 },
            });
        }
    }
    let report = PackReport {
        pack_id: manifest.id.clone(),
        version: manifest.version,
        manifest_sha256: digest,
        editorial_status: manifest.editorial_status,
        trusted_review,
        lesson_count: lessons.len(),
        payload_bytes,
        coverage,
        missing_required_evidence,
        missing_translation_directions: [
            TranslationDirection::ItalianToEnglish,
            TranslationDirection::EnglishToItalian,
        ]
        .into_iter()
        .filter(|d| !directions.contains(d))
        .collect(),
        unmeasured_pacing,
        pacing_outside_target,
    };
    Ok(ValidatedPack {
        manifest,
        lessons,
        report,
    })
}

fn validate_prerequisites(objectives: &HashMap<&str, &Objective>) -> Result<()> {
    let mut degree = HashMap::new();
    let mut dependants: HashMap<&str, Vec<&str>> = HashMap::new();
    for (objective_id, objective) in objectives {
        unique(
            &objective.recommended_prerequisites,
            "objective prerequisites",
        )?;
        degree.insert(*objective_id, objective.recommended_prerequisites.len());
        for prior in &objective.recommended_prerequisites {
            if prior == objective_id || !objectives.contains_key(prior.as_str()) {
                return Err("Unknown or self-referencing objective prerequisite.".into());
            }
            dependants
                .entry(prior.as_str())
                .or_default()
                .push(objective_id);
        }
    }
    let mut ready = degree
        .iter()
        .filter_map(|(id, n)| (*n == 0).then_some(*id))
        .collect::<VecDeque<_>>();
    let mut visited = 0;
    while let Some(next) = ready.pop_front() {
        visited += 1;
        for child in dependants.get(next).into_iter().flatten() {
            let count = degree
                .get_mut(child)
                .ok_or("Unknown objective prerequisite.")?;
            *count -= 1;
            if *count == 0 {
                ready.push_back(child);
            }
        }
    }
    if visited != objectives.len() {
        return Err("Recommended objective prerequisites contain a cycle.".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;
