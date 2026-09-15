> Current target capability and milestone completion are separate axes. Read
> TARGET-REJECTION-RECONCILIATION.md and CONTROL-TARGET-RECONCILIATION.md with this
> worklist. Pure artifact compilation may defer physical rejection to activation;
> the production loader already preflights before installation/effects. A correct
> rejection is not an accepted milestone exclusion. Positive unimplemented
> capability cells remain owned and open until explicit scope acceptance.

# Finite catalog acceptance worklist

This replaces the generic “all overloads/types/layouts” promise with **34 shared
acceptance families linked to all 480 declared candidates across 120 canonical
export names**. `catalog-acceptance-cells.tsv` preserves each candidate ID and its
actual input/output kind-and-dimension expression, type domain, target domain,
source exposure, corrected source recipe, boundary cells, oracle and owner.
It is an audit worklist, not a claim that its cells have executed. No production
code changed and no Cargo run was made for this document.

The 480 rows are coverage accounting, not 480 independent defects. The independent
roots remain the gap register, notably G02 (unfinished configured-target capabilities, including c32 arithmetic),
G03 (callable visibility), G04–G09/G11 (control and nominal families), G17/G18
(dynamic layout). Sharing a scheme does not prove every operation's arithmetic;
sharing a kernel does not prove every source spelling or promotion.

## The two independent axes of a candidate

Each row's `candidate-layout` is an exact, compact transcription of its declared
inputs and outputs, with K0/K1 kind variables and D0/D1 dimension variables.
Repeated D IDs mean shared symbolic identity; distinct D IDs remain independent
even when their runtime sizes agree. The `predicate-domain` retains its actual
constraints; `kind-domain` lists concrete allowed kind classes. `configured-target-domain`
records physical restrictions separately. Do not infer semantic overloads by
trying kernels until one binds.

Candidate coverage requires (a) resolving that candidate in isolation with the
specified kinds, dimension origins and expected result, and (b) source/artifact
execution with the independent output oracle. A literal-only program can choose
a more-specific fixed-shape candidate instead of a compatible-dimension candidate.
Therefore a source success does not certify every candidate admitting that source.
For the pairs preserving the left versus right output dimension owner, explicitly
supply that expected output type and assert retained dimension ownership.

The row's `discovery-command` is an existing executable **export sample only**.
Its original sample may exercise a currently wrongly admitted Internal or unimported
ModuleOnly name; the corrected `source-recipe-json` states the intended positive
operator/import or negative call. The candidate-specific solver assertions and
boundary executions below remain explicit acceptance work. No invented test
command or zero-case filter is presented as completed coverage.

## Callable exposure is itself a closed 120-name matrix

Source catalogs contain **52 Prelude**, **50 ModuleOnly**, **17 Internal-only**
canonical names, plus **string/concat** with both ModuleOnly and Internal exports.
These are canonical-name counts; duplicate exposure records are not another name.

- Prelude names resolve directly, except `set/not_equals` and `set/proper_subset`:
  their underscore spellings are not lexical source identifiers. They are
  **Prelude, not Internal**; their maintained operator witnesses are `≠` and `⊂`.
- ModuleOnly positives prepend the exact module import (`+> math`, `+> stats`,
  `+> combinatorics`, or `+> string`). Each also has missing-import, missing-item,
  exact-item alias, and conflicting-alias negative/positive cells.
- Internal-only direct calls must reject before argument evaluation. Strict
  comparisons, increment ranges, set membership/superset/symmetric difference
  and six joins use their actual operator source forms. There is no demonstrated
  canonical source operator mapping for `set/cartesian-product`, `compare/max`
  or `compare/min`; their direct source cell is negative. Their catalog candidate
  declarations still require isolated type-contract validation. Do not invent
  public source syntax or new runtime factories merely to make those samples pass.
- `string/concat` gets a named binding only from its ModuleOnly export; its
  Internal record permits syntax-level use without exposing a bare name.
- Assignment exports are Prelude but require the addressed syntax-directed call
  layout. A value-only direct function call is not an RMW witness.

