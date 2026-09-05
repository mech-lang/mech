# R5 memory planner

## 1. Status

R5 Memory planner — complete. R6 Memory runtime cutover — next.

R5 defines the deterministic memory plan consumed by the existing runtime. R5
predicts and validates
storage, capacity, placement, lifetime, aliasing, reuse eligibility,
transactions, budgets, and transfers. It does not introduce an allocator or
replace any backing.

The R5 branch is stacked on the exact R4 authority cutover. The root package
remains `0.3.6`, workspace package versions remain unchanged, and R5 metadata
is process-local and non-wire.

## 2. Sources of authority

Memory planning follows one direction:

```text
ResolvedCall
  -> BoundCall
  -> R2 resolved type-memory and operation-memory contracts
  -> physical storage description
  -> existing compute placement
  -> target and implementation memory requirements
  -> R5 memory plan
  -> existing runtime allocation
  -> shadow audit
```

The semantic descriptor determines topology, dimensions, cardinality, and
identity. `BoundCall` determines the operation, implementation, contract, and
execution target. R2 contracts determine storage and operation obligations.
The existing compute planner alone determines CPU/GPU placement. Target
profiles determine byte sizes, alignments, addressability, and target limits.
None of these authorities may be reconstructed from factory names, Rust type
names, pointers, cell allocation order, or catalog insertion order.

## 3. Program, activation, and turn planning

R5 has exactly three stages:

- `ProgramMemoryPlanTemplate` records immutable graph facts, symbolic capacity,
  lifetimes, aliases, reuse groups, transactions, placement, transfers, and
  explicit deferred witnesses. Executable sidecars carry their artifact node
  identities explicitly; marker, composite-pack, and synthetic state-hold
  nodes may legitimately have no call sidecar.
- `ProgramMemoryPlan` combines the template with activation facts and the
  target profile for each occupied memory space to produce exact fixed-layout
  sizes, alignments, arena placements, persistent and activation peaks, and
  remaining turn witnesses. Mixed compute plans use the native-host profile
  for Host values and the adapter profile for Device values.
- `TurnMemoryPlan` combines the program plan with current dynamic footprints
  and selector results before any covered clone, draft, comparison,
  finalization, stage, or publication begins.

There is no fourth stage and no planner registry. Plans contain deterministic
coordinates, not allocation handles or pointers.

## 4. Closed physical layouts

The physical layout vocabulary is deliberately closed:

- exact fixed-width scalars use a scalar fixed slot;
- scalar strings use a string header plus separate payload;
- homogeneous fixed-width matrices use column-major dense fixed slots;
- string matrices use column-major string headers plus separate payload;
- heterogeneous dense values use column-major canonical-value handles plus
  recursive payload;
- enums, options, tuples, records, tables, sets, maps, dynamic values, and
  reified types use canonical snapshots with their R2 topology.

R5 supports rank-two dense matrices. It does not add row-major host matrices,
arbitrary striding, packed records, user-selected alignment, or backend layout
plugins. Fixed byte arithmetic is checked. Host zero-sized values occupy zero
bytes; GPU storage bindings may not be zero-sized.
Complex32 and Complex64 use distinct target slots (8 and 16 bytes on the host);
one complex width is never used as the physical layout of the other.

## 5. Current extent versus required capacity

Current extent and required capacity are distinct. Fixed dimensions reserve
their exact extent. Activation-fixed dimensions reserve the activated extent.
Turn-bounded dimensions reserve their semantic upper bound. Turn-unbounded
dimensions and unbounded dynamic collections reserve the current witnessed
extent and require replanning before growth. Variable-width payloads reserve
their measured current payload and require replanning before payload growth.

A target policy limit is never promoted into a semantic bound. During R5 the
existing runtime may allocate only current capacity; a smaller runtime capacity
than the planned future requirement is reported as `CapacityDeferredToR6`.

Dimension bounds are evaluated recursively through constants, parameters,
addition, multiplication, minimum, and maximum with checked arithmetic and
cycle detection. Dynamic collection cardinality always comes from an explicit
value or activation witness and is checked against any semantic upper bound.

## 6. Lifetimes

Artifact program points are deterministic: `BeforeNode(n) = 2*n`,
`AfterNode(n) = 2*n+1`, and turn end is `2*node_count`. Intervals are closed;
newly live storage is added before storage ending at the same point is removed.

Constants live for the program. Inputs, state, feedback, and published outputs
live for the activation. Derived temporaries live from their producer through
their final consumer. Scratch and transaction stages span their node.
Transfers span their producer/consumer or output boundary. Resident and GPU
state plans include both simultaneously live buffers.

