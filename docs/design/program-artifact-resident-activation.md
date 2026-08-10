# Program artifact resident activation contract

This document is the normative D0 contract for converting a finalized
`ProgramArtifact` into resident execution state. D0 freezes the boundary and
the first admitted workload; it does not implement production activation or
route any program through the resident executor.

The boundary is:

```text
ProgramArtifact
  immutable semantic graph
        |
        v
activation validation and planning
        |
        v
ActivatedPlan
  immutable physical execution plan
        |
        v
ReactiveInstance
  instance identity
  StateArena
  reusable TurnWorkspace
  candidate epochs
  published epoch
```

## Sources of authority

Authority is ordered and non-overlapping:

- `ProgramArtifact` is the semantic authority.
- `ResolvedOperationContract` is the operation-access and interaction
  authority.
- `ActivatedPlan` is the physical execution-plan authority.
- `ReactiveInstance` is the runtime-instance authority.
- `StateArena` is the persistent versioned-storage authority.
- `TurnWorkspace` is the authority for candidate input, dirty scheduling,
  scratch, and bounded turn-local bookkeeping.
- The turn ledger is the retained transition-history authority.

The private Gate B `resident::artifact::ProgramArtifact` is not semantic
authority. It is an efficacy control scheduled for replacement in D1. D1 must
adapt the finalized public artifact instead of preserving two artifact models.

## Identity map

Identity has these distinct domains:

- `ProgramRevision` identifies immutable `ProgramArtifact` content.
- `CellSlotId` is a deterministic logical slot inside the artifact.
- `PlanGeneration` identifies one activated-plan generation.
- `LayoutGeneration` identifies one physical arena-layout generation.
- `ReactiveInstanceId` identifies one runtime activation and rejects stale
  handles.
- `SlotIndex` is a dense activated-plan index.
- `CellId` is the pair `ReactiveInstanceId + CellSlotId`.
- `InstanceEpoch` identifies one candidate or published instance-state
  version.

D1 begins with `PlanGeneration::ZERO`, `LayoutGeneration::ZERO`, and published
`InstanceEpoch::ZERO`. Its first candidate is `InstanceEpoch(1)`. The value
`u64::MAX` is a legal final epoch exactly once. Asking for its successor fails
with identity exhaustion; an epoch never wraps.

Every artifact slot receives exactly one dense `SlotIndex`. The D1 mapping is
identity-ordered:

```text
CellSlotId 0 -> SlotIndex 0
CellSlotId 1 -> SlotIndex 1
...
```

Activation fails before arena allocation if slot declarations are not dense,
a mapping is missing or duplicated, the slot count exceeds `u32`, a schema is
unknown, a producer is invalid, or an initializer is incompatible.

A physical pointer is never an identity. A pointer must not be encoded in a
`CellId`, `SlotIndex`, dependency topology, dirty-scheduling record, receipt,
or observer handle.

## Storage ownership

D1 assigns storage by semantic role:

| Role | Ownership and lifetime |
|---|---|
| Constant | Remains in `ConstantStore`; no `SlotIndex`-backed mutable payload, epoch, or rollback entry. |
| Input | Receives a resolved `SlotIndex`; its payload is installed in reusable `TurnWorkspace` storage and is valid only for the active candidate turn. It is not a published output in D1. |
| State | Lives in `StateArena`, uses two versioned typed buffers, and publishes with the instance epoch. |
| Derived non-output | Lives in reusable typed `TurnWorkspace` scratch and is not retained across turns. |
| Externally declared output | Resolves to a versioned state slot in D1. |

D1 rejects a derived output that requires retention after the turn. Constants,
inputs, persistent state, and derived scratch remain distinct storage classes.

## Observer policy

D1 supports synchronous observation only. An observer may read a published
output synchronously after acceptance, but no borrowed view may survive the
next `begin_candidate`. D1 has no epoch pin, RCU handle, `Arc` snapshot,
version pool, or retained slice. Benchmark correctness code may copy output
after the timed turn. D2 introduces retained-observer policies.

This lifetime rule makes two buffers sufficient for D1.

## Candidate semantics

One `ReactiveInstance` may have at most one active candidate. A candidate
contains:

```text
base_epoch
working_epoch
published buffer index
candidate buffer index
turn input
dirty-node state
touched slots
changed slots
bounded diagnostics
prepared recording ownership
```

Persistent-state reads observe the base published version. Reads of values
produced earlier in the same turn observe the current workspace or candidate
result. A first write marks a slot touched. A semantic change marks it changed.
Scheduler propagation uses changed state, not merely touched state.

Abort leaves `published_epoch` unchanged, invalidates candidate tags, and
discards candidate receipt and effect material. Acceptance executes and
validates the complete candidate, derives its summary, prepares the required
owned record from reserved capacity, publishes one epoch, and then appends the
already-prepared record infallibly.

## Publication

Publication is exactly one release store:

```rust
published_epoch.store(working_epoch, Ordering::Release)
```

A semantically equivalent single release store is permitted. Readers use
acquire ordering. D1 must report all of these structural facts:

```text
publication_store_count = 1
published_buffer_copy_bytes = 0
candidate_seed_bytes = 0
```

