# Mech and Roc executable comparison

Current Mech takes **14.847× Roc's elapsed time**, executes **35.622× the instructions**,
and makes **147.305× the allocation requests** for the same complete eight-byte
level-12 compression. The latest retained compiler/runtime change removes a
redundant clone/drop of temporary managed handles during private record packing.
It improves matched elapsed **1.854%**, instructions **1.853%** and cycles
**2.392%**, with unchanged tracked allocation and heap metrics.

## Per-compression scorecard

Roc is 1× in every row. MB is decimal million bytes. Elapsed, hardware counters,
allocation tracking and sampling are separate cohorts; instrumented elapsed is
excluded from the primary timer.

| Metric | Roc | Previous Mech | Current Mech | Current / Roc |
| --- | ---: | ---: | ---: | ---: |
| Elapsed | 0.289000 ms | 4.371896 ms | **4.290833 ms** | **14.847×** |
| Executed hardware instructions | 1.364 million | 49.519 million | **48.602 million** | **35.622×** |
| CPU cycles | 0.904 million | 13.900 million | **13.567 million** | **15.013×** |
| Allocation requests | 95  | 13,994  | **13,994 ** | **147.305×** |
| Requested allocation bytes | 9.022 MB | 98.587 MB | **98.587 MB** | **10.927×** |
| Extra peak tracked heap during compression | 9.257 MB | 20.575 MB | **20.575 MB** | **2.223×** |
| Total tracked peak heap including preparation | 9.257 MB | 163.282 MB | **163.282 MB** | **17.638×** |
| Resident memory after compression including setup | 14.762 MB | 319.685 MB | **319.734 MB** | **21.659×** |
| Kernel physical footprint after compression | 14.402 MB | 228.706 MB | **228.706 MB** | **15.880×** |
| Machine-code text (different scopes) | 0.195 MB | unchanged | **13.886 MB** | **71.043×** |

The matched primary time is **4.371896 → 4.290833 ms** or
**15.127668× → 14.847173×**, with one shared fresh reference of **0.289 ms**.
All four independent preparations and **62/64 descriptive blocks** improve.
There are 1,984 primary calls/version, 16 alternating-order blocks of 31/version
per pair. The reference has 20,000 calls. The fixed historical 0.300 ms ruler moves
**14.572987× → 14.302777×**. Compare within the matched pair: the previous
round's 4.3493335 ms and 0.285 ms reference belong to a different cohort.

Hardware medians use 32 steady calls/implementation; allocation medians use eight
Mech and 32 reference calls. Boundary-read overhead, approximately 5010 instructions,
is included. No CPU observation incurs a page-in. Current Mech executes
**47.237 million more instructions per compression**. Instructions and cycles
measure different costs; their reductions do not have to equal elapsed gains.

## Memory and buffer ownership

Tracked live heap retained before compression is **142,706,080 bytes** in both
Mech versions. Whole-task RSS and footprint include retained compiler/catalog
and preparation; these are not minimal runtime-only application requirements.
Tracked heap, RSS and physical footprint are different views and must not be
added. Tracked heap excludes allocator-private heaps, direct VM and transient
relocating-realloc overlap. Reallocations remain 1213, including 458 relocations.

| Large backing requests during one compression | Roc | Mech |
| --- | ---: | ---: |
| U64 path capacity, about 2.44 MB each | 1 | **35** |
| U16 tree, about 0.524 MB each | 1 | **5** |
| U32 cache, about 6.00 MB each | 1 | **1** |

Both implementations receive the whole input already in memory. Mech's immutable
input handles share frozen backing, verified on every observation. Neither
streams file input here. The prior ownership census identifies four extra tree
fallback copies totaling **2,097,152 initialized bytes**; the current histogram
retains five tree requests. Extra path requests reserve **83,029,904 bytes of
capacity** but copy only initialized elements. Reserved capacity is not bytes
copied. Total input-copy and total runtime-copy bytes remain unmeasured.

