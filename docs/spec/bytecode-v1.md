# Mech bytecode v1

Bytecode v1 is the first supported `.mecb` format. Files emitted before this
specification was frozen are unsupported proposals.

All integers are little-endian. Lengths and offsets are byte counts. Readers
reject unknown tags, nonzero reserved fields, noncanonical encodings, integer
overflow, out-of-range references, and trailing bytes.

## Header and ABI

The file starts with this fixed 64-byte header:

| Offset | Width | Field | Required value or meaning |
| ---: | ---: | --- | --- |
| 0 | 4 | `magic` | ASCII `MECH` |
| 4 | 2 | `version` | `1` |
| 6 | 2 | `header_size` | `64` |
| 8 | 2 | `mech_major` | `0` |
| 10 | 2 | `mech_minor` | `3` |
| 12 | 2 | `mech_patch` | `5` |
| 14 | 2 | `flags` | `0` |
| 16 | 4 | `register_count` | Number of program registers |
| 20 | 4 | `instruction_count` | Number of instructions |
| 24 | 2 | `section_count` | `7` |
| 26 | 2 | `reserved0` | `0` |
| 28 | 8 | `section_table_offset` | `64` |
| 36 | 8 | `file_len` | Exact file length, including checksum |
| 44 | 8 | `checksum_offset` | Offset of the final four-byte checksum |
| 52 | 12 | `reserved` | All zero |

The ABI fields come from `MECH_LANGUAGE_RUNTIME_ABI_VERSION = (0, 3, 5)`.
This language/runtime ABI is authoritative and intentionally independent of
crate and distribution package versions. A reader must reject any different
ABI tuple.

## Section directory

Seven 32-byte directory entries begin at byte 64, so content starts at byte
288. Each entry is:

| Offset | Width | Field |
| ---: | ---: | --- |
| 0 | 2 | Section kind |
| 2 | 2 | Flags, always `0` |
| 4 | 4 | Item count |
| 8 | 8 | Content offset |
| 16 | 8 | Content length |
| 24 | 8 | Reserved, always `0` |

The entries occur exactly once and in this order:

| ID | Section | Item-count meaning |
| ---: | --- | --- |
| 1 | Types | Runtime-type entries |
| 2 | Constant table | Constant entries |
| 3 | Constant blob | `0` |
| 4 | Symbols | Symbol entries |
| 5 | Instructions | Instructions |
| 6 | Dictionary | Dictionary entries |
| 7 | Application requirements | Requirements |

Every section offset is aligned to eight bytes. Sections are ordered,
non-overlapping, and contained before the checksum. Inter-section and
pre-checksum padding bytes are zero.

## Runtime types

A type entry is `tag u16`, `flags u16`, `payload_len u32`, followed by its
payload. Flags are zero. Child types are `u32` indices into the same table;
the table is topological, so every child precedes its parent.

| Tag | Type | Payload |
| ---: | --- | --- |
| 1–5 | `U8`, `U16`, `U32`, `U64`, `U128` | Empty |
| 6–10 | `I8`, `I16`, `I32`, `I64`, `I128` | Empty |
| 11–14 | `F32`, `F64`, `C64`, `R64` | Empty |
| 15–21 | `String`, `Bool`, `Id`, `Index`, `Empty`, `Any`, `None` | Empty |
| 22 | `Matrix` | Element type `u32`, storage `u8`, rows `u32`, columns `u32` |
| 23 | `Enum` | ID `u64`, name length `u32`, UTF-8 name |
| 24 | `Record` | Count `u32`, then name length/name/type ID for each field |
| 25 | `Map` | Key type ID `u32`, value type ID `u32` |
| 26 | `Atom` | ID `u64`, name length `u32`, UTF-8 name |
| 27 | `Table` | Column count `u32`, name length/name/type ID per column, primary-key index `u32` |
| 28 | `Tuple` | Count `u32`, then child type IDs |
| 29 | `Reference` | Child type ID `u32` |
| 30 | `Set` | Element type ID `u32`, limit-present `u8`, optional maximum `u32` |
| 31 | `Option` | Child type ID `u32` |
| 32 | `Kind` | Canonical recursive semantic-kind encoding |

Named type and schema names are nonempty and must match their stable hashes.
Record fields and table columns are ordered and unique. A nonempty table's
primary-key index must be in range. Runtime-type recursion is limited to 256.

### Matrix storage

The matrix storage byte determines the exact runtime representation; readers
must not infer storage only from dimensions.

