# Immutable value semantics

This document defines immutable values, canonical schemas and snapshot encoding.
Runtime storage and publication are governed by the resident activation and
managed-memory contracts.

## Semantic layers

The value system must keep the following layers distinct:

- `KindExpr` is a polymorphic or dependent type expression used during
  inference, checking, and specialization.
- `KindScheme` is a quantified operation or binding signature.
- `Schema` is a resolved family of legal immutable snapshots. It defines
  element, field, column, option, nominal, and keyability rules, but not a
  physical buffer layout.
- `ShapeInstance` is the concrete dimensions and other shape metadata of one
  snapshot.
- `Value` is one immutable semantic snapshot.
- `CellSlotId` is deterministic identity for a logical slot in one
  `ProgramArtifact`.
- `CellId` identifies one `CellSlotId` in one activated runtime instance.
- `StateArena` is mutable, versioned runtime storage owned by a resident
  instance.

The semantic value shape is conceptually:

```rust
pub struct Value {
    pub schema: SchemaId,
    pub data: ValueData,
}
```

This declaration is illustrative. A semantic `Value` must not contain `Ref`, `ValRef`, `MutableReference`, `CellId`,
`CellSlotId`, `InstanceEpoch`, `ProducerId`, `NodeId`, borrow state, a mutable
buffer, reactive dependencies, allocation capacity, or pointer-derived
identity. Runtime storage and execution identity are not semantic payload.

## Kinds, schemas, and reified type values

`KindExpr` describes semantic types, including wildcard, never, inference holes,
nominal and structural types, and reified types. A reference qualifier belongs
to binding and port-access metadata; it is not an immutable snapshot schema.

`KindExpr::Hole` is a compiler inference or unspecified-shape hole. It is not
an immutable value and cannot produce a `Schema` until resolved.
`KindExpr::Never` is uninhabited compiler type state; it is not execution
control for an operation that produced no result.

Kinds remain first-class reified meta-values because current language
evaluation constructs and passes kind values at runtime. Their conceptual
representation is:

```rust
pub enum ReifiedType {
    Kind(KindExpr),
    Schema(SchemaKey),
}

pub enum ValueData {
    // ...
    Type(ReifiedType),
}
```

Every reified type value has the resolved schema `meta/type`. It is immutable,
may be a constant, and contains no runtime storage identity. A reified type
value is not an unresolved compiler-only placeholder.

`KindScheme` is the semantic, quantified representation of generic functions
and operations. Its conceptual shape is:

```rust
pub struct KindScheme {
    pub kind_parameters: Box<[KindParameter]>,
    pub dimension_parameters: Box<[DimensionParameter]>,
    pub inputs: InputKindScheme,
    pub outputs: Box<[KindExpr]>,
    pub constraints: Box<[KindConstraint]>,
}

pub enum InputKindScheme {
    Fixed(Box<[KindExpr]>),
    Variadic {
        prefix: Box<[KindExpr]>,
        repeated: KindExpr,
        min_repetitions: u32,
    },
}

pub struct KindParameter {
    pub ordinal: u32,
    pub upper_bound: Option<KindExpr>,
}

pub enum KindConstraint {
    Equal(KindExpr, KindExpr),
    Convertible(KindExpr, KindExpr),
    Keyable(KindExpr),
    DimensionEqual(DimensionExpr, DimensionExpr),
    DimensionLessEqual(DimensionExpr, DimensionExpr),
}
```

A `KindScheme` may contain semantic input and output kinds, kind variables,
dimension variables, semantic convertibility, keyability, and shape
relationships. It must not contain Rust concrete types, `Ref`, `Value`, matrix
backing representation, Cargo features, ABI information, native feature
closure, or allocator and buffer choices.

Runtime representation and dispatch are a separate contract whose conceptual
target is:

```rust
pub struct RuntimeRepresentationSignature {
    pub inputs: RuntimeRepresentationInputs,
    pub outputs: Box<[ValueRepresentation]>,
}
```

