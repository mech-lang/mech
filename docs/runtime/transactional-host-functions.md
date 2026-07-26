# Transactional host functions

Every host function declares how it participates in runtime transactions through
`HostFunctionTransactionMode`. The default is intentionally conservative:
unclassified callbacks are `ImmediateOnly`.

## Modes

### `Pure`

Pure functions compute a value and may read immutable process state. They do not
mutate runtime state or anything outside the runtime.

Use `ClosureHostFunction::new_pure` for closure-backed pure functions. The
runtime may call a pure function while constructing the native execution plan
and again while executing it, so the callback must be safe to evaluate more
than once.

### `RuntimeManaged`

Runtime-managed functions may mutate only through `RuntimeServices` operations
that already stage their work in the active runtime transaction. They must not
write files, print output, send network traffic, mutate shared application
state, or retain a runtime pointer.

This is an unsafe-style contract: the runtime cannot prove that an implementation
uses only mediated services. During native-plan construction, the runtime runs
the callback against a private preview savepoint and removes its staged store,
context, and effect changes afterward. Monotonic IDs and consumed budget are not
rewound.

Use `ClosureHostFunction::new_runtime_managed` when a closure satisfies this
contract. The built-in actor state functions use this mode.

### `Staged`

Staged functions return a provisional value and one `PreparedRuntimeEffect`
through `RuntimePreparedHostCall`. The runtime makes the value available to the
Mech program immediately and owns the effect lifecycle.

Use `StagedClosureHostFunction::new`. Its callback must only construct the value
and prepared effect; it must not perform the external mutation itself. Native
plan construction may invoke the callback to preview the value, but the preview
effect is discarded. The effect is staged only when the program call executes.

Choose the effect protocol honestly:

- `Transactional` for a real prepare/commit/abort participant.
- `Compensatable` only when compensation reliably restores the immediately
  preceding state.
- `AfterCommit` for stdout, notifications, DOM writes, physical commands, and
  other irreversible delivery.

The provisional return value must not depend on successful after-commit
delivery.

### `ImmediateOnly`

`ClosureHostFunction::new` creates an immediate-only callback. It is allowed
through explicitly nontransactional host calls and ordinary reactive execution.
The runtime rejects it before invocation whenever a transaction is active,
including implicit retained-program transactions.

Use this mode for commands whose result cannot be known until an irreversible
operation has already happened and which cannot implement a real transactional
preparation protocol.

## Failure behavior

- Operation rollback removes staged host effects created after its savepoint.
- Outer abort discards all staged host effects.
- Failed retained execution does not invoke immediate-only callbacks.
- After-commit delivery failure does not roll back an internally committed
  transaction and does not poison the runtime.
- Incomplete effect abort or compensation poisons the runtime.
- A transactional participant commit failure after the runtime store commits is
  reported as an indeterminate external commit and poisons the runtime.

Reactive turns remain outside retained-program checkpoints and journals. A
staged function invoked by an ordinary reactive turn uses the one-effect
immediate coordinator.
