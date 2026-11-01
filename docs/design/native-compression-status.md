# Native compression: current status

Raw observations, complete stack censuses, historical scripts and failed attempts
stay outside the source tree. Source keeps this status, one maintained harness and
the [compact evidence index](../../benchmarks/mech-deflate/EXPERIMENTS.md).

## Current measured checkpoint: R92

Complete pinned eight-byte level-12 compression improves **9.012791×→8.774856×
Roc**, or **2.639971% less elapsed**, against the freshly measured R91 compiler.
The target remains1×; larger-input throughput is unestablished.

| Metric | R91, freshly measured | R92 |
|---|---:|---:|
| Elapsed |2.5686455ms|2.500834ms (−2.639971%)|
| Elapsed / fresh0.285ms reference |9.012791×|8.774856×|
| Instructions, separate32-call cohort |28,237,280|27,873,787 (−1.287281%)|
| Cycles, same separate cohort |8,133,395.5|7,966,389.5 (−2.053337%)|
| Requests including reallocations |6,399|5,781 (−618)|
| Requested bytes |98,074,884|98,008,140 (−66,744)|
| Incremental peak live allocator bytes |17,851,960|17,782,744 (−69,216)|
| Generated compressor text / entry stack |15,911,648 /143,488B|unchanged|

Four prepared pairs contain3,968 primary/3,984 total observations. All four pairs
and63 descriptive blocks improve; one adverse block is retained. No significance
claim. Reference clocks vary between rounds: compare each matched pair, not a
cross-round ratio. Elapsed, hardware, heap, tracing and sampling are separate
cohorts, never multiplied. Requested capacity is not copied bytes or live memory.
All162 reallocations and the entire large-request histogram remain unchanged.

