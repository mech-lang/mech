# Fresh PGO for the expanded parallel compiler

Complete fresh Deflate source-to-executable builds average **13.596783 seconds** with native-CPU tuning and fresh PGO, versus **15.241434 seconds** for the standard compiler (10.79% faster). Both use source `0c949505cc708c233e27c094aa67e105068d6823`, the same 61 program flags and two workers. All complete executable bytes match.

This is **Deflate-trained PGO**: Deflate is one of eight training programs. Seven disjoint held-out scalar/numeric fixtures improve, but they are small; the numeric inputs are 1,264, 1,258 and 1,633 bytes. This qualification does not establish gains on unrelated substantial programs. Native-CPU tuning is also specific to this hardware. The standard compiler remains a separate baseline. No component percentages or historical gains are added.

A separate eight-build comparison changes only worker count on one immutable PGO compiler: **13.959954 seconds serial versus 13.557414 seconds with two workers (2.88% faster)**. Its blocks change -2.01% and -3.75%. The standard-compiler [parallel qualification](native-parallel-phase-cleanup-compile-profile.md) remains separate. Default `MECH_COMPACT_COMPILE_THREADS=0` is serial; `=2` enables workers.

## Complete build boundary

| Boundary | Standard | Native CPU + PGO | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 15.241434 s | 13.596783 s | -10.79% |
| Source-to-Object process | 15.107846 s | 13.461734 s | -10.90% |
| Final native link | 0.132640 s | 0.133986 s | +1.02% |

The complete-build blocks improve 11.53% and 10.04%. Each observation starts fresh source assembly and ends after the final native link process completes. The fixed runtime/entry adapter is cached and hash-pinned; historical first-use adapter construction of 11.882560208 seconds is excluded. Every primary observation runs serially without overlapping heavy builds, tests, sampling, hashing or packaging.

## Compiler work and held-out results

The separate eight-observation Object cohort records these means; stage durations are seconds and instruction counts are whole counts.

| Work | Standard | Native CPU + PGO | Change |
| --- | ---: | ---: | ---: |
| Internal compiler | 15.076816 | 13.443498 | -10.83% |
| Parser | 1.872594 | 1.520720 | -18.79% |
| Checked frontend | 1.631314 | 1.590726 | -2.49% |
| Native preparation | 1.617338 | 1.475634 | -8.76% |
| Numeric helper generation | 1.785776 | 1.511757 | -15.34% |
| Main source CFG | 2.262404 | 2.030503 | -10.25% |
| Cleanup including compaction | 3.006914 | 2.666369 | -11.33% |
| Instructions retired | 220959046107.250000 | 191491197677.750000 | -13.34% |

Adverse Object fields: encode_seconds +11.19%, peak_footprint_bytes +0.01%. Every field, observation and balanced block remains in the [complete result](../../benchmarks/mech-deflate/native-profile/parallel-native-pgo-apple-m3-20261115.json), including held-out memory movement and native-link/handoff movement. These are observed intervals, not exclusive CPU attribution.

| Held-out program | Process time change | Internal compiler change | Instruction change |
| --- | ---: | ---: | ---: |
| numeric-nested-phase-states | -11.53% | -16.03% | -13.67% |
| numeric-changing-capture-tree | -11.82% | -15.10% | -11.26% |
| numeric-multiple-children | -11.43% | -14.51% | -12.13% |
| scalar-u64-divide | -8.03% | -13.03% | -5.78% |
| scalar-i64-add | -7.14% | -12.70% | -5.73% |
| scalar-i64-multiply | -10.91% | -13.12% | -5.76% |
| scalar-f64-divide | -7.57% | -11.80% | -5.75% |

These 56 held-out compilations use ABBA/BAAB order per case. No pooled tiny-program percentage stands in for large-program evidence. A stronger generalization test would train without Deflate and evaluate unrelated substantial programs.

## Build configuration and exact-output gates

The immutable standard compiler is the qualified source-0c949 binary with SHA-256 `6ce7a53b36d3677cf3b20d919657d2824b7d8561ef5eeaf254377626f27980ae`. Fresh instrumented and profile-use builds use nightly-2026-03-03, explicit `aarch64-apple-darwin`, compiler ThinLTO/CGU1, four Cargo jobs, incremental compilation disabled, preserved `-D warnings`, native CPU, absolute profile paths and the matching Rust LLVM profdata. A native-only subject is omitted; the measured subject combines CPU tuning and PGO. No production source or program/runtime allocator changes are made. Profile-use build emits 18,539 missing-function-profile warnings and no profile hash/control-flow mismatches; complete diagnostics are retained.

The fixed training set is Deflate, numeric-nested-phase, numeric-private-fixed-nested, numeric-array-windows, scalar-bool-equal, scalar-u64-add, scalar-i64-divide and scalar-f64-add. All eight instrumented outputs match the frozen standard reference. All raw profiles and the merged profile are retained and pinned inside a lossless nested XZ archive. Streaming replay verifies each original member against its existing hash; the outer archive uses the maintained gzip pack/verify tools. The initial flat 110,387,554-byte packet is recorded before this packaging-only change. Instrumented build takes 630.392 seconds and profile-use build takes 433.186 seconds; both are excluded from Deflate timing.

Both preliminary complete-executable gates, the default-serial gate and an excluded real-job diagnostic pass before primary timing. The diagnostic records 344 jobs, including 48 cleanup-and-machine jobs. Every main output, complete metadata, 14 fixture pairs and all 20 authoritative complete executables match the frozen source/bytecode/Object/metadata/executable bytes. The complete 2,060,440-byte executable remains SHA-256 `d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`. No runtime timing campaign substitutes for this identity.

The first qualification wrapper silently omitted planned default-serial/isolation stages because its derivation searched for helper filenames that had not been renamed. That attempt was interrupted; its entire partial Object cohort is excluded. Original driver, commands, logs, outputs and interruption are preserved. The corrected driver is checked through its actual AST against every declared stage and helper path before fresh qualification. No compiler rebuild or configuration change occurs between attempts. All corrected gates and cohorts pass. Curation also initially copied historical helper filenames; that footer was corrected before the first replay, with both versions retained and no observation changes.

The forty-fifth [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md) includes all profiles, scripts/derivation, logs, observations, program outputs/executables and fixed native inputs. Compiler binaries, dependency targets and the complete source checkout are excluded and hash/Git pinned. Fresh extraction verifies the repository archive and replays every numerical summary and accepted output byte-for-byte. Compilation below ten seconds remains open.
