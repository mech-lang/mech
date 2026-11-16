# Reuse the checked extent for a packed input read

The native matcher checks that the current input position plus the selected maximum is nonwrapping and fits the current length. On this path the maximum is at least five. Those checks cover a four-byte read at the current position and a four-byte read at the following position. The emitted matcher still compares both starts with its saturating length-minus-three limit. The prototype replaces each proved comparison with its selected edge and retains the load and all preceding checks. The separate table-derived candidate positions keep their own defaults and bounds.

The prototype verifies the three dominating selections in the actual emitted CFG and reproduces the exact retained body before modifying it. Each of the four relinked workers passes 162 complete calls. All 1,276 named addresses match. The source and bytecode, capacities, aliases, payload counters, and timing boundary match the prefetch checkpoint.

| Proof applied | Matcher bytes | Static stack references | 18-case elapsed / control | Random 128 KiB / Roc |
| --- | ---: | ---: | ---: | ---: |
| Retained | 10,044 | 429 | 1.000000× | See paired control |
| Following-position read | 9,948 | 417 | 1.000359× | 1.009005× |
| Current-position read | 9,936 | 405 | 0.996489× | 1.008104× |
| Both reads | 9,916 | 413 | 0.995707× | 1.009306× |

The current-position variant has the better random frontier and repeats the improvement in the eight-case confirmation. It improves random 128 KiB by 0.79%, random 256 KiB by 0.63%, and random 1 MiB by 0.82%. Both 16 KiB text cases improve by about 0.5%. Eight literals slow to 1.033030× their paired control in confirmation and remain 0.166389× Roc. Every screen and confirmation case passes both 1.05 gates. [Compiler-generated comparison](native-packed-base-span-qualification.md).

| Confirmation input | Candidate / Roc | Candidate / libdeflate | Candidate / control |
| --- | ---: | ---: | ---: |
| Random 128 KiB | 1.007193× | 1.566385× | 0.992114× |
| Random 256 KiB | 1.027454× | 1.526181× | 0.993685× |
| Random 1 MiB | 1.028473× | 1.460753× | 0.991777× |

The exact current-position variant puts 3,053 of 4,811 mapped timer samples in the matcher at 128 KiB and 3,617 of 5,358 at 1 MiB. At natural 16 KiB the largest leaf is still the composed parser, with 2,181 of 4,512 samples. The profile includes validation.

Run `python3 -B verify_second_hint.py` to replay 30,144 timed calls, three exact-worker profiles, and the linked receipts. The archive verifies 189 files. SHA-256 `9f32db308c4b1052ee0409423ef333aeda560dd4a415819a1bd8d36263e74e17`. [Observations, linked machine code, and CFG witness](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-word-spans.tar.gz).
