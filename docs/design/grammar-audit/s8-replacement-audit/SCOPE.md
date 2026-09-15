# S8 replacement scope at the implementation freeze

## Decision

Keep implementation frozen. The remainder includes missing language families,
wrong-result defects, incomplete product adoption, and missing qualification.
It is not an adapter-only closeout, and the current S8B PR is not a suitable
review boundary: its frozen delta from S8A changes 91 files (+12,364/-953 lines).
A finite inventory is not a claim that every supported combination was tested.

This audit distinguishes **confirmed defects**, **missing prerequisites**,
**expected rejection**, **corrected probes**, and **untested/blocked obligations**.
The latter are named acceptance work, not evidence of a defect. Nothing in this
report authorizes deleting a language family or silently narrowing its contract.

## Frozen identities and evidence

- S8A: `b573284cb4587d4fb0085cc69a80a036830f1b76`.
- S8B implementation: `662d29b79df8ab05a25bbadb941a689fd5bd5aae`.
- C qualification candidate: `e31d08260fac40bcf7982e854cc5d2623a606347`.
- Audit-only tests, fixtures, records, and documents are on this branch. Numeric
  implementation WIP in the original B worktree is excluded.

The initial 364-case run had 294 pass observations and 70 failures. Correcting
23 probe mistakes yields 317 pass observations and 47 failures. These totals
count fixture observations, not capabilities or root causes. The additional statement-body recursion witness brings the final census to 365
probes: 317 pass observations and 48 failures. All are in observations.json. A result with no explicit
expected value proves admission/execution and source/bytecode agreement only.

The existing canonical engine integration targets executed 179 passing tests.
The disabled-operation profile separately executed its one test under `source`
without `math_add`; its zero-test invocation under the full profile is excluded.
Phase 2I semantic certification executed six tests: four passed and two failed
on constant-shape assumptions/fingerprints. The syntax certification adds one
failed authority-import gate (9 pass / 1 fail); all four S7 document certification
tests pass. The failing gate rejects a newly used canonical snapshot import and
requires explicit allowance review, not removal of the authority check. C's six selected canonical consumer
integration targets executed 134 passing tests. C's full runtime library result
is 713 pass / two fail; the two failures are the factorial document and a
comprehension-derived mutable initializer, already owned below. C's engine
library test build fails with 44 references to deleted parser APIs. A successful
isolated WASM build does not qualify its document loader.

## Root-cause register and ownership

`gaps.tsv` is the machine-readable register. `semantic-obligations.tsv` links
every probe to its observation, oracle strength, and one root-cause owner.
A shared failure text alone is not grounds for deduplication. These groups are
based on the responsible branch/owner in frozen source:

