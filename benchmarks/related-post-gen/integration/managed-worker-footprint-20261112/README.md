# Managed worker footprint: selected IR334 checkpoint

IR334 improves the previous selected IR331 compiler in both the primary cohort and independent orchestration. It remains slower than the fastest qualified Rust control: **1.168× Rust** in the primary cohort and **1.176× Rust** in confirmation. The fastest-Rust target remains unmet.

This is a general engine admission optimization. A prepared, current frame footprint can commit its exact observer delta directly to the existing account and worker budget, avoiding a repeated recursive footprint traversal. The active account, local and parent-relative limits, shared worker debit, sealed-group validity and original error/paid-prefix behavior remain checked. Optional misses use the existing traversal. There is no new compatibility layer, backend or dependency; code generation remains Cranelift. The complete source change relative to IR331 is the engine helper, its tests and the IR334 identity stamp.

The source actually measured is [55a47b080eba45e46596976279d196b9c97eba2e](https://github.com/mech-lang/mech/commit/55a47b080eba45e46596976279d196b9c97eba2e), based on published IR331 [f361f91f298b87ebb9bee880930951fdf79b0d5f](https://github.com/mech-lang/mech/commit/f361f91f298b87ebb9bee880930951fdf79b0d5f). The packet preserves that historical source pin. The verifier also accepts an explicitly recorded published equivalent commit after a docs-only rebase, and checks every measured source hash against it. Later IR335/336 experiments are excluded. The equivalent published source after rebasing onto #973 at `ee77382475a4d98c4483dd2059a040a4eba009d2` is [`6cd626c3dfb5cdac2efc3c6978949a717ada27a8`](https://github.com/mech-lang/mech/commit/6cd626c3dfb5cdac2efc3c6978949a717ada27a8). All 2,814 measured source files match exactly; the six base paths changed only documentation and the experiment ledger.

All timings below are medians of complete continuous post-JSON intervals on one Apple M3 machine. Each process prepares its input, creates/runs/joins its four query workers and materializes the result within that interval. JSON IO and managed artifact restoration remain in separate whole-process diagnostics. The original two Rayon controls preinitialize their pools; they are retained, but excluded from the complete post-JSON reference. Current Rust means the fastest of 22 eligible controls; the fastest of the original 20 serial controls is reported separately. Lower multiples are faster.

| Implementation | Primary, 31 rounds | × current Rust | × original serial Rust | Confirmation, 13 rounds | × current Rust |
|---|---:|---:|---:|---:|---:|
| Fastest eligible Rust, fresh four workers | 1.229250 ms | 1.000 | 0.794 | 1.193917 ms | 1.000 |
| Fastest original serial Rust | 1.547958 ms | 1.259 | 1.000 | 1.478875 ms | 1.239 |
| Managed host-v2 control | 1.605291 ms | 1.306 | 1.037 | 1.581250 ms | 1.324 |
| Previous selected IR331 | 1.439000 ms | 1.171 | 0.930 | 1.435083 ms | 1.202 |
| **Selected IR334** | **1.435291 ms** | **1.168** | **0.927** | **1.404500 ms** | **1.176** |
| Historical compact IR320 control | 1.344625 ms | 1.094 | 0.869 | — | — |

The compact row is an explicitly historical unmetered compiler control, with arithmetic and indexing checks. It was not rebuilt with IR334 and is not evidence of current managed/compact compiler parity.

Incremental IR334 versus IR331, using the per-round percentage `100 × (IR334 / IR331 − 1)`:

| Cohort | Median paired change | 95% bootstrap interval | Faster pairs |
|---|---:|---:|---:|
| Primary | −0.7273% | [−2.0848%, −0.3449%] | 23/31 |
| Independent orchestration | −1.5947% | [−2.2554%, −0.3056%] | 10/13 |

These are paired changes, not the percentage difference between the two marginal medians. Both intervals exclude zero. Confirmation uses a separately implemented runner on the same machine; this is not separate-machine replication. All initial observations are retained, with no warmup removal, outlier deletion or adverse-result rerun.

The primary cohort completed all **868 exact outputs** before final summary processing requested an absent, stale instrumented resource reference. The immutable raw checkpoint is retained alongside its derived report and exact offline recovery script. Recovery changed no observations or orders and reran no executable. The verifier reproduces the complete derived report using the frozen pure summary function, then independently checks timers, medians and bootstrap results. The earlier `first-scoped-frame` attempt is retained separately: its first Rust process printed a timer, but the macOS time wrapper failed on system-time access. No Mech candidate ran and that partial row is excluded from qualified counts. Prospective protocol/plan files retain their original prepared status; completed reports and raw rows establish completion.

A separate seven-round resource cohort completed **196 exact outputs**. Counters below cover all process threads within the complete post-JSON window. Instrumented latency is diagnostic and was not used to select a speed winner.

| Implementation | Instructions | Cycles | Allocation requests | Requested bytes | Additional requested-heap peak |
|---|---:|---:|---:|---:|---:|
| Managed host-v2 | 31,503,210 | 9,932,357 | 735 | 7,036,985 | 4,871,859 |
| IR331 | 29,912,576 | 9,072,521 | 725 | 6,376,193 | 4,211,067 |
| **IR334** | **29,679,792** | **8,844,701** | **725** | **6,376,193** | **4,211,067** |
| Fresh-four Rust reference | 18,176,361 | 6,602,174 | 78 | 7,026,975 | 5,721,694 |
| Original serial Rust reference | 17,664,223 | 5,902,218 | 58 | 6,825,713 | 5,648,124 |

IR334 reduces instructions by 232,784 (0.778%) and cycles by 227,820 (2.511%) versus IR331 in this resource cohort. Allocation requests, requested bytes and additional peak remain unchanged. Freed bytes are retained; free-call counts are unavailable. Requested heap excludes allocator internals, OS stacks and mmap. RSS and physical-footprint endpoints are retained for every window, while peak RSS is measured over the whole process; neither is the requested-heap peak.

| Scope/size | IR331 | IR334 |
|---|---:|---:|
| Whole-process peak RSS, resource median | 26,361,856 B | 26,017,792 B |
| Ordinary executable file | 17,108,464 B | 17,095,936 B |
| Instrumented executable file | 17,221,296 B | 17,208,736 B |
| Emitted program object | 5,670,768 B | 5,670,768 B |
| Native program code, compiler metadata | 1,021,740 B | 1,021,740 B |
| Root native frame, compiler metadata | 1,568 B | 1,568 B |
| Whole-process primary median | 53.386 ms | 52.841 ms |
| Whole-process confirmation median | 44.407 ms | 44.070 ms |

The root native frame is not a total thread-stack measurement. Per-function code/frame sizes, executable sections, separate restore/JSON windows, endpoint memory and all other retained Rust resources remain available in the raw plan/report. Whole-process timings are secondary and are not an acceptance gate.

Qualification comprises a coherent 12-crate compiler closure with five rebuilt crates, **68 focused engine and 97 focused GPU tests passed**, full 5,000-post exact equality, and all **31 edge fixtures** exact. The broad GPU run passed **1,056**, with **16 inherited failures**, **11 ignored** and **35 filtered**. The same 16 failure names were reproduced on the unchanged baseline; there is no full-broad-green claim. The real worker frame-prefix fixture exercises original and decoded source, exact and insufficient limits, retries, owner sharing, policy/error order and sealed/shared worker behavior. Across timed IR331/IR334 observations, logical work remains 36,864,870,728 and each full-input run uses four fresh query workers.

[report.json](report.json) is the concise result. [records.json](records.json) maps historical logical paths to SHA256-checked, deterministic gzip records. Large identical outputs are stored once. Executables and libraries are represented by build/link/hash receipts; emitted program objects, source, bundles, descriptors and metadata are retained as evidence data. The minimal three-file source overlay and complete 2,814-file inventory plus 164 literal includes are checked against published Git, without copying another whole compiler source tree. Original absolute paths are historical provenance, not dependencies of offline verification.

Run from a checkout containing the published source commit:

```sh
python3 benchmarks/related-post-gen/integration/managed-worker-footprint-20261112/verify.py
```

The verifier launches no compiler or benchmark. It verifies compressed bytes, source closure/build pins, all **1,415 completed timed/resource outputs**, the separate partial attempt, seeded orders, raw timer sources, exact derived summary, bootstrap intervals, all resource windows and edge results. Primary bootstrap quantiles use indexes `[250, 9749]`; independent confirmation uses `[249, 9749]`, exactly matching the retained prospective implementations.
