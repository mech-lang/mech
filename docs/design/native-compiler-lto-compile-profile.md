# Optimized compiler build: compilation qualification

The opt-in `compiler` build profile reduces complete fresh level-12 compressor
compilation to **21.830–22.051 seconds** on the Apple M3 Air. Across eight matched
runs, compiler time falls 3.99%, whole-process time 4.11%, native time 5.84% and
retired instructions 4.33%. Source, MBC, Object and complete metadata bytes remain
identical. Frontend time is an adverse 2.76% higher. The ten-second full-compilation
goal remains open.

The profile inherits `release`, enables Thin LTO and uses one codegen unit. It
is defined identically in the production workspace and focused compiler harness.
Existing release and runtime profile settings are unchanged. This optimizes the
Rust compiler executable; the Mech parser, semantic validation, native pipeline,
current ownership proofs, resource accounting and generated-program optimization
level keep their existing behavior.

Build the focused compiler with:

```sh
cargo build --locked --offline --profile compiler -j2 \
  --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --bin compile_profile
```

The production workspace also accepts `--profile compiler`. The observed first
optimized-profile build takes **608.571 seconds** with the existing registry and
other-profile caches. That bootstrap is excluded from every Mech source-to-Object
measurement below. There is no matched cold-bootstrap comparison. The frozen
compiler shrinks from 100,074,208 to 66,540,704 bytes. The ordinary release profile
remains available for routine development and runtime consumers.

Source: `7306a0097a2a8e52bf263bb9d600a5e4ed8b417d`, parent
`1252fcb3ce6443967080f7c437a35ade655d09c2`. The frozen release control comes
from source `57376165608ec32729ff99a04b601d52d8f64950`; its production code is
identical to the documentation-only parent. Both subjects use the same 60 runtime
switches and full fresh source-to-Object pipeline. No cached Object or decoded-only
substitute is used. Hardware: Apple M3 Air, 8 cores (4P+4E), 16 GB, macOS 26.2.

| Measurement | Release control range (4 runs) | Compiler-profile range (4 runs) | Mean change |
| --- | --- | --- | ---: |
| Compiler, s | 22.287956–22.848247 | 21.575810–21.894030 | -3.99% |
| Whole process, s | 22.466337–23.092320 | 21.830462–22.050987 | -4.11% |
| Parse, s | 2.288840–2.455631 | 2.414758–2.437074 | +0.80% |
| Frontend, s | 2.968465–3.078190 | 3.102825–3.139039 | +2.76% |
| Native, s | 16.954898–17.304395 | 15.966328–16.303044 | -5.84% |
| Canonical lowering, s | 0.198611–0.213670 | 0.202238–0.205610 | -1.78% |
| Preparation, s | 3.646018–3.827413 | 3.529229–3.564738 | -6.00% |
| Numeric generation, s | 4.064297–4.139497 | 3.825127–3.860539 | -6.61% |
| Source CFG, s | 2.474967–2.548885 | 2.338951–2.423849 | -5.17% |
| Cleanup including compaction, s | 3.716825–3.811717 | 3.321849–3.507503 | -8.83% |
| Cleanup after compaction, s | 3.553375–3.642206 | 3.156096–3.339580 | -9.21% |
| Compaction, s | 0.163449–0.169511 | 0.165134–0.167924 | -0.67% |
| Retired instructions | 345,210,916,283–345,299,439,254 | 330,068,492,390–330,494,269,016 | -4.33% |
| Peak RSS, bytes | 5,994,233,856–5,998,149,632 | 5,992,873,984–5,996,675,072 | -0.02% |
| Peak footprint, bytes | 5,962,749,232–5,966,648,600 | 5,962,519,808–5,966,337,304 | -0.00% |

Runs are serial control/candidate/candidate/control then
candidate/control/control/candidate. Both block means improve compiler time
(3.87% and 4.12%). Every candidate compiler, whole-process and instruction
observation falls below every control. Parser and frontend means are slower;
the frontend ranges do not overlap. All contrary observations remain. Memory
differences are small and ranges overlap, so this is not presented as a memory
improvement. These finite cohorts are descriptive, not population estimates.
Builds, tests, screens, fixtures, profiling, hashing and packet work do not
overlap primary timing. Both subjects retain value compaction, and its cost is
included in both cleanup comparisons and all totals.