The new pack change is a handle-transport improvement: `Value` already shares
shape, root and optional schema through Arc. It removes reference-count traffic
without claiming a payload-copy or allocation reduction. Other aliases stay real
owners, and later buffer reuse still requires a future-read/exclusivity proof.

## Why the latest change helps

The existing selected call creates temporary managed handles, validates every
field and its original finite charge, clones those handles into the private
frame, then immediately clears the temporaries. Moving the disposable owners
after the same complete preflight removes the duplicate clone/drop. Current
arena roots, external aliases, composite child copies and all charges remain.

Disassembly of the actual measured linked workers shows:

| Reached host helper | Previous | Current |
| --- | ---: | ---: |
| Pack instruction sites |469|**448**|
| Pack atomic increment sites |3|**0**|
| Enclosing frame-action instruction sites |1332|**1332**|

Static sites include error/unreached paths and are not dynamic counts. Dynamic
hardware instructions independently fall **49,519,459.5 → 48,601,797**.
The generated compressor Object remains byte-identical, SHA-256
`3f0c0e1954afe1083254d928a8ff66977ee9203e64bfd281326c75f9ff5f09e9`.
Its entry stack remains **108,400 bytes**.

## Remaining sampled work

Fresh serial samples of the actual linked primary workers use 695 calls each.
The current main thread has 1237 disjoint weights:

| Sample ancestry | Weights | Share |
| --- | ---: | ---: |
| Remaining generated/runtime/harness work | 453 | 36.62% |
| Ordinary control visits | 175 | 14.15% |
| Other invocation helpers | 177 | 14.31% |
| Selected library calls | 110 | 8.89% |
| Function-frame actions | 322 | 26.03% |

Samples include surrounding harness work; these are not exact percentages of
the compression timer or guaranteed removable time. Roc's retained profile has
1416/1513 weights in its hot compression function. Inspected offsets show direct
paired stores in tree/cache initialization. Mech's remaining frame/admission
boundaries and buffer ownership are still concrete optimization targets.
The earlier callback-batching experiment was neutral and was restored.

The unchanged generated entry contains 3,471,424 four-byte instruction sites,
55,450 call sites, 110,409 conditional branches, 225,522 unconditional branches,
737,401 loads and 402,158 stores. Scope differs: Mech's generated compressor entry
alone versus Roc's entire executable text. These figures include padding and
unreached paths, and do not establish instruction-cache misses.

## Pins, scope and reproduction

Baseline production source: `a0ad02497c96ea876799d1f844554cdc23d6c669`.
Implementation: `f85d1d670aba40b145fd5b9586def4aab061b507`.
Optimized host-build source: `be651d31a82008daf24b4ad2d4dcbc553264a10a`.
Both primary and counter/memory variants use their exact same coherent optimized
library graph and same-build Object/image. The transitive Compute artifact is
frozen too. Every non-GPU library hash is unchanged; only GPU rebuilds, in 22.864 s.
The preceding round's diagnostic dependency recovery is retained in its own packet.

Roc's recorded compiler revision remains
`d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`, package
`671077457ff7ef3a3d44caf715a3fc595addf379`, executable SHA-256
`3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`.
All observations use the M1 Mac mini, fixed mimalloc 2.1.7, unchanged level-12
source/bytecode, real 262144 U16 tree/1500513 U32 cache/305257 U64 path capacity,
depth 300, nice 258, original near-EOF limits/allocation sites, output capacity 13
clear/reuse and evaluate_into-only timing. Every Mech call checks exact ten raw
bytes, independent inflate, input aliases/content, work 60,260,219,686 and zero
final managed bytes/nodes. No build/test/audit/profile/diagnostic overlaps primary.

Branch/cache misses, bandwidth, energy and total copied bytes remain unmeasured.
No larger-input scaling or parity claim follows from this small complete case.
[Current design, validation and independent reconstruction](../compression-round-43/CLOSURE.md).
`python3 compare.py OUTPUT_DIRECTORY` regenerates JSON/CSV without executing a
benchmark. Original static commands and mnemonic histograms remain in
`inspect_static.py`/`static.json`; Object identity verifies reuse of that inventory.
