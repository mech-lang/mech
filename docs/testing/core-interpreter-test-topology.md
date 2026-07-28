# Core and interpreter test topology

Core and interpreter tests are organized by the lowest layer that owns the
invariant. Core owns value restoration and reactive-plan mechanics, the
interpreter owns interpreter-local checkpoints and bytecode registration, and
the runtime owns coordination across a reactive transaction.

For runtime integration, public API, compile-fail, and host-conformance
placement, see the companion
[runtime test topology](runtime-test-topology.md) guide.

## Ownership

### ValueStateJournal

`ValueStateJournal` tests live under
`src/core/src/state_journal/tests/`, grouped by the behavior being restored:

- `scalar.rs` owns scalar capture, identity, and type mismatch behavior.
- `nested.rs` owns aliases, cycles, shared cells, and recursive containers.
- `collections.rs` owns map, set, table, matrix, and vector restoration,
  including rebuilt lookup metadata.
- `collisions.rs` owns hash-collision equality, atomic failure, and retry.
- `delta.rs` owns sealing, delta construction, rewind, replay, and lifecycle
  errors.
- `borrow_conflicts.rs` owns capture and restore preflight failures and proves
  that failed preflight does not partially mutate state.

Small fixtures repeated across these files belong in
`src/core/src/state_journal/tests/support.rs`. Journal internals remain private;
the descendant test modules use their existing implementation-private access.

### Reactive plan

Reactive-plan tests live under `src/core/src/functions/tests/` because the core
functions layer owns plan structure, dependency indexes, scheduling, and
staged register commits:

- `reactive_plan.rs` owns node registration and plan shape.
- `dependencies.rs` owns reactive-versus-sampled dependency classification and
  scope.
- `register_commit.rs` owns staged register writes and commit atomicity.
- `checkpoint.rs` owns structural rollback and index rebuilding.
- `solve.rs` owns dirty-cell execution, propagation, and turn scheduling.
- `transaction_state.rs` owns explicit function transaction participation.
- `registry.rs` owns function tables and descriptors.

Only fixtures genuinely reused by multiple plan suites belong in
`src/core/src/functions/tests/support.rs`.

### Interpreter checkpoints

Interpreter-local checkpoint tests live under
`src/interpreter/src/interpreter/tests/`:

- `checkpoint.rs` owns whole-interpreter capture and restore, including private
  state, symbol identity, registers, constants, plan state, recursive child
  interpreters, child-handle identity, and retained compiler-context rejection.
- `reactive_turn.rs` owns compact reactive-turn checkpoints, pending registers,
  trace truncation, plan identity, owner validation, interpreter-ID validation,
  and interpreter-local rollback.
- `execution_services.rs` owns interpreter execution-service borrowing and
  structured borrow conflicts.
- `bytecode.rs` owns decoded bytecode registration and execution invariants.

Interpreter test construction remains local to its owning suite. Add an
interpreter test support module only after the same helper is genuinely reused
by more than one sibling suite.

### Expression evaluation

Expression unit tests live under `src/interpreter/src/expressions/tests/`,
grouped by the expression behavior whose private wiring they inspect:

- `registration.rs` owns initialized function registration, dependency edges,
  alias deduplication, and batch order.
- `comprehensions.rs` owns comprehension output transaction-state roots.
- `structural_access.rs` owns record and tuple alias nodes, member-cell
  dependencies, and source/bytecode parity.
- `variables.rs` owns variable lookup and kind-cast dependency registration.

`tests/mod.rs` carries the original feature gates for each group. Add
`subscripts.rs`, `string_access.rs`, `formulas.rs`, or `matches.rs` only when
the production root contains tests for that behavior; empty category modules
do not document ownership. Keep a helper local to its owning file until at
least two sibling suites genuinely share it, then place only that shared
fixture in `support.rs`.

For the production modules exercised by these suites, see the
[expression evaluation topology](../interpreter/expression-topology.md).

### Statement evaluation

Statement unit tests live under `src/interpreter/src/statements/tests/`,
grouped by the statement behavior whose graph construction or scheduling they
prove:

- `scheduling.rs` owns reachable combinational scheduling and register
  boundaries.
- `activation_scope.rs` owns activation-block lowering, sampled-versus-reactive
  dependencies, plan registration, trigger-write rejection, and plan
  stability. Runtime activation arm selection and dispatch belong to the
  activation subsystem tests instead.
- `variable_define.rs` owns variable-definition registration dependencies.
- `variable_assign.rs` owns whole assignment graph shape, decoded parity,
  matrix root cells, and plain-assignment register commits.
