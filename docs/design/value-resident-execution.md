# Value-resident execution architecture

This document defines the execution architecture that Gate A measures and that
later gates will implement. Gate A itself does not route production work
through this architecture. Its purpose is to freeze the semantic target, bound
the legacy implementation, and make history-dependent costs reproducible.

## Program, plan, and instance decomposition

`ProgramArtifact` is the immutable result of parsing, validation, linking, and
compilation. It owns bytecode or equivalent executable instructions, immutable
constants, schema declarations, dependency metadata, and external-operation
requirements. It has no live cells, host handles, transaction state, or
observer state. Equal artifact identities denote equal executable content.

`ActivatedPlan` is an immutable, runtime-specific activation of a
`ProgramArtifact`. It resolves trusted functions, layouts, and external
requirements against one runtime configuration. It may cache immutable
execution metadata, but it owns no candidate or published values. An activated
plan can be shared by independent instances.

`ReactiveInstance` owns one live activation of an `ActivatedPlan`. It contains
the instance's published slot snapshot, candidate workspace, input sequencing,
turn sequencing, observers, and the bounded recording and effect queues. No
mutable value cell is shared between instances. A turn executes against one
instance unless an explicit cross-instance protocol is selected.

## Immutable snapshot values

Published state is represented by immutable, owned snapshot values. A snapshot
contains data semantics, not `Ref`, `ValRef`, borrow handles, allocator
addresses, or runtime storage identity. Composite snapshots own immutable
children or share them through immutable structural sharing. Once a snapshot
is published, execution cannot mutate it in place.

The current public `Value` model remains in Gate A. Because it contains mutable
references and storage identity, it is not the permanent receipt, ledger, or
snapshot representation. Conversion boundaries and a replacement snapshot
model belong to a later gate.

## Slot and instance identity

An activated plan assigns stable logical slot identities from program
structure. A reactive instance has a stable instance identity independent of
its address, allocation, or restart-local pointers. The pair `(instance,
slot)` identifies a logical state location. Identity never derives from
`as_ptr`, `ValRef`, a `ReactiveCellId` backed by mutable storage, or a collection
bucket.

Gate A does not introduce the final slot, cell, shape, or instance-epoch types.
It records pointer-derived and `ReactiveCellId` identity as legacy deletion
targets so their use cannot expand.

## Candidate epoch visibility

A turn obtains a private candidate epoch from the instance's current published
snapshot. Input installation, reactive execution, integrity validation, and
receipt/effect preparation read the published epoch and write only the
candidate. Observers and concurrent readers cannot see candidate state.

A rejected candidate is discarded without changing the published snapshot. A
candidate is publishable only after all operations that may report an expected
failure have completed and all bounded post-publication resources have been
reserved. Epoch allocation is monotonic and checked; it never wraps or
silently saturates. Gate A does not add an instance epoch to production.

Candidate storage has an explicit visibility class:

```rust
enum VisibilityClass {
    Versioned,
    ExclusiveInPlace,
}
```

`Versioned` writes use unpublished storage and are compatible with concurrent
epoch readers. `ExclusiveInPlace` writes may touch committed storage only when
every observer is excluded until commit or undo completes. A plan must select
the visibility class before execution; a kernel cannot silently change it.

The execution vocabulary distinguishes three facts:

- **touched**: candidate storage was written;
- **invalidated**: downstream execution is required;
- **changed**: semantic inequality is established.

The scheduler operates on invalidation. It must not require an O(payload)
comparison of every matrix output merely to decide whether downstream work is
eligible.

## Publication

Publication is one atomic instance-local transition from the prior immutable
snapshot to the fully validated candidate snapshot. Before publication, the
runtime must have satisfied the selected durability policy's pre-publication
requirements, bound the owned turn record to its required capacity, and bound
owned effect intents to reserved outbox capacity. Publication itself has no
provider callback, allocation, capacity discovery, validation, or expected
failure branch.

After publication, infallible local append operations transfer the prepared
turn record and effects to their bounded queues. External delivery may fail and
retry according to policy, but such failure does not reverse published state.
If a required durable commit is indeterminate, the instance does not publish a
new candidate until durable resolution establishes the outcome.

Capacity is obtained in two stages. An `AdmissionPermit` bounds work admitted
to the turn before execution. A `FinalizationPermit` binds the actual owned
receipt, ledger, durable queue, and outbox capacity required before publication.
No finalization resource may first be discovered after publication.

## Observer retention

Observers receive immutable published snapshots or immutable deltas with
explicit retention ownership. A reader can retain an older snapshot without
blocking a new publication or causing the executor to copy all retained
history. Retention policy is bounded and explicit. Dropping the final observer
releases the retained snapshot; no observer may retain a candidate workspace or
borrowed turn-local storage.

An observer uses one declared policy: synchronous observation, copied host
output, a bounded version pool, epoch/generation pinning, or backpressure.
Each policy must state who owns retained bytes and what happens at its bound.
Unbounded observer retention is not an implicit fallback.

