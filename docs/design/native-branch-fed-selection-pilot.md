# Narrow the comparison chain to branch-fed selections

The [broad conditional selection trial](native-conditional-select-pilot.md) won two repeated-input cases against both peers but slowed natural 8 KiB at fixed function addresses. The narrower ARM64 rule fuses only a bounded integer comparison chain whose I64 selection is tested immediately by a branch. The complete 81-case comparison takes **0.995253×** control time, with both 1.05 gates passing throughout. Roc wins remain **52/81 (64.2%)**. The matched libdeflate score rises from 15 to **19/81 (23.5%)**, and wins over both from six to ten. The normal compiler reproduces the exact timed worker from both input forms; retain the narrowed rule in the opt-in benchmark profile.

| Input | Trial / control | Control / Roc | Trial / Roc | Control / libdeflate | Trial / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.996365× | 0.173391× | 0.172761× | 2.199629× | 2.191633× |
| Natural 8 KiB | 1.002495× | 1.004299× | 1.006805× | 1.251683× | 1.254806× |
| Natural 256 KiB | 1.000429× | 1.144755× | 1.145246× | 0.927846× | 0.928244× |
| Natural 1 MiB | 1.000268× | 1.154351× | 1.154660× | 0.931989× | 0.932239× |
| Repeated 65,535 bytes | 0.989243× | 0.564027× | 0.557960× | 1.003050× | 0.992261× |
| Repeated 65,536 bytes | 0.986259× | 0.569708× | 0.561880× | 1.011859× | 0.997956× |
| Repeated 65,537 bytes | 0.986951× | 0.566452× | 0.559060× | 1.004250× | 0.991145× |
| Periodic 256 KiB | 0.983981× | 0.577266× | 0.568019× | 1.015034× | 0.998774× |
| Periodic 1 MiB | 0.982062× | 0.581981× | 0.571541× | 0.985583× | 0.967904× |

The 16-case screen takes 0.995396× control and the eight-case confirmation takes 0.989900×. Repeated 65,536 and 65,537 bytes beat libdeflate in both additional runs. Repeated 65,535 bytes crosses below libdeflate in the screen and full run. Repeated 32 KiB crosses in the screen, but does not change the full cohort's win set. Periodic 256 KiB was measured only in the full run. Natural 8 KiB takes 1.002962× control in the screen and 0.999453× in confirmation. The worst full elapsed estimate is seven stored bytes with a prefix at 1.016306×; random 8 KiB is 1.007371× in the full run and 0.988690× in confirmation. The archive preserves all three cohorts with 56,640, 245,664 and 55,872 timed calls.

The rule derives the same Boolean tree as the broad trial. It admits only same-block comparison producers and verifies a single direct SSA occurrence of every tree value, including forwarded block arguments. This permits a shared selection result but refuses a separately consumed condition. The initial witness caught two redundant comparisons for a separately forwarded condition; the revised proof declines that input. The final JIT witness runs 196,830 exact calls covering both branch outcomes, shared result versus condition, raw input words, mixed signed and unsigned widths, and selection without a branch. The rule uses no source identities.

The preflight recompiles the pinned, unchanged vector matcher IR in the same target context. With the rule disabled, all 10,188 function bytes agree with both the retained Object and linked worker. With the rule enabled, the emitted body is the same length and retains its 576-byte frame. The branch-fed selection removes six local instructions; the addition-fed selection is left alone. Static explicit stack references in the function rise from 425 to 431. The diagnostic relinks only that relocation-free body at the original address. All 1,348 text symbol addresses agree, and all executable bytes outside the body, UUID and signature agree. The normal compiler emits the same Object and only one of the 357 audited bodies changes. Its linked elapsed and hardware counter workers are both byte-identical to the corresponding measured executables.

| Separate hardware counter cohort | Trial / control instructions | Trial / control cycles |
| --- | ---: | ---: |
| Eight literals | 1.000037× | 1.030391× |
| Natural 8 KiB | 1.000388× | 1.009344× |
| Natural 256 KiB | 1.000512× | 1.002137× |
| Random 1 MiB | 1.002818× | 1.009808× |
| Repeated 65,535 bytes | 0.978382× | 0.989554× |
| Repeated 65,537 bytes | 0.978591× | 0.988616× |
| Periodic 1 MiB | 0.978631× | 0.987432× |

The matched private payload metrics are identical across the full matrix. The exact linked vector matcher accounts for 1,307/4,603 running timer samples on natural 8 KiB, 2,739/5,232 on repeated 65,537 bytes, and 3,173/5,365 on periodic 1 MiB. These are timer samples, not instruction counts.

The measured parent is `af625d512aaffada6f086fd56e278d837d110a66` and the compiler patch SHA-256 is `8edd28c7dc02f6ffe8bffb3eb00f0998c7b18483ed3e07b040f690fcf9b5063d`. Nightly 2026-03-03 builds the ARM64 backend and the six-feature `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit` producer with warnings as errors. `MECH_ARM64_CCMP_BRANCH_SELECT=1` selects the rule. Source and decoded revision-14 inputs both produce Object `dea88d78758f9d72251d7bbbb41bedcd9cabed2d143cb092b8cbf89625ed9ea1` and worker `8fc50d3bc5b78bbb4e34807782250b19d8e828ef3a5b5f6378281fea6be1d81e`; only the matcher body hash changes to `575f5e0c82567989deed3c95c7f09e00ab7f06dc970206df7665e66ee32effeb`. The decoded command takes 574.91 s and the source command 672.69 s, including 57.49 s in parsing and source semantics together. The build receipts record both inputs and all flags. The 0.09 s decoded step is bytecode loading, not parsing.

The placement worker passed 162 linked checks. Both ordinary compiler outputs pass 648 more across their exact and common descriptors, with exact reference streams, independent inflation, capacities, held aliases and zero live private owners. The two direct reduction checks cover 136,192 positions, identical rejection origin and recovery. The independent related-post guard reproduces all 13 pinned bundle files and passes 88 exact outputs at **0.687296×** and **0.701076×** Rust. Changed-file formatting, compiler-planning quarantine, host catalog, resident routing and interactive architecture pass. This qualification uses the local checks; ordinary CI is canceled for the draft as requested.

[Compiler patch, paired observations, assembly and verifier](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-af625d512a-branch-fed-selection.tar.gz). SHA-256 `9cc194f280156b62904481b92a70d03e084922db50afc445b540e781f04a8a31`. Run `python3 -B verify-round.py` after unpacking.

[Source and decoded compiler qualification, linked checks and managed guard](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-af625d512a-branch-fed-selection-qualification.tar.gz). SHA-256 `364254b762f57b7ca893a58aff35287d057c253e842ef39be7d6dbfe15c27015`. Run `python3 -B verify-compiler.py` after unpacking.
