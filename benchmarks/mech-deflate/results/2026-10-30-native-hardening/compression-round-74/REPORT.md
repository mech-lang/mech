# R74: scoped current-context experiment — rejected

**Decision:** reject this runtime candidate. The lifetime-bound context is correct in the executed controls, but elapsed time and instructions regress. R75 tests only the two newly outlined boundaries found in the actual host code. The accepted runtime remains R70 until a successor passes qualification.

| Metric | Roc | Accepted baseline | R74 candidate | Change |
|---|---:|---:|---:|---:|
| Complete-call elapsed | 0.292000 ms | 3.6136460 ms (12.375500×) | 3.7193335 ms (12.737443×) | +2.924678% |
| Retired instructions, separate cohort | 1,364,372.0 | 40,390,829.0 (29.603971×) | 42,280,354.5 (30.988876×) | +4.678105% |
| CPU cycles, separate cohort | 935,625.5 | 11,444,733.0 (12.232173×) | 11,913,926.5 (12.733649×) | +4.099646% |
| Allocation requests, separate cohort | 95 | 6,791 | 6,791 | +0 |
| Requested bytes, separate cohort | 9,021,923 | 98,112,208 | 98,112,208 | +0 |
| Reallocations, included above, separate cohort | 0 | 162 | 162 | +0 |

The primary measurement has four independently prepared pairs, 3,968 timed calls and 3,984 total calls. Only 0/4 groups and 3/64 blocks improve. Blocks within a prepared pair are descriptive, not independent experiments. CPU counters, allocations, full stack censuses, samples and instruction windows are separate serialized cohorts. No percentages from independent cohorts are multiplied.

## Mechanism and design

The callback borrows the existing thread-local context with a lifetime that cannot escape its scope or cross threads. No account, cap or reservation authority is cached. Each debit uses the original live account/limit checks and original failure order. A borrowed reference is carried by Invocation; no RefCell borrow is held across callbacks. Ordinary entry behavior and non-native builds remain supported.

Four separate diagnostic calls agree on 39,030 shared debit attempts: 12,450 routed through the borrowed context and 26,580 ordinary attempts. There are two scope entries. These counts do not claim that all ordinary attempts perform TLS lookup. Diagnostic counters are excluded from performance measurements.

The actual linked ARM64 audit identifies two added call boundaries. The ordinary debit is 158 own static instructions in the accepted baseline; R74 has 104 ordinary and 64 scoped own instructions, each calling the shared debit helper (130 own). The control visitor also calls the newly outlined error adapter (122 own) instead of containing that body. Static counts are not executed instruction counts or a complete regression attribution. Exact symbols/instructions are retained in host-assembly and control-assembly.

R75 changes only the inlining annotations of that shared debit helper and control error adapter. It is compared with both this unfixed binary and the accepted baseline, so merely recovering the R74 regression cannot count as a speedup.

## Revisions and invariants

- Accepted checkpoint: `9d640ebb55cd61d204b61b29a002930ff2efbb24`; actual accepted release source: `3979d6e360ed6d5def3aea745df94d970c92dc4d`.
- R74 actual release source: `e42c2ce1daa4d7a22ff12eda5ecbcce7582a83aa`. Engine, compute, stdlib, runtime and GPU are coherently rebuilt; Core/math/syntax and other project dependencies remain hash-frozen.
- Source commits: `955c7d8c6` (borrowed context and five controls), `180abd6b8` (control admission integration), `e42c2ce1d` (CI selection for the five tests). These remain experimental pending R75; this report does not claim a production restoration.
- IR/work/ABI/bytecode remain 132/95/37/14. Fresh Object, descriptor and metadata are byte-identical to the accepted baseline. The pin and library manifests retain exact hashes and feature/profile flags.
- Complete eight-byte `abcdefgh`, level12; bytes `[75,76,74,78,73,77,75,207,0,0]`; independent inflate; output capacity13 with clear/reuse; logical work60260219686 and comparison work47612; final managed bytes/nodes0. Source, tables, search limits, scratch capacities, allocator, input aliases and evaluate_into timer are unchanged. No larger timing is performed.

## Validation and disposition

Bounded rejected-package checks: **51**, status **passed**. Fresh candidate Rust compilation executes Budget113 including five new controls; the parent executes Budget108 after full staged-source and dependency proofs for its cached compiler output. Planner22, projection12 and shape6 are additional overlapping selections. The preliminary Engine-only API witness also executes Budget113/scoped5/planner22; it is identified separately from integrated-source results.

The five new cases cover ordinary/scoped boundary equivalence, live account/caps and caller/foreign-account precedence, observer high-water with nesting, worker denial atomicity/retry/settlement, and unwind restoration/recovery. Actual cross-crate Rust consumers succeed with cleanup and reject context escape and cross-thread use at compile time. Non-native Engine compilation passes warnings-as-errors. No fresh Core unit execution is claimed.

- full GPU runtime, component236, connected8, linked12, scalar1.05, frame-allocation controls, architecture and CI selections: Not launched after the primary and counter cohorts rejected R74. Required before accepting any successor; no R74 pass is inferred from historical coverage.
- fresh full Core unit suite: Core source and libraries are frozen; prior Core371 is inherited.
- remote CI and larger inputs: Not awaited/run as requested. Complete eight-byte compression only.

Introduced setup failure: the initial generated driver had a Python whitespace syntax error before integration build or timing; it was corrected before continuation. No production compiler or semantic failure is hidden. The failed setup and its disposition are retained in prior-failures.json.

## Reproduction

`python3 reconstruct.py /absolute/path/to/new-replay-directory` verifies every archived file version and reconstructs the scorecard, allocation ledger, sample summary, instruction windows, validation and diagnostic counts without executing a compiler or compressor. Raw calls, complete stack censuses, build/test commands, sources, assembly and SHA-256 manifests are committed with the packet. `reconstruction.json` records the observed replay result.
