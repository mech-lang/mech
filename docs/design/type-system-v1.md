# Type System v1

## 1. Status and scope

**Status: R3 complete.**

Type System v1 is the semantic authority for the language Mech already
exposes. R3 covers first-order, expression-local inference; built-in semantic
predicates; overload resolution; lossless implicit conversions; numeric
promotion; checked explicit casts; and structured diagnostics. It adds no new
language syntax.

Every maintained expression must produce exactly one of a closed
`ResolvedType`, a structured ambiguity error, or a structured incompatibility
error. Trying runtime factories until one accepts an invocation is not type
resolution.

## 2. Semantic authority order

The compiler resolves calls in this order:

1. `Schema` and a validated `ShapeInstance` define actual input types.
2. `KindScheme` and built-in predicate and promotion constraints define operation
   semantics.
3. `TypeConstraintEnvironment` resolves a semantic overload.
4. `ConversionPlan` records every selected input conversion or explicit cast.
5. Converted semantic inputs are passed to physical runtime-factory binding.
6. The produced value is checked against the resolved semantic output.

`FunctionValueRepresentation` and `RuntimeFunctionSignature` are temporary
physical execution metadata. They cannot select a semantic overload or output.
R2 storage compatibility remains shadow-only until R4.

## 3. Builtin scalar registry

`BuiltinScalarKind` is the sole semantic registry for builtin scalar identities,
ordinals, names, paths, schemas, and kind expressions. Its stable order is:

| Ordinal | Kind | Ordinal | Kind |
| ---: | --- | ---: | --- |
| 0 | `u8` | 9 | `i128` |
| 1 | `u16` | 10 | `f32` |
| 2 | `u32` | 11 | `f64` |
| 3 | `u64` | 12 | `c64` |
| 4 | `u128` | 13 | `r64` |
| 5 | `i8` | 14 | `string` |
| 6 | `i16` | 15 | `bool` |
| 7 | `i32` | 16 | `c32` |
| 8 | `i64` | | |

The placement of `c32` preserves all previously assigned ordinals. `c32` and
`c64` are distinct semantic types even when only `c64` has a physical runtime
representation.

## 4. Built-in predicate table

R3 provides a small closed set of compiler-defined type predicates. They
classify semantic kinds but provide no methods, instances, inheritance,
declaration syntax, or runtime dispatch.

The fixed predicates are `Number`, `Real`, `Integer`, `FloatingPoint`,
`Ordered`, `Negatable`, `RangeEndpoint`, `Equatable`, and `Keyable`.

| Predicate | Members |
| --- | --- |
| Number | all integers, `f32`, `f64`, `c32`, `c64`, `r64` |
| Real | Number except complex |
| Integer | signed and unsigned integers |
| FloatingPoint | `f32`, `f64` |
| Negatable | signed integers, floats, complex, rational |
| Ordered | integers, floats, rational, String, Index |
| RangeEndpoint | Index, integers, floats |
| Equatable | supported scalars and recursively equatable aggregates |
| Keyable | exactly the finalized schema keyability relation |

Complex numbers, Bool, Id, Atom, Enum, containers, and Dynamic are not
implicitly Ordered. Dynamic and Reference are not automatically Equatable.
There is no user-facing predicate or instance declaration syntax in Type System v1.

## 5. ResolvedType

`ResolvedType` contains a normalized closed `KindExpr`, canonical declarations
for reachable dimensions, and normalized schema-derived predicate evidence.
Equality uses semantic kind and canonical dimension environment; evidence is
not a new identity component. Evidence imported from equal types is
intersected, never unioned.

Construction rejects holes, undeclared parameters, cyclic bounds,
compile-time runtime dimensions, and malformed structures. Construction from a
schema revalidates its shape, preserves nominal Atom and Enum identity,
preserves dynamic dimension lifetimes and bounds, and does not collapse a
dynamic type to its current cardinality.

## 6. Rigid and bindable dimensions

Only dimensions declared by a candidate scheme are bindable. Dimensions
imported from actual `ResolvedType` values are rigid and carry their type
equivalence-class origin. Equal lifetime and bounds do not make two imported
dimensions equal. Two alpha-equivalent copies of one type may compare equal,
while two independent axes within a type remain distinct.

A repeated scheme variable binds once. Later occurrences must be recursively
equivalent across constants, parameters, addition, multiplication, minimum,
and maximum.

## 7. Dimension evolution and bounds

Dimension evolution is ordered as Fixed, ActivationFixed, TurnBounded, and
TurnUnbounded. Compound expressions take the maximum evolution of their
children. A scheme variable accepts only an actual expression whose evolution
is no more dynamic than its declaration.

Bindings must prove declared lower and upper bounds through checked interval
arithmetic. An unprovable `DimensionLessEqual` relation is incompatibility;
Type System v1 does not defer inequalities to runtime. Saturating arithmetic is
not used for solving or ranking.

## 8. Constraint solving order

One environment validates source-control layout, imports actual and expected
types into rigid namespaces, instantiates scheme variables, unifies exact input
and expected-output structure, solves equality to a fixed point, solves predicate
constraints, promotions, and conversions, checks dimension inequalities and
bounds, closes every output, and rejects every remaining variable or hole.
Kind and dimension bindings use occurs checks.

## 9. Overload scoring

