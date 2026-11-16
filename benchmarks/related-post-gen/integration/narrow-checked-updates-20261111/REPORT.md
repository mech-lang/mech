IR315 extends the existing current-read/update proof to checked U8/U32 arithmetic and unsigned storage widths. A successful checked narrow addition is the same mathematical sum in the proof's U64 domain; it proves that an exact current cell plus one changes. The generated store can then omit its repeated bounds check, old-cell load and equality test. Narrow overflow still executes at the original scalar operation. Arithmetic-guard removal remains U64-only. Unchanged cells, other indices, stale values, unknown calls/joins and unproved ownership retain their original checks.

Independent writes preserve facts only for distinct logical carrier homes under the existing owner/COW witness. Sharing physical backing does not grant independence by itself. The same first-change copy, publication, failure ordering, finite admission and retry contracts remain. The image identity advances to IR315; work126/ABI42/image11 and all production counters remain.

For the same U32-count source, proved increments rise 14→43 and native code falls 1,047,324→1,025,960 bytes. For U32 postings, cross-write increments rise 6→12. The combined source recovers 8→43 increments and shrinks 1,046,936→1,023,588 bytes. Every variant retains 49 emitted numeric functions. The maintained U64 source's full native text is byte-identical to IR314.

## Confirmed selected source

The maintained source is now the exact measured combined U32-count/U32-posting source, SHA-256 `d70a001620977aff9529884d0207fbf7952a359388988431b4102ae0465f1905`. Prefix sums, key arithmetic, positions and ranks remain U64. Existing ordered/distinct rows of at most eight tags and vocabulary at most 64 prove each subset occurs at most once per post. The fast path also requires posts<=u32::MAX; larger inputs take the existing generic scoring fallback. No fixture data, post count, dictionary contents or outputs are precomputed. Four native workers are created, run and joined inside each invocation.

The first cohort contains 31 seeded randomized rounds across 31 variants. Counts-only, the predeclared primary candidate, is inconclusive. Postings-only is also inconclusive. Only the predeclared combined candidate improves against its same-source IR314 parent, published maintained source and same-compiler U64 control. It advances to an independent 31-round confirmation with fresh preregistered seeds. All 22 Rust controls remain in both; the two preinitialized timer scopes remain ineligible as post-JSON references. No unchanged adverse alternative is retried.

| Cohort | Published IR314 U64 | IR315 U64 control | Selected IR315 combined | Fastest eligible Rust | Selected × Rust |
| --- | ---: | ---: | ---: | ---: | ---: |
| First | 1.707417 ms | 1.703250 ms | 1.678834 ms | 1.507834 ms | 1.113408× |
| Confirmation | 1.709875 ms | 1.702167 ms | 1.675500 ms | 1.498584 ms | 1.118055× |

First fastest Rust is the Apple M3 ordered-mask/tables256 control; confirmation fastest is its untuned counterpart. The denominator is chosen afresh, never combined from a different cohort. The authoritative timer is one continuous post-JSON interval including input admission, source preparation, allocations, native work, worker creation/run/join, internal cleanup and materialization. Restoration and JSON IO remain in complete process.

| Paired post-JSON change, median and descriptive 95% interval | First | Confirmation |
| --- | ---: | ---: |
| Selected vs published U64 | -1.703% [-2.024%, -1.100%] | -1.379% [-2.441%, -0.545%] |
| Selected vs same-source IR314 | -2.104% [-2.864%, -1.699%] | -2.058% [-2.617%, -1.256%] |
| Selected vs same-compiler U64 | -1.330% [-1.761%, -0.930%] | -1.200% [-1.875%, -0.750%] |

Intervals use 20,000 bootstrap resamples with seeds declared before capture. Every first sample and outlier remains. There are **1,798 exact latency outputs** and **420 exact outputs from separate resource cohorts**. [All first and confirmation timings](ALL-TIMINGS.md) include every Rust and Mech row. **The 1.00× goal remains unmet; confirmation leaves 176.916 μs to the fastest measured eligible Rust.**

## Resources and sizes

| Post-JSON resource median | Published U64 | IR314 same combined source | Selected IR315 combined | Matched Rust |
| --- | ---: | ---: | ---: | ---: |
| Retired instructions | 31,705,333 | 32,429,018 | 32,232,107 | 17,666,882 |
| Cycles | 10,508,696 | 10,265,778 | 10,306,777 | 5,871,818 |
| Allocation requests | 720 | 735 | 735 | 58 |
| Requested bytes | 7,326,125 | 7,036,985 | 7,036,985 | 6,825,713 |
| Peak additional requested heap | 5,357,407 | 4,871,859 | 4,871,859 | 5,648,124 |

