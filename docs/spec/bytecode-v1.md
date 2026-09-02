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
| 24 | 2 | `section_count` | `18` |
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

Eighteen 32-byte directory entries begin at byte 64, so content starts at
byte 640. Each entry is:

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
| 8 | Artifact schemas | `1` when the artifact is present, otherwise `0` |
| 9 | Artifact constants | `1` when the artifact is present, otherwise `0` |
| 10 | Artifact inputs | `1` when the artifact is present, otherwise `0` |
| 11 | Artifact slots | `1` when the artifact is present, otherwise `0` |
| 12 | Artifact producers | `1` when the artifact is present, otherwise `0` |
| 13 | Artifact nodes | `1` when the artifact is present, otherwise `0` |
| 14 | Artifact bindings | `1` when the artifact is present, otherwise `0` |
| 15 | Artifact outputs | `1` when the artifact is present, otherwise `0` |
| 16 | Artifact integrity constraints | `1` when the artifact is present, otherwise `0` |
| 17 | Artifact operations | `1` when the artifact is present, otherwise `0` |
| 18 | Artifact operation contracts | `1` when the artifact is present, otherwise `0` |

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
Record fields and table columns are ordered and unique. Table primary-key
metadata must be zero because the frozen runtime value cannot preserve a
different key yet. Runtime-type recursion is limited to 256.
Every type-table row must be reachable from a constant's root type through
raw child type IDs. An otherwise valid but unused type row is noncanonical.

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

Entries do not overlap. Gaps and trailing blob bytes are zero. Rows are unique
by canonical runtime type and payload; two IDs must not name the same constant.
Constant IDs are allocated in first-reference order while scanning instructions
(`ConstLoad` values and `CompositePack` templates), beginning at zero. Recursive
constant nesting is limited to 256 and uses checked counts and lengths. Every
constant-table row must be referenced; unreferenced rows are noncanonical.

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
  dynamic-vector columns. Rows and total cells are each limited to 1,000,000,
  and the declared cell count must fit the remaining four-byte child frames
  before any allocation or row iteration begins.
- A reference is one child. It decodes to a new mutable reference containing
  a deep snapshot. Separate occurrences do not preserve aliases; cyclic
  graphs are rejected, while shared acyclic references may be repeated.
- An option is `00` for absent or `01` followed by one framed child for
  present. Only canonical typed option wrappers are accepted. A present option
  retains the concrete runtime representation of its child. An absent option
  uses its declared semantic kind because there is no child representation to
  preserve.
- During annotated source compilation, a bare `Value::Empty` denotes an
  absent option only when its declared schema is `Option<T>`. Composite
  children are collected before their common schema is finalized, so option
  representation selection is independent of tuple, record, table, or matrix
  element iteration order.
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
to their IDs. Symbols and dictionary entries form an exact bijection: every
symbol has its dictionary name and every dictionary row names a symbol.

## Instructions

Each instruction starts with a one-byte opcode; remaining fields are
little-endian. Argument arrays are a `u32` count followed by `u32` registers.

| Opcode | Instruction | Fields after opcode |
| ---: | --- | --- |
| `01` | `ConstLoad` | Destination `u32`, constant index `u32` |
| `02` | `CompositePack` | Destination `u32`, template constant index `u32`, child count `u32`, child registers (`u32` each) |
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

`CompositePack` templates are canonical composite constants. Child registers
are ordered by the template schema, and their runtime kinds must match the
schema's effective child kinds exactly. A `Reference` template child prescribes
its recursively dereferenced kind because bytecode registers carry the stable
referenced cell. The destination receives a reconstructed composite that
retains the child registers as its reactive dependencies.

All registers and indices are in range, and runtime-function IDs are nonzero.
There is exactly one `Return`; it is the final instruction.

### Register initialization

Bytecode validation tracks static register initialization.

The following instructions define a previously uninitialized destination:

- `ConstLoad`
- `CompositePack`
- `ResourceRead`

Each such destination must be uninitialized before that instruction and is
initialized afterward. A register may have only one static defining
instruction.

