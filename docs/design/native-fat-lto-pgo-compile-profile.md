# Full LTO with fresh compiler PGO

Complete fresh Deflate compilation measures **13.536528 to 13.329149 seconds (-1.532%)**. The full-LTO configuration is rejected; the qualified ThinLTO compiler `8043` remains selected. Both use the exact same source, native CPU tuning and their own PGO. PGO fully counts toward the compilation goal. The separate Object compiler cohort changes +0.909%, with both blocks slower; parsing changes +1.224%. This fails the consistency criterion fixed before timing.

## Frozen configuration and complete builds

Source is `ef5e255e817eb5bbc6df61b094a3741b31c49da2`. Control is the existing qualified ThinLTO compiler `8043`; the candidate changes compiler LTO strategy to full/fat LTO with explicit `CARGO_PROFILE_COMPILER_LTO=fat` and `CARGO_PROFILE_COMPILER_CODEGEN_UNITS=1` on both instrumented and profile-use builds. Cargo's verbose final Rustc invocations independently verify `lto=fat`, one codegen unit, native CPU and the candidate's own fresh profile. No production source or generated-program setting changes.

Both subjects use nightly-2026-03-03, explicit `aarch64-apple-darwin`, four Cargo jobs, incremental compilation disabled, `-D warnings`, native CPU and the same predetermined eight-program training corpus. Every program compilation uses the same 61 runtime flags and two workers. Source assembly through final native-link completion is the primary boundary. The runtime and entry adapter are cached and pinned; historical first-use adapter construction of 11.882560208 seconds is separate.

| Work | ThinLTO control | Full LTO candidate | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 13.536528 | 13.329149 | -1.532% |
| Source-to-Object process | 13.407673 | 13.197069 | -1.571% |
| Object handoff | 0.001027 | 0.000926 | -9.837% |
| Final native link | 0.127828 | 0.131154 | +2.602% |

Times are seconds. Each primary cohort has eight serial ABBA/BAAB observations, four per subject. Complete-build block changes are -0.865%, -2.191%. All builds and training finish before qualification, and no heavy builds, tests, sampling, hashing or packaging overlap primary timing. Retention was fixed before candidate timing: complete-build mean and both blocks, plus separate Object compiler mean and both blocks, must all improve. All exact-output gates must pass.

The original selected `8043` primary mean of 13.414825 seconds and its later 13.275817-second control repeat are separate historical cohorts. Only the direct matched control/candidate observations above measure this LTO change; no historical or component percentages are added.

## Compiler phases and held-out programs

| Work | ThinLTO control | Full LTO candidate | Change |
| --- | ---: | ---: | ---: |
| Internal compiler | 13.080831 | 13.199788 | +0.909% |
| Parser | 1.521244 | 1.539862 | +1.224% |
| Checked frontend | 1.378476 | 1.415961 | +2.719% |
| Native preparation | 1.423830 | 1.442300 | +1.297% |
| Numeric helper generation | 1.516385 | 1.518169 | +0.118% |
| Main source CFG | 1.975396 | 2.002129 | +1.353% |
| Cleanup including compaction | 2.662644 | 2.651906 | -0.403% |
| Retired instructions | 191,236,937,706.50 | 190,673,414,509.25 | -0.295% |
| Maximum RSS bytes | 4,490,838,016.00 | 4,490,477,568.00 | -0.008% |

Durations are seconds; instruction and RSS rows are raw counts and bytes. Object compiler block changes are +0.997%, +0.822%. Phase intervals are wall-clock boundaries rather than exclusive CPU attribution. Adverse Object fields: catalog_seconds +2.373%; compiler_seconds +0.909%; frontend_seconds +2.719%; native_seconds +0.627%; numeric_generation_seconds +0.118%; parse_seconds +1.224%; peak_footprint_bytes +0.000%; preparation_seconds +1.297%; source_cfg_seconds +1.353%; whole_process_seconds +0.960%. Adverse complete-build fields: native_link_seconds +2.602%.

| Held-out program | Process time change | Internal compiler change | Instruction change |
| --- | ---: | ---: | ---: |
| numeric-nested-phase-states | -0.649% | -0.676% | +0.056% |
| numeric-changing-capture-tree | -2.298% | -3.505% | -0.044% |
| numeric-multiple-children | +1.835% | +1.367% | +0.033% |
| scalar-u64-divide | -2.401% | +1.838% | -0.211% |
| scalar-i64-add | -0.470% | +2.091% | -0.216% |
| scalar-i64-multiply | +2.078% | +2.036% | -0.223% |
| scalar-f64-divide | -2.840% | +0.367% | -0.197% |

All heldout adverse fields remain:

- numeric-nested-phase-states: instructions +0.056%
- numeric-changing-capture-tree: None in this cohort.
- numeric-multiple-children: compiler_seconds +1.367%; instructions +0.033%; whole_process_seconds +1.835%
- scalar-u64-divide: compiler_seconds +1.838%
- scalar-i64-add: compiler_seconds +2.091%
- scalar-i64-multiply: compiler_seconds +2.036%; whole_process_seconds +2.078%
- scalar-f64-divide: compiler_seconds +0.367%

Training is fixed to Deflate, numeric-nested-phase, numeric-private-fixed-nested, numeric-array-windows, scalar-bool-equal, scalar-u64-add, scalar-i64-divide and scalar-f64-add. The seven disjoint heldouts are small scalar/numeric programs; this does not measure generalization to unrelated substantial programs. Each has its own eight serial observations, with no pooled tiny-program percentage.

A separate same-candidate-binary worker-count cohort measures **13.519329 to 13.362928 seconds (-1.157%)** for serial versus two workers. It remains separate from the LTO comparison. Default `MECH_COMPACT_COMPILE_THREADS=0` remains serial; `=2` enables the existing workers.

## Output identity and reproduction

Instrumented compiler construction takes 1218.856 seconds, training 28.142 seconds and profile-use construction 655.862 seconds. These are excluded from Deflate compilation timing. Profile-use construction emits 16,872 missing-function warnings and zero profile hash/control-flow mismatches; all diagnostics are retained. Candidate SHA-256 is `9b4fe33b16ae9393fa460956de2a93abdf6a0b67577ddf00e014d4855ee4df27`. The merged profile is 101,468,816 bytes, SHA-256 `196baffc87f28f5254291101092c78e85b9f18b9d90816788e06a1635f6cac64`.

All eight training outputs, 14 fixture pairs, 56 heldout compilations, eight Object observations, default-serial and real-worker gates preserve source, bytecode, Object and complete metadata. The worker diagnostic records 344 submitted jobs, including 48 cleanup-and-machine jobs. All 20 complete executable images reproduce every byte of the frozen 2,060,440-byte executable, SHA-256 `d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`. Emitted program code and runtime inputs are identical; no separate runtime timing campaign is claimed.

The fifty-second [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md) retains every command, diagnostic, observation, complete output/executable, native link input, configuration, plan, script and derivation. Eight raw profiles and the merged profile are stored losslessly and stream-verified. Compiler binaries, dependency targets and the full source checkout are excluded and hash/Git pinned. Fresh extraction verifies the repository archive and replays the entire [summary](../../benchmarks/mech-deflate/native-profile/compiler-fat-lto-pgo-apple-m3-20261115.json) byte-identically. Compilation below ten seconds remains open.
