# Review and continuation handoff

**Active objective: earn the 63rd libdeflate runtime win while preserving every existing peer win and all 81 Roc wins.** Qualified selection remains **63,532 B, 81/81 Roc and 62/81 libdeflate**. No new runtime win is claimed.

Measured compiler implementation **`7259e858f3fc4a979c89378d3c73f45af4e6151c`** shares original scalar-read bounds with later immutable byte reuse. It records surviving load identity and current sealed array geometry, without serializing proof authority. It includes previous-DP-cost forwarding and byte reuse; the disabled and older wide-byte controls reproduce their previous object, complete metadata and bytecode exactly. The codec and canonical bytecode remain unchanged; no LLVM dependency is added.

The latest nineteen-case, two-order panel puts the narrow candidate at **1.035632× / 1.045428× libdeflate** on natural 4 KiB, leaving about **3.4–4.3%** less time to parity. It keeps natural 8 KiB, periodic 1 KiB and 320-overlap below C in this panel, repairing earlier narrow discovery losses. The hash combination loses periodic 1 KiB in one order and remains unselected. No additional whole-encoder gain over prior DP work is established. [Source, all observations, prototype failure/repair and portable reproduction](evidence/compared-byte-bounds-20261119/README.md).

The proof removes two repeated bounds decisions: **14,672 fewer matcher instructions on natural 4 KiB** and **3,802,752 on random 1 MiB**, with unchanged wide stack traffic in untimed counters. Profiled encoder sizes are wide **63,548 B**, narrow **63,564 B**, and complements/hash **63,532 B**. These are experimental; the qualified selection is unchanged.

Latest source-through-final-link build medians: retained **2.216112 s**, wide **2.219958 s**, narrow **2.219512 s**, Roc **2.990595 s**, C **0.487171 s**. Ten balanced serial fresh-process orders use prebuilt compilers/support equally. Strip is separate; Mech remains **276,072 B** stripped. Retained paired build ratios are **0.741074× Roc / 4.602640× C**. No established build-time change is claimed.

The new package verifies **12 emissions, eight runtime orders, ten complete-build orders, and 20,736 native comparisons plus three refusal fixtures**. Output/inflation/alias, failure and stack checks pass for all timed candidates. No new full score or recovery campaign is claimed. [Previous byte evidence](evidence/compared-byte-reuse-20261119/README.md) · [Previous DP evidence](evidence/previous-dp-cost-20261119/README.md).

