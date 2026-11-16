# R267 — Reduce the state carried through numeric phases

**Retain R264 for the compact executable.**
Paired geometric elapsed changed -2.1243% across all 81 cases. The share faster than Roc is 22/81 (27.16%) before and 22/81 (27.16%) for the candidate.

PR base `92f75db84e2523693648084bea16cd52ad60777a`. Published parent `8fb68d5649a0c85f5bbafb19cea53ffb387f29c5`. Retained baseline source R264 `9676a9cbad760dfc44f9b69aee746e1ca9080e13`. IR/work/ABI/bytecode 215/107/39/14. Cranelift 0.131.3 and regalloc2 0.15.2.

The candidate is qualified for the compact executable. Its measurement boundary covers complete level-12 compression, scratch construction, output transfer and private cleanup. Ordinary managed admission, accounting and recovery are outside this boundary. The maintained compiler has a separate managed execution contract.

## Full comparison

Two independent elapsed cohorts use all six orders with three actors, four repetitions per order and matched 200 ms warmups. The report aggregates ratios inside each balanced block. The score counts strict paired elapsed ratios below 1.0. The original pooled-median and block-median gates keep their 1.05 ceilings. CPU and heap counters use separate cohorts. Input, source, allocator, exact output, decompression and timing boundary pins are recorded in the packet.

- Primary A -2.3273% and B -1.9210%.
- Pooled gate 81/81. Block gate 79/81.
- Cases crossing 1× in the matched comparison none.
- Pooled gate misses none.
- Block gate misses `empty`, `one-stored`.
- Exact measured heap payload, initialized copy, zeroing and private peak families equal across all 81: True.

| Case | R264 / Roc | Candidate / Roc | Elapsed vs R264 | Retired instructions vs R264 | Cycles vs R264 |
|---|---:|---:|---:|---:|---:|
| prefix-repeated-32767 | 1.02345× | 1.00157× | -2.138% | -5.716% | -2.276% |
| prefix-repeated-32768 | 1.02433× | 1.00468× | -1.919% | -5.719% | -2.443% |
| prefix-repeated-32769 | 1.03258× | 1.00284× | -2.880% | -5.669% | -2.877% |
| prefix-repeated-33027 | 1.03809× | 1.01586× | -2.141% | -5.610% | -2.952% |
| prefix-periodic-32768 | 1.05766× | 1.03260× | -2.369% | -5.705% | -2.665% |
| prefix-periodic-32769 | 1.06133× | 1.03907× | -2.097% | -5.655% | -3.199% |
| eight-literals | 0.41593× | 0.41354× | -0.575% | -0.033% | -2.890% |
| prefix-natural-32768 | 1.81875× | 1.75286× | -3.623% | -7.417% | -4.221% |
| prefix-natural-1048576 | 1.89164× | 1.81871× | -3.855% | -7.024% | -4.002% |
| prefix-random-1048576 | 2.16167× | 2.13155× | -1.394% | -7.503% | -1.108% |

## Why R265 introduced a recurring stack transfer

The trace rebuilt both actual complete-program Objects byte for byte. Their metadata differs only in the source-input-form annotation. CLIF supplies input packet lane 1 as the cache cursor, lane 7 as cache length and lane 27 as the node-buffer owner. The output packet pointer is the third argument. R265 deletes ten dead nontrapping packet loads from the optimized phase. The loop's semantic state still enters the same eight-argument CLIF header.

| Quantity | R264 | R265 |
|---|---|---|
| Cache cursor input CLIF and VCode | `v24`, `v928`, header `v233` | `v24`, `v908`, header `v223` |
| Current cursor update | VCode instruction 170, `v887` in x26 | Instruction 160, `v877` in x14 then stack2 |
| Cache length live range | Bundle 328, priority 829, spill cost 34 | Bundle 318, priority 819, spill cost 35 |
| Owner returned from cold growth | Bundle 74, instruction 305, x24 | Bundle 74, instruction 295, x26 |
| Incoming owner | `v930`, stack2, sp+0x70 | `v910`, stack3, sp+0x78 |
| Output packet pointer | `v194`, stack1, sp+0x68 | `v194`, stack1, sp+0x68 |

