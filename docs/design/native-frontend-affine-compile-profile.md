# Consecutive dimension remapping

Fresh level-12 source-to-Object compilation takes **38.903–39.112 seconds**,
compared with **40.452–41.252 seconds** for its matched control on the Apple M3
Air. Mean frontend time falls **21.60%**, complete compiler time **4.51%** and
retired instructions **3.46%**. Source change
`cfd97b6a35911fbd5286dbf122186426c4425433` changes temporary dimension-map
storage and preserves every emitted program byte.

Schema embedding repeatedly renames consecutive dimension IDs. The temporary
map previously stored every pair, using a bounded dense vector followed by an
ordered sparse map. It now holds a consecutive translation inline as a start
pair and length. An insertion still checks the actual source and target IDs;
gaps or changed replacements materialize the original map. Duplicate insertions
return their original values. Wider comparisons prevent wrapping at `u32::MAX`.
Insertion order, checked ordinal construction, validation and errors remain.
Grammar, language semantics and generated runtime code do not change.

| Balanced run order | Parse | Frontend | Native | Compiler total | Whole process |
| --- | ---: | ---: | ---: | ---: | ---: |
| Control 1 | 2.476 s | 9.052 s | 29.640 s | 41.252 s | 42.363 s |
| Candidate 1 | 2.475 s | 6.831 s | 29.725 s | 39.112 s | 40.234 s |
| Candidate 2 | 2.461 s | 6.834 s | 29.528 s | 38.903 s | 39.055 s |
| Control 2 | 2.449 s | 8.378 s | 29.540 s | 40.452 s | 40.697 s |

All observations use the same 52 flags, source and toolchain. Builds, tests,
fixture comparisons and CPU sampling do not overlap primary timing. Compiler
bootstrap takes **243.87 seconds**, excluded from program compilation. The
initial launch of each subject retains about one second of startup beyond the
compiler's phase timers.

Both elapsed pairs improve, but four observations do not establish a statistical
population. The first control's adverse frontend interval is retained and
enlarges the observed mean improvement. Even the second control's 8.378 seconds
exceeds both candidate frontend observations. Mean native time rises **0.12%**;
this change does not establish a native or parsing improvement. Candidate maximum
RSS is **7,758,938,112–7,760,216,064 bytes**, compared with
**7,473,135,616–7,769,096,192 bytes** for control. Its mean is higher, the ranges
overlap, and no whole-compiler memory saving is claimed.

All four observations retain byte-identical source, canonical bytecode, native
Object and complete metadata. Object SHA-256 remains
`4ddc16b053bda6256656e0d5cbf95ba60cab31b0911b9aadf526b66e1f3cab67`;
metadata remains
`0813625d0178e329bfd780c8a0d5f714bbd15a23727d22505172d1136bd0b44d`.
The optimization preserves generated instructions; no new compression-runtime
timing campaign is claimed.

All **63 focused tests across five suites** pass. Remapping tests compare
irregular, sparse and duplicate mappings with an independent ordered map,
exercise a 200,000-ID inline range, force materialization, and cover nonzero
starts and both overflow boundaries. Six complete additional programs preserve
all four outputs: nested numeric phases, enum-state children, private fixed
nested buffers, changing captures, array windows and multiple children. The
enum-state input follows the maintained nested-phase test assembly. These are
compilation comparisons, without new execution claims.

The initial format check selected Rust 2021 and rejected existing let chains;
the corrected Rust 2024 check passes. An initial evidence snapshot referenced
an absent optional toolchain filename; removing that entry fixes curation.
Both diagnostics and the initial curation script remain in the packet.

PR 973 subsequently advanced with the separately qualified
[runtime owner-slot change](native-inline-owner-slot.md). Merge
`2a0c025994100e27dc37b38df4a0f038a29a9481` preserves both changes. Its fresh
80.01-second compiler build is excluded. A separate integration compile takes
**39.958 seconds** within compiler timers, **41.124 seconds** for the process,
including **7.662 seconds** frontend time, and preserves all four outputs.
This predesignated output gate is retained separately from the balanced cohort.

The [compact result](../../benchmarks/mech-deflate/native-profile/frontend-affine-compile-apple-m3-20261113.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/frontend-affine-compile-apple-m3-20261113.archive.json)
pin the [public raw evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/level12-frontend-affine-compile-apple-m3-20261113.tar.gz):
223 files, 7,365,669 compressed bytes, SHA-256
`dfb488755f5ea522239c4bb44dfe3ad263b0191ee68776a2e2137177f34effa8`.
Independent public retrieval verifies the manifest and reproduces the compact
result byte-for-byte with `python3 replay.py`. Compiler/test executables stay
local with their hashes; program Object and metadata remain retrievable.

The [preceding numeric-facts change](native-numeric-facts-compile-profile.md)
reduced native analysis. Compilation still takes about 39–40 seconds;
the sub-10-second goal remains open.
