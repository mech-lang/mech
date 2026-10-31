# Retained Mech and Roc executable comparison

On the same complete eight-byte level-12 compression, the retained Mech
executable takes **15.08× Roc's elapsed time**, executes **35.92× the hardware
instructions**, and makes **147.31× the allocation requests**. The evidence
points to repeated function-frame handling and working-buffer ownership as
concrete targets. The cache is already allocated once.

This sheet consolidates the latest retained linked-executable measurements,
then adds a fresh static disassembly inventory. It does not claim a new timing
or a result for the pending projection-batching implementation. The previously
confirmed compiler gain remains 2.674% elapsed and 3.316% instructions in its
matched cohort. No positive gain was discarded in the subsequent round; both
subsequent candidates were measured regressions and were removed.

## Per-compression measurements

MB below means decimal million bytes. Each ratio uses the Roc value in that
row as 1×. Time, hardware counters, allocation tracking and sampling were
collected separately so instrumentation does not inflate the primary timer.

| Metric | Roc | Retained Mech | Mech / Roc |
| --- | ---: | ---: | ---: |
| Elapsed time | 0.291 ms | 4.387083 ms | **15.07589×** |
| Hardware instructions | 1,364,372 | 49,008,490.5 | **35.92018×** |
| CPU cycles | 935,346.5 | 13,808,872.5 | 14.76338× |
| Allocation requests, including realloc | 95 | 13,994 | **147.30526×** |
| Requested allocation bytes | 9,021,923 | 98,586,843 | **10.92748×** |
| Additional peak tracked live heap above entry | 9,257,328 | 20,575,432 | **2.22261×** |
| Tracked peak heap including preparation retained at entry | 9,257,360 | 162,171,352 | 17.51810× |
| Reallocation calls | 0 | 1,213 | no finite ratio |
| Relocating reallocation calls | 0 | 458 | no finite ratio |

The elapsed cohort contains 1,984 primary calls for each Mech variant across
four independently prepared triplets and a shared 20,000-call reference.
Only the retained baseline is shown here. CPU medians use 32 steady calls per
implementation; allocation medians use eight Mech and 32 reference calls.
The half instructions/cycles arise from medians of an even observation count.
The hardware counter boundary contributes approximately 5,010 instructions,
recorded without subtraction. No CPU observation incurred a page-in.

The instruction gap is **47.64 million extra instructions per compression**.
Mech retires more instructions per cycle in this cohort (3.549 versus 1.459),
so eliminating a percentage of instructions does not imply the same percentage
of elapsed improvement. CPU cycles also vary with scheduling and core state;
the uninstrumented primary timer remains the latency comparison.

## Memory scopes

The additional peak above entry answers how much extra tracked heap compression
requires. The total task also retains prepared runtime/compiler/catalog state:

| Prepared executable metric | Roc | Mech | Mech / Roc |
| --- | ---: | ---: | ---: |
| Tracked live heap at compression entry | 32 bytes | 141,595,920 bytes | different retained state |
| Kernel physical footprint after compression | 14,402,432 bytes | 227,755,776 bytes | 15.81370× |
| Resident-set size after compression | 14,761,984 bytes | 318,717,952 bytes | 21.59046× |

Footprint, RSS and tracked heap are different accounting views and must not be
added together. The tracked peak excludes allocator-private heaps, direct VM
and transient overlap inside a relocating realloc. Whole-task rows include
setup state; they do not establish the size of a minimal runtime-only Mech
application. The earlier full-JIT multi-gigabyte footprint is a different
configuration and is not the current linked compressor's memory requirement.

## Actual buffer reuse and copying

| Large backing requests during one compression | Roc | Mech | Investigation target |
| --- | ---: | ---: | --- |
| U64 path capacity, about 2.44 MB each | 1 | 35 | 34 additional capacity requests |
| U16 tree, about 0.524 MB each | 1 | 5 | four whole-tree fallback copies |
| U32 cache, about 6.00 MB each | 1 | 1 | already one backing |

The extra path requests total **83,029,904 requested capacity bytes**. They
reserve spare storage and copy only initialized elements; this is not 83 MB
of demonstrated copying. The ownership census distinguishes 32 later appends
without a usable consume proof and two clears retaining private-frame aliases.
Those aliases need a future-read proof before reuse is safe. The four tree
fallbacks do copy all initialized words: **2,097,152 bytes** in total.
The prior completed-call ownership fix already removed four path requests;
its exact receipts remain in `../compression-round-38/`.

Both programs receive the whole input already loaded in memory. Neither
streams file input during compression. Mech's immutable input handles share
the frozen backing; every observation checks that retained aliases and input
contents remain unchanged. The pinned source's stored-block payload append
is preserved. Total input-copy bytes and total bytes copied by all runtime
operations have not been measured; allocation capacity is not a substitute.

## Where the CPU samples go

The latest retained Mech sample has 1,301 main-thread weights, partitioned
without double counting:

