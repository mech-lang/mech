Latest compiler source is **`330416904`**, adding the clamped-window proof and consuming late constants before native lowering. Word is **63,180 B**, probe **63,484 B** and packet-qualified **63,068 B**, all with 4,736 B peak stack. The probe improves natural 4 KiB by 1.04–1.53% and random 1 MiB by 1.61–1.62% against its matching base, while remaining **1.011199× / 1.009384× C** on natural 4 KiB. Every previously required peer win observed in six complete twenty-one-case orders remains a point win. Full qualification and win #63 are outstanding. Complete build is **2.213320 s**, paired **0.742111× Roc / 4.602746× C**. Twenty-three relevant tests, 47,552 native calls, fresh output/error/stack/recovery checks and checked replay pass. The earlier constant-branch version and all adverse counts remain available. The qualified result stays 63,612 B / 81 Roc / 62 C. Next, use the selected `window >= 5` edge to admit the remaining four-byte input read. [Source, all raw orders, machine counts, complete-build profiles and reproduction](evidence/encoder-size-gap/clamped-input-window-study/README.md).

Earlier experimental source is `253736215`: a guarded fixed-view floor removes two collector checks. The word candidate reaches 63,388 B / 4,720 B stack and preserves every prior win in both full 24-block point orders, with fresh outputs/failures/recoveries and checked emission. It remains unpromoted pending full confidence qualification. The qualified 63,612 B histogram worker remains selected at 81 Roc / 62 C. [Current experiment, adverse counts, recovered build-mode cost and reproduction](evidence/encoder-size-gap/bounded-index-range-study/README.md).

# PR #995 review and continuation handoff

**Latest qualified throughput recipe: `m1-bounded-byte-histogram`, 63,612 B, preserving every prior 81 Roc / 62 C win.** It improves random 1 MiB by 2.44–2.73% against its same-run 63,388 B reference. Forward96 and reverse288 pooled blocks retain all original data; the fresh reverse192 repeat independently supports all wins. The original reverse96 had 2 uncertain Roc comparisons and remains archived. Build 2.214983 s, paired 0.740659× Roc / 4.602734× C; stripped executable 276,072 B. No 63rd win. [Full evidence and reproducible recipe](evidence/encoder-size-gap/bounded-byte-histogram-study/README.md).

The 63,388 B recipe below remains the compact choice. The first bounded-index reuse prototype passed its native differential test but admitted no new checks in this codec. Inspection shows the 262,144-entry history floor is already exploited; observation counters use a different small array. Carry a separately guarded current extent for that array, or prove a checked read/write continuation, before making a performance claim. Preserve the scorer composition.

## Preceding compact reference and mechanisms

**Goal: reach 1× libdeflate performance. The next milestone is 63/81 C wins while preserving every existing individual C win and all 81 Roc wins.** The qualified reference is now `m1-word-complement-current-reads`: **63,388 B, 81/81 Roc and 62/81 C**, both point estimates and configured confidence checks in both complete 96-block orders. This is 144 B smaller than the previous reference. There is no 63rd C win.