This contract owns runtime arity, exact scalar backing, exact versus flexible
matrix backing, runtime extraction rules, native ABI requirements, and required
Cargo or native features. The current `FunctionRuntimeType`,
`FunctionValueRepresentation`, `RuntimeFunctionInputs`, and
`RuntimeFunctionSignature` describe this runtime contract; they do not become
`KindScheme`.

## Schema identity

The identity types are conceptually:

```rust
pub struct SchemaId(pub u32);
pub struct SchemaKey(pub [u8; 32]);
```

`SchemaId` must be deterministic and meaningful only inside one
`ProgramArtifact`. A bare numeric `SchemaId` must never be interpreted as a
global identity. External and retained references must use either
`(ProgramRevision, SchemaId)` or `SchemaKey` together with its schema table.
`SchemaKey` is content identity for interchange and retained material.

## Shape and dimension lifetime

Dimension lifetime is defined as:

```rust
pub enum DimensionLifetime {
    CompileTime,
    Activation,
    Turn,
}
```

- `CompileTime` dimensions, such as `Matrix<f64, 3, 3>`, are fixed by the
  artifact.
- `Activation` dimensions, such as `exists m, n. Matrix<f64, m, n>`, are fixed
  for ordinary turns after activation.
- `Turn` dimensions, such as `exists k <= n. Vector<f64, k>`, may differ on
  each accepted turn.

Shape metadata and payload belong to the same snapshot and must publish
together. Physical buffering, capacity, stride, nalgebra allocation, and
version selection must not be part of `Schema`, `ShapeInstance`, or `Value`.

## Equality, hashing, and ordering

The general snapshot `Value` must not derive or implement Rust `Eq`, `Hash`, or
`Ord` indiscriminately. The language defines three separate relations.

### Language equality

`LanguageEquality` is the behavior of Mech equality operations. Floating-point
NaN is not language-equal to NaN, positive zero is language-equal to negative
zero, and ordinary finite and infinite values follow IEEE numerical equality.
Aggregate language equality is recursive according to schema.

### Snapshot equality and `ValueHash`

`SnapshotEquality` is exact immutable snapshot content identity. It includes
the `SchemaKey` or artifact-qualified `SchemaId`, `ShapeInstance`, and canonical
payload encoding. Floating-point payloads preserve the exact IEEE bits,
including signed zero and NaN payloads. Snapshot equality may therefore differ
from language equality.

`ValueHash` is defined from the canonical snapshot encoding and must correspond
to `SnapshotEquality`. It must not be implemented as the general Rust `Hash`
trait on `Value`.

### Key equality, hash, and order

Set and map keys use the distinct `KeyEquality`, `KeyHash`, and `KeyOrder`
relations. Only schemas explicitly declared keyable may be keys. For
floating-point keys, negative zero canonicalizes to positive zero and every NaN
encoding canonicalizes to one key NaN. Canonical key NaNs compare equal.
`KeyHash` must match `KeyEquality`, and `KeyOrder` must be total over the
canonical representation. There is no global `Ord` implementation for every
`Value`.

## Canonical aggregate semantics

- Strings are exact UTF-8 bytes. No implicit Unicode normalization occurs.
- Tuple position is semantic.
- Record field order is schema-declared order. Hash-map iteration order is
  never semantic.
- Table column order is schema-declared order. Row order is semantic unless a
  table operation explicitly defines otherwise.
- Matrix and vector shape is part of the snapshot. Canonical encoding follows
  logical index order and must not depend on backing layout, stride, or buffer
  selection.
- Map and set canonical serialization and `ValueHash` use canonical key order
  and must not depend on hash-table iteration.
- Atom and enum identity is nominal and must use a stable schema identity, not
  a process-local pointer or an unqualified numeric tag.

An empty aggregate retains its schema, shape, and element, field, or column
types. An empty `Vector<f64>`, empty `Vector<string>`, empty `Set<Id>`, and empty
record are distinct snapshots. Empty payload must not erase schema.

## Canonical snapshot encoding

The durable canonical encoding is named `MechSnapshotEncodingV1`. This section
and `tests/architecture/value-system/canonical-encoding-v1.json` are normative.

`SchemaKey`, `ValueHash`, and `KeyHash` are exactly 32 bytes and use SHA-256
with distinct domain separators:

