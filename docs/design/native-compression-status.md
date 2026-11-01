# Native compression: current status

Raw observations, stack censuses, diagnostic code and rejected implementations
remain outside the source tree. Source keeps this status, one maintained harness
and the [compact evidence index](../../benchmarks/mech-deflate/EXPERIMENTS.md).

## Current measured checkpoint: R94

Complete pinned eight-byte level-12 compression improves **8.640209×→8.573384×
Roc**, or **0.773422% less elapsed**, against the freshly measured retained R92
compiler. The target remains1×; larger-input throughput is unestablished.

| Metric | R92, freshly measured | R94 |
|---|---:|---:|
| Elapsed |2.4970205ms|2.477708ms (−0.773422%)|
| Elapsed / fresh0.289ms reference |8.640209×|8.573384×|
| Instructions, separate32-call cohort |27,875,625.5|27,865,806.5 (−0.035224%)|
| Cycles, same separate cohort |7,852,197.5|7,831,250.5 (−0.266766%)|
| Requests including reallocations |5,781|unchanged|
| Requested allocator bytes |98,008,140|unchanged|
| Incremental peak live allocator bytes |17,782,744|unchanged|
| Generated compressor text |15,911,648B|15,641,476B (−270,172B)|
| Entry stack |143,488B|143,424B (−64B)|

Four prepared pairs contain3,968 primary/3,984 total observations. All four pairs
and49 descriptive blocks improve;15 adverse blocks remain. Pair ratios range
0.975556–0.998231; the largest improvement includes a slower baseline pair. No
significance claim. Reference clocks vary: compare each matched cohort, not
ratios from different rounds. Elapsed, hardware, heap, fixed counters and samples
are separate cohorts and never multiplied. Requested capacity is not initialized
copy volume or live memory. All162 reallocations and the large-request histogram
stay equal. Whole-executable RSS/footprint fall180224/131008B in the hardware
cohort; these include prepared state and do not establish a managed-heap win.

