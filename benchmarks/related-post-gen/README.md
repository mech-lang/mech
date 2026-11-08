# Related-post generation: Rust and Mech

The working benchmark is **IR280 with the specialized four/five-tag-row Mech source and four native query workers**. Cranelift remains the production backend; LLVM is an optional comparison. Rust parity remains unmet.

The compiler integrates the current DEFLATE work from PR #973 at `a7c216810387b049516078478165564a8c29cb56`, including the previously unmerged nullary-state foundation. Compiler source: `ccfadbdf0af69f741237f5d1099a3c3eef79386a`. The [maintained Mech source](experiments/subset3-specialized-rows4-5-native-4.mec) specializes triple/quartet generation for checked four/five-tag rows, caches shared choose terms, and keeps the original loops for all other widths. Input, scoring, tie order and Rust oracle are unchanged.

## Latest independent confirmation

Apple M3, 5,000 posts, 31 randomized fresh-process repetitions per implementation, 22 retained Rust controls. Each boundary has its own fastest eligible measured Rust denominator.

| Implementation | Continuous post-JSON | Times fastest Rust | Launch through exit | Times fastest Rust |
|---|---:|---:|---:|---:|
| Fastest eligible measured Rust | 1.517583 ms | 1.000000x | 24.099875 ms | 1.000000x |
| IR280 specialized rows, working source | **2.358833 ms** | **1.554335x** | 36.623625 ms | 1.519660x |
| IR280 original source, matched compiler | 2.606666 ms | 1.717643x | 36.353000 ms | 1.508431x |
| IR278 previous working baseline | 2.633834 ms | 1.735545x | 35.412666 ms | 1.469413x |
| Optional LLVM, unchanged original-source executable | 2.379541 ms | 1.567981x | 36.562042 ms | 1.517105x |

The combined source change wins **31/31 processing pairs in both initial and independent confirmation cohorts**. Confirmation paired processing improves **9.528%**, 95% interval **[8.948%, 10.115%]**, versus the identical IR280 compiler with original source. Processing instructions fall **49,401,422 -> 40,770,303 (-17.471%)**. Relative to IR278, paired processing improves **10.773%**.

The compiler integration alone leaves original-source native arithmetic byte-exact to IR279 and does not establish a processing speedup. The new native state support is qualified independently; no speed gain from it is invented.

Costs are retained: restoration rises **7.239709 -> 7.538334 ms** versus matched compiler. Whole-process paired change versus IR278 is **+4.424%**, 95% **[+0.591%, +8.289%]**. The user's policy permits verified phase improvements while measuring and reporting whole-process regressions. This source is adopted under that policy; the original raw trial decisions and historical IR242 selection remain unchanged.

| Confirmation resource | Original source | Specialized source |
|---|---:|---:|
| Processing allocation requests | 2,945 | 2,945 |
| Processing requested bytes | 7,919,355 | 8,006,235 |
| Processing peak additional requested heap | 5,566,127 B | 5,653,007 B |
| Restore allocation requests | 26,837 | 27,302 |
| Restore requested bytes | 12,830,999 | 13,193,328 |
| Native code | 889,768 B | 1,077,480 B |
| Cumulative native stack | 1,552 B | 2,144 B |
| Native metadata | 4,335,172 B | 4,532,218 B |
| Executable | 16,463,856 B | 16,843,648 B |
| Peak RSS | 26,329,088 B | 27,197,440 B |
| Peak physical footprint | 16,089,448 B | 16,531,816 B |

RSS and footprint are observed process metrics, not complete causal attribution to code or metadata growth.

## What is measured

This workload ports [zupat/related_post_gen](https://github.com/zupat/related_post_gen/tree/fc2982b0c929b1b701209d401a1dc30c773ef4ab): select five related posts by shared tags with exact stable ties. The optimized Mech source interns tags, builds triple/quartet posting indexes, scores candidates with word masks, and runs four emitted native numeric workers. The host adapts JSON and native snapshots; the algorithm stays in Mech.

The primary comparison is one continuous post-JSON interval including input preparation/admission, setup, allocation, worker creation/run/join, native execution, internal cleanup and result materialization. Prepared-input and preinitialized-worker timers are ineligible for that comparison. Separate launch-through-exit timing includes restoration, JSON IO, computation and destruction. Cross-cohort clock changes in unchanged controls are not new compiler gains. Standard Rust remains the exact output oracle; the fastest eligible measured Rust is the performance reference.

## Evidence and current work

[Implementation and both analyses](results/compiler-native-latest-deflate-integration-20261108.json), [working-baseline policy](results/compiler-working-baseline-policy-20261108.json), [recovery audit](results/optimization-recovery-audit-20261108.json), [initial cohort](results/comparison-native-latest-deflate-integration-20261108-5000-m3.json), [independent confirmation](results/comparison-native-specialized-rows-confirmation-20261108-5000-m3.json).

Qualification: **415 actual native CI names**, seven new upstream state/address/geometry checks, three retained mask checks, eight row/route tests, 27 image/caller tests, four explicitly executed optional LLVM checks, **133 exact scalar/DEFLATE non-timing/allocation rows**, **140 related output fixtures**, and **2,332 exact benchmark outputs** across both latency/resource cohorts. Native link audits bind every executable to its actual Object. First setup failures, the missing-dependency witness failure and disk-exhausted link are retained. No full hosted-CI-green claim.

A new diagnostic profile compares the final original and specialized Objects with the same instrumented runtime and calling-thread counters. It is separate from all promotion timings. Further work targets the remaining native instruction and runtime costs; the goal remains to meet or beat the fastest eligible Rust.

[PR #990](https://github.com/mech-lang/mech/pull/990) targets PR #973's branch. The integrated PR head and separately published working benchmark source are identified explicitly in its description. Earlier reports retain their original selection rules and measurements in [the historical record](HISTORY-through-20261108.md).