This is backed by the actual shipping compiler path, not solely enum names:
`ProgramCompiler` tree compilation constructs `new_program`, installs only the
source's declared imports (`compiler.rs:1696-1702,2422`), and initializes
`FunctionEnvironment::from_catalog_defaults` (`interpreter/mod.rs:1231`).
Namespace import delegates through `CompilerPlanningProgram::install_function_module`
to `load_module`; exact bindings are tested in `function/module.rs:2157`.
The canonical named-call lookup instead retrieves type declarations directly
(`source_semantics/frontend.rs:3298-3341`). The following same-catalog differential
pair is an executable acceptance recipe for G03:

```mech
answer := math/cos(0f32)
answer
```

The unimported program must reject on both canonical `compile_document` and
frozen `compile_source`. Prepending `+> math` must produce f32 1 on both, through
bytecode and two turns. `+> wave := math/cos` with `wave(0f32)` tests item aliasing.
`compare/max(1.0,2.0)` remains not-visible even when `+> math` is installed.
The canonical harness currently admits missing imports; that is one shared G03
lookup defect, not 51 separate implementation tickets.

## Explicit kind basis

Every scalar mention in the TSV is a concrete domain. Numeric domains contain
u8/u16/u32/u64/u128/i8/i16/i32/i64/i128/f32/f64/c32/c64/r64;
Bool/String are separate exact schemes. Id and Index are separate schema kinds,
not aliases for u64. The finite structural witnesses are:

| Basis | Exact constructor cells | Rejection cells |
| --- | --- | --- |
| E-STRUCT (Equatable) | empty tuple; tuple(u8,String); record(x:i32,y:Bool); Option(u8) None/Some; Atom(:a); Enum tag with u8 payload; matrix of u8; matrix of tuple(u8,Bool); table with u8/String columns; set of u8; map(String,u8); ReifiedType <u8>. Repeat nested tuple(record(Option(u8))) to cross recursive owners. | Dynamic and Reference have no automatic Equatable claim. ReifiedType is Equatable (`type_system/resolved.rs:409`), while remaining non-keyable. Nominal enum construction is blocked by G11, not excluded. |
| K-STRUCT (Keyable) | the same scalar key domains plus Enum(payload u8); Option(u8) None/Some; empty/nonempty tuple; record(x:u8); matrix u8; set u8; nested tuple(record(Option(u8))). | c32/c64, Dynamic, Table, Map, ReifiedType; an Option/tuple/record/matrix/set containing any non-keyable child. |
| V-STRUCT (transport) | Option(u8), tuple(u8,String), record(x:i32), Enum(u8), nested matrix u8, table(u8,String), set u8, map(String,u8), ReifiedType <u8>, Dynamic carrying u8 and String. | invalid shape instance, wrong child schema, undeclared nominal identity, Hole/Reference materialization. |

These finite constructor and composition cells exercise the recursive owners;
they are not a claim to exhaust infinitely deep source trees. Retain the canonical
schema property tests for recursive closure and resource-limit rejection.
Keyability comes from `src/core/src/schema/validation.rs:111-131`, not a guessed
scalar-only list. In particular, matrices and sets can be keys when their children
are keyable; tables/maps/reified values cannot.

Every layout/lifetime/empty-shape cell first resolves semantic validity and then
configured target capability. A semantically valid cell without a physical
implementation may compile to an artifact and reject during resident activation;
production loading must reject before installation or live effects. That current
rejection does not complete the positive milestone capability or establish an
accepted exclusion. Element-kind target-domain entries do not demonstrate that
every fixed, Activation or Turn dimension layout executes. Record both the current
physical result and the retained positive capability for scope acceptance.

## Exact layout recipes

`M[K]` is `[1<K> 2<K> 3<K>;4<K> 5<K> 6<K>]` (2x3), `C[K]` is
`[1<K>;2<K>]` (2x1), `R[K]` is `[1<K> 2<K> 3<K>]` (1x3), and `S[K]` is
`2<K>`. For Bool substitute alternating true/false; for String use
`"a","b","c","d","e","f"`. A source recipe is `answer := OP(L,R)\nanswer\n`
with the TSV's import or actual operator form. Unary cases use `OP(L)`.
The family oracle determines valid operand values (e.g. unsigned subtraction
must not accidentally use 1-2 for the success case).

