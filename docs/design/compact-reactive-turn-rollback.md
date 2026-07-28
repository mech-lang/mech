# Compact reactive-turn rollback

## Program-local atomicity boundary

Ordinary `MechProgram` input turns, single-interpreter reactive turns, and
selected or whole-plan steps are program-locally atomic. If execution fails,
the program restores input cells, executed function state, committed register
state, and the scheduler state of every affected interpreter. Original
`Ref<T>` and `ValRef` handles remain in place.

At the program layer this boundary covers in-memory execution only. When the
runtime owns the turn, it coordinates the compact rollback state with its
transaction envelope, effects, providers, capability overlay, and store
commit without moving whole-program checkpoint work into the reactive loop.

## ReactiveTurnJournal

`ReactiveTurnJournal` wraps `ValueStateJournal` and exposes only ephemeral
before-state capture and restore operations. `ValueStateJournal` remains the
only layer that knows how cell payloads are physically restored. The compact
journal neither records after-state nor creates a `CommittedValueStateDelta`.

Capture is dynamic. A combinational function is captured immediately before
the scheduler executes that selected node. Unscheduled nodes, sampled-only
consumers, and register nodes encountered during combinational propagation are
not captured.

Register batches use a stronger ordering:

```text
validate node IDs, kinds, order, and output ownership
capture every ordered register function
stage every register
validate every staged output list
commit every register
```

Capturing the complete register batch before the first staging call keeps a
late capture or staging failure restorable.

## Compact interpreter scheduler checkpoint

`InterpreterReactiveTurnCheckpoint` retains a process-local owner token,
interpreter ID, shared plan handles, plan length, activation-registration
depth, pending-register state, and—when enabled—the trace target and saved
trace length. It contains no program state, symbols, functions, values, full
interpreter checkpoint, or full plan checkpoint.

Restore preflight checks ownership, interpreter identity, plan-handle
identity, compact structural guards, saved pending-register validity, and
trace rollback availability. Apply restores only pending-register state and
truncates the trace suffix. Value restoration is performed first by the
shared reactive journal.

`MechProgram::step` preserves its existing plan-step selector behavior. Step ID
zero executes the whole plan once. A positive step ID executes that single
one-based plan function once. Both forms use compact rollback.

The lower-level interpreter stepping API uses the same dynamic journal and
retains its separate repetition-count argument. Every function is captured
immediately before `solve_result`; repeated executions deduplicate cell
identity so rollback returns to the state before the first repetition.

## Multi-interpreter batching

The crate-private program turn journal contains one shared
`ReactiveTurnJournal` and one
compact scheduler checkpoint per affected interpreter. Input preparation
finishes first. Every input target and every affected interpreter checkpoint
is captured before the first assignment is staged or committed. Interpreters
then execute in ascending actual interpreter-ID order.

Rollback is two-phase:

```text
preflight every affected interpreter in ascending ID order
preflight all shared value targets
restore shared values
restore interpreter scheduler state in ascending ID order
```

No value or scheduler is changed until all rollback participants pass
preflight. A failure in a later interpreter therefore restores earlier
interpreter turns and prevents subsequent interpreters from executing.

Each program journal represents exactly one operation. Reuse is rejected
before a second operation can mutate state. Safe callers use coordinated
program entry points; only the program crate may invoke the unsafe
interpreter journal integration boundary.

## Performance model

Work is proportional to the input targets, functions actually selected for
execution, pending registers in the affected batch, reachable cells owned by
those functions, and affected interpreters. It is not proportional to the
complete program graph for a sparse turn. The benchmark suite compares
reactive chains, independent graphs, register batches, multi-interpreter
turns, failure rollback, explicit journal rollback, and a full
`MechProgram::checkpoint` on the same 1,000-node fixture.

This evidence is the basis for future storage decisions. It does not justify
an arena rewrite by itself.

## Lifetime and exclusions

The program reactive-turn journal is ephemeral rollback state.

It is not a committed delta.

It is not serializable history.

It is dropped on success unless the runtime coordinator retains it through
store finalization. The runtime receives only a sealed commit participant, not
the journal itself.

This design introduces no rewind, replay, durable history, or stable
historical cell IDs.
