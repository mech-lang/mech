# R321 — Simplify comparison words on selected edges

**Keep R320 with the R272 runtime.** The 256 KiB natural target moves from 1.078843× to 1.084064× libdeflate in fresh matched blocks. Its paired elapsed change is +0.484% in those blocks and +0.662% in the separate timing screen. The 15-case screen has 15/15 pooled and 15/15 block gates passing. Its paired geometric mean changes +0.453%.

Measured parent ecd961c4f707cb61f23dd4cb5cc2be99c82b92ea. Published parent ecd961c4f707cb61f23dd4cb5cc2be99c82b92ea. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The same level-12 source, bytecode, allocator, capacities, tables, and complete-call boundary are pinned.

## Native comparison evidence

An AND of two scalar comparison results can use the other result on a dominating true edge or zero on a dominating false edge. The rule follows exact comparison SSA identity, including an unsigned extension and equality to one. The alternate-entry, fresh-read and raw-word controls keep their original values and effects.

Only the matcher changes among 347 bodies. Both selected equality decisions go from CMP, CSET, a save, CMP and a branch to CMP and a branch. The linked peers also enter match extension with a direct branch and later compare the resulting length to the current best.

The body grows from 12,784 to 12,952 bytes, and its frame stays at 624 bytes. Whole-body static stack reads change from 541 to 583 and stack writes from 114 to 116. The resulting register allocation also reloads the input base before the second initial byte read on both paths. These are static observations of the new linked code.

## Timings and separate counters

| Input | R320 / libdeflate | R321 / libdeflate | R320 / Roc | R321 / Roc |
|---|---:|---:|---:|---:|
| empty | 0.016463× | 0.016496× | 2.147784× | 2.152199× |
| one-stored | 0.017656× | 0.017768× | 1.996036× | 2.008665× |
| seven-stored-prefix | 0.018466× | 0.018546× | 2.252812× | 2.262545× |
| eight-literals | 2.325009× | 2.337216× | 0.183160× | 0.184122× |
| reymont-prefix-32769 | 1.198664× | 1.196998× | 1.325123× | 1.323281× |
| heldout-window-32768 | 1.199277× | 1.207957× | 1.314717× | 1.324232× |
| prefix-natural-32767 | 1.187902× | 1.237200× | 1.254152× | 1.306199× |
| prefix-natural-32768 | 1.196835× | 1.205383× | 1.315347× | 1.324741× |
| prefix-natural-65537 | 1.117035× | 1.120962× | 1.324473× | 1.329129× |
| prefix-natural-131072 | 1.092908× | 1.099650× | 1.327480× | 1.335669× |
| prefix-natural-262144 | 1.078843× | 1.084064× | 1.335759× | 1.342223× |
| prefix-natural-1048576 | 1.080296× | 1.088568× | 1.339418× | 1.349674× |
| prefix-random-16384 | 2.941446× | 2.950130× | 1.087520× | 1.090731× |
| prefix-periodic-1048576 | 1.214379× | 1.221715× | 0.715290× | 0.719611× |
| prefix-repeated-1048576 | 1.119078× | 1.135726× | 0.710821× | 0.721396× |

The matched screen contains 154,656 timed calls across 15 cases. Each peer uses level 12 with runtime input, matching validation and the original complete-call boundary.

On 256 KiB natural the separate CPU cohort reports 759,431,602.5 to 768,632,364.5 retired instructions (+1.212%) and +0.898% cycles. The six payload counters agree across the 15-case memory cohort.

The candidate profile records 1,942 of 5,331 running samples in the matcher. The separate R320 profile records 1,915 of 5,359. Both runs include validation.

## Qualification and decision

The warnings-as-errors build and 56 CFG tests pass. The focused test executes 320 before/after pairs across both selected outcomes, changing loop visits, fresh loads, stores, alternate entries and raw operands. The original image passes 162 linked calls with exact streams, independent inflate, capacities, held aliases and final managed cleanup.

The first qualification script retained a stale baseline path. The test link also hit disk error 28, and the test fixture initially returned a value from the wrong block. The corrected paths and fixture passed after reclaiming 815,743,704 bytes of old builds. The measured linked candidate has the original successful Object and metadata hashes.

The 256 KiB and 1 MiB natural targets slow in both elapsed comparisons. R320 stays in use. The full 81-case timing and decoded qualification campaigns apply to any later adoption.

The next independent candidate uses a single selection for the proved unsigned saturating subtraction. It follows both effect-free arms to their common result and checks the exact current operands on the checked route before bypassing it.

[Verified patch, linked machine code and pilot observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r321-selected-boolean-72219c85bf5a.tar.gz). SHA-256 72219c85bf5aab357a9a3227b67b04b48e7a40f7d8ec3267755d58e7bb42b29d.