Overloads are ordered lexicographically by conversion cost, wildcard matches,
unconstrained kind bindings, unconstrained dimension bindings, and predicate
generality. Exact matches beat conversions, concrete types beat wildcards, and
narrow predicate constraints beat broader or unconstrained variables.

Registration order, factory identifiers, and storage preference never resolve
semantic ambiguity. Equal-scoring candidates may coexist only when their
outputs and input conversion plans are semantically identical.

## 10. Exact equality

Exact equality means identical normalized semantic kind plus alpha-equivalent
canonical dimension environments. It is separate from permitted conversion,
numeric promotion, and explicit casting. Nominal keys must match; structural
similarity cannot substitute for nominal identity.

## 11. Implicit conversion table

Implicit conversions preserve every source value. They include unsigned and
signed widening, unsigned-to-strictly-wider signed integers, `f32` to `f64`,
`c32` to `c64`, selected exactly representable small integers to floats,
selected integers to `r64`, and exactly representable real-to-complex
conversions. Equal-dimension matrices and Options lift an implicit payload
conversion.

There is no implicit float-to-integer, complex-to-real, rational-to-float,
String/numeric conversion, structural aggregate conversion, or Dynamic escape.

## 12. Numeric promotion table

Promotion chooses the smallest type to which both operands convert losslessly:
same type, then the smallest containing integer, then rational when applicable,
then float, then complex. If no such type exists, promotion fails.

Examples include `u8 + u16 -> u16`, `u8 + i8 -> i16`,
`u32 + i32 -> i64`, `u64 + i64 -> i128`, `f32 + f64 -> f64`,
`u16 + f32 -> f32`, `u32 + f32 -> f64`, `u32 + r64 -> r64`,
`f32 + c32 -> c32`, and `f64 + c32 -> c64`. `u128 + i128`,
`i64 + f64`, `u64 + r64`, and `r64 + f64` have no implicit promotion.

## 13. Explicit cast table

Explicit casts permit checked integer-to-integer conversion; integer/float,
real/complex, complex/real, and rational conversions described by the R3
contract; numeric and Bool display as String; and equal-dimension matrix or
Option payload casts. They do not permit String parsing, String-to-Bool,
arbitrary structural casts, collection element coercion, or Dynamic escape.

## 14. Conversion runtime behavior

Integer casts range-check and never wrap. Float-to-integer rejects nonfinite
values, truncates toward zero, then range-checks. Integer-to-float uses typed
IEEE conversion and rejects a finite source only if the result is nonfinite.
`f64` to `f32` rejects finite overflow. Real-to-complex creates a zero imaginary
part; complex-to-real requires an exactly zero imaginary part. Rational to
integer truncates toward zero. Float-to-rational is unsupported. Numeric and
Bool to String use canonical scalar display. No integer conversion is routed
through `f64`.

Reactive conversion stores one immutable plan, snapshots the live source on
each solve, stages a complete result, and publishes once. Failure leaves the
previous output unchanged.

## 15. Source operation schemes

Every named source operation declares explicit storage-blind semantic
overloads. Schemes cover arithmetic and promotion, strict and promoted
comparison, Bool logic, ranges, matrices, sets, strings, statistics, and
combinatorics. Runtime signatures describe only how an already resolved call is
executed; no semantic scheme is projected from a physical signature.

## 16. Syntax-directed intrinsic boundary

Only explicitly allowlisted parser-only constructs whose typing depends on
selector, target, or construction syntax may be syntax-directed. They still
validate canonical schemas and close their outputs through
`ValueCell::resolved_type()`. Named exported operations are always
scheme-authoritative.

## 17. Diagnostics

Ambiguity and incompatibility are structured and source-located. Failures name
the semantic operation, expected and actual semantic types, and the relevant
predicate, conversion, promotion, nominal, structural, or dimension relation.
Human-readable output never exposes physical representation enums, Rust
factory names, pointers, or storage identities.

## 18. Serialization and artifact policy

Schemes, predicate evidence, resolved calls, and conversion plans are derived
in-memory compiler data. R3 does not change bytecode-v1, canonical schema or
operation-contract encoding, `ProgramArtifact`, stable operation IDs, native
linkage names, dynamic-module ABI v1, or package version 0.3.6. Selected
conversions lower through existing semantic operations.

## 19. R4 handoff

R3 makes semantic resolution authoritative. Physical runtime representations
remain temporary binding metadata, and R2 storage compatibility remains
shadow-only. R4 makes storage compatibility authoritative, removes remaining
representation-based semantic decisions, and corrects the
RowDVector/DVector invariant-axis schema mismatch.

## 20. Non-goals

R3 does not implement higher-order types, generalized dependent types,
let-polymorphism, polymorphic recursion, higher-rank types, new function or
pattern syntax, user-defined predicates or instances, traits, effects,
refinements, coeffects, ownership syntax, or a new bytecode format.

## 21. R3 completion criteria

R3 completes only when builtin scalar identities have one canonical definition
and predicate membership uses one closed compiler-defined classifier; resolved
types and dimensions are closed and sound; exact equality, implicit
conversion, promotion, and explicit casting are separate planned relations;
every named source operation has explicit schemes; semantic resolution occurs
before physical binding; conversions remain reactive and lower into compiled
programs; diagnostics are structured and semantic; standard and full catalogs
and product artifacts pass; the architecture checker runs in normal and Full
CI; both CIs pass on one exact head; R2 remains shadow-only; and the PR retains
the required eight commits.