```text
SchemaKey = SHA-256(
  b"mech-schema-v1\0" || canonical_schema_bytes
)

ValueHash = SHA-256(
  b"mech-value-v1\0" || SchemaKey || canonical_shape_bytes || canonical_payload_bytes
)

KeyHash = SHA-256(
  b"mech-key-v1\0" || SchemaKey || canonical_shape_bytes || canonical_key_payload_bytes
)
```

`ValueHash` is durable snapshot identity, not the scheduler's optional fast
change-detection hash.
No delimiters are added between the fixed 32-byte `SchemaKey` and the
self-framed shape and payload encodings in either value or key hashes.

The common framing primitives are exact. `U8` is one byte; `U16`, `U32`, and
`U64` are little-endian fixed-width integers. `Bytes` is a `U64` byte length
followed by exactly that many bytes. `Utf8` is `Bytes` containing valid UTF-8,
with no Unicode normalization. `Node` is a `U64` byte length followed by
exactly one encoded node. Every recursive schema, kind, and dimension child is
`Node` framed. Unknown tags, trailing bytes, invalid UTF-8, noncanonical
booleans, invalid widths, duplicate names, and inconsistent lengths are
invalid.

Nominal identity is independent of local numeric IDs and filesystems. A
`CanonicalNominalPath` is a non-empty ordered sequence of exact UTF-8 path
segments. Empty, `.`, `..`, and NUL-containing segments are invalid. No
Unicode normalization is performed. Atom and enum identity is:

```text
NominalKey = SHA-256(
  b"mech-nominal-v1\0"
  || U8 nominal-kind-tag
  || U32 segment-count
  || each segment as Utf8
)

0x01 atom
0x02 enum
```

Artifact compilation derives this path from the resolved defining declaration:
the first segment is the exact UTF-8 package name declared by the resolved
package manifest; zero or more following segments are the defining module's
canonical namespace relative to that package root; and the final segment is
the exact declaration name. Package versions and dependency-source identities
are excluded so a compatible package release does not silently change durable
keys. Local crate/dependency aliases and re-export paths are excluded: every
re-export resolves back to the defining declaration before path construction.
Only nominal declarations included in one `ProgramArtifact` participate in
collision detection. Declarations are grouped by complete
`CanonicalNominalPath`; the same full path from two distinct Cargo package IDs
is `AmbiguousNominalDeclarationV1`, while different full paths from same-name
packages are legal. Duplicate dependency names elsewhere in `Cargo.lock` are
irrelevant. A Cargo package ID is only an internal collision discriminator and
is never encoded in `NominalKey`. A filesystem path or process-local numeric
hash is never a nominal identity.

The checked-in golden-vector document is independently frozen by the canonical
JSON SHA-256 `0be9531a4514ef359bedc3172f6ea327c28bcaab0fa493d9725d430799343a9f`,
computed over its UTF-8 JSON with object keys sorted and separators `,` and `:`
with no insignificant whitespace. The checker first verifies this digest, then
reproduces every expected result. Changing an encoder and regenerating its
inputs and outputs together therefore remains contract drift.

Schema and closed-kind roots share one dimension-parameter environment. Each
parameter encodes `U8 lifetime`, `Node lower-bound`, `U8 upper-present`, and a
`Node upper-bound` only when present. Activation and turn lifetime tags are
`0x01` and `0x02`; compile-time dimensions are constants, not parameters.
Explicit parameters keep source declaration order, followed by inferred
parameters in first pre-order occurrence. Names are not encoded, unused
parameters are removed, references are canonical zero-based `U32` ordinals,
and the retained environment includes the transitive closure of parameters
referenced by reachable bounds. A bound may reference only an earlier retained
parameter. Cycles fail with `CyclicDimensionParameterBoundsV1`; forward
references fail with `ForwardDimensionParameterReferenceV1`.

Dimension expressions have the following exact tags:

| Tag | Expression | Fields |
|---:|---|---|
| `0x01` | Constant | `U64 value` |
| `0x02` | Parameter | `U32 ordinal` |
| `0x03` | Add | `U32 count`, then operand `Node`s |
| `0x04` | Multiply | `U32 count`, then operand `Node`s |
| `0x05` | Min | `U32 count`, then operand `Node`s |
| `0x06` | Max | `U32 count`, then operand `Node`s |

