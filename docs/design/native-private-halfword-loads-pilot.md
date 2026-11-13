# Coalesce adjacent private halfword reads

One fixed-address diagnostic reads a pair of adjacent U16 cells as a single packed word. It takes **0.995742×** the paired control across all 81 cases. Both 1.05 gates pass. The matched control and candidate each beat Roc on 53/81 and libdeflate on 25/81. Wins against both rise from nine to ten and wins against either fall from 69/81 to 68/81. The published compiler scoreboard remains unchanged while normal emission is qualified.

| Full-cohort input | Control → trial / Roc | Control → trial / libdeflate | Trial / control |
| --- | ---: | ---: | ---: |
| Eight literals | 0.172361× → 0.171188× | 2.189820× → 2.174923× | 0.993197× |
| Natural 8 KiB | 0.988949× → 0.990008× | 1.230293× → 1.231611× | 1.001071× |
| Held-out window, 32,767 bytes | 1.098778× → 1.102388× | 0.999808× → 1.003093× | 1.003286× |
| Natural 33,027 bytes | 1.100685× → 1.102028× | 0.998204× → 0.999422× | 1.001220× |
| Held-out mixed, 131,073 bytes | 1.014679× → 1.010450× | 1.177262× → 1.172356× | 0.995832× |
| Held-out random, 65,537 bytes | 1.018508× → 1.011915× | 1.727325× → 1.716143× | 0.993526× |
| Random 1 MiB | 1.150800× → 1.143936× | 1.623710× → 1.614026× | 0.994036× |
| Periodic 256 KiB | 0.566170× → 0.565633× | 1.000929× → 0.999981× | 0.999053× |

The same held-out random improvement is 0.90% in the screen and 0.52% in confirmation. Its two order halves improve in all three runs. Natural 8 KiB slows 0.11% in the full run and 0.25% in the adverse retest but stays below Roc in both. Held-out window 32,767 bytes loses a close libdeflate win in the screen and full run; it remains above 1× in confirmation. Periodic 256 KiB is above parity in the screen and below it in the full and adverse cohorts; both arms beat libdeflate in the adverse cohort. The full estimator gives −3.51% to a stored case that never calls the changed matcher, so that row cannot be credited to this optimization. The adverse cohort takes 1.002558× control, with a 1.47% slowdown on periodic 8 KiB and no win-set changes. The screen and confirmation take 0.999432× and 0.997581×.

The retained ARM64 matcher builds the second 16-bit address independently and issues two halfword reads. Roc and libdeflate reuse the first bucket's address calculation. The trial checks the existing table floor, exact common base, and bounded adjacent selectors. Both selected halfwords are initialized inside the same exclusively held vector; there are no intervening effects. It uses one four-byte little-endian read and extracts the two original values. The original halfword stores, lexical checks, failure paths and sequence of visible effects stay in place. Only the 10,172-byte vector matcher changes, to 10,168 bytes with the same 576-byte frame and linked addresses. The separate counters show held-out random at **0.993790× instructions** and **0.993262× cycles**; random 1 MiB at 0.993936× and 0.995687×. The exact candidate places 3,276/4,914 sampled timer PCs in the matcher on held-out random, including validation in the denominator.

| Independent 18-case trial | Paired elapsed | Roc wins, control → trial | libdeflate wins, control → trial | Disposition |
| --- | ---: | ---: | ---: | --- |
| Packed load and stores | 1.002634× | 4 → 4 | 10 → 9 | Reject; random 1 MiB +1.64% |
| Proved current prefix, two words changed | 0.999613× | 4 → 4 | 7 → 7 | Keep as evidence; held-out window and natural 32 KiB cross below 1×, two neighboring natural sizes cross above |
| Reuse the short-record decision | 0.999971× | 4 → 4 | 12 → 9 | Reject; fewer wins |
| Packed load plus short-record decision | 1.003964× | 4 → 3 | 9 → 4 | Reject; frame grows to 592 bytes |
| Narrow hash multiply and shift, fixed layout | 1.000076× | 4 → 4 | 10 → 10 | Keep as evidence; no net wins |
| Narrow hash multiply and shift over packed load | 1.000168× | 4 → 4 | 8 → 10 | Keep as evidence; mixed frontier and no aggregate improvement |

The two prefix-check eliminations together increase code size to 10,240 bytes and static stack references from 437 to 490, so that version stops before timing. The two-word current-prefix edit reuses only the same path's checked readable extent and preserves the original memory address. The short-record candidate routes the cursor increment through the already selected, successful cache write; the store bounds check and its failure stay intact. The hash control replaces each 64-bit multiply, low-word mask and shift with a 32-bit multiply, 32-bit shift and no-op at fixed addresses. Plain CLIF narrowing instead replaces the mask with a move, so it does not remove an instruction. Unlike the earlier broad selector, this comparison keeps other bodies and addresses fixed. The separate combined-store counter row retires almost the same instructions as load-only on held-out random, but takes 1.009070× control cycles; this does not isolate an individual store from register allocation or scheduling.

The seven variants pass 1,134 linked checks, independent inflation, reference bytes, capacity, held aliases, and cleanup. Their 313,920 timed calls use identical level-12 source and bytecode, runtime inputs, capacities, allocator, startup descriptor, complete-call timing boundaries, and both peers. The pack includes 24 independent hardware-counter case comparisons and 20 exact-worker profiles. Full timing is limited to the load-only trial; the other variants stopped after their screens.

Measured compiler `6e7dd1bed4614034887d11b574251fa06cc0dff5`; source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode `51de51658c1d8d0be66567400e382ef5c0f5789baf317c53a8cd56d1df2dd0ca`. Packed-load Object `0b99f66a725e54f26eeb923f9ed60c0504c6da880698e8b9cc9d8d294828cc5e`; timed worker `a7505fd9358b912a5a7ce4b0ff0e53e03e6dacd6ed8514c481a8a2b5a44f7bbf`.

[All timing cohorts, machine code, counters, profiles and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-6e7dd1bed4-private-halfword-loads.tar.gz). SHA-256 `4c6ac235a3ae5b5ce97bbd31cafbd3fcd202c33537e44ec8f1eb7520476a44a1`; all 587 files verified after downloading.