| Tag | Storage | Required dimensions |
| ---: | --- | --- |
| 1–4 | `Matrix1`, `Matrix2`, `Matrix3`, `Matrix4` | 1×1, 2×2, 3×3, 4×4 |
| 5 | `Matrix2x3` | 2×3 |
| 6 | `Matrix3x2` | 3×2 |
| 7–9 | `RowVector2`, `RowVector3`, `RowVector4` | 1×2, 1×3, 1×4 |
| 10–12 | `Vector2`, `Vector3`, `Vector4` | 2×1, 3×1, 4×1 |
| 13 | `RowVectorD` | 1×N, N > 0 |
| 14 | `VectorD` | N×1, N > 0 |
| 15 | `MatrixD` | M×N, M and N > 0 |

## Constants

The constant table has one 24-byte entry per constant:

| Offset | Width | Field | Rule |
| ---: | ---: | --- | --- |
| 0 | 4 | Type ID | In range |
| 4 | 1 | Encoding | `1` (payload in the constant blob) |
| 5 | 1 | Alignment | One of 1, 2, 4, 8, 16 |
| 6 | 2 | Flags | `0` |
| 8 | 8 | Blob offset | Aligned and ordered |
| 16 | 8 | Payload length | Contained in the blob |

Entries do not overlap. Gaps and trailing blob bytes are zero. Recursive
constant nesting is limited to 256 and uses checked counts and lengths.

### Scalars

| Type | Canonical payload |
| --- | --- |
| `U8`, `I8` | One byte |
| `U16`, `I16` | Two-byte little-endian integer |
| `U32`, `I32` | Four-byte little-endian integer |
| `U64`, `I64` | Eight-byte little-endian integer |
| `U128`, `I128` | Sixteen-byte little-endian integer |
| `F32`, `F64` | IEEE bits as little-endian `u32` or `u64` |
| `C64` | Real `F64` bits, then imaginary `F64` bits |
| `R64` | Numerator `i64`, then denominator `i64` |
| `String` | Raw UTF-8 bytes |
| `Bool` | Exactly `00` or `01` |
| `Id` | `u64` |
| `Index` | `u64`, required to fit the executing platform's `usize` |
| `Empty`, `Any`, `None`, `Atom`, `Kind` | Zero bytes |

Floating-point bits, including NaN payloads, are preserved. `R64` has a
positive nonzero denominator and is reduced; readers reject a mathematically
equivalent noncanonical representation. `Any` and `None` represent only their
typed empty values. An atom's ID and name live in its `RuntimeType` and must
match; its constant payload is empty.

### Matrices

A matrix payload is rows `u32`, columns `u32`, then elements in canonical
row-major order. Supported element types are `Index`, `Bool`, every signed and
unsigned integer width, `F32`, `F64`, `C64`, `R64`, and `String`. Fixed-width
elements use their scalar encoding consecutively; each string element is a
`u32` byte length followed by UTF-8. The declared storage tag, dimensions,
element count, and payload length must all agree.

### Composite values

Every nested child is framed as `payload_len u32` followed by its canonical
payload.

- A tuple is `element_count u32` followed by children in declared order.
- A record is `field_count u32` followed by field values in schema order;
  names are not repeated in the constant.
- A map is `entry_count u32` followed by key and value children. Entries sort
  by `(key payload, value payload)`; key payloads are strictly increasing, so
  duplicate canonical keys are rejected.
- A set is `element_count u32` followed by strictly increasing canonical
  element payloads. Duplicates and counts above the optional type limit are
  rejected.
- A table is row count `u32`, column count `u32`, then length-prefixed cells
  in row-major order using the declared column schema. Decoding constructs
  dynamic-vector columns.
- A reference is one child. It decodes to a new mutable reference containing
  a deep snapshot. Separate occurrences do not preserve aliases; cyclic
  graphs are rejected, while shared acyclic references may be repeated.
- An option is `00` for absent or `01` followed by one framed child for
  present. Only canonical typed option wrappers are accepted. A present option
  retains the concrete runtime representation of its child. An absent option
  uses its declared semantic kind because there is no child representation to
  preserve.
- A kind has no constant bytes; its complete semantic kind is carried by the
  `RuntimeType::Kind` payload.

### Enum payloads and inline types

An enum constant is:

```text
variant_count u32
repeated in ascending variant-ID order:
  variant_id          u64
  variant_name_length u32
  variant_name        UTF-8 bytes
  has_payload         u8 (exactly 0 or 1)
  when has_payload == 1:
    inline_type_length u32
    inline_type        canonical RuntimeType key
    value_length       u32
    value_payload      canonical constant bytes
```

