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

This refinement requires the guarded-load optimization. Its ablation retains
trapping physical load effects throughout numeric reads, including captures.

The original source bounds branch, rejecting library operation, arithmetic
checks, finite work debits, output publication and retry paths remain. An
out-of-range source selector still fails at the same source operation. Inlined
numeric bodies and captures without the lifetime proof retain ordinary loads.

Tests execute unguarded native loads for all four unsigned widths, empty and
nonempty views, extreme selectors, real shared-owner writes across the cold
materialization callback, source failures, prior publication, decoded artifacts,
restored execution, finite boundaries and retry. Performance remains an open
measurement until an eligible cohort demonstrates improvement in both scopes.
## Select reads after loop-parameter cleanup

IR250 integrates PR973's unsigned interval predicates and selects reusable
capture reads after native local-phi cleanup. A bounded recursive query accepts
selectors defined before the innermost natural loop, or pure integer expressions
of those values. Advancing selectors, unknown producers and single-use reads
retain ordinary source-guarded addresses and memory flags. The removed clamp
results remain aliases for the following unsigned-extent analysis.

This is a profitability decision. Its safety permission still comes from the
same direct ancestor ownership proof and independently safe initialized address.
Retained clamps use unsigned min/max data flow; source comparisons cannot become
their only protection. Source arithmetic, errors, effects, debits and publication
are unchanged. No runtime cache, counter, allocation or dispatch is added.

The executable selection test combines an invariant read with an advancing read,
checks actual load flags and retired clamp aliases, verifies the native IR and
compares optimized execution across loop bounds and extreme invariant selectors.
The current-window fixture now requires advancing reads to decline selection;
the shared-owner fixture still requires actual selected immutable reads. Empty
matrix, finite-prefix, error and retry cases remain exercised.

Performance is unmeasured until the frozen candidate completes qualification and
both continuous post-JSON and complete-process randomized cohorts.

