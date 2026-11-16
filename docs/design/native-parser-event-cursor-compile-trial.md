# Borrowed parser event-iterator trial

The borrowed event iterator is **rejected**. It reduces the measured parser
phase **1.869500 to 1.842458 seconds (-1.45%)**, about 27 ms, with both parser
blocks faster. Complete source-to-executable builds are **15.215777 versus
15.229858 seconds (+0.09%)**, with mixed blocks. The separate Object cohort's
total compiler time is effectively flat. This establishes a small parser work
reduction without a useful complete-build gain. The original source and
standard compiler are restored; the separately qualified Deflate-trained PGO
configuration remains 13.596783 seconds. Compilation below ten seconds remains
open.

## Frozen implementation

Base is `9c8bab9389fc3650a7058cb2764a73b615d6d6a3`, retaining production source
`0c949505cc`. The entire one-file trial patch is SHA-256
`3e4049488ae14f04a6fdbebef4d6f8264cfec9596b9d277d81dfe7b477daaaf2`.
It changes only `src/syntax/src/document/parser/tree_cache.rs`. Selection uses
the current-source stack diagnostics already published in packet 46; no new
sampling run is included in this trial.

`Sink::advance` originally looks up every event independently in the retained
journal tree. The candidate lazily borrows `Journal::iter_range` for the current
allowance slice. Sequential reads traverse existing pages through the iterator;
a cached-subtree jump discards the cursor and lazily seeks from the new event
index. The existing element helper takes separate mutable field borrows so
event storage can remain immutably borrowed. Its body is unchanged after that
field substitution.

The cursor adds no persistent fields, owned journal snapshots, Arc clones or
heap allocations. Current-journal cache publication and immutable marker-ID
checks remain in their original order. A token audit checks every other
production token, ignoring whitespace/comments and optional trailing argument
commas. Grammar, event ordering, node/token IDs, cached-node ownership, errors,
finite allowances and work/allocation accounting retain their implementation.
The cost or frequency of cursor initialization and reseeking is unmeasured;
the small gain is not assigned to one cause.

## Matched measurements

Standard control `6ce7a53b36` and candidate `5399819ceb` use ThinLTO, pinned
compiler-only mimalloc, the same 61 program flags and two workers on the Apple
M3. No native-CPU or PGO overrides are added. Eight complete builds run serially
in ABBA/BAAB order without overlapping heavy build, test, sampling, hashing or
archive work. Fresh source assembly through final link completion is measured.
Runtime/entry adapters and fixed native inputs are cached and hash-pinned;
historical first-use adapter construction of 11.882560208 seconds is excluded.

| Boundary | Control | Candidate | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 15.215777 s | 15.229858 s | +0.09% |
| Source-to-Object process within those builds | 15.079165 s | 15.098713 s | +0.13% |
| Final native link | 0.135272 s | 0.129974 s | -3.92% |

Complete-build blocks are 15.267956 to 15.303685 seconds (+0.234%) and
15.163598 to 15.156031 seconds (-0.050%). Object handoff falls 12.63%, about
0.169 ms. Link inputs and implementation are unchanged, so faster linking is
not credited to the parser.

A separate eight-run Object cohort uses the same configuration and order:

| Boundary | Control | Candidate | Change |
| --- | ---: | ---: | ---: |
| Internal compiler phases | 15.075401 s | 15.067536 s | -0.05% |
| Complete source-to-Object process | 15.144655 s | 15.135593 s | -0.06% |
| Parser | 1.869500 s | 1.842458 s | -1.45% |
| Source CFG construction | 2.261175 s | 2.280135 s | +0.84% |
| Cleanup including compaction | 3.030749 s | 3.014051 s | -0.55% |
| Value compaction | 0.148406 s | 0.152563 s | +2.80% |

Parser blocks improve 1.22% and 1.68%; total compiler blocks are mixed
(+0.79%, -0.89%). Instructions fall 221.013103 to 220.942866 billion (-0.0318%).
Frontend increases 0.29%, canonical lowering 0.46%, artifact writing 0.22% and
native phases 0.13%. Catalog work falls 1.53%, preparation 0.26%, helper
generation 0.17%, cleanup excluding compaction 0.72% and encoding 0.86%.
RSS and footprint are effectively unchanged (-0.0011%, -0.00009%). Every stage,
counter and adverse observation remains in the
[complete result](../../benchmarks/mech-deflate/native-profile/parser-event-cursor-apple-m3-20261115.json).
The cohorts remain separate; no new PGO or emitted-program runtime campaign is
run.

## Correctness and setup failures

All **526 production document-module unit tests** pass. All **1,780 complete
reference/candidate parse observations** match byte-for-byte: 1,667 existing
cases, the entire current Deflate source and 112 streaming cases. Observations
include full trees and IDs, token kinds/flags/hashes, diagnostics, parser stats,
restart data, streaming changes, poll counts and every stream-work field.
Streaming allowances cover 1, 2, 4, 13, 4,095, 4,096 and 4,097 units, with zero-
allowance probes and one/three input chunks. The full 164,243-byte Deflate parse
has zero diagnostics, 1,998,075 parser steps and 442,929 events in both versions.

Standalone probes compile the exact document-module snapshots and current
compiler feature list against the prior coherent, hash-pinned release parser
test libraries. Only `SourceLocation` records depend on `mech-core`. The full
production compiler check, build, native CLI check and program gates use the
current locked compiler dependencies.

All four outputs, **complete metadata**, 14 scalar/numeric fixture pairs and
every whole executable byte match. Two preliminary executable gates, default
serial output, one excluded real-worker diagnostic and all eight complete
primary builds reproduce the 2,060,440-byte executable, SHA-256
`d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
The worker diagnostic records 344 submitted jobs, including 48 cleanup jobs.
The six-stage driver order and helper paths are audited before execution and
during pure replay.

Compiler check/build take 21.050/378.538 seconds; native CLI check with
`--features cranelift_build` takes 24.559 seconds. Standalone unit build/test
take 11.315/22.436 seconds. All are outside program timings. The packet also
preserves these setup corrections:

- The first token audit failed on four optional trailing call commas introduced
  by formatting. Normalizing those commas on both sides makes the audit pass.
- A standalone unit link using compiler-profile LLVM 22 bitcode failed with the
  system LLVM 15 linker. The corrected coherent release-library recipe passes;
  the production compiler build had already passed.
- The first qualification wrapper omitted runtime flags and started no compiler.
  A partial setup correction then assumed a missing reference-pins file existed;
  the following command started one control screen before its missing-reference
  assertion. Both attempts are retained and excluded. Published pins, all four
  full reference files and 61 flags are verified in preflight before the complete
  authoritative qualification. No failed attempt enters a primary cohort.
- A restoration assertion used a mistyped PGO hash literal after source and
  standard compiler restoration had succeeded. The existing 64-character pin
  and an independent binary hash confirm the PGO artifact was never modified.

The production file and immutable standard `6ce7a53b36` are restored byte-for-
byte. Selected PGO `6fc6db8164` is untouched. Temporary Cargo compiler-profile
rlibs remain from the rejected parser candidate until the next genuine Cargo
rebuild; the restored source's fresh mtime forces affected crates to rebuild.

The forty-seventh
[repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
keeps both full syntax source snapshots, rejected patch, exact tests/audit and
probe recipes, every setup failure and command/log/observation/output, complete
executables, fixed native inputs and restoration receipt. Compiler/test products
are excluded and hash-pinned. Fresh repository extraction verifies all 1,472
files and Git blobs; pure replay reconstructs every summary and reproduces the
compact result byte-for-byte without rerunning timing.
