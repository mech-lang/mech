# DEFLATE binary-tree match finder

The [Mech implementation](../../../hosts/gpu/tests/fixtures/native/deflate-matchfinder.mec)
provides the shared routines needed by the level-12 binary-tree match finder:

- Four-byte little-endian sequence loads and eight-byte comparison loads.
- The wrapping 32-bit sequence hash.
- Match extension using whole-word XOR, trailing-zero count, and a byte tail.
- Node initialization, window rebasing, and wrapping absolute-index recovery.

The [binary-tree implementation](../../../hosts/gpu/tests/fixtures/native/deflate-binary-tree.mec)
adds Roc's complete one-byte search and skip walks. It retains the combined
262144-entry U16 table (children, hash4 heads, two-way hash3 heads), increasing
match-length cache entries, common-prefix bounds, subtree rebuilding, depth
limits, and nice-length exit. Search and skip are separate compiled functions;
the skip path has no U32 match-cache buffer or recording branch.

Initialization uses a filtered comprehension. Its inclusive range has one extra
candidate, excluded by the filter, so zero entries work without an empty numeric
range (which Mech rejects). Rebasing broadcasts a pure scalar statement-body
function over the node matrix. The compiler previously limited this matrix lift
to pattern-body functions; it now uses the same shape-preserving collection path
for expression and statement bodies as well. Match extension now uses ordinary
recursive functions for the same three loops as Roc: the first four words,
remaining words, then trailing bytes. A selected mismatch returns immediately.
Nested tail calls preserve the exact lexical loop target; this does not imply
that the native compiler unrolls the first four comparisons into machine code.

Positions in these helpers are zero-based; their actual Mech matrix reads are
one-based and checked. `lz-hash` requires 1–32 hash bits. As in the C function,
`extend-length` requires `start <= maximum`, `match-at < str-at`, and enough input at
both positions. The first `start` bytes are already matched. DEFLATE uses a
maximum match length of 258. The loaders currently assemble words from checked
byte reads; they do not yet emit one unaligned machine load.
Roc's explicit entry-window check runs before extension work. Search and skip
also retain its table-size and readable-window checks. These are lexical checks
on each invocation, including recursive visits, rather than document constraints.

The node representation follows Roc: unsigned U16 entries are biased by 32768,
so zero represents C's signed -32768 sentinel. Rebasing is saturated subtraction
of 32768. The reference generator converts to and from the original signed C
representation and exercises its real initialization/rebase functions.

## Qualification

The generator validates the pinned C source and header hashes, compiles the
original libdeflate functions, and writes 70 deterministic hash/match cases
and a 512-entry node table. Matching cases cover zero-length tails, changing
known prefixes, overlapping matches, unaligned positions, all byte positions
within comparison words, and the maximum DEFLATE match length. A separate
byte-by-byte oracle also checks the C fixture generation.

```sh
python3 benchmarks/mech-deflate/matchfinder/generate_reference.py /path/to/pinned/libdeflate
cargo test --locked --offline -j1 -p mech-gpu -p mech-runtime \
  --no-default-features \
  --features mech-gpu/canonical-native-program,mech-runtime/full_source,mech-runtime/resident-routing-source \
  --test canonical_artifact_program \
  -- --test-threads=1 matchfinder_qualification function_broadcast_qualification
```

Original and decoded Mech artifacts execute every C case through the native
compiler. Tests also compare resident execution, exact word values, matrix
shape changes, late bounds/overflow failures, unchanged publication, and retry.
The C implementation is only an expected-value generator.

The selected resident comparisons keep their ordinary default compute cap.
With recursive array transport, case `extend-35` reaches that cap (16777216)
before completion; the test checks and records the precise compute denial.
The same original/decoded native programs complete all 70 C cases under the
existing explicit qualification limits. This exposes copying/admission overhead;
it is not a compression timing result and no host default was increased.

The table-capacity check explicitly permits 262144 output elements and ten
billion finite execution and collection work units for the match finder's tables.
The ordinary host profile retains its 65536-element cap. Work units include
admission and publication and are not machine instruction counts or timings.
Other qualification limits and host defaults are unchanged.

The optimal parser, block writer, efficient buffer ownership, and `reymont`
compression timing remain outstanding. These component checks record no
compression performance result. Source licenses are retained
in [the shared notices](../../../hosts/gpu/tests/fixtures/native/deflate-huffman-LICENSES.md).

## Complete tree transitions

```sh
python3 benchmarks/mech-deflate/matchfinder/generate_binary_tree_reference.py /path/to/pinned/libdeflate
```

