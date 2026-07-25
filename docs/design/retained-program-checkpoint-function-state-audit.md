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
- `RuntimeHostNativeFunction` owns an outer output cell in the runtime layer.
  Retaining that topology requires the runtime transaction coordinator, so
  Round 2 rejects it before inspection or mutation.

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

No host crate directly implements `MechFunctionImpl`; runtime host calls are
represented by the structured unsupported boundary above.

Out-of-tree implementations are responsible for honoring this trait contract.
Hidden mutable state must never inherit the permissive output-backed default.

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
