# CPU-native compiler and PGO on the retained source

A compiler built for this CPU with instrumentation PGO reduces complete
level-12 Deflate compilation **15.620738 to 14.027481 seconds (10.20%)**.
Both balanced blocks improve about 10.2%, and every generated artifact and
complete executable byte matches. All seven held-out programs also compile
faster in both process-time blocks. This compiler build configuration is retained.
Compilation below ten seconds remains open.

Both subjects use frozen compiler source
`d26cf2c65eac2f5978cabba257bb8bebbe4b0a80`, including the retained allocator,
schema owners, action-output inventory and once-queued reachability changes.
The program/runtime are the current larger compressor image. The standard
control is the already-qualified `b72f` binary from
[packet 39](native-reachability-worklist-compile-profile.md); the candidate's
SHA-256 is `5a1687ef2d82a3f73d8e04328e99dc72372498a215266d1d5ba1ec56bb71700e`.
Both use the same 61 program flags and `MECH_COMPACT_COMPILE_THREADS=2`.
This comparison measures the combined CPU-native/PGO compiler configuration.

## Build, training and use

The exact Git snapshot contains 36,457 files, excluding benchmark evidence.
The qualified standard compiler is reused. Three new compiler builds use
`nightly-2026-03-03`, explicit `aarch64-apple-darwin`, the `compiler` profile
with ThinLTO and one codegen unit, four Cargo jobs, disabled incremental
compilation, and the existing warnings-as-errors flags. The host is an Apple
M3 with 16 GiB RAM. CPU, OS, target-feature and Rust/LLVM records are included.

| Tool construction | Elapsed |
| --- | ---: |
| CPU-native compiler control | 465.788 s |
| CPU-native instrumented compiler | 620.185 s |
| Eight training programs and profile merge | 25.922 s |
| CPU-native profile-use compiler | 427.460 s |

These construction/training times are excluded from program compilation timings.
The CPU-native control first passes all four artifact and complete executable
gates. Eight fixed complete programs then train the instrumented compiler;
every training artifact matches its unprofiled reference. Matching Rust LLVM
22.1.0 tools merge eight raw profiles totaling 269,301,792 bytes into a
100,519,368-byte profile. Seven separate fixture programs remain held out.

The final build preserves `-D warnings` and adds these compiler overrides:

```text
-C target-cpu=native
-C profile-use=/private/tmp/mech-refreshed-native-pgo-20261115/merged.profdata
-C llvm-args=-pgo-warn-missing-function
```

The build log contains **18,617 functions without training data**, with no
hash/control-flow mismatch diagnostics. Every diagnostic remains in the packet.
All source, build, training, merge and qualification commands are captured.

The selected local compiler is
`/private/tmp/mech-refreshed-native-pgo-20261115/use`. The Cargo-managed standard
compiler remains available at its existing path. This qualification pins this
source, training profile, program/runtime and hardware; later source changes
need combined qualification. The earlier smaller-program 8.22% result remains
[a separate historical cohort](native-current-pgo-compile-profile.md).

## Complete source to executable

Eight primary complete builds run one at a time in ABBA/BAAB order after every
compiler build and training job finishes. No heavy build, test, profiling,
hashing or archival overlaps them. Each includes fresh source assembly,
parsing, checked frontend, native preparation, optimization/code generation,
Object/metadata writes, Object handoff and native link process completion.
The fixed runtime/entry adapter and every native linker input are cached and
pinned. Its historical first-use construction of 11.882560208 seconds is excluded.

| Boundary | Standard compiler | CPU-native + PGO | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 15.620738 s | 14.027481 s | -10.20% |
| Source-to-Object process within those builds | 15.494518 s | 13.894030 s | -10.33% |
| Object handoff | 0.000981 s | 0.000969 s | -1.29% |
| Final native link | 0.125238 s | 0.132483 s | +5.78% |

The complete-build blocks improve 10.196% and 10.203%. Every observation remains.
Link/handoff movement is observational; their code and fixed inputs are unchanged.
These measurements are separate from earlier source, parallel and PGO cohorts;
their percentages and timings are not pooled or added.

## Compiler work and held-out programs

A separate eight-run source-to-Object cohort uses the same subjects and order:

| Work | Standard compiler | CPU-native + PGO | Change |
| --- | ---: | ---: | ---: |
| Internal compiler phases | 15.478214 s | 13.941685 s | -9.93% |
| Complete source-to-Object process | 15.547105 s | 14.010018 s | -9.89% |
| Parsing | 1.872229 s | 1.530978 s | -18.23% |
| Checked frontend | 1.666302 s | 1.631706 s | -2.08% |
| Native preparation | 1.610494 s | 1.492673 s | -7.32% |
| Helper generation | 2.055284 s | 1.822781 s | -11.31% |
| Source CFG construction | 2.371343 s | 2.159990 s | -8.91% |
| Cleanup including compaction | 2.994704 s | 2.670444 s | -10.83% |
| Instructions retired | 222.147 billion | 193.271 billion | -13.00% |
| Maximum compiler RSS | 4,493,389,824 B | 4,489,396,224 B | -0.089% |
| Peak footprint | 4,470,685,368 B | 4,470,271,684 B | -0.009% |

Both Object blocks improve compiler time and instructions. Encoding increases
14.27%, from 0.009382 to 0.010722 seconds. All phase/counter observations and
per-case memory movements are in the
[complete result](../../benchmarks/mech-deflate/native-profile/refreshed-native-pgo-apple-m3-20261115.json).

Each held-out program receives a separate eight-run ABBA/BAAB cohort:

| Held-out program | Mean compile process reduction |
| --- | ---: |
| Nested phase states | 11.15% |
| Changing capture tree | 8.23% |
| Multiple children | 11.42% |
| U64 divide | 5.55% |
| I64 add | 9.07% |
| I64 multiply | 5.86% |
| F64 divide | 8.38% |

Every case lowers mean process time, compiler time and instructions, and both
process-time blocks improve in each case. Tiny-program percentages remain
separate. These are compilation measurements.

## Correctness and evidence

All eight training cases, fourteen fixture pairs, 56 held-out compiles and
every main qualification observation preserve the entire source, bytecode,
Object and complete metadata. Both preliminary CPU-native gates, both final
PGO gates, eight primary complete builds and the default-serial gate reproduce
the entire 2,060,440-byte executable with SHA-256
`d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
The default-serial observation is an unpaired byte gate, excluded from means.
Compiler source, grammar/language, proof/eligibility paths and finite execution
accounting are unchanged. No new emitted-program runtime timing campaign is claimed.

The first extra default-serial gate references an incorrect relative helper
path. Python fails before any compiler executes. Its original driver, command
and logs remain; correcting that path passes the same frozen compiler and
configuration. The primary cohorts are unchanged.

The fortieth
[repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains raw/merged profiles, every command/log, source configuration and script
derivation, all program/fixture/executable bytes, native inputs and pure replay.
Compiler binaries/intermediates and unchanged compiler source are excluded and
hash/Git pinned. The exact tar stream is recompressed with gzip level 6 for
transport. Fresh extraction verifies every manifest/file/Git blob; pure replay
checks profiles, diagnostics, counters, clocks and all outputs and reconstructs
the compact result without recompilation or retiming.