The generator executes the original pinned C search, skip, initialization, and
slide functions. It warms each input prefix with that C match finder, then saves
the complete state immediately before and after one target transition. Sparse
JSON snapshots encode zero entries implicitly; the Mech tests reconstruct and
compare all 262144 entries, including untouched entries. All 512 cache words,
the cache pointer, and both next-position hashes are checked as well. The C
driver converts its signed nodes to Roc's biased U16 representation and table
order. It initializes unused child slots to the same sentinel as Roc.
The 512-word cache is a transition-test buffer; the complete compressor must
retain the 1500513-word cache (1500000 entries plus 513 words of slack)
and its original block boundaries.

The 20 cases cover fresh tables, overlapping and periodic data, depth limits
1/2/64, multiple increasing matches, unaligned reads, maximum length five,
the second hash3 way after a deliberate hash collision, and a slid 32 KiB
window. Original and decoded Mech artifacts execute both walks, and inputs
remain unchanged. The generated skip plan has no U32 cache matrix.

These tests declare the full table extent and finite ten-billion execution,
compute, and comparison work allowances. The generic test comparison allowance
of one million fails even on the fresh search (required 1572981). The wider
allowance qualifies the real copying implementation; it does not remove that
work or change host defaults. Managed memory remains 256 MiB and temporary
memory remains 128 MiB. Efficient ownership must remove physical copies and
their associated costs before attempting the full performance comparison.

The lexical-entry tests also expose the current argument-frame copying:
resident execution of a full-table invalid-window call exceeds its unchanged
16777216-byte temporary limit before the check. The native test admits the
existing full-tree work allowances and 128 MiB temporary allowance and must
fail at the window check before later reads or updates. The test separately
requires the native generic work allowance to report the copying cost as a
compute-work denial. Small-table and scalar checks execute in both
engines. These resource denials remain ownership work, not waived costs.

Recursive statement-body functions now have fresh canonical argument bindings
and can end in a selected tail call. This lets the walks retain named local
calculations and mutable temporaries. The compiler also avoids redundant
compound conversions when finalized schemas are identical. The shared lexical
control limit is now 32, with unchanged total block/operation limits; source,
wire preflight, and native ownership validation retain the same bounded limit.
The control-section decoder replaces the generic JSON container limit with
explicit finite lexical, wire-depth, and count checks before typed allocation.

The entry assertions check table capacity and the readable search/skip window.
Separate caller preconditions are `maximum >= 5`, `5 <= nice <= maximum`, and
positive depth. Match extension also requires its already-matched prefix.
These caller preconditions are not additional reference entry assertions.
Bucket prefetch hints are still missing.
Word loads still assemble checked bytes, and
wrapping narrowing currently uses masks followed by checked conversions. These
are explicit remaining parity/optimization gaps, not a completed timing setup.

## Comparison requirements

The target is the pinned Roc implementation above, at level 12 on the same
6,627,202-byte `reymont` input. Keep its binary-tree tables, search limits,
optimal parsing, block decisions, and Huffman construction. Mech must execute
the compressor itself; the C reference generator is never a timed compressor.
Compiler work should support the equivalent Mech program, including ordinary
collection operations and buffer updates, rather than change the algorithm to
suit the backend.

Before timing, finish the known word-load, prefetch,
wrapping-conversion, and buffer-ownership differences, and inspect the
first-four-word generated code.
Match the Roc harness's output-buffer reuse and
timed boundaries: input reading, output clearing, validation, and file writing
are outside compression timing; compression's own allocation and work remain
inside. Verify decompression against the complete input and compare the emitted
stream and size with the pinned Roc implementation. Report any remaining
implementation differences alongside results. Component qualification work
units are not evidence of equivalent compression performance.

Runtime-shaped unsigned arrays can now be packed into ordinary tuple/record
outputs by the native compiler. The current child schemas determine their
separate dimensions on every binding; a prior output's dimensions do not stand
in for the new result. This removes the combined node-helper return failure.
Tuple destructuring and constant tuple reads now retain each selected array's
actual dimensions. Ordinary tail-recursive functions can carry their argument
tuples, including runtime-sized unsigned buffers, and use scalar literal patterns
such as `(xs, 0u64)`. Function-local mutable bindings are fresh values on each call.
Shape closure, packed draft expansion, and publication remain explicitly admitted.

Nested tail calls through final match expressions now route to their owning
function's loop. A helper function's own loop stays separate. Calls followed by
more work, wrong lexical targets, and unproved partial matches remain rejected.
This does not provide efficient array ownership. Dynamic declared-FSM payload
kinds remain unsupported; the compressor must retain Roc's algorithm while
remaining compiler gaps are filled.
