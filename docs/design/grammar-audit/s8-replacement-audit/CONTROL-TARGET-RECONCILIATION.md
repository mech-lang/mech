# Final control versus target-capability check

Audit-only source inspection at frozen B `662d29b79`. No new tests or production
changes. This checks G06/G07/G08/G09/G18 after the c32-target correction.

| Group | Classification supported by inspected contracts | Exact boundary |
| --- | --- | --- |
| G06 | Required unfinished resident control/value prerequisite. | The declared completion gate explicitly requires composite comprehension bindings/yields on `resident-artifact`. Preserve successful primitive destructuring; extend control-local retained values using supported canonical value/storage providers. This does not require every numeric operation on those values or permit non-keyable Set elements. |
| G07 | Required unfinished canonical control composition/shape prerequisite. | The same completion gate explicitly requires nested-comprehension composition on `resident-artifact`. Match inside a comprehension and a nested comprehension fail before any physical call is selected because the typed body/shape ownership is incomplete. Reuse ordinary operation capability admission within the composed graph. |
| G08 | Required unfinished canonical pattern evaluation prerequisite. | The gate explicitly requires computed-pattern blocks on `resident-artifact`. The concrete `1 + 1` pattern needs an existing f64 add operation with correct lexical evaluation/dominance, not a new arithmetic kernel. |
| G09 | Missing canonical semantic pattern representation/lowering for existing structural match syntax. | The concrete tuple match only needs tuple projection, lexical binds, ordinary f64 add and branch selection, all represented elsewhere in the same target. Do not replace the omitted frontend pattern with a permanent language rejection, and do not infer universal physical literal-comparison capability from this case. |
| G18 | Current fixed-layout target limitation plus a finite unimplemented downstream capability retained for milestone scope review. | The current target can correctly reject an unresolved or changing collection layout during pure activation/preflight. That rejection does not establish milestone completion or an accepted exclusion. Retain the canonical source-to-consumer positive witnesses and resident layout owner until the scope review explicitly resolves them; no new implementation is authorized by this audit. |

## Explicit prerequisite authority for G06–G08

`docs/design/grammar-audit/phase-2i-semantic-completion.tsv:5–7` names:

- `comprehension-composite-bindings-yields`;
- `computed-pattern-blocks`;
- `nested-comprehension-composition`.

All three are `unfinished-implementation`, target `resident-artifact`, owned by
S4 and `required-for-s6=true`. This is stronger evidence than syntax acceptance
or a mathematically meaningful sample. The named resident target is already
required to complete them. `canonical-boolean-control.md:48–62` describes the
current Bool/Index/F64 implementation as an S4 increment and explicitly retains
these implementation obligations; `comprehension-certification-migration.md:20–28`
and `recursive-executable-core.mec:295–306` preserve the same distinction.

The current failures correspond exactly to the incomplete owners:

- G06: `resident/general/comprehension.rs:66–107` admits primitive Bool/Index/F64
  bindings, recursively projects a limited borrowed tuple/matrix source, and
  `:174–189` still requires a primitive output element. The i32, String and tuple
  yield fixtures all fail `UnsupportedControlLayout` at node0. This is not a
  claim that ordinary canonical storage lacks these kinds: the exact schema
  tests and compound-match value publication demonstrate their independent
  support. The control-local storage/ownership connection is missing.
- G07: `source_semantics/comprehension.rs:131–137` rejects a yielded element with
  dimension parameters because it needs an independent nested shape witness;
  `:242–273` only converts pure ordinary nodes into collection steps.
  `comprehension-nested` hits the former check and `comprehension-match` hits the
  latter. `artifact/comprehension.rs:45–52` has Generator/Operation/Filter steps,
  no nested control-body variant. No missing physical math operation explains
  those frontend/representation omissions.
