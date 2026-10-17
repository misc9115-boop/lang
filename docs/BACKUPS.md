# Backup format and replacement contract

Implementation decision under SPEC.md §3 / AC-08, 2026-10-14. This implements
the existing requirement; it does not add sync or multiple learner profiles.

- File extension: `.langbackup`. Format version 1 is a UTF-8 JSON archive with
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
- A preview shows the backup date, level, session count and XP. Confirmation is
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

Schema version 1 has no data-transforming migrations. Unknown database versions
remain refused; this work provides the recovery primitives, not a claimed tested
migration to a schema that does not exist.
