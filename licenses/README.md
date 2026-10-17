# Dependency notices

Original application code remains under the repository's MIT license.
These notices cover the FSRS increment; this directory is not yet a complete
dependency notice inventory for public binary distribution.

- `fsrs` 6.6.2: Copyright (c) 2023, Open Spaced Repetition.
  [BSD-3-Clause notice](fsrs-BSD-3-Clause.txt).
  [Published source](https://crates.io/crates/fsrs/6.6.2).
- `priority-queue` 2.7.0, an FSRS dependency: Copyright 2017 Gianmarco Garrisi
  and contributors. Its published package offers `LGPL-3.0-or-later OR MPL-2.0`;
  this project selects MPL-2.0. The dependency is used without modification.
  [MPL-2.0 text](priority-queue-MPL-2.0.txt).
  [Exact published source](https://crates.io/crates/priority-queue/2.7.0).

Cargo.lock records exact versions and source checksums. The remaining new
dependencies declare MIT, Apache-2.0 or their offered combination; existing
shared dependencies also include Unicode-3.0. Preserve their selected notices,
copyrights and source-availability information in release materials as part of
the full dependency/rights audit required before distribution. No installer or
complete binary notice bundle is produced by this increment.
