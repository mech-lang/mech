# Transactional integrity constraints

Mech’s named constraint syntax (`name! := expression`) defines a live
integrity constraint. Constraints inspect the settled candidate state after
reactive propagation and register writes, but before that state is published.

Only scalar Boolean `true` passes. Boolean `false`, non-Boolean results, and
an unreadable settled result reject the operation. The program evaluates every
retained constraint and reports all failures in deterministic interpreter and
constraint order.

An invalid operation is restored through the same compact program journal and
runtime transaction savepoint used for other execution failures. External
effects are still provisional at validation time, so controller commands,
console or browser output, provider writes, and after-commit delivery cannot
observe invalid state.

Explicit transactions retain earlier valid provisional operations when a
later operation violates a constraint. The invalid suffix is removed, the
transaction remains open, and the caller may repair the candidate or abort.
The runtime revalidates the retained program immediately before a final
explicit commit; a failed final check leaves the transaction active and does
not claim that rollback occurred.

After a successful operation rollback, the runtime emits
`:program/integrity-constraint/violated` through the immediate audit path.
Its payload is detached: it contains stable names, expressions, kinds, and
attempted values, never live cells or addresses. General `ProgramFailed` and
module-failure events remain available as well.

Integrity enforcement has no warning mode, ignore flag, or debug bypass.
Bytecode constraint metadata is deliberately unsupported in this change, so
`mech test` requires source input for named constraints.

## Minimal source example

See [`examples/transactional-integrity`](../../examples/transactional-integrity/)
for a declaration-only program with an initial target of 90, a maximum of 120,
and one named integrity constraint. Running the source evaluates that valid
initial candidate once. It does not inject host inputs or deliver receiver
commands.

## Automated end-to-end transaction proof

The runtime acceptance test exercises the complete `100 → 150 → 110` flow:

- `100` stages, prepares, commits, and delivers a receiver effect;
- `150` reaches receiver staging, then fails the Mech integrity constraint and
  is aborted before effect preparation, durable store commit, delivery, or
  committed capability accounting;
- `110` commits and delivers afterward, proving the runtime remains healthy.

Run the focused proof with:

```sh
cargo test -p mech-runtime \
  integrity_invalid_host_input_aborts_staged_receiver_before_commit \
  -- --nocapture
```
