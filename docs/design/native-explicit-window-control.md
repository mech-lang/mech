# R259: explicit window guards with current geometry

**Archive this diagnostic control; retain R256.** The four-actor comparison completes on all 81 cases. The control is2.8501% slower geometrically by within-block paired effects; full R257 is1.0201% faster in the same cohort. The control/fullR257 ratio is1.039102. No allowance changes or favorable replacement runs. Larger-input parity remains incomplete.

Git parent `488046d851dcaef7569d9d8f56791965572a29dc`; retained compiler R256 `608ed69aedb98bb8c9d61b0bf6364580b304a6d1`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. One existing draft #973/branch. Ordinary IR/work/ABI/MBC215/107/39/14 and the R255 backend remain unchanged.

## What the control establishes

The27 newly geometry-reused adapter sites already have a source-window proof in R256. R256 consumes that proof and keeps a dynamic geometry gate; it does not retain a second dynamic adapter-window gate at those sites. The original source arithmetic and selected readable-window checks precede the packed read in every variant.

R257 consumes both proofs, making the adapter guard constant and deleting checked fallback definitions early. R258 keeps those definitions during SSA construction, but existing cleanup deletes the unreachable path later. R259 instead reuses geometry while deliberately withholding the source-window proof from the adapter. It emits an explicit enough/fits gate and keeps the original checked SSA/fallback reachable. This is a conservative proof-use ablation, not a pure geometry-only deletion. A controlled retention of fallback needs another dynamic condition; adding that condition is real extra work and limits attribution.

The caller passes `keep_dynamic_window` only at newly geometry-reused sites; old complete-child packs remain unchanged. Shape, current logical length, checked source arithmetic, canonical matrix fallback, owner/alias gates, failure ordering and current funding remain. Source algorithm/tables/depth300/nice258/EOF limits, real262144-U16 tree,1500513-U32 cache,305257-U64 reservation, output clear/reuse, allocation sites, allocator and timing boundary are unchanged. This is one executable with runtime inputs through1MiB.

The additional ordinary exact managed admission/accounting/publication/recovery contract remains omitted from the compact entry; tested private aliases/errors/cleanup do not restore it. No production resource-policy change is claimed.

## Matched primary results

R256, the explicit-window control, the original fullR257 and pinned fixed-level12 runtime-input reference are four persistent actors in the same cohort. Each of two fixed cohorts runs all24 actor permutations once per case, equal200ms warmup, matched borrowed output, clock, allocator and validation placement. Compression/scratch construction/transfer/private cleanup are timed; preparation and validation remain outside. Candidate/control and candidate/reference ratios are calculated inside each balanced block first. Both cohorts, legacy point/block gates and adverse results remain. CPU/heap cohorts are separate, and no build/test/profile/other compressor overlaps them.

| Case | R256 / Roc | Explicit window / Roc | FullR257 / Roc | Window change vs R256 | FullR257 change |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.416169× | 0.428989× | 0.417295× | +3.0804% | +0.2705% |
| prefix-natural-32768 | 1.841540× | 1.871839× | 1.817134× | +1.6453% | -1.3253% |
| prefix-random-32768 | 1.841187× | 1.912437× | 1.772060× | +3.8698% | -3.7545% |
| prefix-periodic-32768 | 1.090551× | 1.141804× | 1.094430× | +4.6997% | +0.3557% |
| prefix-repeated-32768 | 1.057452× | 1.099289× | 1.059449× | +3.9563% | +0.1888% |
| prefix-natural-1048576 | 1.905851× | 1.932734× | 1.885660× | +1.4105% | -1.0594% |
| prefix-random-1048576 | 2.171321× | 2.253865× | 2.103148× | +3.8016% | -3.1397% |
| prefix-periodic-1048576 | 1.267686× | 1.327850× | 1.277338× | +4.7460% | +0.7614% |
| prefix-repeated-1048576 | 1.221749× | 1.279962× | 1.231565× | +4.7648% | +0.8035% |