## 7. Alias and reuse groups

Alias decisions derive only from the operation contract and exact physical
compatibility. `NoAlias` never aliases an input. `MayAlias` may reuse only an
eligible derived turn temporary with no later consumer and no loss of failure
atomicity. `InPlaceRequired` requires the specified compatible input plus an
undo snapshot; an unavailable required alias rejects planning.

Alias groups use deterministic union-find and the smallest memory-object ID as
their identity. Reuse is a plan result only. Eligible fixed-width turn
temporaries use deterministic first-fit by lifetime and object ID. R5 neither
reuses nor frees an allocation.

Call-local port, scratch, and transaction identities are remapped into one
program-wide object namespace before publication. The program planner then
validates that every port and transaction reference names an allocation in
that namespace. Resident execution consumes those remapped calls; a
call-local Resident plan is never installed directly into a program plan.

## 8. Transaction requirements

Direct, native, and WASM publications use stage-and-swap. Their read-modify-
write operations also use stage-and-swap. Resident CPU and GPU state use double
buffers. Required in-place operations use undo snapshots. Read-only operations
and unit external effects need no transaction storage.

Published outputs are a distinct planned value class, not state. A published
output uses stage-and-swap unless it is also independently declared as
Resident/GPU state; only that state declaration authorizes double buffering.
The producing call and value plan reference the same globally identified
transaction stage, so this policy never creates a second call-local copy.

The plan counts old and candidate values simultaneously wherever failure
atomicity or rollback requires their coexistence.

## 9. Host/device placement and transfers

Memory spaces are `Host`, `ResidentCpu`, and backend-neutral `Device { region }`.
R5 consumes the existing compute placement; it does not choose a different
target in response to a limit.

Transfers are planned only across an existing host/device boundary or when a
device producer becomes an externally retained output. Uploads and readbacks
are deduplicated by direction, slot, consumer, and interface name. Each plan
records current and capacity bytes plus its transfer lifetime.
The descriptor is the single accounting authority for transfer demand; its
backing allocation is placed but is not charged a second time.
The GPU host's adapter-sized binding arenas are a subordinate
`GpuBackingMemoryPlan` projection of an already selected `GpuExecutionPlan`.
They are not a semantic `ProgramMemoryPlan` and cannot define operation,
alias, lifetime, or transaction policy.

## 10. Resource demand and target limits

Resource demand records persistent, activation, turn, transaction, clone, and
transfer bytes; retained nodes; output elements; storage bindings; and checked
comparison, compute, canonicalization, and scalar-instruction work.

Resident profiles preserve the existing limits: 65,536 output elements,
16 MiB output, temporary, and clone bytes, 65,536 retained nodes and comparison
work, and 16,777,216 compute work. GPU compiler planning preserves 65,536
static-selector elements/source steps and caps total scalar instructions at
16,777,216. Adapter buffer, binding, alignment, workgroup, and invocation
limits come from the queried adapter. Direct, native, and WASM hosts add no
arbitrary quota beyond checked addressability.

Violations identify the owning plan object, budget dimension, required amount,
and limit in deterministic order.
Turn-deferred input and output witnesses are keyed by node, direction, and port.
The turn planner replaces their zero-valued placeholders and re-runs the same
complete call planner used for the original call. Clone, transaction,
canonical finalization/sorting, semantic-hash, retained-node, and output
obligations are therefore derived together rather than patched field by field.
Resolved transaction-stage families (fixed storage plus any separate Resident
payload backing) are resized as one complete footprint and re-placed before
admission; payload growth is never applied independently to every component.

## 11. Implementation scratch classes

Every maintained runtime and resident implementation declares one closed
memory class:

- `NoAdditionalScratch` for operations needing only generic publication;
- `CloneInput` for an extra complete input-proportional allocation;
- `MatrixSolve` for coefficient, solution, pivot, and cubic-work planning;
- `CanonicalFinalize` for complete canonical draft/finalization work;
- `CanonicalSortUnique` for canonical collections with ordering and dedup work.

There is no custom, opaque, unknown, callback, or plugin escape hatch. The
generic call plan always includes current inputs/outputs, the complete staged
candidate, the old value until success, change detection, and complete base
cloning for read-modify-write.

## 12. Shadow audit

The existing runtime continues to create `Vec`, nalgebra, canonical `Value`,
resident arena, and wgpu buffer allocations. R5 observes those allocations and
compares current bytes, capacity bytes, payload, nodes, and logical elements
with the immutable plan.

