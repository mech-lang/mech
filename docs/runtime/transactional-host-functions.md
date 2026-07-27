# Transactional host functions

Mech-callable hosts separate compilation-time planning from runtime
invocation. Registration supplies a planner; the planner returns exactly one
planned invocation class.

## Planning

`HostFunctionPlanner::plan` receives detached argument snapshots. It may
validate types, choose an implementation, and construct immutable planned
state. It must not call the runtime host operation, consume a capability use,
allocate a runtime ID, emit an event, stage store state, or prepare an effect.

Compilation calls planning once for each host node it builds. Only execution
invokes the returned plan.

## Invocation classes

### Pure

A `PlannedPureHostFunction` computes only from detached snapshots and immutable
process state. It cannot access runtime services. Its result is deep-snapshotted
before it enters the program.

### Runtime-managed

A `PlannedRuntimeManagedHostFunction` receives the explicit execution session.
It may mutate only through runtime services that participate in the current
transaction. It must not retain the session or a runtime pointer and must not
perform an uncoordinated external effect.

### Staged

A `PlannedStagedHostFunction` returns a provisional detached result and a
`PreparedRuntimeEffect`. Preparation constructs inert state only; it performs
no external mutation. The runtime stages the effect and owns its lifecycle.

Choose the protocol according to the external system:

- `Transactional` for a real prepare/commit/abort participant.
- `Compensatable` only when compensation reliably restores the immediately
  preceding state.
- `AfterCommit` for output, notifications, DOM writes, physical commands, and
  other irreversible delivery.

The provisional result cannot depend on successful after-commit delivery.
There is no immediate or unclassified Mech-callable host class.

## Authority and snapshots

Host authorization uses the same capability kernel and context authority scope
as resources, persistent sends, and module requirements. Planning performs no
authorization. Invocation checks authority through the active execution
session and transaction overlay.

Arguments and results are detached `RuntimeValueSnapshot` values. A host may
retain a snapshot, but it cannot use that snapshot to mutate a program cell.

## Failure and panic behavior

- A planning or pre-store invocation panic is converted to
  `RuntimeExtensionPanicked` and follows ordinary operation rollback.
- Operation rollback removes effects staged after its savepoint.
- Outer abort discards all staged effects.
- Transactional preparation and compensatable apply failures roll back when
  cleanup succeeds.
- After store commit, a transactional participant failure is external commit
  indeterminacy: committed program and store state remain and the runtime is
  poisoned.
- After-commit delivery failure is reported without rolling back durable state
  or poisoning an otherwise healthy runtime.
- Abort or compensation failure poisons only after all cleanup callbacks are
  attempted.

Runtime-owned reactive turns use the same coordinated effect journal and store
boundary while retaining compact program-local rollback.
