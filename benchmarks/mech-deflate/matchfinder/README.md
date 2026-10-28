# DEFLATE binary-tree match finder


## Executing stored-block writer

`block-write-stored` and its consecutive-block loop now write the pinned stored
(uncompressed) DEFLATE representation. Nine cases match reference bytes and bit
state in original and decoded artifacts, including partial-byte carry and the
65,535-byte split. Five complete streams per artifact independently decompress
to the expected input. The largest payload is 65,536 bytes, written as two stored
blocks; this does not establish level-12 compression of that input.

Every case retains the real 6,633,832-byte output reservation through internal
return, clear, and append. Changing capacity from 131,072 to 6,633,832 changes
allocated bytes by exactly 6,502,760. The 65,536-byte publication witness takes
about 1.07 ms unoptimized, including its output reservation and publication.
The separate internal-reuse observation takes about 0.66 ms; these differ in
observable outputs and are not an optimization comparison or compressor timing.

This new test uses the existing full-block output-element bound of 1,500,513.
It retains a rejection/cleanup check for the generic 65,536 bound, which cannot
hold a 65,535-byte payload plus its five-byte header. Existing test allowances
and compiler defaults are unchanged. [Receipts and exact scope](../results/2026-10-27-stored-writer/review.json)
record both runs. Fixture compression is only golden-file storage and is decoded
before measurement; the compiler executes the writer itself.

Compressed-block payload connection, full compressor integration, complete
correctness, and the first level-12 benchmark remain.


## Stored payload range append

The stored-block path requires `List.append_sublist` at pinned
`BlockOut.roc:603`. The new generic `sequence/append-sublist(destination, source,
start, count)` operates on packed U8/U16/U32/U64 rows. It clamps start to the
source length and count to the remaining length, including U64::MAX inputs.
The source remains borrowed. A uniquely owned destination retains its allocation;
shared destinations preserve aliases. An empty range retains the exact destination
owner when its schema already matches.

The operation uses the existing maintained contract/catalog, managed runtime,
resident kernel and derived native buffer-ownership path. Native IR115 adds the
second managed input to the selected call witness. The native helper borrows that
input before retiring destination slots, so self-append cannot free its source.
The copied source range is charged as cloned bytes; there is no intermediate
sublist allocation or repeated whole-output concatenation.

The [requirement and receipts](../results/2026-10-27-range-append/requirement.json)
include the failing parent acceptance test. Original/decoded native tests retain
6,633,832 bytes of output capacity and copy every byte at the 65,535/65,536 boundary.
The 65,536-byte calls take approximately 0.23–0.29 ms in the unoptimized witness,
including construction of the reserved output. This is a range-copy measurement,
not compression. Small/real reservations request the same 213 allocations and
differ in cumulative allocated bytes by exactly 6,633,320.

Clamping, source/self aliases, zero-count identity, resource-denial recovery,
unchanged prior publication, managed factory signatures, current cached ranges,
and cleanup are qualified. Resident comparisons use the small reservation under
its existing limits; the real reservation runs natively and in the linked test.
Full block assembly, complete compressor integration and correctness still
precede any compression timing.


## Block cost and compressed-header assembly

`block-compute-costs` now executes the block writer's exact dynamic, static,
and stored cost calculation. `block-choose-kind` preserves ties in stored,
static, dynamic order. Twenty-four executed reference cases cover all three
choices and the 65,535/65,536 stored-block length boundary; seven explicit
cost ties are checked separately. Those lengths are cost inputs, not completed
compression sizes.

`block-write-static-header` and `block-write-dynamic-header` match bytes and
remaining bit state in 32 executed reference cases, including final/nonfinal
headers and partial-byte carry. Both use the real 6,633,832-byte output
reservation. All cases pass original and decoded artifacts in the maintained
canonical-native-program profile. The three tests take 65.50 seconds; these
include compilation and qualification work and are not benchmark timings.
[Receipts and scope](../results/2026-10-27-block-assembly/review.json)
retain the initial test-helper shape failure and its correction without a
compiler change.

Block assembly still needs stored range append, payload writer connection,
end-of-block and final draining. Full compressor integration and complete-stream
correctness must precede the first benchmark. No optional tuning was undertaken.