`ResourceRead` is a defining instruction because its first value is supplied
by the external resource provider. Its destination cell exists before
execution with an `Empty` runtime payload, but no program-authored payload is
serialized for that register.

Runtime function instructions, `HostCall`, `ResourceWrite`, and `ResourceSend`
operate on already initialized destination cells and therefore require their
destinations to have an earlier static definition.

All input/source operands and `Return` sources must be initialized before use.

Each register owns one stable outer value cell for the program's lifetime.
Constant loads and external instructions update the value behind that cell;
symbols, downstream instructions, rollback checkpoints, and `Return` all
observe the same cell identity.

Static bytecode initialization is distinct from allocation of the outer cell.
The interpreter allocates register cells before instruction execution.
`ResourceRead` establishes the first concrete payload of that pre-existing
cell.

### Runtime factory contracts

Every trusted runtime-factory entry has an explicit argument contract.
Before native planning or execution, that contract validates instruction
arity, each value's exact runtime representation, matrix dimensions,
cross-argument shape relations, and the output/input alias policy. A valid
CRC does not bypass these checks: malicious bytecode with mismatched dynamic
shapes, invalid matrix products or solves, or forbidden output aliases is
rejected before a factory can mutate program state.

Core bytecode validation and native planning share this one instruction
contract traversal. Native planning supplies a trusted external-contract
resolver for host calls and resource operations; it does not replay the
instruction stream in a second interpreter.

`ResourceRead` has no destination seed. During contract planning, a trusted
external contract resolver supplies a concrete representative of the
provider-owned first output representation. That representative exists only
for validation: it is not serialized, is not a constant, and does not
contribute to program identity. The default structural resolver fails closed
for `ResourceRead` because it has no provider output representation to supply.

Runtime contract planning treats the resolver result as the destination's
value when validating downstream instruction contracts. Runtime execution
independently obtains the actual first value from the provider.

The execution-service boundary makes those two provider interactions
explicit. Contract planning obtains a detached, non-consuming representative
that establishes the expected runtime representation and shape; it must not
advance or inspect the provider's actual observation stream. Execution later
performs the resource read exactly once. The representative is transient
engine-local planning evidence: it is not serialized, is not a program
constant, does not contribute to program identity, and need not contain the
same payload as the actual first value.

During decoded-program installation, the representative temporarily
populates the existing stable destination cell so downstream factories can
bind to the correct representation. The `ResourceRead` node is installed
before its dependents. Initial plan execution replaces or stable-updates the
representative with the one actual provider value before those dependents
run, so successful execution never exposes the representative as a program
observation or live-resource event.

Factories that intentionally update an input register declare that alias
policy explicitly. Other matrix outputs must not alias any input. Dynamic
shape-changing functions keep a stable output reference and replace the value
behind it transactionally. In particular, `NChooseKMatrix` always produces a
`DMatrix`, including when its row and column counts change reactively.

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
Every application-requirement row must be referenced by at least one host or
resource instruction, or by an artifact node when semantic artifact sections
are present. Unreferenced requirement rows are noncanonical.

The same canonical table is authoritative for the semantic program artifact.
Artifact nodes refer to its dense rows directly; neither the operation path
nor a provider-specific payload is used to reconstruct an external request.
The table and each node's optional requirement ID contribute to
`ProgramRevision`.

## Semantic program artifact

Sections 8 through 18 carry the finalized `ProgramArtifact` produced by the
ordinary source compiler. They are a direct part of bytecode v1, not a legacy
adapter and not a second bytecode version. Constructed runtime-only programs
may leave all eleven sections absent. Source compiler output includes all eleven;
partial presence is invalid.

Sections 8 through 17 are compact UTF-8 JSON arrays with no insignificant
whitespace; section 18 is the canonical binary contract table described
below. The JSON arrays use the field order below, decimal JSON integers, JSON
strings, `null` for an absent optional value, and Serde's externally tagged
form for sum types. Decoders first enforce the raw section and aggregate byte
limits, count top-level JSON array elements without constructing them, and
only then allocate and decode typed values.