Add and multiply flatten the same tag, recursively canonicalize, fold
constants, remove their identities, and lexicographically sort encoded
operands. A zero factor makes the whole multiplication `Constant(0)`. Min and
max flatten, recursively canonicalize, deduplicate, and lexicographically sort
encoded operands. A one-operand aggregate becomes that operand. Empty add and
multiply become `Constant(0)` and `Constant(1)`; empty min and max are invalid.
Constant folding uses checked `u64` arithmetic; any nonzero add or multiply
whose result exceeds `u64::MAX` is invalid rather than wrapping or panicking.
Overflow fails with `DimensionOverflowV1`, an out-of-range parameter ordinal
fails with `UnknownDimensionParameterV1`, and empty min or max fails with
`EmptyMinMaxV1`.
Subtraction, division, host `usize`, and opaque callbacks are not V1.

Canonical schema bytes are:

```text
U8 schema-encoding-version = 0x01
U32 dimension-parameter-count
dimension parameters
Node root-schema-body
```

Schema bodies have the following exact tags and fields:

| Tag | Schema body | Fields |
|---:|---|---|
| `0x01` | Bool | none |
| `0x02` | UnsignedInteger | `U16 bit-width` |
| `0x03` | SignedInteger | `U16 bit-width` |
| `0x04` | FloatingPoint | `U16 bit-width` |
| `0x05` | Complex | `U16 component-bit-width` |
| `0x06` | Rational | `U16 numerator-width`, `U16 denominator-width` |
| `0x07` | String | none |
| `0x08` | Id | none; semantic width is `u64` |
| `0x09` | Index | none; semantic width is `u64` |
| `0x0a` | Atom | 32-byte `NominalKey` |
| `0x0b` | Enum | 32-byte `NominalKey`, `U32 variant-count`, variants |
| `0x0c` | Option | `Node element-schema-body` |
| `0x0d` | Tuple | `U32 arity`, then child `Node`s |
| `0x0e` | Record | `U32 field-count`, then fields |
| `0x0f` | Matrix | element `Node`, `U32 rank`, dimension `Node`s |
| `0x10` | Table | `U32 column-count`, columns, row-count `Node` |
| `0x11` | Set | element `Node`, cardinality `Node` |
| `0x12` | Map | key `Node`, value `Node`, cardinality `Node` |
| `0x13` | ReifiedType | none |

Unsigned and signed widths are 8, 16, 32, 64, or 128; floating-point and
complex-component widths are 32 or 64; rational V1 is 64/64. Enum variants
encode an exact local `Utf8` name, `U8 payload-present`, and a payload schema
`Node` only when present. Declaration order determines the `U32` ordinal.
Record fields and table columns encode `Utf8 name` and a child schema `Node`.
Names are unique and declaration order is semantic. Matrix dimensions, table
row count, set cardinality, and map cardinality are dimension-expression
`Node`s in the root parameter environment.

Schema trees must be acyclic and are encoded by value; shared implementation
pointers are not semantic. Recursion fails with
`RecursiveSchemaUnsupportedV1`. Wildcard, never, holes, references, execution
no-result, and uninitialized storage do not produce schemas. Keyability is
derived from the schema body, never encoded as an independently mutable flag.
Bool, unsigned integer, signed integer, floating point, rational, string, ID,
index, and atom are always keyable. Enum, option, tuple, and record are keyable
only when their children are. Complex, matrix, table, set, map, and reified
type are not keyable in V1.

Canonical shape bytes are:

```text
U8 shape-encoding-version = 0x01
U32 resolved-parameter-count
U64 resolved values in schema parameter order
```

`shape_values` are supplied in canonical retained schema-parameter order. The
count must equal that retained parameter count. Bounds are evaluated in order:
each value must satisfy its lower and optional upper bound using only already
resolved earlier parameters. Every schema dimension and cardinality expression
then evaluates with checked `u64` arithmetic. A count mismatch fails with
`ShapeParameterCountMismatchV1`; a lower or upper violation fails with
`ShapeBoundViolationV1`. Compile-time constants already occur in `SchemaKey`
and are not repeated.

