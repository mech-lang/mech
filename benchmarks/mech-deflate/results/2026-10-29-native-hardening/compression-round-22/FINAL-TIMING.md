# Corrected scalar-path admission: protected timing

Base **4857600c48a97c83b51f97535232cb6a12f224a6**; initial production c8700e178ab42682b3b7b79d306510e6f50dbf67; activation repair **43bde0813868613610856eb0e1936070c1a39eae**; measured checkpoint **c3f5cdabafd8e9bb118c7480958b4d27b01bd79b**. IR 125 / work 93 / ABI 36 / bytecode 14. The corrected default optimized compiler passes the unchanged 210-test component cohort, including the independent cost-table regression, before this timing. The actual GPU unit root passes 301 runtime tests with the explicitly absent stats provider omitted. No compiler, test, audit or other benchmark overlaps protected timing or its separate profile.

**Roc = 1×: 72.806671329× → 67.879005245×.** Mech **20.822708 ms → 19.4133955 ms**, against the same fresh **0.286000 ms** reference (20,000 observations). **6.768152% less elapsed; 1.072595× speedup.** All four balanced pairs improve. The earlier [initial timing](TIMING.md) remains labeled with its compiler and introduced activation failure. The preceding round's 73.331359649× uses a different fresh reference; these are separate paired experiments.

| Pair | Order | Before ms | After ms | After / before |
|---|---|---:|---:|---:|
| 1 | before → after | 21.074792 | 19.367167 | 0.918973103 |
| 2 | after → before | 20.628459 | 19.409333 | 0.940900772 |
| 3 | before → after | 21.414416 | 19.596500 | 0.915107841 |
| 4 | after → before | 20.712125 | 19.400750 | 0.936685637 |

Four fixed alternating-order process pairs, three primary observations per control per process: twelve per control. All 40 process observations remain (24 primary, eight warm-ups, eight excluded profile-tagged). A separate 207-repetition process supplies sampling only. Every observation matches the exact 10 raw bytes and independent inflation, output capacity 13, and cleanup of zero bytes/nodes. Output backing changes and its copies remain timed. Same pinned mimalloc 2.1.7, evaluate_into-only harness and existing diagnostic NoLimits. JIT, setup, seed, clear, rebind, verification and drop remain excluded.

Logical work 222038733432 → 222049095608: existing conservative group pricing adds 10362176 while reducing callbacks. No equal-debit claim. Frozen before executable SHA 4a42717a687fa30552221149913ad41008d242da92f4338b96ac76f4bcf3afff; corrected after SHA 7c39895dc9af8de0b6d33c77eb0003edb34ccd98b6cfbe526e25995a9ec2093e. Source, artifact, reference, allocator, harness and dependency hashes, exact commands, all primary observations, and raw profile accompany this report in [final-primary](final-primary/manifest.json).

The source, bytecode, algorithm, tables, 262144-entry U16 tree, 1500513-word U32 cache, depth 300, nice length 258, near-EOF rules, source allocation sites and timing boundaries are unchanged. Mech performs compression itself. Largest complete input remains eight bytes; no larger-input throughput, confidence interval, or parity claim.

Object 13268720 bytes, text 13267988, stack 101456 versus 12924928 / 12924196 / 101152 before. Text SHA 81375460e8949d22efaf08b5ec493112dc34d8b9372d685074d4884e7397e237 is identical before and after the host activation validation repair. There are 1499 static scalar groups / 9425 static sites, not reached frequencies; numeric regions remain 215. Code and stack growth remain explicit.

Fresh sampling partitions 1315 disjoint exclusive ancestry weights: 377 frame actions, 231 ordinary visits, 193 other helpers, 322 selected library calls, 192 remaining. Frame admission accounts for 140. Sample shares are not removable elapsed-time guarantees. Raw stack samples and the partition script are retained.

This corrected timing/profile packet is committed immediately, before further qualification or optimization. Protected scalar controls and final architecture checks follow. The 1× goal remains active.
