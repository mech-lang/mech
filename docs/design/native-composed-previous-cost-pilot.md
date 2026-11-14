# Carry the previous append cost

Libdeflate's pinned AArch64 parser keeps the previous low cost in a register across its position loop. Mech's composed parser reads the low word of the preceding packed output cell. The trial reads the existing seed once, carries the newly stored low word across the normal append edge and resumes from the current packed value on the sparse exit. A zero-length seed contributes zero. The input packet and private backing remain stable, the admitted capacity covers the whole phase, and the three loop entries account for every live-in value. No other write can modify the selected previous cell. The original wrapping additions, strict selection, record checks and append stores are preserved.

| Input | Screen control → trial / Roc | Confirmation control → trial / Roc | Confirmation control → trial / libdeflate |
| --- | ---: | ---: | ---: |
| Eight literals | 0.163788× → 0.164334× | 0.160551× → 0.161286× | 2.054038× → 2.063445× |
| Natural 16 KiB | 1.040300× → 1.049813× | 1.036168× → 1.049936× | 1.045807× → 1.059703× |
| Reymont 16 KiB | 1.036240× → 1.044578× | 1.033439× → 1.045542× | 1.040604× → 1.052791× |
| Reymont 32,769 bytes | 1.058950× → 1.075788× | 1.064360× → 1.084799× | 0.967392× → 0.985968× |
| Natural 256 KiB | 1.081634× → 1.105194× | 1.081901× → 1.107648× | 0.874407× → 0.895215× |

The 18-case screen takes 1.004575× paired control and the eight-case confirmation takes 1.008899×. Both 1.05 gates pass across the 14,592 timed calls and peer win sets agree within each cohort. Natural 16 KiB slows by 0.91% and 1.33%, and Reymont 16 KiB slows by 0.80% and 1.17%. The previous compiler is retained.

The control body is re-emitted byte for byte from CLIF. The candidate removes the recurrent output read but adds a stack reload of the literal table length in the outer loop after allocation. The code grows from 1,216 to 1,232 bytes with the same 224-byte frame and all 1,276 linked addresses fixed. The first link check required identical table relocation operand registers. Allocation selects a different register, so the revised check verifies opcode, target and both allocated register fields independently. The linked binary audit confines differences to one body and the expected Mach-O UUID and signature.

The measured compiler is `170401d69aaeded4304b2992763dd5dfd7e9aabf`. The unchanged source and bytecode hashes start `331e659e9218` and `51de51658c1d`. The trial Object is `02fa52e3eaa55ccd8fc8d6fb951b3397c73a9d04b4b54579b3290886c65cd6a8` and worker is `8604ae9603085c3023bf3812663e4ae5a835876c69f6f533c4a193989e6cdc0c`. All 162 linked checks pass with exact reference streams, independent inflation, aliases, capacity and cleanup. Hardware counters, compiler integration and the full timing matrix were skipped for this rejected diagnostic.

Three exact-worker profiles place 2,320/4,670 running timer PCs in the candidate parser on natural 16 KiB, 2,659/5,286 on Reymont 32,769 bytes and 3,073/5,359 on natural 256 KiB. They include validation and use separate sessions. The public replay verifies all 115 checksummed files. Archive SHA-256 `6691375d2d0eebc50ba1994c7d54e3df6b6f8f33abd35d6e6e00aa93dcd6c83b`. [Timing, profiles, and linked machine code](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-composed-previous-cost.tar.gz).
