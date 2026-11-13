# Qualify a second private write prefetch

Compiler `d17908fdbd20d9935800f6429a82dc5692b09182`, parent `5c0456236b989f8b965db1e7fe7948b3f87b7876`, measured retained baseline `922503c653ae32bddad18155d6c49f219cce8d0d`. Under `MECH_PREFETCH_PRIVATE_WRITES=1`, setting `MECH_PREFETCH_SECOND_PRIVATE_WRITE=1` permits a second hint only when a different bounded selector addresses a disjoint range in the same fixed private view on the same returning transfer. Both actual writes dominate the transfer. The length, current flat geometry, exclusive backing, selector bounds, and arithmetic proof precede insertion. The backend emits `prfm pstl1keep` at the already-selected edge. The hint leaves the program's accesses, checks, error order, allocation, and publication in their existing paths.

Original source and decoded revision-14 bytecode produce Object `cf5e9f1e4de43a7b3211f0def6fac0f0d0e0cdc2111efb14e04ec1876e4fd378`. The timed worker is `970686a4e1fb89209bf9a62b32420a80fd9a4e6d117f1050590a296b3dd26499`. The original and decoded emissions take 235.55 s and 179.15 s, including 57.02 s to finish the source frontend. The optimized build uses nightly 2026-03-03, `-D warnings` and `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit`. Turning off the second hint reproduces the retained Object, descriptor and bytecode exactly. [Source-independent mechanism and controls](native-second-private-write-pilot.md).

| Paired compression cohort | Cases | Candidate / control | Pooled gate | Block gate |
| --- | ---: | ---: | ---: | ---: |
| Screen | 18 | 0.992833× | 18/18 | 18/18 |
| First full matrix | 81 | 0.996545× | 81/81 | 81/81 |
| Frontier confirmation | 8 | 0.992145× | 8/8 | 8/8 |
| Regression screen | 10 | 1.001789× | 10/10 | 10/10 |
| Full repeat | 81 | 0.995305× | 81/81 | 81/81 |

The candidate and paired control beat the same 59 Roc cases and the same 76 cases against at least one peer in both full comparisons. The first full matrix's largest adverse ratio is 1.019148× on repeated 8 KiB. That case measures 1.015499× in the repeat. Periodic 4 KiB measures 1.007252× and 1.021623×. All four observations pass both gates. Eight literals measure 0.159894× and 0.162425× Roc. Both complete matrices validate all 81 level-12 streams, exact pinned output and capacity, independent decompression, live aliases, equal private allocation/copy counters, and cleanup. Each matrix retains 245,664 timed calls with balanced order, 200 ms warmups, and the same allocator and timing boundary. [Matched frontier](native-compression-scoreboard.md).

| Independent counter case | Instructions / control | Cycles / control | Lifetime peak physical / control |
| --- | ---: | ---: | ---: |
| Eight literals | 1.000007× | 0.998791× | 1.000366× |
| Natural 16 KiB | 1.001930× | 1.004862× | 1.000362× |
| Random 128 KiB | 1.007084× | 0.989153× | 1.000353× |
| Random 256 KiB | 1.007164× | 0.987908× | 1.000342× |
| Random 1 MiB | 1.007534× | 0.988330× | 1.000306× |

The separate profiles put 3,373 of 5,314 mapped running samples in the matcher at random 128 KiB and 3,709 of 5,458 at random 1 MiB. At 128 KiB, the shared stack reload at `0x1000637a0` gets 131 samples. The main predecessor already holds the value it stores to that slot. The near-EOF predecessor reaches the same load with the previous cursor. Samples include independent output validation. [Isolated cursor forwarding follow-up](native-matcher-stack-forwarding-pilot.md).

## Validation

- Three focused JIT tests pass with warnings denied. They cover both hint selections, aliasing ranges, minimum length, changed selector width, conditional transfers, loop-local bases, and equal outputs and writes through 352 differential calls.
- Original and decoded linked qualification passes 648 complete calls across the exact and shared descriptors. The direct parser oracle checks 136,192 positions and preserves rejection origin 15525 and valid retry. Formatting, Git whitespace, compiler planning quarantine, native host catalog, interactive architecture, production resident routing, and source-catalog checks pass.
- The ordinary managed guard reproduces all 13 bundle files byte for byte and validates 88 outputs. The two independent cohorts measure 0.714486× and 0.737414× Rust.
- The standalone compiler adapter initially ran an extra emission when imported by the timing driver. The duplicate process was stopped before any measured comparison. The adapter now runs only explicitly selected build stages. The hosted CI and generated-application suite are outside this local run.

The public replay verifies all 380 files, 505,152 timed calls, five counter cases, three exact-worker profiles and the qualification receipts. Run `python3 -B verify_second_hint.py` after extraction. Archive SHA-256 `3a85671c857599e1f488d6fff596bd3a31ba8d7162c6b29842554b3b708930ff`. [Complete qualification archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-922503c653-second-private-write-qualification.tar.gz).