The length bundle has the same summed use weight, 29,000. Its priority denominator drops by ten when those entry loads disappear, so the allocator's integer spill cost rises from 34 to 35. The cursor bundle initially gets x26 in both images and remains cost 34. The growth-result owner scores 3,700 in both. R264 encounters the equal-cost length first at x24 and evicts it. R265's cheapest conflict becomes the cursor at x26. The allocator evicts it, then splits it after the retry finds no cheaper victim. The recurring cursor ranges share stack2. Before VCode instructions 161 and 168, allocator edits move x14 to stack2 and stack2 to x23. Those become the linked `stur x14, [sp,#0x70]` and `ldur x23, [sp,#0x70]` after the cache-cursor update. The output pointer stays in its old home and the incoming owner moves one slot. Growth handling creates this interference even on visits that never take the allocation edge. The static route audit bounds the emitted work on one path. The independently measured retired instructions and elapsed times remain separate observations.

## Capacity and ownership

The proof reuses the checked canonical phase inventory. It requires one root-local unit append, a dominating nonzero guard, a root-local current countdown that decreases by one on every reached latch, and an acyclic traversal after declared latches are cut. The source validator proves root-local immutable values are freshly defined on every visit. The proof resolves the countdown field through checked packs, projections and copies on the selected branch. Nested reductions keep their own checked work. Refusal keeps the existing general body.

The compact caller first verifies the exact current writable owner is unique and the writable view is a row. It then checks `length <= capacity && remaining <= capacity - length` with the current length, capacity and countdown. The selected callee emits the original checked plan with its growth arm removed. The original body handles insufficient capacity. The source append still initializes the cell before advancing length. Final owner/view export and source failures use the same boundary.

| Reached empty-match visit | Instructions | Stack transfers | Path code / frame |
|---|---:|---:|---:|
| R264 | 85 | 17 | 2,048 / 288 B |
| R265 | 86 | 19 | 2,000 / 288 B |
| Unit predicate | 80 | 19 | 2,016 / 288 B |
| No-growth phase | 61 | 2 | 1,424 / 240 B |
| Threaded after growth isolation | 50 | 2 | 1,376 / 240 B |

The route starts at the positive countdown header, uses a readable empty-match cache header and literal/node cells, appends once and returns to the next header. Two allocation-related indirect calls in the general phase disappear from the selected callee. The cache cursor stays in x21, the output pointer stays in x2, and the input owner stays in x11 on the audited no-growth route. The two remaining transfers load literal cost-table geometry. These are explicit static route counts.

The observer ran 162 complete calls with the fixed inventory. It found 404 observed phase entries per complete pass through the 81 cases, all selecting the no-growth body with current length 258, capacity 305,257 and owner reference count one. The largest countdown is 300,095. Each callback borrows the initialized packet for the duration of the observation. All output, independent inflate, alias and cleanup checks pass twice. Observations use a separate executable.

## Separate pilots

### Natural screen

| Input | R264 / Roc | total-item / Roc |
|---|---:|---:|
| eight-literals | 0.39631× | 0.41426× |
| prefix-natural-32768 | 1.82267× | 1.61068× |
| prefix-natural-1048576 | 1.88915× | 1.63019× |
| prefix-random-32768 | 1.80126× | 1.86544× |
| prefix-random-1048576 | 2.16384× | 2.24093× |
| prefix-periodic-32768 | 1.07125× | 1.10763× |
| prefix-repeated-4096 | 0.74860× | 0.75982× |
| prefix-repeated-32767 | 1.03078× | 1.06865× |
| prefix-repeated-32768 | 1.04375× | 1.07870× |

`total-item` screen gate misses none.

### Capacity screen

| Input | R264 / Roc | unit / Roc | no-growth-refined / Roc |
|---|---:|---:|---:|
| eight-literals | 0.40849× | 0.40367× | 0.41281× |
| prefix-natural-32768 | 1.82282× | 1.83315× | 1.76249× |
| prefix-natural-1048576 | 1.89213× | 1.90014× | 1.82042× |
| prefix-random-32768 | 1.82356× | 1.89254× | 1.78191× |
| prefix-random-1048576 | 2.16688× | 2.24955× | 2.13128× |
| prefix-periodic-32768 | 1.06983× | 1.11139× | 1.03622× |
| prefix-repeated-4096 | 0.73910× | 0.75493× | 0.71816× |
| prefix-repeated-32767 | 1.02157× | 1.06031× | 0.99720× |
| prefix-repeated-32768 | 1.02927× | 1.05658× | 1.00126× |

