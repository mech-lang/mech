# Forward a zero-trip reduction to its existing exit

The composed phase admits a vector or short scalar reduction only when the current unsigned start is less than the current stop. The other admission conjuncts are proved true in the measured body. Its false edge forwards the same current arguments through the ordinary scalar header, which repeats that comparison and executes zero visits. The direct trial redirects the false-edge trampoline to the existing typed completion, preserving the exact candidate, cost, cursor and header arguments. The other trial retains the trampoline and proves the ordinary comparison false. Neither skips a load, store, checked operation or source effect.

The control is the normal executable qualified at `f6d27bfabed82e696c98514badb7ec64c8eb0b90`. Both trials change only one composed parser body. The body goes from 1,328 to 1,216 bytes, with 23 to 20 conditional branches, 16 static stack references and an unchanged 224-byte frame. Both original GOT relocations remain at the same offsets. Every linked symbol address and all other executable bytes agree after masking UUID and signature fields.

| Trial and cohort | Cases | Mech / paired control | Roc wins | libdeflate wins | Both wins | Either win |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Direct screen | 18 | 0.989488× | 5 → 6 | 12 → 12 | 2 → 2 | 15 → 16 |
| Repeated-branch screen | 18 | 0.989400× | 5 → 6 | 11 → 12 | 2 → 2 | 14 → 16 |
| Direct full | 81 | 0.996775× | 54 → 55 | 26 → 27 | 11 → 10 | 69 → 72 |
| Direct confirmation | 8 | 0.990833× | 3 → 4 | 4 → 4 | 0 → 0 | 7 → 8 |
| Direct adverse check | 10 | 0.994324× | 6 → 6 | 6 → 6 | 3 → 3 | 9 → 9 |

| Input | Peer | Direct full, control → trial | Direct confirmation, control → trial |
| --- | --- | ---: | ---: |
| Held-out mixed 131,073 | Roc | 1.003010× → **0.992278×** | 1.002011× → **0.990567×** |
| Held-out random 65,537 | Roc | 0.985961× → 0.991636× | 0.983207× → 0.995616× |
| Natural 33,027 | libdeflate | 1.000635× → **0.983978×** | 0.998855× → **0.984358×** |
| Reymont 32,769 | libdeflate | 1.001474× → **0.983061×** | — |
| Natural 256 KiB | Roc | 1.111330× → 1.087334× | 1.115056× → 1.089278× |
| Repeated 32,769 | libdeflate | 0.999664× → 1.000579× | — |
| Random 1 MiB | Roc | 1.108103× → 1.110270× | — |
| Eight literals | Roc | 0.170788× → 0.171113× | 0.173436× → 0.173588× |

Both gates pass in every cohort. The largest full paired increase is +2.89% for one stored byte. Periodic 8 KiB rises +0.84% in the adverse check. Held-out random rises +1.26% in confirmation, staying below Roc in both cohorts. Natural 256 KiB improves by 2.16%, 2.31% and 2.19% across the full, confirmation and adverse cohorts. The direct mixed profile puts 1,974/5,387 running timer samples in the composed parser and 2,149 in the matcher. The replay includes both order halves, all adverse results, 275,040 timed calls, 324 linked checks, and eight exact-worker profiles. Hardware counters and the maintained compiler are qualified separately.

The first proof script compared string-valued parsed arguments to integers and stopped before emitting code. Correcting that check validates the exact typed arguments and confirms that the ordinary loop body cannot have run on this route. The archive contains 253 checksummed files. SHA-256 `7840ef3784da979d31442403744502b251ce1d5b36b22504f98bdb74e258b917`. [Zero-trip trial evidence and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-f6d27bfabe-zero-trip-parser-trial.tar.gz).
