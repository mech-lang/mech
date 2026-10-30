# Native pieces around ordinary scalar child calls

Retain the original selection change: **4.4977705→4.3775000 ms**, or
**15.56322×→15.14706× Roc**, a **2.67400% reduction** in a new matched linked
configuration. All four independently prepared triplets and all 64 descriptive
blocks improve. Actual instructions fall **50,690,556→49,009,728 (3.31586%)**.
Parity remains unachieved; the target is 1×. Only complete eight-byte level-12
compression is timed, with no larger-input throughput claim.

## Exact state and implementation

Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
Starting published head: `369583114cdc10b60372eebaff4b63e3a2721fb6`.
Frozen baseline compiler: `dc77024f29f373f0938695531ab2b7c975bb1e7e`.
Regression first: `f2a68c8431ff55f484763ebba1fe52510078a457`; actual
frozen-compiler rejection: `a27963b492972929073df71dae6675e6b50804c2`.
Measured retained implementation:
`552e537fcbcfc3811ce442c982010b5e1e999e6c`.
The transport-cost experiment `892275fec5b462ca76f0ba47a34dfefb6096c6bc` is
removed by `21b20630b88f0321b025eec8784f25ad24813202`; final production source
is byte-identical to the retained implementation. The separate CI inventory
commit is `e47ab660f6cdbb8fd0743e19411c9d87da9a5643`.

The real optimal-path scan calls OptimalPathMatches, which contains nested
OptimalPathLengths. The existing parent-piece selector required a qualified
array-returning leaf and skipped this parent entirely. Reuse its existing
acyclic native pieces around an ordinary child when the connected scalar proof
covers the child's flat return record and all its descendant loop carriers are
private scalar aggregates with no buffer-frame carrier. Descendants cannot then
select another private-frame parent edge, so the existing single pending edge
remains sufficient. Multiple children, collection loops, unknown/mixed descendant
state and the existing append-child route retain their previous refusal.

The initializer, complete ordinary child/grandchild execution, assertions, exit
and successor remain authoritative. Each piece closes before the child, with no
owner, view or fuel crossing it, and obtains current returned values afterward.
Compilation and build-owned image restoration reuse the same derivation and the
existing scalar-return predicate. There is no second executor/owner/word bank,
source specialization, new operation, price, allowance or protocol. IR/work/ABI/
bytecode remain **131/95/37/14**. `CANDIDATE.md` explains the proof.

