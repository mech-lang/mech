# R322 — Select unsigned saturating subtractions

**Keep R320 with the R272 runtime.** The 256 KiB natural target moves from 1.077036× to 1.081967× libdeflate in fresh matched blocks. Its paired elapsed change is +0.458% in those blocks and +0.497% in the separate timing screen. The 15-case screen has 15/15 pooled and 15/15 block gates passing. Its paired geometric mean changes -0.081%.

Measured parent ecd961c4f707cb61f23dd4cb5cc2be99c82b92ea. Published parent f23d907aaa4a62b94440a2384c23c7a5dafeba6a. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The same level-12 source, bytecode, allocator, capacities, tables, and complete-call boundary are pinned.

## Native comparison evidence

The compiler recognizes the two exclusive arms of an unsigned zero-or-difference expression. Both arms must reach the same continuation with the same other arguments. A nested check is followed only when the original edge proves the exact current unsigned operands. Reached effects, alternate entries, changed operands and signed comparisons keep their original paths.

The linked search and skip subtraction paths change from six and eight instructions to four each when the difference is nonnegative. The zero routes previously took three. Both new paths use a separate subtraction, comparison and zero operand. The pinned reference uses SUBS and CSEL with the zero register. The linked C matcher adds its signed node offset to the input base.

Only the matcher changes among 347 bodies. It shrinks from 12,784 to 12,720 bytes and uses the same 624-byte frame. Static stack reads change from 541 to 553.

## Timings and separate counters

| Input | R320 / libdeflate | R322 / libdeflate | R320 / Roc | R322 / Roc |
|---|---:|---:|---:|---:|
| empty | 0.016769× | 0.016634× | 2.118733× | 2.101678× |
| one-stored | 0.017839× | 0.017920× | 2.028457× | 2.037717× |
| seven-stored-prefix | 0.018633× | 0.018505× | 2.259068× | 2.243602× |
| eight-literals | 2.319459× | 2.313744× | 0.182919× | 0.182468× |
| reymont-prefix-32769 | 1.196507× | 1.197061× | 1.312950× | 1.313558× |
| heldout-window-32768 | 1.196715× | 1.198331× | 1.313961× | 1.315735× |
| prefix-natural-32767 | 1.198017× | 1.197913× | 1.318411× | 1.318297× |
| prefix-natural-32768 | 1.198092× | 1.198822× | 1.315132× | 1.315934× |
| prefix-natural-65537 | 1.116279× | 1.120465× | 1.326275× | 1.331250× |
| prefix-natural-131072 | 1.093920× | 1.100685× | 1.327526× | 1.335735× |
| prefix-natural-262144 | 1.077036× | 1.081967× | 1.334197× | 1.340305× |
| prefix-natural-1048576 | 1.079410× | 1.084695× | 1.336610× | 1.343155× |
| prefix-random-16384 | 2.922281× | 2.920626× | 1.070852× | 1.070246× |
| prefix-periodic-1048576 | 1.203329× | 1.185992× | 0.708205× | 0.698001× |
| prefix-repeated-1048576 | 1.116961× | 1.101775× | 0.710939× | 0.701274× |

The matched screen contains 154,656 timed calls across 15 cases. Each peer uses level 12 with runtime input, matching validation and the original complete-call boundary.

On 256 KiB natural the separate CPU cohort reports 759,430,179.5 to 759,332,455.0 retired instructions (-0.013%) and +0.677% cycles. The six payload counters agree across the 15-case memory cohort.

The candidate profile records 1,962 of 5,361 running samples in the matcher. The separate R320 profile records 1,915 of 5,359. Both runs include validation.

## Qualification and decision

The warnings-as-errors build and 56 CFG tests pass. The focused test executes 1,200 before/after pairs across swapped and complementary conditions, unsigned limits, signed comparisons, different checked operands, stores, alternate entries and changed live arguments. The original image passes 162 linked calls with exact streams, independent inflate, capacities, held aliases and final managed cleanup.

R320 remains the retained compiler after the timing screen. The full 81-case timing and decoded qualification campaigns apply to any later adoption.

The next ARM64 test will combine the exact subtraction and comparison into SUBS and supply the zero arm directly to CSEL. This removes the temporary zero and keeps the arithmetic result and borrow from the same instruction.

[Verified patch, linked machine code and pilot observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r322-saturating-subtraction-a797273773e6.tar.gz). SHA-256 a797273773e649d9383cc60216ef596e17cf44eeca0fcd63d808dd60714eb65f.
