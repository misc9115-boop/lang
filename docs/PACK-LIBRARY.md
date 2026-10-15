# Offline text-pack library

Implementation decision under SPEC.md §§3, 11–12 / AC-03, 2026-10-15.
The core API and developer CLI now install validated directory-based **text**
packs into a separate SQLite content catalogue. Desktop pack settings, native
folder selection, downloads, media and backup pack-reference integration remain
subsequent work. The desktop still uses its embedded development fixture.

## Try the actual library

From the repository root, build once and use the example executable, or run:

```sh
cargo run -p learning-core --example pack_library --locked -- --development test-results/packs.sqlite3 install content/development
cargo run -p learning-core --example pack_library --locked -- --development test-results/packs.sqlite3 list
cargo run -p learning-core --example pack_library --locked -- --development test-results/packs.sqlite3 activate dev.pack.cafe 1
cargo run -p learning-core --example pack_library --locked -- --development test-results/packs.sqlite3 verify dev.pack.cafe 1
cargo run -p learning-core --example pack_library --locked -- --development test-results/packs.sqlite3 active
```

Create `test-results` first if it does not exist. The database parent must exist.
After initial dependency setup, these operations need no network. `npm run
test:packs` builds the executable and runs its lifecycle/exit-code checks on a
fresh temporary catalogue with spaces and accented characters in its path.

Syntax: `pack_library [--development] <catalogue.sqlite3> <command> [arguments]`.

| Command | Behavior |
| --- | --- |
| `install <folder>` | Validate, copy verified bytes and cache a version; does not activate it |
| `list` | List stored/removed versions, declared payload bytes and active pointers |
| `verify <id> <version>` | Recheck bytes, compatibility and current host trust; print coverage report |
| `activate <id> <version>` | Validate and explicitly select an intact version, including rollback |
| `active` | Revalidate all active packs; print their lesson summaries |
| `deactivate <id>` | Clear that pack's active pointer; idempotent |
| `remove <id> <version>` | Delete inactive payloads; retain immutable identity metadata |

Success exits 0 with JSON on stdout. Invalid packs/operations exit 1. Invalid
syntax/version arguments exit 2. Diagnostics go to stderr. Non-install commands
refuse a missing catalogue rather than creating one by mistake.

The default `ReviewedOnly` mode refuses drafts. `--development` explicitly
enables unreviewed development content; it does not grant editorial approval.
Both modes reject forged approval claims. This CLI has **no trusted review
digests**, no approval command and no option to trust a pack's self-assertion.
The Rust host supplies `ContentTrust` from independently completed editorial work.
Every verification/activation/load rechecks that current trust, including after
reopening a database; removing a trusted digest prevents subsequent loading.

## Storage and atomicity

`learning_core::pack_library::PackLibrary` owns a catalogue with a distinct SQLite
application ID and schema version. Passing a learner database, nonempty unknown
database or unsupported catalogue version fails without transforming its schema.
No learner migration or backup format changed in this increment.

For this bounded text-only format, the catalogue stores the original manifest and
declared lesson bytes as BLOBs. This avoids splitting installation between a
filesystem rename and a database commit. It is not the future storage format for
large media/speech assets, which must remain separate files under the spec.

The shared content validator retains exactly the bytes it checked. Installation
uses that snapshot; it never reopens a mutable source folder after validation and
never copies unlisted files. Existing directory path/symlink/size restrictions
still apply. No code is executed and no citations are fetched.

Installation uses an immediate transaction for identity checks, all payloads and
catalogue state. Activation uses an immediate transaction for revalidation,
cross-pack ID checks and switching the active version. A newly cached version
does not replace the active version until explicit activation succeeds. Write or
validation failure preserves the previous state. SQLite uses full synchronous
writes; tests cover injected write/activation failures and abrupt process exit
with an open write transaction, followed by reopen/recovery. This is not a claim
of tested power-loss or faulty-storage behavior on every supported platform.

Active packs must not conflict on lesson, exercise, objective or target IDs.
Corrupt/missing/untrusted active content returns an error with no silent fallback
or deletion. The host must surface that error and offer repair/deactivation.

`list` reports catalogue state; `stored=true`/`active=true` alone are **not** fresh
integrity or editorial assertions. `verify` and `active` recheck actual bytes.
Stored byte lengths are checked before loading; manifest/payload hashes, parsed
identity and the entire content contract are then verified again.

## Versions, repair and removal

- New versions of a pack ID must exceed its recorded maximum. Explicit activation
  can roll back to an intact retained version. Installing an unknown lower version
  is rejected; an exact recorded historical version can be reinstalled.
- A recorded pack ID/version is bound to its exact manifest digest. Changed bytes
  require a new pack version, including manifest formatting changes.
- Changed lesson bytes require a higher lesson version. Changed serialized
  curriculum metadata requires a higher curriculum version. These identities
  cannot regress in a new pack version. Removed lessons can be reintroduced only
  consistently with their retained identity history.
- Reinstalling the exact original pack repairs missing/corrupt stored manifests or
  payloads without changing the active pointer or redefining identity history.
- Removal requires explicit deactivation for an active version. Payload rows are
  deleted; manifests/digests/semantic identity history remain to prevent version
  reuse. Freed SQLite pages can be reused; the file does not automatically shrink.
  No learner records are removed. Existing sessions keep their own snapshots.

The reader's existing 1 MiB manifest, 8 MiB lesson, 64 MiB total lesson-payload and
1000-lesson limits remain. This initial library permits at most 1000 recorded pack
versions, including removed versions; it does not silently prune rollback/history
records. These are bounded input/history limits, not measured course, RAM or disk
budgets. The validator currently retains both parsed lessons and raw verified
bytes; memory/resource targets still need representative measurement.

## Integration boundary

This is a working core library and CLI, not yet learner-facing pack management.
Before connecting it to the desktop: add typed pack references to learner backups,
retain independently trusted historical lesson evidence during restore, surface
missing/corrupt content, show sizes and install/activation/removal confirmation,
and test native folder selection plus interrupted flows. Existing backups still
reject nonempty pack references rather than pretending to restore installations.
The library must not be substituted for the desktop's current lesson source until
that integration is complete. Downloads, media/alternatives, catalogue migrations,
reviewed course authoring, resource benchmarks and the full platform matrix remain.