| Section | Array element |
| --- | --- |
| Artifact schemas | Canonical C0 `SchemaDraft` (`dimension_parameters`, `body`) |
| Artifact constants | Canonical C2 `ValueDraft` (`schema`, `shape_values`, `data`) |
| Artifact inputs | `{input,name,slot,schema}` |
| Artifact slots | `{slot,schema,role,initializer}`; role 1 input, 2 state, 3 derived |
| Artifact producers | `{"Input":input}` or `{"NodeOutput":{"node":n,"output_ordinal":p}}` |
| Artifact nodes | `{node,operation,contract,requirement,input_start,input_end,output_start,output_end}`; `requirement` is a dense application-requirement ID or `null` |
| Artifact bindings | tagged `Input`/`Output` records containing ID, node, port, and source/target |
| Artifact outputs | `{output,name,source,schema}` |
| Artifact integrity constraints | `{constraint,operation,contract,inputs}` |
| Artifact operations | `{module_path,operation_name}` |
| Artifact operation contracts | Canonical binary `OperationContractTable`, described below |

An artifact source is `{"Constant":id}` or `{"Slot":id}`. Schema and
constant arrays are already in canonical ID order. Operation rows are sorted
and deduplicated by module path and operation name; nodes and constraints
refer to them by zero-based `operation` and to the canonical contract table by
zero-based `contract`. The engine reconstructs
`ProgramArtifactDraft`, validates all references and producer/binding
bijections, recomputes `ProgramRevision`, and exposes only the finalized
read-only artifact.

### Operation-contract binary encoding

The artifact operation-contract section is binary rather than JSON. It begins
with `contract_count u32`, followed by `contract_length u32` and exactly that
many canonical bytes for each contract. Contract rows are sorted by their
canonical bytes, duplicate rows are forbidden, and every node and integrity
constraint contract ID is in range. Each contract starts with encoding version
`1 u8` and contract tag `0` for `Declared`. This declared-only encoding is the
initial supported bytecode-v1 baseline established by R1. Pre-R1 experimental
contract tags are unsupported and fail canonical decoding.

A declared contract is:

```text
input_count u32
repeated input_count times:
  schema u32, access u8, delivery u8
output_count u32
repeated output_count times:
  schema u32, access u8, delivery u8
  output_construction
  alias_policy
  change_detection u8
external_interaction
```

Access tags are `0 Read`, `1 Write`, `2 ReadWrite`, `3 Consume`. Delivery tags
are `0 Signal`, `1 Stream`, `2 Future`. Change-detection tags are
`0 KernelReported`, `1 ExactScalar`, `2 SemanticHash`, `3 AlwaysChanged`.

Output construction starts with one tag:

- `0 FullWrite`, followed by a shape rule;
- `1 ReadModifyWrite`, followed by `base_input u16` and a region tag;
- `2 Replace`, followed by a shape rule;
- `3 Build`, followed by `module_segment_count u32`, that many strings, and a
  contract-name string.

A string is `byte_length u32` followed by that many UTF-8 bytes. Shape-rule
tags are `0 Declared`, `1 SameAsInput` plus `input u16`, `2 TransposeOf` plus
`input u16`, and `3 MatrixProduct` plus `lhs u16, rhs u16`. Region tags are
`0 SingleElement`, `1 ContiguousRange`, `2 RectangularRegion`,
`3 CollectionEntry`, `4 Arbitrary`. Alias-policy tags are `0 NoAlias`,
`1 MayAlias` plus `input u16`, and `2 InPlaceRequired` plus `input u16`.

External interaction is `0 Pure`; `1 Observation` followed by replay tag
`0 CaptureAsInputFact`; `2 Effect` followed by a delivery tag
(`0 ProviderDefined`, `1 AtMostOnce`, `2 AtLeastOnce`, `3 IdempotentRetry`)
and idempotency tag (`0 NotRequired`, `1 Optional`, `2 Required`); or
`3 TransactionalExternal` followed by protocol tag `0 PrepareCommit` or
`1 PrepareCommitCompensate`.

