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

The initial 364-case record is preserved as a discovery checkpoint. The current
375-case run distinguishes execution observations from contract outcomes. There
are 317 successful execution observations; 15 of those are wrongly admitted
Internal named calls. The contract comparison therefore reports 302 execution matches plus 27 verified current-target rejections, with 46 other unmatched observations before reconciliation of the two invalid original control positives. The 27 rejection matches leave their positive capability obligations open. These are neither capability counts nor independent defect counts.
The 51 ModuleOnly named samples now have imports, and all 17 Internal named
samples require planning rejection. The separate 12-observation visibility test
contains two confirmed canonical admission violations. Every row records its
oracle strength; a value without an independent expectation remains equivalence-only.
The exact-schema suite contains 26 positive identity/binding cases and five separate validation-boundary negatives. All 26 live source/bytecode paths pass; 23 constant-bound bytecode paths fail under G25 and Dynamic constant identity fails under G26. Bool/String and all five actual validation-boundary negatives pass. These results are separate from the source census.

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
| G02 | Configured numeric/layout capability scope and physical target owners | Four finite unimplemented capability families remain: c32 basic/reduction/updates (11 source manifestations), powers (10), complex/rational matmul (3), and f32 special-binary broadcast (1). Pure compile_document may correctly produce an artifact whose configured target rejects at activation; production loading already preflights before installation/effects. These observations do not prove a compiler preflight bug. They also do not prove an accepted milestone exclusion: positive capability witnesses stay open pending explicit scope acceptance. See the 46 target cells for current rejection and milestone coverage as separate axes. |
| G03 | Canonical named-call environment and catalog exposure | The existing FunctionEnvironment contract is definitive: Internal exports are syntax-only, ModuleOnly names require imports, Prelude names are visible. The dedicated paired shipping/canonical visibility test confirms canonical wrong admission for Internal compare/max and unimported math/cos, with successful imported, aliased, Prelude and operator controls. All 17 direct Internal catalog samples are negative obligations; adding resident factories would not fix the violated contract. |
| G04 | Canonical selected-assignment lowering | `mixed-repeated`, `nested-repeated`. Fused addressed RMW is conditional on same element kind and no remaining selectors; fallback gathers old values then replaces, losing occurrence-ordered accumulation. Preserve the same occurrence contract through promotion and nested paths, including overflow rollback. |
| G05 | Resident activation dependency classification | `match-initializer`, `comprehension-initializer`. Control producers remain turn-only even when all dependencies are available at activation. Classify their dependencies and initialize state once without replaying effects or allowing live-only initializers. |
| G06 | Resident comprehension retained value storage/binding | i32, String and composite comprehension bindings/yields fail in the resident control storage owner. Existing primitive tuple/array destructuring and compound match-result publication already pass and are preserved. Qualify exact schema/value identity for retained bindings, captures and yields using the closed schema cells; do not reimplement working destructuring. |
| G07 | Canonical control-block composition | `comprehension-match`, `comprehension-nested`. Collection bodies accept only ordinary operations and require closed element shapes. Lower nested control into the shared control representation; specify and test shape ownership for nested collections. No special-case source rewrites. |
| G08 | Canonical computed-pattern evaluation | `comprehension-computed-pattern`. Lower an explicit pattern evaluation block with lexical captures and ordered filtering; do not turn arbitrary expressions into literal-only comparisons. |
| G09 | Canonical match-pattern lowering | `match-tuple`. Match lowering accepts scalar literal, wildcard and bind forms, rejecting structural patterns. Qualify tuple/array/tagged patterns, binding scopes, guards, non-match behavior and exhaustiveness. |
| G10 | Canonical semantic declaration environment: kind definitions | `kind-alias`. Collector rejects KindDefine. Resolve aliases through the canonical type environment, including uses in functions and aggregates; diagnose duplicate/cyclic/invalid declarations at retained anchors. |
| G11 | Canonical semantic declaration environment: enums | `enum-declaration`. Collector rejects EnumDefine. Construct nominal enum identity, payload schemas, variant values and pattern use through the same type authority. |
| G12 | Canonical constrained-type contract prerequisite | The grammar admits scalar range constraints; earlier code discarded them and the canonical frontend explicitly rejects represented constrained annotations. Six named design decisions in CONTRACT-DECISIONS-TYPES.md cover meaning, domains, evaluation, identity/transport, enforcement and type relations. This is the remaining unresolved language-contract prerequisite, not permission to erase constraints or claim permanent rejection completes the family. |
| G13 | Canonical dimensionless reified-kind lowering | The specification already permits dimensionless matrix kinds, and ReifiedKind already carries declared dimension parameters. Lower the two independent axes through that representation; assert bounds/lifetime, closed kind structure, exact identity and bytecode. No external matrix value or new wire authority is required. |
| G14 | Canonical FSM declaration and resident continuation owner | Declared increment and named-input FSM witnesses pass syntax/index and hit the explicit FsmSpecification lowering rejection. FSM01-FSM13 name the required declaration, transition, capture, persistence, async timing, fairness, limits and rollback contracts already specified by the language. The older undeclared fsm-pipe sample is not a positive machine-execution witness. |
| G15 | Canonical activation-scope lowering and runtime lifecycle | Valid stable-trigger and patterned-trigger witnesses hit the explicit ActivationScope lowering rejection. ACT01-ACT10 preserve the existing activation edge, arm, capture, ownership and rollback contracts. The original expression-trigger sample is an invalid positive expectation. Context sends inside activation scopes remain excluded by the frozen v0.4 gate contract; do not add them to this scope. |
| G16 | ProgramCompiler ordered dependency graph ownership | `ordered_transitive_explicit_root_witness`. Only direct edges to explicit roots become shared live bindings; a transitive path is detached into constant exports. Two explicit roots must share the same dependency instance through intermediates, preserve caller output order, and plan each provider once. Required output is `1`, then `2`, not `1`, then `1`. |
| G17 | Computed logical-mask shape capability and activation-fact responsibility | The direct logical-read witness supplies no ActivationFacts and receives UnresolvedShape; existing Q09 explicitly requires a population fact and preserves a fixed-population contract. Pure ProgramCompiler is not required by the inspected API contract to infer arbitrary masks. Closed/computed mask admission remains a finite shape-capability/production-boundary obligation for scope review, not a demonstrated compiler defect or a silently accepted exclusion. Changing mask population remains explicitly outside the current fixed-population contract. |
| G18 | Resident downstream comprehension layout capability | Direct comprehension publication exists, while concat/transpose bind fixed dimensions. Current rejection of an unresolved downstream shape is permitted and does not demonstrate missing preflight. The S8 coverage plans do not establish an accepted exclusion of this composed capability; retain the 1x4 and changed-cardinality positive value/shape witnesses with the resident layout owner for scope review. Correct current rejection does not complete this work. |
| G19 | Canonical pattern-function body lowering and collection lifting | Pattern-bodied calls reject before execution; valid broadcast, ordered first-match and partial-match probes isolate this owner. FUN01-FUN06 retain typed binding, lexical formulas, homogeneous matrix/set lifting and ordered branch behavior. Functions permit a partial branch set and report no matching output at runtime; do not import ordinary match exhaustiveness unchanged. Recursion probes that hit this rejection are blocked here, not independent G20 evidence. |
| G20 | Canonical bounded recursive call execution | The statement-recursion witness isolates active-function rejection from pattern-body lowering. Recursion and collection lifting are already required by the specification; implementation needs bounded call/continuation storage, not unbounded inlining. FUN07-FUN10 enumerate base/branch/tail behavior, limits, value capture and rollback; pattern-bodied cases also depend on G19. |
| G21 | C retirement: compiler, interactive and module-index tree authority | Frozen C still exposes tree compiler methods calling plan_artifact_tree_with_services, interactive from_tree/Program storage, and module-index preference for an optional Program tree. Replace each responsibility after its named compiler/consumer cells qualify. Parser deletion alone cannot close those competing authorities. |
| G22 | C browser document transport and lifecycle | `browser_document_payload_witness` exercises the exact old decoder/type against the canonical producer payload. `interactive_tree` in frozen C ignores candidate source. Migrate document bootstrap, capture, replacement and documentation execution to retained documents and canonical artifacts. Prove real browser loading, changed source, hidden/local outputs, failed replacement and stale transitive dependency rejection. |
| G23 | C build/test retirement closure | Engine `--lib --no-run` fails on deleted parser references. Retarget the finite manifest of tests/examples and remove obsolete source features/dependencies; execute each distribution with nonzero test counts. This is not an invitation to retain a parser shim. |
| G24 | Semantic certification maintenance | `every_semantic_rule_meets_its_required_witness_outcome` and `semantic_evidence_distinguishes_non_wire_shape_values_and_slot_ownership` fail after constant-shape improvements. Rebuild shape-sensitive witnesses that still exercise non-wire shape ownership; update fingerprints only after reviewing actual artifact differences. Also review certification_evidence_uses_only_canonical_authorities snapshot-import allowance; keep enforcement. |
| G25 | Canonical constant binding and artifact schema canonicalization | Binding a detached Value imports a schema table with preserved IDs; canonical artifact emission retains that noncanonical order, and its bytecode decoder rejects NonCanonicalSchemaId. 23 exact scalar/structural cases share this cause. Direct live source/bytecode publication succeeds. Recanonicalize schema identity and all references at the owning artifact boundary; verify exact bound-source/bytecode value identity and independently owned schema tables. |
| G26 | Canonical Dynamic constant binding identity | bind_input_constants unconditionally wraps a supplied Value for a Dynamic target, even when that Value is already Dynamic. The bound source artifact changes its exact value hash by adding a second wrapper; live source and decoded publication preserve the correct single wrapper. Preserve already-Dynamic identity while wrapping a bare payload once; test both forms and changed payload schemas. This is independent of G25 schema-order decoding. |

