# Bounded packed vector cost and ordinal study

The compiler now integrates the packed-key prototype under explicit unsigned bounds. **The qualified selection remains 63,612 B / 81 Roc / 62 libdeflate. No new C win or compile-time gain is established.** Every prior peer win observed in all fourteen discovery orders survives; these panels do not establish a new full score or confidence qualification.

The original packed probe takes **0.89% / 0.88% less time on natural 4 KiB** than its matching control, reaching **1.005062× / 1.008491× libdeflate**. Carrying the packed indices removes more instructions but does not improve this latency consistently. Its distribution-change slowdown is repaired by a broader profile: the carried version takes **0.39% / 0.57% less time** than its matching revised-profile control. Both stages, all controls and every adverse observation remain included.

Compiler source: [`f3d74d9488`](https://github.com/mech-lang/mech/commit/f3d74d9488095a1ef4b3dbc36a03d6265f8f14c9). `MECH_ORDERED_PACKED_COST_INDEX=1` enables the experiment; `MECH_ORDERED_VECTOR_CARRY_LANES=1` adds carried indices. The original 2,719-line Mech codec and canonical bytecode are unchanged. No LLVM dependency is added.

The packed key is `(saturating_u16(cost) << 16) | (index + 1)`. The incoming best has ordinal zero, preserving the old choice on equal cost. The earliest original index breaks candidate ties. Packing requires `best <= 65535` and absolute `stop <= 65535`; a same-plan static fact may discharge the stop bound. Large candidate costs cannot beat an admitted best. A guard miss retains the original optimized vector loop. Expressions that use the index in their cost, pointer-carry recipes and unsupported targets keep that loop. Scalar remainders, errors and exact traversal accounting are unchanged.

The shared vector-loop emitter preserves construction order for disabled and legacy settings: **all three artifacts match exactly for sixteen older recipes**, including index-only and mapped-item options. This change adds no unsafe production code.

## All discovery results

Times are Mech / peer; lower is faster. Each cell contains forward / reverse results from a complete 21-case, 24-balanced-block order. The original four variants compare against their matching old word/probe controls. The revised-profile control compares against the old probe; both revised packed variants compare against that revised control. See `result.json` for every case, all control ratios, profiles and artifact/source pins.

| Variant | Encoder text | Natural 4 KiB / C | Distribution time / held control |
| --- | ---: | ---: | ---: |
| `packed-key-probe-profiled` | 63,660 B | 1.005062× / 1.008491× | 1.025129× / 1.036347× |
| `packed-key-word-profiled` | 63,020 B | 1.024351× / 1.024070× | 1.040882× / 1.041999× |
| `packed-key-carry-probe-profiled` | 63,676 B | 1.007900× / 1.007877× | 1.037018× / 1.042360× |
| `packed-key-carry-word-profiled` | 63,036 B | 1.012119× / 1.017657× | 1.032747× / 1.043648× |
| `extended-control-probe-profiled` | 63,436 B | 1.002609× / 1.023989× | 0.998785× / 0.998760× |
| `packed-extended-probe-profiled` | 63,644 B | 1.006192× / 1.018782× | 1.007101× / 1.005799× |
| `packed-extended-carry-probe-profiled` | 63,660 B | 1.008837× / 1.019264× | 0.996137× / 0.994252× |

Untimed counters cover the original six cases and an extended eight-case cohort adding distribution change and overlap. In the raw probe scorer, natural 4 KiB falls from **3,414,780 to 3,379,620 instructions**, and to **3,331,665** with carried indices. Branches change from **569,130 to 573,275**, or **555,990** with carry. Stack loads/stores remain **17,305 / 18,470**. Packed reductions use one horizontal minimum; guard misses use the old two. No scorer vector path executes on distribution change: its instruction counts are identical. Adding that case to the block-layout profile recovers the observed latency penalty; this is evidence for a layout effect, not a hardware-counter attribution.

## Complete executable builds

Apple M1, macOS 15.6.1, arm64. Ten balanced serial fresh-process trials per actor include source compilation and final executable linking. Compilers and fixed support are prebuilt equally. Bootstrap and strip are separate. Every emitted Mech object, complete metadata and canonical bytecode is hash checked.

| Actor | Encoder text | Source through final executable link | Stripped executable |
| --- | ---: | ---: | ---: |
| Qualified histogram | 63,612 B | 2.218823 s | 276,072 B |
| Original-profile packed probe | 63,660 B | 2.220006 s | 276,072 B |
| Revised-profile carried probe | 63,660 B | 2.221298 s | 276,072 B |
| Roc | 57,696 B | 2.985871 s | 191,912 B |
| libdeflate C | 37,080 B | 0.482094 s | 155,752 B |

Repaired carried-probe paired build ratios: **0.743629× Roc / 4.605955× C**, and **1.000756×** qualified control. No compile-time gain is established. Its encoder text is **1.103× Roc / 1.717× C**.

Phase medians for that recipe: parse **702.723 ms**, frontend **217.392 ms**, artifact **6.860 ms**, encode **2.539 ms**, native **1226.954 ms**, final executable link **35.852 ms**. Independently computed medians do not sum exactly to the overall median. The compiler release rebuild itself took **228.247 s**; it is excluded from codec-build timing and is not a clean-bootstrap comparison.

## Validation and replay

- Eighteen optimized ordered-reduction tests pass in three configurations: packing disabled, enabled, and enabled with lane carry plus latch testing. Each configuration includes **28,672 packed-key native calls** and **16,384 existing winner/item calls**. The key fixture covers wide unsigned costs, best/stop bounds, ties, high indices, zero groups, unaligned guarded storage, carried lanes and the optimized fallback. Composition tests cover source remainders, unsigned wrapping, global ties, denial, publication and retry.
- All seven linked workers pass 162 output/inflation/alias observations and six checked failures. Each passes twelve injected failures followed by twelve exact recoveries. Peak stack is **4,736 B**, below the 5,280 B cap. Normal/checked object, full metadata and bytecode match exactly.
- All seven portable recipes reproduce exactly in normal and checked emission with relocated profiles. The full GPU suite is not claimed. The repository unsafe audit remains blocked by the pre-existing absent `benchmarks/mech-deflate/comparison/rust/src/pinned_allocator.rs`; the new fixture has an exact allowlist entry in the publication commit.

```sh
python3 verify.py
python3 reproduce.py /path/to/compile_profile /tmp/packed-replay --role packed-extended-carry-probe-profiled
```

Build the minimal `compile_profile` from the pinned compiler source using the command/environment and complete source overlay in `checks-and-builds.tar.gz`, under `packed-vector-key-compiler/plan.json` and `source-inputs/`. Reproduction also works from the PR head, whose follow-up allowlist metadata does not change emitted code. `portable-replays.json` records all fourteen exact source emissions. `archive-files.json` hashes every member of both archives; the verifier streams hardlink targets, checks source pins with Git, rederives timings and instruction totals, and checks tests, errors, stack, recoveries and complete builds.

The original qualified histogram remains available with its complete forward96 / pooled reverse288 confidence evidence. No prior result is discarded.

Next: test two independent packed accumulators over eight candidates, retaining the ordinary vector fallback and exact tail handling. This targets the packed loop's serial key/minimum dependency and branch count; it is not implemented or claimed as a gain in this study. Keep the original-profile probe and the repaired broader-profile version as separate controls, and retain every existing individual peer win before promotion.
