# R6 memory runtime cutover

Status: implementation in progress. R5 is complete; R6 is not complete until
the qualification gates in this document pass on one exact candidate.

R6 consumes the R4 semantic certificate and the R5 physical memory plan. It
changes who owns and realizes storage; it does not infer types, choose
operations, change placement, or introduce another planner.

## 1. Delivered authority

The completed cutover has one chain of authority:

```text
ResolvedCall
  -> BoundCall
  -> R2 type and operation memory contracts
  -> R5 ProgramMemoryPlan / TurnMemoryPlan
  -> borrowed RuntimePlanView
  -> MemoryReservation
  -> RealizedMemoryPlan
  -> managed execution leases and publication
```

`RuntimePlanView`, `MemoryReservation`, and `RealizedMemoryPlan` are
process-local runtime records. They are not serialized planning stages and do
not change `ProgramArtifact`, bytecode-v1, `GpuExecutionPlan`, or
`NativeBuildPlan`.

R6 implements the storage described by R5. It may reject a malformed, stale,
unrealizable, or over-budget plan. It may not repair one by selecting a new
layout, offset, capacity, alias, lifetime, transaction, transfer, or target.

## 2. Ownership and identity

Each running instance owns one owner-thread-confined `MemoryDomain`.
Standalone direct calls establish a session domain through the same managed
entry point. There is no process-global pool and no user-extensible allocator.

The following identities are independent:

- `CanonicalCellId`: stable logical language cell identity;
- `MemoryPlanRevision`: one validated physical-plan revision;
- `PlanObjectKey`: an R5 `MemoryObjectId` under a plan revision;
- `AllocationHandle`: domain-local physical ownership slot and generation;
- `PublishedValueVersion`: semantic publication/change version;
- `RegionIncarnation`: validity epoch for a reused arena region.

Only their owning authorities construct these values. Checked counters never
wrap. An exhausted allocation slot is permanently retired, and exhaustion of
any other identity rejects before mutation.

Every clone of a mutable `ValueCell` shares one stable cell record. That record
owns the immutable logical ID and schema authority and one atomic publication
record containing shape, publication version, and storage binding. Physical
movement changes allocation generation or region incarnation without changing
logical identity. Equal-content relocation does not report a semantic change.

Managed bindings are closed: host region, canonical payload, device binding,
or an explicitly pinned external boundary. Ordinary source-created values use
managed storage. Pinned backing exists only for explicit ingress and ABI
compatibility.

## 3. Arena realization

R5 declares `ArenaBackingKind` for every arena:

- `ContiguousBytes` owns one alignment-correct block and uses literal planned
  offsets for fixed storage, scratch, transfers, and device buffers.
- `IndirectOwnedPayloads` reserves disjoint charged envelopes for canonical
  recursive payloads whose Rust blocks remain separately owned. Its offsets
  are accounting coordinates, not addresses, and it is not eligible for
  fixed-region reuse.

Host blocks use `Layout::from_size_align`, fallible allocation, and the exact
original `Layout` for deallocation. Zero-byte objects have an explicit empty
binding. Initialization state is separate from capacity, so no read reaches
padding and every successfully constructed nontrivial element is destroyed
exactly once on success, error, or unwind.

Variable canonical storage uses the sealed `PlannedAllocator`. Its clones
share one finite reservation. Allocation and exact growth transfer reserved
authority to physical ownership; a failed request leaves both the container
and accounting unchanged. Fixed arenas never depend on incidental `Vec<u8>`
alignment or interior-pointer ownership tricks.

## 4. Ledger and reservations

The domain ledger reports, without double charging aliases:

- reserved bytes;
- physically committed bytes;
- retired-but-pinned bytes;
- exported immutable snapshot bytes;
- in-flight device and transfer bytes;
- the existing work, output, clone, container, selector, index, and retained
  node dimensions at their existing scopes.

A reservation is non-forgeable, releases unused authority on drop, and cannot
authorize an object absent from its plan view. Unknown witnesses are not
treated as zero. Registry, lease, initialization, transaction, retirement,
and recursive child-block metadata are bounded before activation becomes
visible. Accounting underflow and overflow are structured invariant errors;
they are never hidden by saturating arithmetic.