- G08: `source_semantics/comprehension.rs:93–100` explicitly says computed
  patterns require an executable pattern evaluation block. A pattern-generated
  ordinary node cannot simply be treated as a literal or run in an unrelated
  outer scope. The missing responsibility is evaluation order, lexical capture
  and iteration ownership. F64 addition itself is an existing admitted target.

The grammar independently corroborates the intended semantics:
`specification.mec:2873–2886` permits expression yields, generator patterns,
lexical definitions and expression filters; `:3039–3063` includes expression,
tuple, array and tagged pattern forms. These grammar rows alone would not prove
a target capability; the explicit named completion gate supplies that part.

G06 acceptance should distinguish matrix values from Set keyability. Generic
transport/capture does not grant a type `Number`, `Ordered`, `Equatable` or
`Keyable`, nor authorize a missing ordinary operation. Each inner operation still
uses the configured target's supported kind/layout domain and preflight policy.
Hole/Reference materialization and non-keyable Set output remain negative cells.

## G09 is a semantic lowering omission, not a request for every compare kernel

`specification.mec:2749–2754` attaches ordinary match arms to expressions;
`:3243–3246` uses the shared `pattern` production. The shared pattern grammar
includes tuple, array and tagged forms. The concrete existing witness
`(1,2) ? | (x,y) => x+y | * => 0` has an independent result3.

`source_semantics/frontend.rs:7964–8021` rejects structural patterns before
physical target selection, while `:8024–8033` is the separate restriction on
scalar literal comparison. Those two checks must not be conflated.
`canonical_source_completion.rs:348` already executes tuple/array projection and
repeated binding in comprehensions; `:235,247,285` executes ordinary scalar match,
compound result values and nested matches. The tuple witness does not demand a
new numeric target kind or a new composite value schema.

Keep G09's structural pattern/lexical scope/guard/fallthrough/exhaustiveness
prerequisite. Keep ordinary match exhaustiveness distinct from partial numeric
function bodies (FUN06) and activation-arm totality (ACT08). Any additional
literal-comparison kind must still have its own semantic/target admission cell;
this finding does not mandate all absent comparison kernels.

## G18: current rejection and milestone completion are different questions

The `dynamic-concat` record is `KernelBind { node: NodeId(1), error:
UnsupportedLayout }`, after its comprehension producer was accepted. The frozen
physical implementation explains that result:

- `numeric.rs:5062–5089`, `declared_matrix_dimensions`, resolves both axes from
  the activation layout's `ShapeInstance` and rejects unresolved dimensions.
- `numeric.rs:2945–3002`, `bind_matrix_constructor`, resolves output and input
  extents and encodes them in kernel parameters.
- `numeric.rs:2667–2735`, `bind_transpose`, resolves extents and stores
  `[rows,columns]` in its bound kernel.
- `numeric.rs:9987–10047` concatenates using those fixed parameters;
  `:9335–9359` rejects a transpose input whose count differs from the bound count.
- `resident/general/comprehension_execution.rs:241–255` instead constructs its
  current output shape from `[1,count]` each turn. Supported producer publication
  does not prove the downstream implementation already exists.

**This does not demonstrate a late-rejection or missing-preflight defect.**
`canonical-source-boundary.mec:178–182` explicitly permits resident availability
to remain an activation concern. Pure ProgramArtifact binding at activation is
an allowed capability boundary. The production loader already calls
`preflight_resident_target` in `runtime/program/loading.rs:402`, before external
binding (:424), instance allocation (:433) and installation/activation (:434).
A direct audit activation failure does not prove that production installed a
bad candidate or performed effects. Any such allegation needs its own real
loader witness. The previous recommendation to fold G18 into a missing-early-
admission defect was not justified by this evidence and is withdrawn.

### Milestone scope evidence

The plans make affirmative source/consumer coverage commitments:

- `syntax-v0.4-continuation-plan.mec:261–280` makes S4 replace existing source
  semantic work and handle every executable Phase 2I form through current
  compiler services, with type/artifact/memory/runtime canaries preserved.
