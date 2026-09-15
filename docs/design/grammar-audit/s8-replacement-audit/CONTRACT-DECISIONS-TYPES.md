# Type and catalog contract reconciliation at frozen S8B

This is audit-only analysis of `662d29b79`; it changes no production behavior.
It reconciles G02/G03/G12/G13 using repository contracts and the actual semantic,
visibility, encoding and target-binding authorities. Existing observations are
reproduction evidence, not authority to invent a missing language operation.
No additional Rust tests were run for this reconciliation.

## Decisions

| Group | Reconciled classification | Corrective scope |
| --- | --- | --- |
| G02 | Current pure activation rejection and unfinished positive target capability are separate facts. The 25 numeric/layout observations do not demonstrate a mandatory compiler-stage rejection failure; no accepted S8 exclusion for their positive capabilities was established. | Preserve exact current errors and positive value/type witnesses. Physical binding belongs to resident activation and actual loader preflight; effectful and closed-value compiler paths have their own temporary-execution contract. Retain C32, power, matmul and f32 binary-layout capability owners until implementation or explicit scope acceptance. |
| G03 | Internal exports are deliberately enabled for syntax lowering and deliberately unavailable as named source calls. | Enforce existing callable visibility consistently. Direct `compare/max` and `compare/min` calls must reject; these witnesses do not require new resident factories. |
| G12 | Scalar range constraints are admitted syntax with no represented or specified refinement semantics. Earlier code discarded them. Current rejection is honest but does not complete the language family. | Separate a constrained-type design prerequisite from S8B adapters. The exact finite design decisions and acceptance cells are below; no implementation is authorized by this document. |
| G13 | Dimensionless matrix kinds are specified, and canonical reified kind values already support declared dimension parameters. Requiring an external value or inventing new wire storage is unnecessary. | Lower the unsized two-axis matrix kind into the existing closed kind plus declared dimension environment; preserve canonical identity and the distinction from a materialized matrix schema. |

## G02 — semantic validity and configured-target executability

The authority order is explicit in `docs/design/type-system-v1.md:12-32`:
`KindScheme` selects semantic types and conversions; physical factory signatures
cannot select an overload or output. `BuiltinScalarKind` still contains all 17
kinds. `Number` includes all integers, both floats, both complex widths and r64
(`type-system-v1.md:35-76`). A missing physical kernel does not remove its kind.

Frozen `maintained_source_schemes` uses promoted numeric elementwise schemes for
power and adds an exact r64/i32 -> r64 signature
(`src/core/src/type_system/scheme.rs:1406`). Matrix-product schemes use Number,
lossless promotion and dimension compatibility (`scheme.rs:640-695`). Those are
broader than the configured target's physical execution surface:

| Operation | Implemented element/type family in frozen target | Unavailable implementation in frozen target; not accepted milestone exclusions |
| --- | --- | --- |
| Power after semantic promotion | Same-element u8, u16, u32, f32, f64; scalar r64 base / i32 exponent / r64 result is a separate exact signature. | Same-element u64/u128, all signed integers, c32/c64 and r64; rational matrix power is not supplied by the scalar-only special case. |
| Matrix product after semantic promotion | All five unsigned and all five signed integer widths, f32, f64. | c32/c64 and r64 matrix products; Bool/String and other non-Number inputs are semantic incompatibilities rather than this target gap. |

Evidence is independently consistent across installed native and resident
implementations: `machines/math/src/ops/mod.rs:99-126`,
`machines/math/src/catalog.rs:2085-2097,2320-2331`,
`machines/math/src/ops/pow.rs:33-94`,
`machines/matrix/src/catalog.rs:197-224`,
`src/engine/src/resident/numeric.rs:2348-2353,2427-2435,4899-5001`.
A feature-disabled or backend-specific target may admit fewer physical cells.
That records current implementation availability, not an accepted global language
or S8 milestone exclusion. The current factory list cannot by itself close an
unfinished capability. Preserve the corresponding positive witness and named
owner until scope review accepts implementation coverage or a precise exclusion.

`numeric-u64-pow` (`2<u64> ^ 2<u64>`) and `matrix-c64-matmul` reproduce unavailable
physical binding. `rational-integral-power` is an existing positive witness for
`(1/2) ^ 2<i32> == (1/4)`. Each currently rejected cell retains two independent
oracles: its exact present target error and its positive milestone value/type
expectation. Satisfying the first does not satisfy or erase the second.

