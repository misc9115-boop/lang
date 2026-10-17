# Offline review scheduling

This implements local FSRS scheduling, lesson-seeded histories and saved review
sessions for SPEC.md section 7 / R-04. The desktop Review screen previews item
counts, resumes saved interactions and awards completion credit. Schema/backup
version 3 adds sessions and the review credit ledger; existing profiles upgrade
only after a [validated recovery archive](BACKUPS.md#schema-1-or-2-to-3-migration).
No independently reviewed content is bundled. Synthetic authority used in tests
is confined to disposable fixtures and is not a course approval.

## Engine choice

The exact dependency is `fsrs = 6.6.2`, default features only, using its FSRS-6
default parameters. The published crate and its checksum are pinned in Cargo.lock;
the existing Rust 1.93.1 toolchain compiles it. We call only `FSRS::next_states`.
There is no parameter training, optimizer, simulation, network request or random
interval fuzz in this path. We use the upstream implementation rather than a
locally reconstructed formula. The BSD-3-Clause notice is retained in
[licenses/fsrs-BSD-3-Clause.txt](../licenses/fsrs-BSD-3-Clause.txt).

Sources: [upstream scheduling documentation](https://docs.rs/fsrs/6.6.2/fsrs/struct.FSRS.html#method.next_states),
[published dependency/feature metadata](https://docs.rs/crate/fsrs/6.6.2/features),
[upstream project](https://github.com/open-spaced-repetition/fsrs-rs).

The lock adds 21 packages, including FSRS, ndarray, Rayon and random-number
dependencies. Upstream's Burn training/test dependencies are not included in our
dependency graph. Calling the scheduling API does not train a model. Dependency
count is not a memory, speed or installer-size measurement; the R-07 whole-app
budgets and all target architectures still need measurement before release.
The `priority-queue` dependency offers LGPL-3.0-or-later or MPL-2.0; we select
MPL-2.0 and retain its text and source link in [dependency notices](../licenses/README.md).

## Rating mapping 1

| Observed result                                                 | Rating |
| --------------------------------------------------------------- | ------ |
| Incorrect at the end, or solution revealed                      | Again  |
| Correct after a hint or correction of an incorrect first answer | Hard   |
| Independent correct answer                                      | Good   |
| Independent correct answer plus explicit learner Easy action    | Easy   |

First-attempt correctness remains in the observation after a correction. An Easy
action cannot override a failed, revealed or assisted result. Contradictory
evidence is rejected. Typing speed is not an input. The session engine derives evidence
from stored interactions; the renderer can request explicit Easy but cannot
supply correctness, another rating or an FSRS history.

Desired retention is fixed at **0.9**. This is an algorithm input, not a guarantee
of learning or CEFR proficiency. The schedule identifies both the engine/defaults
(`fsrs6-fsrs-rs-6.6.2-default-v1`) and rating mapping version. A dependency/default
change requires a new identifier and explicit migration.

## Identity, time and replay

`ReviewKey` contains curriculum ID, target kind, target ID and tested skill.
Vocabulary senses, grammar targets and objectives remain distinct; reading,
writing, listening and speaking states cannot be combined. Pack version is not
part of identity. The content adapter resolves exact exercise mappings
against currently installed, independently reviewed content. The kernel does
not establish that an item exists, grant editorial trust or invent targets.

`ReviewHistory` stores observations, not imported stability/difficulty. Before use,
it validates IDs, versions, evidence, chronological order and request/session
uniqueness, then deterministically replays through FSRS. JSON unknown fields and
unsupported versions fail. Limits are 4 MiB JSON and 10,000 observations per
history. Hitting a limit fails without changing history; archival/compaction needs
an explicit later format rather than silently dropping observations.

One item gets at most one rating in a session. An exact retry returns the current
schedule; retrying an older request cannot roll memory state back. A reused
request with changed fields or another rating in the same session fails. The host
must preserve the original request ID and observation timestamp for a retry.
Mutation happens only after validation and calculation succeed.

Elapsed days are whole 24-hour UTC durations since the prior rating, including
zero for same-day practice. The due interval follows upstream's day-granular
example: round to nearest day, minimum one day. Minute-level relearning steps are
not implemented. Due timestamps remain UTC; study-zone changes and DST do not
rewrite history. Clock rollback and calendar overflow fail without recording.

`select_reviews` prefers overdue/due items, oldest due first, with stable identity
as tie-breaker. Explicit practice can include new and future-due items even when
nothing is due. Duplicate keys fail, and each selection input is bounded at
10,000 candidates; this is a batch limit, not a full-course inventory limit.
The host must supply eligible reviewed candidates and manage larger queries.

## Content integration

Current persistence binds each observation to an exact reviewed pack reference,
canonical lesson digest, exercise ID and objective/tested-skill key. The host
derives these bindings from verified pack mappings. Unreviewed lessons never seed
histories. The first resolved occurrence of a key in a lesson session supplies
its single observation; later exercises testing that key cannot inflate ratings.
Each seed is written in the same transaction as advancing the lesson, including
any completion/lesson XP/achievement changes. A failed transaction rolls all back.
Several cached packs carrying identical lesson bytes are resolved deterministically
by pack ID/version. No renderer command accepts arbitrary ratings or histories.

Histories include all original evidence plus one source per observation. Stored
review JSON is bounded at 8 MiB per item; whole-backup bounds also apply. Read APIs
retain history after trust is revoked, while using/exporting/restoring it requires
matching reviewed sources. Pack removal is blocked if a review source needs that
exact version. Backup validation also matches observations to saved graded items,
final attempts and timestamps; it does not trust archive approval labels. These
lesson-seeded observations cannot claim explicit Easy self-ratings.

The currently authored format maps objectives and tested skills precisely at
exercise level, so those keys can be seeded. Vocabulary/grammar target lists are
only lesson-level: do not infer which exercise tests each target. Those mappings
must be authored/versioned before target-specific review can be enabled.

## Saved sessions and completion

The host selects up to 10 distinct introduced objective/skill keys, preferring
oldest due items. This is an implementation batch size, not a course completion
rule. Explicit practice includes future-due items. Unintroduced, inactive,
unavailable or untrusted content cannot be selected. Historical sources remain
required for validating retained observations. Vocabulary/grammar exercise
mappings still require authoring and a versioned content contract.

Review sessions have their own table and exact source/lesson snapshots; they do
not fabricate mixed-content lessons. Each item retains its draft, ordered graded
attempts, hint use and final observation. Session item UUIDs distinguish repeated
exercise content testing different skills. A wrong answer requires correction or
explicit solution review. First-attempt evidence survives retries. Resolved items
cannot be edited, assisted answers cannot claim Easy, and only an explicit Easy
checkbox changes an independent Good result. Typing time is never used.

Continue writes one FSRS observation per key/session and advances atomically;
repeated or stale Continue requests cannot skip items. Save & exit retains the
session. End review marks it aborted, awards no XP and retains ratings already
committed for earlier items. Ending remains possible if content becomes
unavailable; using/exporting that content still requires repair. One lesson or
review can be active at a time. Attempts are bounded at 10,000 per item and saved
session JSON at 8 MiB; backup record/byte limits apply without truncation.

Finishing all items commits the final rating, session completion, study day,
5-XP credit and achievements together. A unique review credit per saved local
calendar date caps review XP at once per day; further sessions still complete.
Lesson and review credits both count toward goals, streaks and 100 XP. First
review is a separate achievement. Changing the IANA study zone does not rewrite
past dates, and goal edits change only the current/future day. A failed award or
FSRS write rolls the complete transaction back. Aborted/opened sessions never
qualify a study day. Completion/XP are not a mastery or CEFR assessment.

Version-3 archives contain sessions, attempts, drafts and credit. Validation
regrades saved answers, checks exact host content authority, chronology, position,
unique keys/requests, rating evidence, both directions of the session/history
link, credit caps and achievements in an isolated candidate. Legacy restores
replace these new collections with empty ones, after verified version-3 recovery.

## Validation

Unit tests cover published upstream initial states for all four ratings,
same-day/later/lapsed replay, serialized history reload, exact/old retries,
session deduplication, separate tested skills, malformed evidence/IDs/versions,
input limits, clock/calendar failures, UTC offsets, due ordering and practice with
no due items. Existing learner/backup/content tests continue to run. CI exercises
the shared core on Windows and macOS; results are recorded with the pull request.

Session tests cover wrong/hinted/revealed/independent/Easy results, exact/changed
request retries, due-only versus explicit practice, SQLite reopen and backup
resume, abort, award rollback, daily credit caps, streak/achievement evidence and
checksummed forged session/history/credit data. The browser suite has a separate
`--review` mode using an explicitly synthetic pack in disposable storage; it
checks correction, resume, typed drafts, Easy, repeated completion/abort and
validated export. Production content trust is unchanged. Native startup continues
to test a real schema-1 upgrade, recovery selection and process restart in CI.
