# Backup format and replacement contract

Implementation decision under SPEC.md §3 / AC-08, 2026-10-14. This implements
the existing requirement; it does not add sync or multiple learner profiles.

- File extension: `.langbackup`. Format versions 1 and 2 are UTF-8 JSON archives with
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
- Current exports use archive/schema version 2 and include `reviewRecords`, even
  when empty. Version-1 archives remain importable and have no review records;
  restoring one explicitly replaces current review histories with an empty set.
  A version-2 recovery archive preserves the histories being replaced. Version-1
  readers reject newer archives rather than silently dropping these records.
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
- A preview shows the backup date, level, session count, XP and review-history count. Confirmation is
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

## Schema 1 to 2 migration

Fresh databases create schema 2. Opening an existing schema-1 store does not itself
upgrade it: the desktop first loads current pack trust and obtains its lifetime
profile lock. `StudyWorkspace::migrate_schema` then reserves a SQLite write
transaction, reads and validates the complete whitelisted learner state, writes a
unique `pre-migrate-v1-to-v2-*.langbackup`, fsyncs it, rereads its checksum and
revalidates its contents. That recovery archive uses the original version-1
contract, so the prior application can read it. It excludes downloadable packs,
as ordinary backups do; exact installed-pack references are retained.

Only after recovery verification does the same transaction create
`review_histories` and advance `PRAGMA user_version` to 2. Other writers cannot
change learner state between snapshot and commit. The migration does not rewrite
lesson snapshots, settings, attempts, credit or achievements. Recovery write,
validation, DDL, commit or process-interruption failures leave the prior schema
and rows intact; a recovery file already saved is retained. Startup reports the
error rather than initializing a replacement profile. Reopening schema 2 is a
no-op and creates no extra upgrade archive. Unknown schema versions remain refused.

The backup is a logical, validated learner archive, not a copy of arbitrary SQLite
tables. It preserves the data in this documented schema; unrelated tables are not
exported or removed. A malformed legacy profile cannot be called a valid backup
and therefore cannot pass migration. Core callers opening an old database must
explicitly migrate after supplying current content context before restore/review
operations. Native startup exercises this path in CI using an isolated legacy
profile; native open/save/folder dialogs still require manual acceptance.
