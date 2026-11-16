# Qualify the defaulted-read setup

The compact compiler normalizes scalar setup in an already admitted, little-endian AArch64 no-growth phase. It identifies the cursor by its input packet word and requires the only use of its unbiased loop parameter to be a wrapping decrement. Each incoming edge supplies that decremented value directly, so the inner record walk retains the actual header ordinal in the existing carried register.

The original unsigned previous-cell check is equivalent to a nonzero length test. The narrow load requires the same admitted private U64 backing, the original selected address and only low-word users of the read result. It reads the initialized low U32 at that address and preserves the original zero default. The pass marks proved effect-free zero defaults cold, folds branches whose empty routes reach the same argument-free merge, and performs masked shift-and-OR arithmetic in the observed width. The source program, current views, caller admission, checked default order, wrapping cost, tie order, private writes and failure edges follow the existing implementation.

Four of 357 audited Object bodies change. Each composed body shrinks from 1,216 to 1,184 bytes and retains its 224-byte frame. The other 353 bodies include the matcher and generated root. All 1,276 linked symbol addresses agree, and outside the four bodies, Mach-O UUID and signature, the complete linked workers match byte for byte. The final decoded Object matches the screened executable's Object. Original emission produces identical code, with only the artifact form differing in the descriptor. With the flag off, the entire Object and descriptor match the retained baseline. [Fixed-layout and SSA comparisons](native-defaulted-read-setup-pilot.md).

| Matched cohort | Cases | Timed calls | Elapsed / control | Pooled gate | Block gate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Screen | 18 | 7,776 | 0.986828× | 18/18 | 18/18 |
| Confirmation | 8 | 6,816 | 0.983134× | 8/8 | 8/8 |
| First full | 81 | 245,664 | 0.995613× | 80/81 | 81/81 |
| Full repeat | 81 | 245,664 | 0.994905× | 81/81 | 81/81 |
| Final compiler build | 81 | 245,664 | 0.990152× | 81/81 | 81/81 |

The first full run's empty-input pooled medians are 42 ns and 83 ns across 12,288 calls per actor. The balanced estimate is 1.000437× control. The repeat and final-build run record 83 ns for each arm and pass both original gates. The first run remains in the evidence. Seven stored bytes has the largest adverse first-run paired estimate at 1.041636×. Random 8 KiB has the largest in the repeat at 1.022829×, and random 65,536 bytes has the largest in the final-build run at 1.019263×.

| Mech faster than | First full | Full repeat | Final build | Final paired control |
| --- | ---: | ---: | ---: | ---: |
| Roc | 64/81 | 64/81 | 64/81 | 62/81 |
| libdeflate | 34/81 | 32/81 | 35/81 | 34/81 |
| Both | 17/81 | 15/81 | 18/81 | 17/81 |
| At least one | 81/81 | 81/81 | 81/81 | 79/81 |

| Input | First full / Roc | Full repeat / Roc | First / libdeflate | Repeat / libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Eight literals | 0.162227× | 0.161294× | 2.057376× | 2.059261× |
| Natural 16 KiB | 0.998322× | 0.998626× | 1.006344× | 1.009197× |
| Reymont 16 KiB | 0.997781× | 0.998619× | 1.006101× | 1.007513× |
| Reymont 32,769 bytes | 1.036621× | 1.036955× | 0.944460× | 0.946387× |
| Natural 256 KiB | 1.048258× | 1.048957× | 0.849339× | 0.851172× |
| Random 1 MiB | 0.987946× | 0.986181× | 1.396003× | 1.395173× |

The two 16 KiB fixtures have the same input bytes. Their full-matrix paired savings range from 3.25% to 3.67%, and both beat Roc in all three matrices. The earlier natural 16 KiB confirmation is 1.001617× Roc. Repeated 33,027 bytes gains a libdeflate win in all three full runs. Periodic 65,536 and 65,537 bytes narrowly lose their libdeflate wins in the first repeat at 1.001221× and 1.000507×, while remaining faster than Roc. Both beat libdeflate again in the final-build run.