No full-write output is seeded from the published version.

## Receipt boundary

D1 continues to use the private Gate B fixed receipt for efficacy evidence:

```text
GateBFixedReceipt
  benchmark-only
  not the permanent TurnReceipt
  not exposed as runtime API
```

The sole normative D1 sequence is:

```text
1. reserve admission/ledger capacity before execution
2. begin candidate
3. execute candidate
4. validate candidate and integrity predicates
5. derive candidate summary
6. prepare the owned receipt/commit using the reserved permit
7. publish with one Release store
8. append the already-prepared record infallibly
```

The D0 boundary records these eight names in the exact `ordered_steps` order;
reordering, omitting, or inserting a step is a contract failure.

Receipt contents depend on the candidate hash, touched slots, changed slots,
and dirty-node count, so receipt preparation cannot precede candidate
execution or summary derivation. D0 does not introduce canonical receipt
types, event projection, effect delivery, or production ledger routing.

## D1 admitted profile

D1 admits exactly this profile:

```text
operation contract:       Declared
delivery:                 Signal
resident interaction:     Pure
input interaction:        Observation(CaptureAsInputFact)
dimension lifetime:       CompileTime
output construction:      FullWrite
alias policy:             NoAlias
change detection:         KernelReported or ExactScalar
observer policy:          Synchronous
```

The sole admitted observation is the EKF frame input adapter. It executes
outside the resident candidate graph and supplies one captured input fact.

Activation rejects the following before arena allocation:

```text
LegacyOpaque
Stream
Future
Effect
TransactionalExternal
Activation dimension
Turn dimension
ReadModifyWrite
Replace
Build
MayAlias
InPlaceRequired
SemanticHash
AlwaysChanged
unknown kernel
unknown operation
unsupported schema
derived retained output
non-output state with no initializer
```

After resident activation has been requested, failure does not fall back to
the legacy executor.

## Frozen D1 EKF profile

The ordinary source fixture at
`tests/architecture/resident-activation/ekf-source-v1.mec` is the exact D1
vertical slice. Its `ekf/*` operations will bind to the typed kernels already
proven by Gate B. They are not compiler-generated source construction and D0
does not register or elaborate them.

The semantic workload and the committed source bytes are authoritative. The
fixture uses the current parser's hanging-call form: no whitespace immediately
after `(` or immediately before `)`, with line breaks permitted after commas.
The exact bytes become frozen when D0 commit 3 is created. D0 does not broaden
function-call whitespace grammar.

Persistent candidate storage per EKF instance is exactly:

```text
state:       3 * f64 = 24 bytes
covariance:  9 * f64 = 72 bytes
total:                  96 bytes
```

The four-element input frame and every intermediate value are reusable
turn-workspace storage and do not count as persistent candidate bytes.

The workload has eighteen ordered operation nodes. Operations 0 through 14 are
resident kernels with one output and `KernelReported` change detection.
Operations 15 through 17 are ordinary pure predicate nodes with one Boolean
output, `Write`, `FullWrite`, `NoAlias`, and `ExactScalar`. The
`ekf/candidate-finite` predicate checks every corrected-state and symmetrized-
covariance element.

Three separate `IntegrityConstraintDeclaration` entries use
`integrity/assert`, read one predicate Boolean, and have zero outputs. Thus the
predicate node owns the Boolean `ResolvedOutputPort`; the integrity assertion
is the zero-output artifact layer. No `ProgramArtifact`, compiler-lowering, or
bytecode change is required. State and covariance each receive one complete
full-write update. `estimate` synchronously observes the published state before
the next candidate.

## Reconfiguration boundary

D1 supports no reconfiguration. Both plan and layout generation remain zero:

```text
PlanGeneration = 0
LayoutGeneration = 0
```

A request requiring an activation-dimension change, turn-varying physical
representation, kernel replacement, slot-layout change, or program-revision
replacement returns an unsupported-activation error. D2 implements
reconfiguration and generalizes storage and shapes.

## Private Gate B control and migration

The private Gate B executor remains an efficacy control. Its typed dual
buffers, candidate epoch, fixed receipt, and single release-store publication
prove that the resident target is effective. D1 replaces its private artifact
and hard-coded activation path with activation from the finalized public
`ProgramArtifact`; it does not duplicate those authorities.

The existing resident module must not gain dependencies on legacy value
storage, mutable-reference identity, reactive-cell identity, legacy journals,
runtime transaction coordinators, transaction-wide state cloning, or
`commit_runtime`. The D migration projection remains unimplemented in D0 and
is reproduced mechanically from the authoritative value-system inventory.

## Phase boundaries

- D0 freezes this contract and changes no production behavior.
- D1 implements the frozen ordinary EKF artifact-to-resident vertical slice.
- D2 generalizes storage, shapes, reconfiguration, and observer retention.
- D3 adds observations, effects, and transactional participants.
- D4 routes supported production programs.
- D5 closes legacy runtime storage.
- Final cutover deletes dead legacy types only after their obligations have
  moved.

There is no bytecode v2 before launch. Bytecode v1 evolves only when the static
`ProgramArtifact` format requires additional pre-launch fields. D0 itself
changes no bytecode.
