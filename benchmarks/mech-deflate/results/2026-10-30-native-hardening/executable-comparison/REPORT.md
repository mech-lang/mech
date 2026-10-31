# Mech and Roc executable comparison

For the same complete eight-byte level-12 compression, current Mech takes
**15.261× Roc's time**, executes **36.294× the instructions**, and makes
**147.305× the allocation requests**. Most of the visible opportunity is in
Mech's runtime bookkeeping and buffer ownership. Neither implementation streams
input in this experiment.

The latest retained compiler change moves immutable record-name hashing into
preparation. It reduces matched elapsed time **0.909%**, from **4.3892295 to
4.3493335 ms**. All four independent preparations improve; 54/64 descriptive
blocks improve. It increases diagnostic instructions **1.046%** and prepared
tracked heap **1,110,160 bytes**, while compression allocations and additional
peak tracked heap stay unchanged. The table makes these tradeoffs explicit.

## Measurements per compression

Roc is 1× in each row. MB means decimal million bytes. Elapsed, hardware counters,
allocation tracking and sampling are separate cohorts. Instrumented elapsed
measurements are excluded from the primary time result.

| Metric | Roc | Previous Mech | Current Mech | Current / Roc |
| --- | ---: | ---: | ---: | ---: |
| Elapsed | 0.285 ms | 4.3892295 ms | **4.3493335 ms** | **15.261×** |
| Hardware instructions | 1,364,372 | 49,006,012.5 | **49,518,676** | **36.294×** |
| CPU cycles | 928,313 | 13,832,994 | **13,775,388.5** | **14.839×** |
| Allocation requests, including realloc | 95 | 13,994 | **13,994** | **147.305×** |
| Requested allocation bytes | 9.022 MB | 98.587 MB | **98.587 MB** | **10.927×** |
| Additional peak tracked heap above entry | 9.257 MB | 20.575 MB | **20.575 MB** | **2.223×** |
| Total tracked peak heap, including preparation retained at entry | 9.257 MB | 162.171 MB | **163.282 MB** | **17.638×** |
| Reallocation calls | 0 | 1,213 | **1,213** | — |
| Relocating reallocation calls | 0 | 458 | **458** | — |

Primary time has 1,984 calls/version across four independently prepared pairs,
16 alternating-order blocks/pair, 31 calls/block/version, with one shared fresh
20,000-call reference. The fixed historical 0.300 ms ruler is also preserved:
**14.631× → 14.498×**. The fresh shared reference is used for the table above.
The baseline and candidate share exact work **60,260,219,686**.

Hardware medians use 32 steady calls/implementation; allocation medians use
eight Mech and 32 reference calls. Half units come from even-count medians.
Boundary-read overhead remains included, approximately 5,010 instructions.
No hardware-counter observation incurred a page-in.

There are **48.154 million extra instructions per compression**. Mech retires
about 3.595 instructions/cycle here versus Roc's 1.470, so an instruction-count
reduction does not translate directly into the same elapsed-time reduction.
Primary uninstrumented time is the latency measure.

## Memory of the prepared executable

| Metric | Roc | Previous Mech | Current Mech | Current / Roc |
| --- | ---: | ---: | ---: | ---: |
| Tracked live heap at compression entry | 32 bytes | 141.596 MB | **142.706 MB** | different retained state |
| Kernel physical footprint after compression | 14.435 MB | 228.296 MB | **228.411 MB** | **15.823×** |
| Resident-set size after compression | 14.795 MB | 319.275 MB | **319.390 MB** | **21.588×** |

Tracked heap, physical footprint and RSS are different accounting views and must
not be added. Tracked heap excludes allocator-private heaps, direct VM and
transient overlap inside a relocating realloc. Whole-task metrics include
compiler/catalog/preparation retained by this harness; they do not establish
the size of a minimal runtime-only Mech application. The prior multi-gigabyte
full-JIT configuration is a separate setup.

The extra 1.110 MB of current prepared heap is the measured cost of the newly
prepared immutable selector metadata. It is retained outside compression's
allocation boundary. This is a time/memory tradeoff, not a memory improvement.

## Buffer reuse and copying

| Large backing requests during one compression | Roc | Mech | Reduction to investigate |
| --- | ---: | ---: | --- |
| U64 path capacity, about 2.44 MB each | 1 | **35** | 34 additional capacity requests |
| U16 tree, about 0.524 MB each | 1 | **5** | four whole-tree fallback copies |
| U32 cache, about 6.00 MB each | 1 | **1** | already one backing |

The 34 extra path requests reserve **83,029,904 bytes of capacity**. They copy
only initialized elements; 83 MB is not demonstrated data copying. The retained
ownership census identifies 32 later appends without a usable consume proof and
two clears retaining private-frame aliases. Reuse requires proving those aliases
have no future read. The four tree fallback copies contain **2,097,152 initialized
bytes** in total. Current allocation histograms agree with these unchanged counts.

Both programs receive the complete input already loaded in memory. Mech's
immutable input handles share frozen backing. Every observation verifies retained
input aliases and contents. The source's stored-block payload append is preserved.
Total input-copy bytes and total bytes copied across all runtime operations remain
unmeasured; allocation capacity is not a substitute.

