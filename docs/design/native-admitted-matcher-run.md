# R337 — Share a cutoff for native runs

**Retain R333 with the R272 runtime.** The compiler experiment stops at the 16-case screen. Natural 256 KiB changes +0.538% in the matched comparison and +0.449% in the primary screen. Reymont changes +0.417% and +0.774% in those comparisons.

Measured compiler ebc837def6d5c878cb3ecdeadd0e3f078a3f12e4. Published parent ee77382475a4d98c4483dd2059a040a4eba009d2. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R333 closure and the archived compiler delta. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2 and IR/work/ABI/bytecode 215/107/39/14. The benchmark source, bytecode, allocator, limits, tables, capacities and complete-call boundary are pinned.

## Admit the full-window interval at phase entry

A native phase uses one current captured boundary and one current captured input length. Its equality route exits with the original slide packet. Two subsequent clamps compare the same wrapping remaining length with the same positive threshold. The compiler prepares the minimum of the captured boundary and the input length minus (threshold - 1), saturated at zero. A current position below that cutoff proves the slide condition and both clamp conditions are false. The new edge transports the original maximum and nice length into the existing join. All other visits execute the original dispatch. The proof declines a different operand, an intervening effect, or a join that cannot transport every live value.

Search and skip use one cutoff from the same phase prologue. The remaining-length subtraction and slide predicate move to their fallback only when the fallback dominates every use. The original slide packet, tail clamps, lexical table and window checks, memory reads and writes, and loop exits remain. The generated guard continues to check each current position, including after skip and on reentry.

The exact linked ordinary search dispatch through its existing minimum-length branch changes from 15 instructions and four conditional branches to nine instructions and two branches. The skip span changes from 15 to eight instructions and from four to two conditional branches. Search keeps four stack reads and skip goes from four to three. Cutoff preparation costs eight instructions per activation. The matcher shrinks from 12,272 to 12,200 bytes with the same 624-byte frame. The other 350 bodies match. The flag-disabled Object and metadata match R333.

The pinned libdeflate and Roc matcher excerpts share one remaining-length comparison across the two conditional selects. A separate Rust scalar control shows the same choice. Its pure cutoff variant still evaluates both paths and selects the results. The linked Mech route branches around the old dispatch.

## Matched measurements

| Input | R333 / Roc | R337 / Roc | R333 / libdeflate | R337 / libdeflate | Matched change | Primary change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 2.114989× | 2.115554× | 0.016669× | 0.016674× | +0.027% | +0.000% |
| one-stored | 2.003218× | 2.031483× | 0.017619× | 0.017868× | +1.411% | +0.000% |
| seven-stored-prefix | 2.240396× | 2.248015× | 0.018646× | 0.018710× | +0.340% | +0.000% |
| eight-literals | 0.183567× | 0.183577× | 2.335014× | 2.335144× | +0.006% | +0.833% |
| reymont-prefix-32769 | 1.260894× | 1.266150× | 1.147431× | 1.152214× | +0.417% | +0.774% |
| heldout-window-32768 | 1.256869× | 1.263580× | 1.148374× | 1.154505× | +0.534% | -0.065% |
| prefix-natural-32767 | 1.258828× | 1.261751× | 1.149286× | 1.151954× | +0.232% | +0.287% |
| prefix-natural-32768 | 1.259068× | 1.262895× | 1.147182× | 1.150670× | +0.304% | +0.266% |
| prefix-natural-65537 | 1.269551× | 1.273086× | 1.067623× | 1.070596× | +0.278% | +0.637% |
| prefix-natural-131072 | 1.267204× | 1.275971× | 1.042391× | 1.049603× | +0.692% | +0.680% |
| prefix-natural-262144 | 1.277154× | 1.284025× | 1.029994× | 1.035536× | +0.538% | +0.449% |
| prefix-natural-1048576 | 1.285935× | 1.291242× | 1.038378× | 1.042664× | +0.413% | +0.609% |
| prefix-random-16384 | 1.070722× | 1.057697× | 2.884528× | 2.849440× | -1.216% | -1.847% |
| prefix-random-1048576 | 1.588480× | 1.580618× | 2.247217× | 2.236095× | -0.495% | -0.628% |
| prefix-periodic-1048576 | 0.708570× | 0.688095× | 1.202589× | 1.167839× | -2.890% | -3.030% |
| prefix-repeated-1048576 | 0.711351× | 0.692069× | 1.116631× | 1.086365× | -2.711% | -2.784% |

154,752 balanced complete calls. The screen gates pass 16/16 pooled and 16/16 block. The primary paired geometric mean changes -0.246%. The highest matched change is +1.411% on one-stored.

| Input | R333 instructions | R337 instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
| eight-literals | 715,802.5 | 715,586.5 | -0.030% | -8.693% |
| reymont-prefix-32769 | 70,038,718.0 | 69,688,903.5 | -0.499% | +0.507% |
| prefix-natural-262144 | 693,692,791.0 | 689,897,842.0 | -0.547% | +0.667% |
| prefix-natural-1048576 | 2,488,088,288.0 | 2,472,601,798.5 | -0.622% | +0.619% |
| prefix-random-16384 | 10,198,281.5 | 10,058,465.5 | -1.371% | -2.189% |
| prefix-random-1048576 | 576,157,180.0 | 567,799,882.5 | -1.451% | -1.060% |
| prefix-periodic-1048576 | 488,911,467.5 | 468,055,227.5 | -4.266% | -3.044% |
| prefix-repeated-1048576 | 496,687,322.5 | 475,908,239.5 | -4.184% | -2.719% |

The six heap distributions match on 16/16 cases. The separately sampled profiles use the exact candidate and include validation.

| Candidate profile input | Running samples | Matcher linked PCs |
|---|---:|---:|
| eight-literals | 5,255 | 621 |
| prefix-natural-262144 | 5,349 | 1,989 |
| reymont-prefix-32769 | 4,493 | 1,671 |

## Qualification

The compiler, producer and tests build with warnings as errors. All 50 CFG tests pass. The focused JIT witness makes 40,000 calls across original and revised paths. It checks slide equality, short and exact cutoffs, unsigned wrapping, a threshold with its high bit set, preserved effects and reused predicate values. The original and decoded images agree and pass 324 linked exact-stream, independent inflation, capacity, alias and cleanup checks. The benchmark uses the compact diagnostic entry.

The first compiler build rejected the new module’s visibility. The corrected build and all emitted forms pass. The test closure restores the five inherited test files used in R336.

The retained all-81 scoreboard remains R333. The archive preserves every pilot result.

[Verified compiler patch, linked controls and raw measurement blocks](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r337-admitted-matcher-run-843b01d1f573.tar.gz). SHA-256 843b01d1f5730e7bb7af025a33e4ae2a573a5c91b90f508d668d1807f879b6d2.
