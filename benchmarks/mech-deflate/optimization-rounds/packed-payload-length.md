# Round 05: count packed canonical payloads from their lengths

Declared before implementation and timing. Starting head:
`615a39c5b8abb966ae7b3f85682068e024ef5efa`. Retained before executable:
`929f96c685c155f632a0b099660c4c5246e05667f389898063129b3c7247f1ae`.
Its last eight-byte result is 1692.80x Roc against a 0.000290-second reference.

The latest 1355-sample profile contains 59 exclusive samples in the packed
sequence encoder specialized for LengthSnapshotSink. Ownership admission calls
retained_footprint, which calculates canonical payload length by running the
encoder. U16/U32/U64 packed sequences therefore loop over their elements merely
to count fixed-width bytes. The real compressor reaches this path through its
262144-entry U16 tree, 1500513-word U32 cache and ordinary buffer operations.
Other packed primitive sequences have the same opportunity.

The retained reference binary's compression entry loads a pointer and length
together, then compares the length directly. Its pinned RocList ABI places
length in the second word. The applicable pattern is using representation
metadata for fixed-size storage, rather than scanning data to recover a size.
Archive the exact binary hash, disassembly and ABI excerpt with this round.

Keep the maintained canonical encoder and its byte/hash/fixed/erased sinks.
Give the existing private sink protocol a sequence operation whose default
performs the unchanged element encoding. Only the length sink specializes
fixed-width packed sequences to saturated count-times-encoded-width addition.
Use logical length, not reserved capacity. Strings, composite values and unit
sequences retain their existing traversal. No new value field, resource
authority, serialization format, execution route, allocation or public API is
introduced. Ordered work/admission visitors remain unchanged.

Require every packed kind, raw floating/complex bits, rational values, signed
and unsigned widths, Boolean/Id/Index, nested composites, variable strings,
empty sequences, spare capacity and saturation to agree with actual byte
encoding and existing erased-sink behavior. Keep ownership bytes/nodes,
exact/one-short finite admission, aliasing, rejection and retry regressions.
Run core snapshot/memory, selected native/component and architecture checks.

Keep the same selected-operation build profile, source, artifact, allocator,
eight-byte harness and evaluate_into-only timer. Refresh 20000 reference
observations; retain one warmup, three primary observations and the excluded
extra call for both subjects. Require exact raw output, independent inflation,
aliases, output capacity 13, logical work 941491317987 and zero terminal managed
bytes/nodes. No compiler may overlap timing. Run the unchanged 24-observation
real-buffer allocation witness and protected original/decoded Bool/U64/I64/F64
controls (unchanged allocations/bytes and median ratios <=1.05), plus the
zero/64-visit scalar-loop zero-allocation-slope control. Profile the new binary
in a separate two-second sample after primary timing. Do not time a larger
input. Commit all observations and failures before the next optimization.