Fourteen successful fixture pairs cover six complete array/enum programs and
eight checked scalar programs. Every source, MBC, Object and complete metadata
file matches between subjects. Both screens and all eight primary compressor
outputs also match the previously qualified program. These are compiler-output
checks; no Rust unit-suite execution under the new profile is claimed.

Two discarded construction trials remain in the same packet:

- Retrying unchanged-state SSA forwarding after compaction passes 23 Rust tests,
  but still changes Object and metadata. Its screen takes 21.813 seconds; it is
  excluded from primary timing and no current runtime nonregression campaign is
  claimed. Compaction does not make it byte-equivalent to the qualified program.
- Ordinary early block sealing passes 21 Rust tests and preserves all output
  bytes. Its screen does not demonstrate reduced construction work: temporary
  value IDs rise from 20,787,570 to 20,790,499, with the same 1,241,393 retained
  definitions. It is not retained or promoted to matched timing qualification.

Both exact patches, source snapshots, commands, test logs, binary pins, screens,
outputs and decisions remain. Neither construction change is in the retained
source. The earlier [SSA rejection](native-ssa-forwarding-compile-trial.md) keeps
its independent runtime evidence and historical configuration.

A separate CPU profile preserves every output byte and retains its raw and
demangled stacks, self tables, command, executable pin, existing demangler source
and any unsuccessful sampling attempts. The first completed window has 212 self
samples in liveness queries, 175 in nonlocal SSA lookup, 159 in dimension
canonicalization and 126 in declaration-array copying. Main-thread waits are
excluded. Samples span stages; these are not stage-exclusive elapsed times or
exact source-line attribution. Those source-level costs remain after this build
optimization.

No new program-runtime timing campaign is claimed. Generated machine code and
complete metadata are byte-identical, and ordinary runtime release settings are
unchanged. Grammar, language, direct Bool/U64/I64/F64 instructions, checked errors,
finite admission and current-visit ownership checks are preserved.

| Output | Bytes | SHA-256 |
| --- | ---: | --- |
| metadata.json | 2,292,704 | `0f6caeb3b18c65552dcda3e5fa460e1796884e4131e275473cfd2858339382cf` |
| program.mecb | 4,138,627 | `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7` |
| program.o | 1,238,328 | `8a4c2b172798d06252055b20398cdbf9517b4b5be76730ea1824435ef89bbba4` |
| source.mec | 160,857 | `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` |

Compiler executable pins:

- Release: `fa4e15f616f0bed889ef300b5907048db12f5db808c4c4688b8698699679d3e2`.
- Optimized profile: `2da379ccabdc68e820b81f74f905f015fd6b74a61fa52b29f04c45fc8b6526eb`.
- Source patch: `5ecc5525dbce5c13d3b74a68dbeb2ef321853c26cd3d27b8bdf69d4476b4c9e8`.

The complete [514-file archive](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/mech-compiler-lto-20261114.tar.gz)
is tracked in benchmarks at the user's request. It contains every raw observation,
counter, output file, command, source patch/snapshot, discarded trial, test log,
profile and pure offline replay script. Executables are excluded and hash-pinned.
Archive SHA-256: `b25e5298f61e3db9aa061b54652bad3559cc562ba0577317b1c747c6dd00dd6f`.
The [receipt](../../benchmarks/mech-deflate/native-profile/compiler-lto-compile-apple-m3-20261114.archive.json)
pins the manifest and verification; the
[compact result](../../benchmarks/mech-deflate/native-profile/compiler-lto-compile-apple-m3-20261114.json)
contains all full-precision observations and arithmetic. The repository copy
was extracted and verified, then replayed byte-identically. Replay needs Python
3.11+ and no compiler or network. See the
[evidence index](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
for commands. Earlier archives remain unchanged.
