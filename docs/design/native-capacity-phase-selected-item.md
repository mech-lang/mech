# R274 — Carry the selected item in capacity-proved reductions

**Retain R274 with the R272 runtime.** Paired elapsed changes -3.089% across 81 cases. The score in this fresh comparison moves from 32/81 to 32/81 below 1× Roc. Both timing gates pass 81/81 and 81/81. The separate four-actor comparison records 3/81 candidate cases below 1× libdeflate.

Paired geomean changes by -3.167% and -3.011% in A and B. Below-parity crossings include none. Above-parity crossings include none. The largest paired slowdown is +0.449% on 33027-periodic-window.

Pooled gate misses include none. Block gate misses include none.

Measured parent e8e87ab7f84a41fe1455bc216136729d90ea3d72. Published parent d42011925b2ab21d5119802141d2612eaea9a36c. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The level-12 source, bytecode, tables, match rules, arithmetic, capacities and R272 runtime stay pinned. The timing boundary includes complete compression, scratch construction, output transfer and cleanup.

## Profile and compiler change

The retained natural 1 MiB profile has 2,938/5,351 samples in the capacity-proved path parser. Its short scalar body and setup account for 530 samples, and winner-index selection for 1,122. The existing four-lane loop accounts for 40. These are CPU samples from the exact linked binary. The large natural input therefore tests the reduction completion seam directly.

The same source-derived complete-child proof still admits the current views, original prices and scalar arithmetic. The existing no-growth contract admits the current unique owner and enough actual capacity for every reached append. Only that callee can carry the pure selected item with the current U32 minimum. It updates both on a strict improvement and preserves the incoming item on ties. The complete-child proof establishes source checks on the winning edge. On other candidates the compiler uses a total, memory-free extension of the same expression and discards its result. The long SIMD prefix retains its ordered winner and materializes that item once. Scalar tails carry the actual item. General, ordinary and checked fallbacks use the existing selection, ownership, publication and error paths.

The earlier R267 prototype changed the general phase. The named condition here is the current capacity-proved phase after growth-state isolation and threading. The first 19-case screen showed a natural-text improvement and exposed eight changed general/vector bodies. The final patch keeps their original item read at its original lexical emission point. Exact Object comparison verifies 347 named bodies and finds four changes, all in no-growth callees. The root stays at 1,039,432 code bytes and 57,072 frame bytes. Each changed callee grows from 1,376 to 1,408 code bytes and from 240 to 256 frame bytes.

For the audited single-candidate strict win, the complete selected reduction route remains at 37 instructions. It goes from five conditional branches and zero stack transfers to four and two. The tie route changes from 33 to 37 instructions and loses the same winner-dependent branch. This is a change in the control sequence. The timing data do not assign a separate cost to branch prediction or register placement.

The pinned reference has 11 instructions in its recurring scalar cost visit and uses one comparison for two adjacent selects. The exact libdeflate scalar loop uses a branch on improvement and stores the item on that edge. The separate Rust source control uses one comparison and two adjacent selects. The current Mech visit still emits a comparison before each select. All three peer snippets are included as machine-code evidence.

## Timing frontier

The following ratios use the separate 81-case four-actor cohort. It runs all 24 balanced actor orders per case, with matched runtime input, warmup, validation, exact raw streams and capacities. Libdeflate 1.26 creates and frees a compressor inside each measured call.

| Case | R272 / Roc | R274 / Roc | R272 / libdeflate | R274 / libdeflate |
|---|---:|---:|---:|---:|
| eight-literals | 0.201229× | 0.199891× | 2.554727× | 2.537733× |
| prefix-repeated-4096 | 0.516482× | 0.514253× | 1.984274× | 1.975714× |
| prefix-periodic-32768 | 0.982359× | 0.980489× | 2.096360× | 2.092368× |
| prefix-periodic-33027 | 0.998306× | 0.995772× | 2.114989× | 2.109621× |
| prefix-repeated-65536 | 1.050521× | 1.041300× | 1.858888× | 1.842571× |
| prefix-natural-32768 | 1.685446× | 1.475704× | 1.535526× | 1.344440× |
| prefix-natural-262144 | 1.747778× | 1.477482× | 1.417864× | 1.198590× |
| prefix-natural-1048576 | 1.760098× | 1.506684× | 1.419691× | 1.215288× |
| prefix-random-1048576 | 2.108295× | 2.107406× | 2.974567× | 2.973314× |

