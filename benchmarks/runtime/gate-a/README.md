# Runtime Gate A benchmark evidence

This directory stores controlled-machine evidence for the value/executor Gate
A stack. The benchmark target uses Criterion and the runtime's non-default
`runtime_bench_probes` feature. Setup constructs source, retained history, and
fixtures outside the measured operation.

Run the complete release benchmark and print a deterministic JSON summary:

```sh
python scripts/run-gate-a-benchmarks.py
```

If automatic CPU/model discovery is unavailable, provide the controlled
machine's stable identity with `--machine-label <model>`; the runner rejects
generic architecture-only labels.

Pass `--output <path>` only when a summary should be written. By default the
runner writes raw combined output beneath `target/gate-a-benchmark-runs/`,
keeps Criterion's raw data beneath `target/criterion/`, and prints the summary
without changing tracked source files.

The full-turn and direct-event histories are 0, 32, 1,024, and 16,384. Minimal
store commits contain one transaction and two events and run against 0, 1,000,
10,000, and 100,000 retained event/transaction pairs. The opt-in `--extended`
sweep adds 1,000,000 and is not part of ordinary hosted CI. Explicit savepoint
histories are 0, 100, and 1,000 staged operations and remain diagnostic in Gate
A. The mixed store fixture exercises every mutation family representable by one
valid `RuntimeStoreCommit`.

Allocation counts cover only the measured operation. Criterion timing remains
supporting evidence: hosted CI does not compare absolute latency. A1's hard
gate is zero context-event snapshot items; A2's hard gate is zero complete
in-memory-store clone records. Controlled-machine median and p95 ratios across
history sizes are diagnostic evidence, not a Gate A pass/fail criterion.

Each Criterion sample constructs one fixture with the exact requested history,
measures exactly one operation, and reports that duration as the per-iteration
estimate. Fixture construction and history population are therefore excluded
from the measured interval, and benchmark calibration cannot silently grow the
history between measured operations.

Committed reports conform to `baseline-schema.json` and identify the exact
measured commit, machine, OS, toolchain, target CPU flags, runtime limits,
sample protocol, probe counts, and median/p95 timing summaries.
