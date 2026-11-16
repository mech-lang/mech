# Current prefix guard trials

The fixed-address clamp worker was the control for both trials. They preserve the entry assertions, selected nonwrapping readable extent, and older-node fallback. The selected route proves a minimum extent of five bytes for the four-byte prefix load. A direct machine diagnostic changes only two words. A second diagnostic re-emits the current-prefix change from the verified IR. Re-emitting the lookahead-only and combined forms increased static stack references from 413 to 473 and 478, respectively.

| Trial | Cases | Mech / paired control | Roc wins | libdeflate wins | Both wins |
| --- | ---: | ---: | ---: | ---: | ---: |
| Direct screen | 18 | 0.999052× | 4 → 4 | 11 → 10 | 2 → 2 |
| Current-prefix screen | 18 | 0.999588× | 4 → 4 | 9 → 9 | 2 → 2 |
| Current-prefix confirmation | 8 | 0.998734× | 2 → 2 | 2 → 2 | 0 → 0 |
| Current-prefix adverse check | 10 | 1.002005× | 6 → 6 | 6 → 6 | 3 → 3 |

| Input and cohort | Peer | Control / peer | Trial / peer |
| --- | --- | ---: | ---: |
| Held-out random 65,537, direct screen | Roc | 1.014357× | 1.012631× |
| Held-out mixed 131,073, direct screen | Roc | 1.012180× | 1.008200× |
| Held-out window 32,767, direct screen | libdeflate | 0.999838× | 1.004296× |
| Held-out random 65,537, current-prefix screen | Roc | 1.007409× | 1.001456× |
| Held-out random 65,537, current-prefix confirmation | Roc | 1.016848× | 1.010233× |
| Held-out mixed 131,073, current-prefix confirmation | Roc | 1.011231× | 1.008273× |
| Natural 32 KiB, current-prefix screen | libdeflate | 0.996165× | 1.001310× |
| Reymont 32,769, current-prefix screen | libdeflate | 1.002642× | 1.008805× |

The re-emitted matcher shrinks from 9,976 to 9,956 bytes with 411 static stack references and the same 576-byte complete frame. The eight exact-worker captures include 3,468 of 5,221 timed running PCs in the direct matcher and 3,513 of 5,248 in the current-prefix matcher for held-out random. Both timing gates passed every measured case. The replay validates 29,376 timed calls and 324 linked calls with byte identity, inflation, ownership and cleanup. The two larger re-emissions are machine-only preflights. No counter or 81-case campaign was run for this trial. The optimization remains an experiment because the target did not cross parity and nearby wins moved against it.

The archive contains 164 checksummed files. SHA-256 `8b223464117f9345ad6d128489b1b51ecbf33f65eefa5d90fc73f920ff10886b`. [Replay the current prefix comparisons](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-e885dd503c-matcher-prefix-guards.tar.gz).
