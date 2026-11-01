# R75: scoped budget call boundaries — rejected

**Decision:** reject the complete scoped-context candidate. Two targeted inlining annotations recover most of R74’s regression, but the result still loses against the accepted R70 runtime. Retain the raw evidence and restore the accepted production source.

## Matched complete-call timing

Fresh Roc reference: **0.294000 ms**. All variants run the complete pinned eight-byte level12 computation.

| Runtime | Median elapsed | Multiple of Roc |
|---|---:|---:|
| Accepted R70 | 3.6076670 ms | **12.270976×** |
| Unfixed R74 | 3.7195210 ms | **12.651432×** |
| R75: two targeted annotations | 3.6311040 ms | **12.350694×** |

| Comparison | Pooled change | Improving prepared groups | Improving blocks |
|---|---:|---:|---:|
| R75 / accepted | +0.649644% | 1/4 | 13/64 |
| R75 / unfixed | -2.377107% | 4/4 | 63/64 |
| Unfixed / accepted | +3.100452% | 0/4 | 3/64 |

Four independently prepared executable triples, all six block-order permutations, 5,952 timed calls (1,984 per variant), 5,976 total. All three executables are ready before timing starts. Blocks within a triple are descriptive. The fourth prepared group favors R75 versus accepted; that contrary result is preserved. Earlier timing reference clocks are not substituted into this cohort.

## Separate instruction, cycle and allocation cohorts

| Metric | Roc | Accepted R70 | R75 | Change |
|---|---:|---:|---:|---:|
| Retired instructions | 1,364,372 | 40,395,456.5 (29.607363×) | 40,624,428 (29.775184×) | +0.566825% |
| CPU cycles | 932,484 | 11,375,020.5 (12.198623×) | 11,570,694 (12.408464×) | +1.720203% |
| Allocation requests | 95 | 6,791 (71.484211×) | 6,791 (71.484211×) | +0.000000% |
| Requested bytes | 9,021,923 | 98,112,208 (10.874866×) | 98,112,208 (10.874866×) | +0.000000% |
| Reallocations, already in requests | 0 | 162 | 162 | +0.000000% |

These CPU/heap cohorts compare accepted R70 and R75 only. R74 has primary triple observations and its separate R74 hardware cohorts; no matched three-way hardware claim is made. Full allocation censuses reconcile with heap counters. Requested capacity is not copied or live bytes. Post-call OS memory includes retained compiler/catalog/image storage and is not peak RSS. Diagnostic CPU elapsed, samples and coarse instruction windows are separately retained, including adverse results. No sample percentage is converted into a function instruction count.

## Change and observed mechanism

R74 borrows the existing work context for the synchronous entry and routes control debits through that reference. Each debit still observes live account/caps, worker ledger and observers; no RefCell borrow is held across callbacks, and no authority is cached. Four diagnostic calls consistently count 39,030 shared debit attempts: 12,450 scoped and 26,580 ordinary, with two borrowed scope entries. Those counts are diagnostic, outside performance cohorts.

R74’s actual host code introduced separate calls for the shared debit validator and the control error adapter. R75 adds exactly two `#[inline(always)]` annotations to those existing functions. It does not contain the rejected R71/R72 arithmetic factoring or R73 public-boundary annotations. The actual ARM64 audits show both extra helper calls disappearing, while the selected control visitor grows from472 to544 own static instructions against accepted R70. The audits are retained in host-assembly and control-assembly; static instruction counts are not dynamic attribution.

Removing a TLS lookup alone did not make this control path cheaper overall. The matched triple distinguishes recovery from R74 from improvement over the accepted implementation. This implementation remains adverse on both primary elapsed and whole-call retired instructions.

## Provenance, restoration and invariants

- Accepted checkpoint `9d640ebb55cd61d204b61b29a002930ff2efbb24`; actual accepted release source `3979d6e360ed6d5def3aea745df94d970c92dc4d`.
- Unfixed scoped release source `e42c2ce1daa4d7a22ff12eda5ecbcce7582a83aa`.
- R75 release source `33caedc032e26f67ac016cdc23f3f4152592d921`; change `33caedc032e26f67ac016cdc23f3f4152592d921` is exactly the two annotations versus R74.
- Disposition: All Engine/GPU production, test and CI source changed by R74/R75 is restored exactly to accepted checkpoint 9d640ebb55cd61d204b61b29a002930ff2efbb24 in 37c7ead1d69c8bb35c5292aaa333f9caadb0cc8b. The new API and five tests remain available in the committed experimental source receipts; no unused runtime API or scoped CI step is retained.
- Engine, compute, stdlib, runtime and GPU rebuild coherently under the retained exact flags. Core/math/syntax and external inputs are hash-frozen. Exact revisions, dependency hashes, source diffs and all feature/profile configurations are archived.
- IR/work/ABI/bytecode remain132/95/37/14. Fresh generated Object, descriptor and metadata are byte-identical across accepted/candidate. No scalar ceiling or resource allowance is increased.
- Complete `abcdefgh`, level12; exact raw bytes `[75,76,74,78,73,77,75,207,0,0]`, independent inflate, outputcapacity13 clear/reuse, logical work60260219686, comparison work47612, final managed bytes/nodes0. Source algorithm, tables, search limits, scratch capacities, allocation sites, allocator, aliases and evaluate_into timer are unchanged. No larger timing.

## Validation and failed attempts

**49 bounded rejected-package checks**, status **passed**. Candidate Rust unit compilation is fresh; Budget113 includes the new five context contracts. Parent Budget108 executes after full-source/library-verified reuse of its unchanged unit binary. Planner22, projection12 and shape6 are overlapping additional selections. Actual Rust consumer checks verify success/cleanup, context escape rejection and cross-thread rejection. Non-native Engine compilation passes warnings-as-errors. Four diagnostic compression calls also verify exact output and zero final managed memory.

The candidate unit compilation succeeded before saving its receipt was denied by the initial execution permissions. No tests had run. The original failure is archived; the complete staged Engine tree and linked libraries were verified unchanged, the existing archive was committed with authorized permissions, and all selections then executed on that same freshly compiled binary. There was no unnecessary recompilation and no compiler test failure.

- full GPU runtime, component236, connected8, linked12, scalar1.05, frame-allocation controls, architecture and CI selections: Not launched after the primary and counter cohorts rejected R75. Required before accepting any successor; no R75 pass is inferred from historical coverage.
- fresh full Core unit suite: Core source and libraries are frozen; prior Core371 is inherited.
- remote CI and larger inputs: Not awaited/run as requested. Complete eight-byte compression only.

## Reproduction

`python3 reconstruct.py /absolute/path/to/new-replay-directory` verifies every archived file version and replays primary triples, hardware counters, full censuses, samples, validation, instruction windows and event counts without a compiler or compressor. `reconstruction.json` records observed archive/file counts and byte-identical derived outputs. Original commands, stdout/stderr, source pins and bounded disassembly are retained with the raw archives.