## Executing sequence and optimal-path writers

The sequence writer and optimal-path writer now execute as compiled Mech
functions using the original sequence records and U64 path nodes. Both preserve
bit-buffer carry and byte flush order. Four maintained tests compare bytes and
remaining bit state against 26 pinned reference cases, in original and decoded
artifacts. The largest supplied path block is 320 bytes; the largest sequence
input is 259 bytes. These are writer components, not complete compression.

Separate internal return/clear/append checks retain the 6,633,832-byte output
reservation for every case. After equal metadata warmup, changing the reservation
from 512 bytes changes allocated bytes by exactly 6,633,320. Publishing the whole
result record canonicalizes nested storage, so capacity is qualified before that
publication boundary. [Receipts and reproducibility evidence](../results/2026-10-27-writers/review.json)
include the unchanged reference cases and precise qualification scope.

The next step connects block cost/type selection, header output, stored payloads,
end-of-block, and final bit draining. Full collector/parser/controller integration,
complete-stream equality and decompression, and the first benchmark still follow.


## Mixed record-list and buffer frames

The sequence writer carries a byte output buffer alongside a list of fixed
sequence records. At compiler base `88605b57230b5a76c36f58f3977b3261fdbe47c8`,
that combination fell back to canonical snapshot packing: a reservation of
6,633,832 bytes became 64 after return, clear, and append. The minimal witness
and unchanged-limit receipts are in
[the mixed-frame evidence directory](../results/2026-10-27-mixed-buffer-frames/).

Private frame eligibility now accepts an immutable matrix of fixed record or
tuple elements beside an ordinary unsigned buffer. Existing ownership lanes,
turn lifetime checks, and connected-component eligibility still apply. The
aggregate matrix itself is not classified as a packed word buffer. Native IR
114 prevents reuse of images planned with the former eligibility rules.

Original and decoded execution retain exactly the requested 512 or 6,633,832
bytes. After equal metadata warmup, both sizes request 5,842 allocations; their
allocated-byte difference is exactly the reservation difference, 6,633,320.
The regression checks aliases, failure after a previous publication, retry,
cleanup, and a zero cloned-byte allowance. The resident comparison uses 512:
the full reservation exceeds its unchanged temporary-byte admission limit
(33,171,230 required versus 16,777,216 allowed). This is not a full compression
measurement.

## Record argument checkpoint: 2026-10-27

Fixed record/tuple elements can now cross turn-shaped matrix argument and capture
boundaries. The compiler retains the existing source lifetime gates, canonical
artifact/codec validation, structural projections and budgeted snapshot
construction. Shape parameters may have different indices in the caller and
callee; exact element schemas and constant extents must still agree, and each
live shape is closed by the maintained binder. Aggregate cells do not enter the
packed unsigned-buffer copy path. Nonrecursive immutable captures are unchanged.
Native program image IR is 113; the base is
`610d5642100f35330512567591918a12fb393141`.

[Requirements and failure evidence](../results/2026-10-27-record-arguments/requirement.json)
trace this to the level-12 all-literal writer's sequence-record loop. The draft
sequence writer now matches 14 pinned cases in original and decoded execution,
with the real 6633832-byte output reservation. The independently drafted path-node
writer matches 12 cases through 320-byte supplied blocks. These are component
results, not complete compression or benchmark timings; their permanent component
registration is the next integration step.

A separate tuple-of-record projection rejection arose from the draft helper's
return packaging. The pinned source has local loop variables, so the helper now
returns one state record without changing its operations or arithmetic. That
incidental projection gap was recorded and left outside this compiler patch.


## Record selection checkpoint: 2026-10-27

The block writer needs to select a sequence record and read its U32/U16 fields.
This path is reachable at level 12: the all-literal decision in
`CompressOptimal.roc:1109–1137` sets `use_items = 0`, selecting
`BlockOut.write_sequences` through `BlockOut.roc:748–753`.

Native scalar matrix selection now admits the same fixed-schema record and tuple
values already admitted for matrix construction and indexed replacement. It
retains exact element/output schema matching and positional selectors, and uses
the existing resident selection implementation. Dynamic child containers remain
outside this native subset. Native program image IR advances from 111 to 112;
there is no new bytecode operation or runtime kernel.

