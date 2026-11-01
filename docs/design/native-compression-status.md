# Native compression: current status

Raw observations, diagnostic code, full censuses and rejected experiments remain
outside source. This status, the maintained harness and the
[compact evidence index](../../benchmarks/mech-deflate/EXPERIMENTS.md) retain the
review and reproduction path.

## Current measured checkpoint: R97

Keeping private parent scratch in SSA improves complete pinned level-12
compression of eight bytes from **8.498282× to 8.378794× Roc's time**: **1.406025%
less elapsed time** against the fresh matched baseline. All four pairs improve;
55 descriptive blocks improve and nine worsen. All adverse observations remain.
The target is 1×. Larger-input throughput remains unestablished.

| Metric | Fresh R95 compiler baseline | R97 |
|---|---:|---:|
| Primary elapsed |2.473000ms|2.438229ms|
| Elapsed / fresh 0.291ms reference |8.498282×|8.378794×|
| Instructions, separate 32-call cohort |27,796,101|27,387,432.5 (−1.470237%)|
| Cycles, same hardware cohort |7,823,060.5|7,689,146.5 (−1.71%)|
| Requests including reallocations |5,153|unchanged|
| Requested allocator bytes |97,997,834|unchanged|
| Incremental peak live allocator bytes |17,782,744|unchanged|
| Generated compressor text |15,641,476B|14,604,696B (−1,036,780B)|
| Machine entry stack |143,424B|135,728B (−7,696B)|

Four prepared pairs contain 3,968 primary/3,984 total observations. Blocks within
a pair are descriptive, not independent experiments. No significance or larger
throughput claim. Compare matched cohorts: R95's historical 8.783390× ratio used
another reference clock and baseline, so its difference from 8.378794× is not the
measured compiler gain.

Primary elapsed, hardware, heap, scalar controls, fixed counters and samples are
separate cohorts, never multiplied. All 162 reallocations and the large-request
histogram stay equal. Requested capacity is not copying or live memory.
Hardware-cohort RSS rises 319,864,832→320,372,736B and physical footprint
228,853,504→229,394,176B. These include prepared code, libraries and harness state;
there is no whole-process memory improvement claim.

Compiler/source: **`20f002f4ff5ff2186888b7efc8517261756b425e`**.
Measured baseline compiler: `67a7c88e745092264a5a735933dd95cf48fc5446`.
Source parent after R96 diagnosis: `38cf6825c575f046ca8b365f2d3c28108f303b49`.
Prior published head: `5848625e2e3ade9d85c4db7f3380b20bae230559`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
[PR #973](https://github.com/mech-lang/mech/pull/973) stays draft pending green CI
and clean review. Publication CI is cancelled by instruction.

## What changed and why it is safe

Selected numeric parent pieces used to load checked helper arguments, store them
in private local homes, reload those homes into SSA, then spill them again at
every exit. The runtime exit consumes the explicit argument exports, not those
private local homes.

The common `Words` implementation now accepts direct initial SSA inputs. Parent
pieces load scratch from the already checked current arguments and spill only
ordinary source homes. Every original source slot, including operands needed
by cold checked-read retry and arithmetic failure, is still saved. Argument
ordinals, the full capacity guard, checked operations, retirement flags, owner
loans, finite funding, decline and publication paths remain unchanged.

All 484 selected pieces retain their source sites, frontiers, register-word
population and exit argument sets/stores. Static emitted IR omits 4,868 entry
home stores, 4,868 reloads and 200,397 exit home stores: exactly 210,133 fewer
instructions. These are IR counts, not hardware instruction counts.

Fixed counters observe the same 938 selected entries/exits, no declines and the
same 528 library helpers. Those visits omit 11,090 entry and 11,090 exit home
stores in the IR. Actual ARM64 excerpts at the retained cold exit 699→700 show
20→0 private-home STRs, while the one retirement argument and full guard remain.
That particular exit is cold in the eight-byte run; no hot-path percentage is
inferred from its excerpt.

Regression controls poison private homes before selected entry and verify that
success, checked-read retry and arithmetic exits leave them untouched. Original,
decoded and restored execution retain aliases, prior publication and cleanup
through success→failure→retry. Restoring only the old exit spills in an isolated
copy makes this control fail; production source remains unchanged by that
negative control.

IR/work/private ABI/MBC are **141/99/38/14**, scalar ABI1. IR141 distinguishes the
new code generation and retains strict old-image rejection. Capacity declarations
and conservative prices remain unchanged. Full logical work is 42,449,879,781
and Huffman work 88,531,834; neither is a hardware instruction count. Existing
unused-constant, canonical-shape and exit-argument improvements remain retained.

## Qualification and evidence

Passing current selections: runtime355, work/memory33, native CI64, image1,
unchanged components236, connected8, path1, linked16+12+12, scalar12, frame8,
workflow132 and architecture/type/memory15. The scalar gate remains 1.05;
maximum observed ratio is 1.002924, with equal pure-scalar requests/bytes and zero
loop allocation slope. Exclusive and aliased frame request/byte counts remain
unchanged. No expected output or component allowance is changed.

Current optimized GPU and unit roots use warnings-as-errors, the unchanged
selected feature/operation profile, and the coherent R95 Core/dependency graph.
There is no full math-library rebuild or fresh all-features dependency graph.
R95's Core376, actual Rust consumers102 and minimal no_std qualification remain
historical coverage; they were not rerun for this GPU-only change. One validation
driver initially requested the wrong work-test receipt filename; corrected
selection executes all33. Its preparation failure is retained. The intentionally
failing old-spill control is separate from passing production qualification.
No required current local failure remains.

Every primary timing packet was committed. R96/R97 evidence is externally
hash-indexed; pure `replay-round` reconstructs the numerical reports without
compilation or compression. Native binaries stay local after the earlier
automatic upload rejection. Descriptors, excerpts and hashes do not establish
independent retrieval of native Object bytes. Full/all-features hosted CI,
large-input timing and independent Object retrieval remain skipped.

Fresh serial profiles cover 695 complete calls per subject (1,330/1,314 sample
weights). Retirement, admission and frame helpers remain prominent. Samples are
not retired instructions or exact timer fractions. R96's separately calibrated
pre-change windows attributed about4.98M instructions to generated entry and
5.50M to parent entry/exit; the reconstructed total differed 0.544114% from its
uninstrumented cohort. Those are diagnostic estimates for that earlier source,
not a post-R97 per-function instruction partition.

## Fidelity and next bounded work

Source SHA-256:
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
Bytecode SHA-256:
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Level12 retains the 262144-entry U16 tree, 1500513-word U32 cache, 305257-entry U64
path capacity, depth300, nice258 and original near-EOF rules. Search and skip
remain separate. Algorithm, tables, source allocation sites, allocator and
timing boundaries stay pinned.

Mech performs compression and returns
`[75,76,74,78,73,77,75,207,0,0]`, checked by independent inflate. Input aliases,
output capacity13 clear/rebind and `evaluate_into` timing remain. Final managed
bytes/nodes are zero. Physical public-output backing still changes; this patch
does not establish physical output reuse.

Next, investigate the duplicate private seed-to-child-carrier handoff. A fresh
inventory finds 89 current sites, 112 reached packs and 543 top-level operand
visits, replacing the older 134/714 inventory. These are candidates, not a
lifetime/consumption proof or promised saving. Establish the adjacent single-use
witness, preserve pack-before-initialization errors and disjoint-lane cleanup,
then measure the smallest general handoff change. Alias proof and numeric
outlining remain separate experiments.