Base/prior evidence checkpoint: `703128776c9eb44e6da9303fd8bf29ffed796586`.
Measured/retained compiler: **`c35698030a4e89f6d4548af1b17762ec737cd427`**.
Initial exit-store patch: `7b25ab730f2e6eff4235d0c78e2d97d7ad0c93fb`.
Baseline compiler: `312e0d7b68d4b85052584bbdbfa9319f124c4f57`.
Separate CI inventory: `3b859a463affc22c3aefdb595575c972bc63fbf1`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
[PR #973](https://github.com/mech-lang/mech/pull/973) stays draft until green CI and
clean review. Publication CI is cancelled by instruction, not counted as passing.

## The retained compiler change

Parent exits previously copied every scratch word into the helper argument bank.
The real exit consumer reads reached-retirement flags on every path and scalar
fields of frames escaping through successful frontiers. Numeric views and private
intermediate fields are not exit inputs. Code generation now derives the union
of those consumed ordinals from the existing immutable proofs and emits only
those argument stores. Original ordinals, the full bank-capacity guard, all
source/local spills, current owner validation and ordinary fallback stay intact.
No runtime permission cache, alternative evaluator or new authority is introduced.

IR/work/private ABI/MBC are **140/99/38/14**, scalar ABI1. Policy99 retains the
original conservative finite prices. Full compressor logical work42,449,879,781
and exact Huffman work88,531,834 stay equal; these are not instruction counts.
The same canonical partition/proof derivation serves preparation and image
loading. Original/decoded/restored forms retain source error origins, reached
cleanup, aliases, publication and retry.

The independent diagnostic counts are identical before/after:5,633 source
admissions,4,911 helpers,528 library callbacks,1,701 arena puts,938 selected native
entries and938 exits; zero declines or failed entries. Four complete calls per
subject reproduce those counts. Reached helper-word exports fall11,090→5,175,
removing5,915 exports per complete call. This is not payload copying or a hardware
instruction percentage.

Across484 selected pieces,4,868 scratch words become2,415 union export words when
counted once per piece. Emitted IR exit stores200,397→85,567 include cold and
unreachable failure paths; they are not dynamic machine counts. Actual ARM64
site699→700 demonstrates20 argument STRs becoming one at ordinal19, with the same
20-word capacity guard and local spills. That particular exit is cold in the
eight-byte call; the dynamic reduction above comes from the observed other exits.

The final Object SHA-256 is
`f6759d4c388e059790ab8490c4d304e8ca51c402c1f11e6c0ebd8428c30ef124`.
Its text SHA-256 is
`b51241ae4b045d08b8fdf733e8284b3d725d7ff71ac4ef8bcea75e69a29fb8a8`.
Native binaries remain local after the earlier automatic upload rejection.
Numerical replay retrieves descriptors, excerpts and retained hashes; it does not
independently retrieve Object bytes.

## The measured suffix alternative was rejected

R93 implemented the requested ordinary scalar prefix / independently derived
native suffix using the same graph partition and freshly derived origins,
frames, liveness, retirements, exits and prices. The2380–2396 pilot executes
natively. However, library callbacks elsewhere return528→535. Two independent
elapsed cohorts are adverse:8.510311×→8.538823× (+0.335028%) and
8.737647×→8.783405× (+0.523687%);7/8 pairs are slower.

Instructions−0.234727%, cycles−0.246734%, code−424484B and stack−4016B remain
favorable contrary evidence. Heap is unchanged. Revert `5addc236c` restores the
faster R92 source and CI inventory exactly. The complete working suffix patch,
clean-parent negative controls, candidate runtime354/CI63/image1/linked12
qualification, profiles and adverse observations remain external. Broader
component/scalar/architecture qualification was skipped for the rejected variant.
Fresh retrieval verifies21 packets/2419 file versions/1642 paths and reproduces
four reports byte-identically. Rejection is for elapsed regression, not because
a positive improvement was small. R91's whole-piece gain remains retained.

## Qualification and failure disposition

Final-source passing overlapping selections: runtime355;work/memory33;native
CI64;image1;unchanged components236;connected8;path1;linked16+12+12;scalar12
(maximum1.009377 at unchanged1.05, equal pure-scalar allocations, zero loop
allocation slope);frame8;workflow132;architecture/type/memory15. GPU-only release,
optimized units and runtime-only cfg retain warnings-as-errors against the
coherent selected-operation Core/Engine graph. No fresh full math/minimal/no_std
build claim. Hosted green CI, giant complete knucleotide and large/full-corpus
timing remain skipped.

New maintained tests inspect actual emitted IR-store counts and poison every
omitted helper word before the real exit handler. Success, checked-read retry and
arithmetic exits0/2/3 preserve current recursive values, published storage, aliases,
cleanup and recovery through original/decoded/restored forms. The original
full-bank emitter is a negative control: one test passes and the emission test
fails90 versus15 stores. Twelve new actual linked cases add changing buffers,
zero/unselected recursion, overflow, failed reads, declined native entry and retry.
Existing16 linked child/assertion cases and12 constant-owner cases also pass.

The runtime-only build initially rejected an ungated code-generation helper as
unused. Its exact feature gate fixes the warning without suppression. Rebuilt
Object, descriptor and metadata are byte-identical to before the feature-only
repair. An inherited control-receipt description saidR91; the actual R92 path and
hash never changed, and the original label receipt is preserved with its
correction. No required current local failure remains.

## Pins and next bounded work

The unchanged source SHA-256 is
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
bytecode SHA-256
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Full level12 still uses the262144-entry U16 tree,1500513-word U32 cache,
305257-entry U64 path capacity, depth300, nice258 and original near-EOF rules.
Search/skip stay separate. Mech compresses the eight input bytes and returns
`[75,76,74,78,73,77,75,207,0,0]`, verified by independent inflate. Allocator,
source allocation sites, input aliases, output capacity13 clear/rebind and
`evaluate_into` timing boundary stay pinned. Final managed bytes/nodes are zero.
Physical public-output backing still changes; physical output reuse is unproved.

R92's618-request dead-constant reduction remains. The next allocation experiment
is the canonical shape solver's lower-bound/witness/fixed-scratch temporaries,
with the real current census as its acceptance witness. The628-request population
is a candidate, not a promised saving. Source/local spill narrowing, child-seed
handoffs, predicate-sensitive alias proof and numeric outlining remain later
independent experiments. Each needs measured evidence; no compressor source
specialization or hidden allowance increase is part of this checkpoint.
