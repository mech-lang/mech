# Move the matcher hints before the demand loads

Roc and libdeflate issue both write-prefetch hints immediately after deriving the next selectors. The current Mech compiler places them on the returning edge. The pilot moves the existing hint for the four-byte bucket, or both hints, to the definitions of those selectors. Each address remains in the same admitted exclusive backing. The guards and actual reads and writes remain in their original order.

The earlier R281 diagnostic encountered an additional spill before linking. This round uses the current matcher with both admitted hints, measures each placement, and checks the same two placements with 32-bit wrapping products. The diagnostic relinker reserves 128 more bytes for the matcher in every worker, including the control. All 1,276 named addresses match within the comparisons. The relinker verifies all unaffected linked text and updates the object symbol and relocation locations. The five workers pass 810 linked calls across the exact reference inventory, independent inflation, capacities, aliases, and cleanup.

| Hint placement | Matcher bytes | Static stack references | Full frame bytes | 18-case elapsed / control | Random 128 KiB / Roc |
| --- | ---: | ---: | ---: | ---: | ---: |
| Returning edge | 10,044 | 429 | 576 | 1.000000× | See paired control |
| Earlier four-byte bucket | 10,108 | 442 | 576 | 1.004993× | 1.021519× |
| Both earlier | 10,176 | 460 | 592 | 1.012508× | 1.043392× |
| Earlier four-byte bucket with narrow products | 10,108 | 442 | 576 | 1.007728× | 1.024300× |
| Both earlier with narrow products | 10,176 | 460 | 592 | 1.010359× | 1.025337× |

All four screens pass both 1.05 median gates, with 31,104 timed calls in total. Moving both hints slows random 128 KiB from 1.010912× to 1.043392× Roc in its comparison. Random 65,537 bytes moves from 0.956826× to 1.013003× and has a paired geometric ratio of 1.058712× despite passing both median gates. The narrow-products variant with both hints early moves distribution change from 0.985721× to 1.001145× Roc. Keep the returning-edge placement.

In the separate exact-worker profile with both hints early, the matcher accounts for 3,130 of 4,901 mapped timer samples on random 128 KiB and 3,656 of 5,371 on random 1 MiB. The larger matcher and extra stack references establish a register-allocation cost from moving the hints. The profile includes validation.

Replay all 205 files with `python3 -B verify_second_hint.py`. Archive SHA-256 `964c46fb5f24b8696ecdb628215feca59163440d799caacef295645c91504696`. [Timed blocks, compiler IR, machine code, and profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-early-hint.tar.gz).
