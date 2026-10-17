# Backup format and replacement contract

Implementation decision under SPEC.md §3 / AC-08, 2026-10-14. This implements
the existing requirement; it does not add sync or multiple learner profiles.

- File extension: `.langbackup`. Format versions 1, 2 and 3 are UTF-8 JSON archives with
  a format identifier, archive/schema versions, creation timestamp, development
  versus learner mode, a typed learner-data payload, and its SHA-256 checksum.
  The checksum detects accidental corruption; it is not encryption, a signature,
  or protection against deliberate editing.
- Export includes settings, sessions/snapshots, saved answers, attempts, XP,
  study dates/goals, achievements and explicitly entered local answer reports.
  Audio/model files and the content catalogue are excluded. Installed text-pack
  references contain typed pack ID, version and exact manifest SHA-256 records.
  Matching versions must already be installed and verified before preview/restore.
  Restoring does not download, install or activate packs. Existing version-1
  archives with empty references remain supported; older builds reject nonempty
  references they do not support rather than silently ignoring them.
  Tutor history and online credentials are not implemented and are not exported.
  Adding either later requires an explicit archive schema/privacy decision.
- Current exports use archive/schema version 3, including `reviewRecords`,
  `reviewSessions` and `reviewXp`, even when empty. Session records preserve drafts,
  graded attempts, hints, sources, status and observations. Version-1/2 archives
  remain importable; absent collections replace the corresponding live records
  with empty sets. Version-3 pre-import recovery preserves the replaced data.
  Older readers reject newer archives rather than silently dropping records.
- Review records contain replayable FSRS histories and an exact pack/lesson/
  exercise source for each observation. Validation replays the pinned engine,
  checks source authority against currently verified historical pack versions,
  and compares each observation with its resolved saved item and final attempt.
  Review labels cannot create authority. Duplicate keys, conflicting evidence,
  unsupported engine versions, missing sources and out-of-range times fail.
- Archives have a 64 MiB input limit and bounded record/string counts. Oversized
  or unsupported archives fail visibly rather than silently losing data. Future
  large-history support must revise these limits with memory measurements.
- Import first checks versions, mode, checksum, types, identifiers, references,
  timestamps, content snapshots, evidence and credit invariants in isolation.
  SQL in an archive is never executed. Unknown fields are rejected.
  Snapshots claiming reviewed status must exactly match installed reviewed content;
  editing an archive cannot turn a development draft into approved course content.
  Missing reviewed content is an explicit restore error until matching packs exist.
  Trusted historical installed versions are checked as well as the active version.
  References and approval labels supply no editorial authority. Duplicate references
  or a mismatched digest fail even with a recomputed archive checksum.
- A preview shows the backup date, level, session count, XP and review-history/session counts. Confirmation is
  separate from file selection. It also lists referenced pack versions. Study or
  verified installed-pack inventory changes after preview invalidate it.
- Replacement takes a SQLite write transaction, verifies the current-state
  fingerprint, writes/fsyncs and revalidates a unique pre-import recovery archive,
  and then replaces the whitelisted learner tables atomically. It does not merge
  histories. A failed backup, import write or transaction preserves the old state.
- Exports use a new filename and never overwrite the live database or an existing
  file. Cancellation changes no learner data. Native Rust file dialogs own path
  selection; the frontend receives summaries/tokens rather than arbitrary file
  read/write authority. Files are local and unencrypted; users choose where to
  keep/share them. Nothing is uploaded.
- Debug/development archives cannot be imported into release learner storage.
  Recovery archives use the same validation/preview/replacement flow.

Settings lists the newest 50 recovery copies. Older copies remain on disk; this
version never silently deletes a recovery archive. Export uses an explicit user
destination. Automatic recovery copies stay in the application's local `backups/`
directory, alongside (not inside) its SQLite database.

## Schema 1 or 2 to 3 migration

Fresh databases create schema 3. Opening schema 1 or 2 does not itself upgrade
it. The desktop first loads current content trust and obtains its lifetime
profile lock. `StudyWorkspace::migrate_schema` reserves a SQLite write transaction,
reads and validates the complete whitelisted learner state, writes a unique
`pre-migrate-v1-to-v3-*.langbackup` or `pre-migrate-v2-to-v3-*.langbackup`, fsyncs
it, rereads its checksum and revalidates its contents. Recovery uses the exact
original archive version, so the prior application can read it. A schema-1 store
moves directly to 3 in this one transaction and retains a genuine v1 recovery;
no intermediate profile or second backup is needed.

Only after verification does the transaction create missing history/session/
credit tables and set `PRAGMA user_version=3`. Other writers cannot change
learner state between snapshot and commit. Existing settings, lesson snapshots,
attempts, histories, credits and achievements are retained. Recovery write,
validation, DDL, commit or process-interruption failures leave the original schema
and rows intact; saved recovery files remain. Reopening schema 3 is a no-op and
creates no extra archive. Unknown versions remain refused.

Logical archives exclude unrelated SQLite tables and downloadable payloads.
Malformed legacy state cannot be called a valid backup or pass migration. Core
callers must explicitly migrate after supplying current content context before
restore/review operations. Settings supports all exact historical migration
prefixes (including v1-to-v2); path/ASCII/root restrictions still apply. Native
startup tests this path in CI with isolated legacy storage; real file/folder
dialogs still require manual acceptance.
