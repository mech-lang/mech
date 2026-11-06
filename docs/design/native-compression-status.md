# Native compression: current status

**R148 is closed for draft review. Prior published heads and the R146 closure branch remain frozen. No implementation task is active.**

| Checkpoint | Value |
|---|---|
| Base | #980 at `2d9d5671ada86d846a33c9620515062280381799` |
| Before / retained runtime | `c4759eb2f5b26e8e883cdc317b57cdc7a218ba8d` / `d9fd2855a2b3aef7a7f843f051466abd39f7ef25` |
| Qualified test revision | `bec5df7ddb426e7c1692204f098527113d63ef69` |
| IR/work/ABI/bytecode | **173/107/39/14**, unchanged |
| Bounded change | Skip impossible paid-source scans using validated loop-ID ordering; exact positive checks remain |
| Frozen prior PRs | #973 → #976 → #977 → #978 → #979; R146 closure; #980 |

One final 20-case cohort: geometric after/before **0.991778937518 (-0.822106%)**; 7 improving / 13 adverse medians. All unchanged 1.05 guards pass. All 3,968 primary / 4,048 total complete calls retain exact bytes, independent inflate, input aliases, prefixes, capacities/clear-reuse and zero final managed data. Both development screens remain separate; no pooling or guard repeats.

| Input | Current ms | Mech / Roc before→after | Change |
|---|---:|---:|---:|
|eight-literals|1.360687|4.7053×→4.7246×|+0.4105%|
|16384-distribution-change|6.214937|7.9129×→7.8670×|-0.5806%|
|33027-periodic-window|10.444499|10.1812×→8.3091×|-18.3883%|
|random-32769|7.919292|7.5456×→7.4852×|-0.8009%|
|repeated-32769|9.558958|7.1320×→7.1256×|-0.0895%|
|reymont-prefix-32769|35.196521|8.0634×→8.0782×|+0.1831%|
|reymont-prefix-16384|18.465375|8.6606×→8.6855×|+0.2879%|
|random-1024|1.262729|4.0027×→4.0214×|+0.4674%|
|repeated-1024|1.547187|4.7983×→4.8049×|+0.1388%|
|reymont-prefix-1024|1.925625|5.8197×→5.8264×|+0.1148%|
|320-overlap|1.354667|4.5576×→4.5999×|+0.9282%|
|random-256|1.320021|4.4865×→4.4899×|+0.0742%|
|128-biased-dynamic|1.154500|3.9851×→3.9948×|+0.2442%|
|random-64|1.268708|4.3272×→4.3523×|+0.5813%|
|32-high-entropy-stored|1.247188|4.3870×→4.3915×|+0.1021%|
|eight-overlap|1.322375|4.4869×→4.4979×|+0.2448%|
|eight-literals-prefix|1.352500|4.6565×→4.6318×|-0.5286%|
|seven-stored-prefix|0.119834|clock-limited reference|-0.3629%|
|one-stored|0.120125|clock-limited reference|-0.2769%|
|empty|0.120958|clock-limited reference|+2.7249%|

All 13 adverse medians remain, including eight-byte literals +0.410479%, overlap320 +0.928209% and empty +2.724852%. Scalar controls are all adverse (maximum 1.011630886559) and pass the same 1.05 gate. No small-case neutrality claim or cause assigned to cross-cohort variation.

Separate counters: periodic 225.347026M→187.398522M instructions (-16.840029%) and cycles -18.017667%. Eight-byte instructions -0.004822%. All 20 heap distributions—requests, requested bytes, reallocations, copy upper bounds and incremental usable peaks—and logical work match exactly. OS memory metrics remain separately archived; requested capacity is not initialized bytes copied.

Direct isolated census: three periodic calls per side show predicate entries 231,382→13,055, inventory searches 229,966→11,639, and segment queries 218,334→7. All saved counts reconcile to 218,327/call; successful admissions and compute charges are identical. This counts actual function entries, not ordinary visit events, and makes no diagnostic timing claim.

Current four-case CPU-time profiles show 52.25–72.01% generated-exclusive weights. Periodic is 69.84% generated / 28.00% host callbacks including frames and cleanup / 2.16% outside or unclassified. These are time samples, not operation or retired-instruction shares. This round removes host checks and adds no lowering coverage.

The original exact source-paid predicate is byte-identical. Every segment member shares its parent iteration; checked preparation requires parent IDs to decrease. A newer iteration therefore cannot be a paid ancestor. IDs passing the cheap comparison retain the full exact inventory/ancestry scan. Region matching, charges, current owners/arguments, admission, failure order/publication/cleanup/retry and finite fallback stay unchanged. No cache, authority bank, new emitter or source rewrite.

Build-owned restoration preserves strict parent-ID order under the existing SAME unchanged checked-plan/Object provenance contract; it does not rerun the full Engine ordering validator. Selected composition membership remains independently reconstructed. Original/decoded/direct/restored tests require non-vacuous false positives, above-root/sibling/None cases and exact inventory holes; actual lifecycle/recovery controls remain.

GPU 571 pass / 7 same exclusions; CI 146 listed, guarded and executed; workflow 132, architecture 15, runtime-only and formatting pass. Focused 6 pass; 24 existing handoff/funding controls pass within the broader GPU run. Unchanged source-derived 236 components / 8 composition / 1 real-capacity path pass; all 138 support sources match. Scalar 12 and frame 8 pass. Nightly 2026-03-03/aarch64/edition2024; warnings denied; library opt3, unit/component roots opt1. Core/Engine/math/dependencies remain frozen and rehashed.

R147 Object/descriptor/metadata/source/MBC are reused byte-identically; generated text 8,848,408 B and machine entry stack 12,544 B stay unchanged. New runtime binary is linked and pinned separately. Algorithm/tables/depth/nice/EOF, real scratch capacities, source allocation sites, output13 clear/reuse, input aliases, allocator and complete evaluate_into timing remain pinned.

| Work board | Disposition |
|---|---|
| Completed | Prior finite milestone and reconciliation; R147 descendant pieces; R148 paid-source scan task |
| Rejected/preserved | R131 snapshots, R141 early refusal, R143 stores; R148 ancestry-walk development candidate superseded |
| Active | None; stop at the review checkpoint |
| Next | Integration review; separately assigned image-validation usability follow-up |
| Deferred | Further lowering/instruction work, child funding, source rewrites, ownership/allocation/SIMD |

Introduced test-only inventory-size and missing above-root assumptions are corrected. An attempted recursive-comprehension witness hit the inherited frozen Engine restriction and is excluded; no language expansion. All failures and sources remain archived. No unresolved introduced functional failure. Read-only implementation review found no correctness/accounting blocker.

Known limitations remain explicit: R140 eight-byte +1.322924% is a dispositioned historical regression. R136 cold image admission rose roughly17 s→35–37 s; R142 improved its matched median1.426206% to34.580817458 s, leaving a separate usability cost. No new matched startup cohort. Finite grandchild coverage proves marker restoration and ordinary return before later Work denial; no whole-program finite-success claim.

Same seven profile exclusions, full dependency/all-features graph, multi-MB timing and separate ten-workload coverage remain skipped. Ordinary hosted CI is cancelled under standing instruction; **draft until green hosted CI and clean external review**. Exact commands, revisions, source/native pins, raw receipts, rejected attempts and replay are external in [R148 evidence](../../benchmarks/mech-deflate/experiments.json). The architecture describes the maintained system; no further implementation begins in this increment. Performance parity remains the longer-term objective.