## Where the sampled work goes

Fresh isolated samples use the actual unchanged primary executables after all
other workers finish. The current Mech main thread has 1,048 weights:

| Disjoint sample ancestry | Weights | Share |
| --- | ---: | ---: |
| Function-frame actions | 268 | **25.57%** |
| Ordinary control visits | 148 | **14.12%** |
| Other invocation helpers | 153 | **14.60%** |
| Selected library calls | 108 | **10.31%** |
| Remaining generated/runtime/harness work | 371 | **35.40%** |

These samples include surrounding harness work. They locate costs, not exact
fractions of the compression timer or guaranteed removable time. A retained
full-compressor census records 12,450 admissions and 8,836 helper visits. The
previous callback-batching experiment was neutral and was restored; these
weights do not turn it into an established optimization.

Roc's retained sample has 1,416/1,513 weights in its hot compression function.
Inspected offsets show direct paired stores in tree/cache initialization loops.
Mech still crosses many frame/admission boundaries. Scalar arithmetic itself
continues to use emitted native instructions.

## Generated machine code

The current generated Object is byte-identical to the previously disassembled
Object, SHA-256 `3f0c0e1954afe1083254d928a8ff66977ee9203e64bfd281326c75f9ff5f09e9`.
The immutable-name optimization changes host runtime code and prepared metadata.
The unchanged generated entry requires **108,400 stack bytes**.

| Static metric | Roc whole executable text | Mech generated compressor entry |
| --- | ---: | ---: |
| Text bytes | 195,456 | **13,885,696** |
| Four-byte instruction sites, including padding | 48,864 | **3,471,424** |
| Call sites | 2,212 | **55,450** |
| Conditional branch sites | 5,389 | **110,409** |
| Unconditional branch sites | 1,560 | **225,522** |
| Load sites | 8,182 | **737,401** |
| Store sites | 7,713 | **402,158** |

Mech's generated entry alone is **71.043×** the reference's whole text size.
Scopes differ: Mech's shared runtime is excluded; Roc's whole text is included.
Unreached paths, errors and padding are included. These are static sites, not
executed branch/load counts. They establish code expansion, but do not establish
an instruction-cache bottleneck. Disassembly covers both text sections exactly,
with no unrecognized tokens.

## Targets and the numbers that should fall

1. **Frame and argument handling:** use the existing compiler proofs to remove
   repeated construction, admission and helper boundaries. Measure hardware
   instructions, reached callbacks and elapsed time together. Preserve every
   finite charge, error order, alias and failure/retry guarantee.
2. **Buffer ownership:** prove dead aliases before reusing path/tree storage.
   Track the 35/5/1 backing counts, initialized bytes copied, allocation requests
   and additional peak heap separately. A prior capacity-only reduction did not
   improve latency; reserve bytes alone are a poor predictor of speed.
3. **Prepared state and generated code:** identify duplicated immutable metadata
   and repeated cold/control paths. Track prepared heap, footprint, generated
   text and stack. Any proposed reduction still needs a matched elapsed result.

No matched branch-miss, cache-miss, bandwidth, energy or total-copy-byte count
is available. These remain unmeasured. No larger-input scaling estimate follows
from this eight-byte case.

## Build and measurement scopes

Previous compiler source: `552e537fcbcfc3811ce442c982010b5e1e999e6c`.
Current implementation: `326576c01541820f3f98839880b90ba4e1e4edb4`.
Release source head: `b8319ff83da250a49e9cee81acb91ba1da5c6a25`.
Roc's recorded compiler revision is `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`,
package `671077457ff7ef3a3d44caf715a3fc595addf379`, executable SHA-256
`3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`.

The CPU/memory cohorts use matched, optimized coherent Compute/GPU rebuilds
with the original frozen Engine libraries and fresh same-build Object/images.
Both Objects match their primary Objects byte-for-byte. The earlier eight-library
freeze omitted a transitive Compute artifact later overwritten in the shared
build directory; the diagnostic rebuild configuration is recorded explicitly.
These counter values are not claimed to come from relinking the original frozen
primary graph. Primary binaries and elapsed observations remained untouched.
An attempted after-only original-graph counter build failed before any execution.

All measurements use the same M1 Mac mini, pinned mimalloc 2.1.7, unchanged source
and bytecode, complete level-12 compression of `abcdefgh`, the real 262144 U16
tree, 1500513 U32 cache, 305257 U64 path capacity, depth 300, nice 258 and original
near-EOF limits. Every Mech call checks exact ten raw bytes, independent inflate,
output capacity 13 with clear/reuse, input aliases, exact work and zero final
managed bytes/nodes. Algorithms, allocation sites and timing boundaries are
unchanged. No build, test, audit, profile or diagnostic overlaps primary time.

`python3 compare.py OUTPUT_DIRECTORY` regenerates the JSON/CSV without executing
a benchmark. The complete current scorecard, archived observations, commands,
profiles, qualification and independent reconstruction are in
`../compression-round-42/`. `inspect_static.py` preserves the original exact
disassembler commands, hashes and mnemonic histograms.
