# Returning-edge write prefetch pilot

One private-write prefetch reduces random 1 MiB time by **2.62%** against the same-layout no-op. The complete candidate increases periodic 1 MiB time by 1.86% and repeated 1 MiB time by 1.61% in the peer comparison. The periodic case crosses above libdeflate. The compiler change was rejected.

## Machine evidence

The starting compiler is `f5322bba9640e53e64017329b1dcfb3af3803354`. Both trials derive the next selector from the same current private writable view, checked byte range, immutable base and returning SSA argument. The early insertion spans the remaining current traversal and spills the hash to a new stack slot. That version was stopped after inspecting the numeric phase.

The second version emits the address and hint immediately before the returning edge. It keeps the original 576-byte total frame and avoids the early spill. Only the vector matcher changes among 351 Object bodies. Its body remains 10,188 bytes. Static stack read and store sites change from 347 and 108 to 346 and 108. The complete candidate emits one `prfm pstl1keep, [x5]` at linked address `0x1000637bc`. The re-signed fixed-layout pair differs at that instruction and in its signature bytes. The no-op preserves the new address arithmetic and register allocation.

## Matched measurements

| Input | Hint / unchanged | Hint / same-layout no-op | Candidate / unchanged in peer run | Candidate / Roc | Candidate / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Natural 8 KiB | 1.000114× | 1.002043× | 0.998393× | 1.028413× | 1.284674× |
| Random 16 KiB | 0.989535× | 0.988427× | 0.991318× | 0.941982× | 2.545749× |
| Random 1 MiB | **0.971178×** | **0.973698×** | **0.973837×** | **1.374494×** | **1.940655×** |
| Periodic 1 MiB | 1.018186× | 1.000359× | 1.018569× | 0.590970× | 1.004809× |
| Repeated 65,537 bytes | 1.014755× | 0.995283× | 1.012905× | 0.576851× | 1.017095× |
| Repeated 1 MiB | 1.016728× | 0.999611× | 1.016115× | 0.604421× | 0.950569× |

The no-op pair isolates a random 1 MiB benefit in both order halves at 0.973303× and 0.974092×. For periodic and repeated 1 MiB, the no-op workers already take 1.017821× and 1.017124× the unchanged worker's time. The periodic peer crossing is 0.986490× to 1.004809× libdeflate in the same cohort. The two 11-case comparisons contain 154,272 timed calls each and both 1.05 gates pass 11/11. The paired means across cases are 1.004908× and 1.002898× control. The fixed level, source, bytecode, fixture inventory, runtime input, startup descriptor, allocator, output ownership and timing boundaries are recorded in each cohort.

Separate counters on random 1 MiB show 1.006587× instructions, 0.974044× cycles and equal lifetime physical peak. Periodic and repeated 1 MiB show 1.017458× and 1.016483× cycles. Independent exact-worker profiles sampled 3,074 of 5,353 random PCs and 3,309 of 5,416 periodic PCs in the matcher, including validation in the denominator.

## Qualification and disposition

The focused JIT suite passes two tests covering 11 structural cases and 360 paired calls across success, rejection and retry. Four linked workers pass 648 complete output, inflate, capacity, alias and cleanup checks. The archived source patch includes the backend overlay used by the exact measured producer. The candidate Object and linked worker hashes begin `ba62b4671f0a` and `effef26ba82e`. The ordinary compiler and the independently retained grouped tally remain at their published revisions. Decoded emission, the managed guard and full-suite adoption were stopped after the repetitive-input slowdown.

[Compiler and backend patches, machine code, raw timings and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-f5322bba96-latch-prefetch-pilot.tar.gz). SHA-256 `e13ff14008ebdffe1a61b67073c1bf7f656bb33ac078a1243b14a169849f59b8`. Run `python3 -B verify-prefetch.py` after unpacking.