The explicit-window control passes79/81 combined legacy point gates and79/81 combined block gates. Periodic262144 and repeated262144 fail both. Individual cohorts retain75point/72block and78point/78block passes. FullR257 passes all 81 combined gates in this new four-actor comparison; this does not erase its R257 historical miss or establish adoption. Every individual control miss is in result-decision.json; no retiming or enlarged1.05 allowance.

## Instructions, cycles and parity budget

Changes below use the independent CPU cohort. Elapsed budget is `max(0, 1 - 1 / paired_reference_multiple)`; it is not a hardware attribution. No primary elapsed/counter cohorts are multiplied.

| Case | Window instructions change | Window cycles change | FullR257 instructions change | FullR257 cycles change | Window further time cut for1x |
|---|---:|---:|---:|---:|---:|
| eight-literals | +0.0641% | -2.2004% | -0.0134% | -3.0769% | 0.00% |
| prefix-natural-32768 | -1.0636% | -0.9133% | -2.5153% | -1.5934% | 46.58% |
| prefix-random-32768 | -0.8246% | -1.2911% | -4.1126% | -3.8245% | 47.71% |
| prefix-periodic-32768 | +0.2162% | +0.0505% | +1.4504% | +0.5803% | 12.42% |
| prefix-repeated-32768 | +0.2173% | -0.4668% | +1.4594% | -0.1305% | 9.03% |
| prefix-natural-1048576 | -1.1291% | -0.5143% | -2.5418% | -0.9067% | 48.26% |
| prefix-random-1048576 | -1.2408% | -0.7147% | -4.9534% | -2.9530% | 55.63% |
| prefix-periodic-1048576 | +0.2241% | -0.0588% | +1.4836% | +1.0858% | 24.69% |
| prefix-repeated-1048576 | +0.2202% | +0.1493% | +1.4641% | +1.1717% | 21.87% |

The explicit control has adverse primary elapsed despite fewer natural/random instructions and slightly lower independent cycle medians. Preserve that disagreement; extra checks, register allocation or layout are not assigned the full elapsed difference. All 81 before/control/fullR257 whole-heap and payload/copy/zero/private-peak fields are identical. Requested capacity, initialized bytes copied, per-call live storage and process RSS are different metrics. No allocation or input-streaming improvement is claimed.

## Actual emitted code

Only837_vector changes among340 numeric definitions:42800→42604B, frame928→944B. The other339 bodies/stack metadata are exact. Root Object bytes are freshly hash-identical to R256; root code1040856B/frame57088B and full worker2100808B remain. Object text1385816→1385560B. The27 static adapter sites include widths4/8 and all retain source-window proofs, deliberately unused for this adapter gate; static eligibility is not an executed site count.

Current random hot assembly shows the retained source check, explicit enough/fits comparison and checked byte fallback before the packed path. The repetitive SIMD comparison core remains equivalent apart from register choice. R259 keeps the scalar-tail read base in a register, avoiding the extra R257/R258 base reload. That is one concrete allocation difference, not a dynamic visit count or complete causal instruction ledger. The explicit checks and changed layout prevent a pure assignment of gains/losses to geometry or fallback removal.

## Current control profile

| Case | Running samples | Groups |
|---|---:|---|
| eight-literals | 5066 | `{"other current image / harness / library": 3572, "root": 736, "numeric phases": 758}` |
| prefix-natural-1048576 | 5375 | `{"numeric phases": 5266, "other current image / harness / library": 52, "root": 57}` |
| prefix-natural-32768 | 5367 | `{"numeric phases": 5106, "other current image / harness / library": 125, "root": 136}` |
| prefix-periodic-1048576 | 5374 | `{"numeric phases": 5179, "other current image / harness / library": 139, "root": 56}` |
| prefix-random-1048576 | 5634 | `{"numeric phases": 5545, "other current image / harness / library": 71, "root": 18}` |
| prefix-random-32768 | 5367 | `{"numeric phases": 5025, "other current image / harness / library": 239, "root": 103}` |

