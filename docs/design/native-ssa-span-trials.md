# Rejected local-span SSA definition storage

Both attempts to accelerate private SSA variable-definition storage made level-12
compilation slower and were rejected. The wider record increases source CFG
construction **5.53%**, compiler phases **0.61%** and retired instructions **0.55%**.
Restoring the original record size still increases source CFG construction
**5.07%**, compiler phases **0.94%** and instructions **0.48%**. Both balanced
compiler-time blocks are slower for each candidate. Production source is restored
exactly to the retained nested-parser compiler; no change from these trials ships.

The retained compiler's prior qualified complete-process range remains
**18.467–18.483 seconds**. The new control observations below belong to their own
matched trials, not a pooled replacement baseline. The sub-ten-second goal remains
open.

## Experiment and matched measurements

The previous finite CPU windows showed work in SSA nonlocal lookup, definition
insertion and sparse-map growth. The existing dense storage begins at block zero.
The hypothesis was that a dense span anchored at a variable's local block IDs
could reduce hashing for late, clustered definitions while preserving every
lookup, pending-None state and ordered enumeration.

Only private `VariableDefs` storage and focused tests changed. Promotion still
requires at least 64 facts, dense capacity is bounded by four times the fact
count, leftward growth is geometric, and sparse entries below/above the span
remain ordered. All SSA traversal, cycle handling and phi-construction code is
byte-identical to the control. The first variant adds an `usize` base, growing
the private record from 64 to 72 bytes. The second uses compact 32-bit base/count
metadata and unsigned offset arithmetic, returning the record to 64 bytes.
The count saturates for the promotion budget; it does not loosen the capacity
bound. Both complete candidate sources and measured patches are archived.

Each variant has its own eight serial complete-source observations: ABBA then
BAAB, four control and four candidate. Every process compiles the full level-12
source through Object and complete metadata with the same 60 runtime flags,
locked dependencies and opt-in `compiler` profile (Thin LTO, one codegen unit).
Builds, tests, IR/layout probes, fixture checks, hashing, copying and archive work
do not overlap primary timing. Screens remain separate. Arithmetic means follow;
phase and nested-stage intervals overlap and must not be added. Positive changes
mean more time/work/memory. These small cohorts do not establish population
estimates or isolate individual causes.

### Wider record

| Measurement | Control mean | Candidate mean | Change |
| --- | ---: | ---: | ---: |
| Complete process | 18.4796 s | 18.5332 s | +0.2901% |
| Compiler phases | 18.2858 s | 18.3981 s | +0.6139% |
| Parsing | 2.0941 s | 2.0895 s | -0.2213% |
| Catalog | 0.0375 s | 0.0356 s | -5.1545% |
| Frontend | 2.5073 s | 2.5050 s | -0.0885% |
| Artifact | 0.0257 s | 0.0264 s | +2.5693% |
| Bytecode encoding | 0.0092 s | 0.0092 s | +0.3595% |
| Native | 13.6112 s | 13.7317 s | +0.8854% |
| Canonical lowering | 0.0982 s | 0.0995 s | +1.2926% |
| Preparation | 2.9928 s | 3.0176 s | +0.8264% |
| Numeric helper generation | 2.1287 s | 2.1226 s | -0.2845% |
| Source CFG construction | 2.3218 s | 2.4501 s | +5.5252% |
| Cleanup including compaction | 3.4178 s | 3.3550 s | -1.8370% |
| CFG cleanup | 3.2503 s | 3.1893 s | -1.8751% |
| Value compaction | 0.1675 s | 0.1657 s | -1.0971% |
| Retired instructions | 270.7365 G | 272.2183 G | +0.5473% |
| Maximum RSS | 5.9169 GB | 5.9109 GB | -0.1018% |
| Peak footprint | 5.8864 GB | 5.8804 GB | -0.1027% |

The two balanced compiler-time block changes are +0.0814%, +1.1477%.

### Compact record

| Measurement | Control mean | Candidate mean | Change |
| --- | ---: | ---: | ---: |
| Complete process | 18.4177 s | 18.5819 s | +0.8919% |
| Compiler phases | 18.2199 s | 18.3914 s | +0.9415% |
| Parsing | 2.0845 s | 2.0848 s | +0.0129% |
| Catalog | 0.0361 s | 0.0359 s | -0.5318% |
| Frontend | 2.4487 s | 2.4625 s | +0.5627% |
| Artifact | 0.0257 s | 0.0259 s | +0.7748% |
| Bytecode encoding | 0.0092 s | 0.0092 s | -0.3664% |
| Native | 13.6143 s | 13.7719 s | +1.1574% |
| Canonical lowering | 0.0975 s | 0.0986 s | +1.1810% |
| Preparation | 2.9914 s | 2.9891 s | -0.0755% |
| Numeric helper generation | 2.1294 s | 2.1338 s | +0.2078% |
| Source CFG construction | 2.3331 s | 2.4515 s | +5.0737% |
| Cleanup including compaction | 3.4086 s | 3.4189 s | +0.3011% |
| CFG cleanup | 3.2452 s | 3.2544 s | +0.2830% |
| Value compaction | 0.1634 s | 0.1645 s | +0.6618% |
| Retired instructions | 269.9193 G | 271.2073 G | +0.4772% |
| Maximum RSS | 5.9122 GB | 5.9117 GB | -0.0077% |
| Peak footprint | 5.8817 GB | 5.8812 GB | -0.0079% |

