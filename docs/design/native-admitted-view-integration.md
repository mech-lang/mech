# Measure normally linked admitted-view optimizations

The compiler-built combination takes **0.999309×** its matched control across 81 cases. Both 1.05 gates pass throughout. The published benchmark stays at **68/81 faster than at least one peer**. This run's control and candidate each win 67/81, with two losses and two gains against the faster peer. The candidate loses one net win against both peers.

| Cohort | Cases | Candidate / control | Roc wins | libdeflate wins | Either win |
| --- | ---: | ---: | ---: | ---: | ---: |
| Screen | 18 | 0.998900× | 4 → 4 | 8 → 9 | 11 → 12 |
| Full | 81 | 0.999309× | 53 → 53 | 24 → 23 | 67 → 67 |
| Confirmation | 8 | 0.997847× | 2 → 2 | 1 → 2 | 3 → 4 |
| Adverse cases | 10 | 0.998858× | 6 → 6 | 4 → 4 | 8 → 8 |

| Full-cohort input | Control → candidate / Roc | Control → candidate / libdeflate | Candidate / control |
| --- | ---: | ---: | ---: |
| Eight literals | 0.171708× → 0.172967× | 2.184176× → 2.200190× | 1.007332× |
| Natural 8 KiB | 0.988311× → 0.984603× | 1.233127× → 1.228500× | 0.996248× |
| Held-out window, 32,767 bytes | 1.098242× → 1.094501× | 1.002263× → 0.998849× | 0.996594× |
| Held-out window, 32 KiB | 1.099523× → 1.095926× | 1.000384× → 0.997111× | 0.996728× |
| Natural 32 KiB | 1.098510× → 1.100242× | 0.999715× → 1.001291× | 1.001577× |
| Natural 33,027 bytes | 1.098903× → 1.100310× | 1.000010× → 1.001291× | 1.001281× |
| Reymont prefix, 32,769 bytes | 1.102269× → 1.111025× | 0.995728× → 1.003638× | 1.007944× |
| Held-out mixed, 131,073 bytes | 1.010797× → 1.011174× | 1.170150× → 1.170587× | 1.000374× |
| Held-out random, 65,537 bytes | 1.015919× → 1.006913× | 1.727619× → 1.712304× | 0.991135× |
| Periodic 256 KiB | 0.565256× → 0.567756× | 0.996616× → 1.001024× | 1.004423× |
| Random 1 MiB | 1.151945× → 1.145171× | 1.631032× → 1.621441× | 0.994120× |

Held-out random improves in both order halves in all three cohorts that measure it. The smaller held-out window is 1.001815× libdeflate in the screen and above parity in confirmation. Natural 33,027 bytes is 0.996423× in the screen and 0.999806× in confirmation, with mixed order halves. Eight literals is +0.47% in confirmation and +0.68% in the adverse cohort. The full run's largest changes are +4.22% on seven stored bytes, +3.38% on the eight-literal prefix, and +1.87% on periodic 8 KiB. Stored output never calls the changed matcher. Both wins against libdeflate at the held-out windows and the full regressions are retained in the evidence.

The two compiler rules share the exact caller-checked length of the current fixed vector. One packs adjacent initialized U16 reads in a private buffer. The other removes unsigned branches proved true by that current length. The caller takes the original path on guard refusal before the numeric phase can act. The two rules preserve the checked source, allocator and full-call timer. The compiler emitted the 10,020-byte matcher from the fixed-address trial, with the same 576-byte frame and identical bytes in all other recorded bodies. Its 128-byte smaller code slot moves subsequent functions. The fixed-address and normally linked timings are independent observations. [Fixed-address trial](native-admitted-view-lengths-pilot.md)

| Separate counter cohort | Candidate / control instructions | Candidate / control cycles |
| --- | ---: | ---: |
| Eight literals | 0.999868× | 1.060146× |
| Natural 33,027 bytes | 0.994817× | 0.994812× |
| Held-out mixed, 131,073 bytes | 0.993398× | 0.997380× |
| Held-out random, 65,537 bytes | 0.984451× | 0.995956× |
| Random 1 MiB | 0.983923× | 0.995320× |

The exact linked profiles place 1,672/4,516 timer PCs in the matcher and 2,219/4,516 in the composed parser on natural 33,027 bytes. The matcher accounts for 3,541/5,270 on held-out random and 3,809/5,375 on random 1 MiB. These samples include validation. Lifetime physical peak is 1.011× the counter control on held-out random and 1.030× on random 1 MiB. The six measured payload allocation, copying, zeroing and cleanup distributions agree on all 81 cases.

The candidate uses compiler `0f41fbaef1a80fea70bd50197612d9a544823b9a` plus the two archived patches. Decoded emission takes 183.31 seconds through the six-feature build with warnings denied. Object `205228d805ba9fa36aea2308528afabc8c03fc430bca0a35989d5a1106d3f868`, worker `a07d1298f3d429d1512777bee25c097028ac6c686c84552ee3ed8be3d384bad4`. The source and bytecode hashes match the prior trial. The decoded worker passes 324 linked checks across both descriptors. A separate parser witness agrees across 68,096 phase positions with the same rejection origin and successful retry. The focused length-folding test runs 1,440 native calls. The initial compiler prototype failed verification after editing branches without rebuilding the CFG and dominator tree. The retry rebuilds both. A second setup attempt refused to overwrite an existing machine receipt, before any timing. The corrected harness verifies the receipt before use. Original source emission and managed timing were skipped for the combined candidate after the full comparison.

[All 267,264 timed calls, patches, counters, profiles and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-0f41fbaef1-admitted-view-integration.tar.gz). SHA-256 `366abb240c4fecea54f886d53840d492fb8bbb0912bf21bf4ffb57061be2bf29`. All 226 files passed replay from the public download.
