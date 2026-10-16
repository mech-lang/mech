# Native control lowering

The native Control frontend lowers supported canonical Mech source into typed
control-flow IR. The Cranelift JIT and object emitter share this IR and code
generator; the resident executor implements the same checked operations.

## State, selection and arithmetic

Synchronous state machines currently support one recurring state and one
terminal state, with statically typed payloads. State transitions become loops
and branches. Generated structural callbacks remain inline when ordinary source
function inlining is disabled. Asynchronous transitions, machine pipelines and
recursive machine invocation are rejected.

Postfix matches evaluate the scrutinee once and execute the selected arm.
Arithmetic errors and effects in an unselected arm do not execute. A partial
match fails if no arm matches. Transition blocks and ordinary functions retain
ordered local definitions and checked operations, including unused definitions.

Record dot selection and one-based scalar indexing use typed selectors. Local
mutation requires an explicit mutable binding and preserves its kind. An indexed
write to a supported fixed byte column produces an updated value copy; it does
not mutate an input payload or captured value. Index and byte-value bounds are
checked. General dynamic collection mutation remains outside this native subset.

Signed `i64` and unsigned `u64` addition, subtraction and multiplication check
overflow. Integer division and remainder reject zero divisors; signed operations
also reject the minimum value divided by minus one. Signed negation checks its
range. Unsigned negation is rejected. Explicit `u64`/`i64` conversions check the
destination range, and either integer kind can convert to `f64`. Ordinary `{}`
format fields retain the value's signedness.

## Ordered reductions

A comprehension sum has one accumulator for its entire source-order traversal.
Multiple generators share that accumulator; an explicitly nested sum owns a
separate accumulator. Definitions and filters retain their source order, and
filtered items do not evaluate the yield. Each function invocation owns its
accumulators, including recursive ordinary functions.

Checked sums preserve intermediate overflow behavior. Parallel range reducers
collect bounded windows of pure callback results and fold them in range order.
Signed and unsigned sum/max fields retain distinct identities and comparison
rules. These operations do not reassociate checked addition into an unordered
reduction tree.

## Artifact boundary and qualification

Control IR is an internal native compiler representation. Its object and JSON
descriptor are linked by the existing generic application builder. This path does
not yet lower the canonical `ProgramArtifact` activation model or provide its
resident scheduling integration. The Control CLI fixture uses the existing
process argument/output launcher; activated resource I/O has a separate
[documented native path](native-resource-io.md).

Focused tests execute canonical state machines, matches, indexed value copies,
checked arithmetic, casts and ordered reductions through resident and native
JIT execution. An explicit linked qualification test emits the unchanged
Fannkuch machine source with `MECH_CONTROL_INLINE=0`, builds one executable and
checks complete outputs for several bounded inputs, including a negative signed
checksum and a rejected input. Optional retained evidence pins source, object,
descriptor and executable hashes. Qualification establishes correctness and
linkage; it records no benchmark timings.
