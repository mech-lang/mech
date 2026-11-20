# Input spans and first-word probes: source integration and measured combinations

**The qualified selection remains 63,612 B, 81/81 Roc wins and 62/81 libdeflate wins. No 63rd C win or new qualified size is claimed.** The new span and probe variants have nineteen-case, two-order discovery evidence. Full 81-case confidence qualification remains outstanding.

The source basis is **`b55805f7f36a6163b4b0f7200d0519461b6448e2`**, on PR #995. It contains the compiler implementation and tests. The supplied Mech codec and canonical bytecode are unchanged; there is no LLVM dependency or codec rewrite. With the new option disabled, the current minimal compiler reproduces the qualified histogram object, full metadata and bytecode exactly.

## What changed

A private caller checks `begin <= end <= current input length` before phase effects. The compiler proves a unit-stride cursor across nested copy phis, including the collector's two interconnected loop headers. Inside the active loop it can replace the two current-byte address predicates. Refusal resumes the original checked source continuation. Unknown strides, signed comparisons, changed lengths/limits, differing initial words and unsupported graph shapes retain their original graph and an empty entry contract. Graph size, recursion depth and total proof work are bounded.

The first implementation at `6ac617bae` ran before readonly default-load conversion. That prevented two existing load optimizations from recognizing their inputs. It removed predicates but introduced extra constant branches and stack traffic. The implementation at `b55805f7f` runs after that conversion, preserving its readonly load flags. The raw word text falls from 63,484 to **63,292 B**. Natural-4-KiB raw matcher counters fall from 1,473,812 to 1,451,264 instructions, with stack loads falling from 131,404 to 109,843 and stack stores from 35,573 to 24,312. These counters describe the matcher before fresh layout, rather than whole-encoder latency.

Metadata's `checks_removed` field counts folded SSA comparisons. After readonly conversion it reports three comparisons, including a predecessor comparison retained for dead-value retirement. The active input-byte checks removed are two. The archived receipt-summary correction records an earlier copied summary that incorrectly said two SSA comparisons; the underlying object, metadata and runtime observations were unchanged.

The span tests exercise **12,096 native calls**, including readonly-load conversion, default reads, partial state, budget failures, unsigned extremes and original error results. Eight proof-miss shapes leave the graph unchanged. Shared histogram tests add **11,760 native calls**, and ten wide-equality tests cover the first-word probe and vector/scalar tails: **15 relevant repository tests pass**. A preliminary composition test failed on its expected IR-comparison count; its log is retained, and the final complete differential tests pass.

## Discovery results and retained alternatives

All five current variants retain the six previous C-winning cases and all nineteen Roc-winning cases present in this nineteen-case panel, in both orders. These subset counts must not be described as a new 81-case score.

The first-word probe compares eight bytes, forwards a failing XOR to mismatch reconstruction and advances successful reads by eight before the vector remainder. With the span optimization and the other features held fixed, it takes **1.2727% / 1.3035% less time** on natural 4 KiB in forward/reverse order. Fresh profiles follow the same acquisition policy for each graph. Without the span optimization, the probe is inconsistent: 0.9284% slower forward and 0.0513% faster reverse. Both combinations remain archived.

The combined candidate's natural-4-KiB ratios are **1.024347x / 1.039711x libdeflate**. It still needs 2.38% / 3.82% less time to reach parity in those orders. The next experiment should remove the remaining repeated saturated input-length calculations, then measure the combined effect. The runtime goal remains the priority.

The preceding bitwise follow-up is also included. Its vector/complement candidate retains every previous named win in two complete 81-case, 24-block point screens: 81 Roc / 62 C. Its 63,356 B word alternative loses `320-overlap` in reverse order, leaving 61 C wins in that order. That smaller candidate remains available for repair. Neither bitwise result is confidence-promoted.

## Complete builds on the Apple M1

| Actor | Encoder text | Source through final executable link | Peak encoder stack | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Qualified histogram, same current compiler | 63,612 B | 2.213534 s | 4,864 B | 276,072 B |
| Span word | **63,292 B** | 2.217011 s | 4,832 B | 276,072 B |
| Span vector/complement | 63,628 B | 2.219826 s | 4,832 B | 276,072 B |
| Span vector/complement + first-word probe | 63,740 B | 2.218439 s | 4,832 B | 276,072 B |
| Roc | 57,696 B | 2.985958 s | — | 191,912 B |
| libdeflate | 37,080 B | 0.483760 s | — | 155,752 B |

Twelve balanced serial fresh-process trials per actor include final executable linking. Compilers and fixed support are prebuilt for every actor; bootstrap and strip are separate. Every generated Mech object, complete metadata and bytecode is hash checked. Word's paired build ratios are **0.742764x Roc / 4.595761x C / 1.001558x qualified control**. There is no established compilation-time improvement. Its encoder text is 1.097x Roc / 1.707x C. The earlier 10-trial span-placement cohort is retained separately; separate cohorts are not causal speed comparisons.

Word phase medians are parse 703.428 ms, semantics 217.955 ms, artifact preparation 6.850 ms, encoding 2.537 ms and native pipeline 1,224.487 ms. Catalog work overlaps parsing; separate medians do not add. Through-strip time is 2.228038 s for word.

All current span/probe variants pass 162 output/inflation/alias observations, six checked failures and the 5,280 B stack cap. Word, vector/complement and the combined probe each pass twelve injected failures followed by twelve successful recoveries. Their normal and allocation-checked source builds reproduce all three artifacts exactly. These diagnostic-entry checks do not establish ordinary managed-entry admission and recovery.

## Reproduce and review

```sh
python3 verify.py
python3 reproduce.py /path/to/compile_profile /tmp/span-word
python3 reproduce.py /path/to/compile_profile /tmp/span-probe --role probe-span-vector-complements-profiled
```

From source commit `b55805f7f`, build the compiler with:

```sh
env CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 \
  CARGO_PROFILE_RELEASE_OPT_LEVEL=3 CARGO_PROFILE_RELEASE_DEBUG=false \
  CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
  cargo +nightly-2026-03-03 build --locked --release \
  --config profile.release.package.libmimalloc-sys.opt-level=1 \
  --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --target-dir /tmp/mech-span-compiler --bin compile_profile
```

The compiler is `/tmp/mech-span-compiler/release/compile_profile`. The checked source replay was also executed using the relocated public profiles for word and the combined probe; both reproduce all three artifacts exactly.

Use the minimal compiler configuration in the archived build plan; the optional source-unit compiler feature is not required by these final flags. Profiles and generation flags are included. Both raw source coordinates and the regenerated numeric checked-region boundaries are preserved. Stale-coordinate failures remain available. Source replay verifies artifacts; fresh runtime and complete-build qualification on other hardware are separate.

`checks-and-builds.tar.gz` binds source snapshots, native tests, normal/checked artifacts, recoveries, counters, profiles and both build cohorts. `raw-runtime.tar.gz` retains all 26 acquisitions, including adverse observations. The verifier independently recomputes point ratios from balanced raw blocks, checks source and worker identities, and replays the build medians and generated-artifact pins.

The entire GPU suite is not claimed. The repository unsafe audit remains blocked by the pre-existing absent allowlisted `benchmarks/mech-deflate/comparison/rust/src/pinned_allocator.rs`. Preserve the qualified 81/62 selection and every original adverse block while continuing toward the next C win.