## Receipts and ledger

Every accepted or rejected turn has a checked monotonic `TurnId`. Accepted
inputs have monotonic input sequence identities and an ordered input range. A
turn receipt has an owned header describing its turn, durable transaction,
input range, final status, and optional bounded failure information. Its body
describes immutable logical state changes and execution evidence without
embedding legacy mutable `Value` instances.

Receipts implement exact retained-size accounting. Before execution or, where
the size depends on execution, before publication, the executor reserves both
record count and byte capacity plus a ledger sequence. The actual owned receipt
is bound to the permit before publication. Appending the prepared receipt after
publication is infallible and allocation-free. Dropped unused permits and
unpublished prepared records return their reservations. The retained ledger is
FIFO, bounded by record and byte limits, and never silently evicts required
records.

## Effect outbox

External effects are represented as owned intents containing a deterministic
turn-and-ordinal identity, bounded operation and target strings, an owned
payload, an idempotency key, and an explicit delivery policy. The default is
at-least-once; exactly-once delivery is not claimed. A provider-transactional
policy is valid only when the provider explicitly supplies that contract.

Outbox count and byte capacity is reserved and the actual owned batch is
prepared before publication. Duplicate effect identities are rejected during
preparation. After publication, queue insertion is infallible and preserves
`TurnId`, then ordinal, ordering. Delivery, acknowledgement, retry, and
dead-letter handling occur outside publication and never borrow instance or
workspace memory.

## Durability

The durability policy is selected explicitly; not every turn synchronously
persists before publication.

| Policy | Before publication | After publication |
|---|---|---|
| `Volatile` | validate and secure required effect capacity | publish and transfer eligible effects |
| `Retained` | reserve and bind owned record/outbox capacity | publish, then infallibly append retained records |
| `AsynchronousDurable` | reserve and bind durable queue/outbox capacity | publish, then infallibly enqueue owned records |
| `SynchronousDurable` | persist prepared receipt, replay/redo material, and durable outbox record | publish, mark applied, make effects eligible |
| `ReplicatedDurable` | satisfy the configured replication preparation protocol | publish or acknowledge according to the deployment contract |

The system reports `Applied`, `Recorded`, `Durable`, and `EffectsCompleted` as
separate acknowledgements. A policy must not claim a later acknowledgement
merely because an earlier one succeeded.

For durable store work, validation and capacity reservation complete before
semantic mutation. The prepared commit owns all decisions and data required
for infallible apply. Durable backends provide the equivalent atomic contract
through their own transaction mechanisms. An ordinary preparation error means
no semantic store change; an indeterminate result requires explicit resolution
and poisons or fences the coordinator until resolved.

Durable transaction records, runtime events, turn receipts, and outbox records
have distinct roles. A transaction record proves the store batch result. A turn
receipt describes the semantic input-to-publication decision. Events preserve
the compatibility lifecycle stream. The outbox owns work that must occur after
publication.

## Explicit transaction overlays

An explicit transaction owns an overlay above the instance's published
snapshot. Child operations may establish savepoints inside that overlay. A
child failure restores only the child candidate, events, effects, capabilities,
and program state, while retaining earlier explicit-transaction work. The
outer commit validates and prepares one complete durable and publication unit;
the outer abort discards the overlay and releases reservations.

Gate A does not redesign explicit transactions. In particular, it retains the
current `RuntimeTransaction` operation-savepoint clone and the existing program
rollback journal. Later gates must replace those mechanisms without weakening
child rollback or outer atomicity.

## Cross-instance limitation

Publication is atomic only within one `ReactiveInstance`. Gate A and the target
described here provide no distributed transaction across instances. A workflow
that affects several instances must use an explicit higher-level coordinator,
idempotent messages, or compensation. It must not imply cross-instance atomic
visibility from coincident turn or ledger sequence numbers.

## Gate A scope

Gate A freezes behavior and introduces measurements, replaces context-event
history copies with mark-based rollback, replaces complete in-memory-store
cloning with prepare/apply commit, and adds bounded owned recording and outbox
primitives. It does not introduce resident execution, change the public value
model, change bytecode v1, begin bytecode v2, alter native generation, add
dependent shapes, or route production turns through the new primitives.

Gate A is successful when the legacy path is bounded and measurable:

- context checkpoint and event-emission snapshot item counts are zero;
- complete `InMemoryStore` clone counts are zero;
- context event visibility and rollback remain exact under retention;
- in-memory commits prepare all expected failures before semantic mutation;
- prepared ledger and outbox appends cannot fail or allocate after publication;
- retained history does not create an append-cost slope.

The Gate A microbenchmark hard requirements are structural:

- zero complete context-history snapshots after A1;
- zero complete `InMemoryStore` clones after A2;
- no iteration across unrelated retained records;
- work proportional to new, touched, or batch data;
- no recoverable failure after prepared apply or append begins;
- bounded amortized event compaction.

