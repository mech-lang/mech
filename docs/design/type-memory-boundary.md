# Type-memory boundary

## 1. Status and R2A scope

R2A defines a read-only semantic projection from a finalized `Schema`, and from
that schema plus a validated `ShapeInstance`, into memory-facing contracts.
It changes no runtime binding, storage, target, or execution behavior.

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

The R2A contract types deliberately implement no serialization traits and have
no wire format. Canonical schema bytes remain unchanged and authoritative.
Derived contracts must be recomputed from the schema rather than persisted as
another compatibility surface.

## 13. R2B handoff

R2B may describe existing storage capabilities and separate logical identity
from physical storage identity. It must consume this semantic projection
without moving schema authority into runtime representations.

## 14. R2C handoff

R2C may derive memory requirements for operation ports and compare those
requirements with storage capabilities. It must not reinterpret schema
identity or mutate the R2A contract.

## 15. R2D closure

R2D owns shadow conformance, architecture enforcement, CI integration, and R2
roadmap closure. R2A alone does not mark R2 complete.

## 16. R3 handoff

R3 may use the boundary while tightening inference and conversion semantics.
It must treat schema as the type authority and the memory contract as derived
metadata.

## 17. R4 handoff

R4 owns retirement of transitional runtime representations and any binding
cutover. R2A neither changes nor endorses a runtime representation.

## 18. R5/R6 handoff

R5 owns deterministic physical layout and resource planning, including sizes,
alignment, strides, placement, and allocation plans. R6 owns allocators,
backing, reuse, reclamation, and resource enforcement. R2A contains none of
those mechanisms.

## 19. Non-goals

R2A does not add storage capability descriptors, operation-port memory
requirements, compatibility checking, physical layout, allocation, inference,
conversion, target availability, function binding, `ValueCell` behavior,
Resident or GPU behavior, bytecode, canonical encoding, ABI changes, or
package-version changes.

## 20. R2A acceptance criteria

R2A is complete when every current `SchemaBody` variant derives a total
contract, extent evolution propagates recursively, supplied shapes are
revalidated against the target schema, checked resolution rejects invalid
dimensions, schema identity and canonical bytes are invariant, the new public
types are not deserializable, and no runtime or wire-format behavior changes.
