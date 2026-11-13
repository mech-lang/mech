# Skip-path reload timing

The retained matcher's skip path reloads the current cache cursor three times after reading the literal. It also reloads the original countdown after calculating its wrapping decrement. A fixed-layout control reuses the already checked cursor for the store and increment. The old countdown is recovered from the decremented word. The original bounds branch, literal fallback, header write, increment, and zero test keep their order.

The two locally signed workers derive from the exact grouped-tally control at `60b1cd11a3a12266727b50ba41c83b58e6e0bec9`. They differ in four instruction words and signature bytes. Both pass 162 linked checks spanning all 81 exact outputs, independent inflate, capacity, aliases and cleanup.

| Input | Candidate / control | Candidate / Roc | Control / libdeflate | Candidate / libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Eight literals | 0.991988× | 0.172103× | 2.219962× | 2.202177× |
| Natural 8 KiB | 1.001501× | 1.025763× | 1.272855× | 1.274765× |
| Held-out random 65,537 bytes | 0.997837× | 1.026834× | 1.744404× | 1.740630× |
| Random 1 MiB | 0.997056× | 1.172837× | 1.663279× | 1.658383× |
| Periodic 1 MiB | 1.000125× | 0.587088× | 0.995994× | 0.996118× |
| Repeated 65,537 bytes | 1.003722× | 0.586581× | 1.026964× | 1.030787× |

The nine-case screen contains 6,912 timed calls. Both gates pass 9/9 and the peer win counts agree. The suite takes **0.999718×** control. Repeated 65,537 bytes slows in both order halves at 1.004774× and 1.002672×. The separate exact-worker profile puts 2,297 of 4,442 sampled PCs in the matcher for that input, including validation in the denominator. This diagnostic supplies timing evidence and leaves the retained compiler unchanged.

[Exact instruction changes, raw timings, profile receipts and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-60b1cd11a3-skip-reload-layout-pilot.tar.gz). SHA-256 `dad3bb54d96c01d7232c8375d1e99273490b96d163017f2d13c7e25d33080ad3`. Run `python3 -B verify.py` after unpacking.
