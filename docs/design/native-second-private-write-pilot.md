# Prefetch a second bounded range in the matcher

The retained AArch64 matcher prefetches one future bucket. The pinned Roc and libdeflate binaries prefetch both independent buckets as the next selectors become available. Both fit in the current fixed, exclusive U16 view. The dense range begins at byte 131,072 and has stride two. The second begins at byte 262,144 and has stride four. The U16 selector bound is 65,535. Their admitted byte ranges are disjoint and fit in the 524,288-byte backing. The reached returning edge follows both actual writes and carries both fresh selectors.

The first fixed-layout experiment narrows the two wrapping products to 32 bits. It uses the resulting instruction slots and the existing alignment gap to test the second hint with the same register allocation and addresses. The paired stub controls differ at one instruction, a NOP, a write prefetch, or a read prefetch. Four linked variants each pass 162 complete calls. Two preliminary layout attempts stopped before linking because the available alignment gap was insufficient or an incoming near-EOF edge still targeted a relocated instruction. The final fixed-layout audit preserves the near-EOF target, all 1,276 named addresses, and the unaffected text.

The compiler-generated variant finds both ranges from their existing typed stores, proved fixed length, and the same unconditional edge. It preserves the denser first candidate and adds one secondary write hint. Only the matcher body changes among 357 audited routines. The body grows by 84 bytes to 10,044, with the same 576-byte frame and 429 versus 410 static stack references. A second normally linked control replaces only the added hint with a NOP. The exact linked worker is `970686a4e1fb89209bf9a62b32420a80fd9a4e6d117f1050590a296b3dd26499` and its Object is `cf5e9f1e4de43a7b3211f0def6fac0f0d0e0cdc2111efb14e04ec1876e4fd378`. Both linked controls pass 162 complete calls with independent inflate, exact output, capacity, aliases, and cleanup.

| Paired screen | Cases | Timed calls | Elapsed / control | Both 1.05 gates |
| --- | ---: | ---: | ---: | ---: |
| Fixed layout, narrow products | 18 | 7,776 | 0.999267× | 18/18 |
| Fixed layout, write hint versus stub NOP | 18 | 7,776 | 0.986869× | 18/18 |
| Fixed layout, read hint versus stub NOP | 18 | 7,776 | 0.987389× | 18/18 |
| Fixed layout, write hint versus retained | 18 | 7,776 | 0.993734× | 18/18 |
| Fixed layout, write hint confirmation | 8 | 6,816 | 0.989363× | 8/8 |
| Compiler-generated hint versus its own NOP | 18 | 7,776 | 0.986213× | 18/18 |
| Compiler-generated hint versus retained | 18 | 7,776 | 0.992833× | 18/18 |

| Random input | Fixed-layout write / NOP | Generated write / NOP | Generated / retained | Generated / Roc | Generated / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| 128 KiB | 0.972239× | 0.977155× | 0.985300× | 1.014249× | 1.574785× |
| 256 KiB | 0.974573× | 0.977191× | 0.987284× | 1.035817× | 1.541278× |
| 1 MiB | 0.968672× | 0.972237× | 0.980759× | 1.038984× | 1.467927× |

The generated screen's largest adverse ratio is 1.003124× on periodic 1 MiB. Eight literals measure 0.161177× Roc and 1.001048× paired control. The fixed-layout write hint increases retired instructions by 0.92%, 0.93%, and 0.97% on the three large random cases and lowers cycles by 1.74%, 1.69%, and 1.95%. Its separate exact-worker profiles still place 3,380 of 5,310 mapped running samples in the matcher at 128 KiB and 3,619 of 5,362 at 1 MiB. A recurring stack reload at `0x100063794` collects 99 samples at 128 KiB. The common predecessor has just stored the value in the same slot. The near-EOF predecessor reaches the same load with the old value. These samples include validation.

The source and revision-14 bytecode pins are `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` and `51de51658c1d8d0be66567400e382ef5c0f5789baf317c53a8cd56d1df2dd0ca`. Each matched screen uses all 24 worker orders, 200 ms warmups, the same allocator and complete-call ownership boundary. All six tracked private allocation and copy metrics match. The isolated timings establish a cache-hint benefit with the register layout held constant. The compiler-generated comparison measures the total effect of enabling it.

Run `python3 -B verify_second_hint.py` after extraction. It verifies 306 files and replays 53,472 timed calls, six independent counter cases, and four exact-worker profiles. Archive SHA-256 `4a7bf7485acc32744bb2edff04f15f9ac937aec88c2ae402a3af9ed4f94dbc71`. [Raw timing blocks, peer assembly, and machine receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-trial.tar.gz).