The two balanced compiler-time block changes are +0.7108%, +1.1713%.

The wider variant improves cleanup time and slightly reduces measured memory,
but those observations do not offset its construction/instruction regressions.
The compact variant also slightly reduces measured memory and preparation time.
All observations, including unrelated-phase movement, remain in the result.
Neither candidate is credited with an optimization.

Separate warm-cache compiler-executable rebuilds take **272.601 seconds** and
**274.722 seconds**, respectively. These are excluded from Mech compilation and
are not a matched cold-bootstrap comparison.

## Correctness and output checks

The wider variant passes 17 focused SSA tests; the compact variant passes 18,
including a record-size check. Existing randomized map-oracle tests and added
local/shifted/outlier, overwrite, descending-ID, near-maximum-ID, clone,
pending-None and ordered-enumeration cases pass. The bounded-capacity and
geometric-movement controls remain in the archived test source.

For each variant, 24 paired CFG cases exercise offsets 0, 4096 and 100000,
65/257-block graphs, loops and forward/reverse sealing. All 48 independently
verified functions preserve complete displayed IR, raw-value aliases and types,
post-seal query results, side-effect lists and ordered variable enumeration.
Both trials produce the same 815,750-byte comparison stream, SHA-256
`6dec1b5ad226bfcd74d928c0fd7942a9348609388cf0c935d414bd32226d9921`.

Every source, MBC, Object and complete metadata byte matches the retained program
in both screens and all primary runs. Fourteen successful control/candidate
fixture pairs per variant also match every file, including numeric phases,
nested/changing captures, array windows and Bool/U64/I64/F64 scalar cases.
Generated instructions and metadata are identical. There is no new compressor
runtime-latency campaign or CPU sampling campaign in these trials.

## What the source inspection shows

Shared word homes are declared and defined together in the entry block before
source CFG emission, including homes whose actual source definitions occur
later. Noninput/nonconstant homes receive a common zero backend SSA seed. The
emitter explicitly says that flow independently proves every reached source read
has an actual input, constant or definition. This common early definition weakens
the proposed benefit of spans anchored at high block IDs. No dynamic locality
histogram was collected; this is a source-based explanation, not isolated causal
attribution for the timing changes. Restoring record size alone did not restore
performance.

The next hypothesis is to defer unnecessary backend zero seeds. This is
unimplemented and has no credited saving. Inputs and actual constants must remain
initialized. `Words::build` currently interprets `None` as a fallback constant or
local-memory load, not an uninitialized SSA variable; simply returning `None`
would be incorrect for the compact emitter's null locals pointer. Any explicit
initialization policy must check padding/inactive lanes, boundaries, spills,
source-flow proofs, checked errors and the builder's lazy-zero contract. Changed
machine code requires runtime qualification. The previously rejected SSA
forwarding shortcut remains rejected. Grammar and language remain unchanged.

## Inspect and reproduce

The [compact result](../../benchmarks/mech-deflate/native-profile/ssa-span-trials-apple-m3-20261114.json),
[receipt](../../benchmarks/mech-deflate/native-profile/ssa-span-trials-apple-m3-20261114.archive.json)
and [packet index](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
include every raw observation, source snapshot, patch, command, output, test log,
IR/layout probe and executable hash. Executables are excluded. The retained base
is `8e3c03512b4df9ae479fe7f586d195166791b3c5`; the control producer is
`ee77e992e12f1880901c2e6b0745e23fcf7f1fe6`. Their production source is identical.
Candidates are identified by that base plus their separate measured patches.

The maintained archive verifier checks the published repository copy, manifest,
safe paths and every file. Pure offline `python3 replay.py` reconstructs each
candidate source from the retained base and patch, checks the stored tests,
complete IR, counters and output bytes, and recreates the compact result without
compilers, benchmark execution, original scratch paths or network. Replay checks
stored measurements; it does not rerun them.

The subsequent [word-home trials](native-word-home-compile-profile.md) test the
zero-seed hypothesis and direct home lookup. Neither establishes a useful saving,
and both are restored. Their complete measurements include a separate indexed
repeat, all output comparisons and both candidate source snapshots. The next
investigation measures exact liveness-query construction and read-set allocation.