A canonical closed `KindExpr` has the root framing `U8 version = 0x01`, `U32`
dimension-parameter count, the parameter frames, and a `Node` containing the
root kind. Its exact tags are:

| Tag | `KindExpr` | Tag | `KindExpr` |
|---:|---|---:|---|
| `0x01` | Wildcard | `0x0a` | Option |
| `0x02` | Never | `0x0b` | Tuple |
| `0x03` | Hole | `0x0c` | Record |
| `0x04` | Named plus canonical nominal path | `0x0d` | Table |
| `0x05` | Id | `0x0e` | Set |
| `0x06` | Index | `0x0f` | Map |
| `0x07` | Atom plus `NominalKey` | `0x10` | Reference |
| `0x08` | Enum plus `NominalKey` | `0x11` | TypeOf |
| `0x09` | Matrix |  |  |

Matrix and aggregate framing mirrors schema framing. A standalone reified
`KindExpr` is closed: kind parameters belong to `KindScheme`, not as free
variables in a reified value. A `ValueData::Type(ReifiedType)` payload is
either `0x01` plus a `Node` containing canonical closed-kind bytes, or `0x02`
plus a 32-byte `SchemaKey`.

Payload encoding is schema-directed and does not add child `Node` framing.
Booleans are one `U8`, exactly `0` or `1`. Unsigned integers use their declared
fixed width and signed integers use two's-complement at their declared fixed
width. Both are little-endian. Floating-point payloads preserve exact IEEE bits
in little-endian order. Complex payloads encode the real floating component
followed by the imaginary component. `Rational64` encodes a signed little-endian
`i64` numerator followed by a positive little-endian `u64` denominator. Strings
use `Bytes`; ID and index payloads are `U64`; atom payload is empty because its
`NominalKey` is in the schema.

An absent option is `U8 0`; a present option is `U8 1` followed by the element
payload. An enum is its `U32` variant ordinal followed by the declared variant
payload when present. Tuples encode child payloads in positional order. Records
encode child payloads in schema field order without repeating names. Matrices
encode elements in logical lexicographic index order with the last logical
dimension varying fastest and do not repeat the shape. Tables encode every row
of each schema column in column order and semantic row order; the row count is
not repeated. Physical stride, buffer selection, and hash-table iteration never
affect encoding.

A set encodes its `U64` element count followed by canonical key payloads sorted
by `KeyOrder`. A map encodes its `U64` entry count followed by each canonical
key payload and value snapshot payload, with entries sorted by `KeyOrder`.
Before storage or encoding, every set element or map key is converted to its
canonical key payload. Equality is detected with `KeyEquality`; a duplicate
fails with `DuplicateCanonicalKeyV1`. The canonical payload, never the original
noncanonical float bits, is encoded.

Before encoding or key comparison, canonical V1 validates the complete schema,
resolved shape, and payload. Integer widths are exactly 8, 16, 32, 64, or 128;
floating and complex component widths are 32 or 64; rational width is exactly
64/64. Record fields, table columns, and enum variants have unique names, and
schema object graphs are acyclic. Tuples have exact arity; records and tables
have exactly their schema-declared names; options accept only `None`, exactly
`{"present": false}`, or exactly `{"present": true, "value": ...}`; enum
ordinals are integers in range and carry a payload field iff the selected
variant declares one. Matrix element count is the checked product of its
dimensions. Every table column has the resolved row count. Set and map counts
equal resolved cardinality, and every map entry has exactly key and value.
`compare_keys` performs these same aggregate checks before lexicographic
comparison.

