# Type-memory boundary

## 1. Status: R2 complete

R2A derives type-memory contracts from finalized schemas and validated shapes.
R2B describes the capabilities of actual backings and separates logical-cell
identity from physical-storage identity. R2C derives operation-port memory
requirements from R1 declarations and performs opt-in shadow checks. R2D
installs stack-wide conformance, permanent architecture enforcement, and CI
ownership for the complete boundary.

The boundary remains descriptive and shadow-only. It changes no runtime
binding, storage, target, or execution behavior. R4 makes the compatibility
boundary authoritative during the binding cutover.

The projection has one direction:

```text
finalized Schema ---------------------> TypeMemoryContract
       |
       +-- revalidated ShapeInstance -> ResolvedTypeMemoryContract
```

The contracts describe obligations that a later storage implementation must
satisfy. They are derived metadata, not another type system.

## 2. Sources of authority

`Schema` remains the sole authority for semantic type, child structure,
nominal identity, field and variant names, equality, `SchemaKey`, and canonical
encoding. `ShapeInstance` supplies dimension-parameter values only after the
target schema validates them. Consumers that need children or names continue
to traverse `SchemaBody`.

| Question | Authority |
| --- | --- |
| What is the semantic type? | `Schema` |
| What is the current validated shape? | `ShapeInstance` |
| What memory-facing structure does a type require? | `ResolvedTypeMemoryContract` |
| What can an existing backing provide? | `StorageCapabilityDescriptor` |
| What does an operation port require? | `PortMemoryRequirement` derived from `OperationContractDeclaration` |
| Can the combination coexist? | R2 compatibility checks |
| What concrete runtime factory/backing is selected today? | transitional runtime machinery |
| When does R2 compatibility become binding authority? | R4 |
| What physical byte layout is chosen? | R5 |
| How is memory allocated, reused, and reclaimed? | R6 |

## 3. One-way boundary

There is no conversion from either memory contract back to a schema. Contract
derivation does not cache data in `Schema`, and the contracts have no encoder
or decoder. A contract cannot create or alter a schema or shape.

## 4. TypeMemoryContract

`TypeMemoryContract` records:

- logical memory topology;
- symbolic extent and its maximum evolution class;
- positional, named, or keyed addressing obligations;
- canonicalization obligations;
- payload, population, and auxiliary accounting classes.

It retains dimension and cardinality expressions only where a later phase must
resolve extent. It does not copy nominal keys, names, or complete child
contracts.

## 5. ResolvedTypeMemoryContract

`ResolvedTypeMemoryContract` has the same topology and obligations, but its
extent contains checked values resolved through a shape revalidated by the
target schema. Matrix axes retain independent evolution classes. Dynamic
collection extents retain an optional resolved upper bound; they do not invent
a current cardinality that is absent from `ShapeInstance`.

## 6. Complete SchemaBody mapping

| Schema body | Topology | Extent | Payload / population | Auxiliary |
| --- | --- | --- | --- | --- |
| `Dynamic` | `Dynamic` | `Single` | self-describing / single | none |
| `Bool` | scalar Boolean | `Single` | fixed-width / single | none |
| unsigned integer | scalar unsigned width | `Single` | fixed-width / single | none |
| signed integer | scalar signed width | `Single` | fixed-width / single | none |
| floating point | scalar floating width | `Single` | fixed-width / single | none |
| complex | scalar complex width | `Single` | fixed-width / single | none |
| `Rational64` | scalar rational | `Single` | fixed-width / single | none |
| `String` | scalar string | `Single` | variable-width / single | none |
| `Id` | scalar id | `Single` | fixed-width / single | none |
| `Index` | scalar index | `Single` | fixed-width / single | none |
| `Atom` | scalar atom | `Single` | fixed-width / single | none |
| enum | tagged, variant count | `Single` | recursive / single | tag |
| option | tagged, two variants | `Single` | recursive / single | tag |
| tuple | unnamed product | fixed arity | recursive / fixed arity | none |
| record | named product | fixed arity | recursive / fixed arity | none |
| matrix | dense sequence, rank | dimensions | recursive / shape-resolved | none |
| table | columnar, column count | row cardinality | recursive / exact or value cardinality | column directory |
| set | ordered set | cardinality | recursive / exact or value cardinality | ordered index |
| map | ordered map | cardinality | recursive / exact or value cardinality | ordered index |
| `ReifiedType` | reified type | `Single` | self-describing / single | none |

