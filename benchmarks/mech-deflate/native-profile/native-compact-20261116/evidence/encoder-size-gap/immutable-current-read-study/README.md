# Immutable current-input reads: qualified repair

**Select `m1-immutable-current-reads`: 63,532 B, saving 160 B, with 81/81 Roc and 62/81 libdeflate wins in both complete orders.** Both orders support all those wins at the configured precision and retain every paired control win. Original Mech source and canonical bytecode are unchanged. The implementation starts at `5ccf097dc02ceb3abb17bccb85cd11a3ae144503`, with final source integration at `2cd17fca3c9d58a5804a18c904fe1daf65598b78`; another agent can build it from this branch. No LLVM dependency is added.

This is a small code-size win with runtime costs, under the policy allowing slowdowns that preserve peer wins. There are **13 established case slowdowns above 0.5% and 15 uncertain comparisons**. Large random 1 MiB costs 0.54% / 0.77% in the full orders; 256 KiB costs 0.75% / 0.81%. The complete stripped executable stays **276,072 B**. The 1× libdeflate code/build goal and next C win remain unfinished.

## Final source integration and latest complete builds

The final compiler source is **`2cd17fca3c9d58a5804a18c904fe1daf65598b78`**, including standard native/parser module layouts and a separate allowlisted JIT test boundary. Its normal and allocation-checked builds reproduce the same **63,532 B** object, full metadata and canonical bytecode. Relinking with the original fixed support reproduces qualified worker **`834c7a7e…`** exactly, so its **81 Roc / 62 C point and precision wins in both complete orders**, output/error/recovery/stack checks and profiles remain applicable.

| Latest Apple M1 build | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Earlier compiler, same qualified encoder | 63,532 B | 2.204676 s | 2.215713 s | 276,072 B |
| Final source/compiler configuration | **63,532 B** | **2.208667 s** | **2.219695 s** | 276,072 B |
| Roc | 57,696 B | 2.980663 s | 2.991847 s | 191,912 B |
| libdeflate | 37,080 B | 0.480168 s | 0.490875 s | 155,752 B |

Eight balanced fresh-process builds per actor use prebuilt compilers and fixed support for every actor. Paired ratios are **0.741248× Roc / 4.602361× C**. The final compiler/configuration costs a paired **3.360 ms** versus the earlier compiler generating the same encoder, 90% interval **0.941–5.050 ms**, 1/8 faster. This comparison includes the mechanical source/build changes; it does not isolate the immutable-read pass. The earlier sixteen-trial same-compiler study below found no established effect from that pass. Do not add or compare separate cohorts as causal gains.

Latest phase medians are **702.055 ms parsing**, **214.546 ms semantics**, **6.834 ms artifact**, **2.532 ms encoding**, **1,221.209 ms native pipeline**, and **35.044 ms final link**. Catalog work overlaps parsing; separate medians do not add. Source peak RSS is **392.9 MB**.

The published named rebuild command also reproduces the encoder and passes **162 outputs, six failures and 12 recoveries**. It rebuilds host support and has a different worker hash (`01ace001…`), so that worker has no inherited runtime-win claim. The matched timings use fixed prebuilt support; runtime qualification belongs to the exact `834c7a7e…` worker. Its fixed-support reproduction is explicitly verified.

Fresh **536 default-feature syntax tests** and four load tests pass after the module moves; the prior 539 full-feature syntax tests are archived separately. Workspace formatting, module layout and warning policy pass. The unsafe audit remains blocked by the existing missing `benchmarks/mech-deflate/comparison/rust/src/pinned_allocator.rs`; no CI-green claim is made.

## History, controls and repairs

The [19-mechanism history review](../../history-and-profiles-20261119/README.md) already led to retained spill pairing and guard/layout combinations. This study revisits historical literal-read-once using generic compiler proofs on the unchanged codec. The collector reads one immutable input byte for observation sampling and the cache header; on 1 MiB those sites execute 1,048,545 and 1,044,385 times. Four copy-phi nodes carry the same current index, with one external seed.

