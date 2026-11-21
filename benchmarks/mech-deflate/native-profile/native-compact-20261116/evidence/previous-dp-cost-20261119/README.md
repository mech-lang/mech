# Previous DP cost forwarding, 19 November 2026

The active target remains **63/81 libdeflate wins while preserving every existing peer win and 81/81 Roc**. The qualified selection remains **63,532 B, 81/81 Roc and 62/81 libdeflate**. No experiment in this study earns the 63rd win. The two qualified 63,532 B and 63,692 B artifacts remain available in the retained study.

Compiler source is pushed at `96b1f1ebc`. It carries the previous appended U64 node across the general scorer path, extending the existing literal-run recurrence. The original 2,719-line Mech codec and canonical bytecode are unchanged. No LLVM dependency is added. The forwarding and cleanup modes are opt-in; disabled mode reproduces the selected object, complete metadata and bytecode exactly.

## Production discovery

Each row uses nineteen workloads and twenty-four balanced blocks in both complete discovery orders: tiny/overlap/periodic guards, natural 1/4/8 KiB, random 1/4/8/16 KiB, random around 32 and 64 KiB, random 1 MiB, and distribution change. The same-run control is the qualified `834c7a7e…` worker. These are point estimates, not a fresh full 81-case qualification or precision-supported floor.

The following table extracts natural 4 KiB. Ratios below one are faster.

| Source experiment | Whole encoder text, B | Mech/libdeflate, forward / reverse | Experiment/control, forward / reverse |
| --- | ---: | ---: | ---: |
| dp-production-guarded-profiled | 63,580 | 1.054870 / 1.074150 | 0.991880 / 1.001163 |
| dp-production-seeded-profiled | 63,564 | 1.038556 / 1.045580 | 0.977260 / 0.979760 |
| dp-production-seeded-body-profiled | 63,564 | 1.042490 / 1.043894 | 0.980932 / 0.982705 |
| dp-floor-seeded-clean-profiled | 63,548 | 1.027708 / 1.044967 | 0.981147 / 0.978732 |
| dp-floor-guarded-clean-profiled | 63,564 | 1.049235 / 1.055862 | 0.978531 / 0.991359 |
| dp-narrow-guarded-profiled | 63,580 | 1.053443 / 1.070816 | 0.988531 / 1.002221 |
| dp-narrow-seeded-profiled | 63,564 | 1.041366 / 1.050721 | 0.977804 / 0.985234 |
| dp-narrow-seeded-clean-profiled | 63,564 | 1.039958 / 1.041570 | 0.976254 / 0.974192 |
| dp-narrow-guarded-clean-profiled | 63,564 | 1.063816 / 1.069033 | 1.003140 / 1.005498 |
| dp-narrow-seeded-body-clean-profiled | 63,580 | 1.042152 / 1.045560 | 0.968956 / 0.980475 |

The seeded cleanup variants improve natural 1 KiB and distribution-change data as well. They keep natural 8 KiB, periodic 1 KiB, and 320-overlap below C in this discovery panel. Several guarded variants lose the natural-8-KiB peer win; those adverse results remain archived. Random 1 MiB remains approximately 1.36× C and 0.96× Roc. Every case and raw observation is retained; no full-score preservation is inferred from nineteen cases.

## What changed and what did not work

- **Late previous-value forwarding:** capture the existing exact `(length - 1)` read and unit append geometry, then apply forwarding after dense lookup composition and the other admitted scorer rewrites. Applying it earlier prevented the intended composition; that earlier source snapshot is preserved.
- **Source authority:** only the private NoGrowth constructor grants this optimization. It excludes indexed writes, unknown children, fallbacks and frame replacement of view geometry. The current write audit classifies all emitted array/packet/stack stores, including clones created later. The captured base must identify the exact current array packet word. Existing ownership, capacity, accounting and checked source paths remain.
- **Guarded versus entry-seeded recurrence:** the guarded mode falls back to the physical load before an append. The seeded mode reads the initialized last element once at entry, guarded against an empty array, then carries one value. The previous general-path read falls from 17,265 historical visits to five per natural-4-KiB compression. Guarded source has no added scorer stack loads; seeding introduces spills. The measured latency improvement, rather than instruction counts alone, is why the seeded candidate remains under consideration.
- **Late cleanup:** the existing join/CFG cleanup removes redundant instructions exposed by forwarding. On random 1 MiB it cuts roughly three million scorer instructions in the untimed diagnostic counts. That is not a measured whole-encoder speedup forecast.
- **Narrow carried cost:** a bounded repair carries I32 only for a proven low-word read while retaining the full stored node. It does not eliminate the spills and adds arithmetic on several random workloads. Both its positive and adverse latency measurements are included.
- **Caller length floor:** a prototype imported the existing 258-entry admission floor. Its one additional IR fold produced identical machine code because the backend already handled it. The prototype source and tests are archived; it is absent from the live compiler.
- **Inspection correction:** the first `dp-late-profiled` timing campaign used an inspection setting that retained extra fallback bodies and changed dispatch. It is classified as diagnostic. All `dp-production-*`, `dp-floor-*`, and `dp-narrow-*` campaigns use normal production observation settings; the separate report flag only logs the applied proof. No gain is attributed to inspection-enabled dispatch changes.