Review [mech-lang/mech PR #995](https://github.com/mech-lang/mech/pull/995), branch `codex/pr993-m3-optimization`. It targets #993's `codex/native-compact-source-handoff`, stacked on #973. Compiler source, original Mech program, build recipes, runtime source and evidence are in this branch. The first immutable-read implementation is `5ccf097dc02ceb3abb17bccb85cd11a3ae144503`; current measured compiler source is **`7259e858f3fc4a979c89378d3c73f45af4e6151c`**. The earlier source-unit minimal compiler `e75d74f7…` and experimental compiler `2a553c89…` reproduce all selected native artifacts in normal and checked allocation modes. Fixed-support linking reproduces the exact qualified worker. The source-unit experiment is opt-in and unselected.

```sh
git fetch origin codex/pr993-m3-optimization
git switch -c codex/review-native-compact origin/codex/pr993-m3-optimization
```

The selected M1 recipe is **`m1-immutable-current-reads`**: **63,532 B**, saving **160 B**, with **81/81 Roc and 62/81 libdeflate wins in both complete orders**. Every paired control win and all 81/62 precision-supported wins are retained. Original Mech source and canonical bytecode are unchanged; no LLVM dependency is added. The latest complete-build cohort uses compiler source `7259e858f3fc4a979c89378d3c73f45af4e6151c`.

Previous source-unit cohort complete build: **2.245045 s**, versus Roc **2.993610 s** and C **0.491877 s**; paired ratios **0.739423× Roc / 4.540212× C**. Eighteen balanced fresh-process orders per actor include final linking with fixed support and compilers prebuilt for every actor. The current minimal compiler is a paired 1.058 ms slower than the earlier compiler producing the identical encoder, 90% interval −8.222 to +19.025 ms: no established change. The repaired unit adds a paired 66.899 ms against its own compiler with the split disabled. No compilation-time improvement is claimed. [All nine actors, phases, raw orders and portable replay](evidence/early-source-unit-repair/README.md).

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

The latest actual-worker captures put 72.3% of 1 MiB encoder samples in matching and 14.4% in scoring; natural 4 KiB is 52.3% scoring / 24.5% matching. Sample fractions are not elapsed phase timers. Large random has about 7.344 ms excess per call, while natural 4 KiB has about 25.1 microseconds. Immediate work targets natural 4 KiB scoring; retain the shared large-random collector as the larger absolute runtime opportunity. The closest next-win check is: 1.063757× / 1.068946× C, needing about 6.45% less time. No 63rd C win is claimed. [Every current case ratio](evidence/encoder-size-gap/immutable-current-read-study/placement/runtime-case-ratios.json).

On 1 MiB, the new collector saves 1.15 million stack loads and 2.10 million stack stores but adds ALU work: 239.33 million instructions versus 233.15 million. Rematerialization repeats the physical byte load. Carrying a valid current-input extent through the private call may remove repeated checks/address work; that requires a real generic admission proof and fallback, not a profile-based safety assumption. The earlier cursor/countdown/ending-pointer repair remains a separate scorer lead. [Source and adverse pointer variants](evidence/natural4k-runtime-target/README.md).

## Main architecture work

The encoder is still **1.713376× C**; the latest selected paired complete build is **4.578593× C**. Its 46,788 B controller exceeds C's entire 37,080 B encoder. The latest selected build spends 705.677 ms parsing, 217.306 ms in semantics and 1,216.669 ms in the native pipeline.

The early-unit repair is now in source. Source-relative hints recover extraction across physical unit boundaries; caller-owned ownership homes, shared numeric packets and borrowed private output scratch avoid repeated transport. Root-only register preference brings actual active stack to **5,280 B**. The repaired encoder is **65,948 B**, down 3,608 B from the same-compiler unrepaired experiment but still 2,416 B above the selected encoder. Fourteen source-bound tests and nine scalar success/error probes pass; all 162 output/inflation/alias checks, six failures and 12 recoveries pass. Both allocation modes and the portable published commands reproduce all artifacts.

The experiment remains unselected: it loses `320-overlap` to C at **1.007444× / 1.009468×** in the focused orders and costs a paired **66.899 ms** to build. No full runtime score is claimed. Keep the gains available for further repair. Four current compiler captures identify repeated full continuation-liveness computation as added planning work. The corrected identical-body inventory finds one 296 B candidate and does not prove safe folding. [Source, every alternative, checks and adverse measurements](evidence/early-source-unit-repair/README.md).

**Runtime priority:** natural 4 KiB is about 52.3% scoring in the current worker and needs about 6.45% less complete-call time in the worse order. Revisit the earlier scorer pointer/cursor work, repair remainder setup, and combine it with broader loop-throughput changes. Keep every existing peer win, both 81/62 point and precision floors, the stack ceiling, original source/bytecode, and matched complete builds. A small slowdown with every peer win intact is allowed. A single near loss calls for bounded repair, not discarding the entire gain.

The qualified late codebook split remains available as `m1-checked-codebook-phase`: 64,444 B, the same peer wins and 160 B lower peak stack in its original cohort, with a 640 B cost versus that cohort's selected reference. It is distinct from the earlier literal-frame 64,444 B recipe. The previous `m1-matcher-guard-facts` and `m1-register-spill-pairs` recipes remain reproducible.

Sealed base phase fusion is already integrated parser maintenance (539 prior full-feature tests, plus 536 fresh default-feature tests after the module relocation), and sparse guard scheduling's earlier 34 ms saving is already adopted. Larger sealed parser-family specialization and conservative prepared-helper IR reuse remain unfinished. Do not add separate cohorts' savings together or report adopted work as a new gain.

Preserve direct Bool/U64/I64/F64 native emission, checked errors/origins, ownership and finite admission. Ordinary resource-accounting limitations remain as documented in QUALIFICATION.md. Formatting, module layout and warning policy pass. The unsafe audit is blocked by the existing missing archived allocator path; earlier M3 ABI oracle failures also remain historical context. No CI-green claim is made. Current M1 code/build evidence must not be replaced with historical M3/LLVM figures.