The pass reuses the strong defaulted-read recognizer: exact pointer/length packet fields, unsigned index bound, width, wrapping address and zero default. Only the exact private fixed-view loan supplies immutable authority. The root retains read-only owners; written backings pass the existing unique-owner gate, and borrowed-copy proofs exclude phase boundaries. A bounded all-incoming copy-phi proof shares equivalent current reads only where every operand is available. Updates, unavailable operands, unknown seeds, seedless cycles and budget exhaustion refuse sharing. Uncertified reads retain their original branches.

| Source-emitted alternative | Whole encoder text | Matcher body | Matcher frame |
| --- | ---: | ---: | ---: |
| Prior selected | 63,692 B | 6,092 B | 384 B |
| Existing safe-address fold on all eight reads | 63,708 B | 6,100 B | 400 B |
| Immutable flags before explicit reuse | 63,724 B | 6,120 B | 400 B |
| All eight reads, reuse with rematerialization | 63,484 B | 5,880 B | 384 B |
| All eight reads, retain load placement | 63,692 B | 6,088 B | 400 B |
| Only two certified reads, retain placement | 63,692 B | 6,088 B | 384 B |
| **Only two certified reads, rematerialization** | **63,532 B** | **5,936 B** | **384 B** |

The 208 B saving was repaired rather than discarded: its first focused order lost `320-overlap` to C by 0.1763% and large-random time rose about 2.7%. Pinning the shared load removed a million dynamic byte loads but lengthened its live range and increased spills. Restricting the pass to the two certified reads removes unrelated safe-address/select work on mutable buffers. The chosen variant keeps 160 B of the saving and restores `320-overlap` in both focused and full orders: full Mech/C ratios **0.996719× / 0.993806×**. Every initial/adverse attempt remains archived.

Cranelift treats `Load` as pure with `readonly`, `notrap` and `can_move`. Rematerialization can repeat the physical read at distant uses while reducing spills. Pinned placement is the default control; `MECH_COMPACT_REMATERIALIZE_IMMUTABLE_READS=1` enables the chosen policy under the separately scoped `MECH_COMPACT_IMMUTABLE_DEFAULTED_READS`. Both choices have the same source safety proof. No source codec rewrite or raw-machine-byte implementation is shipped.

Fresh matcher layouts use each actual graph and untimed counters; the other two retained phase profiles have exact ordered-graph equality proofs. Normal and allocation-checked builds reproduce identical objects, full metadata and bytecode. The disabled pass reproduces the previous selected artifacts.

## Original pass-isolation cohort: every alternative

| Apple M1 actor | Encoder text | Source through final link | Through strip |
| --- | ---: | ---: | ---: |
| Previous compiler, selected encoder | 63,692 B | 2.223243 s | 2.234224 s |
| Current compiler, pass disabled | 63,692 B | 2.213335 s | 2.224324 s |
| All eight, rematerialization | 63,484 B | 2.222866 s | 2.234057 s |
| All eight, pinned placement | 63,692 B | 2.226704 s | 2.237704 s |
| Immutable only, pinned placement | 63,692 B | 2.220956 s | 2.232086 s |
| **Immutable only, rematerialization** | **63,532 B** | **2.211926 s** | **2.223045 s** |
| Roc | 57,696 B | 2.988797 s | 2.999631 s |
| libdeflate | 37,080 B | 0.491145 s | 0.502016 s |

Sixteen balanced fresh-process trials per actor include final linking, with deployment support prebuilt for every actor. Compiler bootstrap is excluded for all; strip is separate. Raw warmups, outliers and every round remain included. All Mech deployments are 276,072 B; Roc is 191,912 B and C 155,752 B.

**The pass has no established compile-time effect:** versus the same compiler with it disabled, paired median **-0.139 ms**, 90% interval **-3.628 to +3.598 ms**, 8/16 faster. Separate medians are not paired differences. The current compiler beats the previous compiler in this cohort, but that observation does not isolate this pass; the same-compiler control does. The other three alternatives cost paired medians of 8.276–10.345 ms versus the disabled control.

