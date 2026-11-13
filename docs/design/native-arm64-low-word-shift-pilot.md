# ARM64 low-word shift pilot

The selector emits one 32-bit logical shift for a 64-bit input masked to 32 bits and shifted by a constant below 32. It applies to an ordinary unsigned bit expression without assumptions about the source program. The shift keeps its original width for amounts outside that range. The selected instruction zero-extends the result.

The vector matcher's four hash computations each lose a mask instruction. The same CLIF input produces a 10,200-byte matcher, up from 10,188 bytes, and keeps the 576-byte frame. Seven additional static stack load sites appear, with the largest concentration in a cold block. The general selector changes 27 of 357 Object bodies, including the root and grouped tally. The root grows by 500 code bytes and its frame falls by 16 bytes. These additional changes are part of the measured candidate.

| Input | Candidate / control | Control / Roc | Candidate / Roc | Control / libdeflate | Candidate / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.994063× | 0.172511× | 0.171487× | 2.196454× | 2.183414× |
| Natural 8 KiB | 1.003229× | 1.022952× | 1.026256× | 1.272066× | 1.276174× |
| Held-out random 65,537 bytes | 0.998559× | 1.027054× | 1.025575× | 1.740308× | 1.737801× |
| Random 1 MiB | 0.998389× | 1.165245× | 1.163368× | 1.650821× | 1.648161× |
| Periodic 1 MiB | 1.026170× | 0.581075× | 0.596282× | 0.987001× | 1.012831× |
| Repeated 65,537 bytes | 1.017568× | 0.565847× | 0.575788× | 0.998211× | 1.015748× |
| Repeated 1 MiB | 1.018152× | 0.596291× | 0.607115× | 0.938491× | 0.955527× |

The 15-case balanced screen contains 154,656 timed calls. Both timing gates pass 15/15. The suite takes **1.007252× control** and loses the two libdeflate crossings above. The repeated 65,537-byte and periodic 1 MiB slowdowns appear in both order halves. The selector was rejected at this screen.

Separate counters on repeated 65,537 bytes record 0.990027× instructions and 1.017468× cycles. Periodic 1 MiB records 0.989441× instructions and 1.017188× cycles. The lifetime peak physical footprint is equal for both in the counter cohort. The exact candidate profile puts 2,848 of 5,239 sampled PCs in the repeated-case matcher, including validation in the denominator.

The control is the exact grouped-tally worker from `659e4c6a59c947dc061474d067abebc516c48396`. Both actors use its measured shared startup descriptor and runtime library. Candidate Object and linked worker SHA-256 start `901d6dd93d28` and `363ba76716a4`. The two focused JIT tests pass 9,216 value checks and the one-shift machine check under warnings-as-errors and the register allocator checker. Exact and common descriptor launches pass 648 complete linked checks. Independent inflate, output capacity, held aliases and cleanup pass. All six private payload metrics match on the timed fixtures. Decoded, managed and full-matrix qualification were stopped at the screen.

[Selector patch, exact CLIF and machine code, timing, counters and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-659e4c6a59-arm64-low-word-shift-pilot.tar.gz). SHA-256 `68e251da84a67c6eb9a3c8ef262b004a05f7c06975df331a943f7d9ebbd7232e`. Run `python3 -B verify.py` after unpacking.