The base is `27bdd8b2c5b1ad35590121653589d06251ae5161`.
[Reproducer and requirements](../results/2026-10-27-record-read/requirement.json)
identify the original resident-success/native-rejection mismatch.
The tests cover original/decoded artifacts, resident/native agreement, nested
fields, index errors and overflow, unselected reads, aliases, unchanged prior
publication, quota denial, cleanup and recovery. The linked executable also
checks assertion-before-arithmetic failure and retry.

This does not complete sequence writing. Passing a record matrix into a recursive
function or lexical selected branch still rejects at source validation; its
[separate reproducer and acceptance criteria](../results/2026-10-27-record-read/next-gap.json)
are recorded before expanding compiler work. The drafted writer and 14 reference
cases have not produced a Mech execution result. Full block writing/choice,
compressor integration, complete byte/decompression checks and first timing
remain outstanding.


## Byte output checkpoint: 2026-10-27

`sequence/append-le-bytes(bytes, word, count)` now supports appending 0–8 low
little-endian bytes to an existing U8 sequence. Counts above eight reject before
mutation. Native execution preserves exclusive backing capacity, grows once for
the complete count, and keeps aliases immutable. The source type retains its row
axis; compatible zero-count calls retain the exact admitted owner, shape and
receipt token. Statically sized inputs requiring output-schema adaptation still
use the admitted copy fallback.

The implementation revision is `3e262c87aac0c0ae49b689561cd53bcfa4b34eaa`, based on
`8fd668d6eb11a49f32790dca11cb9f9b22e6db38`. Native program image IR is 111.
[Receipts and failure dispositions](../results/2026-10-27-byte-append/review.json)
cover eight engine tests and twelve host/linked tests. The real 6633832-byte
reservation and a 16-byte reservation each use 264 allocation requests in the
append/append/zero-append/clear/append witness. Cumulative allocated bytes differ
by exactly 6633816, so only the initial payload reservation scales with capacity.
Original and decoded artifacts produce the same result. These are allocation
checks, not compression timing.

```sh
cargo test --locked --offline -j1 -p mech-engine --no-default-features \
  --features full_source,native-artifact,sequence_buffers \
  --test sequence_buffer_contract -- --nocapture --test-threads=1
cargo test --locked --offline -j1 -p mech-gpu --no-default-features \
  --features canonical-native-program --test canonical_artifact_program -- \
  sequence_capacity_qualification linked_bulk_byte_append --nocapture --test-threads=1
```

This removes the confirmed bulk-byte-output blocker for `write_sequences` and
`flush_block`. It does not implement those functions or complete `compress_into`.
Block writing/choice, multi-block integration, exact compressed-byte validation,
independent decompression and the first full benchmark remain outstanding.

## Component checkpoint: 2026-10-27

The maintained source components now include default cost selection and updates,
dynamic-header construction and exact costs, literal-only and fixed codes,
the dynamic optimization loop, and writer packed/full-length tables. Their
qualification modules are registered in `canonical_artifact_program`; they no
longer require temporary source paths. The reference generators in this directory
accept `reference_root`, `--roc`, `--work-dir`, and `--out`, and validate all pinned
package sources before executing the original functions. `generate_costs_reference.py`
and `generate_default_costs_reference.py` reproduce the previously qualified
10 and 8 cases without changing their expected values.

Run the new component qualification with:

```sh
cargo test --locked --offline -j1 -p mech-gpu --no-default-features \
  --features canonical-native-program --test canonical_artifact_program -- \
  costs_qualification default_costs_qualification header_qualification \
  literal_choice_qualification optimization_qualification writer_tables_qualification \
  --nocapture --test-threads=1
```

The optimization qualification runs both supplied-cache cases (up to 320 bytes)
and Mech's own collector followed by optimization (8-byte literal and overlap
cases). It compares the returned path, current and saved models, frequencies,
codes, costs and pass counts. It retains the real 262144-entry U16 tree,
1500513-word U32 cache, 305257-word U64 path reservation, depth 300 and nice
length 258. Explicit diagnostic work and compilation allowances are unchanged;
these are not production compiler defaults. Reported allocations are cumulative
requests, not peak live memory.