`Dynamic` and `ReifiedType` are self-describing. Enum and option values are
recursive and tagged. Products, dense sequences, columnar values, sets, and
maps are recursive. Sets and maps require ordered, unique keys.

## 7. Extent evolution

Evolution is ordered as:

```text
Fixed < ActivationFixed < TurnBounded < TurnUnbounded
```

Constants are fixed. Activation parameters are activation-fixed. Turn
parameters are turn-bounded when they have an upper bound and turn-unbounded
otherwise. `Add`, `Multiply`, `Min`, and `Max` take the maximum evolution of
their operands. Exact cardinality follows its expression. Bounded dynamic
cardinality is at least turn-bounded; unbounded dynamic cardinality is
turn-unbounded. Composite evolution joins the extent with every nested child.
String payload length is variable-width accounting, not extent evolution.

Finalized schemas cannot retain holes or compile-time parameters. Resolution
reports existing structured semantic errors for invalid or overflowing
expressions.

## 8. Addressing semantics

Whole-value access is universal and is not represented by a flag. Strings and
tuples have positional rank one. Matrices use their dimension count as rank.
Tables have positional rank two and named members. Records have named members.
Sets and maps have keyed members. The contract records obligations, not an
offset, stride, index implementation, or physical lookup structure.

## 9. Canonicalization obligations

Self-describing values carry enough semantic information to identify their
concrete value form. Recursive values require child canonicalization. Tagged
values preserve their discriminant. Ordered collections preserve canonical key
order, and unique-key collections cannot retain duplicates. These are logical
requirements and do not prescribe a representation.

## 10. Accounting obligations

Payload accounting distinguishes fixed-width, variable-width, recursive, and
self-describing values. Population accounting distinguishes single values,
fixed products, shape-resolved sequences, exact cardinalities, and
value-supplied dynamic cardinalities. Auxiliary accounting identifies tags,
ordered indexes, and table column directories. R2A supplies classifications,
not byte counts or budget enforcement.

## 11. Semantic identity versus physical storage

Schema equality and `SchemaKey` define semantic type identity. Logical value
identity remains governed by the value model. A pointer is never semantic
identity, and runtime representation is never logical value identity. Neither
contract contains pointers, runtime cells, owners, targets, factories, or
placement.

## 12. Serialization prohibition

R2 contract types deliberately implement no serialization traits and have no
wire format. Canonical schema and operation-contract bytes remain
unchanged and authoritative. Derived contracts must be recomputed from their
semantic authorities rather than persisted as another compatibility surface.

## 13. R2B storage capabilities and identity

R2B describes existing storage capabilities and separates logical cell identity
from physical storage identity. `StorageCapabilityDescriptor` is derived from
the actual backing. The public compatibility boundary accepts a finalized
`Schema` and validated `ShapeInstance`, rederives the R2A contract internally,
and then checks the backing. Callers cannot pair a schema with a contract
derived from another schema. The checker remains opt-in and shadow-only.

`same_cell` remains a compatibility alias for physical storage identity. New
code chooses `same_logical_cell` or `same_storage` explicitly. No public
physical storage identifier exists, and neither pointers nor runtime
representations can become logical identity.

### Known transitional mismatch

Current `RowDVector` inference marks both dimensions as turn-varying even
though its first physical axis is fixed at one. Current `DVector` inference
likewise marks both dimensions as turn-varying even though its second physical
axis is fixed at one. R2B deliberately reports both as
`DynamicAxisUnsupported` during shadow validation; weakening the truthful
storage descriptors would conceal the mismatch.

R4 must infer each physically invariant vector axis as `Constant(1)` before the
compatibility boundary becomes authoritative. R2C and R2D must not interpret
these expected shadow failures as valid storage-incompatibility policy.

## 14. R2C operation memory requirements

R2C derives `OperationMemoryRequirements` from
`OperationContractDeclaration`. Fixed and variadic inputs use the declaration's
existing resolution path. Each port preserves access and delivery, while
ownership, addressing, publication, construction, aliasing, and change
detection are projected from the existing policy fields. External interaction
is deliberately excluded because provider protocols are not cell-storage
requirements.

