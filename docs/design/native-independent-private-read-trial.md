# Schedule independent reads from a fixed private view

The two hashed partitions of the fixed U16 table occupy disjoint byte ranges. The current selector for the later halfword is already available at the packed four-byte read. The pilot reads the halfword immediately after that packed read and uses the same address for its original store. All four original stores and the cache-write rejection retain their positions. The terminal rejection writes its original origin 1236.

Roc and libdeflate both use a separate hashed address for each partition. Their binaries issue both next-position write hints before accessing the first current bucket, then perform the second demand read after the short-prefix selection. The Mech pilot instead starts its two independent demand reads together. The CLIF audit traces ten joins back to one current selector and verifies all intervening paths. The table partitions span bytes `[131072, 262144)` and `[262144, 524288)` in the caller-admitted view. The other write uses the separately held cache allocation.

| Linked body | Code bytes | Full frame bytes | Static stack references |
| --- | ---: | ---: | ---: |
| Retained | 9,960 | 576 | 410 |
| Scheduled read with address reuse | 9,960 | 576 | 410 |
| Scheduled read without reuse | 10,048 | 576 | 431 |

The other two placements produce 10,064 and 10,068 bytes with 435 static stack references each. All three larger variants exceeded the fixed 9,984-byte slot and stopped at machine preflight. The timed candidate changes only one body with all other executable text and all named addresses fixed. The halfword read moves from `0x100062f90` to `0x100062e7c`, 12 bytes after the first load. The store reuses the same scaled address without reintroducing a spill.

## Matched elapsed

| Input or cohort | Candidate / control | Candidate / Roc | Candidate / libdeflate |
| --- | ---: | ---: | ---: |
| 81-case primary | 0.997551× | | |
| 81-case repeat | 0.998381× | | |
| Distribution change, 16 KiB | 1.003258× | 1.022150× | 1.761813× |
| Natural 16 KiB | 0.997563× | 1.040814× | 1.046776× |
| Reymont 16 KiB | 0.998469× | 1.047623× | 1.052675× |
| Random 128 KiB | 0.987311× | 1.033541× | 1.601536× |
| Random 256 KiB | 0.987161× | 1.051322× | 1.565567× |
| Random 1 MiB | 0.983940× | 1.055882× | 1.492590× |
| Eight literals | 1.004704× | 0.172191× | 2.178628× |

The 1 MiB random improvement repeats at 0.971743× the paired control and 1.059833× Roc. The first full comparison counts 56→57 Roc wins, 28→28 libdeflate, 11→11 both and 73→74 either. Natural 4 KiB moves from 1.001662× to 0.971706× Roc in that cohort. In the second full run both arms are near 0.815×. The same two workers reproduce 58 Roc wins, 28 libdeflate, 11 both and **75 either** in the second full run. Random 65,535 bytes measures 1.191112× versus 1.173010× Roc in the first full run, then 0.987473× versus 0.977750× in the second. The targeted repeat also leaves both below parity at 0.986623× and 0.976597×. The original 81-case result and all timing gates remain in the evidence. Both gates pass in all six cohorts. Periodic 4 KiB has the largest full-repeat increase at +3.65%. The separate adverse cohort takes 1.001877× control. The published compiler remains at its qualified prefetch checkpoint while normal emission is evaluated.

## Independent counters and profiles

| Input | Retired instructions / control | Cycles / control |
| --- | ---: | ---: |
| Random 64 KiB | 1.000004× | 0.990383× |
| Random 128 KiB | 1.000005× | 0.986800× |
| Random 256 KiB | 1.000011× | 0.986366× |
| Random 1 MiB | 0.999437× | 0.981580× |
| Eight literals | 0.999963× | 1.091776× |

The lifetime peak physical footprint in the separate 1 MiB counter process is 1.055644× control. Tracked payload allocations, requested bytes, copies, initialization and private peak agree in each matched timing case. The whole-process peak includes the timing harness and allocator state. The eight-byte counter cohort runs against a separate instrumented link and retains its adverse cycle result.

| Input | Mech matcher / total | Roc collector / total | libdeflate matcher / total |
| --- | ---: | ---: | ---: |
| Distribution change, 16 KiB | 1,405 / 4,957 | 2,562 / 5,182 | 1,197 / 5,278 |
| Natural 16 KiB | 1,655 / 5,285 | 2,057 / 5,294 | 1,219 / 5,313 |
| Reymont 16 KiB | 1,641 / 5,179 | 2,014 / 5,224 | 1,237 / 5,336 |
| Random 1 MiB | 3,707 / 5,360 | 3,092 / 5,394 | 3,655 / 5,354 |

These samples include validation. Symbol scopes differ across executables. The separate candidate profile assigns 3,666 of 5,365 mapped running samples to the matcher on 1 MiB random. The exact Mech distribution-change profile places 1,104 of 4,957 samples in the root. Two sampled root spans include per-item output packing, repeated owner updates and stack loads from the root’s wide frame. Their linked disassembly is included for the next experiment.

## Reproduction

The exact control worker is `1bfd044d1218b68549bfbfd4e3a6b25d150762108dfdbb0ca3d680b9fed903f8`. The relinked worker is `a9bc82ded208702690dcea67b8f75313d06b91ae24a17c63aebdb66eb8bff179`. All 162 linked checks preserve reference bytes, independent inflation, capacities, aliases and cleanup. The compiler source, bytecode, allocator and full-call timer match the qualified prefetch worker. Run `python3 -B verify-round.py` after extraction. The archive contains 403 checksummed files and replays 519,744 timed calls, eight counter cases, and 16 exact-worker profiles. SHA-256 `4c5905f8c3b41ce9ac4acbb68a7525c2ee81c64d6461979a6855c2e22d5d6c4a`. [Measurement and disassembly archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-621eb82bf4-independent-bucket-schedule.tar.gz).
