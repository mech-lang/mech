# R334 — Measure a stop-relative reduction cursor

**Retain R333 with the R272 runtime.** The revised compiler candidate stops at the 16-case screen. Natural 256 KiB changes +2.560% in the matched comparison and +4.381% in the primary screen. The largest adverse matched movement is +5.278% on prefix-natural-1048576.

Measured and published parent ebc837def6d5c878cb3ecdeadd0e3f078a3f12e4. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R333 closure and the archived two-file compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, and IR/work/ABI/bytecode 215/107/39/14. The source, bytecode, allocator, limits, table contents, capacities and complete-call boundary are pinned.

## Use the address displacement for termination

The existing complete-child proof admits the same current forward and reverse views, source checks and visits. On entry to the continuing scalar body, compute the index relative to its admitted stop and anchor the forward read at that stop. Adding the signed displacement reconstructs the original address. The displacement starts negative and advances to zero exactly after the last original visit. The reverse load preserves its existing writeback. The scalar entry hoists the forward bias used by the selected item. An unrelated index use keeps the original lowering. Singleton peeling, vector prefix, strict unsigned selection and terminal results keep their existing semantics.

The first build reached 19 continuing instructions from 16. The selector materialized -1 for its termination test and rebuilt the item bias. The revised build tests the updated cursor directly and computes the item bias at the scalar entry. The exact linked loop changes 16 to 16 instructions, 2 to 2 comparisons, and 3 to 3 moves. It keeps two loads and zero explicit stack transfers inside the continuing route. The register-register item ADD falls outside the current post-allocation flags tracker, so a second cost comparison occupies the instruction saved by the zero-test latch.

Eight of 351 bodies change. The inspected composed phase is 1,520 to 1,584 code bytes and 2,272 to 2,288 stack bytes. The root and 342 other existing bodies are byte-identical. Disabling the flag restores the retained Object and metadata.

The pinned Roc loop advances its logical index and branches on a separate countdown in 11 instructions. The C parser excerpt uses a strict improvement branch and uses a relative byte displacement in its enclosing match-record walk. The safe Rust loop control shares the comparison across its two scalar choices.

## Matched measurements

| Input | R333 / Roc | R334 / Roc | R333 / libdeflate | R334 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.116917× | 2.065484× | 0.016704× | 0.016298× | -2.430% | +0.000% |
| one-stored | 1.972177× | 1.952383× | 0.017747× | 0.017569× | -1.004% | +0.000% |
| seven-stored-prefix | 2.221735× | 2.200639× | 0.018597× | 0.018420× | -0.950% | +0.000% |
| eight-literals | 0.182147× | 0.182456× | 2.313150× | 2.317074× | +0.170% | +1.207% |
| reymont-prefix-32769 | 1.264659× | 1.299741× | 1.146124× | 1.177917× | +2.774% | +3.168% |
| heldout-window-32768 | 1.256741× | 1.293840× | 1.148171× | 1.182066× | +2.952% | +2.888% |
| prefix-natural-32767 | 1.256653× | 1.292409× | 1.148205× | 1.180875× | +2.845% | +2.712% |
| prefix-natural-32768 | 1.259085× | 1.300146× | 1.148562× | 1.186019× | +3.261% | +3.231% |
| prefix-natural-65537 | 1.265880× | 1.314132× | 1.064310× | 1.104880× | +3.812% | +4.193% |
| prefix-natural-131072 | 1.273111× | 1.318265× | 1.044476× | 1.081521× | +3.547% | +3.981% |
| prefix-natural-262144 | 1.267862× | 1.300319× | 1.053682× | 1.080656× | +2.560% | +4.381% |
| prefix-natural-1048576 | 1.273197× | 1.340397× | 1.028053× | 1.082314× | +5.278% | +4.133% |
| prefix-random-16384 | 1.081324× | 1.086909× | 2.895163× | 2.910117× | +0.517% | -0.130% |
| prefix-random-1048576 | 1.598567× | 1.599876× | 2.255428× | 2.257275× | +0.082% | -0.208% |
| prefix-periodic-1048576 | 0.709635× | 0.707678× | 1.202034× | 1.198718× | -0.276% | -0.184% |
| prefix-repeated-1048576 | 0.713612× | 0.711136× | 1.119984× | 1.116098× | -0.347% | +0.071% |

154,752 balanced matched calls. Both established screen gates pass 16/16 and 16/16. The primary paired geometric mean changes +1.824%. Reymont changes +2.774% in the matched comparison and +3.168% in the primary screen.

The separate 256 KiB hardware cohort measures 693,680,976.5 to 715,191,939.5 instructions (+3.101%). Cycles change +3.061%. The six heap distributions match on 16/16 screen cases. The independently sampled profiles include validation.

| Profile input | Running samples | Composed phase linked PCs |
|---|---:|---:|
| eight-literals | 5,243 | 0 |
| prefix-natural-262144 | 5,299 | 3,121 |
| reymont-prefix-32769 | 4,454 | 2,319 |

Both warnings-as-errors builds pass. The original and decoded linked images match byte for byte and pass 324 exact-stream, independent inflation, capacity, alias and cleanup checks. The focused oracle checks 136,192 candidate positions across both image forms and 68,096 retained positions. It covers zero candidates, nonzero starts, vector remainders, exact capacity, wrapping, ties, error origin and retry. Direct phase testing uses admitted packets. The benchmark comparison uses the compact diagnostic entry.

The revised producer initially hit an environmental disk denial from the linker. Reclaiming the superseded, hash-verified build outputs allowed the same command to pass.

The retained 81-case scoreboard remains the R333 measurement. The archived 16-case screen includes all matched, primary, counter and adverse results.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r334-stop-relative-reduction-ce3cb6038375.tar.gz). SHA-256 ce3cb60383758583525a9239159e36642b9bb0ccd8241f336e997da80bb2ef6e.