The public port checker accepts `Schema`, `ShapeInstance`, a derived port
requirement, and `StorageCapabilityDescriptor`. It resolves the type-memory
contract once and checks the complete compatibility triangle: semantic type to
storage, operation-port addressing to semantic addressing, and operation-port
requirements to storage capabilities. Canonical `Value` storage is mechanically
universal but cannot authorize positional, collection-entry, or regional access
that the semantic type does not expose. Stream and Future delivery remain
visible metadata and are not rejected by this generic storage boundary.

`FunctionInvocation::check_operation_memory_contract` is a separate opt-in
shadow check. It validates the current single-output compatibility bridge and
uses `same_storage` for operation alias policy. It is not called from runtime
signature checks, factories, source specialization, Resident or GPU
activation, or native planning. R2C does not make these requirements
authoritative and does not mark R2 complete.

## 15. R2 closure

R2 is complete when the conformance matrix, architecture checker, normal CI,
and exact-head Full CI are green. Completion does not mean the checker controls
production binding: the compatibility boundary remains shadow-only until R4.

## 16. R3 closure

R3 consumes `Schema`, `KindExpr`, `KindScheme`, validated dimensions, and
declared operation requirements to produce closed semantic types before
physical binding. Type System v1 inference, built-in classes, promotions,
conversion plans, and diagnostics are complete and authoritative. It does not use `StorageCapabilityDescriptor`,
`FunctionValueRepresentation`, exact Rust matrix backing classes,
`CanonicalCellId`, `same_storage`, or pointer or allocator identity as
inference inputs.

## 17. R4 handoff

R4 consumes the complete R2 compatibility boundary. It owns the production
binding cutover, removal of representation-based semantic decisions, and
correction of the `RowDVector`/`DVector` invariant-axis inference mismatch.

## 18. R5/R6 handoff

R5 owns deterministic physical layouts; sizes, alignment, strides, offsets,
and placement; lifetimes and alias plans; and allocation and resource plans.
R6 owns allocators, backing cutover, pooling and reuse, copy-on-write,
reclamation, and runtime enforcement. R2 contains none of those mechanisms.

## 19. Non-goals

R2 does not add another operation declaration surface, interaction-provider
requirements, physical layout, allocation, inference, conversion, target
availability, function binding, production rejection, `ValueCell` mutation,
Resident or GPU behavior, bytecode, canonical encoding, ABI changes, or
package-version changes.

## 20. R2 completion criteria

R2 is complete when:

1. Every finalized `SchemaBody` derives deterministic `TypeMemoryContract` data.
2. Every resolved contract revalidates its `ShapeInstance` against the schema.
3. Existing `ValueCell` backings advertise truthful declarative capabilities.
4. Canonical `Value` storage is mechanically universal.
5. Canonical `Value` storage cannot override semantic addressing eligibility.
6. Exact scalar storage preserves scalar kind.
7. Exact matrix storage preserves element kind and physical extent constraints.
8. Operation memory requirements derive only from `OperationContractDeclaration`.
9. Fixed and variadic inputs use `InputPortLayout::resolve`.
10. Delivery modes remain descriptive rather than generic storage rejections.
11. Semantic addressing and backing addressing are separate checks.
12. Logical value identity, logical cell identity, and physical storage identity are distinct.
13. Operation alias policy uses physical `same_storage`.
14. The current zero-output unit bridge is checked explicitly.
15. Multiple semantic outputs fail honestly in the current invocation bridge.
16. R2 analysis is deterministic and non-mutating.
17. R2 metadata is not serialized.
18. R2 contains no physical layout, allocation, or reclamation policy.
19. The known `RowDVector`/`DVector` inferred-axis mismatch remains explicit and assigned to R4.
20. R2 checks remain shadow-only.
21. Normal CI runs the architecture checker.
22. Full CI runs the architecture checker.
23. Checker changes themselves trigger Full CI.
24. README, ROADMAP, type-memory design, and v0.4 endgame agree that R2 is complete and R3 is next.
25. Package version remains `0.3.6`.
