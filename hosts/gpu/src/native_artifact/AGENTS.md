# Native execution requirements

Fast scalar native execution is a first-class maintained capability. Bool, U64,
I64 and F64 arithmetic, comparisons and conversions must retain direct native
instruction emission, checked errors and finite resource admission. Scalar-only
programs must remain efficient even when each invocation is very short.

Do not retire the efficient scalar entry because an interface is unmerged,
exported, called experimental, or described as a predecessor. Retire duplicate
machinery only after a canonical Program scalar specialization preserves the
same guarantees and demonstrates no latency or allocation regression in
optimized original/decoded tests. Output equivalence alone is insufficient.

Share the scalar vocabulary and signatures in engine `native_scalar`, preserve
private witness construction, and reuse native scalar instruction emission.
Avoid interpretation, generic per-operation dispatch, per-operation allocation,
or weaker accounting to obtain consolidation. Keep any unresolved performance
regression explicit in the cleanup ledger and PR; do not mark retirement done.