| Layout ID | Enumerated concrete cells |
| --- | --- |
| U2 | S; M; 1x1 matrix; 1x3 row; 2x1 column; typed 0x0, 0x3 and 2x0 matrices via explicit input schema/ShapeInstance. |
| B10 | SS, MM with identical dimensions, MS, SM, MC, MR, CM, RM, independently owned equal-size MM retaining left output axes, independently owned equal-size MM retaining right output axes. These correspond to the ten promoted/boolean scheme alternatives. |
| EQ32 | exact whole-value E-STRUCT; numeric MM; Bool MM/MS/MC/MR/SM/CM/RM; String same seven; compatible numeric MM output-left/output-right; compatible Bool two; compatible String two; promoted numeric B10. |
| STRICT2 | identical semantic whole values -> scalar Bool; same-element matrices with independently owned compatible axes -> scalar Bool. Different kinds reject in source; do not confuse resident mismatch fallback with source admission. |
| B10-ORDERED; STRING2 | numeric B10 intersect Ordered; exact String SS/MM only. String broadcasting is not declared by these two string-order schemes. |
| RMW3 | scalar lhs/scalar rhs; matrix lhs/scalar rhs; matrix lhs/matrix rhs. For addressed variants the selector is a separate syntax layout input, never a numeric promotion operand. |
| RANGE2/RANGE3 | start/end; start/step/end, in source `1..3` and `1..2..=8`. Output 1xN, not Nx1. |
| PRODUCT2 | [1 2 3;4 5 6] ** [1 2;3 4;5 6]; same values with independently owned compatible inner axes. Also 1x3*3x1 and 3x1*1x3. |
| DOT2 | [1 2;3 4] · [5 6;7 8], scalar 70; same shape with independent axes. |
| SOLVE2 | [2 0;0 4] backslash [6;8] -> [3;2]; same square shape under independent dimensions; 2 RHS columns. |
| CONCAT/TRANSPOSE | one, two and three operands; scalar/row/matrix concatenation; [1 2;3 4]' -> [1 3;2 4]; typed empty shapes; dynamic cardinality changes. |
| MATRIX-COMP/SET-COMP | empty generator; one/two generators; filters yielding 0/1/multiple elements; repeated set yields; each listed scalar and structural binding/capture/yield kind. |
| SET-DEFINE/MEMBERSHIP/UPDATE/BINARY/RELATION/CARTESIAN/POWERSET/SIZE | Exact finite sets A={1,2}, B={2,3}, E={} with explicit element schema; singletons; permutation/duplicate construction; K-STRUCT keys and the rejection basis. |
| JOIN-TEMPLATE | common key only; composite common keys; disjoint key columns; duplicate matching keys; empty left/right; unmatched left/right; same-name payload conflict; differing key kinds; each six join modes. |
| CHOOSE-SCALAR/MATRIX | n=4,k=2 ->6; [1 2 3],k=2 -> columns [1;2],[1;3],[2;3], plus typed domain variants. |
| SUM/STRING | [1 2;3 4] reduced on each named axis; string scalar concatenation "a"+"β" -> "aβ". |

Every matrix family additionally uses **fixed**, **Activation-owned** and
**Turn-owned** dimensions, and negative incompatible shapes 2x3 versus 3x2.
Use `a<[K]>` / `b<[K]>` canonical input declarations with explicit compiler input
schemas and activation/turn values for the latter two; a constant declaration is
not a substitute for a live dimension. Record ownership in the expected result.
Empty shapes use typed input values; untyped `[]` cannot witness every element kind.

## Family ownership and target restrictions

Owner shorthand resolves to repository paths: `scheme.rs` =
`src/core/src/type_system/scheme.rs`; `numeric.rs` =
`src/engine/src/resident/numeric.rs`; `environment.rs` =
`src/engine/src/function/environment.rs`; `comprehension.rs`,
`document_assignment.rs` and `frontend.rs` are in
`src/engine/src/source_semantics/`; resident set/table owners are
`src/engine/src/resident/{set,table}.rs`. Machine catalog names refer to
`machines/<machine>/src/catalog.rs`; engine intrinsic catalog is
`src/engine/src/intrinsics/catalog.rs`. SetDefinition template authority is
`src/core/src/function/catalog.rs`.