The actual boundary is established by `canonical-source-boundary.mec:178-182`:
provider-independent contracts describe semantics, while resident availability
can remain an activation concern. Pure `compile_document` skips temporary
execution when no writes/planning values require it (`runtime/program/compiler.rs:884`)
and produces a semantic artifact plus encoded bytes, not an ActivatedPlan.
`install_resident_artifact` calls `preflight_resident_target` and reports
`SemanticUnsupported` / `OperationUnavailableForTarget` before binding/installation
(`runtime/program/loading.rs:393-415`). Unsupported pure activation is therefore
not, by itself, evidence of missing compiler preflight.

For effectful or closed planning-value compilation, the helper temporarily
activates and prepares the required projection, calls the effect-free provider
planning hook, and aborts (`compiler.rs:904-928`; `resource.rs:138-148`). Static
initializers execute their own closed projection (`compiler.rs:2779`). These
responsibilities must reject unsupported execution before live preparation,
publication or delivery; ordinary pure observations do not execute or certify
their negative target cases. `TARGET-REJECTION-RECONCILIATION.md` and its finite
cell ledger own these distinctions.

Finite acceptance obligations:

1. Resolve each of the 17 scalar-kind pairs through the existing power and
   matrix-product schemes, including lossless mixed-kind promotion. Record
   semantic incompatibility separately from target unavailability. Do not
   count 289 pairs as 289 defects.
2. For each successfully resolved output element, classify the target cell
   using the table above and the selected feature/backend declarations.
   Power layouts are scalar/scalar, scalar/matrix, matrix/scalar, compatible
   matrix/matrix; rational/i32 remains its explicit scalar signature. Matmul
   includes scalar-shaped 1x1 matrices, row/column products, rectangular
   matrices, closed and runtime-owned dimensions, and incompatible inner axes.
3. Check current unavailable physical cells at their actual boundary. Pure
   artifact production may succeed and resident activation/loader preflight may
   reject with the recorded target error. Do not invent mandatory rejection
   before `activate`. Separately qualify effectful, closed supplied-value,
   initializer and open-input responsibilities, preserving zero live effects or
   publication on rejection. Effect-free provider planning hooks are permitted.
   A matching current rejection does not complete the positive capability.
4. Supported cells must preserve independently calculated values and exact
   output kind through artifact bytecode, two turns and rejected-turn rollback.
   Integer powers include checked overflow; float domain results follow their
   existing powf contract; r64/i32 includes negative and zero exponents and its
   established rational domain behavior. This audit does not assert those edge
   cases have all been executed.

No second type authority may be derived from factories. The existing semantic
schemes remain authoritative, and missing physical implementation remains
visible as finite C32 arithmetic/reduction/update, power, matrix-product and
f32 binary-broadcast capability groups. Their positive witnesses are retained
for scope acceptance; no permanent exclusion has been established here.
The S8 rehearsal explicitly disallows silently converting uncovered rows to
unsupported scope (`syntax-s8-execution-rehearsal.mec:275-278`). Resolving these
capabilities requires their named implementation owners or an affirmative,
precise scope decision, not merely changing a test to expect rejection.

## G03 — Internal is not source-callable

`FunctionEnvironment::from_catalog_defaults` enables Internal operations without
installing name bindings (`src/engine/src/function/environment.rs:29-48`).
`bind_catalog_export` explicitly rejects such bindings with “internal operations
cannot install source-callable names” (`environment.rs:83-96`). The test
`internal_exports_cannot_be_bound_as_source_names` preserves that distinction
(`environment.rs:471`). `docs/design/function-catalog-engine-transition.md:50-76`
requires named calls to resolve the current environment; syntax operators use
canonical operation IDs independently.

Both compare/max and compare/min are Internal
(`machines/compare/src/catalog.rs:84-97`). The frozen function surface also lists
both in `full_source_specializers` and neither in `prelude_source_specializers`
nor `module_exports` (`tests/architecture/function-system/function-surface.json`).
Inventory membership therefore never promised direct source calls.

Canonical calls currently retrieve a type declaration by spelling after import
alias rewriting, without the same visibility gate
(`src/engine/src/source_semantics/frontend.rs:3298-3341`). This explains why the
audit's direct `compare/max(1.0,2.0)` and `compare/min(1.0,2.0)` calls reach late
binding. A missing factory is downstream evidence of wrongly admitted syntax.

Finite acceptance obligations:

1. Enumerate the configured catalog's Internal/Prelude/ModuleOnly exports and
   test every lexically expressible Internal spelling as a negative named
   call, including import aliases. Underscore-containing internal identifiers
   are syntax rejection cases; use their real operator form for positive
   execution coverage instead of inventing a source spelling.
2. Preserve Internal syntax operations: strict comparisons, set operators,
   table joins and selected assignment still resolve their operation IDs.
   Named user functions may use a spelling without replacing operator meaning.