| Sample ancestry | Weights | Share |
| --- | ---: | ---: |
| Function-frame actions | 340 | 26.13% |
| Ordinary control visits | 201 | 15.45% |
| Other invocation helpers | 190 | 14.60% |
| Selected library calls | 126 | 9.68% |
| Remaining generated/runtime/harness work | 444 | 34.13% |

The sample includes surrounding harness work, so these are not precise shares
of the compression timer or additive promises of speedup. A retained-compressor
census records **12,450 admissions and 8,836 helper visits**. The later projection
census observes 2,173 managed numeric-buffer field projections, 1,829 scalar
projections and 13 other projections. It identifies 67 same-source managed
runs with an upper bound of 706 per-field admissions/helper boundaries that
could be combined, subject to correctness and selection proofs.

Roc's retained reference sample has 1,416 of 1,513 weights in its hot compression
function. Inspected offsets include paired stores in direct tree/cache
initialization loops. This contrasts with Mech's many reached runtime
boundaries. It does not establish that every reference sample is initialization
or that all Mech bookkeeping can safely be removed.

## Machine-code inventory

A fresh disassembly counts every instruction site in the same retained Mech
Object and original reference executable. Both decode without unrecognized
tokens and exactly cover their text sections at four bytes per site.

| Static metric | Roc whole executable text | Mech generated compressor entry |
| --- | ---: | ---: |
| Text bytes | 195,456 | 13,885,696 |
| Four-byte instruction sites, including padding | 48,864 | 3,471,424 |
| Call instruction sites | 2,212 | 55,450 |
| Conditional branch sites | 5,389 | 110,409 |
| Unconditional branch sites | 1,560 | 225,522 |
| Load instruction sites | 8,182 | 737,401 |
| Store instruction sites | 7,713 | 402,158 |

The Mech entry alone has **71.04×** the reference application's text size.
Scopes differ: Mech's shared runtime is excluded, while Roc's whole text is
included. Error paths, padding and paths the small input never reaches are
included. These counts establish code expansion, not executed calls, branch
misses, memory traffic or an instruction-cache bottleneck. Runtime scalar
arithmetic remains directly emitted; the repeated frame/admission boundaries
are the current bounded simplification target.

## Next reductions and acceptance measures

1. **Frame transport:** combine proven adjacent managed field transport through
   the existing owner/admission machinery. Measure actual callbacks, hardware
   instructions and matched elapsed time, retaining ordered errors, finite
   charges, aliases, cleanup and recovery. The current implementation is
   unmeasured and receives no improvement credit here.
2. **Buffer reuse:** remove only proven dead aliases causing path reserve or
   tree copy fallbacks. Track backing requests, initialized bytes copied and
   incremental peak separately. A prior 9.77 MB requested-capacity reduction
   saved only 8,224 initialized copy bytes and did not improve elapsed time;
   requested capacity alone is a poor latency predictor.
3. **Code and prepared state:** measure generated text, stack, prepared live
   heap, RSS and footprint after every retained change. Determine what is
   retained solely for setup before promising minimal runtime-only memory.

No matched branch-miss, cache-miss, bandwidth, energy or total-copy-byte
measurement is available. These remain blank rather than inferred from
instructions or elapsed time. No larger-input throughput estimate is made.

## Pins and reproduction

Mech compiler source: `552e537fcbcfc3811ce442c982010b5e1e999e6c`;
published checkpoint: `0d31783d7d7cbb4d0ee411710f77fd1c317d7126`.
The measured compiler is unchanged by the reverted round-40 experiments.
Roc compiler revision recorded at original build:
`d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`; package:
`671077457ff7ef3a3d44caf715a3fc595addf379`.
Static input Object SHA-256:
`3f0c0e1954afe1083254d928a8ff66977ee9203e64bfd281326c75f9ff5f09e9`;
reference executable:
`3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`.

The unchanged source/bytecode, pinned mimalloc 2.1.7, M1 hardware, compiler
configuration, exact original/decoded bytes, independent inflation, output
capacity 13/reuse, input aliases and zero final managed bytes/nodes are recorded
in `../compression-round-40/CLOSURE.md`. This is complete level-12 compression
of `abcdefgh` with the real 262144 U16 tree, 1500513 U32 cache, 305257 U64 path
capacity, depth 300, nice 258 and original near-EOF limits. Algorithms, allocation
sites and timing boundaries are unchanged.

`python3 compare.py OUTPUT_DIRECTORY` regenerates the JSON and CSV from the
checked-in scorecards and static receipt without executing a benchmark.
`inspect_static.py` records the exact disassembler commands, tool version,
binary hashes and mnemonic histogram for a fresh static reproduction.
The round-40 independent reconstruction was rerun for this comparison:
**68 archives, 475 file occurrences and 368 unique paths verified**, with both
scorecards and the projection analysis reproduced byte-for-byte. No build,
test or profile overlapped any primary elapsed measurement. The new static
inspection ran after verifying that no measurement worker was active.
