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

Run the A3 recording-primitives benchmark with:

```sh
python scripts/run-gate-a-benchmarks.py --bench recording_primitives
```

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
in-memory-store clone records. In A2 reports, benchmark probes also record the
prepare and infallible-apply portions of each in-memory commit separately.
A1 controlled-machine median and p95 ratios across history sizes are diagnostic
evidence. A2 controlled-machine timing checks require the largest minimal-store
history median to be no more than 1.10 times the zero-history median.
The A3 report covers retained-ledger, owned-queue, record-pool, and effect-outbox
append at histories 0, 1,000, and 100,000. Its Criterion distribution measures
the append phase, while probes record reserve and prepare timing, accounted
bytes, pool reuse, and post-preparation allocations. A3's hard gate is zero
allocations during every prepared append.
A3 controlled-machine timing checks require the largest history median to be no
more than 1.10 times the zero-history median.

Each Criterion sample constructs one fixture with the exact requested history,
measures exactly one operation, and reports that duration as the per-iteration
estimate. Fixture construction and history population are therefore excluded
from the measured interval, and benchmark calibration cannot silently grow the
history between measured operations.

Committed reports conform to `baseline-schema.json` and identify the exact
measured commit, machine, OS, toolchain, target CPU flags, runtime limits,
sample protocol, probe counts, and median/p95 timing summaries.
