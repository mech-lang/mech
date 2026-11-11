# R318 — Select ordered results in the native matcher

**Keep R317 with the R272 runtime.** The 256 KiB natural target moves from 1.083509× to 1.086378× libdeflate in fresh matched blocks. Its paired elapsed change is +0.265% in those blocks and +0.060% in the separate timing screen. The 15-case screen has 15/15 pooled and 15/15 block gates passing. Its paired geometric mean changes -0.170%.

Measured parent 8903df3b33c310ba314500832aab10c9e09345bf. Published parent 8903df3b33c310ba314500832aab10c9e09345bf. PR base 92f75db84e2523693648084bea16cd52ad60777a. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The same level-12 source, bytecode, allocator, capacities, tables, and complete-call boundary are pinned.

## Native comparison evidence

Both reference executables select the next match length using a conditional select. R318 replaces a current unsigned comparison and its two empty jump arms with `umin` or `umax` when the sole merged result is exactly one of those compared operands. An alternate entry, side effect, other operand or signed comparison keeps its original lowering.

The four matcher descent sites now use CMP and CSEL. There are 15 substitutions in four machine bodies, with six in the matcher. The matcher shrinks from 12,904 to 12,720 bytes and its frame grows from 608 to 624 bytes. Its static stack-load count drops from 570 to 532. Two of the four routes lose a second tree-base reload between their store and child lookup. Each route still reloads the table base before its store.

## Timings and separate counters

| Input | R317 / libdeflate | R318 / libdeflate | R317 / Roc | R318 / Roc |
|---|---:|---:|---:|---:|
| empty | 0.016789× | 0.017288× | 2.029357× | 2.089640× |
| one-stored | 0.017594× | 0.017988× | 2.003734× | 2.048681× |
| seven-stored-prefix | 0.018266× | 0.018496× | 2.209888× | 2.237742× |
| eight-literals | 2.321267× | 2.312502× | 0.182842× | 0.182152× |
| reymont-prefix-32769 | 1.200479× | 1.201775× | 1.323809× | 1.325238× |
| heldout-window-32768 | 1.198953× | 1.200891× | 1.321403× | 1.323539× |
| prefix-natural-32767 | 1.198364× | 1.201040× | 1.318174× | 1.321118× |
| prefix-natural-32768 | 1.201199× | 1.201555× | 1.320823× | 1.321214× |
| prefix-natural-65537 | 1.120261× | 1.124299× | 1.325986× | 1.330765× |
| prefix-natural-131072 | 1.098945× | 1.102475× | 1.332072× | 1.336351× |
| prefix-natural-262144 | 1.083509× | 1.086378× | 1.339336× | 1.342883× |
| prefix-natural-1048576 | 1.087341× | 1.088765× | 1.347187× | 1.348951× |
| prefix-random-16384 | 2.926934× | 2.895095× | 1.136194× | 1.123835× |
| prefix-periodic-1048576 | 1.223685× | 1.206292× | 0.721682× | 0.711425× |
| prefix-repeated-1048576 | 1.133717× | 1.116952× | 0.718843× | 0.708213× |

The matched screen contains 154,656 timed calls across 15 cases. Each peer uses level 12 with runtime input, matching validation and the original complete-call boundary.

On 256 KiB natural the separate CPU cohort reports 773,578,398.5 to 770,007,514.0 retired instructions (-0.462%) and +0.019% cycles. The six payload counters agree across the 15-case memory cohort.

The candidate profile records 1,937 of 5,348 running samples in the matcher. The separate R317 profile records 1,940 of 5,360. Both runs include validation.

## Qualification and decision

The warnings-as-errors build and 55 CFG tests pass. The focused test executes 504 before/after pairs, including signed, alternate-entry and reached-store controls. The original image passes 162 linked calls across 81 inputs with exact bytes, independent inflate, capacities, held aliases and final managed cleanup.

The primary screen improves its case-weighted aggregate, including 1 MiB periodic and repeated. Both matched natural-text targets slow slightly, so R317 stays in use and R318 stays available as an independently measured alternative. The full 81-case campaign and decoded-image qualification apply to any later adoption.

The next candidate will compare the complementary current conditions preceding the matcher’s saturating subtraction and test whether both can use the same established fact.

[Verified patch, linked machine code and pilot observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r318-ordered-choices-c6997e54c284.tar.gz). SHA-256 c6997e54c284d2f976334b8bf59523cc7453b49d0de7b4e4ce31f56138bca9be.
