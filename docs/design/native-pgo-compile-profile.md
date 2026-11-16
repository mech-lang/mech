# Compiler PGO qualification

Instrumentation PGO reduces the matched complete level-12 Deflate build from
**17.606236 to 15.973487 seconds (9.27%)**, including fresh source compilation
and final native linking with the fixed runtime adapter cached. Both balanced
blocks improve: 9.29% and 9.26%. Every emitted program and executable byte is
unchanged. Complete compilation below ten seconds remains unachieved.

This is an independent experiment on frozen source
`fad53d24d9d2cad61262cb94932405fd9ee40341`. It predates the two schema reuse
changes and [integrated phase workers](native-parallel-phases-compile-profile.md).
Neither subject enables workers. Do not add this result to their percentages;
The [current-source native-CPU + PGO qualification](native-current-pgo-compile-profile.md)
now measures that separate configuration with two workers in both subjects.

## Build and training

The unprofiled control, instrumented compiler and profile-use compiler use the
same frozen tracked source, explicit `aarch64-apple-darwin` target,
`nightly-2026-03-03`, `compiler` profile, Thin LTO, one codegen unit, four Cargo
jobs, disabled incremental compilation and `-D warnings`. The instrumented build
adds `-C profile-generate=ABSOLUTE_PATH`; profile use adds
`-C profile-use=ABSOLUTE_PATH` and
`-C llvm-args=-pgo-warn-missing-function`. Matching LLVM 22.1.0 tooling merges the
eight raw profiles. Every actual command, path, version and build artifact
receipt is in the packet.

Training uses the full Deflate benchmark, three nested/array numeric programs
and four Bool/U64/I64/F64 scalar programs. All eight reproduce their unprofiled
source, bytecode, Object and complete metadata. Training is not a performance
cohort. The raw profiles total 269,166,752 bytes; the merged profile is
100,296,128 bytes.

Profile use reports **18,352 functions without training data** and no
hash/control-flow mismatch diagnostics. All messages remain in the build log.
The fixed suite does not cover the entire compiler. The compiler binary changes;
the generated Deflate executable does not.

| Separate tool work | Elapsed time |
| --- | ---: |
| Control compiler build | 470.881 s |
| Instrumented compiler build, pipeline wrapper | 626.273 s |
| Training and profile merge | 27.990 s |
| Profile-use compiler build | 458.065 s |

The profile-use pipeline wrapper records 458.305 seconds, including its launcher
and receipt work. These are Rust compiler-tool builds and training, all excluded
from Mech program compilation timings. Cargo job count is not Mech function
parallelism.

## Complete source to executable

Eight fresh builds run serially in ABBA / BAAB order, four per compiler, without
concurrent builds, tests, sampling, hashing or archival. Each timed build includes
source assembly/analysis, native optimization and code generation, Object and
metadata writes, Object handoff and final linking. The same adapter and linker
inputs are pinned and reused; adapter first-use construction is excluded.

| Boundary | Control mean | PGO mean | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 17.606236 s | 15.973487 s | -9.27% |
| Source to Object within those builds | 17.466185 s | 15.844157 s | -9.29% |
| Final native link | 0.139137 s | 0.128412 s | -7.71% |

The complete-build saving is **1.633 seconds**. Final-link movement is
observational and is not attributed to PGO. Object handoff increases about
four microseconds. All four PGO builds take 15.930–16.000 seconds.

## Separate source-to-Object cohort

Eight additional serial ABBA / BAAB builds use the precise process-completion
driver and macOS instruction/memory counters. Preliminary screens and cached
link-only gates are separate from both primary cohorts.

| Work | Control mean | PGO mean | Change |
| --- | ---: | ---: | ---: |
| Complete source-to-Object process | 17.929307 s | 15.902447 s | -11.30% |
| Internal compiler phases | 17.842525 s | 15.835393 s | -11.25% |
| Retired instructions | 257.856371 G | 231.615065 G | -10.18% |
| Parsing | 2.099622 s | 1.745861 s | -16.85% |
| Frontend | 2.523307 s | 2.616360 s | +3.69% |
| Source CFG construction | 2.336310 s | 2.122473 s | -9.15% |
| Cleanup including value compaction | 3.398593 s | 3.080015 s | -9.37% |
| Maximum compiler RSS | 5,910,974,464 B | 5,909,336,064 B | -0.03% |
| Peak memory footprint | 5,880,603,856 B | 5,881,132,222 B | +0.01% |

The first control process takes 19.392 seconds; the remaining controls take
17.438–17.444 seconds. That slow observation stays in the means and raw data.
Both blocks improve complete process time, 13.24% and 9.26%, and instructions.
This explains why the Object cohort's overall percentage is larger than the
separate complete-executable cohort's 9.27%. Do not pool them or remove the
outlier. Encoding also increases 2.92%; all stages and counters remain in the
[complete numerical result](../../benchmarks/mech-deflate/native-profile/pgo-compile-apple-m3-20261114.json).

## Held-out programs and output gates

Fourteen paired fixtures reproduce all four outputs. Seven are held out of
training and also receive separate eight-run ABBA / BAAB compile cohorts,
56 additional serial compiles. Every output remains identical.

| Held-out program | Internal compiler time | Retired instructions |
| --- | ---: | ---: |
| Nested phase states | -16.78% | -11.80% |
| Changing capture tree | -10.79% | -9.68% |
| Multiple children | -11.69% | -10.42% |
| U64 divide | -7.07% | -5.80% |
| I64 add | -8.93% | -5.85% |
| I64 multiply | -5.09% | -5.73% |
| F64 divide | -7.39% | -5.79% |

The first nested-phase control has a large startup outlier, retained in its
complete-process mean. U64-divide and F64-divide complete-process blocks are
mixed: their first blocks regress 2.11% and 4.26%, respectively, and their second
blocks improve. No pooled tiny-program percentage is used. Every per-case
process time, compiler time, instruction count, RSS and block result is saved.
These are compiler timings, not emitted scalar program runtime measurements.

Both initial link gates and every complete build reproduce the entire
1,878,568-byte executable, SHA-256
`557d0f5ef2b979e01851ae23b5b3944453a0bd9e772d68161be198916ca06c2c`.
Source, bytecode, Object and complete metadata match the current accepted pins.
No grammar/language change, weaker proof check, scalar retirement or fresh
emitted-program runtime campaign is claimed.

## Evidence and decision

The twenty-sixth [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains raw and merged PGO data, all build/training/qualification logs and
commands, source configuration and harness, all generated programs, fixtures,
complete executables and frozen native linker inputs. Compiler executables are
excluded and SHA-pinned. Unchanged compiler source is pinned to the Git commit;
Rust/LLVM tools and the Cargo dependency cache are not redistributed.

Two packaging-only failures remain: curation first requests a nonexistent
`rust-toolchain.toml`, and the maintained packer refuses to overwrite an earlier
draft archive. The corrected curation uses actual configuration files; every
Cargo command already selects its toolchain explicitly. No compiler build,
generated output or timing changes in either packaging correction.

The repository copy is extracted, manifest/Git-blob verified and replayed
byte-identically against the compact result. Pure replay independently checks
profiles, commands, output bytes and numerical summaries without rerunning
builds or timings. The later
[current-source native-CPU + PGO qualification](native-current-pgo-compile-profile.md)
improves its matched complete build 8.22%; its configuration and cohort remain
separate from this earlier pair. Complete compilation below ten seconds remains
unachieved.
