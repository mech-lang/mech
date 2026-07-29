Syntax Fuzzing
===============================================================================

The fuzz package is intentionally outside the main workspace. Install
`cargo-fuzz` and build all targets with:

```bash
cargo install cargo-fuzz --version 0.13.2 --locked
cargo fuzz build
```

CI and final Phase 1 validation use `cargo-fuzz 0.13.2` with
`nightly-2026-03-03`. Local reproductions should select those exact versions
instead of moving tool versions:

```bash
rustup toolchain install nightly-2026-03-03 --profile minimal
rustup default nightly-2026-03-03
```

Run the Phase 1 validation budgets from the repository root:

```bash
cargo fuzz run parse_document -- -runs=100000
cargo fuzz run incremental_equivalence -- -runs=50000
cargo fuzz run recovery_progress -- -runs=100000
cargo fuzz run edit_piece_table -- -runs=50000
```

Crash, timeout, and invariant-violation inputs belong in
`src/syntax/tests/fixtures/document/fuzz-regressions` as deterministic tests
before a parser change is reviewed.

Refresh a local corpus from the checked-in syntax fixtures and repository
documents with:

```bash
bash fuzz/scripts/refresh-corpus.sh
```

The refresh script writes generated seeds into the actual libFuzzer target
directories:

```text
fuzz/corpus/parse_document/
fuzz/corpus/incremental_equivalence/
fuzz/corpus/recovery_progress/
fuzz/corpus/edit_piece_table/
```

The refresh first removes untracked generated seeds from each target directory
while retaining checked-in intentional seeds. It then repopulates every target
from the Phase 0 accepted and rejected fixtures, the repository `.mec` corpus,
Phase 1 malformed fixtures, and promoted regressions. The ordinary
`cargo fuzz run <target>` commands above therefore consume the refreshed corpus
without additional path arguments.

The hosted `Syntax fuzzing` workflow runs all four targets for pull requests
targeting `feature/syntax` with 5,000 runs and a 180-second limit per target.
Pushes to `feature/syntax` and manual dispatches use 25,000 runs and a
900-second limit per target. Its Monday schedule has the same larger budget,
but GitHub activates that schedule only after this workflow file exists on the
repository default branch.
