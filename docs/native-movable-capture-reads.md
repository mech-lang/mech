# Movable reads from immutable numeric captures

IR248 lets the native optimizer reuse and move reads from a checked subset of
direct immutable ancestor captures. It applies inside outlined numeric callees.
The root has admitted each current view before calling that callee, and resumes
ordinary source execution only after the callee returns.

The compiler excludes mutable arrays, carrier arrays, frame captures,
values defined inside the region, publication destinations,
and any canonical slot a selected write could retire. The remaining ancestor
slot keeps its backing alive throughout the call. A writable alias must use
the existing copy-on-write path while that owner remains live. The selected
first-change callback cannot retire this slot or mutate that backing.

Read addresses are safe without relying on the source selector branch. The
logical ordinal is clamped to the last initialized element before the existing
column-major to row-major translation. Empty views use a nonzero safe divisor
without changing their original logical dimensions or source guards. An
empty capture lends a static, immutable zero cell and retains logical length
zero. Even a raw zero or maximum unsigned selector therefore produces a valid
physical address. This permits readonly, nonfaulting and movable native loads;
it grants no disjoint memory class or extra alignment.

The original source bounds branch, rejecting library operation, arithmetic
checks, finite work debits, output publication and retry paths remain. An
out-of-range source selector still fails at the same source operation. Inlined
numeric bodies and captures without the lifetime proof retain ordinary loads.

Tests execute unguarded native loads for all four unsigned widths, empty and
nonempty views, extreme selectors, real shared-owner writes across the cold
materialization callback, source failures, prior publication, decoded artifacts,
restored execution, finite boundaries and retry. Performance remains an open
measurement until an eligible cohort demonstrates improvement in both scopes.
