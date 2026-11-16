# Narrow the selected item in ordered reductions

The fresh original-artifact candidate beats R355 on all 81 paired point estimates. Its geometric mean is **0.956458× R355**, and its closest result is natural 256 KiB at **0.994081×**. It beats Roc on **47/81 (58.0%)**, libdeflate on **10/81 (12.3%)**, both on **1/81**, and either on **56/81 (69.1%)**. R355 wins 47 and 7 in the same cohort.

| Input | Mech / Roc | Mech / libdeflate | Mech / R355 |
| --- | ---: | ---: | ---: |
| Eight literals | 0.171242× | 2.184396× | 0.938913× |
| Natural 8 KiB | 1.044187× | 1.298666× | 0.989101× |
| Natural 128 KiB | 1.183691× | 0.974307× | 0.988011× |
| Natural 256 KiB | 1.183400× | 0.961223× | 0.994081× |
| Natural 1 MiB | 1.191266× | 0.962851× | 0.985480× |
| Periodic 1 MiB | 0.601745× | 1.017969× | 0.944648× |
| Repeated 1 MiB | 0.614416× | 0.965273× | 0.949334× |
| Random 1 MiB | 1.412174× | 2.023425× | 0.962411× |

The nearest case above libdeflate parity is repeated 256 KiB at **1.009911×**. Natural 65,535 bytes is the nearest below at **0.987720×**. Random 16 KiB is nearest below Roc parity at **0.945768×**.

## Direct comparison with the previous executable

Natural 256 KiB is **0.976079×** the matcher-terminal executable in the seven-case direct cohort. Eight literals is **1.002809×** and seven stored bytes is **1.040603×**, with identical 83 ns pooled medians on the latter. Periodic, repeated and random 1 MiB are **0.999959×**, **1.002332×** and **0.999567×**. All nine separate natural-text retests beat R355. The 64 KiB boundary through 1 MiB spans **0.978809–0.992487× R355** and **0.975670–0.981388×** the previous executable.

## Compiler mechanism and linked evidence

The existing total-item proof allows the selected value under a U32 conversion to be computed in the low word. Addition, bit operations and the supported constant shifts commute with that projection. The full-width emitter handles the remaining operations. The ARM64 selector uses the zero-extended architectural result of the scalar W-register OR, including when the low word has other uses. Strict winner selection, original checks, and fallback stay in their existing paths.

The exact linked scalar routes replace `ADD X; ORR X; MOV W; CSEL` with `ADD W; ORR W; CSEL`. The final emitter reuses the existing condition flags. The composed routine shrinks from 1,376 to 1,344 bytes, and the ordinary no-growth routine from 1,520 to 1,472 bytes. The direct phase oracle passes 68,096 positions across both routines, including ties, wrap, empty and nonzero starts, vector and scalar tails, failure origin, and recovery. The selector witness passes 384 native calls, including upper-bit inputs and simultaneous narrow and widened uses.

The independent counter cohort retires **600,528,608** instructions at natural 256 KiB versus **614,713,286** on the previous executable, a **2.31%** reduction. Cycles fall **1.88%**. In the exact linked executable's independent profile, the composed parser accounts for **55.9%** of natural 256 KiB running samples, compared with **58.7%** in the prior profile. These shares include validation. The eight-literal counter cohort has **1.049182×** prior cycles with **0.999820×** prior instructions, while the paired elapsed cohort has **1.002809×** prior time. The archive retains the contrary observations.

## Qualification and scope

Compiler base `ef3beb1985bbbf8e17e9436d1013ebb4a7eb215f`, aggregate patch `f88e20105c4bb92cff988fd91bbbea8bf9b1a53f18713ad03c9d3e87da4f71c0`, Object `aa844aa787180285059f63c73e705c84858101673a2955dd4a9226c151d259d5`, linked worker `6a6ac5e8f7da75eb489d8c40b7c0d438e11e8e42f000f61341c966a479e822fd`. Source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` and bytecode `51de51658c1d8d0be66567400e382ef5c0f5789baf317c53a8cd56d1df2dd0ca` match the predecessor. The narrow locked release build took 75.45 seconds, and original Object emission took 627.51 seconds.

All 162 linked original checks pass exact output, independent inflation, alias and cleanup checks. The archive replays 245,664 matched calls in rotated order, 41,832 direct calls, 648 natural retests, 500 counter calls, and five fresh profiles. The same compact diagnostic entry, complete-call timing boundary, warmup and validation placement are used. The full cohort retains 6,617 zero timer observations. Decoded emission and the managed related-post guard on the exact new source are pending. This is the measured external patch, with checked-in integration qualification handled separately.

[Checksummed observations, linked audit and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/compression-narrow-ordered-b5d80da2c63f.tar.gz).