## Qualification and counters

- All 324 full calls across the original and decoded images match the reference bytes, independent inflate, capacities, input/output aliases and final zero private ownership. The two full Objects and metadata match byte-for-byte.
- The linked diagnostic build runs 126 controls across both artifact forms. Nine cases per form observe the actual capacity-proved route and check the first winning length at one, two, three, 15, 16 and 17 candidates, later segments and incoming ties. Additional cases cover distinct matrix and vector geometry, nonzero initial cost, exact capacity, growth, allocation rejection, assertion origin, unchanged prior publication, aliases, cleanup and retry.
- The first witness link ran out of disk space. Reclaiming verified old builds let the same link run. The control initially observed phase entry without distinguishing its internal route. The separate observer build records that route. The growth-denial witness needed its existing diagnostic injection restored after switching to the R272 allocation helper. These adjustments apply to the witness only.
- Both timing cohorts include all 81 cases, 24 balanced blocks per case and unchanged 1.05 pooled and block gates. The counter and heap cohorts are independent. The full library follow-up covers 245,664 timed calls and all 81 inputs. The exact-image CPU samples include the harness and validation. Ordinary managed recovery and hosted CI are outside this measurement boundary.
- The six logical heap families match across the 81 cases. Counters measure whole-call instructions, cycles and lifetime physical footprint. Natural 1 MiB retires 3.10% more instructions while using 14.55% fewer cycles. Eight literals uses 5.26% more cycles in this separate cohort. The primary elapsed measurements for eight literals pass both gates. All observations are included in the replay.

| Case | Before instructions | Candidate instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 812,177.5 | 812,169.0 | -0.001% | +5.257% |
| prefix-repeated-4096 | 4,050,412.0 | 4,051,191.0 | +0.019% | +0.889% |
| prefix-periodic-32768 | 24,863,308.0 | 24,867,712.0 | +0.018% | -0.170% |
| prefix-periodic-33027 | 25,369,621.5 | 25,373,765.5 | +0.016% | -0.656% |
| prefix-repeated-65536 | 49,388,516.5 | 49,398,169.0 | +0.020% | +0.164% |
| prefix-natural-32768 | 91,899,888.0 | 94,677,523.5 | +3.022% | -12.558% |
| prefix-natural-262144 | 922,814,237.0 | 954,411,590.5 | +3.424% | -15.254% |
| prefix-natural-1048576 | 3,350,219,905.0 | 3,454,155,914.5 | +3.102% | -14.546% |
| prefix-random-1048576 | 858,568,251.0 | 858,592,481.5 | +0.003% | -0.087% |

| Profile input | Running samples | Hottest linked symbol | Samples |
|---|---:|---|---:|
| eight-literals | 5509 | `_mech_compact_closed_v2` | 1777 |
| prefix-natural-1048576 | 5594 | `_mech_compact_numeric_15296_no_growth` | 2664 |
| prefix-periodic-32768 | 5357 | `_mech_compact_numeric_837_vector` | 3521 |
| prefix-repeated-33027 | 5562 | `_mech_compact_numeric_837_vector` | 3347 |

## All primary timings

