# Program artifact resident activation contract

A finalized `ProgramArtifact` is converted into a physical execution plan and
independent resident instances. The shipping runtime executes admitted
artifacts through this boundary. Unsupported activation returns a structured
error.

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
- `ResolvedOperationContract` is the operation-access and interaction authority.
- `ActivatedPlan` is the physical execution-plan authority.
- `ReactiveInstance` is the runtime-instance authority.
- `StateArena` is the persistent versioned-storage authority.
- `TurnWorkspace` owns candidate input, dirty scheduling, scratch, and bounded
  turn-local bookkeeping.
- The turn ledger owns retained transition history.

The finalized public artifact is the authority used by resident activation.
Benchmark control fixtures do not supply production semantics. Memory planning
and realization follow [Memory planner](memory-planner.md) and
[Managed memory runtime](memory-runtime.md).

## Identity and activation

Identity has distinct domains:

- `ProgramRevision` identifies immutable artifact content.
- `CellSlotId` is a deterministic logical slot inside the artifact.
- `PlanGeneration` identifies one activated-plan generation.
- `LayoutGeneration` identifies one physical arena-layout generation.
- `ReactiveInstanceId` identifies one runtime activation and rejects stale handles.
- `SlotIndex` is a dense activated-plan index.
- `CellId` combines instance identity with a logical slot.
- `InstanceEpoch` identifies one candidate or published instance-state version.

Initial plan, layout, and published-epoch generations are zero. The first
candidate epoch is one. Epochs and generations use checked advancement; they
never wrap. `u64::MAX` is a legal final epoch, and requesting its successor
fails with identity exhaustion.

Every artifact slot receives exactly one dense `SlotIndex`. Activation
validates slot density and uniqueness, representable counts, known schemas,
producer ownership, initializers, operation contracts, and physical layout
before execution. A physical pointer never supplies logical identity,
dependency topology, scheduling identity, or receipt identity.

Reconfiguration validates the replacement plan and layout before publication.
Generation identity distinguishes the replacement from stale handles. Shape,
operation, or layout requests outside the admitted profile fail explicitly.

## Storage ownership

Constants remain immutable and have no mutable epoch or rollback entry.
Captured inputs live in reusable candidate input storage. Persistent state and
published output slots use versioned storage. Derived computations use scratch
unless their semantic producer requires input or activation-constant storage.
These classes remain distinct even when their concrete scalar or matrix
representation is the same.

An output may be materialized from a constant, an input, or a computed value;
publication does not require the source program to disguise an output as state.
Mutable payload is owned by the instance. Independent instances do not share
mutable value cells.

Borrowed synchronous output views cannot outlive their permitted version.
Retained observation requires explicit owned snapshots or version ownership;
no pointer may silently keep a reusable candidate buffer alive. Observer
retention and bounded-resource failures are validated by the runtime's
publication and history contracts.

## Candidate semantics

One instance has at most one active candidate. It owns base and working epochs,
published and candidate buffer identities, captured input, dirty-node state,
touched and changed slots, bounded diagnostics, and prepared recording
ownership.

Persistent-state reads observe the base published version. Reads of values
produced earlier in the same turn observe the current workspace or candidate
result. A first write marks a slot touched. A semantic change marks it changed.
Scheduler propagation uses changed state, not merely touched state.

Abort preserves the published epoch, invalidates candidate tags, and discards
candidate receipt and effect material. Acceptance executes and validates the
complete candidate before publication. Expected failures, integrity rejection,
and capacity exhaustion leave published state unchanged.

## Publication and recording

The normative sequence is:

1. Reserve admission and recording capacity.
2. Begin and execute the candidate.
3. Validate the candidate and integrity predicates.
4. Derive the candidate summary.
5. Prepare the owned receipt or commit using the reserved capacity.
6. Publish with one release store.
7. Append the already-prepared record infallibly.

Readers use acquire ordering. Receipt preparation failure aborts before
publication. Complete full-write outputs require zero candidate seed bytes
and zero published-buffer copy bytes. Read-modify-write operations retain
their explicit preservation and initialization requirements.

The artifact-derived engine path returns a typed `ResidentTurnSummary` with
instance and program identities, before and after epochs, candidate hash, and
bounded touched, changed, and dirty counts. Benchmark receipt formats are
separate from canonical runtime recording. Observations, deferred effects,
replay, and transactional participants obey their declared operation contracts
and the runtime's admission and delivery rules.

## EKF fixture

`tests/architecture/resident-activation/ekf-source-v1.mec` is the ordinary-source
EKF fixture. Its `ekf/*` operations bind to typed kernels after source
compilation; the fixture is not a hand-built artifact. Its semantic workload
and committed source bytes remain authoritative for the behavioral and
allocation tests.

The instance's persistent candidate state is three `f64` values (24 bytes) plus
nine covariance `f64` values (72 bytes), totaling 96 bytes. The four-element
input frame and intermediate values use reusable turn workspace.

The workload has eighteen ordered operation nodes: fifteen resident kernels
with `KernelReported` change detection, followed by three pure Boolean
predicates with `FullWrite`, `NoAlias`, and `ExactScalar`. Separate
`IntegrityConstraintDeclaration` entries read those predicates and have no
outputs. State and covariance receive complete full-write updates. The
synchronous estimate observes accepted published state before the next
candidate.
