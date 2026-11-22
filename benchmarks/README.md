# Benchmarks

Benchmark programs, timing evidence, optimization history, and the chart now live
in [mech-lang/benchmarks](https://github.com/mech-lang/benchmarks). The extraction
preserves every historical benchmark tree and records the original Mech commit
IDs in its [migration map](https://github.com/mech-lang/benchmarks/blob/main/migration/commit-map.tsv).

The [published explorer](https://mech-lang.org/benchmarks/) continues at the same
address. Compiler regression tests remain in this repository under `hosts/gpu/tests`
and `src/engine/src/native_sequence`. New benchmark results belong in the benchmark
repository.

The compression records from [Mech PR #973](https://github.com/mech-lang/mech/pull/973)
are preserved in the [benchmarks archive](https://github.com/mech-lang/benchmarks/blob/4fb50f3879599b4a799e85a73fa39eccd8706e2c/archive/compiler/mech-pr-973-9087b1f077/)
and its [pinned payload release](https://github.com/mech-lang/benchmarks/releases/tag/mech-pr-973-evidence-9087b1f077).
The archive manifest records every original path, Git blob, SHA-256 and byte count.
Maintained regression inputs live under `hosts/gpu/tests/fixtures/native/`;
correctness tests do not download benchmark records.

The follow-up records from [PR #993](https://github.com/mech-lang/mech/pull/993)
and [PR #995](https://github.com/mech-lang/mech/pull/995) are preserved in
[benchmarks PR #10](https://github.com/mech-lang/benchmarks/pull/10), with
[pinned payloads](https://github.com/mech-lang/benchmarks/releases/tag/mech-pr-993-995-evidence-20261121).
Their manifests preserve original paths, Git blobs, SHA-256 and byte counts.
Qualification policy tests now belong to the benchmarks correctness workflow.
Compiler regression inputs remain local under `hosts/gpu/tests/fixtures/native/`.

PR #995 retains a byte-identical compact compiler regression program at
`hosts/gpu/tests/fixtures/native/compact-compiler-regression.mec`. The archive
also preserves its final parser-repair and complete-build checkpoint.