These are compressor components, not a complete compressor or a compression
benchmark. Remaining work includes `write_sequences`, block choice
and `flush_block`, observation-driven carryover and `compress_into`. Complete raw
bytes, independent decompression and full-input timing have not been established.
Earlier sections below describe historical checkpoints and failures that led to
these components.


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
one-based and checked. A byte outside the input defaults to zero. A four- or
eight-byte load defaults the entire word to zero when its window is unreadable,
including a partial trailing window or a U64 maximum offset. It never zero-pads
a partial word or forms a one-based index on that fallback path. `lz-hash` requires
1–32 hash bits. As in the C function,
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
shape changes, whole-word zero fallback for unreadable windows, preserved prior
output aliases, and later valid reads. Ordinary indexed-read failure/retry tests
remain independent of these defaulting loader helpers.
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

The complete block collector, optimal parser, block writer, efficient buffer ownership, and `reymont`
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

Ordinary nonrecursive checked calls now capture their immutable arguments rather
than copying them into an argument tuple. The lexical-entry tests require both
resident and native full-table invalid-window calls to reach the window assertion
under the ordinary policy. No enlarged allowance hides an entry-frame copy.
The [allocation diagnostic](../argument-transport/README.md) measures this with the
real table and full cache. Recursive carrier and updated-buffer costs remain.

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

Obtain a complete correct baseline before optional performance tuning. Record
the remaining word-load, prefetch, wrapping-conversion, and generated-code
differences alongside that baseline. Fix compiler paths that prevent execution
using the unchanged real-buffer witness before proceeding.
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

## Whole-word fallback reproduction

The standalone [reference expectations](word-load-reference.roc) pin valid loads,
partial windows, empty input, and maximum offsets using the original builtins.
They run with compiler revision `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d` in
optimized mode. The shared reference routines at
`671077457ff7ef3a3d44caf715a3fc595addf379` use those loads with `?? 0`.

```sh
ROC_CACHE_DIR=/tmp/mech-word-reference-cache roc test --opt=speed --verbose \
  benchmarks/mech-deflate/matchfinder/word-load-reference.roc
```

The Mech regression executes original and decoded artifacts in both engines,
including changing input extents and valid reads after zero fallbacks. Existing
C oracle values, transition cases, search and skip signatures, entry assertions,
limits, and qualification allowances are unchanged. The byte assembly remains
explicit; this correction does not claim native bulk-load or prefetch parity.

## Consecutive real-buffer prefix

[The prefix source](../../../hosts/gpu/tests/fixtures/native/deflate-collector.mec)
initializes the 262144-entry U16 tree and 1500513-word U32 cache in Mech and
completes six consecutive positions in one compiled computation. The input is
320 copies of byte 97, with depth 300 and nice length 258. The first three
positions search; the next three skip. This is a bounded state-update witness,
not a complete block collector: observations, block decisions and carryover
still need to be connected.

The [current checkpoint](../results/2026-10-26-block-statistics/REPORT.md) records
the completed original/decoded trace, full-vector agreement and unchanged
allowances. The current compiler is `799f26b3765e8b807775a627b2ba23e1eb010a7c`.
Earlier admission failures below are historical.

The [reference witness](collector-prefix-reference.roc) calls the pinned search
and skip implementations and checks every tree and cache entry. Copy it beside
`Matchfinder.roc` and `BtMatchfinder.roc` in the pinned package, then run it with
the compiler revision specified above:

```sh
cp benchmarks/mech-deflate/matchfinder/collector-prefix-reference.roc \
  /path/to/pinned/roc-deflate/package/CollectorPrefixWitness.roc
roc test --opt=speed /path/to/pinned/roc-deflate/package/CollectorPrefixWitness.roc
```

The Mech test keeps initialization inside the measured call, reports allocation
requests and elapsed time, and compares the full state outside measurement:

```sh
cargo test --locked --offline -j1 -p mech-gpu --no-default-features \
  --features canonical-native-program --test canonical_artifact_program \
  collector_qualification -- --test-threads=1 --nocapture
```

