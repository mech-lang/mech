# Mech: Atom, Enum, and Tagged Union System — Implementation Sketch

## Overview

This document defines a unified design for atoms, enum constraints, and tagged unions in Mech.
The core idea is:

- **Atom** is the primitive.
- **Enum declarations** constrain which atoms are valid in a context.
- **Tagged unions** are enums whose variants may carry typed payloads.

This model supports clear type checking, pattern matching, and synthesis-phase workflows.

## 1. Primitives

### 1.1 Atoms

An atom is a globally unique symbolic constant. Its name is its value.

```mech
:red
:ok
:idle
:done
```

Properties:

- Atoms are interned globally. `:red` is always the same value everywhere.
- Atoms are not owned by any type. Types constrain atoms, but do not namespace them.
- Atoms are compared by identity.
- Atoms are valid standalone values with type `atom` when unconstrained.

Structured atoms (atoms with payloads):

```mech
:ok(123)
:error("something went wrong")
:some(3.14)
```

A structured atom is a `(tag, payload)` pair where the tag is an atom and payload is any Mech value.
Before applying a type constraint, `:ok(123)` is inferred as an existential shape:

- `∃T. :ok(T)` (with literal inference giving `T = i64` here)

Constraints later resolve this existential during type checking.

## 2. Enum Declarations

An enum is a named **constraint** over a set of atoms (or structured atoms). It does not namespace variants.

### 2.1 Simple Enums

```mech
<color> := :red | :green | :blue
<direction> := :north | :south | :east | :west
<status> := :pending | :active | :done
```

Semantics:

- `<color>` means “value must be one of these atoms.”
- Atoms still exist globally.
- Multiple enums can share atoms.

### 2.2 Typed Variable Declaration

```mech
foo<color> := :red
```

Type checking:

- Validate membership of `:red` in `<color>`.
- Reject if atom is not in the enum’s variant set.

### 2.3 Compiler Membership Predicate

Internal type-checking helper:

```text
member(atom, enum_decl) -> bool
```

Behavior:

1. Check tag membership against enum variants.
2. If structured, validate payload type against declared payload schema.

## 3. Tagged Unions

A tagged union is an enum where variants may carry typed payloads.

### 3.1 Declaration

```mech
<result> := :ok(<i64>) | :error(<string>)
<option> := :some(<f64>) | :none
<tree>   := :leaf(<i64>) | :branch(<tree>, <tree>)
```

Semantics:

- Variants are either bare atoms or structured atoms with typed payloads.
- Payload type may be primitive, enum, record, etc.
- Recursive declarations are permitted.

### 3.2 Construction

```mech
bar<result> := :ok(123)
baz<result> := :error("not found")
nothing<option> := :none
```

Compiler behavior:

1. Check tag exists in target type.
2. If payload is expected, unify provided payload type.
3. Emit type error on unknown tag or failed unification.

### 3.3 Untyped Construction

```mech
x := :ok(123)
```

Without annotation, this remains weakly typed and unresolved until constrained.
A useful internal representation:

- `∃T. { tag: :ok, payload: T }` with current inference `T = i64`.

If later assigned to a typed position, validate retroactively.

## 4. Pattern Matching

### 4.1 Basic Match

```mech
color-string := foo?
  | :red   -> "Red"
  | :green -> "Green"
  | :blue  -> "Blue".
```

Rules:

- Match subject must have resolved enum/union type.
- Arms must be legal members for the subject type.
- Exhaustiveness required unless wildcard `_` appears.

### 4.2 Structured Match

```mech
message := bar?
  | :ok(n)    -> n + 1
  | :error(e) -> 0.
```

Bound variables inherit payload types from declaration (`n: i64`, `e: string` for `<result>`).

### 4.3 Partial Match (Non-Exhaustive)

If variants are missing and `_` absent:

- permissive mode: warning
- strict mode: error

### 4.4 Nested Match

```mech
<wrapped> := :some(<result>) | :none

x<wrapped> := :some(:ok(42))

result := x?
  | :some(:ok(n))    -> n
  | :some(:error(e)) -> -1
  | :none            -> 0.
```

Nested structured atoms are matched recursively, with type constraints resolved layer-by-layer.

## 5. Namespace Collision and Resolution

### 5.1 Global Atoms, Local Constraints

Atoms are global; enums constrain usage contexts.

### 5.2 Type-Context Resolution

In typed positions, context selects the relevant enum membership check.

```mech
foo<color>  := :red
bar<status> := :red
```

### 5.3 Ambiguous Untyped Atom

```mech
x := :red
```

If `:red` appears in multiple enums and no type context exists, emit ambiguity error and require annotation:

```mech
x<color> := :red
```

### 5.4 Explicit Qualification (Escape Hatch)

```mech
:color/red
:status/red
```

Qualification is contextual syntax sugar only. It does not change atom identity.

## 6. Suggested Compiler Data Model

A straightforward representation for semantic analysis:

```text
AtomId        := interned symbol id
TypeId        := canonical type reference

VariantSpec   := { tag: AtomId, payload: Option<TypeExpr> }
EnumDecl      := { name: TypeId, variants: Vec<VariantSpec> }
AtomValue     := { tag: AtomId, payload: Option<ValueId> }
```

Type checks:

- `check_atom_against_enum(value, enum_decl)`
- `check_match_exhaustive(subject_enum, seen_tags, has_wildcard)`
- `infer_pattern_bindings(pattern, variant_spec)`

## 7. Implementation Phases

1. **Parser**: ensure syntax support for structured atoms and typed variants.
2. **Type declarations**: store enum variant specs (tag + optional payload type).
3. **Checker**: implement membership and payload unification.
4. **Match checker**: validate legal patterns and exhaustiveness.
5. **Inference**: carry unresolved structured atom forms until constrained.
6. **Diagnostics**: ambiguity errors and strict/permissive non-exhaustive behavior.

## 8. Compatibility and Migration Notes

- Existing atom-only usage remains valid.
- Existing enums map naturally to constraint-only interpretation.
- Tagged-union behavior is additive when payloads are introduced.
- Explicit qualification can be optional and only needed for ambiguous untyped contexts.
