# Italian Learning Application Specification

Status: **Approved — source of truth for development**

This user-approved specification is the source of truth for development.
It records confirmed requirements, bounded implementation decisions, and explicitly
deferred work. The user approved the completed specification and requested that
it be pushed to the project repository. Application code had not been written
when approval was granted.

## 1. Requirement status and approval

- **Confirmed:** Explicitly requested or accepted by the user.
- **Recommendation:** A proposal for consideration; not a product requirement.
- **Unresolved:** A decision or fact still needed. No default is implied.
- **Deferred:** Explicitly accepted for a later release, with its boundary recorded.

Recommendations do not become requirements through inclusion in this document.
Unanswered questions do not authorize implementation assumptions. When a decision
is made, record the answer, affected sections, and acceptance criteria here.

### Confirmed requirements

- C-01: Build an Italian language-learning application.
- C-02: The application must be completely free, open source, and contain no ads.
  Donations are acceptable in principle; a donation mechanism is not yet required.
  AI conversations must also be available without charging users. Funding online
  AI remains unresolved; a required paid API key cannot satisfy free access.
- C-03: Offline study must cover every agreed learning feature except AI
  conversations and shared leaderboards. Initial downloads are acceptable.
  An offline conversation mode is not required by the accepted server approach.
  Lessons and progress work offline; pending leaderboard contributions upload
  when connectivity returns.
- C-04: Duolingo is an inspiration. The selected features are recorded in C-17;
  exercise/interaction rules are specified below. The course uses original content.
- C-05: Initial platforms are macOS and Windows.
- C-06: iPhone support is planned later; timing, feature parity, and distribution
  have not been decided.
- C-07: Before application code, produce a comprehensive specification covering
  architecture, UI/UX, curriculum, exercises, progress tracking, audio, speaking
  practice, AI tutoring, testing, and deployment.
- C-08: Do not invent requirements or assume the user's preferences. Ask for
  clarification when a decision is necessary.
- C-09: Distinguish confirmed requirements, recommendations, and unresolved
  decisions.
- C-10: After the completed specification is approved, use it as the source of
  truth for development.
- C-11: The initial learner is the project owner; open-source distribution is
  intended. Public-release support scope remains unresolved.
- C-12: Present the learner with an option to select their Italian proficiency
  level: complete beginner or CEFR A1–C2. Changing the starting level is allowed
  without erasing progress; earlier topics stay accessible. Offer an optional
  placement test. The selected level recommends a starting point.
- C-13: Instructions use English. Other interface localization is not decided.
- C-14: The learning goal is Italian fluency with a CEFR C2 course endpoint.
  Placement/checkpoints recommend progress without CEFR certification claims.
  Advanced offline writing uses model answers and self-review rubrics; record
  self-assessment separately from objectively scored evidence.
- C-15: Official launch is a long-term goal, and the user wants all agreed features
  available then. Do not propose omitting speaking, audio, or AI conversations
  from official launch merely to shorten the schedule. The detailed feature
  inventory and internal development milestones are defined below, with exact
  lesson counts selected against the agreed coverage map during authoring.
- C-16: AI conversations support both text and voice. Select an approach that
  keeps them free to users. The owner can support an always-on computer but has
  not committed to operating it now. AI availability must be independent of the
  rest of the application; no specific model, server hardware, or budget is chosen.
- C-17: Include every motivation/navigation feature listed in discovery except
  hearts/lives: sequential lesson path, freedom to jump between topics, short
  lessons, streaks, XP, daily goals, achievements, leaderboards, reminders, and
  spaced-repetition review. No lives or hearts mechanic.
- C-18: Aim for the broadest computer compatibility and keep the application as
  lightweight as possible. The user accepted the proposed target: macOS 12+
  (Intel x86-64/Apple Silicon ARM64), Windows 10/11 x86-64, and Windows 11 ARM64;
  target 4 GB system RAM with no dedicated GPU required for offline learning.
  These are requirements to validate, not measured compatibility claims. Linux,
  older systems, and 32-bit support have not been added to the initial OS scope.
- C-19: Use Tauri 2, Svelte/TypeScript, Rust, and SQLite.
- C-20: Shared leaderboards are unavailable offline. Offline lessons/progress
  continue normally, and their contributions populate the leaderboard once back
  online. Participation is optional under a nickname; offline study needs no
  login. Use an all-time leaderboard without leagues; identity credentials and
  hosting are bounded technical/deployment choices under R-07.
- C-21: When the AI server is off or unreachable, only AI features are unavailable;
  the rest of the application must continue to work. Include the AI feature even
  if the owner does not run its server during early development.
- C-22: Offline speaking feedback is word-level only for now. Detailed sound,
  stress, and pronunciation scoring are not in the current launch specification.
- C-23: Cover vocabulary, explicit grammar explanations, reading, writing,
  listening, speaking, cultural context, and dialogues through C2.
- C-24: Include multiple choice, translation in both directions, word ordering,
  dictation, fill-in-the-blank, reading comprehension, listening comprehension,
  and read-aloud exercises.
- C-25: Course content may be original AI-drafted material manually reviewed before
  inclusion. Track attribution/licensing for external material. Aim for natural,
  convincing content and polish comparable to established language-learning apps.
- C-26: Lessons are approximately 5–10 minutes, with immediate explanations and
  unlimited retries. This is a pacing target, not a mandatory time limit.
- C-27: Use playful, colorful original branding; desktop sidebar navigation;
  light/dark themes; keyboard, screen-reader, text-scaling, and reduced-motion
  support. Brand name/artwork are explicitly deferred.
- C-28: Use pre-generated, downloadable audio packs with a standard Italian voice,
  normal/slow playback, and unlimited replay. Check voice/model redistribution
  rights before selection. No specific voice/provider is chosen.
- C-29: Initially support one local learner profile, with export/import backups,
  no login required for offline study, and optional nickname-based leaderboards.
- C-30: Automatic cross-device progress synchronization and multiple local learner
  profiles are outside the initial desktop scope. iPhone planning may revisit them.
- C-31: Apply the accepted progress/review/motivation rules in section 7 (R-04),
  including 10 XP/lesson, 5 XP/review, daily credit caps, goals, streaks,
  achievements, reminders, and offline spaced repetition.
- C-32: Apply the accepted answer/evidence/accessibility rules in sections 5–6:
  first-attempt evidence survives retries, reviewed answer variants, meaningful
  accents preserved, advanced self-review, and authored accessible alternatives.
- C-33: Apply the accepted AI conversation/privacy/retention contract R-06.
- C-34: Apply the accepted resource budgets and bounded technical/content
  decision process R-07. Exact versions/models/voices/lesson counts are selected
  within those limits, documented, and validated; feature/budget changes need
  clarification. Brand name/artwork and public hosting specifics are deferred.
