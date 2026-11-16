# Forward a recently stored matcher cursor

The current matched worker stores the new cache cursor from `x20` into `[sp,#0xd8]` and immediately reloads it into `x8`. The alternate near-EOF predecessor reaches the same load with the previous cursor, already loaded into `x1`. The compiler-generated worker's exact Time Profiler capture attributes 131 of 5,314 mapped samples to the shared reload at random 128 KiB.

The fixed-layout test gives both actors an identical cold-edge trampoline that moves the previously loaded value into `x8` on the near-EOF route. The candidate changes the common-path reload at `0x1000637a0` to a register move from `x20`. The original store remains because later users still read the spill slot. The 1,276 symbol addresses and all unrelated instruction words match. Each linked variant passes 162 complete calls for exact output, independent inflation, capacities, aliases and cleanup.

| Input | Retained / Roc | Forwarded / Roc | Forwarded / retained | Isolated move / reload |
| --- | ---: | ---: | ---: | ---: |
| Eight literals | 0.162977× | 0.162809× | 0.998968× | 0.998562× |
| Random 128 KiB | 1.014211× | 1.014249× | 1.000037× | 1.000440× |
| Random 256 KiB | 1.036205× | 1.035908× | 0.999714× | 1.001289× |
| Random 1 MiB | 1.040594× | 1.039744× | 0.999183× | 0.998985× |

The two 18-case screens measure 0.998836× for the single-word comparison and 0.998211× for the complete change. Both gates pass for all 15,552 timed calls. Repeated 32,769 bytes moves from 0.997621× to 1.002784× libdeflate in the complete comparison. That case remains faster than Roc at 0.511068×. The random cases show no material gain, so the retained compiler keeps its current cursor handling.

The five-second Time Profiler acquisition reached its time limit and stalled during finalization. It was terminated after nearly four minutes. A separate four-second system sample of the exact candidate puts 1,932 of 3,016 collapsed leaf samples in the matcher at random 128 KiB and 2,056 of 3,024 at 1 MiB. The system sampler groups by symbol and includes validation, so it does not establish a per-instruction change.

Replay all 93 files with `python3 -B verify_second_hint.py`. Archive SHA-256 `19cb79316d395748ea49c745d7bbf4f94409a3d05ea6cfebb85b83ebd92798af`. [Timing, assembly, and exact-worker profile receipts](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-stack-forward.tar.gz).