Fixed Resident and GPU observations must match exactly. Missing, unexpected,
or oversized observations are mismatches. A current allocation that is smaller
than future planned capacity is accepted only as `CapacityDeferredToR6`.
Production does not fail solely for that deferred capacity; tests and CI reject
all actual mismatches.
Resident String and canonical Snapshot constants and initializers contribute
their measured payload and recursive retained-node footprints. The shadow
audit measures those live backings instead of copying the planned values into
its observations.

## 13. Resident/GPU hard safety preflight

R5 remains a shadow planner except for existing safety boundaries. Resident
arena and turn limits are evaluated before the corresponding oversized arena,
clone, draft, finalization, comparison, stage, or write begins. One accumulated
turn plan covers all phases of a value-dependent operation. Empty indexed
assignments return unchanged after semantic validation without output-sized
planning or staging.

Resident kernels obtain permits only by reconciling their concrete estimate
with the real node-scoped `TurnMemoryPlan` from the activated program. The
kernel estimate cannot manufacture a semantic-empty plan or replace the
program's node identity, globally remapped objects, transactions, or arena
placement. Provider-resolved external contracts likewise re-run complete call
planning; changing a bound contract never leaves stale aliases, scratch,
transactions, or demand in its sidecar.

GPU buffer and binding limits are evaluated from the adapter-backed profile
before buffer creation. Cartesian scalar-instruction expansion is calculated
with checked arithmetic and admitted before reserve, register creation,
instruction append, or WGSL generation. A rejected expansion leaves compiler
collections unchanged.

## 14. Preserved compatibility

R5 does not change bytecode-v1, canonical encoding v1, `ProgramArtifact`,
`GpuExecutionPlan` v1, `NativeBuildPlan`, dynamic-module ABI v1, operation IDs,
runtime IDs, native linkage names, package versions, or `Cargo.lock`. Memory
plans and compiler sidecars deliberately implement no serialization format.

R5 also introduces no allocator, pool, free list, reclamation, copy-on-write,
backing replacement, pointer identity, buffer movement, language syntax,
conversion, overload, placement policy, backend, or user memory annotation.

## 15. R6 handoff

R6 consumes the R5 layouts, capacities, arena placements, lifetimes, alias
groups, reuse groups, transaction requirements, budgets, and transfer
requirements. R6 may implement allocation handles, pools, managed backing,
actual reuse, movement, publication, and reclamation. R6 may not silently
derive a different physical plan.

## 16. Completion criteria

R5 closes when every maintained value, call, artifact slot, implementation,
resident arena, compute transfer, and GPU buffer has one deterministic plan;
all variable work has an explicit witness; all current allocations conform to
the shadow audit; the deferred Resident and GPU safety gaps reject before
materialization; fresh processes produce byte-for-byte identical diagnostics;
and the R1 through R5 architecture contracts and exact-head CI are green.

Given the same semantic program, selected implementations, target profile, and
activation/turn facts, the plan must be independent of pointers, cell
allocation order, catalog insertion order, and process identity.

## Review correction: live turn evidence and complete scratch geometry

Resident execution now measures borrowed input values and the separately retained
published output immediately before entering a node's turn-planning scope.
String retained capacity and logical encoded length are distinct. Canonical
Snapshot measurements use the bounded shared traversal and retain current
cardinality and node count. Resolved selector populations are recorded as region
facts rather than substituted with `WholeValue`.

A concrete kernel preflight refines the candidate before receiving its existing
materialization permit. Published-output footprints remain separate: equality
work covers the old value and the new candidate independently. Where only the
candidate's retained footprint is available before construction, its complete
retained size is the conservative encoded-size bound; it is not reported as a
measurement of an already-created candidate.

Implementation clone, solve, pivot/index, and canonical draft/finalization
requirements now have allocation identities, layouts, lifetimes and placements.
Temporary-byte demand is derived from these records, not only a scratch counter.
Their turn-dependent sizes are re-derived and re-placed when live facts change.

Aggregate memory admission composes overlapping temporary and transaction
lifetimes per memory space, and checks transfer/binding totals while preserving
existing per-call compute, comparison and output limit scopes. It does not add
new execution quotas or implement managed allocation/reuse.

### Fixed-width execution cost

Resident execution memoizes the validated base turn plan only when all ports
have activation-invariant fixed-width storage, no deferred witnesses or regions,
and no additional implementation scratch. Cached plans never contain an
execution permit: every kernel still admits its concrete demand. String and
Snapshot payloads and data-dependent selectors continue to supply live facts.

Incremental borrowed traversals check accumulating demand against the active
node plan without rebuilding schemas, scratch allocations, or arena placement
for every visited chunk. The progress check cannot accept candidate publication
facts and cannot issue a materialization permit. Final admission still resolves
the complete candidate and runs the full checked planner.