These are fresh exact-image CPU-time samples including harness/validation, not instruction partitions or removable-time budgets. Current symbol-relative PCs are used; root PCs are explicitly unattributed to source. No old source map is applied and no giant root IR dump is generated. Narrow actual control/reference assembly is retained.

## Pinned reference profile

| Case | Running samples | Groups |
|---|---:|---|
| eight-literals | 5362 | `{"generated compression": 5193, "other image / harness / library": 165, "reference builtin/ownership": 4}` |
| prefix-natural-1048576 | 5378 | `{"generated compression": 5290, "other image / harness / library": 88}` |
| prefix-natural-32768 | 5518 | `{"generated compression": 5369, "other image / harness / library": 146, "reference builtin/ownership": 3}` |
| prefix-periodic-1048576 | 5362 | `{"generated compression": 5285, "other image / harness / library": 77}` |
| prefix-random-1048576 | 5372 | `{"generated compression": 5319, "other image / harness / library": 52, "reference builtin/ownership": 1}` |
| prefix-random-32768 | 5363 | `{"generated compression": 5287, "other image / harness / library": 72, "reference builtin/ownership": 4}` |

These are fresh exact-image CPU-time samples including harness/validation, not instruction partitions or removable-time budgets. Current symbol-relative PCs are used; root PCs are explicitly unattributed to source. No old source map is applied and no giant root IR dump is generated. Narrow actual control/reference assembly is retained.

## Qualification and interruption

- Fresh 268800 original/decoded parent/control calls cover widths2/4/8, guarded/plain and live-alias variants, actual native wide reads before additional valid windows, canonical matrix oracle, logical length versus capacity, checked arithmetic/load/assertion order, zero/unselected paths, fresh visits, aliases, prior publication, failures, cleanup and retry. Status/origin/results agree. Calls are executions, not unique test definitions.

- Fresh 162 complete all 81 smoke calls and every primary/counter/profile call verify exact reference bytes, independent inflate, input/prepared-output aliases and capacity. Final completed worker sessions report zero private bytes/owners. The initial environmental session termination is recorded separately.

- Full disabled Object/metadata reproduces R256; final source annotations reproduce the measured control. GPU/worker builds keep warnings-as-errors, opt3, pinned nightly2026-03-03/Apple ARM64 and aot/jit/canonical-native/program/runtime. Frozen dependency hashes and exact commands remain.

- The unchanged R258715-test maintained suite, seven inherited exclusions and previous extended matrix/resource/work-ledger checks remain inherited, not rerun or relabelled fresh. Focused decoded image pairs execute; an additional decoded full compressor Object is not emitted. Ordinary managed-contract restoration, wider dependency/all-features/registry-project builds, multi-MB timing and separate workload suite stay outside this round. Hosted CI is cancelled by instruction and review workflows remain.

- Environmental ENOSPC interrupted pretty-JSON output after the first three tiny cases. The first two observation hashes are preserved, and all four worker journals contain a complete third batch plus CASE_DONE. Reconstructed third JSON matches the original written prefix byte-for-byte. No timing was repeated. All three completed cases are skipped on resume, with fresh equal warmups for the remaining cases and unchanged timers/order/gates. Initial failure receipts and partial data remain.

- Verified obsolete completed test binaries168440512B were removed; lossless map compression freed1286251892B and verified duplicate extractions were removed. Future observations are compressed between cases, with journals appended across resume. An inverted resume-helper assertion stopped before launching workers; it was corrected and all four scripts syntax-checked. This is storage/harness recovery, not a compiler/resource denial.

## Next bounded work

Retain R256. Close this explicit-window control as an adverse component and preserve R257/R258 separately. Pure geometry-only attribution remains limited by the existing source proof; search-versus-skip execution counts are not invented from static sites.

Next inspect and count the exact current empty outer path route. The helper receives the already-decremented header cursor; an empty result must keep that cursor. Its header read, literal cost and following node append remain. Current path15296 bytes match R256, and actual hot empty-route assembly still shows terminal-tag tests and scalar transport, but the whole closed route and eligible dynamic count remain to be established before changing it.

