# Curriculum authoring contract v1

Implementation decision under SPEC.md §§3, 5–6 / AC-03, 2026-10-15.
This contract defines an **offline, read-only text-pack preflight**. It does not install,
download or extract packs, change learner storage, approve a curriculum or award
credit. The separate library and desktop own installation and learner integration.

## Run

```sh
npm run check:content
# Another authoring directory:
cargo run -p learning-core --example check_pack --locked -- path/to/pack
```

The default checks `content/development/pack.json` and the existing `cafe.json`.
Success exits 0 and prints a JSON report to stdout; rejection exits 1; incorrect
arguments exit 2. Build messages and the approval disclaimer go to stderr. After
initial Cargo setup, validation needs no network and adds no dependency.
The executable contract is `learning_core::content::{PackManifest, validate_pack}`.
Unknown fields are rejected. This is an internal versioned reader contract; the
complete public media/installation format is unfinished.
The separate [text-pack library](PACK-LIBRARY.md) now provides atomic local
installation, verification, version activation/rollback, removal and repair for
developer/host use and desktop Settings, with typed learner-backup references.

## Records

| Record             | Contract                                                                                                                                                       |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Pack               | `format=lang-curriculum-pack`, format/lesson schema version 1, positive version, ID/title, `language=it`, `instructionLanguage=en`, scope and editorial status |
| Reader range       | Positive inclusive min/max reader protocol versions; currently 1, independent of app package version                                                           |
| Objective manifest | Schema version 1, ID/version; objectives, vocabulary/grammar targets and ordered thematic units                                                                |
| Objective          | ID, A1–C2 level, description, required flag, strands, tested skills and recommended prerequisite objective IDs                                                 |
| Unit               | ID, level, title and ordered lesson IDs; every lesson belongs to exactly one unit                                                                              |
| Target             | ID, description and vocabulary-sense or grammar type; lessons reference them separately                                                                        |
| Lesson record      | Lesson/unit IDs, file record, provenance, objective/target references, recommended prior knowledge, nullable estimated minutes and exercise mappings           |
| File               | Portable relative path, exact bytes and lowercase SHA-256; verified bytes are parsed as the existing version-1 `Lesson`                                        |
| Exercise mapping   | Existing exercise ID, objective-specific tested skills and an explicit direction for translation exercises                                                     |
| Provenance         | Original AI draft, original human or external origin; nonblank license/attribution; external material needs a source citation                                  |
| Review             | Reviewer attribution, UTC timestamp and explicit review areas; metadata cannot establish that review occurred                                                  |

Objectives/targets may override pack provenance, and individual exercise mappings
may override lesson provenance. Omitted overrides inherit that enclosing source;
every external override needs its own citation. This supports mixed-source text
without treating the whole lesson as original material.

Beginner remains an entry route into A1. IDs start with a lowercase ASCII letter
or digit, contain lowercase letters/digits plus `.`/`_`/`-`, and have at most 128
bytes. Lesson and exercise IDs are each unique across a pack; option/tile IDs are
local to their exercise. Targets and objectives use typed reference namespaces.

Objective prerequisites must exist and form an acyclic graph. Recommendations do
not lock topic access. Lesson objectives must match the lesson level. Unit ordering
and lesson metadata must agree. Each actual exercise needs exactly one mapping;
dangling mappings/references, duplicates and incorrect target types fail.

## Evidence and coverage

Reports include all six levels × eight strands: vocabulary, grammar, reading,
writing, listening, speaking, cultural context and dialogues. Each cell separates
required objectives, objectives with every declared skill structurally represented,
and externally trusted reviewed objectives. **Zero required objectives means an
unplanned area, not completed coverage.** Missing required skill mappings and
translation directions are listed explicitly.

This audits authoring metadata; it does not count learner attempts, mastery or
CEFR proficiency, and exposes no automatic level/course-completion flag. Full
curriculum approval still requires the reviewed objective map and manual content
QA, including advanced synthesis, extended production and rubrics.

| Implemented exercise                          | Italian skill mapping in reader v1                                 |
| --------------------------------------------- | ------------------------------------------------------------------ |
| Choice, reading comprehension, ordering tiles | Reading                                                            |
| English → Italian translation                 | Writing                                                            |
| Italian → English translation                 | Reading; writing an English answer is not Italian writing evidence |
| Typed cloze                                   | Writing                                                            |

These are modality tags, not proof of task quality/difficulty. Authors/reviewers
must verify declared language/direction and naturalness; the checker does not
detect language automatically. No current type supplies listening/speaking evidence.
Planned objectives may require those skills, but stay unrepresented until supported
activities exist. Future exercise/media fields are refused by this reader.

The café mappings are development metadata, not the approved course map. The
fixture only has English → Italian translation. Its duration remains unmeasured.
Missing estimates are reported; estimates outside 5–10 minutes are flagged for
review, not treated as a mandatory time limit. Reviewed lessons need an estimate
and pacing-review record.

## Editorial authority

The CLI uses an empty `ContentTrust` registry: it accepts valid drafts and rejects
approval claims. A future trusted host may supply the exact SHA-256 of a manifest
only after real external editorial review. That digest pins payload hashes,
provenance, mappings and review records. Editing any manifest bytes invalidates
its trust. A pack cannot supply its own registry; there is no approve command.

Even with a trusted digest, approved content needs course scope, approved lesson
status, consistent licenses, pacing estimates and review records for linguistic
quality, pedagogy, answer variants/distractors, licensing and pacing. Draft packs
cannot carry approved lessons. The development fixture has no review records;
synthetic records exist only in isolated unit tests.

**Hashes verify bytes, not authorship, rights, quality or human review.** The
host's registry must come from independent editorial work, never automatically
from the pack's labels or printed digest. Real review, rights and publication
remain release gates. Media/voice rights and audio QA need a later media contract.

## Files and limits

Only directory-based text packs are supported. Paths use `/` and portable ASCII
segments. Absolute paths, traversal, backslashes, device names, alternate streams,
trailing dots/spaces and case collisions are refused. Referenced files/folders
cannot be symlinks and must resolve under the chosen root. Non-regular files are
rejected before opening them. Nothing is executed or fetched from source citations.

Limits: 1 MiB manifest, 8 MiB per lesson, 64 MiB total declared lesson bytes,
1000 lessons, and 1000 exercises/options/accepted variants per applicable array.
Descriptions/attribution/hints/option text are limited to 4000 bytes. These are
input limits, not measured hardware performance or promised full-course/media
budgets. Larger formats need measured limits and compatible readers; no silent
truncation occurs.

Only declared lesson files are read; returned lessons are parsed from verified bytes.
Unlisted authoring files are ignored,
not inspected or approved. A future installer must stage/copy only verified
payloads, not the entire source directory, and recheck integrity. Preflight is
not atomic installation or defence against concurrent hostile filesystem edits.

`.gitattributes` pins repository JSON fixtures to LF for identical byte hashes on
Windows/macOS. Updates must change hashes and increment relevant content versions;
the future installer must reject changed payloads reusing installed versions. This
read-only tool does not enforce an installed-version update/rollback policy.

Still required: full reviewed A1–C2 objectives/content; media and accessible
alternatives; downloads, full course/reference browsing; review-item generation
and FSRS. None is marked complete by passing preflight.
