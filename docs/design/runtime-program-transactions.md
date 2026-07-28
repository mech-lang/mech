# Runtime-owned program transactions

Every runtime-owned retained-program operation has one atomic internal
boundary. The runtime coordinates the following mechanisms rather than
teaching any one of them about the others:

- `MechProgramCheckpoint` owns structural program rollback and treats its
  value-state journal as opaque.
- `RuntimeTransaction` stages store records and runtime events.
- `RuntimeLiveStateSnapshot` captures live input bindings, persistent sends,
  the live context template, and registration mode.
- `RuntimeContextCheckpoint` captures operation context.
- the effect journal coordinates transactional, compensatable, and
  after-commit host and provider work;
- the capability overlay records provisional grants, revocations, and uses.

## Implicit retained operations

A retained source operation without an explicit transaction receives a normal
runtime transaction ID. A successful operation commits its program, live state,
staged store changes, success events, and a durable `TransactionRecord`.

A failed operation restores program, live, store-staging, and context state,
then aborts the hidden transaction. `TransactionStarted` and
`TransactionAborted` remain observable, but there is no durable transaction
record. The failure audit is emitted after restoration, so rolled-back success
events do not survive.

Runtime event sequence numbers and generated IDs are never rewound. Gaps after
rollback are expected.

## Explicit ownership and savepoints

Program ownership is lazy. Beginning an explicit store-only transaction does
not checkpoint or lock the retained program. Its first successful retained
operation captures the outer program and live baseline and claims the single
program-writer slot.

While transaction A owns that slot:

- retained operations from A are allowed;
- retained operations from transaction B are rejected;
- implicit retained operations are rejected;
- store-only work in B remains allowed.

Each retained operation inside A has its own savepoint. If a later operation
fails, its program, live, staged-store, event, and context changes are removed
while earlier successful operations remain provisional. Outer commit keeps the
provisional program. Outer abort restores the original program and live
baseline and discards all staged store state.

A store commit failure does not discard the program baseline. The transaction
remains active, retains ownership, and may be retried or aborted.

## Context identity and rollback

An active transaction is bound to the exact runtime, subject, task, actor,
actor message, and actor-state identity captured at begin. A same-subject
context with a different execution identity cannot drive that transaction.

Operation rollback restores module binding, capabilities, access, events,
identity fields, and budget maxima. It does not refund resource use:

- steps;
- bytes;
- items;
- messages.

For each counter, restored use is the greater of current and checkpoint use.
Elapsed time and generated transaction, event, object, task, actor, and message
IDs are also monotonic.

Only access added after the outer context baseline enters the durable
transaction read and write sets.

## Rollback failure and health

Ordinary parsing, preflight, execution, or store-commit failures do not poison
the runtime. Poisoning is reserved for an incomplete restoration or a broken
coordinator invariant.

A poisoned runtime preserves the original operation error and every rollback
failure for diagnostics. It rejects new retained execution, begin, and commit.
Read-only inspection, abort cleanup, and shutdown remain available. Round 3
does not provide an unpoison operation.

## Host output and external effects

Runtime-host plan nodes expose their outer output cell and reachable value
graph to the checkpoint journal. Rollback therefore restores the output cell's
identity, topology, and payload.

Host compilation calls a planning interface only. Runtime invocation uses an
explicit execution session and one of three distinct interfaces: pure,
runtime-managed, or staged. Staged hosts and resource providers return
prepared effects without performing the external mutation. The runtime owns
prepare, commit, abort, apply, compensate, and after-commit delivery.

Provider preparation takes `&self`. It must validate and construct inert
effect state only; external mutation belongs to the returned effect protocol.
Capability authorization for hosts and resources always goes through the
capability kernel and transaction overlay.

## Reactive and module boundaries

Whole-program checkpoint work does not run in the reactive inner loop. The
core, interpreter, and program layers provide compact program-local
reactive-turn rollback, and the runtime coordinates that compact journal with
the active `RuntimeExecutionTransaction`, effects, capability overlays,
provider work, and store commit. A pre-store failure restores program state
before discarding staged runtime work. A post-store error retains program
state.

Retained root-module construction and execution now share the same
runtime-owned program transaction. The existing recursive dependency builder
stages module and module-version records in the execution transaction, and the
program operation savepoint carries a module-journal mark. A pre-store root
failure therefore restores the prior retained program and exposes no partial
durable graph. After the durable store commit, graph and program state remain
committed even if an external transactional participant reports commit
indeterminacy.

One-shot bytecode execution and isolated dependency-module programs continue
to use temporary programs and are not retained-operation transactions.
Public callers cannot obtain the retained program, its journal, or mutable
runtime component handles.

## Value storage guardrails

`ValueStateJournal` remains the only layer that knows how current cells are
physically restored. Program checkpoints and the runtime coordinator only
compose opaque checkpoint APIs.

Checkpoint identity is process-local and never becomes transaction identity.
Pointer addresses and `ReactiveCellId` are not durable history keys. Durable
history requires an explicit stable logical cell ID first. This separation
allows future values to use storage other than `Rc<RefCell<T>>` without
teaching transaction coordination about the current representation.

## macOS CLI serve-test classification

The first serial CLI failure was
`serve::tests::poll_workspace_once_updates_registry_after_manual_refresh`.
It failed identically in three of three runs at both the Round 3 parent
`7a5b0895771de0d5aeda0885ae4c8ae68cb3390b` and corrective head
`da384fe619e769dba0ab720348b08ab1733638bd`.

The test expected `server.workspace_session` to be `Some` after loading
`main.mec`, but the actual value was `None`. A single existing Mech source is
classified as a configured served project, which does not create a workspace
session. The assertion failed before the later `source/main.mec` route lookup.

Every run used a canonical temporary root under
`/private/var/folders/.../T/mech-serve-refresh-*`; no `/var` versus
`/private/var` comparison was reached. The remaining eleven serial failures
were follow-on failures from the poisoned current-directory test lock. This is
a pre-existing CLI fixture failure, not a Round 3 runtime regression.
