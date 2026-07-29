Manual Syntax Parser-Input Harness
===============================================================================

This optional package is intentionally outside the main workspace. It is not
part of Phase 1 completion criteria, required agent validation, ordinary CI, or
scheduled execution. Agents must not invoke this harness unless explicitly
instructed.

For deliberate manual use, install `cargo-fuzz` and build all targets with:

```bash
cargo install cargo-fuzz --version 0.13.2 --locked
cargo fuzz build
```

The harness is pinned to `cargo-fuzz 0.13.2` and `nightly-2026-03-03`.
Deliberate manual runs and reproductions should select those exact versions
instead of moving tool versions:

```bash
rustup toolchain install nightly-2026-03-03 --profile minimal
rustup default nightly-2026-03-03
```

When explicitly requested, run an individual target from the repository root
with a budget chosen for that request:

```bash
cargo fuzz run parse_document -- -runs=1000
cargo fuzz run incremental_equivalence -- -runs=1000
cargo fuzz run recovery_progress -- -runs=1000
cargo fuzz run edit_piece_table -- -runs=1000
```

Any input that exposes a parser mismatch, failure to make progress, or
piece-table discrepancy belongs in
`src/syntax/tests/fixtures/document/fuzz-regressions` as deterministic tests
before a parser change is reviewed. The eight checked-in promoted regressions,
deterministic property tests, and mutation tests remain part of the ordinary
syntax test suite.

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
Phase 1 malformed fixtures, and promoted regressions. A deliberate
`cargo fuzz run <target>` command therefore consumes the refreshed corpus
without additional path arguments.

The hosted `Manual syntax parser-input validation` workflow is
`workflow_dispatch`-only. It has no pull-request, push, or scheduled trigger.
Starting it is a deliberate manual action and is subject to the same
explicit-instruction rule for agents.
