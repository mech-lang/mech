# R319 — Reuse complementary native comparisons

**Keep R317 with the R272 runtime.** The 256 KiB natural target moves from 1.081977× to 1.083789× libdeflate in fresh matched blocks. Its paired elapsed change is +0.167% in those blocks and +0.156% in the separate timing screen. The 15-case screen has 15/15 pooled and 15/15 block gates passing. Its paired geometric mean changes +0.052%.

Measured parent 8903df3b33c310ba314500832aab10c9e09345bf. Published parent 54f86f2ab0aa5fdd5c82bf78dc248cb722ef8249. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The same level-12 source, bytecode, allocator, capacities, tables, and complete-call boundary are pinned.

## Native comparison evidence

A selected edge establishes an unsigned ordering for the exact current I64 operands. The complement may reuse that ordering in the dominated region. The rule normalizes swapped operands and retains comparisons using fresh loads, signed interpretations, or paths with an alternate entry.

The linked search and skip paths lose their repeated comparison and branch before the unsigned saturating subtraction. The original zero-versus-subtract decision still executes. The same pass proves one further comparison in the vector matcher. One of 347 bodies changes. Its code shrinks from 12,904 to 12,804 bytes and its stack frame grows from 608 to 624 bytes. Static stack reads fall from 570 to 559 and static branches fall from 295 to 292.

The exact linked Roc binary uses SUBS and CSEL for the saturating result. The linked libdeflate matcher instead checks a signed tree offset against its cutoff and adds the offset to its input base. Its two byte loads on the first mismatch route are repeated directly before the direction comparison.

## Timings and separate counters

| Input | R317 / libdeflate | R319 / libdeflate | R317 / Roc | R319 / Roc |
|---|---:|---:|---:|---:|
| empty | 0.016348× | 0.016568× | 2.076374× | 2.104339× |
| one-stored | 0.017666× | 0.017774× | 2.015721× | 2.028012× |
| seven-stored-prefix | 0.018338× | 0.018507× | 2.227701× | 2.248253× |
| eight-literals | 2.311239× | 2.324163× | 0.181189× | 0.182202× |
| reymont-prefix-32769 | 1.199887× | 1.200262× | 1.322197× | 1.322610× |
| heldout-window-32768 | 1.201146× | 1.200512× | 1.319546× | 1.318849× |
| prefix-natural-32767 | 1.198257× | 1.198517× | 1.316147× | 1.316433× |
| prefix-natural-32768 | 1.201528× | 1.199333× | 1.319764× | 1.317353× |
| prefix-natural-65537 | 1.119934× | 1.122056× | 1.329417× | 1.331936× |
| prefix-natural-131072 | 1.106781× | 1.101826× | 1.340626× | 1.334624× |
| prefix-natural-262144 | 1.081977× | 1.083789× | 1.341249× | 1.343495× |
| prefix-natural-1048576 | 1.086463× | 1.087645× | 1.344997× | 1.346460× |
| prefix-random-16384 | 2.903720× | 2.902477× | 1.075160× | 1.074700× |
| prefix-periodic-1048576 | 1.222749× | 1.239981× | 0.720306× | 0.730457× |
| prefix-repeated-1048576 | 1.131959× | 1.146373× | 0.721185× | 0.730368× |

The matched screen contains 154,656 timed calls across 15 cases. Each peer uses level 12 with runtime input, matching validation and the original complete-call boundary.

On 256 KiB natural the separate CPU cohort reports 773,529,424.0 to 764,793,168.5 retired instructions (-1.129%) and -0.037% cycles. The six payload counters agree across the 15-case memory cohort.

The candidate profile records 1,951 of 5,344 running samples in the matcher. The separate R317 profile records 1,940 of 5,360. Both runs include validation.

## Qualification and decision

The warnings-as-errors build and all 55 CFG tests pass. The focused test executes 1,200 before/after pairs and checks signed, swapped, alternate-entry, fresh-load, snapshot, repeated-visit and reached-store cases. The original image passes 162 linked calls across 81 inputs with exact bytes, independent inflate, capacities, held aliases and final managed cleanup.

The first test link hit disk error 28. Reclaiming 392,433,760 bytes of superseded build products allowed the same warnings-as-errors test build to finish.

The target and both large repetition controls slow in both elapsed cohorts. R317 stays in use. The 81-case timing and decoded qualification campaigns apply to any later adoption.

R313 already shares the dominating window checks. R314 proves the initial scalar reads. R315 bypasses dispatch on the initial mismatch, then still executes the directional reads. The next selected-edge experiment will test reusing the initial byte pair with current address, range and memory-effect proofs.

[Verified patch, linked machine code and pilot observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r319-current-complements-7695bd693477.tar.gz). SHA-256 7695bd69347799bcf2d81023d5cd94db9ba5e4d6f0e92f7168d1d0c481fed034.