| Group | Exports / candidates | Common owner | Required boundary cells |
| --- | ---: | --- | --- |
| C01 basic arithmetic | 4 / 40 | scheme.rs:219; numeric.rs:2323,2390 | ARITH;PROMOTION;LAYOUT;TURN |
| C02 power | 1 / 11 | scheme.rs:1406; numeric.rs:2348,2427 | POWER;PROMOTION;LAYOUT;TURN |
| C03 modulus | 1 / 20 | scheme.rs:1418; numeric.rs:2341 | MOD;PROMOTION;LAYOUT;TURN |
| C04 negation | 1 / 2 | scheme.rs:1424; numeric.rs:2356 | NEG;LAYOUT;TURN |
| C05 absolute value | 1 / 4 | scheme.rs:1083; numeric.rs:2365 | ABS;LAYOUT;TURN |
| C06 floating unary | 38 / 76 | scheme.rs:1427; numeric.rs floating unary dispatch | FLOAT-UNARY;LAYOUT;TURN |
| C07 floating binary | 8 / 80 | scheme.rs:1426; numeric.rs floating binary dispatch | FLOAT-BINARY;PROMOTION;LAYOUT;TURN |
| C08 boolean binary | 3 / 30 | scheme.rs bool_binary; numeric.rs semantic logical binding | BOOL;LAYOUT;TURN |
| C09 boolean not | 1 / 2 | scheme.rs bool_unary; numeric.rs logical not binding | BOOL;LAYOUT;TURN |
| C10 non-strict equality | 2 / 64 | scheme.rs:317-414,455; numeric.rs:1862 | EQ;PROMOTION;LAYOUT;TURN |
| C11 strict equality | 2 / 4 | scheme.rs:417-452; numeric.rs strict comparisons | STRICT;LAYOUT;TURN |
| C12 ordered comparison | 4 / 48 | scheme.rs comparison_promoted_for_predicate,string_ordering; numeric.rs comparisons | ORDER;PROMOTION;LAYOUT;TURN |
| C13 Internal min/max | 2 / 20 | compare/catalog.rs:84-97; environment.rs:83-96 | VISIBILITY |
| C14 addressed compound updates | 11 / 33 | scheme.rs exact_assignment; document_assignment.rs; numeric.rs selected update binding | RMW;PROMOTION;TURN |
| C15 range generation | 4 / 4 | scheme.rs:567-623; frontend.rs range; numeric.rs:3962-4132 | RANGE;PROMOTION;TURN |
| C16 matrix product | 1 / 2 | scheme.rs:640-695; numeric.rs:4899-5001 | PRODUCT;PROMOTION;LAYOUT;TURN |
| C17 matrix dot | 1 / 2 | scheme.rs matrix_dot,dynamic_matrix_dot; numeric.rs:4995,5004 | DOT;PROMOTION;LAYOUT;TURN |
| C18 matrix solve | 1 / 2 | scheme.rs:752-785; numeric.rs:5156 | SOLVE;LAYOUT;TURN |
| C19 matrix concatenation | 2 / 2 | scheme.rs:788-847; resident matrix construction/concat | CONCAT;LAYOUT;TURN |
| C20 matrix transpose | 1 / 1 | scheme.rs:627-638; resident transpose binding | TRANSPOSE;LAYOUT;TURN |
| C21 matrix comprehension | 1 / 1 | scheme.rs:1463; comprehension.rs; resident/general control | COMP;TURN |
| C22 set comprehension | 1 / 2 | scheme.rs:1020-1045; resident/set.rs; comprehension.rs | COMP;KEY;TURN |
| C23 set definition | 1 / 1 | catalog.rs source template SetDefinition; resident/set.rs | SET-DEFINE;KEY;TURN |
| C24 set membership | 2 / 2 | scheme.rs:850; resident/set.rs | MEMBERSHIP;KEY;TURN |
| C25 set insert/remove | 2 / 2 | scheme.rs set_insert,set_remove; resident/set.rs | SET-UPDATE;KEY;TURN |
| C26 set binary construction | 4 / 4 | scheme.rs:892-925; resident/set.rs | SET-BINARY;KEY;TURN |
| C27 set relations | 7 / 7 | scheme.rs:966-977; resident/set.rs | SET-RELATION;KEY;TURN |
| C28 Internal Cartesian product | 1 / 1 | set/catalog.rs:97-102; scheme.rs:925-944; resident/set.rs | VISIBILITY;CARTESIAN;KEY |
| C29 powerset | 1 / 1 | scheme.rs:946-964; resident/set.rs | POWERSET;KEY |
| C30 set size | 1 / 1 | scheme.rs:1496; resident/set.rs | SET-SIZE;KEY |
| C31 axis sums | 2 / 2 | scheme.rs:1048; numeric.rs:2783-2895 | SUM;LAYOUT;TURN |
| C32 string concatenation | 1 / 1 | string/catalog.rs:12-35; scheme.rs:1075 | STRING;TURN |
| C33 table joins | 6 / 6 | scheme.rs instantiate_table_join_scheme; resident/table.rs; intrinsics/catalog.rs:249-292 | JOIN;KEY;TURN |
| C34 n choose k | 1 / 2 | scheme.rs:991-1019; numeric.rs:4134-4310 | CHOOSE;PROMOTION;TURN |

