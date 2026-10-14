# Foundation implementation record

Date: 2026-10-14. Authority: approved SPEC.md; user instruction to begin development.
This file records delegated technical choices under R-07. It does not amend the
confirmed product requirements or mark unimplemented obligations complete.

## Scope and component boundaries

`src/` renders the local interface and invokes named Rust commands. The browser
does not grade answers, award XP, or write SQL. `src-tauri/` owns the application
data location, trusted current time and a mutex-protected `Store`.
`crates/learning-core/` validates settings, interprets versioned lesson snapshots,
evaluates accepted answers and commits learner state to SQLite. It has no HTTP
client, AI client, media engine, analytics or login dependency.

The current route inventory is onboarding, Learn, lesson/summary, Progress and
Settings. Other specified screens are not implemented or represented as working.
Browser-only opening reports a missing desktop connection. Only test tools can
inject the Rust test driver's transport.

### Dependency/build selection

- Svelte 5.57.2, TypeScript 5.9.3, Vite 8.3.4, Svelte Vite plugin 7.3.1.
- Tauri JS API 2.12.4 and CLI 2.12.3; resolved Rust Tauri 2.12.4.
- SQLite via bundled rusqlite 0.38.0, avoiding an external SQLite installation.
- Chrono/chrono-tz for UTC instants and explicit IANA study dates; UUID request,
  session and ledger IDs; Unicode NFC normalization.
- Rust dependency resolutions and checksums are in Cargo.lock; npm exact direct
  versions and transitive resolutions are in package.json/package-lock.json.
- Playwright for browser integration, tiny_http only as a development dependency
  for the disposable driver. No driver is linked into the desktop application.
- Rust standard unit tests, rustfmt and clippy; pinned-action Windows/macOS CI.
- Rust-only Tauri dialog plugin 2.8.3 for backup file selection; SHA-256 for
  archive integrity and tempfile for durable new-file writes.
- Optional WebDriver plugin 1.5.0 is enabled only by `native-test`. A compile-time
  guard forbids that feature in release builds. Test storage and WebView data are
  isolated from the normal profile; no test-only file-selection IPC is added to
  the application.