The enum and variant names must hash to their IDs, and variant IDs are unique.
The inline type is the same ID-independent canonical recursive `RuntimeType`
key used to finalize the main type table, rather than an index into that table.

## Symbols and dictionary

A symbol entry is ID `u64`, register `u32`, flags `u32`. Entries are sorted by
unique ID, the register is in range, and the only flag bit is bit 0 for a
mutable symbol. A dictionary entry is ID `u64`, name length `u32`, and UTF-8
name. Dictionary entries are sorted by unique ID; names are nonempty and hash
to their IDs. Every symbol and every named runtime value must agree with the
dictionary.

## Instructions

Each instruction starts with a one-byte opcode; remaining fields are
little-endian. Argument arrays are a `u32` count followed by `u32` registers.

| Opcode | Instruction | Fields after opcode |
| ---: | --- | --- |
| `01` | `ConstLoad` | Destination `u32`, constant index `u32` |
| `10` | `RuntimeNullary` | Function ID `u64`, destination `u32` |
| `11` | `RuntimeUnary` | Function ID, destination, source |
| `12` | `RuntimeBinary` | Function ID, destination, left, right |
| `13` | `RuntimeTernary` | Function ID, destination, A, B, C |
| `14` | `RuntimeQuaternary` | Function ID, destination, A, B, C, D |
| `15` | `RuntimeVariadic` | Function ID, destination, argument count and registers |
| `20` | `HostCall` | Requirement index, destination, argument count and registers |
| `21` | `ResourceRead` | Requirement index, destination |
| `22` | `ResourceWrite` | Requirement index, destination, source |
| `23` | `ResourceSend` | Requirement index, destination, source |
| `FF` | `Return` | Source `u32` |

All registers and indices are in range, and runtime-function IDs are nonzero.
There is exactly one `Return`; it is the final instruction.

## Application requirements

Each requirement has a fixed 16-byte prefix followed by four UTF-8 strings:

```text
kind u8, intent u8, delivery u8, flags u8
operation_len u16, context_len u16
primary_len u32, secondary_len u32
operation, context, primary, secondary bytes
```

Kind 1 is a host function. Intent and delivery are zero, operation, context,
and secondary are empty, and primary is the nonempty exact function name.

Kind 2 is a resource. Intent is 1 `Read`, 2 `Assign`, or 3 `Send`; delivery is
0 `Snapshot` or 1 `Live`. Operation, context, and primary base URI are
nonempty; secondary is the path. URI, operation, context, delivery, and path
must form a canonical valid execution request. Requirement flags are zero.

## Checksum and validation limits

The final four bytes are the little-endian CRC32/IEEE of every byte in
`[0, checksum_offset)`. `checksum_offset + 4 == file_len == actual file len`.

Default read limits are:

| Resource | Limit |
| --- | ---: |
| File bytes | 67,108,864 |
| Registers | 1,000,000 |
| Instructions | 1,000,000 |
| Runtime types | 100,000 |
| Constants | 1,000,000 |
| Symbols | 1,000,000 |
| Dictionary entries | 1,000,000 |
| Dictionary bytes | 16,777,216 |
| Application requirements | 10,000 |
| Variadic or host-call arguments | 65,536 |
| Type recursion | 256 |
| Constant recursion | 256 |

## Determinism

Writers finalize child types before parents and sort equal-depth types by
their canonical keys. Constants, symbols, dictionary entries, requirements,
sections, maps, sets, and enum variants use their specified canonical order.
All reserved fields and alignment bytes are zero. Equivalent programs
therefore produce identical bytes.

The corpus under `tests/architecture/bytecode-v1/` records source or
construction origin, bytecode SHA-256, decoded structure, native-plan digest,
graph, and expected output. `scripts/check-bytecode-v1-format.py` checks it
independently of the Rust reader.

## Unsupported constants

Bytecode v1 deliberately has no fallback serializer. It rejects
`Value::MatrixValue`, `Value::IndexAll`, user functions, native closures,
dynamic-module functions, dynamic-library-backed values, opaque host objects,
cyclic references, arbitrary alias-preserving graphs, general `Value::Typed`
wrappers other than `Option`, and noncanonical nonempty `EmptyKind` values.
Unsupported values produce `BytecodeConstantUnsupported`; excessive constant
nesting produces `BytecodeConstantDepthExceeded`.

## Version policy

Version 1 is the only supported bytecode format. There is no pre-v1 reader,
translation branch, or compatibility promise. Any incompatible wire-format
change requires a new bytecode version. A language/runtime ABI change is a
separate explicit decision and must update the header authority.