| ID | Owner and bounded responsibility | Demonstrated witness / acceptance |
| --- | --- | --- |
| G01 | Engine resident numeric binding: c32 basic arithmetic and reduction | `scalar-c32`, `numeric-c32-{add,sub,mul,div}`, `matrix-c32-{add,sub,mul,div,sum}`, `matrix-repeat-c32`. Basic advertised complex32 operations must execute and preserve type/value through bytecode and subsequent turns. Power/matmul admission is G02. |
| G02 | Core source operation schemes and target capability admission | Same-kind powers for u64/u128, signed integers, c32/c64/r64, and complex/rational matrix products currently lower then fail kernel binding. Reconcile supported signatures with resident factories; unsupported target combinations must reject during planning, before host effects. Do not invent new numeric powers to make probes green. Rational-to-i32 power already passes. |
| G03 | Catalog exposure / ordinary resident operation binding | `catalog-compare-max`, `catalog-compare-min` lower and fail with MissingResidentFactory. Both exports are Internal. Explicitly decide whether ordinary source calls must reject Internal exports or whether these operations require a resident factory; apply the chosen exposure rule consistently, not just to these spellings. |
| G04 | Canonical selected-assignment lowering | `mixed-repeated`, `nested-repeated`. Fused addressed RMW is conditional on same element kind and no remaining selectors; fallback gathers old values then replaces, losing occurrence-ordered accumulation. Preserve the same occurrence contract through promotion and nested paths, including overflow rollback. |
| G05 | Resident activation dependency classification | `match-initializer`, `comprehension-initializer`. Control producers remain turn-only even when all dependencies are available at activation. Classify their dependencies and initialize state once without replaying effects or allowing live-only initializers. |
| G06 | Resident comprehension value storage/binding | `comprehension-i32`, `comprehension-string`, `comprehension-composite`. The resident control layout admits only its primitive subset despite richer artifact schemas. Qualify bindings, captures, and yields for the closed scalar/structural census, including destructuring and rest bindings. |
| G07 | Canonical control-block composition | `comprehension-match`, `comprehension-nested`. Collection bodies accept only ordinary operations and require closed element shapes. Lower nested control into the shared control representation; specify and test shape ownership for nested collections. No special-case source rewrites. |
| G08 | Canonical computed-pattern evaluation | `comprehension-computed-pattern`. Lower an explicit pattern evaluation block with lexical captures and ordered filtering; do not turn arbitrary expressions into literal-only comparisons. |
| G09 | Canonical match-pattern lowering | `match-tuple`. Match lowering accepts scalar literal, wildcard and bind forms, rejecting structural patterns. Qualify tuple/array/tagged patterns, binding scopes, guards, non-match behavior and exhaustiveness. |
| G10 | Canonical semantic declaration environment: kind definitions | `kind-alias`. Collector rejects KindDefine. Resolve aliases through the canonical type environment, including uses in functions and aggregates; diagnose duplicate/cyclic/invalid declarations at retained anchors. |
| G11 | Canonical semantic declaration environment: enums | `enum-declaration`. Collector rejects EnumDefine. Construct nominal enum identity, payload schemas, variant values and pattern use through the same type authority. |
| G12 | Canonical constrained-type semantics | `constrained-kind`. Scalar constraints explicitly reject because there is no constrained schema representation. This is a prerequisite design/implementation decision, not a compiler adapter fix. Define validation and artifact identity before implementation. |
| G13 | Canonical reified kind construction | `reified-inferred-matrix`. A reified matrix kind without dimensions rejects while ordinary dimensionless matrix annotations can infer dimensions from a value. Decide whether unsized reified kinds are valid independently of a value; encode the answer in positive or positioned negative tests. Do not conflate this with the already-supported annotated value case. |
| G14 | Canonical FSM declarations and resident continuation owner | `fsm-pipe` reaches an artifact and fails activation; document FSM declarations are explicitly rejected by the collector. Required acceptance is the specification's FSM state/transition/guard/async semantics, persistence, resource effects and fairness. The old completion ledger's “intentionally unavailable” is not product completion. |
| G15 | Canonical activation-scope semantics | `activation-scope`. Collector rejects ActivationScope. Define activation-arm evaluation/ownership and connect it to the same control/effect owner; syntax recognition alone is insufficient. |
| G16 | ProgramCompiler ordered dependency graph ownership | `ordered_transitive_explicit_root_witness`. Only direct edges to explicit roots become shared live bindings; a transitive path is detached into constant exports. Two explicit roots must share the same dependency instance through intermediates, preserve caller output order, and plan each provider once. Required output is `1`, then `2`, not `1`, then `1`. |
| G17 | Resident shape planning for computed logical reads | `logical-read`. The selected population is not available as an activation shape fact. Support the maintained dynamic-read contract through changing masks and abort/commit, or identify a pre-existing fixed-population input requirement explicitly; never substitute an unused-write fix. |
| G18 | Resident downstream layout for variable-cardinality collection results | `dynamic-concat`. Direct comprehension output works; a downstream concatenation cannot bind its runtime layout. Qualify concat, transpose and nested publication against current cardinality, including empty and changed results. G07 owns constructing nested control; this item owns its consumer layout. |
| G19 | Canonical function body lowering | `pattern-function`, `factorial`. Only FunctionDefineStatements is found by the inliner. Lower match-bodied functions using canonical pattern/control semantics, with typed argument/output binding and exact source anchors. |
| G20 | Canonical recursive function execution | Statement-body recursion witness isolates the inliner's active-function rejection from G19. Recursion cannot be completed by unbounded inlining. Define its bounded execution/continuation contract and test base case, recursion, limits and rollback. |
| G21 | C retirement: compiler/tree/cache authority | Frozen C still exposes tree compiler methods that invoke `plan_artifact_tree_with_services`; interactive `from_tree` and old Program storage survive. Remove these authorities after their responsibility-specific replacements qualify. Mechanical parser deletion alone does not close this item. |
| G22 | C browser document transport and lifecycle | `browser_document_payload_witness` exercises the exact old decoder/type against the canonical producer payload. `interactive_tree` in frozen C ignores candidate source. Migrate document bootstrap, capture, replacement and documentation execution to retained documents and canonical artifacts. Prove real browser loading, changed source, hidden/local outputs, failed replacement and stale transitive dependency rejection. |
| G23 | C build/test retirement closure | Engine `--lib --no-run` fails on deleted parser references. Retarget the finite manifest of tests/examples and remove obsolete source features/dependencies; execute each distribution with nonzero test counts. This is not an invitation to retain a parser shim. |
| G24 | Semantic certification maintenance | `every_semantic_rule_meets_its_required_witness_outcome` and `semantic_evidence_distinguishes_non_wire_shape_values_and_slot_ownership` fail after constant-shape improvements. Rebuild shape-sensitive witnesses that still exercise non-wire shape ownership; update fingerprints only after reviewing actual artifact differences. Also review the failed canonical snapshot-import allowance in syntax certification; preserve the canonical-authority enforcement. |

