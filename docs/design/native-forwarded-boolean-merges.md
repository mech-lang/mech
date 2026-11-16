# R311 — Measure forwarding proved Boolean values at merges

**Keep R309 with the R272 runtime.** The 256 KiB natural target moves from 1.117704× to 1.120318× libdeflate in fresh matched blocks. Its paired elapsed change is +0.234% in those blocks and +0.198% in the separate timing screen. Both observations agree on direction. The 10-case screen has 10/10 pooled and 10/10 block gates passing. Its paired geometric mean changes +0.893%.

Measured parent 6120aba1263cab886cd1d66c3c3c8d610c590172. Published parent 333e51a4d9e46f4856186933f7bcc8a8d303fb1d. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The same level-12 source, bytecode, allocator, capacities, tables, and complete-call boundary are pinned.

## Native comparison evidence

Libdeflate carries the selected child address across tree descent and combines the two stop predicates with CMP and CCMP. The safe Rust control emits the same two comparisons. The retained Mech phase materializes a combined Boolean for both branch selection and the next visit. R310 changed the selection and retained that later Boolean use.

R311 applies the direct branches only when both original arms reach the same merge with exactly those two predecessors. The original arms still execute their stores. The common continuation receives the corresponding negated Boolean as a typed parameter and the original negation becomes that parameter. Another incoming edge, a different consumer or an unavailable merge proof keeps the original lowering.

The combined word and its negation are absent from both selected CLIF routes. One of 347 machine bodies changes. The vector matcher goes from 13,840 to 13,796 bytes and from 752 to 784 frame bytes. Link placement changes 223 of 227 addresses.

## Timings and separate counters

| Input | R309 / libdeflate | R311 / libdeflate | R309 / Roc | R311 / Roc |
|---|---:|---:|---:|---:|
| empty | 0.018501× | 0.016590× | 2.336314× | 2.095050× |
| one-stored | 0.018599× | 0.017746× | 2.100809× | 2.004377× |
| seven-stored-prefix | 0.018543× | 0.018461× | 2.238039× | 2.228077× |
| eight-literals | 2.369997× | 2.362901× | 0.184360× | 0.183808× |
| prefix-natural-32768 | 1.239309× | 1.237799× | 1.359149× | 1.357493× |
| prefix-natural-262144 | 1.117704× | 1.120318× | 1.387077× | 1.390321× |
| prefix-natural-1048576 | 1.136056× | 1.131993× | 1.403598× | 1.398578× |
| prefix-random-16384 | 2.955721× | 2.992405× | 1.102585× | 1.116269× |
| prefix-periodic-1048576 | 1.263074× | 1.313609× | 0.744773× | 0.774571× |
| prefix-repeated-1048576 | 1.161497× | 1.206837× | 0.739389× | 0.768252× |

The matched screen contains 154,176 timed calls across 10 cases. Each peer uses level 12 with runtime input, matching validation and the original complete-call boundary.

On 256 KiB natural the separate CPU cohort reports 805,643,376.0 to 800,794,869.5 retired instructions (-0.602%) and -0.032% cycles. The six payload counters agree across the 10-case memory cohort.

The candidate profile records 2,128 of 5,581 running samples in the vector matcher. The exact R309 target profile records 2,147 of 5,607. These independent samples include validation.

## Qualification and decision

The warnings-as-errors compiler build and 53 CFG tests pass. The tests cover all Boolean outcomes, alternate inputs to the false selector and merge, a false-arm side effect, an extra use and a raw non-Boolean value. The original image passes 162 complete linked calls across all 81 inputs with exact bytes, independent inflate, original capacities, held aliases and zero remaining private ownership.

The timing and profile screens are complete. The candidate stays experimental and did not enter the full timing matrix or decoded-image qualification. R309 remains the retained build, with 46/81 cases faster than Roc and three shorter cases faster than libdeflate in its complete matched run.

The next structural candidate should shorten lifetimes of immutable input packet words used only in matcher exits. The current linked descent reloads the U16 base before the child update. Establish which spills compete for the same registers and compare the recurring path with the libdeflate register-held child addresses.

[Verified patch, linked machine code and pilot observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r311-boolean-merges-23a28bbc60ed.tar.gz). SHA-256 23a28bbc60eda6afb7d3940b2919e0c6e0d1f0cdb7f69799e96aaea5ca0e4f10.