- C-35: Use the user's existing public GitHub repository
  [misc9115-boop/lang](https://github.com/misc9115-boop/lang), with `main` as its
  default branch. The repository was verified empty before this specification.
  Clone URL: `https://github.com/misc9115-boop/lang.git`.
- C-36: The user accepted the architecture, screen-flow, audio, synchronization,
  backup, test, and R-05 distribution/update/license recommendations. Use GitHub
  Releases for macOS DMG/Windows installers; install updates only with user approval;
  MIT for original code and CC BY 4.0 for eligible original curriculum/media.

Both the individual decisions and this completed specification are approved.
The current behaviors below are confirmed except where explicitly labeled as
recommendations, unresolved decisions, or deferred work. Implementation selection
within R-07 does not authorize expanding scope or relaxing agreed budgets.

## 2. Product definition and release scope

**Confirmed:** Initial personal use; open source; no ads; donations acceptable;
English instructions; selectable starting level; fluency goal; desktop first;
iPhone later; all agreed features available at official launch. Initial downloads
are acceptable; offline study works except for AI conversations and shared
leaderboards. Offline leaderboard contributions synchronize after reconnection.

**Unresolved product decisions:** None blocking this completed specification.
The user has explicitly delegated bounded technical/content choices and deferred
later editorial/public-host decisions under R-07. Final specification approval
has been granted; development must follow the confirmed requirements below.

**Deferred or bounded work:** Brand name/artwork, later interface localization,
public audience/capacity commitments, human review staffing, signing/funding, and
specific online host hardware. Donation support is acceptable but a donation
mechanism is not a mandatory learning feature. iPhone and cross-device sync remain
later scope. Detailed course/technical selections follow the confirmed R-07 process.

**Required implementation/release evidence:** An explicit launch scope, exclusions, learner
persona, offline capability list, and measurable learning/product outcomes.

## 3. Platforms and architecture

**Confirmed:** Initial macOS and Windows applications; future iPhone support;
Tauri 2 + Svelte/TypeScript + Rust + SQLite; support targets in C-18.
Non-conversation learning features must execute offline after their initial assets
are available. Their scoring, media, persistence, and any included tutor
explanations cannot require a remote service. The shared leaderboard is also an
approved online service, but neither online service may gate local study.
See R-02 for the accepted stack and remaining design recommendations.

### Confirmed component responsibilities

The following responsibilities are confirmed; implementation details follow R-07:

- UI: Render locally packaged screens; collect answers; display feedback,
  progress, downloads, and conversation state. Ship fonts/icons locally where
  used so learning screens do not depend on external resources.
- Learning core: Interpret versioned curriculum data, select exercises, evaluate
  supported answers, schedule reviews, and update progress without network calls.
- Persistence: Store learner settings, attempts, mastery, reviews, session state,
  streaks, XP, goals, and achievements transactionally in SQLite. Keep large audio
  and speech-model files outside the database.
- Content manager: Read installed curriculum/media packs; validate compatibility
  and integrity before installation; report missing assets and download progress.
- Audio/speech adapter: Play installed audio and run offline speech recognition
  for selected speaking exercises. Load recognition models only while needed.
  Recognition and pronunciation assessment are separate capabilities.
- Conversation adapter: Exchange text with a selected AI endpoint. Voice mode
  adds microphone input, transcription, and response audio. Keep this dependency
  outside lesson scoring and review so its outage does not interrupt offline study.
- Leaderboard adapter: Keep contributions from offline study locally, submit them
  when connected, and fetch shared standings. Show unavailable status while
  offline. Apply R-04 competition rules; host/identity/protocol details follow R-07. Do not present fabricated learners as real people.
- Platform adapter: Microphone permissions, local notifications, file dialogs,
  and packaging. Future iPhone adapters can share learning/content rules while
  adapting platform interactions.

**Confirmed data boundaries:** Separate installed curriculum versions from learner
state. Use stable content/skill identifiers so content updates do not silently
erase progress. Record attempts separately from derived progress, with schema
versions and migration/backup rules defined before implementation. Exact fields are selected under R-07; retention follows R-06 and migrations must
preserve validated backups and progress.

### Confirmed network and failure contracts

- Missing/unreachable AI server: Text/voice AI sessions are unavailable; normal
  lessons, offline speaking, audio, review, and local progress remain functional.
  Opening the app or starting a lesson must not require AI service startup.
- Offline leaderboard: Shared standings are unavailable; lessons still record
  local progress and contributions that will populate the leaderboard online.
- Reconnection: Synchronize pending leaderboard contributions without requiring
  the learner to repeat completed lessons.
- Initial downloads: Network access may be needed to obtain assets; once required
  assets are installed, the confirmed offline study contract applies.

**Confirmed synchronization mechanism:** Persist a local
outbox in the same transaction as the qualifying progress change. Give each
contribution a stable identifier and require idempotent server acknowledgements
so retries cannot count it twice. Retain unacknowledged entries across restarts;
retry with backoff; acknowledge only committed server updates. Never block local
progress writes on a network request. Authentication and validation details follow R-07. R-04 uses all-time XP, so
there is no expired weekly period that drops legitimate offline contributions.

**Confirmed outage UX:** Show AI service status within the
AI screen with a retry action and a route back to offline study. A missing
endpoint should report that conversations are not configured. Do not simulate a
live tutor. Apply bounded timeouts/cancellation to pending conversation requests,
and prevent repeated automatic requests while the service is down. If voice mode
fails mid-turn, preserve any recoverable draft/transcript; do not award XP or mastery for
AI chat; R-04 excludes conversation XP and R-06 defines history/retention. Leaderboard status should disclose queued contributions and successful
synchronization without presenting stale standings as current.

**Bounded implementation work (R-07):** Validate supported targets and budgets;
select dependency versions/build tooling, schema fields/indexes, credentials and
recovery, pack formats, IPC, and network protocol. Preserve offline study, backups,
version compatibility, idempotent progress/sync, and failure isolation. Reuse the
learning core/content on future iPhone; mobile UI/platform validation is later work.

### Confirmed local records and interfaces

**Confirmed:** Use the following conceptual schema;
implementation may refine field types/indexes after approval without changing
the agreed behavior:

- Profile/settings: Stable local profile ID; chosen level; theme/accessibility
  preferences; goal; reminder settings; study time zone; schema version.
- Content catalogue: Pack ID/version, level, installed location, integrity hash,
  license/attribution, compatibility range, and install state.
- Curriculum: Stable unit/lesson/objective/skill/exercise IDs, content version,
  ordering, recommended prerequisites, and asset references.
- Attempt: Unique ID; session/exercise/content-version IDs; response or evaluated
  result as permitted by retention policy; hints; correctness; completion time.
- Session: Type; lesson/review ID; current position; saved answer state; original
  selection seed; start/completion status; first-attempt results.
- Progress: Completed lessons, assessment evidence, XP ledger, study-day ledger,
  goals, and achievement grants. Completion and skill mastery remain distinct.
- Review item/log: Vocabulary sense or grammar objective plus tested skill,
  scheduler state, due time, result/rating history, and scheduler version.
- Leaderboard outbox: Unique contribution ID, qualifying local event, XP amount,
  identity, submission/acknowledgement state, and retry metadata.
- Online identity: Nickname, server identifier, participation consent, and a
  protected credential reference; credential/recovery method still to be decided.
- Tutor history: Conversation/turn IDs, selected scenario/level, and text only if
  per R-06. Raw recordings are not retained by default.

Suggested typed core commands cover course browsing, starting/resuming a session,
submitting an answer, applying a review rating, fetching progress, managing packs,
export/import, speech capture/transcription, conversations, and leaderboard sync.
The UI cannot write arbitrary SQL or invoke unrestricted native operations.

**Confirmed backup contract:** Export a versioned archive of learner data/settings and
installed-pack references, not duplicate all downloadable media. Validate import
before replacement; create a pre-import backup; never silently merge two learner
histories while automatic cross-device sync is out of scope. Include tutor text only with explicit export consent (R-06); define secure
credential recovery under R-07 without creating unsupported concurrent identity use.

**Required implementation/release evidence:** Approved stack with researched tradeoffs;
component/data-flow description; storage schema; asset lifecycle; platform
support matrix; explicit network boundary and failure behavior.

## 4. UI/UX and accessibility

**Confirmed:** English instructions and a learner-selectable CEFR Italian level;
sequential lesson path plus freedom to jump between topics; short lessons;
streaks, XP, daily goals, achievements, leaderboards, reminders, and review;
no hearts/lives; playful original visuals; sidebar; light/dark themes; keyboard,
screen-reader, text-scaling, and reduced-motion support. Beginner/A1–C2 selection,
optional placement, freely changeable level, and approximately 5–10-minute lessons
with immediate explanations and unlimited retries are accepted.

### Confirmed screen and interaction contract

**Confirmed:**

1. Onboarding: Explain local study/online exceptions; choose beginner/A1–C2 or
   optional placement; select/install required packs; enter the course without
   signing in. Request microphone/notification permissions when those features
   are invoked, not as a prerequisite for reading lessons.
2. Learn: Recommended sequential path, level selector, searchable/free topic
   access, lesson objectives, completion evidence, and installed-asset state.
3. Lesson: Prompt, contextual audio/image where authored, answer controls,
   Check/Continue action, optional hint, immediate result/explanation, progress
   indicator, and exit/resume control. Unlimited retries do not erase first-answer
   evidence. Do not imply that a hint-assisted answer was independent mastery.
4. Lesson summary: What was practised, first-attempt results, review needs,
   completion, XP/goal/streak changes, and next recommended action.
5. Review: Due vocabulary/grammar/skill items, session preview, and ability to
   practise when nothing is due. Missed days do not block new lessons.
6. Reference: Searchable vocabulary/grammar explanations, examples, and replayable
   installed audio. Its separation from topic browsing is a delegated layout choice.
7. Conversations: Text/voice mode, selected level/scenario, service status,
   turn history, send/stop/retry, recording indicator, and recoverable drafts.
8. Progress: Course completion, skill evidence, review due counts, goals, XP,
   streaks, and achievements with clear explanations of what each metric means.
9. Leaderboard: Optional participation, nickname, standings, and pending/synced
   contribution state. Offline/unreachable state cannot interfere with study.
10. Settings: Level, theme, accessibility, reminders, audio, microphone, downloads,
    backups, local data reset/deletion, service configuration, and attribution.

Proposed lesson state sequence: ready → answering → evaluated → feedback → next
exercise → summary. Saving/exiting is available between interactions; cancelling
an unfinished session does not count as completion. Exact skip/hint mechanics follow R-07 while preserving first-answer evidence. Disable repeated submission while an answer result is being committed.

Proposed accessibility checks: Visible keyboard focus; logical tab order; labels
and announcements for prompts/results; no information conveyed by color alone;
layouts usable with enlarged text; no essential animation; alternative text for
meaningful images; readable contrast in both themes. Provide a non-speaking
study path when a microphone is absent; accessible substitutes can complete lessons without fabricated skill evidence.

**Bounded implementation/editorial work (R-07):** Refine responsive layouts,
exercise controls, placement items, hint/skip/resume mechanics, state visuals, and
accessible focus/announcements while preserving the confirmed flows and evidence
rules. Brand name/artwork are explicitly deferred. Document interaction refinements
and validate them against the accepted accessibility and pacing requirements.

**Required implementation/release evidence:** Approved screen inventory and flows, interaction
rules, accessibility target, and representative wireframes if requested.

## 5. Curriculum and content

**Confirmed:** CEFR framework, learner-selected starting level, C2 endpoint,
English instructions, all course strands in C-23, and original AI-drafted content
with manual review and tracked external attribution/licenses. Standard Italian
audio and both translation directions are selected.

### Confirmed curriculum map and authoring contract

**Confirmed:** Use level → thematic unit → lesson →
exercise. Each lesson declares communicative objectives, vocabulary senses,
grammar targets, skill coverage, recommended prior knowledge, and an estimated
duration. Recommended prerequisites guide ordering without locking topic access.

The following are confirmed Italian teaching themes aligned to the accepted CEFR
endpoint, not a claim that CEFR prescribes this grammar sequence:

- Beginner/A1: Italian spelling/sounds and everyday expressions; introductions,
  numbers, time, family, food, travel basics; articles, gender/number agreement,
  present tense, core verbs, negation, and simple questions.
- A2: Everyday transactions, home/work, directions, routines, and short narratives;
  common past/future forms, pronouns, comparisons, and connected short writing.
- B1: Sustained everyday conversation, experiences, plans, opinions, and
  problem-solving; narration, tense contrasts, conditional forms, common
  subjunctive uses, and connected reading/listening/writing.
- B2: Arguments, current affairs, work/study, and less predictable situations;
  complex sentences, nuance, register choices, idioms in context, and sustained
  comprehension/production using varied speakers and texts.
- C1: Demanding texts/discussion, implicit meaning, persuasive structure,
  professional/academic expression, collocations, pragmatic nuance, and flexible
  reformulation. Include authentic-style longer passages and extended production.
- C2: Subtle register/style distinctions, dense or idiomatic language, synthesis
  across sources, precise argument, and natural reformulation. Use original
  advanced texts/dialogues and production tasks with explicit assessment rubrics.

The beginner choice is an entry route into A1 foundations, not a new CEFR level.
Advanced reading/listening/writing can span linked short lessons; do not reduce
C1/C2 coverage to isolated vocabulary recognition or imply that word-level speech
feedback proves advanced spoken proficiency.

**Confirmed content release rule:** Approve a versioned objectives manifest
covering every level/strand; map reviewed lessons and assessment evidence to it;
do not mark a level complete while required objectives are unrepresented.
Exact lesson totals are selected during content authoring under the accepted
bounded approach (R-07). The complete curriculum remains an official-launch
requirement even if built/tested incrementally.

**Confirmed content record:** Stable ID/version; level/unit/lesson; skill/objective
tags; English instruction; Italian/English prompt text; response type; canonical
and explicitly accepted answers; distractors; explanations; hint; audio/image
references; matching rules; source/license; editorial review status.

AI drafting is an authoring activity, not a runtime internet dependency. Proposed
pipeline: objective brief → original draft → linguistic/pedagogical review →
answer-variant/distractor review → audio generation/review → schema/reference
validation → versioned pack. Unreviewed drafts do not ship as approved lessons.
Review should cover natural Italian, translation ambiguity, believable dialogue,
register, grammar accuracy, distractor fairness, difficulty, and audio/text match.

**Confirmed assessment boundary:** The optional placement test is an offline,
authored diagnostic with explanations and a recommended entry level that the
learner can override. It does not award unearned completion or certify CEFR
proficiency. Per-level checkpoints sample the approved objectives; advanced
production uses exemplars/rubrics and records self-assessment separately from
objectively scored answers. Certification is outside this proposal. Native or
otherwise qualified Italian review is recommended for the final curriculum;
reviewer availability/cost is not assumed.

**Illustrative lesson example — not final authored content:** An illustrative A1 café lesson teaches ordering politely, article/
noun agreement, and basic prices. It combines short vocabulary/audio recognition,
ordering a phrase, translation, a brief recorded/generated dialogue with a
comprehension question, and optional read-aloud. The exercise sequence and Italian
utterances still require authored review; this example is not launch content.

**Bounded authoring/release work (R-07):** Author detailed objectives, units,
examples, vocabulary senses/forms, accepted variants, rubrics, diagnostics, and
lesson counts against the confirmed map. Use natural standard Italian with
register/cultural notes where the objective requires them. Resolve individual
external rights and human reviewer sign-off before a pack becomes approved.
Original-content licensing follows confirmed R-05 and any third-party rights.

**Required implementation/release evidence:** Approved curriculum map, lesson/content schema,
sample lesson, sourcing plan, editorial quality criteria, and launch inventory.

## 6. Exercises and answer evaluation

**Confirmed:** All exercise types in C-24; immediate explanations; unlimited
retries; offline operation; both Italian→English and English→Italian translation.

### Confirmed per-exercise contracts

**Confirmed:**

- Multiple choice: Choose the authored correct option; distractors are plausible
  and unambiguous. Shuffle without changing correctness references.
- Translation: Type an answer or use authored tiles where appropriate. Match
  curated variants/normalization; explain the target construction and accept
  genuinely equivalent reviewed alternatives.
- Word ordering: Assemble the target sentence; accept authored alternate orders.
  Distinguish identical duplicate tiles by internal IDs rather than visible text.
- Dictation: Listen and type the Italian utterance; normal/slow replay is allowed.
  Reveal the transcript after evaluation or on an explicit solution action.
- Fill-in-the-blank: Select/type a form consistent with the sentence's meaning
  and grammar; provide contextual explanations for gender, number, tense, etc.
- Reading/listening comprehension: Answer authored questions about a passage or
  dialogue; source, question, and answer must agree. Transcript reveal is a hint
  for listening assessment and not independent listening evidence.
- Read-aloud: Record an expected utterance; compare recognized words against its
  authored accepted variants; display word-level feedback and allow another try.
- Extended writing/production: Use guided prompts, reviewed exemplars, and
  self-review rubrics offline. Do not claim exhaustive automatic evaluation of
  unrestricted prose unless a separately approved offline method supports it.

**Confirmed answer policy:** Normalize Unicode, whitespace, typographic apostrophes,
case, and non-semantic terminal punctuation. Preserve meaningful Italian accents,
apostrophes, agreement, and lexical differences; classify a clear accent/typing
slip separately from a meaning/grammar error. Do not generically strip accents
(`e`/`è`) or apostrophes. Permit only reviewed equivalences for an exercise;
accept a harmless typo with a note only when the exercise explicitly permits
that variant and spelling is not its target. Otherwise return correction/retry;
do not use an unrestricted fuzzy match that accepts a different valid word.
free translation is not guaranteed to have a single natural answer. Surface an
"answer should be accepted" local report for editorial review. AI service output
must not determine offline lesson correctness or unlock progress.

**Confirmed evidence rules:** Store first-attempt correctness separately from
eventual completion; hints/reveals remain visible in evidence. Retrying a failed
item is encouraged but cannot rewrite the original result. A completed session
means all required items were attempted and corrected/reviewed under the agreed
policy, not that every response was first-try correct. Approved accessible substitutions and advanced self-assessment can complete
lessons, with their evidence labeled separately.

**Confirmed alternative paths:** Microphone denial, unavailable audio access, or
unusable recognition offers an authored non-speaking/non-audio activity covering
the communicative objective. It may count for lesson completion, but does not
add speaking/listening mastery evidence that was not demonstrated. Track the
substitution explicitly. If no alternative asset is installed, explain what is
missing without silently grading an exercise the learner cannot perform.

**Bounded implementation/content work (R-07):** Select per-lesson item counts,
randomization seeds, permitted typed/tile responses, explicit normalization/variant
settings, speech matching confidence, and resumable state contracts. These choices
must preserve first-answer evidence, offline scoring, unlimited retries, approved
accessible alternatives, and natural/fair prompts. Document skip/reveal handling
so completion cannot silently become independent mastery evidence.

**Required implementation/release evidence:** Per-exercise interaction and scoring contracts,
representative correct/incorrect/alternative-answer examples, and fallback rules.

## 7. Progress, review, and learner data

**Confirmed:** One local profile; no login for offline study; export/import backups;
optional nickname-based online leaderboard; no initial automatic cross-device
sync or multiple local profiles. Study/progress work offline, with queued
leaderboard contributions submitted later.

### R-04: Confirmed progress, review, and motivation rules

**Confirmed:**

- Completion: Finish required lesson interactions, including correction/review of
  mistakes. Preserve independent first-attempt evidence; completion is not mastery.
- XP: 10 XP for a completed lesson, 5 XP for a completed review session. Each
  specific lesson earns XP at most once per study day; review XP at most once per
  day. Other practice remains unrestricted. No XP for opening screens, retries,
  aborted sessions, placement tests, or unavailable AI turns. Keep AI conversation
  out of XP/leaderboard scoring initially so server outages do not disadvantage
  offline learners. These quantities/limits were accepted in discovery round five.
- Daily goal: Default 10 XP; configurable positive goal. A goal edit applies to
  the current/future day without rewriting previously recorded days.
- Streak: One completed lesson/review qualifies a study day, independently of XP
  caps. Use a saved study time zone and local calendar dates; never retroactively
  shift historical days when the zone changes. No lives, paid repairs, or freezes
  in this proposal. A gap restarts the current streak, preserving best streak.
- Achievements: First lesson, first review, 7-day streak, 100 earned XP, and each
  completed course level. Grants are idempotent and list their criteria.
- Reminder: User opts in and chooses a local daily time; do not send after the
  day's goal is met. Denying permission does not affect lessons or streak rules.
- Review: Schedule vocabulary-sense/grammar/skill items locally using FSRS, with
  desired retention 0.9 and a versioned rating mapping. Incorrect/revealed
  answers rate Again; successful but hinted/corrected attempts rate Hard;
  independent correct answers rate Good. Easy is available only through an
  explicit learner self-rating, not inferred from typing speed. Review selection
  prefers due items; practice is still allowed without due items. Avoid recording
  multiple ratings for the same review item inside a single session.
- Skill evidence: Track reading/listening/production separately; report limited
  evidence honestly rather than equating XP or FSRS retention to a CEFR level.
  Placement recommends entry and does not mark unattempted lessons complete.
- Leaderboard: All-time qualifying XP, opt-in nickname participation, equal-score
  ties, no leagues/weekly reset initially. Offline contributions retain unique
  event IDs and upload when the service returns. Include a contribution once;
  do not fabricate offline ranks. Server validates identity, IDs, known content,
  XP rules, and plausible dates; an offline client cannot make cheating impossible.
  Refine validation without silently changing legitimate offline-credit rules.

FSRS has a Rust implementation; exact version/dependency footprint and rating
integration require validation. These are scheduling choices, not a claimed
guarantee of memory retention.
[FSRS Rust project](https://github.com/open-spaced-repetition/fsrs-rs)

**Bounded implementation work (R-07):** Finalize FSRS dependency/rating
integration, event/schema details, goal/streak time-zone handling, backup validation,
retention implementation, and online identity recovery. Apply R-04/R-06; automatic
cross-device sync is not part of this desktop release. Test import/retry/reset
behavior explicitly rather than merging histories as if sync existed.

**Required implementation/release evidence:** Progress data model, update rules, review
algorithm contract, data lifecycle, and corruption/recovery scenarios.

## 8. Audio and listening

**Confirmed:** Audio/listening is within the requested specification and must
work offline as part of the agreed official-launch feature set. Use pre-generated
downloadable standard-Italian audio packs, normal/slow playback, unlimited replay,
and validate voice/model redistribution rights before selection.

**Confirmed audio contract:** Generate reviewed utterances
during content production; ship audio by stable utterance ID/content version.
Package selected-level packs and offer installation of the complete course.
Provide visible installed/download/error state and retry interrupted downloads.
Play from local files with no runtime TTS service dependency. Slow playback
preserves pitch; listening transcripts are available after answering or as an
explicit hint. Missing/corrupt media offers repair and a clear explanation, not
a silently substituted exercise or automatic wrong answer. Test intelligibility
and text/audio consistency on real speakers/headphones and weak hardware.

**Bounded content/technical work (R-07):** Choose a redistributable generated
voice, codec/compression, slow-playback factor, pack boundaries and coverage,
output-device/error handling, and distinguishable dialogue voices if needed for
comprehension. Review naturalness/intelligibility and text/audio match before
selection; disclose pack sizes and test playback entirely offline.

**Required implementation/release evidence:** Audio source and license decisions, coverage
targets, packaging strategy, playback rules, and native-speaker quality checks.

## 9. Speaking practice

**Confirmed:** Speaking practice is within the agreed official-launch scope;
practice outside AI conversations must work offline; word-level feedback only.

**Deferred:** Detailed sound/stress/pronunciation scoring beyond word recognition.
No commitment or schedule for adding it later has been made.

**Confirmed feedback contract:** Display the recognized
transcript and highlight expected words as matched, missing, or different.
Permit re-recording without consuming lives. Explain uncertainty when confidence
is insufficient rather than marking the learner's pronunciation as wrong based
only on recognition. Exact matching/normalization and low-confidence thresholds
remain to be selected and validated.

**Bounded validation work (R-07):** Compare speech candidates on target hosts,
choose model/version and word-matching confidence thresholds, and author read-aloud
variants. Validate error tolerance, recording states, permission denial, missing
models, low-confidence feedback, and accessible alternatives. Keep raw recording
retention consistent with R-06 and keep recognition independent of online AI.

**Required implementation/release evidence:** Speaking interaction contract, selected
recognition/assessment approach, validated quality criteria, and privacy rules.

## 10. AI tutoring

**Confirmed:** AI conversations belong in the official-launch feature set and
may depend on internet access; shared leaderboards are a separate approved network
exception. This does not select an AI provider or model.
Any other tutor features, if selected, must work offline.
Conversations support both text and voice and cannot require payment from users.
The owner can supply an always-on computer later; server operation is not required
to begin application development. Server outages affect AI features only.

**Confirmed hosting approach:** Use an open-weight model on an operator-run
server, behind a conversation service; share that service between text and voice.
Keep heavy conversation inference off the base desktop client. See R-03.

**Bounded implementation/release work (R-07):** Choose host/model/voice after
hardware and rights are known; validate natural Italian, corrections, level
adherence, quality thresholds, capacity, and response timing. Operate under R-06's
context/retention rules. Actual server operation and public service capacity may
wait, but successful real text/voice AI validation is required before official
launch. No offline conversation mode is required by the current specification.

**Required implementation/release evidence:** Approved tutor scope, feasibility evidence,
model/provider decision, input/output contracts, evaluation set, and fallbacks.

### R-06: Confirmed conversation, privacy, and retention contract

**Confirmed:**

- Tutor scope: Level-adjusted open chat and practical role-play; Italian dialogue
  with concise English corrections/explanations on request or via an explicit
  correction setting. The tutor can reference approved lesson summaries sent with
  the current conversation. It does not change objective exercise answers,
  completed lessons, XP, or mastery automatically.
- Text/voice: Share the same conversation state. Prefer local transcription if
  the validated speech engine can handle it; show/edit the transcript before send.
  AI reply audio can be generated on the server using a separately validated
  voice. Voice failure offers text mode if text AI is still available.
- Service states: Not configured, connecting, ready, generating, unavailable,
  cancelled, and error. Make requests cancellable and time-bounded; allow an
  explicit retry, preserve the draft, and return to offline study at any point.
- Context: Send only the current conversation, selected level/scenario, and
  explicitly relevant approved lesson context. Do not upload the full local
  learner database, unrelated files, or complete exercise history.
- History: Save conversation text locally with a delete-history control; no raw
  microphone recordings kept after transcription by default. Backups include
  conversation text only with an explicit export option. Document that context
  is transmitted to the configured AI operator when online conversations are used.
- Speech upload: Do not upload raw audio by default if local transcription is
  sufficient. If server transcription is later selected, obtain separate approval
  for the visible disclosure, consent, and server retention policy.
- Server records: Do not retain prompt/transcript/audio bodies by default. Keep
  only operational metadata needed for service operation, with documented limits.
  Never embed a shared administrative/model-provider secret in the public client.
- Analytics: No third-party analytics or automatic crash-report uploads initially.
  Local diagnostics can be exported deliberately with sensitive text removed.
- Leaderboard privacy: Upload only opted-in identity/nickname, qualifying XP event
  IDs/amounts/content references, and required timestamps. Do not publish answers,
  tutor messages, or audio. Queue while opted in; stopping participation cancels
  unsent contributions. Backfilling pre-opt-in history requires separate consent.
  Specify server removal/recovery behavior before enabling public participation.
- Backup/reset: Export progress/settings with media-pack references; validate and
  back up before import replacement. Do not duplicate online identity across two
  concurrently active installs as if automatic multi-device sync were supported.
  Offer separate local reset and leaderboard removal; local deletion while
  offline cannot claim remote data has already been deleted.

The chosen AI model must be tested for natural Italian, educational corrections,
level adherence, and licensing on the actual host. Model weights, voice, hosting
hardware, capacity, and exact timeout/metadata-retention limits are bounded
technical/release decisions under the accepted R-07 process.

## 11. Testing and quality assurance

**Confirmed acceptance obligations:**

- With initial assets installed and network access blocked, all agreed learning
  features outside AI conversations remain usable, including audio and offline
  speaking exercises; shared leaderboards are also unavailable offline. Detailed
  scenarios depend on the feature inventory.
- The learner is offered Italian-level selection. Whether a placement test is
  taken is optional; offer beginner and A1–C2, permit level changes, preserve
  progress, and keep earlier topics accessible.
- Instructions use English; the application contains no ads.
- No lives/hearts mechanic limits practice, and no required charge gates access
  to the agreed learning features, including text and voice AI conversations.
- With the AI server absent, failing, or timing out, the app starts and offline
  study works; neither text nor voice AI causes a whole-app failure.
- Lessons completed offline update local progress and later contribute to the
  leaderboard when connectivity/service availability returns.
- Offline speaking feedback stays at word-recognition level; it does not claim
  detailed pronunciation assessment.
- Each selected motivation/navigation feature has acceptance coverage once its
  detailed behavior is agreed.
- All agreed exercise types work offline, provide immediate explanation/retry,
  and cover both translation directions where relevant.
- UI supports the approved themes/accessibility features; original reviewed
  content and downloaded standard-Italian audio are used.
- Offline study requires no login; one profile can export/import progress;
  leaderboard participation is optional; initial desktop builds do not depend on
  automatic cross-device sync.

### Confirmed test and release gates

**Confirmed:**

- Rust unit/property tests for answer normalization, accepted variants, progress
  transactions, scoring caps, time zones, scheduling, pack validation, migrations,
  and idempotent outbox handling. Assert behavior on ambiguous/edge cases.
- UI/component tests for exercise states, keyboard use, screen-reader output,
  enlarged text, both themes, reduced motion, and service/download error states.
- Integration tests against temporary SQLite data and a controllable local test
  server for offline work, timeout, partial response, duplicate acknowledgements,
  corrupt packs, interrupted imports, and recovery after a crash.
- Desktop end-to-end tests for clean install, onboarding, optional placement,
  lesson/review, speaking permissions, exit/resume, export/import, queued
  leaderboard sync, server-off AI, and normal text/voice conversations when hosted.
- Use free test tooling: Rust's standard test tooling, frontend test tools chosen
  for Svelte, and WebdriverIO's embedded Tauri driver for native UI where validated.
  The current Tauri documentation supports an embedded driver on macOS/Windows;
  it must be restricted to test builds. Browser-only UI tests cannot replace
  native audio/permissions/installer checks.
  [Tauri WebDriver documentation](https://v2.tauri.app/develop/tests/webdriver/)
- Validate accepted OS/architectures on actual hosts or suitable VMs. Build success
  alone does not establish minimum-OS behavior, speech speed, or accessibility.
- Content QA: Schema/assets/licensing pass; manual linguistic/editorial review;
  reviewed answer variants; listening-audio consistency; per-level objective
  coverage; advanced tasks tested against their rubrics.
- Speech QA: A consented Italian learner-speech fixture set, quiet/noisy input,
  varied microphones and accents, low-confidence/unintelligible input, latency,
  memory, and word-feedback consistency. Select measurable thresholds before
  choosing the engine; word recognition is not pronunciation certification.
- AI QA: Versioned Italian role-play/correction scenarios, level adherence,
  naturalness, factual/grammar error checks, cancellation/timeouts, unavailable
  model, and separation from offline grading. Manual checks complement automated
  fixtures; do not claim that a prompt guarantees correctness.
- Release blocker: Any confirmed acceptance failure; data-loss/migration issue;
  broken offline path; duplicate leaderboard credit; unusable supported installer;
  missing content/media rights; uncovered required course objectives; unmet agreed
  performance/accessibility gates. No tests have run yet: there is no app code.

### Acceptance traceability

- AC-01 (C-02–03, C-21, C-28–29): Install required assets, disable network and AI
  server, then start/restart the app and complete lesson, review, audio, and offline
  speaking. Local progress persists; no login or payment is required.
- AC-02 (C-12, C-14, C-26, C-32): Choose beginner/A1–C2, skip optional placement,
  change level, and revisit earlier content; no progress is reset. Checkpoints
  and self-review never claim external CEFR certification.
- AC-03 (C-23–25, C-32): Each required exercise/strand is represented in the approved
  course map; prompts/explanations/variants/media are reviewed. First-attempt
  results and hint/substitution evidence remain intact through retries/resume.
- AC-04 (C-17, C-31): Verify XP amounts/day caps, goal edits, study-zone/day
  boundaries, current/best streak, achievement de-duplication, reminders, and FSRS
  updates. Unlimited practice is possible after earning caps are reached.
- AC-05 (C-20, C-29, C-33): Opt in; study offline; kill/restart; reconnect; retry a
  submission with a lost acknowledgement. Count each qualifying contribution once.
  Opt-out cancels unsent contributions; no answers/audio/tutor text are uploaded.
- AC-06 (C-16, C-21, C-33): With a real configured server, complete text and voice
  chat, level-adjusted role-play, correction, transcript editing, and cancellation.
  Disconnect mid-turn and verify recoverable draft/offline study; no conversation
  XP is awarded. Test local text-history deletion and recording non-retention.
- AC-07 (C-22, C-27–28, C-32): Verify normal/slow replay, word-level feedback,
  microphone denial, low confidence, missing media/model, accessible substitutions,
  keyboard/screen reader/text scaling/themes/reduced motion.
- AC-08 (C-19, C-29, C-34): Export/import and migrate with a pre-change backup;
  inject corrupt archives/failed migrations. Preserve the valid prior state and
  avoid silently combining histories or duplicating unsupported online identities.
- AC-09 (C-05, C-18, C-34): Verify clean install and real study on the accepted
  OS/architecture targets; benchmark all R-07 package/memory/startup/latency budgets.
- AC-10 (C-07–10, C-15, C-25, C-35–36): Before official release, all required
  content/features pass their gates, rights/signing/hosting decisions are resolved,
  artifacts and source are traceable to the user's GitHub revision, and development
  began only after final specification approval.

**Confirmed additional acceptance scenarios:** Kill the app
after a queued contribution is saved; restart; reconnect; verify delivery. Drop
the server acknowledgement and retry; verify XP is counted once. Disconnect
mid-conversation; verify the learner can exit and study offline immediately.

**Bounded implementation/validation work (R-07):** Select test packages and CI,
record reference hardware, build versioned speech/AI/content fixtures, and set
engine-specific quality thresholds before selecting models. All confirmed release
gates apply. This specification records intended checks; no application tests or
performance measurements have been performed yet.

**Required implementation/release evidence:** Traceability from confirmed requirements to
tests, representative fixtures, test environments, and explicit release gates.

## 12. Packaging, deployment, and maintenance

**Confirmed:** Desktop support targets and stack are accepted. AI service
operation can wait; its absence must be handled by the application. Official
launch must include the agreed AI capabilities, but continuous service operation
and public capacity commitments remain to be defined.

### R-05: Confirmed distribution and maintenance plan

**Confirmed:** Publish source and desktop releases through
[misc9115-boop/lang](https://github.com/misc9115-boop/lang); macOS DMG and Windows
NSIS setup executable; document building from source. Use the existing public
repository; no new GitHub account or repository is needed.
Use platform-native CI builders with pinned dependencies/lockfiles, artifact
checksums, license notices, test results, and tagged release notes. Public hosting
is a future deployment action, not authorized by drafting this specification.

Use a small installer with initial asset/runtime downloads where necessary;
offer downloadable packs for offline use after installation. Keep content-pack
versions independent of app versions. Check/install app updates explicitly with
user consent, with an offline-safe "check later" path. Verify pack/update integrity
and compatibility; preserve the previous valid install until a replacement is
ready. Back up learner data before schema migration; a failed migration must not
silently overwrite progress. Define supported downgrade behavior per schema.

For public macOS builds, recommend Developer ID signing/notarization; Windows
signing expectations and certificate funding need a separate decision. Personal
development builds can precede paid distribution credentials. Signing/notarization
is a deployment concern; do not present unsigned personal builds as equivalent to
signed public releases.
[Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/)

Deploy leaderboard and AI services independently or as separately failing
components on the owner's host. Use authenticated contributions, encrypted public
transport, bounded requests, service health reporting, persistent leaderboard
storage/backups, and configurable endpoints. Package an operator guide for
hardware/model setup, start/stop, retention/deletion, backups, and upgrades.
Local development can use deterministic test endpoints; users never see simulated
AI/standings as live service results. Whether a public leaderboard can run while
AI is off depends on the eventual host arrangement and needs explicit handling.

**Confirmed licenses:** MIT for original code and CC BY 4.0 for
original curriculum/artwork/audio when the creator/voice rights permit. MIT allows
reuse, modification, and redistribution including proprietary/commercial forks;
GPLv3 was considered but not selected; it was the alternative for distributed
derivative code to remain under copyleft terms. Original-content licenses do not override third-party licenses.
Neither license requires third-party forks to charge zero. The project
itself remains free by the confirmed product requirement.
[MIT license](https://opensource.org/license/mit),
[GPLv3](https://www.gnu.org/licenses/gpl.en.html),
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)

### R-07: Confirmed resource budgets and technical decision boundaries

**Confirmed:** Measure on a documented reference machine
representing the accepted 4 GB/no-dedicated-GPU target, with release builds and
representative installed content. These are agreed acceptance budgets, not
existing performance measurements:

- Base compressed desktop package: At most 75 MiB, excluding separately identified
  WebView2 installation, curriculum/audio packs, and downloadable speech models.
  Still report complete install/download size so exclusions cannot hide costs.
- Whole-app memory, including attributable webview/helper processes: At most
  300 MiB idle, 512 MiB for ordinary lessons, and 1 GiB peak during offline speech.
- Startup to usable installed course: At most 3 seconds under the defined warm
  filesystem-cache benchmark; record cold-start results separately.
- Ordinary answer/control feedback: p95 at most 150 ms, excluding explicit audio
  capture, downloads, and model work. Local study never waits for an online server.
- Speech transcription after a 10-second recording: Target at most 5 seconds on
  the reference host. Validate quality and latency together before selecting an
  engine; report poor confidence rather than silently using a remote service.
- Content/audio/model pack sizes: Report before download and retain user control
  over installed packs. Set per-pack budgets during authoring, documenting full
  course size; no invented full-course size guarantee before assets exist.

**Confirmed delegated decisions, bounded by this specification:**

- Choose exact dependency versions, build/test tools, file organization, schema
  fields/indexes, IPC types, outbox protocol, and timeouts using the confirmed
  architecture, confirmed behaviors, and test/performance gates.
- Choose the offline speech engine after comparison on a versioned, consented
  learner-speech evaluation set; choose a redistributable standard-Italian voice
  after audio review. Document actual quality metrics and selected thresholds.
- Select/size the self-hosted model only after host hardware is known; keep it
  replaceable behind a documented adapter. Server operation may wait while the
  desktop client and deterministic test fixture are developed. Do not treat an
  unavailable server as successful validation of AI quality or launch readiness.
- Determine exact lesson counts and item sequences while authoring against the
  approved objective map, 5–10-minute pacing, naturalness, and manual review gates.
  Do not omit levels/strands or replace advanced production with recognition alone.
- Develop in internal milestones: offline study/progress foundation; complete
  exercise/media/review flows; online leaderboard/conversation integration;
  full beginner–C2 content and public-release QA. These are development stages,
  not permission to omit agreed features from official launch.
- Before public release, resolve signing credentials/funding, service capacity,
  operator responsibilities, model/media rights, and actual hardware evidence.
  These are release gates, not assumed resources or promises.

If a candidate cannot satisfy an agreed budget or quality gate, record evidence
and ask for a revised choice instead of quietly relaxing the specification.
Brand name and final artwork are explicitly deferred editorial choices. The
overall specification is approved; changes to agreed behavior require clarification.

**Unresolved product decisions:** None blocking the specification. GitHub
repository, licenses, installers, and explicit update consent are confirmed.
Public-release resources below remain explicit deferred release work.

**Deferred release work (R-07):** Actual signing/notarization credentials/funding,
service hardware/capacity/operations, support expectations, final artifact builds,
and verified third-party rights. These must be resolved and evidenced before the
corresponding public release. iPhone distribution remains later work.

**Required implementation/release evidence:** Platform-specific build/release procedures,
cost and licensing review, clean-machine installation checks, and update policy.

## 13. Accepted recommendations, tradeoffs, and candidate evidence

### R-01: Use CEFR levels to define the learning endpoint and starting levels

**Accepted with a different endpoint:** The user approved CEFR and selected C2
instead of the proposed C1. CEFR describes skill proficiency across A1–C2. C2
includes broad comprehension, synthesizing sources, and nuanced expression.
The course targets C2; the accepted assessment boundary makes no certification
claim and separates objective results from advanced self-assessment.

Source: [Council of Europe CEFR global scale](https://www.coe.int/en/web/common-european-framework-reference-languages/table-1-%20cefr-3.3-common-reference-levels-global-scale).

### R-02: Lightweight cross-platform application stack

**Accepted:** Tauri 2 + Svelte/TypeScript + Rust + SQLite. Architecture/asset strategy are accepted; exact engines and pack contents are
selected and validated under R-07.

- Tauri uses the system webview and targets desktop/mobile platforms. This
  supports a small application shell and a route to iPhone support. It does not
  establish a tested minimum OS or guarantee every plugin works on iPhone.
  [Tauri overview](https://v2.tauri.app/start/)
- Svelte compiles UI components into JavaScript; TypeScript supports explicit
  exercise/content contracts. A packaged client UI needs no UI server at runtime.
  [Svelte overview](https://svelte.dev/docs/svelte/overview)
- Rust handles learning rules, storage access, and platform adapters behind a
  typed command boundary. Keep model inference away from the UI event loop.
  This responsibility split is accepted; performance must still be benchmarked.
- SQLite provides in-process local persistence without a separate database
  server. [SQLite architecture](https://www.sqlite.org/serverless.html)

**Tradeoffs:** Native speech integrations and Rust add build complexity; system
webviews require platform-specific UI testing; assets may dominate storage even
when the shell is small. Future iPhone packaging and native capabilities require
their own validation. Exact dependency versions will be selected after approval.

**Alternative approaches considered:** Separate platform-native apps maximize
native control but duplicate UI work; a fully native shared renderer avoids
webview differences but needs a different frontend ecosystem. Neither alternative
has been benchmarked for this product. Tauri is recommended because the confirmed
priorities combine desktop/mobile reuse, lightweight packaging, and offline logic.

**Accepted support targets — require implementation validation:**

- macOS 12+ on Intel x86-64 and Apple Silicon ARM64.
- Windows 10/11 on x86-64; Windows 11 on ARM64.
- Aim for 4 GB system RAM and no dedicated GPU requirement for offline lessons,
  audio, and the chosen offline speaking exercises.
- Linux, 32-bit processors, and older OS releases have not been added to the
  initially selected platform scope. The user's "yes" accepted the presented
  support targets; it did not separately request Linux.
- OS versions and speech performance are accepted targets, not measured support
  claims. Validate the complete dependency chain on representative hosts.

**Confirmed footprint strategy:** Keep curriculum/media/speech packs versioned and
separable; provide a way to install all required assets for complete offline
study. Do not load whole courses or speech models unnecessarily. Track shell,
webview/runtime, curriculum, audio, and models separately in size measurements.
App-memory/startup/base-package budgets are confirmed in R-07; asset pack sizes
are determined/disclosed during authoring.

Windows needs WebView2; installation can download it or include its offline
installer, with different package sizes. Initial downloads are allowed, so an
internet-assisted small installer is a candidate rather than an approved choice.
[Tauri Windows distribution](https://v2.tauri.app/distribute/windows-installer/)

**Offline speech candidates — evaluation required:** Vosk lists a small Italian
model at 48 MB under Apache 2.0; whisper.cpp supports desktop/mobile and CPU
inference. Compare Italian learner-speech accuracy, licensing, installed size,
peak memory, and latency before selecting one. A transcript match cannot on its
own establish accurate pronunciation scoring.
[Vosk models](https://alphacephei.com/vosk/models),
[whisper.cpp](https://github.com/ggml-org/whisper.cpp)

### R-03: Free-to-user text and voice AI conversations

**Confirmed approach with deferred deployment:** The owner can support an always-on computer but is unsure
about operating it now. The application must support an unavailable AI service.
Use an operator-hosted open-weight model so
ordinary clients do not need a large language model or a paid API key. Text and
voice use the same conversation service; use local speech where validated or
server speech within the AI conversation's permitted network boundary.

llama.cpp is a possible server inference engine with CPU/GPU support; its engine
license and the chosen model's license are distinct. No model has been selected
or validated for Italian tutoring.
[llama.cpp server documentation](https://github.com/ggml-org/llama.cpp/tree/master/tools/server)

This approach keeps the user-facing price at zero; it does not make compute,
hosting, electricity, or maintenance cost zero. The operator's hardware/budget
and expected capacity must be confirmed. Donations may contribute but are not
guaranteed funding. Do not base launch availability on an unverified unlimited
free provider tier.

Alternatives: An existing owner-operated machine may serve initial personal use;
local conversation inference avoids a shared hosting bill but increases client
hardware/storage needs; an optional community-hosted endpoint needs a reliable
operator and service policy. None can yet be promised for every computer at
unlimited capacity. Failure isolation, outage UX, and privacy are confirmed. Model, host hardware,
quality thresholds, capacity, and ongoing funding follow R-07 as bounded
implementation/public-release work.

## 14. Decision and approval record

Entries summarize answers at each discovery round; later answers supersede
earlier open questions. Current requirements are recorded in sections 1–12.

### First discovery round — answers recorded

- D-01, partially resolved: Initial learner is the owner; English instructions;
  selectable Italian level; fluency goal. Level scale and endpoint remain open.
  Affects sections 2, 4, 5, and 11.
- D-02, partially resolved: Open source, no ads, donations acceptable in principle.
  Online AI funding and licenses remain open. Affects sections 2, 10, and 12.
- D-03, partially resolved: All learning features except AI conversations offline;
  initial downloads accepted. Hardware/storage limits remain open. Affects
  sections 3 and 8–12.
- D-04, partially resolved: All agreed features at official launch; no reduced
  official-launch scope selected. Specific Duolingo-inspired features remain open.
  Affects sections 2, 4, 6–10, and 12.

### Second discovery round — answers recorded

- D-05, resolved for framework/endpoint: CEFR approved; C2 selected. Assessment
  details remain open. Affects sections 2, 4, 5, 7, and 11.
- D-06, partially resolved: Both text and voice AI conversations; choose an
  approach that does not charge users. Host/funding remain open. Affects 3, 9–12.
- D-07, resolved for listed feature inclusion: Include every presented feature
  except lives. Offline/shared-leaderboard boundary remains open. Affects 3, 4,
  7, and 11.
- D-08, partially resolved: Broadest possible compatibility and lightweight
  application; requested a stack recommendation. R-02 is proposed, not accepted.
  Affects sections 3, 8–12.

### Third discovery round — answers recorded

- D-09, resolved: User accepted the presented stack/support targets. Linux or
  older/32-bit platforms were not separately requested. Affects 3 and 8–12.
- D-10, partially resolved: Owner can supply an always-on computer; timing is
  uncertain. Include AI and isolate server outages to AI features. Exact service
  hardware/model/deployment and public capacity remain open. Affects 3, 10–12.
- D-11, resolved for network behavior: Shared online leaderboard unavailable
  offline; offline lessons/progress continue and upload contributions on return.
  Identity, leaderboard rules, and server hosting remain open. Affects 3, 7, 11–12.
- D-12, resolved for feedback granularity: Word-level only for now. Detailed
  pronunciation scoring excluded from current scope. Affects 6, 9, and 11.

### Fourth discovery round — answers recorded

- D-13, resolved for course/exercise inclusion and authoring approach: All presented
  strands and exercise types; original AI-drafted, manually reviewed material;
  close to established apps in realism/quality. Detailed map, reviewer capacity,
  C2 assessment criteria, and inventory remain open. Affects 5, 6, 8, 11.
- D-14, resolved: Beginner/A1–C2; freely changeable level without resetting progress;
  earlier topics available; optional placement; 5–10-minute lessons; immediate
  explanation and unlimited retries. Affects 4–7 and 11.
- D-15, resolved for direction: Playful/colorful original style; accepted sidebar,
  themes, keyboard, screen-reader, text-scaling, reduced motion. Affects 4 and 11.
- D-16, resolved for approach: Pre-generated downloadable standard-Italian audio,
  normal/slow playback, unlimited replay, license checks before voice selection.
  Affects 3, 8, and 11–12.
- D-17, resolved for profile/sync scope: One local profile, no offline login,
  backups, optional nickname leaderboard; automatic device sync deferred.
  Affects 3, 7, and 11–12.

### Fifth discovery round — answers recorded

- D-18, resolved: R-04 progress/scoring/leaderboard rules accepted; sections 5–6
  answer/evidence/assessment/accessibility contracts accepted.
- D-19, resolved: R-06 AI role/correction, privacy, retention, service behavior,
  and limited opt-in leaderboard data accepted.
- D-20, partially resolved: User wants the project in their GitHub. They asked
  what the distribution/license proposal means, so R-05 is not accepted yet.
  Explain repository versus release assets and license permissions; obtain
  repository destination/visibility and license/distribution choices.
- D-21, resolved: R-07 budgets and bounded technical/content decisions accepted,
  along with remaining architecture, screen, audio, synchronization, backup, and
  test proposals. Brand name/artwork and actual public host details may be deferred.

### GitHub/license clarification — answers recorded

- D-22, resolved: The user answered yes to the explained recommended license set:
  MIT original code and CC BY 4.0 eligible original content.
- D-23, resolved: The user accepted GitHub Releases with explicitly approved
  app updates. R-05 is confirmed; signing/resources remain release gates.
- D-24, resolved: The user supplied `https://github.com/misc9115-boop/lang.git`.
  Repository metadata confirms public visibility, default branch `main`, and
  an empty repository at inspection. Use that existing destination.

All other remaining detailed selections are bounded implementation/authoring or
explicit public-release gates under R-07. They are not unresolved product choices
that authorize silent feature changes.

### Specification approval gate

The completed specification has received final user approval. Review checks:

- Required launch behavior and explicitly deferred features are recorded.
- Necessary product and technical decisions are resolved or explicitly accepted
  by the user as bounded implementation choices.
- Architecture, curriculum, UX, data, audio, speech, AI, testing, and deployment
  sections contain concrete contracts and acceptance criteria.
- Feasibility and cost implications are documented against the free/offline goals.
- Final approval was granted after the completed specification was presented;
  the user replied, "yes push to the repo". This supersedes the earlier pending
  approval status without changing the confirmed product requirements.

Approval: **Granted by the user: "yes push to the repo".**

This approval establishes the development baseline. No application code was
written before approval. The subsequent "continue" instruction authorized
completion of the interrupted approval-recording and repository update.

Implementation changes must remain traceable to this document.
New product choices require clarification, and accepted changes must be recorded
here so the specification continues to reflect the agreed behavior.