`unit` screen gate misses none.

`no-growth-refined` screen gate misses none.

### Segments screen

| Input | R264 / Roc | total-item / Roc | segments-image / Roc |
|---|---:|---:|---:|
| eight-literals | 0.39369× | 0.41069× | 0.41671× |
| prefix-natural-32768 | 1.82396× | 1.61306× | 1.60590× |
| prefix-natural-1048576 | 1.88985× | 1.62762× | 1.61883× |
| prefix-random-32768 | 1.81557× | 1.88804× | 1.88070× |
| prefix-random-1048576 | 2.16713× | 2.24721× | 2.25163× |
| prefix-periodic-32768 | 1.07624× | 1.11844× | 1.10457× |
| prefix-repeated-4096 | 0.74539× | 0.77038× | 0.76906× |
| prefix-repeated-32767 | 1.03019× | 1.06476× | 1.05877× |
| prefix-repeated-32768 | 1.02752× | 1.06868× | 1.06929× |

`total-item` screen gate misses none.

`segments-image` screen gate misses eight-literals.

### Isolated-thread screen

| Input | R264 / Roc | no-growth-refined / Roc | isolated-threaded / Roc |
|---|---:|---:|---:|
| empty | 1.97619× | 1.97619× | 1.97619× |
| one-stored | 1.97619× | 1.97619× | 1.97619× |
| eight-literals | 0.40342× | 0.41709× | 0.41132× |
| prefix-natural-32768 | 1.82239× | 1.76520× | 1.68452× |
| prefix-natural-1048576 | 1.88892× | 1.82086× | 1.75304× |
| prefix-random-32768 | 1.81271× | 1.79542× | 1.77573× |
| prefix-random-1048576 | 2.16832× | 2.13012× | 2.11963× |
| prefix-periodic-32768 | 1.07025× | 1.04292× | 1.03300× |
| prefix-repeated-4096 | 0.73895× | 0.72232× | 0.71520× |
| prefix-repeated-32767 | 1.02037× | 1.00859× | 0.99509× |
| prefix-repeated-32768 | 1.02952× | 1.00546× | 0.99454× |
| prefix-repeated-32769 | 1.03743× | 1.01469× | 1.00201× |
| prefix-repeated-33027 | 1.04964× | 1.03142× | 1.00948× |

`no-growth-refined` screen gate misses empty, one-stored.

`isolated-threaded` screen gate misses none.

The unit predicate control replaces the eight-instruction spare-cell expression with unsigned length less than capacity. Its screened allocation still puts the cursor on the stack, and the nine-case paired geometric elapsed rises. The no-growth candidate uses the original predicate in the general body.

The separate natural-text variant carries the selected item and minimum together. It computes a pure, total machine expression for the item. The complete-child proof certifies the source checks on the actual winning edge, and strict selection preserves the previous item on a tie. The original vector winner and short/long boundary stay in place. The scalar finish stops reconstructing its item from an index sentinel. The natural 1 MiB entry census contains 14,442,759 reductions and 39,253,430 candidate lengths. 11,700,784 reductions have one to three lengths. These are source-derived visit counts.

The follow-up accumulator prototype proves the three returned scalar lanes are exact projections on every reached terminal edge. The parent receives the current index, minimum and item directly and carries them into its next segment. This removes the child terminal tag reconstruction for completed reductions. The original gate remains per segment. The eight-byte screen misses the existing gate, so the prototype remains separate from the full comparison.

R260 was repeated only after isolating growth state and confirming that the cursor stayed in a register. The same three threaded edges and six local repair parameters shorten the audited route from 61 to 50 instructions while keeping its two stack transfers. The former fixed-placement experiment remains historical evidence. [R260 fixed-placement control](native-empty-path-placement-control.md)

