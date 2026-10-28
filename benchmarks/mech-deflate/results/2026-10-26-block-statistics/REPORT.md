# Executing collector prefix and checked counter merges

The real six-position collector completes. The original 259-entry U32 frequency
merge and ten-type observation merge also execute and match the pinned original
functions. There is still no complete block collector, compressor or compression
time.

## Exact changes

- Base: `4a85f1d339832607147fae08d013e3aa60dd24a5`.
- Checked U32 addition: `fb3f8f5d11690f239a48f8b1f2cdd619bb113a7f`.
- Source port and component qualification, current compiler/source checkpoint:
  `799f26b3765e8b807775a627b2ba23e1eb010a7c`.
- Pinned original implementation: `671077457ff7ef3a3d44caf715a3fc595addf379`;
  original compiler `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`.
- Artifact revision 14; scalar IR 8; program IR 108; program ABI 35; work policy 83.

The parent compiler, with only the new test overlaid, accepts checked U32
addition in resident execution but rejects native lowering with
`MissingSemanticWitness { node: NodeId(0) }`. The exact source requirement is
`left<u32> + right<u32>`, used by `CompressOptimal.merge_match_len_freqs` and
`CompressLazy.merge_observations`. The maintained numeric leaf now certifies that
exact scalar operation. Shared emission adds in the validated native lane and
checks against U32's maximum before publishing the result. Other arithmetic
paths retain their existing behavior. The new executable-image operation is
covered by an actual linked runtime-only consumer.

The source port retains U32 counter buffers, all 259 frequency indices, all ten
observation types, zero-frequency write skipping, fresh-range clearing and
checked count addition. It introduces no new buffer initialization sites. The
existing tree/cache, search/skip, near-EOF source and all expected vectors and
allowances remain unchanged. The original functions used by the reference are
byte-identical to the pin; [provenance](provenance.json) records their hashes.

## Validation and measured execution

Commands use `nightly-2026-03-03`, locked offline dependencies, one build job,
`mech-gpu --no-default-features --features canonical-native-program`, and the
maintained warnings-as-errors policy. Exact commands, configurations and counts
are in each JSON/log pair.

| Receipt | Observed outcome |
| --- | --- |
| [Minimal parent witness](checked-u32-add-parent-witness.json) | Expected failure: resident results established, native missing witness; 0 passed, 1 failed |
| [Minimal operation after fix](checked-u32-add-focused.json) | 1 passed; direct/buffer reads, unsigned boundaries, overflow, aliases, cleanup and retry; original/decoded |
| [Counter merge and operation](checked-u32-statistics-corrected.json) | 3 passed, 0 failed/ignored; 9.13 seconds execution |
| [Actual linked image](checked-u32-linked-image.json) | 1 passed, 0 failed/ignored; 89.57 seconds execution including runtime-only consumer build; no parser/JIT, `-D warnings` |
| [Pinned original functions](pinned-block-statistics-reference.json) | 2 new reference expectations + 1 imported expectation passed; 623.8 ms |
| [Architecture/static contracts](checked-u32-static-contracts.json) | All 12 passed, including warnings, formatting, artifact/type-memory boundaries, routing, quarantine and unsafe audit |
| [Final unchanged collector](checked-u32-final-real-collector.json) | 1 passed, 0 failed/ignored; both forms, every tree/cache word; clean checkpoint `799f26b37` |

The counter tests include overflow at the last frequency/observation counter,
after earlier writes; valid → rejected → valid execution; nonzero fresh offsets;
U32 overflow before an also-failing U64 count addition; unchanged published
outputs and input aliases; and zero retained managed bytes after teardown.

The full 259-counter merge took 0.132927542 seconds original and 0.130994292
decoded on the first valid invocation. It requested 144874/144873 allocations
and 11728406/11728342 cumulative bytes. These are debug-host diagnostic
observations, not release benchmark samples or peak-memory measurements.

On the final compiler, the unchanged real-buffer trace again completes all six
positions from the 320-byte input, taking 14.278137041/14.314659583 seconds for
original/decoded. Each uses 9793861508 work units, 357598325 comparison units and
19781 control visits. Allocation requests are 617047/617046, cumulative requested
bytes 478106375/478106311. The full 262144-entry U16 tree and 1500513-word U32
cache, depth 300, nice length 258, 10-billion work limit, 256 MiB managed and
128 MiB temporary limits are unchanged. Compilation and expected-state checking
are outside the reported evaluation times. The host is unoptimized; generated
code uses Cranelift `speed`.

The previous denial was a computation-budget rejection, not a buffer-memory
rejection. [Selected length lowering](../2026-10-26-selected-length/REPORT.md)
removed unnecessary general binding and scalar-result construction. Remaining
argument, aggregate and update overhead is large; this is not evidence of good
whole-compressor performance or proof that the cost model is exact.

## Remaining work and failures

The missing native U32 witness is an inherited compiler gap at the exact parent,
now fixed. A new observation test initially matched `Primitive` on the outer
action enum instead of its `Scalar` child and failed Rust compilation; the
[failed receipt](checked-u32-statistics-final.json) is retained. The corrected
test passes and verifies the exact failing primitive. No unresolved failure
remains in these focused commands.

The broad component suite and independent feature matrix were not rerun here.
No CI was triggered or awaited, no PR was created, and no release measurement,
full-file compression, independent decompression or reymont run occurred.

Permanent executing source now includes the matchfinder helpers, three extension
loops, initialization/rebase, separate binary-tree search/skip, consecutive
prefix collection, frequency merging and observation merging. The largest
collector witness still advances only six positions in a 320-byte input.

The next bounded source step is the actual block loop: sample observations
before cache headers, finish the complete skip before a block decision, then
rewind/carry over at the original boundary. The retained-capacity U64 path/U8
output operations, optimal parser, block writer, full Huffman integration and
`compress_into` remain to be connected. Any new compiler change requires an
exact blocking operation and minimal acceptance witness first.
