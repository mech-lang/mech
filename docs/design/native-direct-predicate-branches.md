# R310 — Measure direct predicate branches in tree descent

**Keep R309 with the R272 runtime.** The 256 KiB natural target moves from 1.125883× to 1.142173× libdeflate in fresh matched blocks. Its paired elapsed change is +1.447% in those blocks and +1.421% in the separate timing screen. Both observations agree on direction. The 29-case screen has 29/29 pooled and 29/29 block gates passing. Its paired geometric mean changes -0.247%.

Measured and published parent 6120aba1263cab886cd1d66c3c3c8d610c590172. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The same level-12 source, bytecode, allocator, capacities, tables, and complete-call boundary are pinned.

## Native comparison evidence

Libdeflate carries the selected child address across the tree descent and combines the two stop predicates with CMP and CCMP. The safe Rust control also emits CMP and CCMP. The retained Mech phase builds a word from two comparison Booleans, selects its one and zero arms, and complements the word for the next visit. This experiment routes the first and second comparisons directly to the original arm. It requires normalized scalar comparison results, a distinct single-predecessor zero selector for the same word, and effect-free instructions in that selector.

The original and candidate CLIF show that the combined word still reaches a complement after the arms rejoin. Its later use remains live. Two of 347 machine bodies change. The vector matcher goes from 13,840 to 13,832 bytes with the same 752-byte frame. Link placement changes 224 of 227 addresses. These measurements establish the complete-call effect of this candidate.

## Timings and separate counters

| Input | R309 / libdeflate | R310 / libdeflate | R309 / Roc | R310 / Roc |
|---|---:|---:|---:|---:|
| empty | 0.016583× | 0.016572× | 2.099498× | 2.098167× |
| one-stored | 0.017684× | 0.017880× | 2.016841× | 2.039159× |
| seven-stored-prefix | 0.018357× | 0.018534× | 2.238080× | 2.259718× |
| eight-literals | 2.323980× | 2.319927× | 0.182114× | 0.181796× |
| prefix-natural-4096 | 1.685006× | 1.704977× | 0.981441× | 0.993073× |
| prefix-natural-262144 | 1.125883× | 1.142173× | 1.389281× | 1.409383× |
| prefix-natural-1048576 | 1.131826× | 1.149609× | 1.402689× | 1.424727× |
| prefix-random-16384 | 3.000620× | 2.997738× | 1.101123× | 1.100065× |
| prefix-random-1048576 | 2.368369× | 2.403315× | 1.677637× | 1.702391× |
| prefix-periodic-1048576 | 1.255069× | 1.245965× | 0.739416× | 0.734052× |
| prefix-repeated-4096 | 1.455902× | 1.444838× | 0.388801× | 0.385846× |
| prefix-repeated-1048576 | 1.162314× | 1.155639× | 0.738409× | 0.734169× |

The matched screen contains 154,368 timed calls across 12 cases. Each peer uses level 12 with runtime input, matching validation and the original complete-call boundary.

On 256 KiB natural the separate CPU cohort reports 805,860,883.5 to 806,583,888.5 retired instructions (+0.090%) and +1.531% cycles. The six payload counters agree across the 12-case memory cohort.

The candidate profile records 2,190 of 5,590 running samples in the vector matcher on 256 KiB natural. The retained exact-image profile records 2,147 of 5,607. The separate candidate eight-literal profile records 661 of 5,248. These sample counts include validation and do not partition retired instructions.

## Qualification and decision

The warnings-as-errors compiler build and 53 CFG tests pass. The tests cover all Boolean outcomes, alternate inputs to the false selector, a false-arm side effect and a raw non-Boolean value. The original image passes 162 complete linked calls across all 81 inputs with exact bytes, independent inflate, original capacities, held aliases and zero remaining private ownership.

The timing and profile screens are complete. The candidate stays experimental and did not enter the full timing matrix or decoded-image qualification. R309 remains the retained build, with 46/81 cases faster than Roc and three shorter cases faster than libdeflate in its complete matched run.

The next experiment can carry a proved constant along each original arm into the common continuation. The negation can then use that selected value without recomputing the combined word. Require both arms to reach the same join with no third predecessor and preserve the reached stores.

[Verified patch, linked machine code and pilot observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r310-direct-branches-7b0cfef042be.tar.gz). SHA-256 7b0cfef042bee6be90b36ab7ed43a5d10a4fef84e48e7faf50f1f27ec20a9e1a.
