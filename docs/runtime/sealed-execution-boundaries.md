# Sealed runtime execution boundaries

The runtime is the only safe-code owner of live transactional state. Public
methods expose detached values and immutable summaries, while mutation flows
through context-aware runtime operations.

## Detached snapshots

`RuntimeValueSnapshot` deep-snapshots the reachable value graph. Runtime
results, module exports, host arguments, and host results cross this boundary
without sharing the runtime's mutable cells. Retaining or mutating a snapshot
cannot change an installed program. Internal cell identities remain stable
across successful turns and rollback.

Snapshots are observations, not transaction identities or durable history.
Pointer addresses and process-local reactive cell IDs are never persisted as
identity. Durable history requires a separate stable logical cell ID.

## Explicit execution sessions

Runtime-owned program execution receives an explicit, lexical execution
session. The session lends only the services required by a callback and keeps
the retained program installed throughout execution. There is no raw runtime
pointer, thread-local execution target, or take-and-replace program bridge.
Standalone programs use `NoMechExecutionServices`.

RAII guards own the active program-operation and effect-phase markers. Normal
returns, errors, and unwinding all release those markers.

## Host planning and invocation

Compilation calls `HostFunctionPlanner::plan` exactly once for a planned host
node. Planning may inspect detached arguments and metadata, but cannot invoke a
host callback, consume authority, allocate runtime IDs, stage events, or stage
store state.

Execution dispatches a planned host through one explicit interface:

- a pure host computes from detached snapshots;
- a runtime-managed host uses the explicit execution services;
- a staged host prepares a provisional value and inert runtime effect.

There is no unclassified or immediate-only Mech-callable host path.

## Authority

The capability kernel is the single authorization system. Configuration grants
are materialized as ordinary capabilities in both the store and kernel.
Resource reads, resource writes, persistent sends, host calls, and module
requirements all use the same scoped check.

`RuntimeContext` owns an immutable runtime identity, subject, and current
transaction from the perspective of external safe code. Its authority scope is
either all matching capabilities for the subject or an explicit allowlist.
Transaction-local grants, revocations, and pending uses extend or restrict that
scope provisionally and rollback restores the prior scope.

## Function and register participation

Every production `MechFunctionImpl` explicitly declares its transaction-state
strategy. Stateful functions checkpoint and restore hidden state; stateless
functions say so; unsupported state rejects transactional execution before
mutation.

Register commits are sealed. The program validates and captures the complete
register batch before staging, then commits through a private
`ReactiveRegisterCommit` implementation. Safe callers cannot implement the
commit participant or obtain a reactive journal.

## Providers and effects

Provider write preparation takes `&self` and must not mutate external state.
It returns a prepared transactional, compensatable, or after-commit effect.
The runtime owns the full lifecycle and attempts independent cleanup or
delivery callbacks even when another callback fails.

Transactional preparation and compensatable apply happen before store commit.
Their failures are ordinary operation failures when cleanup succeeds.
Transactional commit happens after store commit, so failure is indeterminate
and poisons the runtime. After-commit delivery failures are reported without
rolling back durable state or poisoning an otherwise healthy runtime.
Cleanup callback failure poisons only after all cleanup callbacks have been
attempted.

## Panics and store semantics

Every extension trait boundary converts unwinding into a structured
`RuntimeExtensionPanicked` error. Pre-store panics use the same rollback policy
as returned errors. Post-store participant panics retain committed program and
store state and poison the runtime when the external commit is indeterminate.

A panic from `MechStore::commit_runtime` is different: durability is unknown.
It becomes `RuntimeStoreCommitIndeterminate`; the runtime clears transaction
ownership, retains the program, poisons itself, and never attempts rollback,
compensation, abort, or retry.

`commit_runtime` promises an atomic all-or-nothing batch when it returns. This
is an atomicity contract, not isolation from concurrent external writers.

## Public and unsafe boundaries

Safe public code cannot obtain mutable program, store, capability-kernel,
resolver, host-registry, host-policy, scheduler, or actor-driver handles. It
cannot edit active context identity, transaction, budget maxima, event storage,
or access sets. Structural components are selected through the builder before
runtime creation.

Journal integration is safe Rust. The lower-level interpreter entry points are
used only by the program crate, while runtime code coordinates turns through
sealed operations and `ProgramTurnFinalization`. No compatibility bypass,
rewind facility, or history mechanism is part of this boundary.
