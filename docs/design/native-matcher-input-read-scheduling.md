# Schedule the checked current input load around bucket reads

The control is compiler `14d4d42214c2d76849cfa5bdfdf3b0e510935ea1` with the packed-base proof enabled. In the qualified matcher, the current packed input word reaches its read after the bucket result. The trial starts the already-proved read earlier so the processor can overlap it with the two bucket reads. The selected current input checks dominate all three placements. The read uses the current shared input, and the intervening writes use the separate admitted private tree. Each variant preserves both access widths and the original error paths.

The CLIF edits change only the matcher. The retained CLIF re-emits the exact 9,936-byte body. All three variants take 10,040 bytes and increase static stack references from 405 to 432. Each has the same 576-byte frame. The initial linker used the preceding matcher's slot size and stopped before building a trial. The aligned comparison uses the current slot and adds 128 bytes for all four actors. All 1,276 linked symbol addresses agree.

| Placement | Elapsed / control, 18 cases | Random 128 KiB / Roc | Random 256 KiB / Roc | Random 1 MiB / Roc |
| --- | ---: | ---: | ---: | ---: |
| Before both bucket reads | 1.004632× | 1.008860× → 1.016486× | 1.028190× → 1.039314× | 1.030345× → 1.038748× |
| Between the bucket reads | 1.002937× | 1.008911× → 1.017125× | 1.029843× → 1.036605× | 1.031505× → 1.037522× |
| After the bucket reads | 1.005834× | 1.006916× → 1.012506× | 1.028541× → 1.033568× | 1.029367× → 1.039421× |

All four workers pass 162 linked checks each, with exact reference bytes, independent decompression and unchanged aliases and cleanup. All three screens pass both 1.05 gates, and their win sets match their paired controls. Every placement slows all three targeted random cases. The trial ends at the screen.

Xcode Time Profiler captured both recordings, and its second export failed with internal error 10. The system sampler then verified two four-second recordings of the exact before-bucket worker. The matcher accounts for 1,934 of 2,994 collapsed leaf samples at 128 KiB and 2,074 of 3,026 at 1 MiB. The no-growth parser accounts for 401 and 387 samples respectively. These sample counts include validation and come from a separate cohort.

Run `python3 -B verify_second_hint.py` to replay 23,328 timed calls and the linked and system-profile receipts. Archive SHA-256 `02f7b1203fd18cd30198c0fd1566616c044c7452a04fa865551a6e7b22d54d20`. [Verified scheduling archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-scheduled-input-read.tar.gz).