Work is in [mech-lang/mech PR #995](https://github.com/mech-lang/mech/pull/995), branch `codex/pr993-m3-optimization`, targeting #993's `codex/native-compact-source-handoff`, stacked on #973. The branch contains compiler source, the original 2,719-line Mech codec, reproducible build recipes and the raw evidence. No LLVM dependency is added. Canonical bytecode remains `c6529d934f020f07f8e59f13fab40927e066649e202269818ad3f009a942a8c3`.

```sh
git fetch origin codex/pr993-m3-optimization
git switch -c codex/review-native-compact origin/codex/pr993-m3-optimization
```

## Qualified reference and reproduction

The selected timing worker is `fab303151eca902a95986488684102fc1707ec7f8df1324ebfd9c55543cfc368`. All 162 output/inflation/alias checks, six failures, twelve injected failures and twelve recoveries pass. Peak active stack is **4,864 B**, below the 5,280 B cap. Normal and allocation-checked compilers reproduce the complete object, metadata and bytecode. Qualification transfers between compiler builds only when the resulting artifacts and linked worker match exactly.

```sh
python3 rebuild.py /tmp/mech-word-complement --variant m1-word-complement-current-reads
python3 evidence/encoder-size-gap/word-complement-current-read-study/verify.py
python3 evidence/review-followup-20261119/verify.py
```

Use the existing optional allocator argument to link/check a worker. Different support can produce a different worker and requires its own runtime evidence. [Full qualification, original required win sets, recovery and reproduction](evidence/encoder-size-gap/word-complement-current-read-study/README.md).

The original timing compiler is 5bcc; integrated source `546af95af3735591a77455932dc48920a8fc071d` reproduces all artifacts with compiler 47ee. Commit `ce5abdb0d13cae82cf01f4935a43ab667971ce0c` adds the opt-in first-word probe, with disabled mode reproducing the same selected linked worker. Commit `00765f3be` makes an inapplicable saved-prefix strategy leave the graph unchanged and report no reuse. The latter passes repository tests and reproduces the exact qualified artifacts and linked worker in normal/checked word and word-reuse modes, with no misleading reuse metadata.

## Keep both finalists

The recipes already combine previous-DP-cost forwarding, cleanup and proved compared-byte reuse. Word/complement uses scalar word comparisons and complementary predicates; vector/narrow uses mismatch forwarding and narrow DP carry. Do not add their gains to earlier isolated DP gains.

The new direct cohort runs the control, both finalists, Roc and C together: **19 cases, all 120 five-actor permutations per case, both complete case orders**. Vector/narrow is faster on natural 4 KiB and small random inputs. Word/complement is smaller and stronger on fragile overlap/periodic wins. Keep both rather than choosing from separate cohorts' C ratios.

| Natural 4 KiB | Word/complement / C | Vector/narrow / C |
| --- | ---: | ---: |
| Direct cohort, forward | 1.037325× | 1.031229× |
| Direct cohort, reverse | 1.029387× | 1.021762× |

The word/vector differences are supported by the 90% paired intervals. Vector/narrow's reverse `320-overlap` result is **1.000442× C**, with interval **0.998664–1.002287**; repair this fragile case before treating it as a full-score replacement. A single near loss is a repair target, not grounds to discard the gain. Small slowdowns are allowed when every existing peer win remains intact.

The full qualified word result is a different cohort: **1.034702× / 1.043348× C** on natural 4 KiB, improving **2.73–2.74% against its same-run previous reference**. All six established small slowdowns and four uncertain control comparisons are retained. The earlier lower discovery ratios remain archived and do not replace the full result. [Direct comparison, all cases and confidence intervals](evidence/review-followup-20261119/README.md).

The secondary word-reuse/complement's original random-1-MiB reverse regime change is preserved, all 24 blocks included, at **1.015314× Roc**. Repeating its exact worker and original complete reverse order gives **0.960743× Roc**, without the earlier jump. This alternative is distinct from the qualified word-current worker. Neither run is selectively edited.

## Next mechanisms

Give substantial effort to the common path, particularly the bounded literal histogram and collector position/window facts. The histogram still performs five defensive checks per byte and executes **23,068,768 instructions / 7,340,036 branches** on random 1 MiB. Connect the exact input span, byte index range, fixed histogram extent, unique writable ownership, zero initialization and maximum increments. A guarded span/extent specialization can retain the checked U32 increment until initialization/count bounds are actually proved. Proof work must be bounded, and proof misses must leave the original checked path available. Carry position/extent relationships across entry and backedges; revisit the repeated observation read/write guard and producer-defined scorer roles.

The incremental first-word probe forwards a loaded XOR directly on failure, advances eight bytes on success and retains vectors for long comparisons. It admits both codec equality loops. On natural 4 KiB, **2,983 of 5,562 probes fail immediately**, while **2,579 enter vectors**. Its three profiled alternatives pass output/failure/stack gates; latency is still unmeasured here. Compare against both finalists and corresponding vector controls with other flags fixed.

Scorer winner-frequency measurement is complete: natural 4 KiB has **1,449 vector replacements among 3,550 reductions (40.82%)**. Natural 1/8 KiB are 34.77%/41.68%. Measured random/distribution-change cases have no vector reductions. Current code already reconstructs the horizontal argmin at reduction end, rather than every SIMD chunk. Deferred reconstruction must budget work for the roughly 41% of natural reductions that supply a winner. It does not directly address the random losses.

The old overwritten stack load and duplicate checked-subtraction comparison are absent in the current object. Do not reuse their historical instruction budgets. Retain source/error/ownership authority and direct Bool/U64/I64/F64 native paths.

## Build time, size and validation

Current-source matched complete-build medians are **2.240053 s Mech, 3.007427 s Roc and 0.487046 s C**, eight balanced fresh-process orders with compilers/support prebuilt for everyone and final linking included. The paired Mech ratios are **0.741226× Roc / 4.593452× C**. The previous compiler produces the same encoder in 2.229851 s; the paired difference is not established. The earlier ten-trial selection cohort measured 2.214373 s Mech and is archived separately. Bootstrap and strip are separate metrics.

Encoder text is **63,388 B Mech / 57,696 B Roc / 37,080 B C**. Stripped executables are **276,072 B / 191,912 B / 155,752 B**. Keep build time and artifact size tracked after each runtime repair; do not infer speed from size alone.

The relevant repository build now passes all **10 wide-equality tests**, including the new probe and exact no-op proof miss; **1,350 tests were filtered out**. The prior disk-exhausted build is retained. The repository unsafe audit still has the pre-existing missing allowlisted `benchmarks/mech-deflate/comparison/rust/src/pinned_allocator.rs`; no all-CI-green claim is made. Diagnostic-entry qualification does not establish ordinary managed-entry admission/recovery.

Keep the full-catalog repository test target separate from the smaller compiler target. Machine crates emit shared unhashed rlib/cdylib names, so switching feature sets in one target directory can leave a fresh fingerprint pointing at the other build's library. The original test binary and benchmark workers remain pinned while the compiler's affected libraries are rebuilt separately.

Historical alternatives remain available for repair: the [source-unit experiment](evidence/early-source-unit-repair/README.md), [history and loss profiles](evidence/history-and-profiles-20261119/README.md), [DP forwarding](evidence/previous-dp-cost-20261119/README.md), [compared-byte bounds](evidence/compared-byte-bounds-20261119/README.md), and [original mismatch discovery](evidence/vector-mismatch-20261119/README.md). The preceding 63,532 B and 63,692 B qualified artifacts are retained. The goal remains unfinished.
