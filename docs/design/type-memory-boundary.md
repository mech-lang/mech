# Type-memory boundary

## 1. Scope

Type-memory contracts derive from finalized schemas and validated shapes.
Backing capabilities separate logical-cell identity from physical-storage
identity; operation-port requirements derive from declared operation contracts.
Compatibility is mandatory before binding, storage installation, resident
construction, and host/resource ingress. The relation chooses no layout or allocator.

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
| Can the combination coexist? | type-memory compatibility checks |
| What concrete runtime factory/backing is selected today? | physical implementation binding after semantic validation |
| When does type-memory compatibility become binding authority? | before every physical binding |
| What physical byte layout is chosen? | the complete memory plan |
| How is memory allocated, reused, and reclaimed? | the managed memory runtime |

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
ordered indexes, and table column directories. Schema-derived memory contracts
supply classifications, not byte counts or budget enforcement.

## 11. Semantic identity versus physical storage

Schema equality and `SchemaKey` define semantic type identity. Logical value
identity remains governed by the value model. A pointer is never semantic
identity, and runtime representation is never logical value identity. Neither
contract contains pointers, runtime cells, owners, targets, factories, or
placement.

## 12. Serialization prohibition

Type-memory compatibility contract types deliberately implement no serialization traits and have no
wire format. Canonical schema and operation-contract bytes remain
unchanged and authoritative. Derived contracts must be recomputed from their
semantic authorities rather than persisted as another compatibility surface.

## 13. Storage capabilities and identity

Storage capabilities separate logical cell identity from physical storage
identity. `StorageCapabilityDescriptor` is derived from
the actual backing. The public compatibility boundary accepts a finalized
`Schema` and validated `ShapeInstance`, rederives the memory contract internally,
and then checks the backing. Callers cannot pair a schema with a contract
derived from another schema. Semantic binding invokes the checker as a mandatory binding
precondition.

`same_cell` remains a compatibility alias for physical storage identity. New
code chooses `same_logical_cell` or `same_storage` explicitly. No public
physical storage identifier exists, and neither pointers nor runtime
representations can become logical identity.

### Invariant vector axes

A dynamic row vector's first axis and a dynamic column vector's second axis
remain invariant. Semantic binding represents those axes as `Constant(1)`
before authoritative compatibility validation.

## 14. Operation memory requirements

`OperationMemoryRequirements` is derived from `OperationContractDeclaration`. Fixed and variadic inputs use the declaration's
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

`FunctionInvocation::check_operation_memory_contract` validates the current
single-output compatibility bridge and uses `same_storage` for operation alias
policy. Semantic binding requires that validation before a selected implementation is
returned from binding; missing operation-contract authority is an error.

## 15. Semantic and physical owners

The type system consumes schemas, kinds, validated dimensions, and declared
operation requirements before physical binding. Storage capabilities, Rust
backing classes, logical cell IDs, pointers, and allocator identity do not
participate in inference.

Semantic binding validates descriptors before physical selection and allocation.
RowDVector and DVector expose their invariant axes as Constant(1). Memory planning
owns layouts, capacity, placement, lifetimes, alias and reuse groups, transactions,
budgets, and transfers. The managed runtime realizes those plans; it may reject an
invalid plan but cannot silently substitute a different physical one.

## 16. Non-goals

This compatibility relation adds no second operation declaration, inference,
conversion, placement, allocator, bytecode, canonical encoding, or ABI authority.
Its metadata stays deterministic, non-mutating, and process-local. The type and
memory contract owner suites verify semantic eligibility, exact storage extents,
identity distinctions, operation requirements, and authoritative physical binding.
