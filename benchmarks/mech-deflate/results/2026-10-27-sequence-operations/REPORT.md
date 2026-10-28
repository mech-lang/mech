# Executable packed sequence operations

The stack is linear above the rearchitecture. This patch supplies buffer
operations needed by the level-12 parser and writer. It is a compiler/runtime
prerequisite, not a complete compressor or compression timing.

## Revisions and scope

Base: `28d137e68eb3dd5c29302e2ce15174ad0c0a98ba` (#940).
Validated code: `aad1f6528dd49815be930bf42f044296968fa38e` (#941).
The packet commit changes documentation and receipts only.

| Commit | Change |
| --- | --- |
| `2791da25d48d2447d14782a631ef685572e999a4` | Source contracts, resident execution, native selection, buffer ownership and tests |
| `2771d1c23109bcd480ec4fa16daa67bd0d2809fe` | Initial CI scheduling |
| `ee32fbfa98b174d32a3b53788f70b1f965ac144c` | Retain the existing CI shard, workflow contracts and inventory |
| `1a22b9698ecf359d3e2da959addf000631f5143e` | Accept generic rank-two parameters; validate actual one-row geometry at execution |
| `aad1f6528dd49815be930bf42f044296968fa38e` | Propagate capacity changes; admit the full old backing when consuming growth reallocates |

Pinned source: `671077457ff7ef3a3d44caf715a3fc595addf379`.
Pinned reference compiler: `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`.
All 73 existing native/component and executable bytecode fixture files are byte
identical to the base; see `unchanged-fixtures.json`. Algorithms, expected
results, allowances, source allocation sites and timing boundaries are unchanged.
Native IR/ABI/work-policy versions are **109/35/86**; canonical bytecode remains
revision **14**.

Published order: integration/v0.4 → #934 → #935 → #939 → #913–#927 → #930 → #931
→ #932 → #933 → #937 → #938 → #940 → #941. Native stack 936 has 26 PRs and 25
matching adjacent base/head branch edges. This does not claim the foundation
PRs are merged or their separate performance gates accepted.

## Requirement and design

Pinned `CompressOptimal.roc` reserves **305257 U64 path nodes**, clears them for
parsing and appends selected nodes. The output reserve for reymont is **6633832
U8 slots**, retained across repetitions. The ordinary operations are:

```text
sequence/with-capacity(prototype, capacity<u64>)
sequence/reserve(values, additional<u64>)
sequence/append(values, element)
sequence/clear(values)
```

U8/U16/U32/U64 share maintained type schemes, contracts, runtime factories,
resident execution and selected native calls. The prototype fixes the element
kind; with-capacity initializes no elements. Reserve is exact. Append follows
the pinned runtime's 64-byte first allocation, doubling below 4 KiB and above
128 KiB and growing by 3/2 between them. See the
[pinned allocation rule](https://github.com/roc-lang/roc/blob/d99c7b838fb05bb1047eb0c18d4d06d0f86f345d/src/builtins/utils.zig#L700).

Capacity is separate from initialized length. Operations accept generic rank-two
input schemas, check actual row count, and refine successful results to one row
with a current Turn length. Growing recursion no longer freezes to the initial
empty length or an increasing affine schema. No parser or recursive-schema
validation was relaxed.

Same-plan read/definition/successor facts derive private last-use candidates.
Future reads precede a later definition's kill. Loops, collection, publication
and effects retain conservative ownership frontiers. The runtime checks the
selected factory/site/current visit, retires matching frozen owners only, and
requires the core's actual exclusive same-account handoff. Images rederive
candidates from validated plans, without serialized ownership authority. Aliases
and imports stay immutable. Scalar instruction emission is unchanged; plans
without these calls skip the new read inventory.

Metadata and payload allocations follow finite admission. Reallocation includes
old/new capacity overlap. Consuming Vec growth charges **full old capacity** to
compute and cloned-byte limits, including spare storage; fresh shared copies
charge initialized bytes. Clear retains capacity without zero filling. Failures
preserve published outputs and aliases, clean staged storage and permit retry.
Resident change detection includes capacity, while language equality/hashes/tokens
continue to compare logical values. Canonical U8/U16/U32 scalar codecs now follow
the existing unconditional U64 codec independently of legacy factory selection.

`mech-engine/sequence_buffers` is explicit and does not select text. The canonical
native Program host enables it. Standard/full distribution manifests and catalog
snapshots are unchanged.

## Validation

Final code was tracked-clean at `aad1f6528`; only this packet was untracked.
Cargo used nightly-2026-03-03, `--locked --offline -j1`, no incremental
compilation and unoptimized dev/test profiles with debug info off. Warnings remain
errors. Exact commands, environment, revision, dirty state, exit and elapsed time
are in `receipts/`; unchanged terminal output is compressed in `logs/`, with
SHA-256 hashes in `MANIFEST.json`.

| Coverage | Observed result |
| --- | --- |
| Canonical native Program, components plus buffer and linked cases | **152 passed, 0 failed, 0 ignored, 129 filtered**; 441.24 s tests, 544.536 s including build |
| Engine source/resident/direct managed buffer contract | **5 passed, 0 failed, 0 filtered**; 0.72 s tests |
| Engine runtime-only `sequence_buffers` | `cargo check --no-default-features --features sequence_buffers --lib`: pass |
| Engine source-free native | `cargo check --no-default-features --features native-artifact,sequence_buffers --lib`: pass |
| GPU source-free runtime | `cargo check --no-default-features --features canonical-native-program-runtime --lib`: pass |
| Core `no_std` | `cargo check --no-default-features --features no_std --lib`: pass |
| Formatting, warnings, native analysis layout, resident routing, source entrypoints, Rust layout, host catalog, patch whitespace | pass on final code |
| CI workflow / test inventory | **90 / 42 passed**; live inventory has zero gaps after CI correction |
| Static distribution profiles / compiler-planning quarantine | pass; existing checks and snapshots retained |

The 152 tests comprise the unchanged 143 component cases, **8 native buffer cases
and 1 linked/AOT buffer case**. CI schedules the engine target as a plain Cargo
invocation in the existing native shard with
`semantic-compiler,resident-artifact,sequence_buffers`; five tests execute.

Coverage includes original/decoded artifacts, resident/native agreement, real
reservations, all widths, exact growth, capacity-only turns, invalid geometry
and kinds, unselected paths, aliases, prior output on failure, finite work/copy
admission, cleanup and retry. The linked harness executes reserve/append/clear.
Both review findings have red/green receipts: resident capacity 32→64 previously
stayed 32; an empty capacity-1000000 buffer previously passed a 999999-byte growth
copy allowance. The compute control measures the selected reserve call's
no-growth work and pins rejection to that call.

Real direct reservations use 235 allocator requests for either small or real
buffers. U64 requested bytes change from 26101 to 2468029, exactly the payload
difference; U8 from 25989 to 6659805. These are cumulative allocator requests in
a small unoptimized computation, not peak memory or compression performance.

Remote CI was canceled as requested, including code-head run
[40466158359](https://github.com/mech-lang/mech/actions/runs/40466158359).
No completed remote CI result is claimed. No full reymont, optimized comparison
or complete compression run was executed. The expensive ignored collector
diagnostics were not repeated for this buffer-only patch.

## Open blocker and measured next step

`recursive-growing-buffer.mec` computes correct contents but recursive
argument/frame transport loses reserved capacity. The pending acceptance
`pending-recursive-capacity.rs.txt` covers original/decoded programs, U64/U8,
small/real reservations and 0/1/3 appends. **All 24 contents and cleanup checks
pass; all 24 capacity checks fail.** These failures are not counted as passing
qualification. See `recursive-capacity-observations.json` and the saved minimal
consumer with matching locked dependencies (188 package tuples).

`CompositeSnapshotConstructor::construct` clones `child.data()`; packed Vec
cloning loses spare capacity. Both that preexisting file (blob
`f039eb02d9668d1b7e02e4a39033a0487c14200f`) and sequence storage (blob
`b09593e2d2872c4da9c1c05614ef088634d4024b`) are byte identical to the clean parent.
The new APIs expose this transport limitation. They do not exist in the parent,
so the negative source program cannot run there.

The unchanged real argument witness on final code carries the **262144 U16 tree
and 1500513 U32 cache** through one recursive step. Successful original/decoded
evaluations request **5508 allocations / 26611790 cumulative bytes** in
48.7–51.5 ms. Assertion rejection requests 2802 / 13315186 bytes, preserving
outputs, aliases, cleanup and recovery. Payload is 6526340 bytes. Allocation
volume is measured; memcpy bytes and peak memory are not measured by this
counter. See `large-argument-baseline.json` and the raw full qualification.

The next bounded change must remove physical copies in ordinary argument/frame
transport, retain capacity, and preserve aliases and recovery. Cloning larger
buffers to retain capacity is insufficient. Use the pending acceptance and real
tree/cache baseline, retaining direct scalar execution. The optional generic
external growing-recursion schema probe remains unqualified and is not claimed
fixed.

`FAILURES.json` retains every earlier failing validation command and its
disposition: corrected implementation/test/profile integration, corrected CI,
two fixed review findings, or pending transport. No existing component failure
remains. Early dirty-tree receipts retain their actual dirty state; their parent
HEAD is not presented as the code under test.

## Compressor checkpoint

Permanent functions executing include `bt-get-matches`, `bt-skip`,
`optimal-search-position`, `optimal-skip-run`, `optimal-block-loop`,
`optimal-collect-block`, `optimal-first-block` and `optimal-carry-block`, including
cache records, statistics, decisions, rewind and sliding. Search and skip remain
separate; skip has no cache argument or recording path. Real buffers, depth 300,
nice length 258 and original near-EOF limits remain unchanged.

The largest completed collector input is **320 bytes**, with full scalar and
buffer reference agreement for original/decoded artifacts, about 663.5 s per
evaluation in the earlier explicit unoptimized diagnostic configuration. This is
match collection, not compression timing.

Remaining `compress_into` work: default/adjusted costs, `find_min_cost_path`,
code-cost iteration and choice, qualified Huffman integration with real sizes and
allocation sites, block header/bit writing and flush, orchestration/carryover and
repetitions. Exact raw-byte agreement, independent decompression, full reymont
and a complete measured run follow the connected small cases. No complete
compression time exists yet.