The actual pinned reference path uses adjacent unsigned conditional selects for the minimum and chosen item. It retains out-of-line uniqueness and reserve-for-append calls on cold edges. The C and Rust optimized controls hoist whole-span capacity admission and emit a direct state update and store in the loop. They serve as assembly controls.

## Correctness and measured scope

- 720 native tests passed with the seven inherited exclusions. The compile profile includes AOT, JIT, canonical-native, canonical-native-program, canonical-native-program-runtime and canonical-native-runtime. The build keeps warnings as errors.
- The original and decoded complete Objects and metadata match the screened no-growth Object and metadata exactly. All 324 full smokes agree with the reference and independent inflate, preserve borrowed inputs/outputs and retire private allocations.
- Both forms passed the 40 existing path cases and seven focused capacity cases. A separate alias witness passed two cases per form. The focused fixture uses the general compact phase with the vector alternative disabled to isolate selection.
- Exact fit returns capacity 514 with one payload request. One more append retains the general path, grows to 771 with two requests and copies the 514 initialized words. Injected failure returns status 103426 at source origin 392, preserves earlier publication and aliases, cleans up, and recovers on retry. The independent assertion witness returns status 9 at source origin 234 and also recovers.
- The shared-owner witness first refuses the phase, leaves its old nodes intact, and re-enters the no-growth phase with a new unique owner.
- The test packet preserves original/decoded native controls for the unit predicate, total item and accumulator. Existing resident rows were reused where recorded. A fresh full linked/AOT campaign, full 6.6 MB input and hosted CI are outside this receipt.

## Exact-image CPU sampling

### qualification

| Input | Running samples | Numeric phase share | Root share |
|---|---:|---:|---:|
| eight-literals | 5340 | 15.39% | 15.67% |
| prefix-natural-1048576 | 5354 | 97.72% | 1.31% |
| prefix-natural-32768 | 5355 | 94.83% | 2.76% |
| prefix-periodic-1048576 | 5357 | 96.81% | 0.95% |
| prefix-periodic-32768 | 5363 | 89.65% | 1.44% |
| prefix-random-1048576 | 5353 | 98.58% | 0.22% |
| prefix-random-32768 | 5361 | 93.38% | 2.07% |
| prefix-repeated-32768 | 5625 | 90.42% | 1.55% |

### unit

| Input | Running samples | Numeric phase share | Root share |
|---|---:|---:|---:|
| prefix-repeated-32768 | 5366 | 90.76% | 1.47% |

### total-item

| Input | Running samples | Numeric phase share | Root share |
|---|---:|---:|---:|
| prefix-natural-1048576 | 5362 | 97.52% | 1.34% |

### segments-image

| Input | Running samples | Numeric phase share | Root share |
|---|---:|---:|---:|
| prefix-natural-1048576 | 5366 | 97.28% | 1.34% |

### isolated-threaded

| Input | Running samples | Numeric phase share | Root share |
|---|---:|---:|---:|
| prefix-random-1048576 | 5357 | 98.66% | 0.26% |

Profiles use exact linked images after the measurement barriers. Samples include the harness and give CPU-time shares. The separately measured hardware counters give whole-call totals.

## All 81 paired rows

