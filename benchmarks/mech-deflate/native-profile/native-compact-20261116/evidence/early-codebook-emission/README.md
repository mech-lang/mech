# Early codebook emission: actual units, still an experiment

**885 original source operations now belong to a separately built native unit before root SSA.** This reduces total constructed work, but this first implementation is **not selected**: the whole encoder is larger, its build is slower, and peak stack exceeds the ceiling. The selected 63,804-byte worker still holds 81/81 Roc and 62/81 libdeflate in both complete orders. No runtime win count is assigned to the new worker.

| Apple M1, same session | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Selected reference | 63,804 B | 2.209673 s | 2.220990 s | 276,072 B |
| Unsplit, same usable late policies | 67,364 B | 2.212749 s | 2.224043 s | 276,056 B |
| Early codebook experiment | 69,828 B | 2.277083 s | 2.288544 s | 292,568 B |
| Roc | 57,696 B | 2.983069 s | 2.994434 s | 191,912 B |
| libdeflate | 37,080 B | 0.481339 s | 0.492590 s | 155,752 B |

Ten balanced fresh-process builds per actor include final linking, with fixed support prebuilt for every actor. The early experiment costs a paired **68.174 ms** versus the selected compiler (90% interval **59.898–71.740 ms**, 0/10 faster), and **64.578 ms** versus the same-policy unsplit compiler (**62.281–65.413 ms**, 0/10 faster). Native phase medians are 1.212840, 1.214659 and 1.278339 seconds respectively. Parsing and source semantics remain approximately 710 and 213 ms. Source peak RSS is 392.7, 392.9 and 387.1 MB. Separate phase medians need not add.

## All-unit construction

The normal and counter-enabled compilers reproduce the same object, complete metadata and canonical bytecode for all three cases. These counts include every DFG body, clone and compaction replacement, not just the smaller root.

| Counter | Selected | Same-policy unsplit | Early experiment |
| --- | ---: | ---: | ---: |
| Allocated SSA slots | 1,407,099 | 1,400,221 | **1,145,776** |
| Constructed instructions | 497,878 | 489,920 | **360,196** |
| Constructed blocks | 174,834 | 172,386 | **128,800** |
| Largest SSA arena | 831,378 | 831,378 | **692,257** |
| Peak resident SSA slots | 955,192 | 955,192 | **800,278** |
| CFG builds | 1,335 | 1,048 | 1,147 |
| Dominance builds | 1,242 | 1,011 | 1,106 |
| Full verifications | 825 | 532 | 584 |

Versus the selected compiler, allocated SSA drops **18.6%**, instructions **27.7%**, and blocks **26.3%**. Against the same-policy control, CFG/dominance/verification counts increase. Arena positions are not register liveness; plan/proof tables, scratch maps and allocator capacity are outside these counters. Counter executions are not timings. The measured slower build shows why construction counts alone cannot establish a speed gain.

## Implementation and costs

Enable `early-region-emission` and set `MECH_COMPACT_EARLY_SOURCE_REGION=4119:5031`. The source planner checks ownership, interfaces, initialization and complete cursor/builder scopes. The root emits a coarse call; the helper emits the original source actions using the same global ownership, guards and transfer decisions. No retain/release operations are introduced at the physical call boundary. Errors return the original status and source site immediately.

Physical word liveness covers old destination owners, transfers, conditional clears and edge retirement. A forward must-zero proof follows aggregate copies and clears, intersects joins and constrains loops with the actual entry state. Numeric phase/private-handoff, word-pack and reduction bypasses have their own effect paths: a skipped source clear cannot grant a zero fact. Continuation liveness stores rows only for region-written words. Classifier and overwrite-reuse state remain excluded from this prototype; neither is active for this MechDeflate build. The feature is off by default.

The resulting boundary transports **42 input and 27 output words**; a further 27 inputs and 27 outputs are proved zero. The earlier typed-only 22/six interface does not include all physical retirement state. Root and helper use 13,371 and 1,625 word homes, with separately scoped collection state. Numeric kernels are shared and built once.

Old native-numbered controller/continuation windows are excluded until they can be remapped. The same-policy control excludes them too. Their omission changes 63,804 to 67,364 bytes before the early split. The split then removes **5,232 B** from that root, adds a **7,348 B** helper and **348 B** elsewhere/alignment: **2,464 B net growth**. The final root is 48,672 B. This is not a whole-encoder size win.

The new worker `499983ea…` passes **162 exact output/decompression/alias observations, six checked-failure cases and 12 successful recoveries**. All **17 focused planner/word tests** pass, as do nine native scalar success/error cases, including failures in the helper and after returning. The ordinary feature-off build type-checks and the selected object/metadata/bytecode reproduce exactly.

Peak active stack is **5,808 B**, exceeding the unchanged **5,280 B** ceiling by **528 B**. Root and helper frames are 3,424 and 1,072 B. Both full benchmark orders are still required for this changed worker after fixing stack. Correctness success does not confer the selected worker's 81/62 qualification.

## Next work and runtime target

The closest remaining libdeflate loss on the qualified worker is **`prefix-natural-4096`**: paired Mech/C ratios **1.04960× and 1.07280×**, requiring about **6.8% less Mech time** to win both orders. The next closest case needs approximately 25.6%. `runtime-targets/nearest.json` ranks all remaining losses from the exact qualified summaries; ratios use paired observations and displayed absolute times use separate medians.

Keep this early implementation available for repair. Profile its additional planning cost, reduce capture/retirement state and active scratch, and restore valid late sharing without relying on old native block numbers. A possible stack follow-up is sharing inactive caller scratch with the synchronous helper, but passing more pointers can create spills and must be priced. The source program is unchanged, no LLVM dependency is added, and the small-controller architecture is not complete.

## Reproduction and evidence

Build the minimal benchmark compiler with `--features early-region-emission` using the established optimized profile and toolchain from the parent README. Then run:

```sh
python3 reproduce.py --compiler /absolute/path/to/compile_profile --output /tmp/early-codebook-replay
python3 verify.py
```

Add `construction-census` to the compiler build for counters; do not use that binary for latency. `reproduce.py` restores the three pinned numeric layout profiles and checks exact artifacts. The original Mech source is `../../program.mec`. `implementation.patch.gz` preserves the complete compiler change against `228e8fa0d1e2900a7ff7ef79518593826eb97c70`; the same implementation is live in this PR. Normal compiler is `4c737bba…`, counter compiler `9752a11c…`. Build logs, source patches, all cases, complete timing observations, checked worker, failures, recoveries, stack evidence and earlier unsuccessful attempts are included. `as-run/` scripts retain original paths as provenance; the portable entry points do not depend on those scratch paths.
