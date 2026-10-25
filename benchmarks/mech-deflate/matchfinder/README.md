# DEFLATE match-finder helpers

The [Mech implementation](../../../hosts/gpu/tests/fixtures/native/deflate-matchfinder.mec)
provides the shared routines needed by the level-12 binary-tree match finder:

- Four-byte little-endian sequence loads and eight-byte comparison loads.
- The wrapping 32-bit sequence hash.
- Match extension using whole-word XOR, trailing-zero count, and a byte tail.
- Node initialization, window rebasing, and wrapping absolute-index recovery.

Initialization uses a filtered comprehension. Its inclusive range has one extra
candidate, excluded by the filter, so zero entries work without an empty numeric
range (which Mech rejects). Rebasing broadcasts a pure scalar statement-body
function over the node matrix. The compiler previously limited this matrix lift
to pattern-body functions; it now uses the same shape-preserving collection path
for expression and statement bodies as well. The sequential match-extension
loop stops at the first unequal byte.

Positions in these helpers are zero-based; their actual Mech matrix reads are
one-based and checked. `lz-hash` requires 1–32 hash bits. As in the C function,
`#Extend` requires `start <= maximum`, `match-at < str-at`, and enough input at
both positions. The first `start` bytes are already matched. DEFLATE uses a
maximum match length of 258. The loaders currently assemble words from checked
byte reads; they do not yet emit one unaligned machine load.
The Mech loop also lacks C's initial four unrolled word comparisons.

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

The table-capacity check explicitly permits 262144 output elements and ten
billion finite execution and collection work units for the match finder's tables.
The ordinary host profile retains its 65536-element cap. Work units include
admission and publication and are not machine instruction counts or timings.
Other qualification limits and host defaults are unchanged.

The full binary-tree match finder, optimal parser, block writer, efficient buffer
ownership, and `reymont` compression timing remain outstanding. These component
checks record no compression performance result. Source licenses are retained
in [the shared notices](../../../hosts/gpu/tests/fixtures/native/deflate-huffman-LICENSES.md).

## Comparison requirements

The target is the pinned Roc implementation above, at level 12 on the same
6,627,202-byte `reymont` input. Keep its binary-tree tables, search limits,
optimal parsing, block decisions, and Huffman construction. Mech must execute
the compressor itself; the C reference generator is never a timed compressor.
Compiler work should support the equivalent Mech program, including ordinary
collection operations and buffer updates, rather than change the algorithm to
suit the backend.

Before timing, finish the known word-load, first-four-word comparison, and
buffer-ownership differences. Match the Roc harness's output-buffer reuse and
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

This does not provide efficient array ownership. General nested tail calls and
dynamic declared-FSM payload kinds also remain outside the current native path;
they must not be bypassed by changing the compression algorithm to fit it.
