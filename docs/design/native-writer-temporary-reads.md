# Borrow checked writer reads from held arrays

The 16 KiB distribution-change profile places 1,104 of 4,957 running samples in the generated root. Source tags reproduce all 956,596 root instruction bytes, and associate 125 samples with the reverse path-item, literal-codeword and literal-width reads and their retirement. Another 102 samples map to the literal append. The exact libdeflate writer loads its code and width directly and uses a wide store when output capacity allows. The pinned Roc source reads from the arrays held by the writer loop. The three Mech lookup calls build and project temporary aggregate views and increment and decrement the owner references during each visit.

The isolated ARM64 borrow pilot skips one retain and both of its possible release paths per lookup. The original source home remains held on each active path. The alternative pilot removes the redundant second bounds branch at the same three reads. Each read's default branch, checked position, pointer and published result are preserved by the borrow pilot. The relinks preserve all 1,276 symbol addresses and every instruction outside the selected patch words. Source, bytecode, allocator and full-call timing remain pinned.

| Change and cohort | Cases | Candidate / control | Distribution-change / Roc | Distribution-change / libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Borrow, first 17 completed cases | 17 | 0.995103× | 1.011902× | 1.748091× |
| Borrow, screen | 18 | 0.996616× | 1.005091× | 1.725017× |
| Borrow, confirmation | 8 | 0.991717× | 1.010539× | 1.747525× |
| Remove second bounds checks, screen | 18 | 0.999804× | 1.028658× | 1.777489× |

The distribution-change borrow takes 0.989717× control in the screen and 0.993648× in confirmation. The bounds-only variant takes 1.002252× control on that case. Both 1.05 gates pass on every completed case. The borrow screen records 12/18 wins against either peer versus 11/18 for its control. The confirmation has 3/8 in both arms. The largest borrow-screen increases are seven stored bytes at 1.005609× control and 32 high-entropy stored bytes at 1.003922×. In confirmation natural 16 KiB takes 1.003216× control and eight literals takes 0.994243×. Periodic 4 KiB is 0.986558× in the screen and 0.944424× in confirmation. The qualified full matrix remains 75/81 while the compiler rule is tested separately.

| Input | Retired instructions / control | Cycles / control |
| --- | ---: | ---: |
| Distribution change, 16 KiB | 0.959434× | 0.980591× |
| Natural 16 KiB | 0.997118× | 1.000773× |
| Reymont 16 KiB | 0.997108× | 0.998269× |
| Natural 256 KiB | 0.998660× | 1.000541× |
| Eight literals | 0.999254× | 0.960730× |

The distribution-change counter worker executes 570,414 fewer retired instructions per median call. Lifetime peak physical memory moves from 45,286,336 to 45,188,032 bytes in that separate counter process. Tracked payload requests, requested bytes, initialized copying, zeroing, and private peak agree in the paired elapsed observations. Separate exact-worker sampling finds 937 of 4,409 mapped distribution-change samples in the candidate root. Both samples include validation.

The initial screen requested a nonexistent 32,768-byte Reymont case and stopped at the inventory assertion after 17 cases and four normal cleanup receipts. Its 62,784 timed calls are retained and replayed. The corrected screen uses the existing 32,769-byte case. An earlier source-tag diagnostic failed writing the pre-cleanup IR with `ENOSPC`. The successful tagged diagnostic routes only that large pre-cleanup dump to `/dev/null`. Compiler builds and hardware samples are outside elapsed measurement.

The archive replays 195,360 timed observations, 324 linked checks, four exact-worker profiles and five independent counter cases. Run `python3 -B verify_pilot.py` after extraction. SHA-256 `5b95f0c0c6a5fc4d24a94ea80a3997d2f3ec1fc853e90ba6c78b7e72fe8919c1`. [Paired timing and source-to-machine proof](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-bbc8791a5a-writer-temporary-reads.tar.gz).