## Concrete boundary cells and independent oracles

The cell labels in the TSV expand to the following finite recipes. They are
shared test dimensions, not additional defect tickets. Instantiate only the
candidate's declared kind/layout; negative cells deliberately cross that boundary.
The source and decoded artifact must both satisfy the independent oracle.

| Cell | Source/value recipe | Required oracle |
| --- | --- | --- |
| ARITH | `(6<K>,2<K>)` for +,-,*,/; signed `(−6,2)`; rational `(1/2,1/3)`; complex `(1+2i,3−i)`; exact integer min/max boundaries with one overflowing element at first/middle/last matrix positions. | 8,4,12,3; signed/exact Fraction/complex arithmetic; overflow preserves every prior output and state cell. |
| POWER | `2<K>^3<K>`, `0^0`, max representable checked base/exponent boundary; floats `(4,0.5),(-1,0.5)`; `(1/2)^(-2<i32>)`; every G02 promoted output kind whose target implementation is currently unavailable. | 8; existing integer pow identity; 2; IEEE NaN; exact rational 4. Unavailable target combinations have separate current activation-rejection and positive capability oracles; rejection before live effects does not close the latter. |
| MOD | `(5,2),(-5,2),(5,-2),(-5,-2)` on signed/float kinds; `(5,2)` unsigned; divisor zero; signed MIN/-1. | Truncation remainder 1,-1,1,-1; existing arithmetic error and rollback for invalid integer cases. IEEE float result class follows existing contract. |
| NEG / ABS | 0,1,-1; signed MIN and MIN+1; float -0, ±infinity, NaN; c64 `(3,4)` and `(−3,−4)`; rational ±1/2. | Exact sign inversion or absolute value; c64 abs is `(5,0)` **with c64 output**, per `types/complex_numbers.rs:56`; integer MIN rejects; sign bits/classes independently asserted. |
| BOOL | Both inputs from {false,true}, four combinations; unary both inputs; repeat alternating matrix values. | Exact and/or/xor/not truth tables, output Bool and declared shape. |
| EQ / STRICT | equal/unequal scalar pair; `1<u8>` versus `1<u16>`; tuple/record one child changed; empty/nonempty; nominal tags equal/different; float ±0 and NaN. | Non-strict declared promotion versus strict identical-kind rejection; recursive value equality; numeric MM returns Bool matrix, whole structural and strict matrix comparison returns scalar Bool. |
| ORDER | `(1,2),(2,1),(2,2)`; signed endpoints; rational `(1/3,1/2)`; `"a","β"`; float NaN; complex/Bool/aggregate negative inputs. | <,<=,>,>= truth table and exact rational comparison; no implicit Ordered for excluded kinds. |
| PROMOTION | Same-kind every listed kind; all 225 ordered pairs of the explicitly listed 15 numeric kinds; float-only families restrict to four f32/f64 pairs. Additionally Id/Index/String/Bool crossed with a numeric operand as negatives. | Expected promotion selected by the explicit lossless conversion table, not by output from either compiler. Anchors: u8+i8->i16; u32+i32->i64; u64+i64->i128; u16+f32->f32; u32+f32->f64; f32+c32->c32; f64+c32->c64. u128+i128, i64+f64, u64+r64, r64+f64 reject. Source type and target support are separate assertions. |
| LAYOUT | The layout table plus fixed/activation/turn axes, 0x0, 0x3, 2x0, 1x1, 1x3, 2x1, 2x3; mismatch 2x3/3x2. | Exact logical dimensions, canonical element schema and source-to-bytecode shape ownership; no current extent substituted for symbolic identity. |
| RMW | `~a := [10<K> 20<K>;30<K> 40<K>]`; apply -= and /= at scalar, range `1..=2`, repeated vector `[1 1 2]`, logical mask, rows/all, columns/all, nested tuple/record matrix path; same and promoted rhs; overflow after a prior repeated occurrence. | Sequential fold per selected occurrence, not gather-once/replace; untouched addresses unchanged; failed turn restores the complete prior state. First success and second-turn recurrence are both asserted. |
| RANGE | `1..3`, `1..=3`, `1..2..=8`, `3..-1..=1`, equal bounds, descending bounds with wrong-direction step, zero step, nonintegral floating step 0.5, unrepresentable cardinality. | [1,2], [1,2,3], [1,3,5,7], [3,2,1]; correct empty/result or explicit domain error per existing range contract; cardinality and terminal bound computed independently. No enormous allocation to test a rejected budget. |
| PRODUCT / DOT | Rectangular recipes above; zero inner extent through typed inputs; integer overflow in middle summand; mixed kinds; incompatible axes. | Product [[22,28],[49,64]]; dot scalar 70; empty dot identity where contract admits it; checked failure preserves outputs. |
| SOLVE | Diagonal 2x2 and two RHS columns; permutation matrix; singular `[1 2;2 4]`; non-square lhs; mismatched RHS rows; changing RHS. | Known exact solution and residual; structured singular/shape failure with atomic outputs. Type domain f32/f64 only. |
| CONCAT / TRANSPOSE | Explicit element basis values, singleton operand, scalar+row, two/three matrices; incompatible common axis; typed empty; comprehension filtering 2->0->1 elements followed by concat/transpose. | Concatenated logical coordinates; transpose twice identity; current cardinality and exact nested child schema; G18 failure owned once. |
| COMP | `[x | x <- [1<K> 2<K>]]`, filter x>1, two generators, tuple destructuring, named rest, capture one external value, nested match and nested comprehension; corresponding set syntax with duplicate yields. | Reference ordered list or canonical deduplicated set; each listed binding/yield/capture domain; G06–G09 own blocked forms explicitly. |
| KEY | Every K-STRUCT positive and rejection constructor; values permuted and duplicated; float +0/-0 and two NaN payloads; nested child one-bit change. | Canonical key identity from finalized schema contract, independent set membership/cardinality; keyability error before construction for invalid children. |
| SET-DEFINE / UPDATE | E,A,B; insert existing/new value; remove present/absent value; duplicate source elements and reversed source order. | Canonical sets {1,2}, {1,2,3}, {2}; no mutation of immutable input; exact cardinality. |
| MEMBERSHIP | `1 ∈ A`, `3 ∈ A`, both ∉ forms; sought value of unlike kind; empty set. | true,false and complements; exact kind-aware membership semantics, with the declared distinct sought/set-kind variables retained. |
| SET-BINARY / RELATION | A={1,2}, B={2,3}, E={}, same-set, strict subset singleton, disjoint singleton. | Union {1,2,3}, intersection {2}, A\B={1}, symmetric difference {1,3}; equality/subset/proper/superset/disjoint truth table. |
| CARTESIAN | Direct named call negative; isolated artifact input sets {1,2} and {"a","b"}; empty input. | Named visibility error; artifact declaration/value oracle is exactly four tuple pairs or empty. Runtime qualification is conditional on an actual Internal artifact consumer, not invented source syntax. |
| POWERSET / SET-SIZE | Powerset E, singleton, {1,2,3}; duplicate construction and budget bound. Size E,A,{1,1,2}. | 1,2,8 canonical subsets; u64 sizes 0,2,2; budget rejection before partial publication. |
| JOIN | L={(id:1,x:10),(id:2,x:20),(id:2,x:21)}, R={(id:2,y:30),(id:3,y:40)}; each six join operators; empty L/R; duplicate R id2; two common keys. | Inner 2 rows; left outer 3; right outer 3; full outer 4; left semi 2; left anti 1. Verify every field, multiplicity and Option payload of unmatched rows; canonical key behavior and same-name/type errors. |
| SUM | [1 2;3 4], 1x3, 2x1, typed empty axes, signed cancellation, integer overflow and exact scalar token changes. | sum/row=[4 6]; sum/column=[3;7]; axis-preserving shapes and zero identity where admitted, atomic overflow rejection. |
| STRING | `""+""`, `"a"+"β"`, combining Unicode sequence, embedded newline, zero byte through host snapshot input, repeated concatenation after changed input. | Exact concatenated UTF-8 bytes; no normalization, truncation, or hidden output. |
| CHOOSE | scalar (4,2),(0,0),(4,0),(4,4),(3,4), negative/nonintegral n/k; matrix [1 2 3],k=2, k=0, k>n; allocation budget. | Exact binomial count; ordered combination columns for matrix mode; domain/target/resource rejections explicitly distinguished. An absent physical implementation may correctly reject without becoming an accepted milestone exclusion. Preserve specified domain restrictions; do not manufacture a new complex combinatorial definition. |
| VISIBILITY | Each source exposure case specified above; rejected name's argument contains a host operation; imported alias conflicts. | Precisely positioned lookup failure before argument lowering/host effects; operator identity unaffected by user function shadowing. |
| TURN | Start mutable/input value 1, publish turn1; change to 2, publish turn2; failing third turn; repeat publication after failure. | Independent expected recurrence/value on both artifacts; exact scalar identity even when integer output is not f64; accepted state survives rejected candidate/turn. |