G12 contains six unresolved constrained-type design decisions. G02/G17/G18 also retain explicit target-capability scope acceptance: correct current rejection cannot close their positive milestone obligations. G03/G13 and
FSM/activation/function requirements are already constrained by repository
contracts, as reconciled in CONTRACT-DECISIONS-TYPES.md and
CONTRACT-DECISIONS-CONTROL.md. Their missing execution/storage designs are bounded
implementation prerequisites; they are not invitations to silently narrow the
language or turn all missing kernels into new language work.

## Corrected probes and expected rejections

- Matrix `stats/sum/row([1 2;3 4])` returns `[4 6]`, not `[3;7]`.
  The latter is the column operation. Fourteen false wrong-value observations
  disappear; c32 reduction still fails late binding (the same G02 target-admission boundary).
- Subtracting 2 from a matrix containing unsigned 1 correctly fails checked
  arithmetic. Five probes now subtract 1 to exercise a valid success domain.
- Bessel jn/yn source schemes use floating inputs. Two probes now use `2.0`.
- Two Prelude set export names contain underscores and are not direct lexical
  function spellings. Their maintained operator spellings (`≠`, `⊂`) execute.
- The current target implements scalar r64/i32 power; `rational-integral-pow`
  exercises it successfully. Same-kind rational power remains an unimplemented
  CAP-POWER capability with its positive witness retained. Semantic admission,
  current target rejection and milestone scope are separate questions.