The compiler's production defaults reproduce the retained artifacts. The current compiler also reproduces all three artifacts of the 63,548 B wide cleanup candidate exactly. Source overlays, complete recipes and portable block profiles allow another agent to reproduce either candidate without depending on a built-only compiler tree.

## Validation and evidence

The exact complete production forwarding module and its committed tests pass optimized, verified native JIT comparisons over **5,760 combinations**: all three full/narrow read forms, empty and nonzero initial arrays, wrapping U32 costs, full-word upper bits, both append backedges, zero iterations and repeated calls. Three admission fixtures check the exact base and refusal of offset views or unknown writes before mutation. The harness copies the actual `packet_word`/incoming helpers verbatim. Historical prototype tests and compile failures remain separate.

Every timed production candidate passes 162 output/inflation/alias observations, six checked failures and the **4,848 B** active-stack gate under the 5,280 B limit. No new twelve-recovery campaign or full 81-case score is claimed for these discovery probes. A full optimized GPU test build was stopped before completion to finish the source repair; it is not reported as passing. Formatting, module layout and warning-policy checks pass. The two new committed JIT tests are executed through the exact-module harness.

`source-and-receipts.tar.gz` contains all DP compiler overlays, source recipes, objects, metadata, profile inputs, diagnostic counters, test logs, and the additional paired/cache repairs explored after the prior study. `raw-runtime.tar.gz` contains the actual timing observations and actor pins. `archive-files.json` pins every entry; `verify.py` recalculates hashes, native text sizes, canonical bytecode, committed source identity and every paired runtime ratio. Counter clones are untimed and never used as latency inputs. Time Profiler PCs may skid; block profile estimates are partial-flow layouts, not exact edge counts.

To replay an older prototype, apply its complete `source-inputs` overlay to its recorded base, including untracked files. Build using its recorded compiler command and profile. Rebind each `MECH_COMPACT_PHASE_BLOCK_LAYOUT` profile argument to `portable-profiles/<sha>.json` after hash verification. Use emission receipts for size and object identity; old inherited `size_bytes`/`encoder_sha256` runtime labels are not authoritative.

Fresh source-through-final-executable-link timings are recorded below. Compiler bootstrap is not codec compilation time.

## Fresh complete-build cohort

Ten balanced serial fresh-process orders per actor include final executable linking. All compilers and fixed support are prebuilt; strip is reported separately. Every trial reproduces its complete generated artifacts. These measurements belong to source `96b1f1ebc`; separate historical cohorts do not establish causal improvements.

| Actor | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Current compiler, selected encoder | 63,532 B | 2.216559 s | 2.228135 s | 276,072 B |
| Wide recurrence plus cleanup, unselected | 63,548 B | 2.219409 s | 2.230497 s | 276,072 B |
| Narrow recurrence plus cleanup, unselected | 63,564 B | 2.221740 s | 2.233070 s | 276,072 B |
| Roc | 57,696 B | 2.990605 s | 3.001593 s | 191,912 B |
| libdeflate | 37,080 B | 0.483321 s | 0.494416 s | 155,752 B |

Selected-build paired ratios are **0.740679× Roc / 4.578593× C**. The wide recurrence is **0.741845× Roc / 4.592967× C**; narrow is **0.742028× Roc / 4.596912× C**. These are build ratios, separate from compression latency.

The wide recurrence costs a paired median 1.843 ms versus the same compiler producing the selected encoder, 90% interval -7.380 to 7.986 ms, 5/10 faster. The narrow recurrence costs a paired median 2.041 ms versus the same compiler producing the selected encoder, 90% interval -2.627 to 16.039 ms, 3/10 faster. No established compilation-time improvement is claimed.

The wide candidate phase medians are parsing **705.497 ms**, semantics **217.842 ms**, artifact preparation **6.887 ms**, encoding **2.587 ms**, native pipeline **1,217.434 ms**, and final link **35.684 ms**. Catalog work overlaps parsing; separate medians do not add. Compiler bootstrap is excluded from all actors equally.

## Reproduce with a compiler built from the PR

From this evidence directory, use the `compile_profile` binary built from source `96b1f1ebc`:

```sh
python3 reproduce.py /path/to/compile_profile /tmp/dp-wide --variant wide
python3 reproduce.py /path/to/compile_profile /tmp/dp-narrow --variant narrow
python3 reproduce.py /path/to/compile_profile /tmp/dp-control --variant control
```

The script restores portable profiles, source-compiles the unchanged codec, and checks the object, complete metadata and bytecode. It never executes an archived compiler or machine-code patch. Rebuilding on other hardware does not inherit the M1 runtime score. The exact recorded compiler build uses nightly 2026-03-03, release opt-level 3, 16 codegen units, LTO disabled and one build job; its complete command and source overlay are archived.