## Block counter merges

The [block-collector source](../../../hosts/gpu/tests/fixtures/native/deflate-block-collector.mec)
now contains the original 259-entry U32 frequency merge and ten-type observation
merge. Zero fresh frequencies skip their writes; observation merging updates all
ten counters and resets the original fresh range. Both retain checked U32
addition. The full block-collection loop is still being assembled.

The minimal blocking operation was `left<u32> + right<u32>`: resident execution
already accepted it, but native lowering rejected it as `MissingSemanticWitness`
at the preceding compiler checkpoint. The maintained numeric leaf now certifies
that exact scalar operation. Shared native emission checks the result against
U32's maximum before storing it. Scalar IR 8 and program IR 108 include the new
operation; program ABI 35 and work policy 83 are unchanged.

`checked_u32_add_keeps_frequency_width_overflow_and_recovery` exercises direct
arguments and buffer reads on original/decoded artifacts. The block-statistics
tests execute all 259 counters, late overflow after earlier writes, nonzero
observation offsets, U32 overflow before an also-failing U64 count addition,
unchanged published outputs/input aliases, cleanup and recovery. The linked U32
addition test restores the operation from its executable image in a runtime-only
consumer. These are bounded regression checks, not a full compression benchmark.

The [reference witness](block-statistics-reference.roc) calls the pinned original
merge functions and independently checks the same full vectors and totals.
Copy it beside the pinned package's modules and run:

```sh
roc test --opt=speed --verbose /path/to/pinned/roc-deflate/package/BlockStatisticsWitness.roc
cargo test --locked --offline -j1 -p mech-gpu --no-default-features \
  --features canonical-native-program --test canonical_artifact_program \
  block_statistics_qualification -- --test-threads=1 --nocapture
```

## Earlier checkpoints

The initial executable attempt completed no search positions. Cache construction
rejected output admission at 16777228 bytes against the unchanged 16777216-byte
limit. The generic comprehension builder stages scalar drafts instead of packed
U32 cells. Its actual execution requested 101445977 allocation bytes in 397
requests and took 3.51036625 seconds in the unoptimized Rust test profile. The
failure left publication empty and retained memory unchanged. This is a compiler
representation blocker, not a compression performance result.

Unsigned comprehensions now accumulate U8/U16/U32/U64 cells at their exact
width and transfer the completed backing to the existing canonical constructor.
The standalone 1500513-word U32 initialization succeeds for original and decoded
artifacts under the same limits. In the unoptimized test build it took
6.870867625 and 6.874258709 seconds respectively, with 176 allocation requests
and 22798145 requested bytes per call. The regression also checks every word,
output rejection after success, unchanged published aliases, cleanup and retry.
The selected collection, recursion, assertion, projection and component run
passed all 113 tests without changing component expectations or allowances.

Recursive unsigned argument projection and aggregate field reads now use the
existing packed constructors. Their standalone real-buffer regressions complete
for original and decoded artifacts, including late failure, unchanged prior
publication and recovery. Field reads with both real buffers take about 0.044
seconds and request 20384163 bytes across 9045 allocations in the unoptimized
test build. The source/artifact checks and maintained access factory remain.

Plain unsigned indexed updates now construct a packed replacement after the
existing bounds, routing and mutation admission. The maintained implementation's
memory class admits its packed constructor instead of scalar drafts; other
output kinds keep the canonical finalizer bound. Updating either end of the real
1500513-word U32 buffer succeeds for original and decoded artifacts, takes about
1.03 seconds and requests 6157958 allocation bytes. Bounds rejection preserves
the preceding publication and input aliases, and the next valid update succeeds.
Rectangular indexing is checked for U8/U16/U32/U64 buffers as well.

The earlier collector attempt got past cache writes but had no completed six-position
result. At `core/composite-pack` (node 0, block 243, local 0), managed memory
requires 274541914 bytes against the unchanged 268435456-byte limit. Execution
took 15.377818042 seconds and requested 231881425 bytes across 195482 allocations.
Publication remained empty and retained memory was unchanged. Intermediate
positions are not published, so their completed count is unreported on failure.
Requested allocation bytes are cumulative heap requests, not peak memory or
measured copy bytes. This was the next compiler blocker; it is not a compression
timing. All resource allowances and compressor expected values remain unchanged.

