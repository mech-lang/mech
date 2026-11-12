# R336 — Reuse checked spans for packed prefix loads

**Retain R333 with the R272 runtime.** The compiler experiment stops at the 16-case screen. Natural 256 KiB changes +0.426% in the matched comparison and +0.599% in the primary screen. Reymont changes -0.022% and +0.432% in those comparisons.

Measured compiler ebc837def6d5c878cb3ecdeadd0e3f078a3f12e4. Published parent 993e0c35b9706464244817844cac711ef0994f44. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R333 closure and the archived compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, limits, tables, capacities and complete-call boundary are pinned.

## Reuse the current checked input extent

The proof matches an unsigned comparison against a zero-saturated length minus a positive constant. It requires the same current length, a selected nonwrapping sum of the current base and an extent, and an extent floor that covers the packed read and its constant offset. The comparison becomes true at the original lexical position. Unproved candidates, short extents and other views keep their current bounds and fallback. No read moves and the original table-size, overflow and readable-window rejection order stays intact.

The linked search path already proves at least five bytes fit. The four-byte reads at current position and position plus one each lose a comparison and branch. Both older-node prefix guards remain. The same four U16 hash-table accesses share a base reload, which changes four to one in that span. The original and decoded body is 12,272 to 12,168 bytes with the same 624-byte frame. The other 350 bodies match. Disabling the flag restores the original Object and metadata.

The R335 retained profile locates the current-position prefix guard at 523 of 5,362 random 1 MiB running samples. The pinned Roc code also checks the current prefix in its enclosing matcher. Libdeflate’s corresponding three-byte hash check uses admitted direct packed reads. Different profile scopes and data paths keep those samples separate from the elapsed and hardware counters.

## Matched measurements

| Input | R333 / Roc | R336 / Roc | R333 / libdeflate | R336 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.167142× | 2.102044× | 0.017075× | 0.016563× | -3.004% | +0.000% |
| one-stored | 1.990446× | 1.972508× | 0.017887× | 0.017726× | -0.901% | +0.000% |
| seven-stored-prefix | 2.250458× | 2.239880× | 0.018720× | 0.018632× | -0.470% | +0.000% |
| eight-literals | 0.183629× | 0.183319× | 2.327560× | 2.323632× | -0.169% | +0.152% |
| reymont-prefix-32769 | 1.265318× | 1.265044× | 1.150956× | 1.150707× | -0.022% | +0.432% |
| heldout-window-32768 | 1.260172× | 1.262217× | 1.148716× | 1.150581× | +0.162% | +0.687% |
| prefix-natural-32767 | 1.259455× | 1.259932× | 1.148336× | 1.148771× | +0.038% | +0.429% |
| prefix-natural-32768 | 1.259040× | 1.264390× | 1.149206× | 1.154089× | +0.425% | +0.268% |
| prefix-natural-65537 | 1.264630× | 1.274480× | 1.065904× | 1.074207× | +0.779% | +0.702% |
| prefix-natural-131072 | 1.269017× | 1.278033× | 1.044830× | 1.052254× | +0.710% | +0.425% |
| prefix-natural-262144 | 1.279894× | 1.285346× | 1.032630× | 1.037029× | +0.426% | +0.599% |
| prefix-natural-1048576 | 1.282341× | 1.292340× | 1.036067× | 1.044145× | +0.780% | +0.905% |
| prefix-random-16384 | 1.061730× | 1.051821× | 2.849829× | 2.823233× | -0.933% | -1.828% |
| prefix-random-1048576 | 1.587998× | 1.578889× | 2.249129× | 2.236227× | -0.574% | -0.470% |
| prefix-periodic-1048576 | 0.708723× | 0.697081× | 1.203849× | 1.184073× | -1.643% | -1.592% |
| prefix-repeated-1048576 | 0.710724× | 0.701390× | 1.115044× | 1.100401× | -1.313% | -1.375% |

154,752 balanced matched calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes -0.045%. The largest adverse matched movement is +0.780% on prefix-natural-1048576.

| Input | R333 instructions | R336 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 715,988.5 | 715,741.0 | -0.035% | -5.539% |
| reymont-prefix-32769 | 70,039,340.5 | 69,738,710.5 | -0.429% | +0.463% |
| prefix-natural-262144 | 693,701,065.5 | 690,979,472.5 | -0.392% | +0.866% |
| prefix-natural-1048576 | 2,488,026,909.5 | 2,477,054,864.5 | -0.441% | +1.012% |
| prefix-random-16384 | 10,198,996.0 | 10,026,446.0 | -1.692% | -1.512% |
| prefix-random-1048576 | 576,162,188.0 | 566,210,229.5 | -1.727% | -0.179% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running samples | Matcher linked PCs |
|---|---:|---:|
| eight-literals | 5,253 | 629 |
| prefix-natural-262144 | 5,362 | 1,960 |
| reymont-prefix-32769 | 4,465 | 1,678 |

## Qualification

The compiler, producer and tests build with warnings as errors. All 50 CFG tests pass. The focused JIT witness makes 78,732 calls on the original and revised paths, including short extents, a different current length, unsigned wrap, missing proof and exact readable-window boundaries. The original and decoded images agree and pass 324 linked exact-stream, independent inflation, capacity, alias and cleanup checks. The benchmark uses the compact diagnostic entry.

The first test command found four missing inherited test sources. The next found the inherited stale wide-equality test signature. The test-only closure uses the five exact files from the prior qualified test tree, with no changes to the measured source. Both failed receipts and the reconstructed hashes are archived.

The retained all-81 scoreboard remains R333. The archive preserves every pilot result.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r336-checked-packed-prefix-00a34e16027a.tar.gz). SHA-256 00a34e16027ab62368ef3d3a940949f57cd0dff5e219c6d82c90f30e4f4a0c9c.
