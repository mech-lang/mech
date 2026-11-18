# Combine cleanup validation while retaining graph checks

The selected `m1-literal-frame-cleanup` recipe also enables `MECH_COMPACT_VERIFY_CLEANUP_ONCE=1`. It preserves the exact **64,444 B** encoder, metadata, bytecode, fixed-support worker, **4,656 B** stack and **276,072 B** stripped deployment.

Ten balanced same-compiler builds measure a paired **6.570 ms** source-through-final-link saving, **8/10 faster**, with a paired-trial bootstrap 90% interval of **3.576–10.249 ms**. Both adverse trials remain in the raw data. The paired source CPU saving is **5.657 ms**; the native-phase saving is **3.816 ms**. These scopes differ, so the entire wall-time gain is not attributed to the native phase.

| Same-session Apple M1 build | Encoder text | Source to link | Source to strip | Source CPU | Source peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| 64,652 B control | 64,652 B | 2.196778 s | 2.207659 s | 2.214722 s | 393.1 MB |
| Separate verification | 64,444 B | 2.243242 s | 2.254121 s | 2.261760 s | 393.4 MB |
| Combined verification | 64,444 B | 2.237550 s | 2.248510 s | 2.256515 s | 393.4 MB |
| Roc | 57,696 B | 2.994681 s | 3.005370 s | 2.949604 s | 392.7 MB |
| libdeflate | 37,080 B | 0.481723 s | 0.492787 s | 0.444262 s | 71.5 MB |

The selected build takes **0.746428× Roc** and **4.639985× libdeflate** time, and still costs a paired **42.327 ms**, or **1.0193×**, versus 64,652 B. Encoder ratios remain **1.116958× Roc** and **1.737972× libdeflate**. Compiler and fixed deployment support are prebuilt for every actor; source processes, linking and stripping are timed serially.

Parsing 0.710195 s; source semantics 0.218262 s; artifact creation 0.006793 s; encoding 0.002485 s; native emission 1.233225 s; catalog join wait 0.000019 s. Link takes 0.036532 s and strip 0.010741 s. The median per-trial source time outside explicit phase scopes is 0.027699 s. Catalog CPU still runs during parsing. Independently computed medians need not add.

## Preserved validation

Two route-cleanup sites previously validated IR during cleanup and immediately repeated its full IR validation while checking cached graphs. The new combined method owns the context through cleanup and its following integrity check. It retains every pre/post-phi IR check and independently reconstructs the expected CFG and dominators for the identical cache comparisons. With optional verification disabled, it retains the unconditional full `Context::verify` fallback. The original call pair remains available as the off path. No runtime proof, resource admission, error status or unsafe boundary changes.

All **19 focused tests** pass: two validation tests, 15 SSA-builder tests and two native cold-tail tests. The validation tests cover 16 cleanup cases across both verifier modes, pruning, phi edits and invalid IR. An independent test corrupts CFG and dominator caches and checks rejection with each verifier setting. The same source file is registered in the ordinary GPU owner suite. The CI checkout regression also passes and is wired into static CI.

## Source and exact workers

Measured compiler source: `1201a97b6a206f7e17c878f108055c2ef0f02a76`. Registering the tests in source `567f6dcac24b7112ad891d2e24b354a269b86dfd` reproduces the **entire measured compiler binary exactly**, SHA256 `3bcb51ef0789e65179a7efbaa73dd93e50da4d439b4dbbe4b3cdf874628454ba`. The source recipe pins that current source; the original acquisition retains its original source identity.

Both separate and combined paths reproduce qualified worker `f77ad5066a61f30cf67f714e5edc990efe295682e9de1d8bf36ee9a1a93041c6` and stripped hash `a2b16eed3f55512674a724a968c39f1d86ed3140d7abf7e084871036555db698`. Its existing runtime result stays **81/81 Roc and 62/81 libdeflate point wins in both full orders**, covering 74 and 56 distinct winning contracts. Reverse precision still supports 61 C wins; the two established control slowdowns and ten uncertain cases remain unchanged.

The full named source rebuild passes **162 success observations, six failure cases and 12 exact recoveries**. Its newly rebuilt host-support worker is `c101560b7a46707aa2549071e81f1da98deb5232b2f744039972cc7e819f1c4d`; runtime wins were not measured on that worker. The original Mech source is unchanged. The smaller recipe stays opt-in beside the maintained 71,192 B frame-copies control. No LLVM dependency is added.

Run `python3 -B verify-results.py` here to recompute all medians, ratios, paired effects, the bootstrap interval and recorded identity checks from raw records on any Python host.

[All measurements](deployment.json) · [Every trial](build-timings.csv) · [Raw acquisition](build-comparison.json.gz) · [Plan and inputs](build-plan.json) · [Source reproduction](registered-source-reproduction.json) · [Exact workers](exact-workers.json) · [Source checks](source-rebuild.json) · [Validation and prior setup diagnostic](validation.json) · [Pre-change profile](pre-change-profile.json) · [Implementation plan](plan.md)

The next work should use a whole-build profile and the remaining checked-root construction and transport costs. The root still occupies 47,428 B, exceeding C's complete encoder by 10,348 B. These two cleanup experiments are complete; their measured effects and remaining costs are recorded rather than treated as a route to the entire 1× C target. Review concurrent upstream compiler work before selecting the next structural change.