Declared-contract validation is part of canonical decoding. Every referenced
input ordinal must be in range. `MayAlias` and `InPlaceRequired` require the
referenced input schema ID to equal the output schema ID exactly. `Effect`
contracts have zero outputs; `Observation` and `TransactionalExternal`
contracts are not subject to that restriction. Every `Build` module-path
segment and contract name is nonempty, trimmed, is neither `.` nor `..`, and
contains no NUL, `/`, or `\` character.

The source compiler's in-memory sidecar supplies an exact schema kind for
every semantic register, one role for every instruction, source-definition
order, the return register, and explicit integrity-result registers. Artifact
lowering is fail-closed: it does not infer an output schema from an input,
discard unresolved ports or instructions, or invent fallback interface and
operation names. External-input and output names retain their source symbol
names; runtime, host, and resource operation paths retain their registered or
declared segments. Legacy integrity marker instructions remain executable
compatibility details and are omitted from artifact nodes; their Boolean
results instead become `integrity/assert` declarations in the artifact
integrity-constraint section. Register nodes retain a source constant as their
state initializer when one exists. An executable effect whose bytecode
destination is the compiler-only `Empty`, `Any`, or `None` pseudo-kind remains
an artifact node with all inputs in order and has zero semantic outputs; the
adapter never fabricates a schema for that non-value destination.
Legacy atom, enum, or named-kind IDs do not contain a canonical declaration
path. The source adapter therefore rejects them with a structured unresolved
nominal error unless future compiler metadata supplies that exact path; it
never embeds a synthetic `legacy/...` identity in an artifact.

The constant representation is total for the C2 snapshot family. A reified
schema carries its `SchemaKey`; a reified kind carries validated canonical
closed-kind bytes and is reconstructed without a legacy kind value. Decoding
rebuilds the closed semantic kind, runs the C1 canonical encoder, and requires
the reencoded bytes to equal the supplied bytes exactly. Dynamic shape
parameter values remain in `ValueDraft.shape_values`.

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
| One artifact section | 16,777,216 bytes |
| All artifact sections | 67,108,864 bytes |
| Artifact schemas | 100,000 |
| Artifact constants, inputs, slots, nodes, bindings, outputs, constraints | 1,000,000 each |
| Artifact operations | 1,000,000 |
| Artifact operation contracts | 100,000 |
| Variadic or host-call arguments | 65,536 |
| Type recursion | 256 |
| Constant recursion | 256 |
| Table rows | 1,000,000 |
| Table cells | 1,000,000 |

## Determinism

Writers finalize child types before parents and sort equal-depth types by
their canonical keys. Constants, symbols, dictionary entries, requirements,
sections, maps, sets, and enum variants use their specified canonical order.
All reserved fields and alignment bytes are zero. Equivalent programs
therefore produce identical bytes.

The corpus under `tests/architecture/bytecode-v1/` records source or
construction origin, bytecode SHA-256, decoded structure, runtime-function
identity, and expected output. `scripts/check-bytecode-v1-format.py` checks it
independently of the Rust reader.

Native-build plans are deliberately outside this frozen corpus. Their digest
includes the selected implementation, dependency-resolution seed, and
workspace fingerprint, so it must change when build ownership or workspace
content changes without altering bytecode v1. Native-plan determinism and
generated dependency graphs are enforced by the dedicated `mech-build` tests
and native-plan CI job.

## Unsupported constants

Bytecode v1 deliberately has no fallback serializer. It rejects
`Value::MatrixValue`, `Value::IndexAll`, user functions, native closures,
dynamic-module functions, dynamic-library-backed values, opaque host objects,
cyclic references, arbitrary alias-preserving graphs, general `Value::Typed`
wrappers other than `Option`, and noncanonical nonempty `EmptyKind` values.
Unsupported values produce `BytecodeConstantUnsupported`; excessive constant
nesting produces `BytecodeConstantDepthExceeded`.

## Version policy

Version 1 is the only bytecode format before launch and evolves directly with
the architecture. There is no pre-v1 reader, translation branch, or
compatibility promise for earlier prerelease v1 layouts. At launch, bytecode
v1 freezes as the first supported public format; after that boundary, an
incompatible wire-format change requires bytecode v2. A language/runtime ABI
change is a separate explicit decision and must update the header authority.
