# Review and continuation handoff

Review [mech-lang/mech PR #995](https://github.com/mech-lang/mech/pull/995), branch `codex/pr993-m3-optimization`. It targets #993's `codex/native-compact-source-handoff`, stacked on #973. Compiler source, original Mech program, build recipes, runtime source and evidence are in this branch. The first immutable-read implementation is `5ccf097dc02ceb3abb17bccb85cd11a3ae144503`; final standard module integration is **`2cd17fca3c9d58a5804a18c904fe1daf65598b78`**. Final compiler `94aad561…` reproduces all native artifacts and the exact fixed-support qualified worker. Subsequent evidence commits preserve these Rust inputs.

```sh
git fetch origin codex/pr993-m3-optimization
git switch -c codex/review-native-compact origin/codex/pr993-m3-optimization
```

The selected M1 recipe is **`m1-immutable-current-reads`**: **63,532 B**, saving **160 B**, with **81/81 Roc and 62/81 libdeflate wins in both complete orders**. Every paired control win and all 81/62 precision-supported wins are retained. Original Mech source and canonical bytecode are unchanged; no LLVM dependency is added. Final compiler source is `2cd17fca3c9d58a5804a18c904fe1daf65598b78`.

Latest complete build: **2.208667 s**, versus Roc **2.980663 s** and C **0.480168 s**; paired ratios **0.741248× Roc / 4.602361× C**. Eight balanced fresh-process trials per actor include final linking with fixed support prebuilt for every actor. The final compiler/configuration costs a paired **3.360 ms** (90% interval 0.941–5.050 ms) versus the earlier compiler generating the same encoder. In the separate sixteen-trial pass-isolation cohort, the pass itself is tied versus its disabled same-compiler control. No compilation-time gain is claimed.

The complete stripped executable remains **276,072 B**. Runtime reports **13 established slowdowns above 0.5% and 15 uncertain comparisons**; every peer win is retained. Large random 1 MiB costs 0.54% / 0.77%. [Full source/evidence and both build cohorts](evidence/encoder-size-gap/immutable-current-read-study/README.md).

## Build and verify

```sh
python3 rebuild.py /tmp/mech-immutable --variant m1-immutable-current-reads \
  --mimalloc-object /path/to/mimalloc.o
python3 evidence/encoder-size-gap/immutable-current-read-study/verify.py
```

With an existing compiler built from this source, use the evidence package's `reproduce.py` for portable normal/allocation-checked object, metadata and bytecode replay. The selected fixed-support worker is `834c7a7e423a69c46483a3dae74d9450949fdf142b5ad2dede894e0da3ca8693`. It passes 162 output/inflation/alias observations, six failures, 12 recoveries and 4,848 B stack under the unchanged 5,280 B ceiling. Four source-bound executable load tests and four unchanged optimized private-contract tests cover original/decoded authority, null/default/wrapping values, current loops and divergent paths. Normal and checked artifacts match exactly. The disabled pass reproduces the previous selected 63,692 B worker.

## History and remaining runtime losses

The [19-mechanism audit and all-loss profiles](evidence/history-and-profiles-20261119/README.md) led to retained spill pairing, then unsigned guards/fresh layout, and now literal-reuse repair. Do not reject gains just for small slowdowns with peer wins intact. Preserve every adverse attempt and make bounded repairs before abandoning a mechanism. The 208 B smaller broad variant had a single near C loss; the narrower variant keeps 160 B and recovers it. The larger-random cost falls from roughly 2.7% focused to 0.54–0.77% in full 1 MiB orders.

The latest actual-worker captures put 72.3% of 1 MiB encoder samples in matching and 14.4% in scoring; natural 4 KiB is 52.3% scoring / 24.5% matching. Sample fractions are not elapsed phase timers. Large random has about 7.344 ms excess per call, while natural 4 KiB has about 25.1 microseconds. Optimize the shared random collector first, keeping natural 4 KiB as the closest next-win check: 1.063757× / 1.068946× C, needing about 6.45% less time. No 63rd C win is claimed. [Every current case ratio](evidence/encoder-size-gap/immutable-current-read-study/placement/runtime-case-ratios.json).

On 1 MiB, the new collector saves 1.15 million stack loads and 2.10 million stack stores but adds ALU work: 239.33 million instructions versus 233.15 million. Rematerialization repeats the physical byte load. Carrying a valid current-input extent through the private call may remove repeated checks/address work; that requires a real generic admission proof and fallback, not a profile-based safety assumption. The earlier cursor/countdown/ending-pointer repair remains a separate scorer lead. [Source and adverse pointer variants](evidence/natural4k-runtime-target/README.md).

## Main architecture work

The encoder is still 1.713376× C; complete build is 4.505969× C. Its 46,788 B controller alone exceeds C's entire 37,080 B encoder. Latest parsing takes 702 ms, semantics 215 ms and the native pipeline 1,221 ms. The post-encode compiler capture again finds dominance, SSA construction, validation and guard inference among the leading active functions. Reduce work constructed and analyzed rather than extend the small late-pass campaign indefinitely.

The existing early codebook unit is the pilot: 885 source operations omitted before root SSA, 42 input/27 output words, total allocated SSA down 18.6%, constructed instructions down 27.7%. Its recorded implementation is unselected: 69,828 B whole text, 5,808 B stack above the 5,280 B cap, and complete build 2.277083 s versus 2.209673 s in its cohort. Correctness/errors/recovery pass; it has no inherited 81/62 runtime qualification. Keep its gain and repair boundary/retirement state, active scratch and valid late sharing through source/unit maps. Old native block IDs do not remain valid after partitioning. Price all helpers, call setup, spills, stubs and alignment. [Early source emission and all adverse evidence](evidence/early-codebook-emission/README.md).

The qualified late codebook split remains available as `m1-checked-codebook-phase`: 64,444 B, the same peer wins and 160 B lower peak stack in its original cohort, with a 640 B cost versus that cohort's selected reference. It is distinct from the earlier literal-frame 64,444 B recipe. The previous `m1-matcher-guard-facts` and `m1-register-spill-pairs` recipes remain reproducible.

Sealed base phase fusion is already integrated parser maintenance (539 prior full-feature tests, plus 536 fresh default-feature tests after the module relocation), and sparse guard scheduling's earlier 34 ms saving is already adopted. Larger sealed parser-family specialization and conservative prepared-helper IR reuse remain unfinished. Do not add separate cohorts' savings together or report adopted work as a new gain.

Preserve direct Bool/U64/I64/F64 native emission, checked errors/origins, ownership and finite admission. Ordinary resource-accounting limitations remain as documented in QUALIFICATION.md. Formatting, module layout and warning policy pass. The unsafe audit is blocked by the existing missing archived allocator path; earlier M3 ABI oracle failures also remain historical context. No CI-green claim is made. Current M1 code/build evidence must not be replaced with historical M3/LLVM figures.