Selected versus published U64 saves 9.063% peak additional requested heap and 3.947% requested bytes. Allocation requests rise 720→735. Instructions remain above the U64 source and are 1.824× matched Rust, despite the elapsed gain; resource regressions are not hidden. The compiler alone reduces the same combined source's confirmation instructions 32,429,018→32,232,107. Hardware counters aggregate all participating threads, so their ratios are not elapsed ratios.

Selected native code is 1,023,588 bytes versus U64's 1,031,964. Reported module stack bound is 1,568 versus 1,584 bytes. Executable size is 17,082,512 versus 17,181,584 bytes. The selected query is 31,452 bytes versus 32,192, with 7,863 versus 8,048 static instructions and 1,572 versus 1,730 static stack references; alignment padding is excluded. Its reported frame grows 560→576 bytes. Static references are not dynamic spill counts. This query text is unchanged from IR314's same combined source; the new checked-update proof shrinks producer/scatter code.

Restoration remains visible: selected 162,845 requests /189,837,445 bytes /334,212,987 instructions, versus published 167,071 /193,372,864 /341,578,639. Instrumented whole-process instructions are 496,996,448 versus 503,894,386; peak RSS 27,049,984 versus 27,836,416, lifetime physical footprint 16,302,440 versus 17,088,872 bytes. Raw reports retain all cycles, heap stages, residency and footprint fields.

Confirmation launch-through-exit median is 46.350500 ms for selected Mech, versus 47.420416 ms for published U64. Its paired change is -2.166% [-3.212%, -0.638%]. Fastest complete-process Rust is the ordered-mask-reuse control at 21.954875 ms; it differs from the post-JSON reference. Whole-process remains visible and is not an acceptance gate.

## Qualification and current profile

All 508 selected native compiler contracts pass, retaining all 505 IR314 and 484 IR308 names. The 22 focused update/traversal tests include actual selected U8/U32 emission and ablations, overflow, bounds, unchanged/different cells, physical aliases with logical independence, original/decoded/restored execution, six finite dimensions, failure publication and retry. All 128 scalar and five DEFLATE rows preserve every non-timing/resource field. Four linked variants pass 32 exact cases each and their Object byte audit.

The retained U64-specific backing/handoff witness is pinned to `experiments/related-u64-four-workers-ir314.mec`; it also passes after source promotion. The selected source uses the separate original/decoded and linked checks below.

The selected complete 5000-post source separately passes original and decoded artifact execution, 12 finite denials with prior outputs preserved, and 12 successful complete retries. Linked restoration is exercised by every linked-output/cohort call. Generic tiny-source finite tests find exact successful boundaries; full-source probes deliberately test zero-budget denial and retry. These scopes are distinct and are not presented as full-input exact-threshold sweeps.

Fresh same-Object profiles of U64 and selected combined code each finish 10,001 invocations and an exact final 5000-post output. The native query is the largest active reported leaf, followed by the UTF8 histogram and native index loops. Budget escrow locking, frame allocation and worker lifecycle remain visible. The profile loop excludes repeated input admission/materialization; sample categories include short-lived-worker disturbance and are not exact CPU or latency shares. A fresh full-boundary phase study is still needed to divide the remaining critical-path gap.

Initial qualification preparation failed on a Rust lifetime in the new test helper and then on source-fixture cast syntax. The first code, build manifests and failing receipts remain. The corrected tests pass; these failures started zero benchmark calls. No production behavior was weakened to bypass them.

The branch includes #973 through `6f50ac6d77fc05ef292e746fa32ae966e2c251fa`. All 2,737 inventoried compiler/configuration files match the measured compiler after the documentation-only rebase, and the maintained Mech source is byte-identical to the measured candidate. Cranelift remains production; LLVM is not required. The checked DEFLATE backend/SSA-width work remains integrated; experimental compact runtime bundles have not replaced this compiler wholesale.

`summary.json` indexes the compressed raw records, full hashes, build recipes, source inventories, failures, profiles, timings and selection decision. No parity, merge or green-hosted-CI claim is made.

The next source experiment is to construct triple and quartet indexes as two explicit native tasks, keeping both result banks independent and preserving coverage/fallback selection. Any speculative work must be explicit and measured; no gain is assumed. The indexed follow-up plan records the dependency, native-worker and qualification obligations.