- Dynamic-to-concrete casting, materialized Empty, negative/out-of-range extents,
  heterogeneous invalid comparisons, non-keyable set/map keys, and unbound
  names/unknown calls remain deliberate positioned rejections under their
  canonical type contracts. Existing semantic rejection suites exercise these
  rules. A rejection caused by a missing family above is not reclassified as a
  user error merely because it is cleanly reported.

## Finite acceptance accounting

The six qualification packages retain concrete cells. An unrun cell is named
acceptance work, never an implementation pass or an extra root cause.

| Package | Exact worklist and evidence boundary |
| --- | --- |
| O01 catalog | 480 candidate rows / 120 names / 34 shared families in catalog-acceptance-cells.tsv. Each row has its exact candidate layout, kind domain, target domain, source exposure/recipe, boundary cells and independent oracle. Candidate-specific resolution must identify the actual candidate; an export sample cannot certify competing overloads. |
| O02 compiler/frontend | compiler-acceptance-cells.tsv: 53 responsibility cells covering 36 public compiler methods, 18 CanonicalSourceFrontend methods, six CanonicalSourceProgram methods and three internal compiler entrances. Existing named tests, observed failures and 14 precise untested obligations are separate. Nonempty input, transitive provider, context ownership, options identity and later-root rollback cells are explicit. |
| O03 output identity/oracles | semantic-obligations.tsv labels every explicit two-turn value, negative planning contract and equivalence-only sample. schema-acceptance-cells.tsv adds 17 exact scalar identity and nine structural generic-Value probes plus existing codec/live evidence. RuntimeHostInputValue has 20 admitted input forms but only four returned/default forms; P29 specifies an unrun census of that actual asymmetry instead of assuming all schemas use this adapter. |
| O04 production consumers | consumer-acceptance-cells.tsv: 54 positive/negative cells across the frozen 27 contracts, actual production boundary, executable recipe, expected policy, exact-head evidence and blocker. 24 recorded passes, five partial, nine untested and 16 blocked are distinct from 134 passing prepared-adapter tests. Complete configured applications and actual browser publication remain required. |
| O05 configured backends | The maintained workflow matrix is .github/workflows/ci-full.yml, not an invented all-features Cartesian product. Each affected acceptance cell must retain its CPU/SIMD/JIT/native GPU/browser WebGPU and reduced/no-source/bytecode execution or explicit configured rejection at the final head. Extraction checks are recorded separately and cannot substitute for final distribution qualification. |
| O06 grammar/control/schema | rule-crosswalk.tsv and rule-acceptance-links.tsv account for 80 Phase2I rules plus 131 S7 dispositions, separately from 112 S7 syntax witnesses. control-acceptance-cells.tsv names 33 FSM/activation/function/recursion cells. schema-acceptance-cells.tsv names 64 cells covering every SchemaBody variant. Structural membership, artifact codec and live exact value publication remain distinct proofs. |

The ledgers make the remaining test responsibilities enumerable without claiming
an exhaustive Cartesian product of infinite source programs. Recursive owners
retain their existing resource/property tests. A new observed root cause must
amend this register and its owning corrective boundary before production work;
it must not silently expand S8B.

## Review boundaries: extract accumulated work before more implementation

See `PR-STACK.md`. Existing B content must first be partitioned by responsibility,
with the union of extracted patches exactly reproducing frozen B. Extraction
changes review boundaries, not implementation behavior. Keep the original B
branch as the immutable comparison/reference until extraction is verified.
No scope is accepted merely because a draft PR exists. Do not resume production
implementation until the prerequisite decisions and the extraction review agree
on the required contracts and the finite acceptance cells.
