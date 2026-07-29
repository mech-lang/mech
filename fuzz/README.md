Syntax Fuzzing
===============================================================================

The fuzz package is intentionally outside the main workspace. Install
`cargo-fuzz` and build all targets with:

```bash
cargo install cargo-fuzz
cargo fuzz build
```

CI runs these commands with the repository CI toolchain,
`nightly-2026-03-03`. Local reproductions should select that same toolchain
instead of an unpinned moving nightly:

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
```

Those generated files are ignored, while the small intentional seed files stay
checked in. The seeds include the Phase 0 accepted and rejected fixtures, the
repository `.mec` corpus, Phase 1 malformed fixtures, and promoted regressions.
The ordinary `cargo fuzz run <target>` commands above therefore consume the
refreshed corpus without additional path arguments.