Source SHA stays
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
bytecode SHA stays
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
The pinned mimalloc 2.1.7 object SHA remains
`38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.
Reference binary SHA remains
`3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`,
package revision `671077457ff7ef3a3d44caf715a3fc595addf379`, original compiler
revision `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`. Earlier reference-source
verification remains in `../deep-profile/`; removed checkout Git metadata is not
presented as freshly queried. Algorithm, tables, depth 300, nice 258, near-EOF
limits, real 262144 U16 tree/1500513 U32 cache/305257 U64 path capacity, source
allocation sites, output capacity/reuse and compression timer are unchanged.

## Measurements and the retained small gain

Early sequential JIT checks were inconclusive: the initial candidate first
appeared 2.75225% slower, then faster on repeat. Keep both unchanged receipts.
All three versions then move to the SAME linked configuration, using their own
pinned Object, descriptor, identity and libraries. Bodies normalize identically
except binding paths/symbols. The full compressor is linked ahead of time; the
same two tiny seed/clear helpers retain JIT outside compression. This keeps three
large full-JIT preparations out of the workers and allows a resident triplet.
**No JIT-to-linked speed gain is claimed.** Establish a fresh linked baseline.
See the preregistered `MEASUREMENT.md`.

| Complete eight-byte primary median | Time | Shared 0.289 ms reference = 1× | Fixed historical 0.300 ms ruler |
| --- | ---: | ---: | ---: |
| Baseline | 4.4977705 ms | 15.56322× | 14.99257× |
| Initial, retained | 4.3775000 ms | 15.14706× | 14.59167× |
| Transport-cost experiment | 4.3849585 ms | 15.17287× | 14.61653× |

Four triplets × 16 rotating-order blocks × 31 calls per version give **5952
primary calls**, 5976 including warm-up/final, with one shared 20,000-call
reference. All three are ready before timing; only the requested worker runs.
No build/test/audit/profile/diagnostic overlaps primary. Blocks share four
preparations and are not 64 independent executable preparations. Each cohort is
committed immediately. Every call checks the same ten raw bytes, independent
inflation, unchanged input contents/storage aliases, capacity 13 and stable work;
every worker finishes with zero managed bytes/nodes.

The retained candidate improves by 2.96512%, 2.60832%, 2.64388% and 2.44462%
across the four triplets. The experiment also beats baseline in all four, but
versus the initial candidate it is mixed (two improving/two regressing, 24/64
improving blocks), 0.17038% slower pooled, and executes 65,084.5 more instructions.
It has no established additional gain. Remove that heuristic and retain the
original broader selection and its proven small improvement. `REFINEMENT.md`
and every intermediate result remain available; its motivation was a preliminary
hypothesis, not a confirmed regression of the original change.

## Actual counters, allocation and code size

These are separate diagnostic cohorts in the SAME linked configuration, excluded
from primary elapsed comparisons. CPU uses 32 steady calls per executable;
memory eight steady Mech/32 reference observations. Hardware counters include the
recorded empty-getter calibration without silent subtraction. All retained CPU
observations have zero page-ins. MB means decimal.
Instructions and cycles are compression-boundary differences of macOS
`proc_pid_rusage(RUSAGE_INFO_V4)` counters, not static disassembly counts or
Mech's logical work receipt. Empty-boundary calibration is retained separately.

| Per compression | Roc | Baseline | Retained | Retained/Roc |
| --- | ---: | ---: | ---: | ---: |
| Actual CPU instructions | 1,364,372 | 50,690,556 | 49,009,728 | 35.92109× |
| CPU cycles, diagnostic | 886,670.5 | 14,166,741 | 14,004,336.5 | 15.79430× |
| Allocation requests | 95 | 13,994 | 13,994 | 147.30526× |
| Requested bytes | 9,021,923 | 98,586,819 | 98,586,843 | 10.92748× |
| Additional peak live usable heap | 9,257,328 | 20,575,432 | 20,575,432 | 2.22261× |

Requests and additional peak are unchanged; requested bytes rise by 24 fixed
bytes. Large backing histograms stay **35 path / five tree / one cache**.
This round does not eliminate those remaining copies. Logical work instead rises
44,443,824,446→60,260,219,686 under unchanged conservative entry/loan prices;
those units are not retired CPU instructions. The native entry inventory creates
that logical overhead while reducing actual callbacks and executed work.

Whole-task linked footprint is 227,936,000→228,034,240 bytes and RSS
318,881,792→319,012,864 bytes. Tracked prepared live heap at entry is
141,537,520→141,595,920 bytes. Additional peak above that baseline excludes direct
VM/private heaps and transient internal realloc overlap; it is not exact total
RAM. The reference whole-task footprint is 14,418,816 bytes and RSS 14,778,368
bytes, versus retained Mech's 228,034,240 and 319,012,864. Compiler/catalog/setup
state remains in the Mech diagnostic, so these are measured prepared executables,
not minimal runtime-only applications. The earlier full-JIT ~16.9 GB footprint
is not compressor-only memory. Both receive already loaded whole input and
neither streams file input during the timer. Input aliases remain unchanged.
No total-copy-byte, cache/branch-miss, bandwidth, energy or input-scaling result
is claimed. Requested capacity is not initialized payload copied.

Generated compressor text grows 13,801,616→13,885,696 bytes; checked stack falls
108,560→108,400 bytes. Static slots include padding and are not executed
instructions. Reference whole executable text stays 195,456 bytes; comparing it
to the Mech entry is a scope bound, not a function-matched/minimal-application
size comparison. The diagnostic binaries retain compiler/catalog/setup code.

## Mechanism, validation and remaining work

The actual complete-compressor census changes admissions **13678→12450** and
helpers **9156→8836**. Reached path parents 136/170/300 change 606/1212/606
admissions to 299/598/299, with unchanged child/grandchild visits. There are 232
additional successful native piece entries. Static emitted pieces grow 39→67.
No source rewriting or function-name specialization is involved.

Fresh matched linked profiles use 695 calls each, entirely excluded from primary.
Baseline has 1115 sample weights; retained has 1160. Retained disjoint ancestry
weights are frame actions 322, ordinary control 154, other helpers 147, selected
library calls 99, remaining 438. These include harness/native work and are not
precise timer percentages or paired samples. Frame handling remains about 28%
of weights and is the next major transport target. Managed projections still
split otherwise numeric prefixes into multiple pieces; any expansion must have
its real-operation reproducer and failure/alias/resource acceptance tests first.

The retained reference profile/disassembly in `../deep-profile/REPORT.md`
identifies direct paired stores in tree/cache initialization. Mech still reaches
12,450 admissions and 8,836 helpers for this same complete compression. This
supports simplifying repeated runtime transport and boundaries around native
work. It does not establish that removing all allocation capacity, copying or
checks would deliver a proportional elapsed-time gain. The unchanged real
buffer requirements remain part of the test.

The final selected qualification reruns against the frozen retained libraries,
not merely the discarded heuristic. There are **587 distinct selected tests**:
313 runtime, 31 work/memory/value, 234 unchanged component-selection cases, eight
composition cases and the new real path-selection mechanism case. The new
Boolean/current-argument and six-dimension exact/one-short tests are included in
the runtime count, not counted twice. Original/decoded exact pinned nodes,
real sizes/capacity, assertions, error precedence, publication, aliases, cleanup
and retry remain checked. No prior expected value or allowance changes.

Release builds take 75.989 s and 77.587 s, reusing Engine/math dependencies:
all seven non-host library hashes remain identical. Compiler is rustc nightly
2026-03-03 (`ec818fda361ca216eb186f5cf45131bd9c776bb4`), LLVM 22.1.0,
aarch64-apple-darwin. Release libraries use opt 3; maintained qualification roots
opt 1; warnings are errors, incremental off, locked/offline, one build job.
Exact commands/environments/profiles/hashes/counts/stdout/stderr are archived.
Twelve scalar controls retain the unchanged 1.05 ceiling (largest final ratio
1.0140548) and zero allocation slopes; eight managed-frame controls retain
requests/bytes/work and zero slopes.
Ten architecture/catalog/factory/warning/unsafe/format checks pass, with final
format/diff checks repeated after removing the heuristic.

The CI inventory guard is separately updated 13→15 for the two added tests;
the actual Rust selection lists and executes 15 cases. The unchanged configured
workflow/inventory Python suite passes all 132 cases. System Python's missing
YAML dependency is environmental and is resolved by the existing configured CI
environment. The initial new fixture's row/general-matrix and recursive schema
errors, and a source-check adapter's `&String`/`&str` error, are introduced
diagnostic setup failures; corrected fixtures/checks pass and failures remain
archived. No benchmark source or unrelated parser change addresses them.
The final control launch was initially denied access to `ps` by the restricted
environment before any observation ran. The same command with process visibility
passed; the check receipts and disposition are retained in `final-checks/`.
An initial receipt commit was also denied worktree-index write access; its
already-created archive was committed successfully with the required access.

The unchanged reduced-catalog knucleotide case is excluded. Full Core/Engine/
all-feature suites, unrelated targets, a new linked assertion-recovery suite and
larger full-compressor inputs are not newly claimed. All three actual linked
compressors execute exact outputs and cleanup across primary/counter/profile
calls. No unresolved introduced correctness failure is known. Broader coverage,
remaining runtime overhead and parity remain outstanding. PR #973 stays draft
pending green CI and clean review; hosted CI is cancelled by user direction and
is not reported as passing. A new review is requested on publication.

`analyze.py`, `reconstruct.py`, `scorecard.json` and `scorecard.csv` preserve a
recomputable ledger for all three variants, including the rejected experiment.
Independent extraction verifies **55 archives, 616 file occurrences and 526
unique paths**, checks every stored length/hash, and regenerates both scorecards
byte-for-byte. Repeated scripts are verified within each original archive before
extraction, preserving their recorded successive versions. Run
`python3 reconstruct.py EMPTY_SCRATCH_DIRECTORY`; this performs no compilation,
network access or new benchmark observation.