| Input | Final control → candidate / Roc | Final candidate / libdeflate |
| --- | ---: | ---: |
| Eight literals | 0.161196× → 0.161213× | 2.051159× |
| Natural 16 KiB | 1.034124× → 0.997890× | 1.007176× |
| Reymont 16 KiB | 1.029963× → 0.992628× | 1.006790× |
| Reymont 32,769 bytes | 1.060096× → 1.035645× | 0.945749× |
| Natural 256 KiB | 1.080465× → 1.055083× | 0.855102× |
| Natural 1 MiB | 1.092606× → 1.058501× | 0.854026× |
| Random 1 MiB | 0.985462× → 0.990721× | 1.399015× |

Measured compiler parent `7dabc907f14f63ddad4cb8ed0d730836737fff03`. PR base `92f75db84e2523693648084bea16cd52ad60777a`. The production and final proof use optimized nightly 2026-03-03 with warnings denied and `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit`. Set `MECH_NORMALIZE_DEFAULTED_READ_SETUP=1` in addition to the retained compact benchmark flags. Mech source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`, bytecode revision 14 SHA-256 `51de51658c1d8d0be66567400e382ef5c0f5789baf317c53a8cd56d1df2dd0ca`. Candidate Object `ea7328a18d060573a5ab1e38ab072e7d4b612f33f028e5c903ed76fc54e0b32c`, measured worker `eaba302967d65165ad895b609c4b03f60785d8e911be21847c04c0c6eff6c270`. The existing four-actor order balances all permutations with 200 ms warmup. Each case preserves the full level-12 call, source allocation capacities, exact streams and independent inflation. The compiler version does not change the timed source.

Original and decoded workers each pass 324 linked checks across the two descriptors. Each form also passes 68,096 direct phase positions and 288 boundary pairs with unchanged failure origin 15525, held publication and cache, and successful retry. Two focused IR tests pass using the production rule and its extracted helpers. They exercise current base identity, full-word observation, an effectful default, an unequal bias and an independently live unrebased cursor. Formatting and five architecture/catalog gates pass. Initial harness attempts exposed Cranelift builder types, missing test dependency unification and an SSA fixture that did not match the source-derived arithmetic. The evidence retains those failures and the passing focused receipts.

Exact worker profiles locate 2,039/4,420, 2,590/5,361 and 2,937/5,355 running timer PCs in the main parser at natural 16 KiB, Reymont 32,769 bytes and natural 256 KiB respectively. Samples include validation. The recurring record completion and continuation remain the hottest parser instructions.

| Separate hardware cohort | Retired instructions / control | CPU cycles / control | Lifetime physical peak before → after |
| --- | ---: | ---: | ---: |
| Eight literals | 0.999921× | 1.005162× | 44,827,584 B → 44,811,200 B |
| Natural 16 KiB | 0.961908× | 0.964310× | 45,843,392 B → 46,121,920 B |
| Reymont 32,769 bytes | 0.963242× | 0.975501× | 45,843,392 B → 46,121,920 B |
| Natural 256 KiB | 0.961208× | 0.971683× | 50,693,056 B → 50,676,672 B |
| Random 1 MiB | 1.000472× | 1.004587× | 53,052,352 B → 52,511,680 B |

The ordinary managed target reproduces all 13 artifact files and passes 88 exact outputs at 0.717424× and 0.723303× Rust in its primary and confirmation cohorts. The receipt records 56.50 seconds to rebuild the GPU compiler crate and 63.32 seconds for final decoded emission. The original-form replay took 118.03 seconds with the pinned dependency closure and stage diagnostics. These are qualification command times. The separate source-to-Object compiler comparison uses a different measured closure.

The three complete cohorts contain 736,992 timed calls. The published replay verifies all 617 files, including the first-run empty-input gate miss, all peer timings, five counter cases, three profiles, and the artifact and managed qualification. Archive SHA-256 `9c02bb42c351b41c3c4251de179683d9bb80f1f569de3d1d3a877408c4e45529`. [Complete observations, compiler patch and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-defaulted-read-compiler.tar.gz).