Temporary-byte admission also checks the capacities and closed lifetimes of
the supplied allocations independently of the semantic demand summary. All
objects live at one plan point count together, including transaction payloads
and construction workspace; disjoint reuse members count only during their
own lifetimes. An understated summary cannot authorize an over-budget physical
plan, and this validation does not change its placement or capacity.

Detached immutable snapshots retain a pointer-free accounting ticket. The
ticket does not retain a mutable domain, cell, or executor, and releases its
physical charge when the last snapshot owner is dropped.

## 5. Managed execution and leases

Maintained function implementations execute through
`MechFunctionImpl::solve_managed` with a `KernelMemoryFrame` and execution
services. `FunctionInstance` convenience methods establish that scope; there
is no unmanaged default implementation path.

`ManagedPort<T>` is a logical port capability. It stores no permanent pointer,
allocation handle, owning matrix, or mutable payload. A
`PreparedCallAccess` caches only plan-revision geometry, permissions, and
bounded lease workspace. Each invocation resolves current handles and checks
domain, generation, incarnation, shape, selectors, and bounds.

The frame acquires the complete call access set atomically in deterministic
domain/slot/region order. Reads may overlap reads. Any overlapping write
conflicts unless a single exclusive lease implements the explicit in-place
plan. Partial acquisition is released before returning an error. Leases cannot
escape, be cloned, be serialized, or cross the owner thread.

Contiguous conflicts use exact half-open byte intervals. Strided and rectangle
access conservatively uses their enclosing spans. Column-major execution views
honor reserved row capacity; canonical matrix snapshots remain row-major and
copy through explicit coordinates.

## 6. Growth and plan replacement

Activation realizes R5 required capacity, including bounded dimensions. A
managed production audit cannot report `CapacityDeferredToR6`.

Growth within capacity initializes only newly exposed elements and publishes
shape plus binding atomically without allocation. Growth beyond capacity is:

```text
measure bounded current and candidate facts
  -> request a revised R5 plan
  -> reserve old + replacement + staging coexistence
  -> allocate replacement
  -> initialize or copy the complete candidate
  -> validate every output
  -> atomically publish shape and bindings
  -> retire old ownership
```

The `CanonicalCellId` remains stable. A changed arena stride or placement
replaces and rebinds the complete affected arena at a safe point. No geometric
growth, background compaction, automatic shrinking, hidden retry loop, or
unplanned fallback allocation is permitted. Any failure leaves the previous
publication, shape, identity, and ledger unchanged.

## 7. Publication and transactions

R6 executes the exact R5 transaction requirement:

- `StageAndSwap`: publish a fully validated candidate binding;
- `DoubleBuffer`: write the non-published state buffer, then switch identity;
- `UndoSnapshot`: create the complete admitted undo image before mutation;
- `None`: no backing transaction, while leases and lifetimes still apply.

A multi-output preparation owns all candidate bindings, undo records, shape
updates, dirty records, and reservations. Everything fallible happens before
the owner-thread commit point. Commit performs no allocation, callback, or
validation and is applied exactly once. Dropping an uncommitted preparation
aborts. Embedded fixed regions use the same explicit staged requirement and a
prevalidated allocation-free copy when rebinding a subregion would violate
arena ownership.

## 8. Reuse, retention, and reclamation

R6 obeys R5 reuse groups only after executor retention facts prove an object is
ephemeral. Cached incremental values remain retained when a later turn can use
them while their producer is skipped. R6 never forces execution or silently
allocates replacement storage to make reuse convenient.

Owned records transition only through `Reserved`, `Initialized`, `Live`,
`Retired`, and `Free`. Region incarnation is revoked before another reuse-group
member becomes accessible. Reclamation waits for destruction plus every lease,
snapshot owner, external pin, and device-submission hold.

`MemoryDomain::close` prevents new work, aborts unpublished CPU candidates,
retires instance storage, and drains existing GPU completion handling before
device reuse. Exported snapshots remain valid and pin only their immutable
payload. Ownership graphs use weak links where necessary and must return to
zero after repeated activate/run/abort/close/drop cycles except for explicitly
retained snapshots.

## 9. Canonical payloads

`ManagedString` and `ManagedSequence<T>` expose fallible construction and exact
reservation-backed growth without leaking their mutable allocator. Canonical
`ValueData` variants, validation, ordering, equality, schema authority, and
wire encoding do not change.

