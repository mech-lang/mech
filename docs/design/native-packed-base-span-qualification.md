# Reuse the current checked extent for a packed read

The compiler recognizes the saturated shared read limit and proves the original comparison true only when the same current base has a selected nonwrapping endpoint, that endpoint fits the same length, and the checked extent is at least the packed width. It folds one comparison in the emitted matcher. The read stays at its lexical position, and independent candidate positions retain their guards and defaults. The pass runs after the existing scalar-span proof with `MECH_PROVE_PACKED_BASE_SPANS=1`. This revisit of the earlier packed-prefix experiment restricts the proof to the exact checked base and uses the current two-hint matcher. [Earlier packed-prefix comparison](native-checked-packed-prefix.md). [Current isolated comparison](native-packed-base-span-pilot.md).

Compiler `14d4d42214c2d76849cfa5bdfdf3b0e510935ea1`, review parent `614be97d06479f5e81d7632bf402ecf6b2ad70bc`, measured control `d17908fdbd20d9935800f6429a82dc5692b09182`. The decoded Object is `c04be2f8f15d2cdcfd05f5bf88ffa93a89dc082a73bfa007928ec2da9c1b53d2` and the normally linked worker is `ea1da2b6086ac53d71d273ed185918aaff6f1d5893f6cc1774767e61f00674cf`. Its matcher is byte-identical to the current-position pilot. The compiler changes one of 357 bodies, reduces its size by 108 bytes to 9,936, and removes 24 static stack references with the same 576-byte frame. The optimized build uses nightly 2026-03-03 and `-D warnings` with `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit`.

| Matched cohort | Cases | Timed calls | Elapsed / control | Pooled and block gates |
| --- | ---: | ---: | ---: | ---: |
| Screen | 18 | 7,776 | 0.995527× | 18/18 |
| First full matrix | 81 | 245,664 | 0.998878× | 81/81 |
| Confirmation | 8 | 6,816 | 0.994750× | 8/8 |
| Full repeat | 81 | 245,664 | 0.998260× | 81/81 |

The first full matrix beats Roc on 59/81 and libdeflate on 28/81, including 12 against both and 75 against at least one. Natural 32,767 bytes is above both in that cohort for both Mech workers. The repeat wins 59 against Roc, 29 against libdeflate, 12 against both, and 76 against at least one. The repeat gains a Roc win on random 64 KiB, and both full matrices gain libdeflate wins on repeated 16 KiB and repeated 32,769 bytes. The prior published score remains 76 wins against either peer. The current proof improves the three remaining random cases against their matched controls in the full matrix, confirmation, and repeat.

| Input | First control / Roc | First candidate / Roc | Candidate / libdeflate | Confirmation / Roc | Repeat control / Roc | Repeat candidate / Roc |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.161122× | 0.161068× | 2.053762× | 0.160473× | 0.160939× | 0.161389× |
| Random 128 KiB | 1.010629× | 1.005455× | 1.566253× | 1.005001× | 1.046984× | 1.034138× |
| Random 256 KiB | 1.033797× | 1.029658× | 1.530591× | 1.029785× | 1.061032× | 1.052528× |
| Random 1 MiB | 1.036960× | 1.029766× | 1.458260× | 1.033187× | 1.040277× | 1.032715× |
| Natural 16 KiB | 1.041801× | 1.042593× | 1.050393× | 1.036071× | 1.039792× | 1.040481× |
| Reymont 16 KiB | 1.040103× | 1.034597× | 1.042570× | 1.035262× | 1.074578× | 1.073839× |

Natural 1 KiB has the largest adverse paired point estimate in the full repeat at 1.070539×. Both median gates pass, and the input measures 0.452461× Roc. The first full run measures 0.995014× paired control on this case. Natural 16 KiB takes 1.000761× and 1.000663× in the two full matrices. Both values remain in the observations. The matched workers use level 12, the same allocator, all 24 balanced orders, and the unchanged complete-call boundary, source, revision-14 bytecode, output bytes, capacities, aliases, and cleanup. The six tracked allocation and copy metrics agree.

| Independent counter case | Instructions / control | Cycles / control | Lifetime peak physical / control |
| --- | ---: | ---: | ---: |
| Eight literals | 1.000047× | 0.984088× | 1.000000× |
| Natural 16 KiB | 0.998204× | 0.988190× | 1.000000× |
| Random 128 KiB | 0.997401× | 0.990191× | 1.000000× |
| Random 256 KiB | 0.997251× | 0.992992× | 1.000000× |
| Random 1 MiB | 0.996985× | 0.987199× | 1.000000× |

Exact-worker profiles place 3,192 of 5,061 mapped running samples in the matcher at random 128 KiB and 3,593 of 5,338 at 1 MiB. At 128 KiB the newly direct current-position load accounts for 412 samples. Its remaining dependency is the selected bucket result. The composed parser accounts for 2,202 of 4,448 samples at natural 16 KiB. These are timer samples with validation included. The next matcher candidate can start that already-proved current word load before consuming the current bucket, then check whether the extra live word affects spills.

Three focused JIT tests pass. The new witness exercises 108 calls across the proved base and five refusal shapes with a bypass, wrap, short extent, distinct current length, and distinct start. The first emission ran the proof before the current predicates were available and reproduced the retained Object; its local qualification also reused an existing receipt name. The later phase placement and unique receipt name produce the measured candidate. The initial focused test used the wrong packed-load builder signature, followed by a standalone adapter with unused imports. Both are corrected with warnings denied. The original/decoded and managed qualification receipts are tracked separately from these timing results.