| Case | Bytes | R264 / Roc | Candidate / Roc | Candidate / R264 | Pooled and block gates |
|---|---:|---:|---:|---:|---|
| empty | 0 | 2.087107× | 2.224103× | 1.065639× | pass / fail |
| one-stored | 1 | 2.001837× | 2.118854× | 1.058455× | pass / fail |
| seven-stored-prefix | 7 | 2.212090× | 2.203795× | 0.996250× | pass / pass |
| eight-literals | 8 | 0.415931× | 0.413540× | 0.994253× | pass / pass |
| eight-overlap | 8 | 0.415847× | 0.411416× | 0.989345× | pass / pass |
| 320-overlap | 320 | 0.452129× | 0.445721× | 0.985826× | pass / pass |
| eight-literals-prefix | 8 | 0.412836× | 0.416060× | 1.007809× | pass / pass |
| 128-biased-dynamic | 128 | 0.413185× | 0.411781× | 0.996602× | pass / pass |
| 32-high-entropy-stored | 32 | 0.421525× | 0.418227× | 0.992175× | pass / pass |
| 16384-distribution-change | 16384 | 1.863993× | 1.813426× | 0.972872× | pass / pass |
| 33027-periodic-window | 33027 | 1.070852× | 1.046940× | 0.977670× | pass / pass |
| random-64 | 64 | 0.451478× | 0.443172× | 0.981603× | pass / pass |
| random-256 | 256 | 0.483182× | 0.479541× | 0.992466× | pass / pass |
| random-1024 | 1024 | 0.550235× | 0.547383× | 0.994817× | pass / pass |
| repeated-1024 | 1024 | 0.528550× | 0.522516× | 0.988584× | pass / pass |
| random-32769 | 32769 | 1.810384× | 1.758787× | 0.971499× | pass / pass |
| repeated-32769 | 32769 | 1.029173× | 1.002923× | 0.974494× | pass / pass |
| reymont-prefix-1024 | 1024 | 0.682662× | 0.675166× | 0.989020× | pass / pass |
| reymont-prefix-16384 | 16384 | 1.751783× | 1.692010× | 0.965879× | pass / pass |
| reymont-prefix-32769 | 32769 | 1.818707× | 1.755956× | 0.965497× | pass / pass |
| heldout-window-32767 | 32767 | 1.818920× | 1.753591× | 0.964084× | pass / pass |
| heldout-window-32768 | 32768 | 1.819330× | 1.757886× | 0.966227× | pass / pass |
| heldout-slides-65537 | 65537 | 1.890163× | 1.826917× | 0.966540× | pass / pass |
| heldout-random-65537 | 65537 | 1.960770× | 1.926476× | 0.982510× | pass / pass |
| heldout-mixed-131073 | 131073 | 1.865525× | 1.805611× | 0.967884× | pass / pass |
| prefix-natural-1024 | 1024 | 0.673258× | 0.661745× | 0.982900× | pass / pass |
| prefix-natural-4096 | 4096 | 1.314139× | 1.266419× | 0.963687× | pass / pass |
| prefix-natural-8192 | 8192 | 1.571747× | 1.521897× | 0.968284× | pass / pass |
| prefix-natural-16384 | 16384 | 1.753542× | 1.692679× | 0.965292× | pass / pass |
| prefix-natural-32767 | 32767 | 1.821052× | 1.755381× | 0.963938× | pass / pass |
| prefix-natural-32768 | 32768 | 1.818751× | 1.752857× | 0.963770× | pass / pass |
| prefix-natural-32769 | 32769 | 1.818938× | 1.752241× | 0.963332× | pass / pass |
| prefix-natural-33027 | 33027 | 1.824386× | 1.758299× | 0.963776× | pass / pass |
| prefix-natural-65535 | 65535 | 1.864685× | 1.796184× | 0.963264× | pass / pass |
| prefix-natural-65536 | 65536 | 1.857383× | 1.787522× | 0.962387× | pass / pass |
| prefix-natural-65537 | 65537 | 1.890945× | 1.822783× | 0.963953× | pass / pass |
| prefix-natural-131072 | 131072 | 1.871290× | 1.799251× | 0.961503× | pass / pass |
| prefix-natural-262144 | 262144 | 1.884747× | 1.812233× | 0.961526× | pass / pass |
| prefix-natural-1048576 | 1048576 | 1.891641× | 1.818709× | 0.961445× | pass / pass |
| prefix-random-1024 | 1024 | 0.542978× | 0.531716× | 0.979260× | pass / pass |
| prefix-random-4096 | 4096 | 0.935175× | 0.904701× | 0.967414× | pass / pass |
| prefix-random-8192 | 8192 | 1.167802× | 1.139627× | 0.975874× | pass / pass |
| prefix-random-16384 | 16384 | 1.533213× | 1.496168× | 0.975838× | pass / pass |
| prefix-random-32767 | 32767 | 1.804008× | 1.766187× | 0.979035× | pass / pass |
| prefix-random-32768 | 32768 | 1.811232× | 1.771059× | 0.977820× | pass / pass |
| prefix-random-32769 | 32769 | 1.810100× | 1.775767× | 0.981032× | pass / pass |
| prefix-random-33027 | 33027 | 1.811942× | 1.773010× | 0.978514× | pass / pass |
| prefix-random-65535 | 65535 | 1.984073× | 1.945095× | 0.980355× | pass / pass |
| prefix-random-65536 | 65536 | 1.989311× | 1.953720× | 0.982109× | pass / pass |
| prefix-random-65537 | 65537 | 1.978141× | 1.936821× | 0.979111× | pass / pass |
| prefix-random-131072 | 131072 | 2.068751× | 2.033327× | 0.982877× | pass / pass |
| prefix-random-262144 | 262144 | 2.118888× | 2.088121× | 0.985480× | pass / pass |
| prefix-random-1048576 | 1048576 | 2.161670× | 2.131547× | 0.986065× | pass / pass |
| prefix-periodic-1024 | 1024 | 0.524855× | 0.514385× | 0.980052× | pass / pass |
| prefix-periodic-4096 | 4096 | 0.746744× | 0.736143× | 0.985805× | pass / pass |
| prefix-periodic-8192 | 8192 | 0.857506× | 0.831722× | 0.969932× | pass / pass |
| prefix-periodic-16384 | 16384 | 0.956498× | 0.929451× | 0.971723× | pass / pass |
| prefix-periodic-32767 | 32767 | 1.058957× | 1.034492× | 0.976897× | pass / pass |
| prefix-periodic-32768 | 32768 | 1.057656× | 1.032597× | 0.976307× | pass / pass |
| prefix-periodic-32769 | 32769 | 1.061326× | 1.039069× | 0.979029× | pass / pass |
| prefix-periodic-33027 | 33027 | 1.065759× | 1.042635× | 0.978302× | pass / pass |
| prefix-periodic-65535 | 65535 | 1.142926× | 1.115012× | 0.975577× | pass / pass |
| prefix-periodic-65536 | 65536 | 1.141825× | 1.111765× | 0.973673× | pass / pass |
| prefix-periodic-65537 | 65537 | 1.138499× | 1.111075× | 0.975912× | pass / pass |
| prefix-periodic-131072 | 131072 | 1.179228× | 1.150531× | 0.975665× | pass / pass |
| prefix-periodic-262144 | 262144 | 1.202935× | 1.177611× | 0.978948× | pass / pass |
| prefix-periodic-1048576 | 1048576 | 1.221963× | 1.194428× | 0.977467× | pass / pass |
| prefix-repeated-1024 | 1024 | 0.517988× | 0.511269× | 0.987029× | pass / pass |
| prefix-repeated-4096 | 4096 | 0.735116× | 0.709858× | 0.965640× | pass / pass |
| prefix-repeated-8192 | 8192 | 0.833543× | 0.817735× | 0.981035× | pass / pass |
| prefix-repeated-16384 | 16384 | 0.942267× | 0.926910× | 0.983702× | pass / pass |
| prefix-repeated-32767 | 32767 | 1.023453× | 1.001569× | 0.978618× | pass / pass |
| prefix-repeated-32768 | 32768 | 1.024335× | 1.004680× | 0.980812× | pass / pass |
| prefix-repeated-32769 | 32769 | 1.032578× | 1.002842× | 0.971202× | pass / pass |
| prefix-repeated-33027 | 33027 | 1.038092× | 1.015862× | 0.978586× | pass / pass |
| prefix-repeated-65535 | 65535 | 1.109691× | 1.079880× | 0.973135× | pass / pass |
| prefix-repeated-65536 | 65536 | 1.108107× | 1.079294× | 0.973998× | pass / pass |
| prefix-repeated-65537 | 65537 | 1.105322× | 1.080293× | 0.977356× | pass / pass |
| prefix-repeated-131072 | 131072 | 1.149269× | 1.125041× | 0.978918× | pass / pass |
| prefix-repeated-262144 | 262144 | 1.171312× | 1.146796× | 0.979070× | pass / pass |
| prefix-repeated-1048576 | 1048576 | 1.178492× | 1.152290× | 0.977766× | pass / pass |

[Verified source closure, raw paired observations, counters and profile receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r267-loop-state-fc9a8770f4b1.tar.gz). SHA-256 `fc9a8770f4b1bb80af8fa9d8d73810bf879d6012e7bfbbb6362c5a02c4372a49`.