Paired complete-build ratios are **0.739157× Roc / 4.505969× C**. Encoder ratios are **1.101151× Roc / 1.713376× C**, leaving **26,452 B** above C. That cohort’s phase medians are **704.371 ms parsing**, **217.767 ms semantics**, **6.834 ms artifact creation**, **2.515 ms encoding**, and **1,218.926 ms native pipeline**. Catalog work overlaps parsing; separate medians do not add. Source CPU is 2.237413 s and peak RSS 392.6 MB.

## Where the remaining losses accrue

Fresh captures bind the actual selected worker `834c7a7e…` and pinned C. On 1 MiB random input, **72.3%** of Mech encoder leaf samples are matching and **14.4%** scoring. Natural 4 KiB is **52.3% scoring / 24.5% matching**. Each capture verifies two complete output/inflation/alias calls. Sample fractions are diagnostics, not elapsed phase timers or savings forecasts; C matching is inlined into its near-optimal body. Some constructor, transfer and cleanup work is outside the selected function trees.

Keep the shared large-random collector as the primary runtime target: its larger separately measured median excess is **7.344 ms/call**, versus **25.1 µs** for natural 4 KiB. The latter remains closest to a new C win, at **1.063757× / 1.068946× C**, requiring about **6.45%** less Mech time in the worse order. [All case ratios](placement/runtime-case-ratios.json) and [all 19 remaining named C losses](placement/remaining-losses.json) preserve both orders.

| Original collector instructions on 1 MiB | Prior selected | New selected |
| --- | ---: | ---: |
| Instructions | 233,154,478 | 239,331,515 |
| Branches | 41,017,489 | 38,926,518 |
| Byte loads | 4,002,154 | 4,002,154 |
| Stack loads | 22,267,325 | 21,121,137 |
| Stack stores | 8,653,030 | 6,553,980 |

The selected variant saves spills and static code while adding ALU work; it does not eliminate dynamic byte loads. Diagnostic clones count original instructions and verify outputs, but are never timed. Their Mach-O section-coordinate adapter preserves every code/data byte and relocation target, and its linked worker is byte-identical to the original. Actual original objects/workers are used for latency.

The post-encode compiler capture includes native compilation through process exit. Dominance, SSA construction, validation and guard inference are among its leading active functions; it is not an exclusive elapsed-phase profile. The old `MECH_PROFILE_COMPACT_DETAIL` hook is absent from current production source, so its no-scope attempt is retained without invented attribution. **Reducing constructed/analyzed controller state remains the larger code/build task.** The checked controller still occupies 46,788 B, with a 3,296 B frame. The earlier early-unit construction gain remains available for repair, including its adverse size/stack/build evidence.

## Validation and replay

The actual selected worker passes **162 output/inflation/alias observations, six failures, 12 recoveries and 4,848 B peak active stack**, under the unchanged 5,280 B ceiling. Four source-bound executable load tests cover null/default/wrapping values, I8/I32 extension, current-loop sharing and divergent/changing indices. Loop sharing, divergence and changing indices run in both placement modes. Four optimized private-phase contract tests cover original/decoded codec proofs, unique ownership, growing-storage refusal and unpermitted cloned proofs. Unchanged private-contract source binds those reused tests. The initial stale fixture include failure and corrected actual-codec test are archived. Existing ordinary resource-accounting limitations are unchanged; no new CI-green claim is made.

```sh
python3 verify.py
python3 reproduce.py /absolute/path/to/compile_profile /tmp/mech-immutable-replay
```

Or build the compiler and recipe from this checkout:

```sh
python3 ../../../rebuild.py /tmp/mech-immutable-build --variant m1-immutable-current-reads \
  --mimalloc-object /path/to/mimalloc.o
```

`as-run/` records original machine paths and exact acquisition scripts. Compressed source snapshots keep historical Rust out of architecture scans. Both complete orders retain all 96 paired blocks for every case. The compiler source, generation route, profiles, raw measurements, adverse controls and verifiers are in PR #995; the worker is generated from original Mech source.