3. Prelude calls remain directly visible; ModuleOnly calls require exact
   imported export bindings. Unknown names and missing modules reject before
   lowering effectful arguments. The canonical route must preserve the supplied
   catalog rather than recover an unavailable named function from a global
   maintained-name fallback.
4. Existing witness IDs `catalog-compare-max` and `catalog-compare-min` become
   negative visibility assertions. Do not “fix” them by adding kernels unless
   a separate authorized consumer of those Internal operations requires one.

## G12 — scalar constraints require a prerequisite decision

The active grammar explicitly retains
`kind-scalar := identifier, ?(colon, range-expression)`
(`docs/design/specification.mec:2747`). `range-expression` accepts two or three
formula operands and inclusive/exclusive operators (`specification.mec:2804`).
The canonical typed syntax exposes the range
(`src/syntax/tests/canonical_phase_2i_typed_views.rs:590-605`). The old parser
consumed and discarded it (`src/syntax/src/literals.rs:520-524`); that is evidence
of the original semantic gap, not an acceptable behavior to preserve.

Neither `KindExpr` (`src/core/src/kind_expr.rs:19-50`) nor `SchemaBody`
(`src/core/src/schema/mod.rs:93`) has a scalar refinement variant. Type System v1
specifies operation and dimension constraints, not scalar-value refinements.
The frozen frontend rejects both reified and annotation positions explicitly
(`src/engine/src/source_semantics/frontend.rs:6872-6878,7101-7108`). Its claim that
this needs a “first-class constrained schema” is an implementation diagnostic,
not a reviewed proof that only that representation is possible.

Thus the audit can close the historical question: this was not completed
semantics hidden behind an adapter. It cannot honestly close the design question
by assigning arbitrary range semantics or reclassifying admitted syntax as a
permanent user error. The constrained-type prerequisite needs an explicit contract document settling these six
specific decisions before production implementation:

1. **Meaning:** interval membership versus membership in the stepped sequence
   denoted by the existing full range expression; inclusive/exclusive endpoints,
   descending ranges, a zero step and empty ranges.
2. **Domains:** which scalar kinds admit constraints; exact comparison and
   conversion rules for typed endpoints, mixed widths, rational/complex values,
   infinities and NaN. `RangeEndpoint` alone is not a justified implicit answer.
3. **Evaluation:** whether endpoints must be closed constants, activation
   values or turn values; effects and lexical captures in formula operands;
   invalid or changing endpoint behavior.
4. **Identity and transport:** refinement participation in kind/schema identity,
   reified values, membership, bytecode encoding, hashing and imports. An
   integrity check attached to one assignment cannot silently replace a type
   contract in annotations elsewhere.
5. **Enforcement:** definition, external input, parameter, return, aggregate
   element, conversion and mutable-update boundaries; violations must reject
   atomically without publishing invalid state.
6. **Type relationships:** inference, equality, promotion, subtyping or explicit
   absence of subtyping, and interaction with aliases and nested annotations.

For each decision the reviewed contract must choose supported semantics or an
explicit positioned unsupported-target rejection. Until then, these are six
identified design obligations, not an unbounded search and not implementation
approval. The minimal witness is `constrained-kind`:
`x := 2<u8:1..10?>; x`. The original bare constrained-definition sample failed indexing and was not semantic evidence. The corrected form follows existing constrained-expression tests. Its eventual acceptance requires both an in-range success and an
out-of-range failure plus retained-state rollback under the chosen contract.
Use boundary pairs (lower, upper, just outside), all supported/rejected domains,
two-/three-operand ranges and each enforcement site to make the eventual
acceptance matrix finite. A parser acceptance test or silently erased constraint
cannot close any of these cells.

## G13 — dimensionless reified matrix kinds already have a representation

The specification explicitly calls `<[u8]>` a dynamic matrix kind
(`docs/design/specification.mec:688-700`), and kinds remain first-class immutable
meta-values (`docs/design/value-semantics-and-migration.md:80-97`). Its closed-kind
restriction forbids free **kind** parameters, not a dimension parameter whose
canonical declaration travels with the value (`value-semantics-and-migration.md:449`).

The actual encoder takes a declared dimension environment
(`src/core/src/kind_expr.rs:76-97`); `ReifiedKind` retains canonical bytes and can
recover that environment (`src/core/src/snapshot/data.rs:441-487`). The existing
`canonical_parameterized_kind_bytes_reconstruct_exactly` test creates and
round-trips a reified matrix kind with a declared dynamic extent
(`snapshot/data.rs:875-892`). No value supplying a current matrix size is needed.
`KindExpr::Hole` and undeclared/free parameters remain explicit errors.

