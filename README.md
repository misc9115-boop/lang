# Italian learning — development foundation

An early implementation of the approved [SPEC.md](SPEC.md). The application is
free, open source, and ad-free. The full specification remains the source of
truth; the implemented foundation is **not the complete application or a release**.
The displayed name, monogram, and geometric artwork are development placeholders.

## What currently works

- Tauri 2 desktop shell, Svelte 5/TypeScript UI, and a reusable Rust learning core.
- One local profile: Beginner/A1–C2 selection, positive daily goal, IANA study
  time zone, light/dark/system theme, 100–200% text size, and reduced motion.
- Five draft exercise fixtures: choice, typed translation, word ordering,
  fill-in-the-blank, and reading comprehension. Immediate explanations,
  unlimited retries, hints, explicit solution review, and local answer reports.
- SQLite transactions, saved answer drafts, exit/resume, lesson snapshots, and
  first-attempt evidence that retries cannot rewrite.
- Versioned local backup export, validated restore previews, explicit replacement
  confirmation, and verified pre-import recovery copies. Settings includes the
  recovery list; existing files are never overwritten. See [backup contract](docs/BACKUPS.md).
- Core rules for 10 XP per reviewed lesson per study day, current/best streaks,
  daily goals, and first-lesson/7-day/100-XP achievements. These are covered by
  test-only approved fixtures; **there are no reviewed lessons to earn them in
  the application yet**.

Development fixtures cannot earn learner XP, streaks, achievements, or course
credit. See [content status](content/README.md) and the [implementation record](docs/IMPLEMENTATION.md).

## Run locally

Use Node.js/npm, Rust's MSVC toolchain on Windows, Visual Studio C++ tools and
Windows SDK, and WebView2. On macOS, use the Apple development tools. See
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
The initial setup downloads dependencies; the implemented lesson core makes no
network requests during study. The Vite development server is local build tooling.

From the repository directory:

```sh
npm ci
npm run tauri dev
```

PowerShell users can use `npm.cmd` if execution policy prevents `npm.ps1`.
The debug app opens its development workbench. Its database is
`development.sqlite3` in Tauri's per-app data directory, separate from release
`learner.sqlite3`. No cloud account or API key is used.

To compile a Windows/macOS development executable without an installer:

```sh
npm run tauri -- build --debug --no-bundle
```

Build outputs are under `target/debug/`. This is an unsigned development build;
installer packaging, signing, updates, and supported-target validation remain
future work. A non-debug build excludes the embedded unreviewed lesson fixture.
The macOS icon/DMG workflow still needs implementation and testing.

`npm run dev` alone serves the frontend in a browser, with a clear message that
it has no desktop database connection. It does not silently substitute a fake
backend or use browser storage for learner data.

## Checks

```sh
npm run build
cargo fmt --all --check
cargo test -p learning-core --locked
cargo test -p lang-desktop --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
npm run test:ui
npm run test:native
```

UI checks start a loopback test driver using the real Rust core and a disposable
SQLite database, plus a test-only browser transport. They exercise onboarding,
validation, retries, hints, reports, accent handling, saved drafts after reload,
ordering, reveal, summary, draft-credit exclusion, themes and enlarged text.
Backup checks use real archives and transactions with a test file-selection
adapter; they cover cancellation, replacement, recovery and corruption.
The driver and transport are not part of the packaged app. Ports 1420, 1421 and
(on Windows with an explicit browser override) 19223 must be free. Install the
test browser first with `npx playwright install chromium`. The default is
Playwright's downloaded Chromium with its browser sandbox enabled;
`LANG_TEST_BROWSER_PATH` can explicitly select Chrome/Edge on Windows.
`LANG_TEST_CDP_URL` can point to a separately
started test browser. Screenshots go to ignored `test-results/`.

The native smoke test builds an explicitly enabled debug-only WebDriver feature,
uses actual WebView/IPC/SQLite, and checks recovery plus saved drafts after an
application restart. It uses an isolated SQLite profile under `test-results/`
and an ephemeral WebView (also a separate data directory on Windows); it never
opens the normal learner database. Port 19445 must be
free. The driver binds only to loopback and cannot compile into a release build.
It does not automate native open/save dialogs. Logs, status and screenshots are
kept in `test-results/` for diagnosis. A failed native startup fails the test;
there is no browser fallback.

To inspect the same disposable integration workbench manually:

```sh
npm run test:serve
```

Open the printed local URL. Stopping it discards its test database. This workbench
is a browser integration fixture, not evidence that native permissions or audio
have passed. It must not be exposed on a public interface.

The first implementation was checked with Node 24.11.0, npm 11.20.0, Rust 1.93.1,
Visual Studio 2022 C++ tools and WebView2 on the development Windows host.
This does not establish minimum-OS support, hardware budgets, or macOS behavior.
The foundation's Windows/macOS CI passed on [PR #1](https://github.com/misc9115-boop/lang/pull/1).
That run predates backup/recovery and the native smoke test added here.
See [validation notes](docs/IMPLEMENTATION.md#validation-record).

## Still required

Reviewed beginner–C2 curriculum and objective coverage; placement/checkpoints;
all audio, dictation, listening and offline speaking; extended writing/rubrics;
FSRS review; full reference browsing; pack installation/integrity/repair;
tested data-transforming migrations; remaining achievements and
reminders; authenticated leaderboard/outbox; real text/voice conversation
adapters; privacy/deletion controls; installers, signing and update consent;
complete native accessibility, performance, platform and content QA.

No AI model, speech engine, voice, host hardware, funding, or performance result
has been selected or claimed. See SPEC.md for the full release obligations.

## Licenses

Original code: [MIT](LICENSE). Eligible original curriculum/media: CC BY 4.0,
with item-specific provenance and review status; see [content/README.md](content/README.md).
Third-party dependencies retain their respective licenses. A full dependency
notice/rights audit is still a public-release gate.