Removing draft construction changes representation-dependent budget receipts:
the shallow packed projection must fit below its old draft expansion, and the
default resident Huffman call reaches ComparisonWork admission (68699 against
65536) before ComputeWork. Those tests retain their rejection, publication,
cleanup and retry checks. The broader run passed 116 tests; the precise updated
Huffman budget check then passed separately. No compressor reference vector or
qualification allowance was changed.

## Invocation accounting and initialization checkpoint

Optional per-site bindings could exhaust managed capacity before a mandatory
call scope was admitted. A small original/decoded record-packing regression
reproduces that failure with two three-element buffers and twelve cache slots.
New bindings now prove that the existing finite temporary allowance can coexist;
otherwise the same selected leaf executes uncached. Binding prices, call limits,
publication boundaries and failed-call debits are unchanged. Changing scalar
arguments, invalid input, a tighter enclosing work scope, cleanup and same-account
retry are covered. This gets the real-buffer witness past its earlier memory
refusal, but does not establish a completed collector prefix.

Generated reference copies also repaid full payload destruction even though
their source arena owner had already funded that one destruction. A private
same-arena copy now retains the immutable owner and pays fixed alias work. It
does not change `Value` construction or admission, create a payload copy, or
allocate a new budget wrapper. Generation, schema, account and finite-work checks
precede replacement. Native work policy 81 identifies the new accounting path;
the program IR, ABI, frame and slot layouts are unchanged.

The source regression relays a 1500513-word U32 buffer 64 times. The parent hits
the unchanged one-billion native work limit. The new path completes original
and decoded artifacts, retaining the input's exact frozen storage. The measured
call requests 16254 heap bytes across 130 allocations. Fresh values, assertion
failure, unchanged prior publication and retry are checked. These measurements
describe reference transport, not compression or removal of aggregate copies.

The six-position collector still reaches its ten-billion work limit. Its latest
uninstrumented run took 17.0618445 seconds and requested 285481056 heap bytes in
294081 allocations before rejection. Publication remained empty and retained
memory returned to its pre-call value. The completed-position count remains
unreported because failed execution does not publish intermediate state.

The separate zero-position trace executes the same initialization and returns
the complete state. Every tree and cache entry is checked. Original and decoded
artifacts each consume 5981116713 work units and 12338685 generated control visits.
They take 8.595357125 and 8.583951292 seconds respectively in the unoptimized
test profile, requesting about 61.2 MB cumulatively. Initialization and returning
the state are inside measurement; compilation and checking are outside it.

The next bounded compiler requirement is efficient general constant-yield
unsigned collection construction. Pinned source revision
`671077457ff7ef3a3d44caf715a3fc595addf379` uses `List.repeat` in
`Matchfinder.roc:36` and `CompressOptimal.roc:718`. The existing minimal source
reproducer is:

```mech
repeat(value<u32>, count<u64>) => (out<[u32]>) :=
  out := [value | i <- 0u64..=count, i < count].
repeat(word<u32>, count<u64>)
```

The maintained acceptance test is
`large_unsigned_comprehension_initializes_a_real_buffer_and_recovers_after_denial`.
It covers zero, small and real cache lengths, nonzero cells, original/decoded
artifacts, resident agreement on small inputs, refusal after publication and
recovery. Any bulk construction must derive eligibility through shared same-plan
facts, retain source error/effect order and finite resource admission, and reject
unproved/effectful bodies. No benchmark names or fixed buffer lengths select it.
The unchanged six-position witness and its complete reference vectors remain
the integration acceptance test. This construction improvement was absent at
`fd790ca0`. The subsequent [ordinary uniform-construction patch](../uniform-words/README.md)
implements the precise pure region through shared facts and the existing packed
builder. Its real-buffer initialization qualification executes 76 control visits;
the six-position witness still rejects its unchanged work limit during a
`sequence/length` admission. Reserve/append/clear with retained capacity, the full
collector, optimal parser, writer, `compress_into`, and complete compression
timing remain.