Cloning a finalized `Value` retains an immutable owner. A real deep copy uses
an admitted managed draft builder and preserves recursive work accounting.
Exporting mutable dense storage produces a planned canonical snapshot copy;
retaining an already immutable value is allocation-free except for explicitly
planned export bookkeeping.

Payload-dependent calls measure current inputs, the separately retained
published output, and the prospective candidate even when schemas and shapes
are unchanged. The complete R5 call plan is re-derived from those witnesses;
fixed-width calls whose requirements are invariant keep the cached fast path.
Maintained canonical builders receive their charged payload admission before
allocating result Strings, containers, or finalization storage, and complete
that admission only after the frozen value validates. Adoption of an already
existing external value is a distinct boundary. A payload-producing operation
without either a prospective witness or an explicitly declared external
adoption policy fails before execution; the previously published footprint is
never silently treated as authority for the next result.
Set operations, canonical matrix access/assignment, String matrix transpose,
and canonical matrix construction all use this prospective boundary; only
explicit external host/resource adoption may present an already-built value.

An ordinary same-schema, same-shape snapshot shares the published frozen data
and its accounting owner. Equivalent closed metadata in another schema table
may share that data only while the returned value retains the target table.
Schemas containing `Dynamic` children and shape/schema transformations use an
explicit canonical reconstruction so nested schema identities are remapped or
rejected; they cannot silently take the no-copy path or shed the retained ticket.

Payload envelopes retain their declared node-registration bound independently
of any spare capacity returned by the host allocator. Function binding also
revalidates every transaction variant and object pair against the declared
output construction. Required in-place calls coalesce repeated logical input
roles under one exclusive physical lease, and that lease remains installed
from mutation through commit or rollback.

## 10. Backends and compatibility

GPU realization extends the subordinate R5 backing projection. Every managed
buffer tracks plan object, capacity, usage, mapping, content version, handle
generation, and last submitted use. Submission completion updates an
`Arc`-owned atomic watermark; the owner thread performs reclamation. Queued
uploads, readbacks, copies, compute, and mapped views retain and charge their
storage until the existing completion boundary.

The browser uses its existing promise/event-loop completion path. R6 introduces
no per-kernel blocking wait, new GPU backend, placement policy, batching
policy, or unbounded input queue.

The V1 ABI remains call-scoped. Native/direct wrappers acquire leases for the
whole ABI call, and non-contiguous values use R5-planned contiguous bridge
storage. Existing `Ref<T>` constructors remain explicit pinned-external
boundaries; ordinary maintained kernels do not retain `Ref` payloads.

## 11. Errors and observability

R6 reports structured `MemoryRuntimeError` values for invalid revisions,
objects, handles, generations, incarnations, domains, layouts, lifetimes,
reuse, access, capacity, budgets, allocation, initialization, publication,
device loss, closure, and accounting invariants. Allocation/admission errors
are not flattened into semantic shape errors.

Audit observations come from actual blocks, payload tickets, initialized
extents, generations, incarnations, and device buffer sizes—not copied plan
numbers. Diagnostics distinguish logical demand, physical retained storage,
reservations, snapshots, and pending GPU retirement. Cross-process projections
exclude pointers and global domain IDs.

The ledger bounds Mech-owned memory under configured limits. It does not claim
to bound process RSS, allocator fragmentation, driver-private storage, or a
third-party module's private heap.

## 12. Compatibility and non-goals

R6 preserves package versions, dependency versions, `Cargo.lock`, the pinned
`nightly-2026-03-03` toolchain, canonical encoding v1, bytecode-v1, operation
and runtime IDs, native linkage names, module ABI v1, and every maintained
semantic oracle.

R6 adds no type rule, syntax, scheduler, automatic placement, kernel fusion,
NUMA or remote memory, process-global pool, compactor, tracing GC, general
copy-on-write framework, allocator plugin, or global allocator interception.
R7—not R6—performs release qualification and version changes.

## 13. Completion gate

R6 is complete only when one exact candidate proves all maintained direct,
source, Resident, bytecode, native, WASM, compute, GPU, and browser paths use
managed realization; growth preserves logical identity and previously bound
consumers; failed growth and publication are atomic; admitted reuse is physical
and safe; detached snapshots survive instance close; all unretained ownership
is reclaimed; Miri passes the safety suite; and normal plus Full CI are green.

Until that gate passes, documentation and roadmaps continue to say R6 is in
progress and R7 has not begun.
