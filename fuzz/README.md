Syntax Fuzzing
===============================================================================

The fuzz package is intentionally outside the main workspace. Install
`cargo-fuzz` and build all targets with:

```bash
cargo install cargo-fuzz
cargo fuzz build
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

The refresh script writes generated seeds under `fuzz/corpus/generated`, which
is ignored. Checked-in corpus seeds remain small and intentional.