Absolute median and p95 ratios across history sizes remain diagnostic evidence
for Gate A. The earlier `1.10 ×` A1/A2 microbenchmark reference is not an
architecture pass/fail criterion. Gate B's raw-Rust efficacy ratios remain hard.

## Gate B efficacy benchmark

The primary Gate B workload is the retained EKF turn. An alternative workload
may not replace it without an explicit architecture amendment approved before
implementation. Gate A does not implement the EKF.

Before timing begins, Gate B freezes the equations, matrix and state dimensions,
f64 precision, matrix storage order, factorization and solve algorithm, initial
state, deterministic input sequence, accepted numerical tolerance, state and
output hash rules, and primary and scaled sizes. Every comparison lane uses
that same frozen work.

The required lanes are:

- **rust-kernel**: direct preallocated Rust implementation with no scheduling,
  epoch, receipt, or ledger;
- **rust-epoch**: raw Rust with equivalent candidate storage, validation, epoch
  publication, receipt construction, and retained append;
- **numpy-persistent**: NumPy in a persistent Python process, excluding process
  startup and IPC from internal timing;
- **mech-legacy-atomic**: the current ordinary atomic turn;
- **mech-resident-kernel**: typed resident kernels and arena access without a
  scheduler or receipt;
- **mech-resident-turn**: complete resident input, scheduling, execution,
  validation, publication, receipt, and retained ledger append.

A fail-stop lane may remain diagnostic only; it is not a decision lane. A
secondary full-write matrix pipeline detects copies of complete committed state
or complete output buffers, but cannot substitute for the retained EKF.

Primary `mech-resident-turn` timing begins immediately before input installation
and ends after the owned receipt is appended to the retained in-memory ledger.
It includes input installation, write barriers, dirty scheduling, kernel
execution, candidate shape and integrity validation, epoch publication, receipt
construction, and retained ledger append. It excludes parsing, bytecode
decoding, activation, process startup, input generation, benchmark IPC, output
formatting, disk persistence, network delivery, and effect execution.

Gate B passes the correctness gate only when all lanes produce equivalent
accepted state and outputs within tolerance. Its remaining hard criteria are:

Let `T_lane` denote that lane's median primary-size per-turn time under the
same controlled run after the correctness gate passes. The efficacy formulas
are:

```text
legacy_gap_closure =
  (T_mech-legacy-atomic - T_mech-resident-turn) /
  (T_mech-legacy-atomic - T_rust-epoch)

raw_epoch_ratio = T_mech-resident-turn / T_rust-epoch

executor_tax = T_mech-resident-turn - T_mech-resident-kernel
```

`legacy_gap_closure` is valid only when
`T_mech-legacy-atomic > T_rust-epoch`; a non-positive denominator cannot satisfy
the Gate B closure criterion. Durations in the executor-tax comparison use the
same unit, including the `15 microseconds` floor below.

- the fixed EKF resident turn performs zero steady-state allocations;
- publication uses one constant-time instance operation;
- no complete committed arena, event history, store, or full-write output clone;
- no positive algorithmic slope from prior turn or retained record count;
- `legacy_gap_closure >= 0.80`;
- `raw_epoch_ratio <= 1.25`;
- `executor_tax <= max(0.20 × T_mech-resident-kernel, 15 microseconds)`;
- p95 is at most `1.50 ×` median.

The external target is `mech-resident-turn <= 1.10 × numpy-persistent`. Missing
that target may produce a **Conditional pass** only when all raw-Rust and
structural gates pass and the remaining cost is isolated to kernel selection,
the linear algebra backend, or data layout.

**Pass** means every hard criterion and the NumPy target pass. **Conditional
pass** means only the preceding narrowly defined NumPy exception applies.
**Fail** means any correctness, structural, raw-Rust, history, allocation,
publication, executor-tax, or tail criterion fails. A fail blocks production
cutover and broad operation migration.

## Legacy deletion inventory

The following mechanisms intentionally remain after Gate A and are deletion
targets for later gates:

- implicit `RuntimeExecutionTransaction` construction for standalone turns;
- `RuntimeTransaction` operation-savepoint cloning, including restore cloning;
- `ReactiveTurnJournal` and `ValueStateJournal` program rollback;
- `transaction_state_values()` discovery;
- `ValRef` as mutable execution storage;
- `ReactiveCellId` tied to current value storage;
- pointer-derived live binding and program-cell identity;
- the standalone coordinator's call to `commit_runtime`;
- the current public mutable `Value` model;
- `transaction_state_values()`-based capture of function state.

The legacy-boundary manifest permits these uses to disappear but rejects new
production paths or excess occurrences. Later gates remove the earliest owning
boundary and then tighten the manifest; they do not hide a legacy dependency in
a replacement wrapper.