The PROMOTION pair count is finite: 15x15 semantic pairs, not a new fixture-count
claim or a ticket for each pair. An isolated scheme solver checks every pair;
source tests exercise each distinct selected conversion/target/layout class and
its value-domain boundaries. This avoids multiplying type-pair membership into
hundreds of redundant end-to-end tests while retaining explicit pair accounting.

### Floating operation-specific anchors

For C06/C07 test both f32 and f64. Scalar input values below lift into their
candidate's layout, including ordered broadcast inputs. Special float values use
exact host snapshots so spelling or parsing is not the oracle. Exact algebraic
anchors compare exact values/classes; nonexact anchors compare an independent
high-precision reference rounded to the declared width, with the maintained
machine test tolerance. Record the tolerance and reference provenance in the
result. Do not use the same libm call as both implementation and oracle.

| Operation suffix | Exact/known anchor | Domain and rounding boundary inputs |
| --- | --- | --- |
| acos / asin | acos(1)=0; asin(0)=0 | -1,0,1 and adjacent representable values outside [-1,1] |
| acosh / asinh / atanh | acosh(1)=0; asinh(0)=0; atanh(0)=0 | acosh below 1; asinh ±1; atanh ±1 and adjacent values |
| acot / acsc / asec | acot(1)=π/4; acsc(1)=π/2; asec(1)=0 | -1,0,1,2; signed zero and infinities |
| atan / atan2 | atan(0)=0; atan2(0,1)=0 | ±0,±1, infinities; atan2 quadrants (±1,±1) and both signed zeros |
| sin / cos / tan | sin(0)=0; cos(0)=1; tan(0)=0 | ±π/2,±π, adjacent to pole for tan, infinity, NaN |
| sinh / cosh / tanh | 0,1,0 at argument 0 | ±1, large finite ±100, infinities, NaN |
| sec / csc / cot | sec(0)=1; csc(π/2)=1; cot(π/4)=1 | 0,±π/2,±π and adjacent values around denominator zero |
| cbrt / sqrt | cbrt(8)=2; sqrt(4)=2 | -8/-1, signed zero, smallest positive subnormal, infinity, NaN |
| log / log2 / log10 / log1p | 0 at 1/1/1/0 | -1,0,1; log1p adjacent to -1 and ±smallest subnormal |
| ceil / floor / trunc | ceil(1.5)=2; floor(1.5)=1; trunc(1.5)=1 | ±0,±0.5,±1.5,±2.5, infinities, NaN |
| rint / roundeven / round | rint(2.5)=2; roundeven(2.5)=2; round(2.5)=3 | ±0.5,±1.5,±2.5 and adjacent representable values; signed zero |
| erf / erfc | erf(0)=0; erfc(0)=1 | ±1,±infinity, NaN |
| lgamma / tgamma | lgamma(1)=0; tgamma(1)=1 | 0,-1,-0.5,0.5,2,overflow boundary; sign and class |
| bessel/j0 / j1 / jn | j0(0)=1; j1(0)=0; jn(0,0)=1 | arguments 0,1,2; orders -1,0,1,2 and 1.5; order uses existing float-to-i32 cast, not a newly invented type error |
| bessel/y0 / y1 / yn | independent reference at positive argument 1 | argument 0,-1,1,2; same order boundaries; negative domain/class |
| copysign | copysign(1,-1)=-1 | both signed zeros and NaN sign payloads |
| fdim | fdim(1,2)=0 | (2,1), equal pair, infinities, NaN |
| fmod / remainder | fmod(5,2)=1; remainder(5,2)=1 | (3,2),(-3,2),(3,-2), divisor 0, infinite dividend, signed zero; distinguish truncation from nearest-even quotient |
| nextafter | nextafter(1,2)=next representable >1 | equal pair, ±0 direction, smallest subnormal, max finite towards infinity, NaN; exact bits |

