# Retained program checkpoint function-state audit

`MechFunctionImpl::transaction_state_values` is the checkpoint contract for
mutable state retained inside a plan node. The default is correct when all
semantic mutation is reachable from `reactive_output_values`. Implementations
with hidden cells must return those cells explicitly, and implementations whose
state cannot be represented by journaled `Value` cells must return
`TransactionStateUnsupported`.

The Round 2 audit covered every production `MechFunctionImpl` under `src/`,
`machines/`, and `hosts/`.

## Explicit retained-state discovery

- `UserFunction`: result cell, symbols, and nested plan state.
- Activation state: `Matcher`, `Finalize`, `UnmatchedFinalize`,
  `GuardFinalize`, and `Select`.
- Outer `Ref<Value>` outputs: `ConvertSEmpty`, `ConvertMatPassthrough`,
  `ValueMatrixComprehension`, `VariableDefineEmpty`,
  and `MatrixAccessScalarValueF`.
- Non-reactive activation payload state is retained by the plan collector's
  empty-output fallback through `ActivationEffectPayloadCapture::out`.

## Structured unsupported state

- `NChooseKMatrix<T>` retains `Ref<Matrix<T>>` and can replace the outer matrix
  enum. That outer topology is not a journalable `Value` cell, so checkpoint
  creation returns `TransactionStateUnsupported` before mutation.

## Round 3 runtime-host output state

`RuntimeHostNativeFunction` exposes its outer `Ref<Value>` and complete
reachable output graph to `ValueStateJournal`. The runtime-owned program
transaction coordinator now restores this output state when a retained
operation or explicit transaction rolls back.

Round 3 does not undo arbitrary external effects performed by the host call.
Host effect preparation, commit, suppression, and compensation belong to
Round 4.

## Implementations covered by the default

- Core unary, binary, and register families.
- Remaining interpreter conversion, access, assignment, definition,
  concatenation, table, set, string, and matrix functions.
- `ScopePulse`, `MatchGate`, and `Gate`; committed gate captures are reactive
  outputs.
- Dynamic-module functions and `ClosureNativeFunction`; their executable
  handles are immutable and their semantic mutation is output-backed.
- Remaining machine functions in stats, matrix, range, compare, logic, math,
  string, set, and scalar combinatorics.
- I/O print functions and `ActivationEffectBarrier`, which retain no mutable
  semantic state.

Provider crates under `hosts/` do not directly implement `MechFunctionImpl`.

Out-of-tree implementations are responsible for honoring this trait contract.
Hidden mutable state must never inherit the permissive output-backed default.

## Plan function-object lifetime constraint

A plan checkpoint retains the identity of every function object that existed
when the checkpoint was created. It does not clone or take secondary ownership
of `Box<dyn MechFunction>`. The retained identity is a process-local preflight
token only, not a durable cell or runtime transaction identity.

Supported structural changes include:

- appending new plan nodes;
- changing metadata on pre-existing nodes;
- changing value-backed state owned by pre-existing functions;
- adding activation registrations and scopes.

Restoration removes appended nodes, restores metadata and registrations, and
restores value-backed function state.

Removing or replacing a pre-existing function object invalidates the
checkpoint. Restore detects the missing or changed function identity during
preflight, returns a structured error, and performs no partial restoration.

Runtime transaction coordination must treat failure to restore a checkpoint as
a runtime-poisoning condition.

## Checkpoint architecture guardrails

- `ValueStateJournal` is the only layer that knows how current cell payloads
  are physically captured and restored. Structural checkpoints coordinate its
  preflight and apply API without reaching through a value handle.
- `MechProgramCheckpoint` keeps the journal behind the opaque
  `InterpreterCheckpoint`; program code never inspects journal entries or
  performs physical cell restoration.
- These checkpoints are explicit, process-local savepoints. They are not
  durable history, and runtime transaction identity must never be derived from
  pointer addresses.
- `ReactiveCellId` remains a process-local scheduler identity. Durable history
  requires an explicit stable logical cell ID before it is introduced.
- Journal capture and restoration occur only through explicit checkpoint APIs,
  never in the reactive solver inner loop.
- The current benchmark suite measures capture and restore separately across
  the required retained structures. A value-arena rewrite must be justified by
  additional representative, real Mech workload measurements rather than
  these structural microbenchmarks alone.
- `Ref<T>` hides its current `Rc<RefCell<T>>` backing and exposes handle,
  borrowing, and identity operations through its own API so future value
  storage changes remain localized.