| Input | Bytes | R272 / Roc | R274 / Roc | R274 / R272 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.104166× | 2.085302× | 0.991035× | pass/pass |
| one-stored | 1 | 1.976103× | 1.974509× | 0.999193× | pass/pass |
| seven-stored-prefix | 7 | 2.299336× | 2.277989× | 0.990716× | pass/pass |
| eight-literals | 8 | 0.197864× | 0.196688× | 0.994057× | pass/pass |
| eight-overlap | 8 | 0.186693× | 0.186795× | 1.000549× | pass/pass |
| 320-overlap | 320 | 0.228867× | 0.228543× | 0.998584× | pass/pass |
| eight-literals-prefix | 8 | 0.198280× | 0.197593× | 0.996536× | pass/pass |
| 128-biased-dynamic | 128 | 0.265275× | 0.266327× | 1.003969× | pass/pass |
| 32-high-entropy-stored | 32 | 0.263247× | 0.262575× | 0.997449× | pass/pass |
| 16384-distribution-change | 16384 | 1.686676× | 1.669415× | 0.989766× | pass/pass |
| 33027-periodic-window | 33027 | 0.991864× | 0.996321× | 1.004493× | pass/pass |
| random-64 | 64 | 0.280451× | 0.281199× | 1.002668× | pass/pass |
| random-256 | 256 | 0.350600× | 0.349978× | 0.998225× | pass/pass |
| random-1024 | 1024 | 0.441179× | 0.440458× | 0.998364× | pass/pass |
| repeated-1024 | 1024 | 0.306235× | 0.305822× | 0.998648× | pass/pass |
| random-32769 | 32769 | 1.725273× | 1.725829× | 1.000322× | pass/pass |
| repeated-32769 | 32769 | 0.957613× | 0.955729× | 0.998033× | pass/pass |
| reymont-prefix-1024 | 1024 | 0.552126× | 0.547145× | 0.990978× | pass/pass |
| reymont-prefix-16384 | 16384 | 1.599740× | 1.425775× | 0.891254× | pass/pass |
| reymont-prefix-32769 | 32769 | 1.681202× | 1.470112× | 0.874441× | pass/pass |
| heldout-window-32767 | 32767 | 1.679898× | 1.466883× | 0.873198× | pass/pass |
| heldout-window-32768 | 32768 | 1.683402× | 1.471783× | 0.874291× | pass/pass |
| heldout-slides-65537 | 65537 | 1.729212× | 1.512371× | 0.874601× | pass/pass |
| heldout-random-65537 | 65537 | 1.907988× | 1.907614× | 0.999804× | pass/pass |
| heldout-mixed-131073 | 131073 | 1.757038× | 1.645246× | 0.936375× | pass/pass |
| prefix-natural-1024 | 1024 | 0.554184× | 0.549071× | 0.990773× | pass/pass |
| prefix-natural-4096 | 4096 | 1.153027× | 1.105176× | 0.958499× | pass/pass |
| prefix-natural-8192 | 8192 | 1.402246× | 1.306708× | 0.931868× | pass/pass |
| prefix-natural-16384 | 16384 | 1.596212× | 1.427885× | 0.894546× | pass/pass |
| prefix-natural-32767 | 32767 | 1.683260× | 1.470441× | 0.873567× | pass/pass |
| prefix-natural-32768 | 32768 | 1.683327× | 1.472700× | 0.874875× | pass/pass |
| prefix-natural-32769 | 32769 | 1.681243× | 1.471151× | 0.875038× | pass/pass |
| prefix-natural-33027 | 33027 | 1.684111× | 1.471421× | 0.873708× | pass/pass |
| prefix-natural-65535 | 65535 | 1.723484× | 1.473464× | 0.854933× | pass/pass |
| prefix-natural-65536 | 65536 | 1.720756× | 1.471529× | 0.855164× | pass/pass |
| prefix-natural-65537 | 65537 | 1.722391× | 1.465310× | 0.850742× | pass/pass |
| prefix-natural-131072 | 131072 | 1.733882× | 1.476480× | 0.851546× | pass/pass |
| prefix-natural-262144 | 262144 | 1.746936× | 1.479786× | 0.847075× | pass/pass |
| prefix-natural-1048576 | 1048576 | 1.757804× | 1.502778× | 0.854918× | pass/pass |
| prefix-random-1024 | 1024 | 0.444472× | 0.444376× | 0.999783× | pass/pass |
| prefix-random-4096 | 4096 | 0.797506× | 0.796452× | 0.998679× | pass/pass |
| prefix-random-8192 | 8192 | 1.053921× | 1.049495× | 0.995801× | pass/pass |
| prefix-random-16384 | 16384 | 1.436715× | 1.435210× | 0.998953× | pass/pass |
| prefix-random-32767 | 32767 | 1.736390× | 1.733778× | 0.998496× | pass/pass |
| prefix-random-32768 | 32768 | 1.736624× | 1.737009× | 1.000222× | pass/pass |
| prefix-random-32769 | 32769 | 1.725926× | 1.727942× | 1.001168× | pass/pass |
| prefix-random-33027 | 33027 | 1.738992× | 1.734362× | 0.997338× | pass/pass |
| prefix-random-65535 | 65535 | 1.932495× | 1.932256× | 0.999876× | pass/pass |
| prefix-random-65536 | 65536 | 1.930898× | 1.928772× | 0.998899× | pass/pass |
| prefix-random-65537 | 65537 | 1.924748× | 1.919989× | 0.997527× | pass/pass |
| prefix-random-131072 | 131072 | 2.033763× | 2.031843× | 0.999056× | pass/pass |
| prefix-random-262144 | 262144 | 2.083904× | 2.086459× | 1.001226× | pass/pass |
| prefix-random-1048576 | 1048576 | 2.107201× | 2.105615× | 0.999247× | pass/pass |
| prefix-periodic-1024 | 1024 | 0.371100× | 0.370850× | 0.999325× | pass/pass |
| prefix-periodic-4096 | 4096 | 0.579315× | 0.580731× | 1.002444× | pass/pass |
| prefix-periodic-8192 | 8192 | 0.708073× | 0.705922× | 0.996963× | pass/pass |
| prefix-periodic-16384 | 16384 | 0.847140× | 0.846446× | 0.999181× | pass/pass |
| prefix-periodic-32767 | 32767 | 0.975738× | 0.974315× | 0.998542× | pass/pass |
| prefix-periodic-32768 | 32768 | 0.977254× | 0.977479× | 1.000230× | pass/pass |
| prefix-periodic-32769 | 32769 | 0.984870× | 0.984108× | 0.999227× | pass/pass |
| prefix-periodic-33027 | 33027 | 0.993487× | 0.993897× | 1.000412× | pass/pass |
| prefix-periodic-65535 | 65535 | 1.071838× | 1.071718× | 0.999888× | pass/pass |
| prefix-periodic-65536 | 65536 | 1.072178× | 1.069495× | 0.997498× | pass/pass |
| prefix-periodic-65537 | 65537 | 1.071657× | 1.070022× | 0.998474× | pass/pass |
| prefix-periodic-131072 | 131072 | 1.120481× | 1.117203× | 0.997074× | pass/pass |
| prefix-periodic-262144 | 262144 | 1.146474× | 1.147197× | 1.000631× | pass/pass |
| prefix-periodic-1048576 | 1048576 | 1.157579× | 1.159150× | 1.001357× | pass/pass |
| prefix-repeated-1024 | 1024 | 0.307403× | 0.307212× | 0.999377× | pass/pass |
| prefix-repeated-4096 | 4096 | 0.529044× | 0.530578× | 1.002900× | pass/pass |
| prefix-repeated-8192 | 8192 | 0.670146× | 0.669265× | 0.998686× | pass/pass |
| prefix-repeated-16384 | 16384 | 0.803675× | 0.803860× | 1.000231× | pass/pass |
| prefix-repeated-32767 | 32767 | 0.934041× | 0.931992× | 0.997807× | pass/pass |
| prefix-repeated-32768 | 32768 | 0.930091× | 0.928602× | 0.998399× | pass/pass |
| prefix-repeated-32769 | 32769 | 0.951761× | 0.950451× | 0.998624× | pass/pass |
| prefix-repeated-33027 | 33027 | 0.973888× | 0.969737× | 0.995738× | pass/pass |
| prefix-repeated-65535 | 65535 | 1.042072× | 1.039437× | 0.997471× | pass/pass |
| prefix-repeated-65536 | 65536 | 1.048081× | 1.040630× | 0.992891× | pass/pass |
| prefix-repeated-65537 | 65537 | 1.048733× | 1.043315× | 0.994833× | pass/pass |
| prefix-repeated-131072 | 131072 | 1.093580× | 1.092561× | 0.999068× | pass/pass |
| prefix-repeated-262144 | 262144 | 1.116069× | 1.116206× | 1.000123× | pass/pass |
| prefix-repeated-1048576 | 1048576 | 1.120390× | 1.120343× | 0.999958× | pass/pass |

[Verified compiler patch, linked controls and all measurement receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r274-selected-item-capacity-4fb1081c5175.tar.gz). SHA-256 `4fb1081c5175f5be6e67b85163af94c205620a8dddd91625ec9327782a75290c`.