C07 has an additional concrete target distinction: f64 `bind_binary` uses scalar,
row and column broadcast modes (`numeric.rs:2295-2320`). The f32 special-function
fallback requires both input schema keys to equal the output schema key
(`numeric.rs:1610-1650`) and equal element lengths (`numeric.rs:8485`). Thus f32
scalar/matrix and row/column broadcast cells are **not demonstrated by f32 scalar
success** and lack physical support in that frozen binder. The minimal source
witness is `+> math\na := [1f32 2f32;3f32 4f32]\nmath/atan2(a,1f32)\n`.
The `catalog-f32-binary-broadcast` observation in
`s8-audit-published-contracts.log` records `KernelBind { node: NodeId(0), error:
UnsupportedLayout }` at activation for both direct and decoded artifacts. G02
owns the finite CAP-F32-BINARY implementation capability; this correct current
rejection is not a demonstrated compiler admission-stage defect. Its positive
broadcast value/layout oracle remains open. The target restriction must not
silently rewrite the storage-blind floating scheme or the accepted milestone.

## Completion accounting

This worklist ends when every `(export, overload-id)` has:

1. An exact candidate resolution oracle covering its concrete kind domain,
   layout and dimension owner, including the specific rejected cells above.
2. Its exposure contract proved at ProgramCompiler and its real operator/import
   source recipe executed where a public source form exists.
3. One shared family boundary result for every applicable cell, linked back to
   this candidate's implementation/semantic owner, plus operation-specific
   floating/relational/collection expected values.
4. Source/bytecode identity and two-turn/rollback evidence under the affected
   configured target profiles from maintained CI. No library-total substitution.

Rows blocked by a named gap retain that gap and their concrete cell. Rows with
no demonstrated public source form prove rejection and isolated declaration
semantics; they do not manufacture an arbitrary new application contract.
The worklist does not require the Cartesian product of backend, syntax spelling,
scalar kind, shape, value and every consumer when they demonstrably delegate to
the same owner. It does require the selected shared evidence and delegation link.
