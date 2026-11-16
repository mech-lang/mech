# Unselected worker payment experiments: IR332 and IR333

**IR331 remains selected.** Extending paid cleanup to workers (IR332) and
grouping preflighted cold-alias payments (IR333) reduces measured instructions
but establishes no latency improvement. The experiments remain archived for
future combinations. There was no independent confirmation or adverse-result
rerun for either candidate.

This historical packet is pinned to the published IR331 source commit
[`f361f91f298b87ebb9bee880930951fdf79b0d5f`](https://github.com/mech-lang/mech/commit/f361f91f298b87ebb9bee880930951fdf79b0d5f)
plus retained source overlays. It contains no later IR334 engine candidate,
source, metadata or timing. Cranelift remains the backend; no mandatory
dependency is added.

## Latency

Thirty-one prospective randomized rounds cover thirty implementations,
producing **930 exact full outputs**. All runs use fresh ordinary executables,
the fixed 5,000-post input, and the complete unchanged Rust Gold result. No
observation is removed. “× Rust” means Mech time divided by Rust time; lower is
better. The reference is the fastest eligible measured implementation, not a
claim about every possible Rust implementation.

| Same-cohort implementation | Post-JSON ms | × fastest eligible Rust | × original serial Rust |
| --- | ---: | ---: | ---: |
| Managed host-v2 control | 1.613250 | 1.32732 | 1.06699 |
| Managed short-overlap | 1.485166 | 1.22194 | 0.98228 |
| **Managed IR331, retained selection** | **1.448542** | **1.19181** | **0.95806** |
| Managed IR332 paid-cleanup | 1.460708 | 1.20182 | 0.96610 |
| Managed IR333 paid-alias-v2 | 1.450792 | 1.19366 | 0.95955 |
| Retained compact IR320, separate compiler mode | 1.320625 | 1.08656 | 0.87345 |
| Fresh four-worker Rust, Apple M3 target | 1.215417 | 1.00000 | 0.80387 |
| Fastest original twenty serial Rust controls | 1.511958 | 1.24398 | 1.00000 |

Relative to IR331, IR332's median paired change is **+0.777%** with bootstrap
95% interval **[+0.548%, +1.638%]**: slower in this cohort. IR333's paired change
is **−0.196%**, interval **[−1.080%, +0.147%]**: the interval spans zero. A ratio
of medians and a median of paired ratios need not have the same sign.

The retained compact compiler mode is included in the same cohort and is
**1.08656×** the newly measured fastest Rust. This row does not imply managed
and compact modes have identical contracts. Its own adapter, receipt and
boundary are retained in the config. Managed IR331 remains **1.19181×** that
Rust reference, so the managed target is unmet.

Post-JSON timers include input preparation, worker creation/run/join, numeric
execution, internal cleanup and result materialization. JSON decoding and
artifact restoration are separate. Historical managed final input/output
destruction remains outside that timer; the compact adapter includes its input
descriptor and output-backing destruction. Whole-process elapsed time covers
launch through exit and is retained separately, secondary and non-gating.
Two preinitialized Rust controls remain visible but are ineligible as complete
post-JSON references. Twenty serial plus two fresh four-worker Rust controls
are eligible. Both default and Apple M3 Rust builds remain in the inventory.

`first-paid-workers/paired-bootstrap-protocol.json` preserves the derivation:
per-round `100*(candidate/IR331-1)`, all thirty-one pairs, median statistic,
10,000 resamples, seed 202611114700 reset for each implementation, sorted
bootstrap endpoints at indices 250 and 9749. These intervals are diagnostics on
one shared Apple M3 machine, not independent-machine replication.

## Resources

Seven separately randomized resource rounds cover twenty-nine implementations:
four managed candidates and all twenty-five retained Rust resource rows,
producing **203 exact outputs**. The reference follows the ordinary latency
cohort above. Instrumented timings do not choose a speed winner.

These medians cover the complete **all-thread post-JSON** window:

| Implementation | Allocation requests | Requested bytes | Additional peak requested heap | Instructions | Cycles |
| --- | ---: | ---: | ---: | ---: | ---: |
| Host-v2 | 735 | 7,036,985 | 4,871,859 | 31,512,136 | 10,129,123 |
| IR331 | 725 | 6,376,193 | 4,211,067 | 29,974,067 | 9,226,162 |
| IR332 | 725 | 6,376,193 | 4,211,067 | 29,749,600 | 9,046,855 |
| IR333 | 725 | 6,376,193 | 4,211,067 | 29,691,144 | 9,048,071 |
| Fresh four-worker Rust, Apple M3 | 78 | 7,026,975 | 5,721,694 | 18,184,270 | 6,707,553 |

IR332 removes 224,467 instructions (0.749%) relative to IR331; IR333 removes
282,923 (0.944%). Allocation requests, requested bytes, additional peak requested
heap and freed bytes remain unchanged across these three managed revisions.
Instruction reductions alone do not establish a latency gain. Instrumentation,
thread scheduling and CPU frequency can affect cycle and timing observations.

Requested heap counts allocator allocations and reallocations, excluding
allocator internals, mappings and OS thread stacks. Additional peak heap is
above the initial live requested bytes of the window. Freed bytes are retained;
the number of free calls is unavailable. All accounting underflows are zero.
RSS/physical-footprint endpoints, separate restore and JSON windows, and
launch-through-exit `/usr/bin/time -lp` counters retain their own scopes.

Whole-process peak RSS medians are 26,230,784 bytes for IR331, 26,214,400 for
IR332 and 26,492,928 for IR333; these are not post-JSON heap peaks. Checked
native metadata reports 1,021,740 code bytes and a 1,568-byte root stack frame
for all three. Per-function stack entries are retained separately; the root
frame is not total process stack allocation. Ordinary executable sizes are
17,108,464, 17,108,768 and 17,107,104 bytes respectively. Instrumented
executables and Mach-O sections/segments have separate retained inventories.
No LLVM sizing tool or dependency is required.

## Changes and qualification

IR332 permits an exact, successful paid lexical-cleanup prefix on the existing
worker account. Aggregate denial spends no new work and retains the original
ordered path. IR333 reuses that same non-cloneable receipt to group the exact
cold-alias retirement fees after private setup. Current owner, type, generation,
ordinal and ledger checks remain; logical prices, failures, retry, arithmetic,
index checks and admission are preserved. The benchmark source and four-worker
adapter are unchanged.

The read-only worker-scope audit establishes source eligibility, not execution
counts. The **180 shared-lock saving estimate is static**, for one successful
cold-alias suffix across four workers. Successful dynamic batch counts and
actual lock savings were not measured. The actual complete instruction totals
above must not be converted into that unmeasured count.

Both timed candidates pass complete 5,000-post qualification and all thirty-one
maintained edge cases. The original IR332 focused receipts retain nineteen
passing tests, one failed cleanup-owner fixture, and a selector that matched
zero tests. That fixture retained a root budget handle, leaving 200 metadata
bytes live at its final zero assertion. The test helper was corrected. The
equivalent alias helper pattern was corrected by static inspection before its
first test execution; **there was no second observed owner-leak failed run**.
These were test-lifetime corrections, not production ownership fixes.

IR333 v2's new focused contracts retain seven passes and one failure: the new
cold-alias positive fixture did not exercise an actual compiled prefix. The
v2 broad native run explicitly skips that broken fixture and records **1,059
passed, sixteen inherited failures, eleven ignored, thirty-six filtered out**.
The sixteen names match the earlier unchanged baseline. This is not broad green.

The v3 override corrects only the positive fixture's test source and passes that
one focused test. Its product build stages and libraries are unchanged from
v2. There is no full broad v3 rerun or independent latency confirmation. The
packet retains original failures, source corrections, build receipts and exact
test scopes rather than assigning later passes to an earlier source snapshot.

## Provenance and offline verification

`report.json` contains derived rows and scope. `records.json` maps logical
historical paths to content-addressed gzip blobs with compressed/uncompressed
SHA-256 and size. Repeated output bytes share one blob. Prospective configs,
randomized orders, stdout, stderr, elapsed/exit receipts and full outputs are
retained. Executables and libraries are represented by hashes/build closures;
their binary copies are omitted. Generated native objects/bundles remain as
evidence data.

`publication-source-closure.json` pins each measured source inventory to
published IR331 plus minimal frozen overlays: four changed files for IR332 and
nine for IR333. The final nine-file `source.patch` includes corrected test
fixtures; the measured v2 executable's production source is identical, while
its earlier test-only files are retained and labeled separately. Core, engine
and stdlib are verified in their non-GPU configuration scope. Historical GPU
entries in that Core closure's inventory were not compiled by it. No repeated
full GPU snapshots or current experimental source are needed.

The independent source audit verifies 2,813 inventory entries, patch application
against published base bytes, source transitions and literal test includes. The
recorded library-test snapshots include their actual `related_optimized.mec`
input. Missing example-only includes belong to files outside the recorded
`rustc --test hosts/gpu/src/lib.rs` build and are explicitly labeled in the audit.

From a repository checkout containing `f361f91f`, run:

```sh
python3 benchmarks/related-post-gen/integration/managed-worker-payments-20261112/verify.py
```

This checks every retained blob, all 1,133 timed outputs, timer/order receipts,
reference selection, paired medians/bootstrap intervals, scoped counters,
qualification results and overlays. It applies the nine-file patch in memory
without fuzz and compares the result byte-for-byte. It launches no compiler or
benchmark. Successful output is retained in `verification.json`.

`assemble.py` documents the historical input roots and refuses to overwrite an
existing packet. The packet has 2,923 logical records in 1,177 unique blobs,
totaling 9,933,548 compressed blob bytes.