Base/prior published head: `fbf4c54a865e877f2026e7a825bab818889f628b`.
Measured/retained compiler: **`312e0d7b68d4b85052584bbdbfa9319f124c4f57`**.
Baseline compiler: `31d77c7c1f5fdeecc9095664af19092353df06b3`.
Separate CI inventory: `5691e6edf8df386e0fea9107df78c46caa588d4f`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
[PR #973](https://github.com/mech-lang/mech/pull/973) stays draft until green CI and
clean review. Publication CI is cancelled by instruction, not counted as passing.

## What changed

The canonical Engine all-path semantic read inventory finds500 constants:
191 with a runtime reader,309 without (248U8,51U32,10Index). That inventory already
covers expanded virtual actions, primitive/library operands, structural reads,
branches, collection sources, loop seeds/next/returns, effects and outputs. The
minimal `[1u8,2u8,3u8]` retains four immutable constants but only consumes the array.

Preparation records zero admission bytes only for a constant with no reader.
Invocation setup skips its independent ownership import and arena installation.
All500 immutable constants remain in the same plan for compilation/provenance.
Live scalar-only constants and consumers on unselected paths remain owned.
There is no per-loop lazy check, new permission cache, reused account authority,
interpreter or alternate preparation pipeline.

Image loading independently reconstructs the exact bounds and rejects changed
live/dead entries. Current account/funding checks remain for each materialized
owner. Constant totals and worker preflight use the same tape; empty arena slots
remain valid at retirement. Original/decoded/restored behavior and failure recovery
are qualified.

IR/work/private ABI/MBC are **138/99/38/14**, scalar ABI1. Policy99 explicitly omits
physical construction/import/installation/owner-cleanup prices for unread roots.
Logical work43,123,410,789→42,449,879,781; this is not hardware instructions.
The exact Huffman work assertion88,767,602→88,531,834 is a235,768-unit policy99
pricing change. Output goldens, component allowances, unsafe allowlists, warning
policy and architecture rules stay unchanged.

## Direct allocation and binary evidence

Fresh complete before/after censuses reconcile every successful request, requested
byte and reallocation with the independent heap counters. Constant setup is:

| Exact stack family | Before | After |
|---|---:|---:|
| 104-byte registrations |500|191|
| 112-byte ownership wrappers |500|191|
| Combined requests |1,000|382|

These account exactly for618 requests and66,744 bytes removed. After subtracting
this family, all remaining size/kind histogram bins agree. Both censuses contain
3,592 full stack/size/kind groups, with1,240/1,239 symbolized PCs. No exact
cross-binary join of every other stack family is claimed; all raw stacks remain.

The actual generated Mach-O `__TEXT,__text` is byte-identical, SHA-256
`62ed89d66389e5ae65e730779ef534dc1a11fc5fa8bdbc1b5613dc16c5fd4401`.
The improvement removes host setup/ownership work. Object identity changes with
the explicit revision; generated text and entry stack do not grow. Fresh serial
profiles cover695 Mech calls per subject and20,000 reference calls. CPU sample
weights are not retired-instruction shares or exact timer fractions.

Raw Object binaries remain local following the earlier automatic approval
rejection of binary upload. Numerical replay verifies pinned build/hash receipts
and retrieved descriptor/metadata, not independently retrieved Object bytes.

## Qualification and failure disposition

Current-source passing overlapping selections: runtime353;work/memory33;native
CI62;image1;unchanged components236;connected8;path1;linked16+12;scalar12
(maximum1.003625 at unchanged1.05, equal pure-scalar allocations, zero loop
allocation slope);frame8;workflow132;architecture/type/memory15. GPU-only release,
optimized units and runtime-only config retain warnings-as-errors against the
coherent selected-operation Core/Engine graph. No fresh full math/minimal/no_std
build claim. Hosted green CI, giant complete knucleotide and large/full-corpus
timing remain skipped.

Three new tests cover U8/U32 arrays, unchanged immutable constants, all-path and
unselected consumers, live scalar ownership, original/decoded/restored setup,
exact image-bound derivation/rejection, finite setup, source arithmetic origins,
prior publication, aliases, cleanup and retry. Twelve new linked/AOT cases actually
execute restored images, including unselected division, failure/recovery and zero
setup work. Existing linked16 and all component output/allowance checks pass.

Resolved preparations: a diagnostic fixture's unequal branch shapes were correctly
rejected; a test sum needed an explicit type; the intended negative baseline
failed ownership/image tests; the work golden advanced explicitly with99; the
counter driver retained a stale96-byte expectation from R90. That driver resumed
only its not-yet-run after-memory subject; completed hardware observations were
preserved unchanged. No required current local failure remains. Fresh retrieval verifies30 packets,
1,437 file versions and437 paths, including one path with multiple preserved
versions; all four numerical reports reproduce byte-identically. Historical
failed and adverse observations stay externally retrievable.

## Next bounded experiments

R91's retained whole-piece composition variant is already measured independently.
The next runtime experiment is the separately requested scalar-prefix/native-suffix
split: leave the paid prefix executing, derive fresh suffix proofs at its frontier,
and compare directly against the retained variant. Do not truncate an old proof,
remove the scalar fallback, or multiply the earlier gains.

Then target shape-solver temporary ownership and exact exit argument writes as
separate changes/cohorts. Live scalar constants, direct child seed handoffs,
predicate-sensitive last use and numeric outlining remain later work. Ordinary
debit-function tuning stays closed.

## Fidelity

The [component assembly](../../hosts/gpu/tests/fixtures/native/deflate_assembly.rs)
performs all compression as one compiled computation. Source SHA-256
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
MBC SHA-256
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Algorithm, tables, source allocation sites, depth300, nice258, near-EOF checks,
tree262144 U16/cache1500513 U32/path305257 U64, input aliases, pinned allocator,
output capacity13 clear/rebind and evaluate_into timer remain unchanged. Search
and skip stay separate; skip has no cache/recording path. Exact bytes
`[75,76,74,78,73,77,75,207,0,0]` and independent inflation agree. Final managed
bytes/nodes are zero. Physical public-output backing still changes.
