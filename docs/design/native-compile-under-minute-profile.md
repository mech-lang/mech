# Level-12 compilation below one minute on Apple M3

Fresh compilation of the current #973 compressor takes **58.188 s and 57.933 s**,
including parsing, checked source compilation, canonical bytecode encoding and
native Object emission. Whole-process times are **59.405 s and 58.168 s**. Both
runs reproduce the currently qualified compression Object byte for byte.

With default compiler settings, the same source takes **55.704 s**, down from
the preceding packed-liveness baseline's **143.598 s**: **61.2% less elapsed**.
The default and qualified configurations are separate comparisons. These are
individual fresh compiles, not a statistical timing campaign.

The tested source is `64eda4ca6330e7a8112ab27409acc2fc7816ab39`, including
public #973 through `4ab19cd765`. The compressor remains the pinned
160,857-byte source with SHA-256
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
Apple M3 MacBook Air, eight cores, 16 GB, macOS 26.2; Rust
nightly-2026-03-03 and the repository's vendored backend. The release profiling
tool is built separately. Application linking and compression execution are
outside these source-to-Object measurements. No Rust build or test overlaps a
measured source compile.

## Work removed

Guard closure retains synchronous inference order while filtering facts already
known in that round. Removing facts or relations preserves an existing closure
only when its dependencies remain valid. An immutable relation index can be
shared; path-specific truths remain local. Old-owner queries first exclude CFG
regions with no possible read of that old owner. The original guarded traversal
and conservative refusal behavior remain. The measured guard/owner interval
falls from **47.045 s to 0.709 s** in the default comparison.

Dimension identity validation and source remapping avoid repeated set and map
work, with explicit sparse/high-ID handling and original first-error order.
Numeric liveness uses query-local dense ordinals and bounded packed storage,
falling back to sparse words when its memory bound would be exceeded. SSA cycle
detection uses generations, and variable rows use occupancy-bounded dense
prefixes with sparse high IDs. Boolean target proofs, read membership and CFG
reachability reuse immutable inventories within their original validity scope.

Numeric candidate derivation shares the checked facts for its exact immutable
plan. If tree discovery fails, preparation reuses the already completed leaf
proof. Independent image validation still reconstructs its own facts. The final
same-configuration control takes **63.699 s**; the mean of the two accepted
compiles is **58.060 s**, **8.85% less elapsed**. No facts are reused across a
changed plan, separate compilation or runtime invocation.

| Configuration and run | Source → Object | Frontend | Native | Preparation | Guard / owner interval |
| --- | ---: | ---: | ---: | ---: | ---: |
| Default, packed-liveness baseline | 143.598 s | 23.992 s | 109.602 s | 24.760 s | 47.045 s |
| Default, final | 55.704 s | 9.176 s | 36.341 s | 13.893 s | 0.709 s |
| Qualified flags, final-step control | 63.699 s | 8.429 s | 45.143 s | 20.240 s | 0.710 s |
| Qualified flags, final | 58.188 s | 8.887 s | 39.242 s | 14.161 s | 0.716 s |
| Qualified flags, confirmation | 57.933 s | 8.406 s | 39.393 s | 13.993 s | 0.722 s |

The qualified configuration uses the 50 flags sealed with the current worker's
qualification. Default runs remove all inherited `MECH_` variables. Intervals
include surrounding work and are not exclusive function timers. Peak RSS
remains approximately **7.4–7.6 GB** in the final qualified runs.

## Program performance and qualification

Source, canonical bytecode, Object and metadata agree exactly before and after
within each configuration. The qualified Object is
`cf5e9f1e4de43a7b3211f0def6fac0f0d0e0cdc2111efb14e04ec1876e4fd378`,
also the hash independently retrieved from the
[current worker qualification](native-second-private-write-qualification.md).
Default output remains
`2a5cf7742a71959b288b40a7f1e5dfc3ca1a2b7502dee4304329fce1508896ff`.
Runtime code and its metadata are unchanged by these analysis optimizations.
The 81-case compression timing campaign was not rerun, and no runtime speedup
is claimed.

All **56 focused release tests** pass with warnings as errors: 26 guard, owner
read, reachability and Boolean predecessor tests; 10 liveness tests; 14 SSA
tests; three dimension-remap tests; and three old-value-read tests. Tests cover
independent traversal oracles, exact inference order, joins, redefinition,
cycles, high IDs, bounded storage, sparse fallback and cache independence.
Five Core dimension tests and 22 dimension/schema contracts pass. Seven fresh
scalar/source controls reproduce source, bytecode, Object and metadata exactly.
The artifact-review tool confirms fresh source compilation matches the pinned
bytecode and decoded re-encoding. Rust formatting, whitespace checks and the
five selected architecture/catalog checks pass. Full repository CI was not rerun.

Rejected attempts remain in the archive. In particular, an ASCII literal-parser
shortcut changed CRLF streaming diagnostics and was removed; all **484 parser
tests** pass on the restored parser. Sparse guard words, a ranked guard pass,
SSA waypoints, an SSA last-lookup cache and dense old-read-site masks did not
establish an elapsed benefit and are absent. The ranked owner traversal was
replaced by the exact-order read filter.

## Evidence

The [compact comparison](../../benchmarks/mech-deflate/native-profile/compile-under-minute-apple-m3-20261113.json)
and [archive receipt](../../benchmarks/mech-deflate/native-profile/compile-under-minute-apple-m3-20261113.archive.json)
pin [all raw evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/level12-compile-under-minute-apple-m3-20261113.tar.gz):
722 files, 27,188,938 compressed bytes, SHA-256
`fcedbd7daacb96cd17526cccd6c89dc8edb53573d30abfbdaeef9ed45a2684cc`.
Source snapshots and patches, all accepted and rejected captures, commands,
tests, output hashes and an offline replay are retained. Compiler executables
are excluded. Independent public retrieval verifies every file before replay.

```sh
python3 benchmarks/mech-deflate/experiment.py fetch --receipt benchmarks/mech-deflate/native-profile/compile-under-minute-apple-m3-20261113.archive.json --output /tmp/level12-under-minute-download
python3 benchmarks/mech-deflate/experiment.py verify /tmp/level12-under-minute-download/level12-compile-under-minute-apple-m3-20261113.tar.gz --receipt benchmarks/mech-deflate/native-profile/compile-under-minute-apple-m3-20261113.archive.json --extract /tmp/level12-under-minute-replay
python3 /tmp/level12-under-minute-replay/replay.py
```