- `syntax-v0.4-continuation-plan.mec:369–386` requires source semantics and typed
  consumers to cover the complete canonical fixture corpus. Its S7A record
  explicitly distinguishes errors for unimplemented units from completion;
  source rejection is not automatically a successful language capability.
- `syntax-s8-cutover-plan.mec:40–52` keeps missing language execution and
  type/shape/control work with S4. Its B1 boundary (:76–80) preserves initializers,
  observable symbols, module resolution, compute partitioning, memory planning
  and artifact ownership. The product matrix (:142–174) requires positive
  execution through actual CLI/runtime/native/browser routes.
- `syntax-s8-execution-rehearsal.mec:220–237` bounds required behavior by maintained
  source/type/operation contracts and checked-in applications, explicitly testing
  complete documents, live changes and downstream propagation.
  `:275–278` says an uncovered row may not silently become unsupported: the
  exclusion must already belong to the maintained contract or be an explicit
  scope decision.

There is no inspected milestone sentence that separately names general
turn-varying concat/transpose as either complete or excluded. The positive G18
source is composed from maintained comprehension and concatenation semantics;
those plans require its unresolved consumer capability to retain an owner and
be considered during scope review. Factory limitations do not settle that review.
The 27 frozen consumer rows identify API ownership, input normalization and
failure handling, not a reviewed exclusion of G18's language/target combinations.
`runtime.program-artifact`, `runtime.program-compile`, and interactive/mixed
compiler consumers are the relevant entry boundaries; a `propagate` failure
policy does not turn every missing implementation into an accepted feature loss.

Explicit exclusions found have narrower scope:

- `canonical-source-boundary.mec:195–205` records current changing-range and
  logical-mask population limits. It does not declare a general exclusion of
  downstream comprehension consumers.
- `runtime/program/tests.rs:2037`,
  `live_comprehension_membership_is_rejected_before_resident_activation`, checks
  `ReactiveComprehensionStructureUnsupported` on the retiring source route for
  changing generator/filter membership. Canonical S4 already has its distinct
  named-target changing-comprehension behavior. That old route test is relevant
  migration history, not permission to remove canonical behavior or invent a
  concat/transpose milestone exclusion.
- `static-stdlib-composition.md:163–165` selecting `matrixd` establishes dynamic
  storage, not every reactive shape transition; `catalog-closure-gate.md:109`
  makes that evidence limit explicit. Again, implementation/evidence limits do
  not authorize dropping unfinished milestone scope.

### Finite retained work and acceptance

Retain **G18 — resident downstream collection shape/layout capability** as an
unimplemented scope-review item. Its positive witnesses are:

1. `xs := [1 2]; y := [x | x <- xs]; [y y]` returns a 1x4 F64 matrix
   `[1,2,1,2]` through source and decoded artifact on two turns. The current
   implementation fails; the witness is not a passing claim. A closed-cardinality
   compiler specialization can satisfy this particular source only if proved.
2. For `y := [x | x <- signal<[f64]:1,2>, x>0]`, inputs `[1,2]`, `[-1,2]`,
   `[-1,-2]` imply `[y y]` values `[1,2,1,2]`, `[2,2]`, `[]` and shapes
   1x4,1x2,1x0. Transpose implies shapes2x1,1x1,0x1. These explicit positive
   value/shape oracles remain part of the bounded capability proposed for
   acceptance; they are neither silently waived nor implementation approval.
3. Invalid child schemas, incompatible common axes and resource failures reject
   without publication; an explicitly unsupported current target may reject
   safely during its established pure binding/loader-preflight boundary.

The scope review must decide the retained capability against these concrete
sources and named resident/source consumers. Until then it remains visible
unimplemented work, not a completed expected rejection. If an exclusion is
accepted, record its precise target/layout/lifetime and the affirmative decision;
if accepted as required, implement it in the resident layout owner and preserve
G07's separate nested-control construction responsibility. No further production
implementation is authorized by this audit correction.
