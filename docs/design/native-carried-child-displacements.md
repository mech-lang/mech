# R328 — Carry scaled child displacements

**Keep R327 with the R272 runtime.** The 256 KiB natural case moves from 1.052317× to 1.062575× libdeflate. The candidate changes elapsed +0.975% in the matched comparison and +0.521% in the separate timing screen. The paired geometric mean changes +0.236% across 15 cases. The pooled and block gates pass 15/15 and 15/15.

Measured parent ba8f02f61b3735a2160cc6177e76c6f7cc59cf7b. Published parent ba8f02f61b3735a2160cc6177e76c6f7cc59cf7b. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, IR/work/ABI/bytecode 215/107/39/14. The original source, bytecode, capacities, allocator, search limits, tables and complete-call boundary are pinned.

## Carry the scaled index with a separate base

The compiler changes a closed class of loop arguments from logical slots to byte displacements when the class has only compatible typed memory uses. The current base and element width must agree, with one equivalent scaled value on every incoming edge. Each selected access uses the current base plus the carried displacement and keeps its original immediate offset, flags, width and order. Logical users, a different base or a shared address expression keep the original class.

Four child lookups supply their already-scaled CLIF values to the next tree visit. The two linked search paths take one and two address arithmetic instructions, down from three each. The search stores still reload the table base. The new skip paths include more stack loads. The pinned C executable carries pending addresses. The pinned Roc executable keeps the table base in a register and scales the logical index in its halfword access.

Only the matcher changes among 347 bodies. Its code is 12,272 to 12,288 bytes and its frame grows from 624 to 640 bytes. Static stack reads move from 543 to 567 and writes from 115 to 121. Disabling the pass reproduces the retained Object and metadata byte for byte.

## Matched results

| Input | R327 / Roc | R328 / Roc | R327 / libdeflate | R328 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| empty | 2.094510× | 2.124979× | 0.016534× | 0.016774× | +0.000% |
| one-stored | 2.027540× | 2.032973× | 0.017807× | 0.017855× | +0.000% |
| seven-stored-prefix | 2.221607× | 2.230389× | 0.018386× | 0.018458× | +0.000% |
| eight-literals | 0.182465× | 0.181868× | 2.329658× | 2.322036× | +0.608% |
| reymont-prefix-32769 | 1.285222× | 1.296020× | 1.170848× | 1.180686× | +0.619% |
| heldout-window-32768 | 1.288057× | 1.296890× | 1.168847× | 1.176862× | +0.560% |
| prefix-natural-32767 | 1.285592× | 1.292902× | 1.171914× | 1.178578× | +0.485% |
| prefix-natural-32768 | 1.284558× | 1.292951× | 1.173669× | 1.181337× | +0.335% |
| prefix-natural-65537 | 1.297192× | 1.303441× | 1.092770× | 1.098035× | +0.335% |
| prefix-natural-131072 | 1.303279× | 1.308410× | 1.073225× | 1.077450× | +0.463% |
| prefix-natural-262144 | 1.307377× | 1.320121× | 1.052317× | 1.062575× | +0.521% |
| prefix-natural-1048576 | 1.308323× | 1.315422× | 1.052309× | 1.058019× | +0.443% |
| prefix-random-16384 | 1.063604× | 1.062965× | 2.889293× | 2.887559× | -0.658% |
| prefix-periodic-1048576 | 0.709662× | 0.711456× | 1.204525× | 1.207571× | +0.388% |
| prefix-repeated-1048576 | 0.711394× | 0.708813× | 1.117447× | 1.113392× | -0.544% |

The comparison contains 154,656 timed calls. It uses balanced level-12 runtime input, matching validation and fresh C compressor allocation.

## Independent counters and qualification

On 256 KiB natural the separate CPU cohort reports 745,152,321.0 to 744,566,743.5 retired instructions (-0.079%) and +0.423% cycles. The six payload counters agree across all 15 memory cases.

The exact linked-PC profile records 1,940 of 5,395 running timer samples in the candidate matcher. The separate retained profile records 1,912 of 5,353. Samples at stack-load PCs rise from 26.26% to 44.23% of matcher samples. These separate profiles include validation.

The warnings-as-errors build and 60 CFG tests pass. The focused witness checks 240 before/after JIT pairs with a numerical use, a different base, a shared expression, missing seeds, a changing base, wrapping indices, an immediate access displacement and an affine incoming slot. The original image passes 162 linked calls with exact streams, independent inflation, capacities, held aliases and final managed cleanup.

Both timing comparisons slow Reymont, 256 KiB natural and 1 MiB natural. The shorter address sequences do not lower elapsed time on those cases. Keep R327.

A focused untimed probe in the retained backend shows why the original store expands. A multiply by two reaches a separate doubling instruction before STRH. An explicit unit shift selects a scaled STRH with the same width and effective address. The pinned reference and an optimized safe Rust witness also put the shift in the store. This provides a narrower next experiment that keeps the logical loop arguments and changes address selection alone.

The candidate stopped at the 15-case timing screen with original-image qualification. Full timing and decoded-image qualification remain acceptance steps for any adoption.

[Verified patch, linked machine code and pilot observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r328-carried-child-displacements-209305838183.tar.gz). SHA-256 209305838183525640f4438e842afbefac0c61463e9ab07582bc097887d6d882.