- `op_assign.rs` owns operator-assignment graph shape, decoded parity, staged
  register commits, and multi-turn propagation.
- `support.rs` contains only the assignment graph and interpreter fixtures
  shared by `variable_assign.rs` and `op_assign.rs`.

`tests/mod.rs` preserves the original feature boundaries. Files for
destructuring, integrity declarations, kinds, enums, state machines, or
decoding should be added only when statement-owned tests for those behaviors
exist; do not create empty ownership placeholders.

### Reactive transaction coordination

Coordination across runtime state, interpreter execution, capabilities,
effects, stores, and transaction finalization lives under
`src/runtime/src/runtime/reactive_transaction/tests/`:

- `coordination.rs` owns implicit-versus-explicit transaction coordination,
  checkpoint reuse, ownership exclusion, and scoped transaction services.
- `rollback.rs` owns coordinated rollback and runtime health after turn
  failures.
- `finalization.rs` owns capability, store, participant, and after-commit
  outcomes.
- `service_borrow.rs` owns runtime-service reentrant-borrow recovery.

Keep interpreter-local rollback in `interpreter/tests/reactive_turn.rs` and
plan-only rollback in `core/functions/tests/checkpoint.rs`. Move a scenario to
the runtime coordinator only when its invariant crosses one of those
boundaries.

## Placement rules

- Put a test with the lowest subsystem that can prove its complete invariant.
- Group extracted tests by behavior; do not create generic regression,
  miscellaneous, or development-round buckets.
- Keep a short test for one private helper inline only when it needs no shared
  fixture and does not assemble a journal, reactive plan, or interpreter.
- Keep scenario-specific setup beside the scenario. Use a local `support.rs`
  only for small fixtures repeated across sibling test files.
- Do not create a shared framework spanning core, interpreter, and runtime.
- Preserve existing feature gates at the narrowest owning module. Do not add a
  broad gate or platform exclusion to make a test disappear.
- Preserve externally filtered test function names. If an external consumer
  also depends on a fully qualified harness path, retain that path with a thin
  wrapper.
- Keep production changes limited to test-module wiring and narrow test-only
  access. Do not expose private checkpoint, plan, or journal state publicly.
- Use explicit imports and behavior-oriented names. Avoid wildcard parent
  imports and development-round terminology.

## Critical suites

| Contract | Location and anchor |
| --- | --- |
| Journal restore preflight is atomic | `src/core/src/state_journal/tests/borrow_conflicts.rs` — `state_journal_restore_preflight_is_atomic` and `state_journal_split_restore_preflights_before_apply`. |
| Collection collisions use payload equality and remain retryable | `src/core/src/state_journal/tests/collisions.rs` — the set, map, and collection-collision cases. |
| Collection lookup metadata survives rewind and replay | `src/core/src/state_journal/tests/collections.rs` — the map, set, and nested hashed-collection cases. |
| Plan rollback restores structure and indexes | `src/core/src/functions/tests/checkpoint.rs` — `plan_rollback_restores_full_structure_and_rebuilds_consumers`. |
| Register commits stage atomically | `src/core/src/functions/tests/register_commit.rs` — `reactive_register_commit_stages_all_before_any_commit` and the stage-error cases. |
| Dirty scheduling preserves plan order and transaction boundaries | `src/core/src/functions/tests/solve.rs` — the `reactive_dirty_scheduler_*` and `reactive_turn_*` suites. |
| Whole-interpreter checkpoints restore private recursive state | `src/interpreter/src/interpreter/tests/checkpoint.rs` — `interpreter_checkpoint_restores_private_state_and_recursive_child_identity`. |
| Reactive-turn checkpoints remain compact and reject mismatches before mutation | `src/interpreter/src/interpreter/tests/reactive_turn.rs` — the checkpoint-shape, owner, interpreter-ID, plan-handle, and pending-node cases. |
| Bytecode registration preserves dependency semantics | `src/interpreter/src/interpreter/tests/bytecode.rs` — the nullary, unary, binary, variadic, and alias-deduplication cases. |
| Runtime coordinates checkpoint ownership without redundant full checkpoints | `src/runtime/src/runtime/reactive_transaction/tests/coordination.rs` — the implicit/explicit checkpoint and owner-exclusion cases. |
| Runtime reentrant service borrowing is recoverable | `src/runtime/src/runtime/reactive_transaction/tests/service_borrow.rs` — `reentrant_runtime_service_borrow_returns_structured_error_and_recovers`. |
