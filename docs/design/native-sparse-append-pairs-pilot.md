# Pair checked sparse append visits

The control is compiler `14d4d42214c2d76849cfa5bdfdf3b0e510935ea1`, Object `c04be2f8f15d2cdcfd05f5bf88ffa93a89dc082a73bfa007928ec2da9c1b53d2`. The sparse no-growth parser accounts for 671 of 5,061 mapped timer samples on random 128 KiB and 705 of 5,338 at 1 MiB. Its selected empty visit carries the previous cost and runs separate countdown, cache, and output updates. The pinned libdeflate binary carries the cost and predecrements the header pointer. Mech keeps the original packed node and amortizes the loop updates across two visits.

The pilot checks the admitted window, private capacity, nonwrapping byte extents and disjoint input and output spans at entry. It reads two adjacent 32-bit headers with one 64-bit load. It checks both empty selectors and both current cost indices before any pair store. It adds the two costs in the original order with wrapping U32 arithmetic and appends the two original packed U64 nodes. A single remainder, nonempty header, short cost view or entry refusal executes the existing path with the same current count, cursor, length and preceding word. The source, tables, output and scratch capacities, and timing boundary match the control.

| Isolated change | 18-case elapsed / control | Random 128 KiB / Roc | Random 256 KiB / Roc | Random 1 MiB / Roc |
| --- | ---: | ---: | ---: | ---: |
| Derive the cache cursor from count | 0.997984× | 1.013998× | 1.029897× | 1.030642× |
| Move the zero check to the latch | 0.999792× | 1.008724× | 1.028841× | 1.028982× |
| Combine cursor derivation and latch check | 1.001629× | 1.009936× | 1.029276× | 1.023504× |
| Shorten packed-word construction | 0.997720× | 1.005706× | 1.030624× | 1.028269× |
| Pair two checked visits | 0.974972× | **0.966467×** | **0.988707×** | **0.987487×** |

All screens pass both 1.05 gates. Cursor derivation loses a close libdeflate win at repeated 32,769 bytes. The paired loop is the only pilot that moves all three random cases below Roc. Its parser grows from 1,504 to 1,792 bytes. The local stack allocation falls from 176 to 160 bytes and static stack references fall from 40 to 34. Both actors in the paired test use the same 256-byte slot expansion and retain all 1,276 symbol addresses relative to one another. Only the selected ordinary parser body and link signatures differ between actors.

| Full 81-case result | Paired control | Two-visit pilot |
| --- | ---: | ---: |
| Faster than Roc | 59/81 | 62/81 (76.5%) |
| Faster than libdeflate | 28/81 | 33/81 (40.7%) |
| Faster than both | 11/81 | 16/81 (19.8%) |
| Faster than at least one | 76/81 | 79/81 (97.5%) |
| Pooled gate | — | 80/81 |
| Block gate | — | 81/81 |

| Input | Control / Roc | Pilot / Roc | Pilot / libdeflate | Confirmation / Roc |
| --- | ---: | ---: | ---: | ---: |
| Eight literals | 0.162974× | 0.162757× | 2.052790× | 0.162040× |
| Random 128 KiB | 1.009047× | **0.966108×** | 1.502593× | 0.966703× |
| Random 256 KiB | 1.027231× | **0.986245×** | 1.465904× | 0.987878× |
| Random 1 MiB | 1.025421× | **0.987895×** | 1.400525× | 0.985179× |
| Natural 16 KiB | 1.042185× | 1.040993× | 1.047269× | 1.034448× |
| Reymont 16 KiB | 1.038518× | 1.036860× | 1.038730× | 1.036156× |

The first complete matrix runs at 0.982570× paired control across 245,664 calls. Its empty input misses the pooled gate at measured medians of 42 ns and 83 ns. The paired block ratio is 1.027446× and the block gate passes. The published compiler score remains 76/81 while the compiler-generated variant is measured. The eight-case confirmation takes 0.979755× control and passes both gates.

Each of the seven linked workers passes 162 exact complete-stream, independent-inflate, alias and cleanup checks. Each passes the existing direct parser oracle at 68,096 positions, including wrapping and ties, rejection origin 15525, and recovery on retry. In separate four-second system-sample cohorts, the paired parser accounts for 273 of 3,016 collapsed leaf samples at 128 KiB and 271 of 3,026 at 1 MiB. The latch-only trial measures 408 of 3,015 and 399 of 3,016 respectively. Validation runs inside the sampled workers.

The public replays verify 202 and 262 files. The cursor and latch archive SHA-256 is `f09dc13f1fc51043bcf2e95f4cbdffb4ac7fa4b03b3cb020e4d9f5bc4f887482`. The packing and paired-visit archive SHA-256 is `7dea505ee0bfcf563e95db99d3abd0b02698c7154cb03afd05bb66ca1074b33e`. [Cursor and latch trials](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-sparse-parser-latch.tar.gz) · [Paired append trial](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-sparse-parser-pairs.tar.gz).