The canonical annotation path already expands absent matrix dimensions to two
independent inferred extents (`frontend.rs:7160-7184`), while the reified path
unconditionally rejects absence (`frontend.rs:6922-6938`). Reified map/set/table
kinds already use declared inferred extents via `inferred_kind_dimension`
(`frontend.rs:6905,6919,6996,7004-7017`). That is a concrete shared representation
for G13; rejecting every dimensionless reified matrix is not forced by closure.

Finite acceptance obligations:

1. `<[f64]>` produces an immutable ReifiedType with two independent nonnegative
   dimension declarations. Preserve the distinction from `[f64; fixed rows,
   fixed columns]` and from a current matrix instance. Reuse the existing
   reified-aggregate dimension convention; explicitly record lifetime/bounds in
   the acceptance oracle rather than copying an annotation's runtime instance.
2. Assert decoded closed kind structure, two distinct axes, lower bounds and
   lifetime; round-trip the artifact/bytecode and independently compare canonical
   type identity. Two independently constructed equivalent type values must be
   equal; changing a constrained extent or element kind must distinguish them.
3. Test scalar, fixed matrix, unsized matrix and nested matrix-kind values, both
   kind syntax delimiters, and supported containment in option/tuple/record/set
   and map type expressions. An omitted dimension must not introduce a free
   kind parameter, a Hole, or infer equal rows and columns.
4. Preserve errors for unresolved kind holes, undeclared dimensions, invalid or
   negative extents and unsupported rank. The specification has a historical
   “3D Matrix” example next to the explicit two-dimensional semantics
   (`specification.mec:698,736`); this task does not silently expand resident
   matrices to rank three. Record that documentation contradiction for the
   rank contract instead of copying the example into new execution semantics.

`reified-inferred-matrix` is the existing failing executable witness and requires
a positive value-structure/identity test. Its current
`expected: null` proves no independent type identity, so that oracle must be
strengthened before claiming the family certified.

## Reproduction

From this audit worktree, select each existing observation without modifying the
frozen implementation:

```sh
MECH_AUDIT_CASE=numeric-u64-pow ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh semantic_replacement_witnesses
MECH_AUDIT_CASE=matrix-c64-matmul ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh semantic_replacement_witnesses
MECH_AUDIT_CASE=rational-integral-power ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh semantic_replacement_witnesses
MECH_AUDIT_CASE=catalog-compare-max ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh semantic_replacement_witnesses
MECH_AUDIT_CASE=constrained-kind ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh semantic_replacement_witnesses
MECH_AUDIT_CASE=reified-inferred-matrix ./docs/design/grammar-audit/s8-replacement-audit/run-probes.sh semantic_replacement_witnesses
```

Default observation success does not certify a behavior. `MECH_AUDIT_REQUIRE_PASS=1`
checks the exact current boundary oracle; G03 uses the maintained negative named-
visibility contract, while G02/G17/G18 retain separate unfinished capabilities.
Set `MECH_AUDIT_REQUIRE_CAPABILITY=1` as well to require positive execution and
its preserved expected values for those capability fixtures. The two-axis harness
checks direct and decoded current rejection without treating those matching
failures as positive semantic qualification. Two identical wrong outputs likewise
cannot satisfy an independently expected value.

## C32 reconciliation: transport support and unfinished arithmetic

Native math/stats catalogs enumerate C64 arithmetic and reduction factories,
while the resident predicates accept Complex(W64)
(`machines/math/src/catalog.rs:1115,1224,2048,2082`,
`machines/stats/src/catalog.rs:148`,
`src/engine/src/resident/numeric.rs:2339-2340,2362,2373,2839`).
`RuntimeCheckedArithmetic` has a C64 implementation, not C32
(`machines/math/src/ops/mod.rs:99`). This proves current implementation limits;
it does not prove that S8 accepted a C32 arithmetic exclusion.

QT-c32 passes exact live source/bytecode transport. Eleven existing C32 basic,
reduction and selected-update fixtures therefore identify missing operation
capability rather than a missing general storage family. C32 power and matrix
product retain their separate power/matrix-product capability owners. The exact
current target errors are recorded under G02, while the positive arithmetic
outputs remain CAP-C32 obligations with explicit scope acceptance pending.

The previous claims that early compiler rejection was the required correction
and that no C32-kernel work could belong to the milestone are withdrawn. Current
physical rejection can be correct while the promised source/resident capability
remains incomplete. The relevant kernel/binder work stays finite and reviewable;
complex arithmetic follows its floating contract, without an invented checked-
integer overflow rule. No production implementation is authorized by this audit.
