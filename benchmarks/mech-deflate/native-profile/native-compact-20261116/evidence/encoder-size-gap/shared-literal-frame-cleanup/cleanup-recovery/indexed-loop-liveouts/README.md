# Index loop live-out uses once per immutable graph

The selected `m1-literal-frame-cleanup` source recipe now also enables `MECH_COMPACT_INDEX_LOOP_LIVE_OUTS=1`. It preserves the exact **64,444 B** encoder, complete metadata, canonical bytecode, fixed-support worker and stripped deployment. The existing M1 runtime result remains **81/81 Roc and 62/81 libdeflate point wins in both orders**, with 74 and 56 distinct winning contracts. Reverse precision still supports 61 C wins; the two established control slowdowns and ten uncertain cases in the parent replay remain unchanged.

The loop-route pass formerly walked reachable instructions for each skipped SSA definition. It now optionally shares the existing acyclic route pass's immutable use inventory and stopped-reachability cache. The inventory is built lazily, used only while collecting plans on one unchanged graph, and dropped before any rewrite. Current dominance, definition-stop semantics, branch arguments, aliases, shared-merge refusals, effect/exception refusals and SSA repair are preserved. All existing verification calls remain in place. The original traversal is the off path for the same-compiler ablation.

## Measured effect

Ten balanced fresh builds per actor use one prebuilt compiler and the same prebuilt deployment support. All actors run serially. The indexed path saves a paired median **46.909 ms**, with **9/10 faster**, and a paired-trial bootstrap 90% interval of **41.430–60.082 ms**. The one adverse trial, **111.600 ms slower**, is retained. Source CPU falls by a paired median **48.249 ms** and the native phase by **49.791 ms**, supporting a reduction in compiler work.

The 64,444 B indexed build still costs a paired median **48.337 ms**, or **1.0221×**, versus the 64,652 B control. This recovers about half of the extra cleanup cost in this cohort. The difference between two actor medians is not the paired effect.

| Same-session actor | Encoder text | Source to link | Source to strip | Source CPU | Source peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| 64,652 B control | 64,652 B | 2.189798 s | 2.201042 s | 2.218932 s | 393.0 MB |
| 64,444 B original traversal | 64,444 B | 2.281106 s | 2.292151 s | 2.308613 s | 393.0 MB |
| 64,444 B indexed traversal | 64,444 B | **2.237580 s** | **2.249211 s** | 2.260798 s | 393.0 MB |
| Roc | 57,696 B | 2.995987 s | 3.007166 s | 2.955014 s | 393.9 MB |
| libdeflate | 37,080 B | 0.483521 s | 0.494623 s | 0.445641 s | 74.3 MB |

The selected source-to-link ratios are **0.746123× Roc** and **4.627236× libdeflate**. Encoder ratios remain **1.116958× Roc** and **1.737972× libdeflate**. Stripped deployment stays **276,072 B**, and peak native stack remains **4,656 B**.

Selected compiler phase medians are **711.901 ms parsing**, **219.357 ms source semantics**, **6.817 ms artifact creation**, **2.504 ms encoding**, and **1,231.053 ms native emission**. Catalog join wait is **0.023 ms**; catalog CPU still runs during parsing. Link takes **34.775 ms** and strip **10.813 ms**. The median per-trial source time outside the explicit phase scopes is **27.522 ms**. Independently calculated medians need not add to the complete elapsed median.

[Complete measurements](deployment.json) · [Every build timing](build-timings.csv) · [Raw acquisition](build-comparison.json.gz) · [Plan, inputs and balanced orders](build-plan.json) · [Current profile](../profile/README.md)

## Source, checks and exact identities

Compiler source: `d49f735b853504725de7b89f81f37bde0298fe05`. Compiler SHA256: `a96172619c5fce80baf71a5efaa9f0e662d05de68a4f1dd2f7b950008b7caa7c`. The measured compiler uses the normal level-3 source recipe and nightly `2026-03-03`; the original Mech program is unchanged.

All **113 control-flow tests pass**, including independent old-traversal comparisons across every value/target in seven fixtures and exact on/off IR, report and repair comparisons. The root test build uses level 0 only for its unrelated full math dependency; the compiler/GPU tests and the measured benchmark compiler use level 3. Earlier setup interruptions, the corrected fixture's stale-graph failure and the mixed-workspace cache recovery are recorded in [validation.json](validation.json) and [complete validation logs](validation-logs.tar.gz).

The source-built named command passes **162 output/decompression/alias observations**, **six injected failure cases** and **12 exact recovery calls**. Its newly rebuilt host-support worker is `c0334d84410a20727043d217154132a9448652210233b69c135478221a0a2e34`; runtime wins were not measured on that worker. The separately relinked fixed-support worker exactly reproduces the qualified `f77ad5066a61f30cf67f714e5edc990efe295682e9de1d8bf36ee9a1a93041c6`. All measured indexed/unindexed deployments reproduce the same stripped hash `a2b16eed3f55512674a724a968c39f1d86ed3140d7abf7e084871036555db698`.

[Artifact screen](artifact-screen.json) · [Exact worker links](exact-workers.json) · [Source rebuild](source-rebuild.json) · [Successes](source-successes.json) · [Failures](source-failures.json) · [Recovery](source-recoveries.json) · [Previous recipe](previous-recipe.json)

The maintained frame-copies control remains 71,192 B; this smaller recipe stays opt-in. Reproduce it with the parent directory's `rebuild.py --variant m1-literal-frame-cleanup` command. Compiler and support bootstrap builds are outside the matched source-through-link timing boundary. No LLVM tool or dependency is added.

Run `python3 -B verify-results.py` in this directory on any Python host to independently recompute the cohort, paired effect, uncertainty interval, phases and recorded identity checks from the published raw files.
