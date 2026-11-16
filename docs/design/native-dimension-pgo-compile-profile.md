# Fresh PGO after dimension declaration scanning

Complete fresh Deflate source-to-executable builds average **13.414825 seconds** with native CPU tuning and fresh PGO, versus **14.981386 seconds** for the standard compiler (-10.46%). Both use source `ef5e255e817eb5bbc6df61b094a3741b31c49da2`, the same 61 program flags and two compiler workers. The combined compiler configuration is retained; every complete executable byte matches.

The preceding [source optimization](native-dimension-declaration-scan-compile-profile.md) measured the standard compiler at 14.971381 seconds. That is a separate cohort. The earlier [Deflate-trained PGO result](native-parallel-pgo-compile-profile.md), 13.596783 seconds, belongs to source `0c949505cc`. This experiment rebuilds instrumentation and profile use from current source. It does not reuse the older profiles, add percentages or infer an old-versus-new PGO improvement from separate historical means.

## Complete build boundary

| Boundary | Standard | Native CPU + PGO | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 14.981386 s | 13.414825 s | -10.46% |
| Source-to-Object process | 14.849567 s | 13.278165 s | -10.58% |
| Object handoff | 0.000917 s | 0.001096 s | +19.53% |
| Final native link | 0.130901 s | 0.135563 s | +3.56% |

The two balanced complete-build blocks change -10.23% and -10.68%. Four observations per subject use serial ABBA/BAAB order. Every observation starts fresh source assembly and ends after the final native link process completes. The fixed runtime and entry adapter are cached and hash-pinned; historical first-use adapter construction of 11.882560208 seconds is separate. All compiler builds and training finish before qualification. No heavy build, test, sampling, hashing or packaging overlaps primary timing.

A separate eight-build cohort changes only worker count on one immutable current PGO binary: **13.632845 seconds serial versus 13.396915 seconds with two workers (-1.73%)**. It is separate from the standard/PGO comparison. Default `MECH_COMPACT_COMPILE_THREADS=0` remains serial; `=2` enables workers.

## Compiler work and held-out fixtures

The separate eight-observation Object cohort records seconds for durations and whole counts for instructions and bytes.

| Work | Standard | Native CPU + PGO | Change |
| --- | ---: | ---: | ---: |
| Internal compiler | 14.811926 | 13.152857 | -11.20% |
| Parser | 1.869885 | 1.520524 | -18.68% |
| Checked frontend | 1.417462 | 1.372791 | -3.15% |
| Native preparation | 1.595270 | 1.418370 | -11.09% |
| Numeric helper generation | 1.780064 | 1.510476 | -15.14% |
| Main source CFG | 2.265762 | 1.999141 | -11.77% |
| Cleanup including compaction | 2.984498 | 2.673408 | -10.42% |
| Instructions retired | 220425374395.500000 | 191227081999.750000 | -13.25% |
| Maximum RSS | 4494368768.000000 | 4490838016.000000 | -0.08% |

Adverse Object fields: encode_seconds +13.78%. All observations, stage stamps, counters, blocks and adverse native-link/handoff results remain in the [complete summary](../../benchmarks/mech-deflate/native-profile/dimension-native-pgo-apple-m3-20261115.json). Intervals do not represent exclusive CPU attribution.

| Held-out program | Process time change | Internal compiler change | Instruction change |
| --- | ---: | ---: | ---: |
| numeric-nested-phase-states | -8.65% | -13.29% | -13.70% |
| numeric-changing-capture-tree | -7.84% | -10.28% | -11.22% |
| numeric-multiple-children | -8.78% | -13.15% | -12.17% |
| scalar-u64-divide | -6.45% | -8.56% | -5.83% |
| scalar-i64-add | -7.46% | -8.47% | -5.80% |
| scalar-i64-multiply | -5.07% | -7.35% | -5.76% |
| scalar-f64-divide | -3.49% | -7.18% | -5.76% |

The fixed training corpus is Deflate, numeric-nested-phase, numeric-private-fixed-nested, numeric-array-windows, scalar-bool-equal, scalar-u64-add, scalar-i64-divide and scalar-f64-add. Deflate is included in this training corpus. The seven disjoint heldouts are small scalar/numeric fixtures; 7 of seven improve in whole-process time. This does not establish gains on unrelated substantial programs. Each heldout uses its own eight serial ABBA/BAAB observations; no pooled tiny-program percentage is used.

The corpus and an additional seven-profile set excluding Deflate were fixed before any new compiler build or observed outcome. The latter configuration is only a recorded plan and is not built or qualified here. Native CPU tuning is hardware specific, and no CPU-only subject is measured. The comparison therefore measures CPU tuning and PGO together.

## Reproduction and unchanged program

The standard control is the qualified current-source compiler with SHA-256 `3c09f358909db5ab07d09a9ee78a16d3de83f58d9f181f750985e7f3a88653f3`. Fresh instrumented and profile-use builds use nightly-2026-03-03, explicit `aarch64-apple-darwin`, compiler ThinLTO/CGU1, four Cargo jobs, incremental compilation disabled, preserved `-D warnings`, native CPU and matching Rust LLVM profdata. Instrumented construction takes 622.487 seconds; profile-use construction takes 426.845 seconds. Both and training are excluded from Deflate timing. All diagnostics, including missing-function warnings, are retained.

All eight training outputs, 14 fixture pairs, 56 heldout compilations, eight Object observations, two preliminary executable gates, default-serial gate, real-worker diagnostic, eight primary native builds and eight serial/two-worker native builds preserve the complete applicable outputs. The worker diagnostic records 344 submitted jobs including 48 cleanup-and-machine jobs. All 20 complete executables match the frozen 2,060,440-byte executable, SHA-256 `d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`. Source, bytecode, Object and complete metadata all match. No separate runtime timing campaign is claimed; the executable images are identical.

The qualification driver is checked from its actual AST against all eight planned stages and helper hashes before running. The forty-ninth [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md) retains every raw profile, merged profile, per-case profile association, source configuration, recipe/derivation, diagnostic, observation, output, complete executable and fixed native link input. All profile bytes are stored losslessly in a nested XZ archive and stream-verified against their original pins. Compiler binaries, dependency targets and full source checkout are excluded and hash/Git pinned. Fresh repository extraction verifies the archive and replays its full summary byte-identically. Compilation below ten seconds remains open.
