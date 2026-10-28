# Native private unsigned indexed-write recurrences

Baseline compiler: `efa2ff81741362394553d5c7eae7d9dfc39766c4` (round 11).
Keep the pinned compressor and eight-byte evaluate-into timing unchanged.

The real operation is `huff-count-symbols` in the pinned Huffman component:
read a frequency, increment its counter, replace that counter and recurse with
the resulting array. The generic reproducer performs the same checked read,
increment and indexed replacement. Both return an array. At 256 symbols the
unchanged Huffman witness requests 5,629 allocations / 827,362 bytes and admits
13,846 ordinary visits (original and decoded). These System/probe observations
are diagnostics, not a breakdown of primary mimalloc compression timing.

Extend the established numeric leaf proof, emitter and finite closed-tail
protocol. Derive array roots and per-path current-version facts from the exact
private frame and selected factory dataflow. Decline old-version reads after
replacement, unknown forwarding, escapes and unsupported effects. Retain scalar
SSA state and invocation-current array owners. The first changed write uses the
original selected replacement, preserving checked geometry and admission. Then
hold exclusive fixed-shape cells and a pre-admitted future immutable publication;
subsequent successful indexed updates emit typed native stores. No source names,
table sizes or values select the class. No general per-element dispatcher.

Acceptance includes generic and actual unchanged Huffman selection, direct
loads/stores and a funded private backedge without ordinary callbacks; lost
allocation/visit slopes; all unsigned widths and rectangular indexing; aliases,
old-version exclusions, no-op/empty inputs, changing parent invocations, ordered
errors and checked overflow; finite fuel synchronization, unchanged publication,
cleanup and retry; original/decoded/restored shared JIT/Object execution. Preserve
scalar controls and independently rederive image proofs. Record every failed
attempt. Repeat paired complete small-case timing and a separate profile, then
commit the packet before another optimization. No larger input is queued.