Configuration follows the official [Tauri Vite integration](https://v2.tauri.app/start/frontend/vite/)
and [typed command boundary](https://v2.tauri.app/develop/calling-rust/).
System fonts and original CSS geometry avoid external UI assets. CSP permits
packaged resources and native IPC, with no external script/font/service sources.
No shell, filesystem, arbitrary SQL or network plugin is exposed to the UI.

## Storage schema version 1

SQLite `PRAGMA user_version=1`. Creation is transactional and foreign keys are
enabled. Unknown newer schema versions are rejected before learner writes.
There are no data-transforming migrations yet. Any future migration must add a
validated pre-migration backup and recovery tests; downgrades to incompatible
schemas are refused. Versioned export/import and pre-import recovery are now
implemented; the full contract and limits are in [BACKUPS.md](BACKUPS.md).

| Table          | Purpose/invariant                                                                                                          |
| -------------- | -------------------------------------------------------------------------------------------------------------------------- |
| profile        | Exactly one settings record; selected level, zone, goal, theme and accessibility                                           |
| sessions       | Stable lesson ID/version and immutable full lesson snapshot; one active session; saved position/status; practice-only flag |
| session_items  | Current draft, first correctness, first independent correctness, hint/reveal flags, resolved state and feedback            |
| attempts       | Unique request ID; evaluated result and hint/reveal evidence; timestamp; no need for retained raw response in this ledger  |
| xp_ledger      | 10 XP per lesson/date; unique session and unique lesson/date; no credit for drafts                                         |
| study_days     | Literal calendar date, original study time zone, goal snapshot                                                             |
| achievements   | Idempotent grants for the three implemented achievement criteria                                                           |
| answer_reports | Explicitly entered local editorial reports; never automatically uploaded                                                   |

Debug application data uses `development.sqlite3`; release uses `learner.sqlite3`
in Tauri's per-app **local** data directory (not its roaming-data directory).
Tests use temporary databases.
The production app currently has no curriculum installed. Only debug builds
embed the draft café fixture, and practice-only sessions cannot touch learner
XP, study-day, achievement or course-completion records.

## Interaction and evidence decisions

- Typed commands cover app state/settings, lesson start, draft save, submit,
  hint, reveal, advance and local editorial reports. Frontend TS contracts mirror
  serialized Rust types. A shared-schema generator is not implemented yet.
- The current deterministic fixture sequence is saved in a lesson snapshot.
  Randomized selection/seeds are later work; no shuffling is claimed.
- All mutations use the native clock; the UI cannot choose awarding timestamps.
  Tests inject explicit times through the core API for day/DST cases.
- Saves occur as text/options/tiles change and are drained before navigation or
  evaluation. Reopening restores position, draft, hints and evidence. Sudden
  termination before an in-flight keystroke commits can lose that keystroke;
  full crash/power-loss durability testing remains open.
- Answer normalization applies NFC, whitespace, case, curly apostrophes and
  non-semantic terminal `. ! ? …`. Meaningful accents/apostrophes and internal
  punctuation are preserved; authored alternatives handle permitted differences.
  No fuzzy typo acceptance or AI correctness service exists.
- Word tiles have stable IDs; identical visible words cannot reuse one tile ID.
- First result and independent-first result are written once. A hint before a
  correct answer prevents independent evidence; correction cannot repair the
  original result. Explicit reveal records non-independent evidence and allows
  completion after solution review. No mastery metric is inferred from XP.
- Check/Continue are disabled while a command commits. Submission request IDs
  make retries idempotent. Continue includes the prior exercise ID so duplicate
  clicks cannot skip the next item.
- Completion and eligible XP/day/achievement changes share one transaction.
  The same lesson can be practised without a daily limit; only XP is capped.
  Streak days qualify on completed reviewed lessons even when XP is capped.
- Goal edits update today's snapshot and future days; previous dates keep their
  prior goal. Study zone changes do not rewrite historical dates/zones. Current
  streaks consider today's/yesterday's literal dates without discarding historical
  dates ahead of the new zone's local date.
- Leaderboard participation does not exist yet, so no contributions are uploaded
  or queued. Before enabling participation, add the authenticated identity and
  outbox in the qualifying transaction under C-20/R-04/R-06.
- Reviewed-content XP behavior is tested with test-only copies marked approved.
  Tests do not approve the fixture for learners. Course-level achievements need
  an approved full objective manifest and are intentionally not granted yet.

## Accessibility and UI boundaries

Visible focus, semantic headings/forms, skip link, English labels, radio controls,
keyboard-operable word tiles, live feedback, focus on the next exercise, dark/light
themes, enlarged text and reduced-motion handling are present. Contrast, native
screen readers, OS text settings, motor-access workflows and full accessibility
conformance still need dedicated QA. No audio/microphone permissions are requested
because those features are not implemented. Placeholders do not pretend to be
listening/speaking alternatives or award unobserved skill evidence.

## Validation record

Development checks are recorded separately from full acceptance gates:

- Rust core unit tests cover presentation normalization without losing accents,
  duplicate tile IDs, wrong/retry/hint evidence, duplicate submit/advance,
  draft exclusion, XP caps, goals, DST and study-zone changes, streak gaps,
  achievement de-duplication, reopen/resume and unknown-schema preservation.
- Frontend `npm run build` passed with zero Svelte errors/warnings.
- Windows native `cargo check -p lang-desktop --locked` passed.
- Browser interactions against the real Rust/SQLite disposable driver verified
  onboarding validation; wrong answer/hint/correction; local report; accented
  translation; exit/reload/resume; ordering; reveal; reading; and the saved
  summary: 2/5 first correct, 2/5 independent, one hint and one reveal.
- Dark theme and 200% text were checked in the connected browser at the native
  minimum window width (720px): document width 705px, no horizontal overflow.
- Standalone Playwright runner launch was attempted but blocked by Windows
  sandbox pipe permissions. It has not passed here. Connected-browser checks
  are a separate record, not a claimed pass of the unattended runner.
- The first cargo registry download hit a Windows Schannel credential error.
  This execution environment used a temporary loopback bridge outside the repo;
  Node verified upstream HTTPS certificates, and Cargo used the canonical index
  checksums. No TLS verification was disabled; Cargo.lock retains crates.io URLs.
  Vite's optional mapped-drive `net use` probe also needed an environment-only
  guard for synchronous child-pipe denial. These are not product configuration.

- Foundation core test pass: **15 passed, 0 failed**, including injected-award-failure
  rollback (completion, day and achievement all remain unchanged) and preserving
  an active lesson snapshot across a content version update.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- Windows x86-64 development executable built successfully with packaged frontend
  assets (`npm run tauri -- build --debug --no-bundle`, followed by a final native
  rebuild after source changes). This verifies compilation, not native runtime.
- Native `cargo check -p lang-desktop --release --locked` passed, including the
  non-debug code path that excludes the embedded development lesson fixture.
- The foundation was published and merged in [PR #1](https://github.com/misc9115-boop/lang/pull/1).
  Its Windows/macOS CI passed core tests, the unattended browser integration,
  frontend/native builds and release compilation. Those checks did not run a
  native application or establish minimum-OS/resource-budget support.
- No changes to the approved SPEC.md; no public release performed.

### Backup/recovery increment

- Local verification: **30 core tests and 1 desktop path-validation test passed**;
  frontend build has zero Svelte errors/warnings; rustfmt and Clippy passed for
  the normal workspace and the optional native-test feature.
- Core tests cover round-trip settings, saved drafts, attempts and earned credit;
  recovery contents; corrupt or unsupported archives; altered evidence; untrusted
  reviewed-content claims; stale previews; one-use/cancelled tokens; overwrite
  refusal; oversized input; failed recovery writes; and transaction rollback.
- Connected-browser checks against the real SQLite driver exercised export,
  disabled confirmation, cancellation, replacement, and restoring the pre-import
  copy. Only native file selection was replaced by the test adapter.
- Windows native-test executable compiled. Two attempts to run the smoke test
  on this execution host failed during Tauri setup with `Access is denied.
(os error 5)`, before the application database was created. Tauri creates its
  configured window before the application's setup callback. The precise OS
  denial has not been isolated; native startup/IPC and dialogs are unverified here.
- The Windows/macOS workflow now includes the native smoke test. Adding a test
  to CI does not establish that it has passed; inspect the current run separately.

No installer, native end-to-end,
macOS/ARM, offline-media, speech, AI-quality, complete accessibility, minimum-OS,
reference-hardware or resource-budget gate has been claimed as passed.

## Next implementation work

1. Pass native startup, IPC and restart checks on the supported hosts; verify
   native file dialogs and recovery manually, then extend failure/crash coverage.
2. Define a versioned objective/skill manifest and signed-off content-pack format,
   author reviewed lessons and track real editorial provenance.
3. Implement FSRS review, full exercises/media and accessible alternatives,
   resource-controlled pack management, and tested pre-migration recovery.
4. Implement independent leaderboard and text/voice conversation adapters, with
   privacy, cancellation, credential/outbox and real-service tests.
5. Finish full beginner–C2 coverage and all public-release gates in SPEC.md.

Speech/model/voice selection still requires the specified evaluation evidence.
AI hardware/host/funding, human editorial review, signing and final artwork remain
explicit work; none has been invented or silently declared resolved.
