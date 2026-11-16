# Adjacent private halfword store trial

Two checked halfword stores use the same fixed private buffer and adjacent offsets. Their values already exist at the first store. The original control-flow path between the stores has one incoming edge at each block and no intervening effect. Both complete elements fit the caller-admitted view. The pilot writes the same four little-endian bytes in one store. The real reads, lexical checks, source work and all other stores keep their original paths.

The exact Roc and libdeflate bodies use two halfword stores at the corresponding update. In the Mech search pilot the generated body shrinks from 9,960 to 9,940 bytes, static stack references drop from 410 to 407, and the frame remains 576 bytes. Packing in both search and skip produces 9,956 bytes and 410 references. The skip-only preflight produces 10,036 bytes and 429 references, exceeding the 9,984-byte fixed-address slot. That form was inspected without timing.

| Input or cohort | Search only / control | Search and skip / control |
| --- | ---: | ---: |
| 18-case geometric mean | 1.004226× | 1.004285× |
| Distribution change, 16 KiB | 1.003087× | 0.998438× |
| Natural 16 KiB | 1.002964× | 1.004782× |
| Random 1 MiB | 1.011934× | 1.010605× |
| Periodic 1 MiB | 1.015557× | 0.999772× |
| Repeated 1 MiB | 0.998479× | 1.004161× |

Both measured variants passed the pooled and block gates on all 18 cases. Each still won 10 cases against Roc and five against libdeflate. Search-only random 1 MiB measured 1.087890× Roc and 1.540210× libdeflate. The two-route variant measured 1.087026× Roc and 1.537589× libdeflate. Keep the qualified prefetch executable. The compact code and fewer static references did not translate into lower elapsed time.

The two matched screens include 15,552 timed calls, 324 linked checks and eight separate exact-worker profiles. The replay checks complete bytes, independent inflation, capacities, payload counters, aliases and cleanup. The machine probes hold all named addresses fixed. The control is the worker from compiler `241e01883a6000d9b78f407be94d50d0ed860ee7`. The trial is confined to the matcher and leaves the pinned level-12 source and bytecode fixed.

Run `python3 -B verify-round.py` after extraction. The archive contains 128 checksummed files. SHA-256 `3af368205af8bf19e402b8954b749f2a8c5ff27655c6390d3b8408edef54e4a2`. [Paired screens and exact disassembly](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-241e01883a-adjacent-private-store-trial.tar.gz).
