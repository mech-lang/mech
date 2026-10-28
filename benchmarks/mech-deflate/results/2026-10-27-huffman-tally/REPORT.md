# Block tally and real-size Huffman checkpoint

The permanent Mech components now execute collector → backward path → forward
symbol tally → Huffman codes as one native computation. This is not yet a
complete compressor or a complete compression timing result. The connected
qualification still supplies cost tables and stops before block writing.

PR: [#945](https://github.com/mech-lang/mech/pull/945), directly above #944.
The published stack snapshot has 30 PRs, 29 matching adjacent base/head pairs,
and no divergent edge. The architecture/cleanup foundation is retained.

| Revision | SHA |
| --- | --- |
| Exact base, #944 | `95f8dc4c45fe9670315cedcdc8c4768b760cd3c2` |
| Compiler annotation fix | `269fe951f183869e76b43091c61f1a6dac40e960` |
| Components and exact qualified/reviewed source head | `333683b40b6d42ce0ead91d0c1158762e99680fd` |
| Pinned target source | `671077457ff7ef3a3d44caf715a3fc595addf379` |
| Pinned reference compiler | `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d` |

The evidence-only commit containing this packet follows the qualified source
head. It changes no compiler or compressor code. `fidelity.json` records the
complete 12-path source diff and its hash. All 30 pre-existing native fixtures
and all earlier qualification support files retain their expected values and
allowances. No native lowering, code generation or runtime source changed.
Typed IR 109, ABI 35, canonical codec 14 and native work policy 90 are unchanged.

## Compiler requirement and fix

The exact blocking operation is returning the two buffers in
`HuffmanEncode.make_code` after sort/tree/code generation. Structural projection
reifies dimension declarations as Explicit; their existing order can differ
from a fresh output annotation. The clean tracked base rejects the final
original-records port with `WitnessTypeMismatch { node: NodeId(34) }` because
source conformance emitted a redundant record conversion.

The minimal schema witness is:

```
actual:   { lens: U8[1, p1], codewords: U32[1, p0] }
expected: { lens: U8[1, p0], codewords: U32[1, p1] }
p0, p1: distinct, Explicit, Turn, lower bound 0, no upper bound
```

These finalized schemas differ because Explicit declarations preserve order.
A bijection `actual p1 → expected p0`, `actual p0 → expected p1` establishes an
identity annotation. The new source-only helper checks that bijection, repeated
references, constants, field names/order and exact unsigned element kinds.
Only unconstrained Turn axes enter this new case. Bounds, different lifetimes,
changed axis relationships and value conversions retain the maintained path.
Exact canonical-schema identity, including removal of unused projection axes,
continues to work as before.

Five focused unit tests cover renumbering, unused projection axes, repeated-axis
relationships, coupled-versus-independent axes, constants, element kinds,
bounds and Activation lifetime. Three existing source annotation tests retain
independent inner axes and dependent target bounds. Both original and decoded
full components compile and execute. This does not add general record casting
to the native backend or relax its factory/type checks.

The discarded named-state draft hit the existing closed-kind restriction.
Ordinary recursive functions carry the actual dynamic buffers in the maintained
port. No padded 288-entry replacement state or new global-FSM mechanism was
introduced.

## Permanent functions and fidelity

`deflate-huffman-level12.mec` implements `huff-make-code`, symbol counting and
placement, saturated-bucket heapsort, packed tree construction, length limiting,
length assignment, canonical code generation and reversal-table construction.
It retains the original sorted/generated/code record representations and read,
write and checked-arithmetic order. U32 wrapping operations retain their width.

The real calls use 513/270/19 U32 frequency buffers and 288/32/19 U8 lengths and
U32 codewords, with maximum lengths 14/15/7. Sort counters are `n` U64 entries;
normal tree construction allocates `maximum + 2` U64 length counts, then 17 U32
next-codeword entries. Degenerate codes omit those last two allocations. The
256-byte reversal table uses capacity plus append and is shared between codes.

`optimal-tally-and-build-codes` walks the packed U64 path forward, preserves the
wrapping per-item counts/cursor, adds the end symbol with checked arithmetic,
and returns all six original frequency/length/codeword buffers. No cache is
passed to skip; the search/skip sources are unchanged. Depth 300, nice length
258, original near-EOF limits, the 262144-entry U16 tree, 1500513-word U32 cache,
and 305257-entry U64 path reservation are retained.

The generators execute the unchanged pinned functions and verify all fourteen
tracked package modules byte-for-byte first. Seventeen Huffman cases compare
all lengths and all codewords, including unused symbols. All 256 reversal bytes
and its retained reservation are checked. Six tally/path cases compare every
entry in all six output buffers. No expected value comes from the Mech port.

## Exact-head validation

All following commands ran at tracked-clean `333683b40b` with
`nightly-2026-03-03` (`rustc ec818fda361ca216eb186f5cf45131bd9c776bb4`),
`aarch64-apple-darwin`, locked/offline dependencies, one build job, incremental
compilation disabled and test/dev debug info disabled. Warnings remain errors
through `.cargo/config.toml`. Exact commands, configurations, status snapshots,
times and log hashes are in `receipts.json`; raw logs are compressed separately.

| Selection/profile | Observed result |
| --- | --- |
| GPU `canonical-native-program`, maintained component/linked selection | **33 passed**, 0 failed/ignored, 273 filtered; 643.65 s execution |
| Same profile, exact caller-layout unit | **1 passed**, 422 filtered; persistent 1112 / invocation 1288 bytes |
| Engine `full_source,native-artifact,sequence_buffers`, annotation selections | **8 passed**, 0 failed/ignored, 1085 filtered |
| Engine source-free `native-artifact,sequence_buffers` | check passes |
| Engine runtime-only `sequence_buffers` | check passes |
| GPU runtime-only `canonical-native-program-runtime` | check passes |
| Formatting, resident routing, source/host catalogs, native analysis/module layout | pass |
| CI integration inventory | 0 gaps |

The GPU command is `cargo test --locked --offline -j1 -p mech-gpu
--no-default-features --features canonical-native-program --lib --test
canonical_artifact_program -- ... --nocapture --test-threads=1`; its receipt
contains every selected filter. Including compilation it took 781.119 seconds.
It covers nine earlier Huffman tests, seven buffer-frame tests, six record-buffer
tests, three earlier path tests, two linked recovery tests, and six new
Huffman/tally tests. Earlier expected values and allowances are unchanged.
An unmatched caller-layout filter initially selected zero library tests; the
separate exact-name invocation executed one. Empty selections are not counted.

The new tests check true success → checked arithmetic failure → success, input
aliases, unchanged prior output owners, cleanup and recovery. The combined
program additionally checks success → default transport denial → success with
zero rejection allocations and unchanged publication. Original/decoded forms
agree with the independent executed reference. The earlier resident resource
error/retry and linked record/frame recovery tests pass unchanged.

Automated code review completed at `333683b40b` with no findings or review
threads; `review.json` preserves the receipt. CI run
[40503346856](https://github.com/mech-lang/mech/actions/runs/40503346856) was
canceled at the user's instruction. No remote-CI success is claimed or awaited.

## Measured execution

These are unoptimized component diagnostics. Input creation, compilation,
reference checking and final published-output drop are outside the measurement;
invocation admission, temporary cleanup and publication are inside it. Allocator
bytes are cumulative requests, not peak memory or measured physical copy bytes.

| Computation | Original / decoded elapsed | Requested bytes, original / decoded |
| --- | --- | --- |
| Own collector → path → tally → codes, 8 literals | 13.048218 / 13.039249 s | 132325790 / 132325726 |
| Same, 8-byte overlap | 15.082548 / 15.067176 s | 147061441 / 147061441 |
| Path → tally → codes, 320 overlap positions, supplied real cache | 1.771251 / 1.763942 s | 852885229 / 852885229 |
| Same, 320 tied-cost positions | 2.048022 / 2.039606 s | 861417339 / 861417339 |

The newly connected largest input is **8 bytes**. The 320-position result uses
the qualified reference cache as input; it is not a new 320-byte full collector
or compression timing. `measurements.json` contains all 62 final recorded runs.
Allocation overhead remains substantial; this checkpoint preserves that baseline.

The combined program compiles 9140 native control blocks under the existing
explicit 16384-block diagnostic. Native transport demand is **1103136 bytes**;
the unchanged 1 MiB default rejects before allocation or execution. The new
program declares an explicit finite **2 MiB transport allowance**, with **1 GiB
managed memory**, **128 MiB temporary memory**, and **1T execution, compute and
comparison work**. Only this new diagnostic receives that transport allowance.
The default denial remains a tested outcome. Compiler/default policies and old
qualification limits are unchanged. An early failed log mislabeled intended
positions as completed; actual execution at that preflight denial was zero.

## Dispositions and remaining work

`failure-disposition.json` classifies all fifteen nonzero development commands:
the inherited annotation and closed-state restrictions, expected transport
admission, the environmental reference-cache denial, discarded reduction
candidates, and corrected test-harness/fixture errors. There are no failing
commands in the final nine-command selection.

An exploratory full new-component resident attempt is **not qualified**: the
minimal resident harness rejects reversal activation at NodeId(2). The retained
executable built at clean `93a7be76e` reproduces that behavior; its hash/build
receipt and the evidence-only difference to the base are in `fidelity.json`.
`resident-reversal-witness.mec.txt` preserves the source. Its root cause is not
claimed fixed or fully diagnosed. It does not prevent the native benchmark
path, and no full new-component resident agreement is claimed. The earlier
maintained resident denial regression still passes. The reduced unit-profile
baseline failures documented in #944 were not reopened as work packages.

The complete previous 171-test selection was not rerun here; this change reruns
the 27 affected earlier component/linked tests plus six new tests. Full raw-byte
comparison, independent decompression, complete multi-block/carryover validation,
reymont, release timings and performance comparisons remain unexecuted because
the full compressor is not connected.

Functions still needed for `compress_into` include default-cost choice,
`set_default_costs`, `set_costs_from_codes`, adjustment/observation blending,
`choose_all_literals`, `compute_true_cost`, block header/precode construction,
packed length/offset tables, block/bit writing, and the full
`CompressOptimal.compress` / `Deflate.compress_into` orchestration with repeated
path/output capacity reuse. Level 12 uses at most ten optimization passes.

The next bounded step is to replace the supplied cost tables with the original
cost-model calculations and qualify the first pass. A separate external draft
of default/from-code/blended table updates already matches ten executed pinned
cases in original/decoded native programs; it is not part of this PR or its
permanent qualification. No new compiler change was needed for that draft.
Full byte correctness and a complete measured run come before optional tuning.
