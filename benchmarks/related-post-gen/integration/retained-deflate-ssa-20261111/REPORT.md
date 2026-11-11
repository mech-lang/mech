# Retained DEFLATE SSA compiler integration

PR #990 includes #973 through `d09e3a4b78126b888e241599752b13a514e089d7`. The retained DEFLATE compiler exists in public experiment archives as well as maintained Git code. This change reconstructs and hash-verifies all 842 files of the retained R304 closure, then integrates its portable structural SSA passes into the selected native compiler. It preserves the existing managed numeric leaf/tree route, narrow checked-write proofs, backend patches and guard ordering. It does not claim that the entire experimental compact runtime has been imported.

The imported passes refine current predicates, forward pure identity routes and loop edges with typed live-outs, preserve agreeing Boolean literals across joins, fold scoped bitwise constants, share fixed-window limits and ordered capture spans, and reuse adjacent typed addresses. They operate on current immutable CLIF words. Effects, traps, source work debits, ownership, memory permission, output publication and failure order retain their existing contracts. Independent emission ablations remain available. Numeric guard proofs run before the late imported transforms.

Two integration gaps were repaired: structural cleanup now normalizes the aliases it creates before returning, preserving idempotence; fixed-window matching accepts equivalent add-immediate subtraction under the same selected nonwrapping length proof. The first failed build and first 528-test run remain in the evidence. That run passed 526 and exposed the two subsequently fixed cases. The missing window tests were supplied locally with executable short/exact/wrapping bounds, distinct rejection, side-effect, alternate-entry, publication and retry coverage. Published address/span test patches are retained byte-for-byte.

## First matched timings

| Implementation | Post-JSON median | × fastest eligible Rust |
| --- | ---: | ---: |
| Fastest eligible Rust (`rust-ordered-mask-tables256`) | 1.514417 ms | 1.000000× |
| Published IR315, same maintained source | 1.683458 ms | 1.111621× |
| Imported IR317, same maintained source | 1.675333 ms | 1.106256× |

The median paired change is **-0.229%**, with a descriptive 95% interval **[-1.149%, +0.659%]** and 18/31 paired wins. This is **inconclusive latency evidence**. No unchanged confirmation cohort was repeated. The compiler changes are retained for verified instruction/code savings under the relaxed acceptance policy. They are not described as a confirmed latency improvement. The 1.00× goal remains unmet; this first cohort leaves 160.916 μs to its measured fastest eligible Rust.

There are 31 randomized latency rounds across 25 implementations and seven separate resource rounds across 26. All 22 Rust controls remain, including the two ineligible preinitialized scopes. All 775 latency outputs and 182 resource outputs match Rust exactly. No sample or outlier is removed. The fastest reference is selected afresh in this cohort. The source SHA is `d70a001620977aff9529884d0207fbf7952a359388988431b4102ae0465f1905`, unchanged from the independently confirmed IR315 source. Four query workers and all preparation, input admission, allocation, worker creation/run/join, internal cleanup and materialization remain within one continuous post-JSON interval.

[Every implementation's timing](ALL-TIMINGS.md) includes restoration and complete-process measurements. The earlier independent IR315 confirmation remains 1.675500 ms /1.118055× against its own 1.498584 ms Rust reference. Its ratio cannot be compared directly to the new cohort's ratio to infer a speedup.

## Resources and emitted code

| Post-JSON resource | Published IR315 | Imported IR317 | Matched Rust |
| --- | ---: | ---: | ---: |
| Retired instructions | 32,215,881 | 32,165,948 | 17,663,248 |
| Cycles | 10,379,846 | 10,387,767 | 5,862,818 |
| Allocation requests | 735 | 735 | 58 |
| Requested allocation bytes | 7,036,985 | 7,036,985 | 6,825,713 |
| Peak additional requested heap bytes | 4,871,859 | 4,871,859 | 5,648,124 |

Imported IR317 retires 49,933 fewer instructions (−0.155%) than same-source IR315. Allocations and requested/peak additional heap are identical. Cycles change +0.076%; no cycle improvement is claimed. All 49 numeric functions remain native. Generated code changes 1,023,588→1,021,528 bytes (−2,060), with reported module stack bound unchanged at 1,568 bytes. Executable size changes 17,082,512→17,066,000 bytes. The compiler simplifies 291 identity routes and 8 loop edges on this source. Other newly imported passes do not fire here, and no savings are attributed to them.

The hot query grows 31,452→31,584 code bytes while its frame remains 576 bytes. It has 26 identity-route shortcuts but no loop-edge shortcut. Overall text savings come from other bodies. Shorter control flow is not assumed to produce better register allocation or elapsed time.

Instrumented process instructions change 497,926,294→496,816,291. Peak RSS changes 27,099,136→26,820,608 bytes and lifetime physical footprint 16,351,592→16,122,240. The complete-process median is 47.630125 ms versus 48.521833 ms; its paired change is -1.952% [-4.296%, -0.874%]. Whole-process remains visible without an acceptance gate. Raw records retain every restoration/JSON, heap, instruction, cycle and residency field.

## Qualification and profile

All 528 selected native compiler contracts pass, retaining all 508 parent and 484 selected IR308 names. The 60 focused cleanup contracts pass. All 128 scalar and five DEFLATE rows match every previous non-timing/resource field. The linked executable passes 32 exact-output cases and an Object-to-executable byte audit. The actual 5000-post source passes original and decoded execution, 12 zero-budget denials preserving prior published roots, and 12 successful complete retries. Full-source zero-budget probes are not presented as exact-threshold sweeps; the small generic contracts retain exact finite-boundary checks.

Fresh same-Object diagnostic profiles of IR315 and IR317 each complete 10,001 invocations with an exact final 5000-post output. The compiled query remains the largest active reported leaf, followed by the compiled UTF8 histogram and numeric index loops. Work-escrow locking and worker/frame management remain visible. In IR317 the query has 4867 reported leaf samples, the histogram 1203, the index at header 774 has 718, and budget escrow locking 583. These are diagnostic sample counts, not additive CPU percentages or estimated latency savings. The repeated loop includes invocation/worker lifecycle and cleanup but excludes repeated input admission/export/materialization. A full-boundary critical-path study remains needed before assigning savings to serial preparation.

Measured compiler commit `b9c11064c62b6b2e2c318a3e6dd6a0f3df23e13e` is based on #973 `d09e3a4b78126b888e241599752b13a514e089d7`. All 2,755 inventoried compiler/config sources match the qualified compiler; the maintained source is byte-pinned separately. Identity is IR317/work126/ABI42/image11. Cranelift 0.131.3 and regalloc2 0.15.2 remain production; LLVM is not required. The unqualified parallel-index prototype remains isolated and is not included in this comparison.

`summary.json` indexes compressed raw observations, source inventories, archive/source reconstruction pins, exact recipes, tests, first failures, executable audits and current profiles. Large repeated output payloads are represented by their exact comparison/result hashes in the retained cohort records; the complete local raw directories are preserved.