The exact additional V1 failures are
`ShapeParameterCountMismatchV1`, `ShapeBoundViolationV1`,
`AggregateArityMismatchV1`, `AggregateFieldMismatchV1`,
`PayloadCardinalityMismatchV1`, `EnumOrdinalOutOfRangeV1`,
`EnumPayloadMismatchV1`, `MapEntryArityMismatchV1`,
`DuplicateSchemaNameV1`, and `InvalidSchemaWidthV1`. Existing failures
`DimensionOverflowV1`, `UnknownDimensionParameterV1`, `EmptyMinMaxV1`,
`CyclicDimensionParameterBoundsV1`,
`ForwardDimensionParameterReferenceV1`, `RecursiveSchemaUnsupportedV1`,
`DuplicateCanonicalKeyV1`, `SchemaNotKeyableV1`, and
`NonCanonicalRationalV1` remain unchanged.

Canonical `KeyOrder` is numerical for integers, `false < true` for booleans,
and lexicographic UTF-8 byte order for strings. F32 and F64 negative zero
canonicalize to `0x00000000` and `0x0000000000000000`. Every F32 NaN
canonicalizes to `0x7fc00000`; every F64 NaN canonicalizes to
`0x7ff8000000000000`. All other float bits are retained, and IEEE 754
`totalOrder` is applied after normalization. Snapshot payloads do not perform
this normalization.

A canonical `Rational64` has a signed `i64` numerator, a positive `u64`
denominator, greatest common divisor one, and represents zero only as `0/1`.
Other representations fail with `NonCanonicalRationalV1`. Rational order uses
checked-exact `i128` cross products: `n1/d1 < n2/d2` exactly when
`i128(n1) * i128(d2) < i128(n2) * i128(d1)`. Tuples and records compare
lexicographically; absent options sort before present ones; nominal values
compare `SchemaKey`, variant ordinal, then payload.

`canonical-encoding-v1-vectors.json` commits eleven literal value vectors,
five key vectors, and seven recursive dimension-normalization vectors. Every
value vector includes exact schema, schema-key, shape, payload, and value-hash
hexadecimal bytes. A separate test-only reference encoder must reproduce those
constants and declared errors; tests never generate their own expected bytes.

## Ownership and borrowing

A semantic `Value` owns immutable data. Inline scalars, inline small fixed
aggregates, owned immutable allocations, and shared immutable allocations for
large payloads are permitted. The representation must not require `Arc` for
every value.

A general `Value<'a>` borrowing arena storage is prohibited. Borrowed pointers
must not escape a turn. `ValueData` must not contain mutable buffers, and an
arena-backed slice must not escape without an explicit observer pin that owns a
version lifetime. Typed kernels read `StateArena` directly and must not
materialize `Value` on every access.

Snapshot materialization may occur at host boundaries, serialization,
language-level composite construction, diagnostics, debugger snapshots,
ledger snapshot or delta material, and general dynamic execution.

## Constants and cells

The target distinction is conceptually:

```rust
pub struct ConstantId(pub u32);

pub enum ReadSource {
    Constant(ConstantId),
    Cell(CellId),
}
```

Constants live in `ConstantStore`. A constant has no epoch, write barrier,
rollback entry, journal entry, or mutable producer. A literal must not receive
a mutable cell merely because a legacy machine interface expects `Ref<T>`.

## Construction and storage boundaries

Snapshot modules must not import mutable references or runtime cell identities.
Schema modules must not depend on runtime or engine crates. Canonical values are
built through validated constructors; engine artifacts accept immutable values.
The retired value enum and conversion adapters have no compatibility alias.
`scripts/check-no-retired-value-system.py` enforces their absence.

An unresolved empty expression, an absent option, an operation with no result,
and an uninitialized runtime slot are distinct concepts. Only an absent option
under a resolved option schema becomes `ValueData::Option(None)`. Empty
aggregates retain their resolved element schema and concrete shape. Matrices
are homogeneous; heterogeneous data uses tuples, records, or tables.

## Performance and publication

Resident performance changes require fresh controlled measurements under
benchmark ownership. Historical timing ratios are advisory. Correctness,
steady-state allocation behavior, candidate isolation, publication, effects,
replay, and bounded history remain required behavioral contracts.

For complete full-write resident outputs, publication uses one release store,
zero candidate seed bytes and zero published-buffer copy bytes. The prepared
post-publication append is infallible. Benchmark checks retain their sample
protocols, exact probe identities, isolated comparison processes and Cargo
targets, and numerical correctness checks.