Then run the single current-baseline R247 placement control, preserving unrelated linked starts/root bytes, and inspect short-reduction completion/live-state pressure. Do not repeat old phase-only isolation or broad R252/R254 repair-argument forwarding. Only a demonstrated hot instruction sequence and its exact current fact authorize a loop-state change. One later lead is scalar winner-index conversion versus direct item transport on short total reductions; another is scalar-tail byte addressing versus the already-live vector pointers. Neither is implemented or assigned a saving. No backend switch or source algorithm change is proposed.

## All 81 paired observations

| Case | Bytes | R256 / Roc | Explicit window / Roc | FullR257 / Roc | Window change | FullR257 change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 0 | 2.151859× | 2.103028× | 2.138119× | -2.2693% | -0.6385% |
| one-stored | 1 | 2.036012× | 2.032360× | 2.038126× | -0.1793% | +0.1038% |
| seven-stored-prefix | 7 | 2.240904× | 2.263537× | 2.274004× | +1.0100% | +1.4771% |
| eight-literals | 8 | 0.416169× | 0.428989× | 0.417295× | +3.0804% | +0.2705% |
| eight-overlap | 8 | 0.413607× | 0.420802× | 0.411427× | +1.7396% | -0.5272% |
| 320-overlap | 320 | 0.455684× | 0.466565× | 0.451347× | +2.3878% | -0.9519% |
| eight-literals-prefix | 8 | 0.417314× | 0.426675× | 0.414875× | +2.2433% | -0.5845% |
| 128-biased-dynamic | 128 | 0.415828× | 0.421988× | 0.410529× | +1.4814% | -1.2743% |
| 32-high-entropy-stored | 32 | 0.422736× | 0.432777× | 0.424669× | +2.3752% | +0.4571% |
| 16384-distribution-change | 16384 | 1.887545× | 1.947041× | 1.833366× | +3.1520% | -2.8703% |
| 33027-periodic-window | 33027 | 1.106918× | 1.155579× | 1.107379× | +4.3961% | +0.0416% |
| random-64 | 64 | 0.438393× | 0.437640× | 0.434221× | -0.1716% | -0.9516% |
| random-256 | 256 | 0.478755× | 0.484339× | 0.475853× | +1.1663% | -0.6062% |
| random-1024 | 1024 | 0.541069× | 0.550817× | 0.533312× | +1.8016% | -1.4336% |
| repeated-1024 | 1024 | 0.518218× | 0.530419× | 0.520608× | +2.3546% | +0.4613% |
| random-32769 | 32769 | 1.821815× | 1.894903× | 1.759946× | +4.0118% | -3.3960% |
| repeated-32769 | 32769 | 1.060838× | 1.105628× | 1.063702× | +4.2221% | +0.2699% |
| reymont-prefix-1024 | 1024 | 0.680362× | 0.692514× | 0.680995× | +1.7861% | +0.0930% |
| reymont-prefix-16384 | 16384 | 1.767725× | 1.798486× | 1.741241× | +1.7402% | -1.4982% |
| reymont-prefix-32769 | 32769 | 1.840607× | 1.869906× | 1.817449× | +1.5918% | -1.2582% |
| heldout-window-32767 | 32767 | 1.847039× | 1.879642× | 1.821599× | +1.7652% | -1.3773% |
| heldout-window-32768 | 32768 | 1.847476× | 1.876774× | 1.821286× | +1.5858% | -1.4176% |
| heldout-slides-65537 | 65537 | 1.905515× | 1.931222× | 1.882941× | +1.3491% | -1.1847% |
| heldout-random-65537 | 65537 | 1.995153× | 2.070601× | 1.921125× | +3.7815% | -3.7104% |
| heldout-mixed-131073 | 131073 | 1.923984× | 1.998316× | 1.896236× | +3.8634% | -1.4422% |
| prefix-natural-1024 | 1024 | 0.679995× | 0.685365× | 0.673523× | +0.7897% | -0.9517% |
| prefix-natural-4096 | 4096 | 1.328223× | 1.345398× | 1.290722× | +1.2931% | -2.8234% |
| prefix-natural-8192 | 8192 | 1.598018× | 1.631055× | 1.571315× | +2.0674% | -1.6710% |
| prefix-natural-16384 | 16384 | 1.765097× | 1.797826× | 1.738929× | +1.8543% | -1.4825% |
| prefix-natural-32767 | 32767 | 1.843108× | 1.875714× | 1.820505× | +1.7691% | -1.2263% |
| prefix-natural-32768 | 32768 | 1.841540× | 1.871839× | 1.817134× | +1.6453% | -1.3253% |
| prefix-natural-32769 | 32769 | 1.841854× | 1.873740× | 1.817206× | +1.7312% | -1.3382% |
| prefix-natural-33027 | 33027 | 1.842789× | 1.874197× | 1.818535× | +1.7044% | -1.3162% |
| prefix-natural-65535 | 65535 | 1.909017× | 1.943756× | 1.889446× | +1.8197% | -1.0252% |
| prefix-natural-65536 | 65536 | 1.906727× | 1.940786× | 1.888302× | +1.7863% | -0.9663% |
| prefix-natural-65537 | 65537 | 1.911824× | 1.944542× | 1.890679× | +1.7113% | -1.1060% |
| prefix-natural-131072 | 131072 | 1.907591× | 1.940683× | 1.883755× | +1.7348% | -1.2495% |
| prefix-natural-262144 | 262144 | 1.909292× | 1.942428× | 1.893078× | +1.7355% | -0.8492% |
| prefix-natural-1048576 | 1048576 | 1.905851× | 1.932734× | 1.885660× | +1.4105% | -1.0594% |
| prefix-random-1024 | 1024 | 0.540171× | 0.550624× | 0.537829× | +1.9352% | -0.4335% |
| prefix-random-4096 | 4096 | 1.007092× | 1.015186× | 0.941095× | +0.8037% | -6.5532% |
| prefix-random-8192 | 8192 | 1.206269× | 1.244129× | 1.155000× | +3.1386% | -4.2502% |
| prefix-random-16384 | 16384 | 1.556310× | 1.623149× | 1.506979× | +4.2947% | -3.1698% |
| prefix-random-32767 | 32767 | 1.840227× | 1.906070× | 1.779239× | +3.5779% | -3.3142% |
| prefix-random-32768 | 32768 | 1.841187× | 1.912437× | 1.772060× | +3.8698% | -3.7545% |
| prefix-random-32769 | 32769 | 1.829494× | 1.893626× | 1.758208× | +3.5054% | -3.8965% |
| prefix-random-33027 | 33027 | 1.896699× | 1.986325× | 1.865136× | +4.7254% | -1.6641% |
| prefix-random-65535 | 65535 | 2.010285× | 2.087156× | 1.941476× | +3.8239% | -3.4229% |
| prefix-random-65536 | 65536 | 2.006212× | 2.096081× | 1.939890× | +4.4796% | -3.3058% |
| prefix-random-65537 | 65537 | 1.996940× | 2.072275× | 1.930121× | +3.7725% | -3.3461% |
| prefix-random-131072 | 131072 | 2.089221× | 2.164635× | 2.021663× | +3.6097% | -3.2336% |
| prefix-random-262144 | 262144 | 2.187193× | 2.263916× | 2.122939× | +3.5078% | -2.9377% |
| prefix-random-1048576 | 1048576 | 2.171321× | 2.253865× | 2.103148× | +3.8016% | -3.1397% |
| prefix-periodic-1024 | 1024 | 0.527473× | 0.537386× | 0.526542× | +1.8793% | -0.1764% |
| prefix-periodic-4096 | 4096 | 0.770656× | 0.783976× | 0.753730× | +1.7283% | -2.1964% |
| prefix-periodic-8192 | 8192 | 0.880333× | 0.904723× | 0.882281× | +2.7705% | +0.2212% |
| prefix-periodic-16384 | 16384 | 0.989899× | 1.029671× | 0.984866× | +4.0178% | -0.5084% |
| prefix-periodic-32767 | 32767 | 1.092894× | 1.141078× | 1.099244× | +4.4089% | +0.5810% |
| prefix-periodic-32768 | 32768 | 1.090551× | 1.141804× | 1.094430× | +4.6997% | +0.3557% |
| prefix-periodic-32769 | 32769 | 1.095392× | 1.147273× | 1.104508× | +4.7364% | +0.8323% |
| prefix-periodic-33027 | 33027 | 1.106374× | 1.156332× | 1.109357× | +4.5154% | +0.2696% |
| prefix-periodic-65535 | 65535 | 1.174096× | 1.230044× | 1.178482× | +4.7653% | +0.3736% |
| prefix-periodic-65536 | 65536 | 1.173244× | 1.227882× | 1.180900× | +4.6570% | +0.6525% |
| prefix-periodic-65537 | 65537 | 1.178250× | 1.230999× | 1.180386× | +4.4769% | +0.1812% |
| prefix-periodic-131072 | 131072 | 1.216640× | 1.275864× | 1.226387× | +4.8678% | +0.8012% |
| prefix-periodic-262144 | 262144 | 1.237006× | 1.310382× | 1.248638× | +5.9317% | +0.9403% |
| prefix-periodic-1048576 | 1048576 | 1.267686× | 1.327850× | 1.277338× | +4.7460% | +0.7614% |
| prefix-repeated-1024 | 1024 | 0.523156× | 0.525809× | 0.517549× | +0.5069% | -1.0718% |
| prefix-repeated-4096 | 4096 | 0.758341× | 0.770099× | 0.747772× | +1.5505% | -1.3938% |
| prefix-repeated-8192 | 8192 | 0.864148× | 0.882559× | 0.855970× | +2.1305% | -0.9464% |
| prefix-repeated-16384 | 16384 | 0.954499× | 0.991275× | 0.958028× | +3.8529% | +0.3697% |
| prefix-repeated-32767 | 32767 | 1.057266× | 1.100514× | 1.059579× | +4.0905% | +0.2188% |
| prefix-repeated-32768 | 32768 | 1.057452× | 1.099289× | 1.059449× | +3.9563% | +0.1888% |
| prefix-repeated-32769 | 32769 | 1.060276× | 1.097966× | 1.063164× | +3.5547% | +0.2723% |
| prefix-repeated-33027 | 33027 | 1.080723× | 1.120631× | 1.084937× | +3.6927% | +0.3899% |
| prefix-repeated-65535 | 65535 | 1.138343× | 1.187936× | 1.147528× | +4.3566% | +0.8069% |
| prefix-repeated-65536 | 65536 | 1.139370× | 1.190218× | 1.144830× | +4.4628% | +0.4792% |
| prefix-repeated-65537 | 65537 | 1.140638× | 1.188576× | 1.150522× | +4.2028% | +0.8665% |
| prefix-repeated-131072 | 131072 | 1.186237× | 1.233513× | 1.186961× | +3.9854% | +0.0610% |
| prefix-repeated-262144 | 262144 | 1.225227× | 1.302744× | 1.237262× | +6.3267% | +0.9822% |
| prefix-repeated-1048576 | 1048576 | 1.221749× | 1.279962× | 1.231565× | +4.7648% | +0.8035% |

Offline replay recomputes six reports from all retained paired observations, counters and profile aggregates without a compiler/compressor. Complete826-file compiler source and reversible patch to825-file R256, commands, pins, partial/recovered observations and narrow assembly are archived. Native binaries/Objects/libraries, bytecode, full compressor descriptors/IR/maps and raw traces/stacks are excluded.

[Verified evidence archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r259-explicit-window-cdc32081dd49.tar.gz) — SHA-256 `cdc32081dd49a091adb3426c7dc8571594d68f923aa0edbbd1fa77287ea86a64`, 12,045,072 bytes, 2,425 files. Fresh public retrieval verifies every file, exact826-file source reconstruction/reversal to825-file R256 and six report replays. No executables, bytecode, full compressor descriptors/maps/IR or raw traces are included.
