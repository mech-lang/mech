# Compact compression and fixed managed guard diagnostic

Local source `92fa316769ebb817ebd2eaa051d082eb16319489`. Prior published source head `acb86c04e6d80aacfa25d375a4157deafeb8e5da`. This is a diagnostic of the checked-in local source. The reviewed compiler commits remain local while hosted qualification is open.

Across 81 matched cases and 245,664 calls, the compact candidate was 0.957572× R355 by the geometric mean of paired block ratios. Each case used all 24 four-actor orders. All 81 paired point estimates improved over R355. The 7,014 timer-floor samples on the three stored cases are retained in the archive. Each block mean was positive.

| Comparison | R355 faster | Candidate faster |
| --- | ---: | ---: |
| Roc | 47/81 | 47/81 |
| libdeflate | 6/81 | 10/81 |
| Both | 0/81 | 1/81 |

| Input | Candidate time | Candidate/Roc | Candidate/libdeflate | Candidate/R355 |
| --- | ---: | ---: | ---: | ---: |
| eight-literals | 0.049334 ms | 0.171875× | 2.177691× | 0.941893× |
| prefix-natural-1048576 | 192.415271 ms | 1.193332× | 0.960311× | 0.980449× |
| prefix-natural-131072 | 26.357083 ms | 1.182194× | 0.972087× | 0.988044× |
| prefix-natural-262144 | 53.716895 ms | 1.186112× | 0.961013× | 0.986920× |
| prefix-natural-32768 | 5.068563 ms | 1.163147× | 1.059899× | 0.978937× |
| prefix-periodic-1048576 | 18.865478 ms | 0.602789× | 1.018858× | 0.943274× |
| prefix-random-1048576 | 40.727187 ms | 1.415332× | 2.004066× | 0.957913× |
| prefix-repeated-1048576 | 21.035208 ms | 0.614429× | 0.965208× | 0.949103× |
| reymont-prefix-32769 | 5.097583 ms | 1.166243× | 1.061001× | 0.980101× |

The Roc win set is unchanged from R355. Three natural inputs around 64 KiB and the 1 MiB repeated input cross below libdeflate. The nearest misses are 8 KiB natural text at 1.045414× Roc and 256 KiB repeated at 1.006565× libdeflate. The full archive includes every paired observation, crossing, hardware counter, instrumented heap result and sampled profile. The counters and profiles were collected independently of primary elapsed measurements.

| Fixed managed guard | Mech/Rust | Pairs faster | Exact outputs |
| --- | ---: | ---: | ---: |
| confirmation | 0.712233× | 13/13 | 26 |
| primary | 0.709007× | 31/31 | 62 |

The managed Object matches the one reported at `54fe2abe`. The fixed historical #990 Object has a different SHA-256. Both identities and raw timer fields are included.

Both compact forms passed 162 exact-output, independent-inflate, alias and cleanup checks. The ordinary hosted suite passed 42 tests, failed five with `Value(StaleHandle)` before commit, and ignored one linked test. All five failures reproduced individually on the exact published parent. A separate instrumented run reached library site 4, where generic matrix length tried to copy canonical slot 40. The text-line producer had stored a private view in its dedicated owner table. Source history places the new generic dispatch arm in integration commit `95c9a1f454`. The hosted runtime repair, policy and linked acceptance gates remain open.

| Finding | Local commit | Receipt |
| --- | --- | --- |
| P1 Canonical comparisons through widening | `788d1b9533` | 87 passed, 0 failed |
| P2 Worker fuel on one thread across two accounts | `ed9d0ab313` | 46 passed, 0 failed |
| P2 Standalone Sequence and Control backend overrides | `716b0766f3` | 3 passed, 0 failed |
| P2 Hosted compiled-module image qualification | `38119f0ffe` | 42 passed, 5 failed, 1 ignored |
| P2 Shared unsigned reads in checked append traces | `3c7ca53806` | 87 passed, 0 failed |

Run `python3 -B verify-package.py` from this directory to check hashes and replay the measurements. The archive contains no benchmark executables, compiled Objects or raw Time Profiler traces.

[Download the diagnostic archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-92fa3167-compact-diagnostic.tar.gz) · [SHA-256 receipt](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-92fa3167-compact-diagnostic.tar.gz.sha256)