G03, G12, G13, G14, G15 and G20 contain explicit contract/architecture decisions.
They are named prerequisites with acceptance criteria, not permission to invent
semantics during an S8B fix. Their outcome may be a documented supported-target
rejection only where the language/target contract actually permits that outcome.

## Corrected probes and expected rejections

- Matrix `stats/sum/row([1 2;3 4])` returns `[4 6]`, not `[3;7]`.
  The latter is the column operation. Fourteen false wrong-value observations
  disappear; c32 reduction still fails binding (G01).
- Subtracting 2 from a matrix containing unsigned 1 correctly fails checked
  arithmetic. Five probes now subtract 1 to exercise a valid success domain.
- Bessel jn/yn source schemes use floating inputs. Two probes now use `2.0`.
- Two Internal set export names contain underscores and are not direct lexical
  function spellings. Their maintained operator spellings (`≠`, `⊂`) execute.
- Same-kind rational power is not the maintained rational/integer exponent
  contract. `rational-integral-pow` exercises that valid contract successfully.
  The overbroad source admission remains G02; a missing new power is not inferred.
- Dynamic-to-concrete casting, materialized Empty, negative/out-of-range extents,
  heterogeneous invalid comparisons, non-keyable set/map keys, and unbound
  names/unknown calls remain deliberate positioned rejections under their
  canonical type contracts. Existing semantic rejection suites exercise these
  rules. A rejection caused by a missing family above is not reclassified as a
  user error merely because it is cleanly reported.

## Coverage accounting and explicit open obligations

The inventory crosswalk is the acceptance surface. Each row distinguishes
inventory membership, tested examples, and coverage still owed. In particular:

1. **O01 — catalog overload qualification.** The 120 exports expand to 480 explicitly identified overload/intrinsic rows
   in `catalog-signatures.tsv`. Each export has a
   concrete source/operator witness and its declared type schemes. The floating
   happy-path sample is not overload coverage. For each row, enumerate the
   admissible overload/layout cells from its scheme and target factories; test
   both admitted and rejected cells. Do not multiply cases into independent
   defect tickets. Each failure belongs to its demonstrated owner above or a
   separately reviewed new root cause.
2. **O02 — compiler-specific context qualification.** The 36 public compilation
   entry methods are mapped individually. The 18 simple route probes establish
   only basic dispatch. Existing named tests cover nonempty inputs, live defaults,
   static projections, host planning, resolved revision identity and mixed
   partitioning. Ordered transitive live roots are failing G16. Nonempty context
   coverage must be repeated through each distinct underlying responsibility;
   aliases can share evidence only after their delegation is checked. Cross-root
   function resource context, transitive explicit-root provider counts, and
   rollback after a later-root error are explicit untested cells.
3. **O03 — independent output oracles.** Every fixture without `expected` is
   marked as equivalence-only in the crosswalk. Add mathematical/reference output
   and second-turn state expectations for those fixtures before counting them as
   behavior certification. Numeric domain edge tests must use the type contract,
   not the output from either implementation as their oracle.
4. **O04 — production consumer qualification.** All 27 rows retain their frozen
   positive/negative contracts. The table records actual C route adoption and
   blockers. Prepared native adapter tests do not qualify browser execution.
   Full configured particle and EKF applications must run through CLI, served and
   browser boundaries on the eventual deleted-code candidate; extracted regions
   and kernel benchmarks are insufficient. G22 currently blocks browser document
   routes, G23 blocks the full distribution test build.
5. **O05 — backend/feature matrix.** Repeat affected source-to-artifact activation,
   two-turn publication and rejection/rollback under CPU, SIMD, JIT, native GPU
   and browser WebGPU profiles, plus reduced/no-source/bytecode consumers. Record
   exact head, command, target and nonzero count. Historical runs on other heads
   remain historical evidence. The precise maintained feature matrix is the
   existing `.github/workflows/ci-full.yml`, not a newly invented all-features
   Cartesian product.
6. **O06 — rule and schema semantic closure.** The 80-rule semantic inventory and
   131 S7 disposition rows are accounted for separately from 112 S7 syntax
   witnesses. Syntax/structural membership is not execution. The 17 builtin scalar
   kinds and every SchemaBody variant retain named positive, rejection or blocked
   obligations. G06/G09/G10/G11/G12/G14/G15/G19/G20 block the corresponding semantic
   families. The shape-sensitive certification repairs are G24.

These are six finite qualification work packages with enumerated inventory rows;
they are not six more runtime defects. No suite total is used to hide an untested
cell. The crosswalk deliberately retains those cells until evidence closes them.

## Review boundaries: extract accumulated work before more implementation

See `PR-STACK.md`. Existing B content must first be partitioned by responsibility,
with the union of extracted patches exactly reproducing frozen B. Extraction
changes review boundaries, not implementation behavior. Keep the original B
branch as the immutable comparison/reference until extraction is verified.
No scope is accepted merely because a draft PR exists. Do not resume production
implementation until the prerequisite decisions and the extraction review agree
on the required contracts and the finite acceptance cells.
