# Review boundaries for existing work and corrective prerequisites

The original production comparison remains frozen at `662d29b79`. The first stack
moves existing code into review boundaries; it adds no semantic fixes. The
corrective plan is now accepted as a recovery baseline; current implementation
and still-required scope decisions are tracked in [RECOVERY-STATUS.md](RECOVERY-STATUS.md).
The original B branch/PR #828 remains the immutable comparison reference.

## Existing B extraction stack

The original proposed eight slices were revised after inspecting actual function
and feature dependencies. The resulting eleven boundaries are below. Large shared
files are extracted by complete existing function/hunk ownership, not copied into
one catch-all compiler PR. Exact dependency closure is in EXTRACTION-DEPENDENCIES.md.

| Slice | Branch / head / PR | Existing code ownership | Recorded validation or required check |
| --- | --- | --- | --- |
| E1 configuration | `codex/syntax-s8e1-config-syntax` / `8be3118e1` / [#831](https://github.com/mech-lang/mech/pull/831) | Restricted canonical configuration preparation, positioned diagnostics, its tests and feature gate; nine files. | 45 canonical + 39 public configuration tests pass. |
| E2 syntax | `codex/syntax-s8e2-syntax` / `89403ba4b` / [#832](https://github.com/mech-lang/mech/pull/832) | Existing canonical document/table/streaming corrections; 14 files. Retirement enforcement remains E11. | 46 tests pass across document roots, review regressions, S7 certification and streaming partitions. |
| E3 resident primitives | `codex/syntax-s8e3-resident-values` / `732208864` / [#833](https://github.com/mech-lang/mech/pull/833) | Addressed numeric contracts, resident arithmetic, population shape construction and exact scalar tokens; four files. | Engine full source/compiler resident-artifact check passes. Semantic emitters land in E4. |
| E4 semantic frontend | `codex/syntax-s8e4-semantic-frontend` / `7d0db6ede` / [#834](https://github.com/mech-lang/mech/pull/834) | Canonical documents/functions/imports/assignments/projections, type/shape and artifact helpers, prepared EKF caller; 18 files. Projection refresh API remains E7. | All 179 tests in seven canonical engine integration targets pass. |
| E5 ordinary compiler | `codex/syntax-s8e5-compiler-planning` / `0c8a95794` / [#835](https://github.com/mech-lang/mech/pull/835) | Document/artifact/input/default/static APIs, resource planning/preflight and output identity; complete ordinary methods plus their existing tests. | 21 existing canonical compiler tests pass; compute-disabled build check passes. |
| E6 graph compiler | `codex/syntax-s8e6-graph-planning` / `ef30e7e81` / [#836](https://github.com/mech-lang/mech/pull/836) | Rooted/resolved/ordered methods, canonical import handoff, graph tests and complete five-route provider test. | 695 runtime library and 10 declaration-handoff tests pass. G16 remains an explicit known audit failure. |
| E7 interactive lifecycle | `codex/syntax-s8e7-interactive` / `634f45620` / [#837](https://github.com/mech-lang/mech/pull/837) | Retained candidate submit/replace/reset/clear, projection refresh API and its runtime query caller together. | 19 interactive lifecycle tests pass. |
| E8 mixed compiler | `codex/syntax-s8e8-mixed-planning` / `2a091a128` / [#838](https://github.com/mech-lang/mech/pull/838) | Mixed document/rooted APIs, source dependency metadata in every constructor, activation input capture and canonical compute port decoding. | 713 runtime library tests pass; exact manifest preserves 134 existing helper/test bodies and extracts seven mixed tests. |
| E9 compute execution | `codex/syntax-s8e9-compute-execution` / `199b42d5d` / [#839](https://github.com/mech-lang/mech/pull/839) | ComputeActivationValues, remainder IR, fixed-shape publication storage and exhaustive GPU/SIMD/JIT consumers, retained readback and owned backend tests. | Native/JIT GPU crate check passes; activation and publication rollback regressions each pass. GPU adapter execution and browser JS are not certified. |
| E10 web handoff | `codex/syntax-s8e10-web-handoff` / `68eeb8828` / [#840](https://github.com/mech-lang/mech/pull/840) | Bundle identity/dependencies, browser/server planning and presentation, CLI/WASM mixed adapters, public source path helper and JS/smoke tests. | 61 bundle-web, 34 document-render and one runtime bundle test pass. JavaScript unrun locally; G22 still blocks actual browser document-loader closure. |
| E11 qualification | `codex/syntax-s8e11-qualification` / `75ae76bf7` / [#841](https://github.com/mech-lang/mech/pull/841) | Remaining CI, manifest, retirement enforcement and documentation changes from frozen B. | Seven-file extraction manifest and complete tree equality to frozen B pass. Existing G24 certification findings remain open. |

All eleven slices are published draft PRs (#831–#841), each targeting its preceding
branch. Exact heads, extracted spans and completed checks are recorded in
extraction-results.json. No row is a seal claim. Redundant lower extraction CI is
canceled so E11 can use runners; local intermediate validation remains recorded.

The final extraction passes `git diff --exit-code 662d29b79 75ae76bf7`: the complete
E11 tree exactly reproduces frozen B across all 91 changed paths.
Audit-only files live on the separate audit branch and therefore require no
production-path exclusion. If a compile fails, move the missing existing
prerequisite into its owning slice; do not change implementation or weaken tests.
Each intermediate head also retains its exact copied symbol/hunk manifest.

## Corrective stack after scope acceptance

The earlier twelve combined scopes hid independently reviewable owners. The
following table contains the original 23 boundaries and the three subsequently
demonstrated corrections, R24 through R26. These keep the responsibilities
separate. IDs refer to the stable deduplicated gap register; fixture repetitions
never create extra PRs.
Existing implementations that already satisfy a cell remain in place.

| Boundary | Gap | Owning layer and finite acceptance |
| --- | --- | --- |
| R01 named visibility | G03 | Canonical named-call environment. Internal negatives, imported/unimported ModuleOnly, aliases and Prelude positives across the closed 120-name census. Preserve operator IDs independently. |
| R02 numeric target capability | G02 | Settle the explicit target capability floor for CAP-C32, CAP-POWER, CAP-MATMUL and CAP-F32-BINARY. Preserve 25 positive witnesses and correct current-target rejection tests separately. Any accepted kernel work belongs to these physical owners; pure artifact compilation is not incorrectly forced to execute/preflight. No automatic scope exclusion. |
| R03 bound schema identity | G25 | Canonical constant binding/artifact schema remapping. Independently owned scalar/structural Value inputs retain canonical IDs and exact source/bytecode values; all referenced schemas and dynamic descendants are remapped consistently. |
| R04 Dynamic binding identity | G26 | Canonical constant binder. Already-Dynamic values retain one wrapper and exact identity; bare payloads wrap once. Test changed payload schemas and bytecode independently of G25. |
| R05 selected updates | G04 | Canonical addressed RMW. Mixed and nested repeated occurrences, noncommutative updates, promotions and atomic failure. |
| R06 logical activation facts | G17 | Reconcile actual loader/input-fact responsibilities for closed/computed masks, then implement only the accepted shape capability. Keep no-fact rejection and same-population positive behavior separate; changing population retains its explicit exclusion. |
| R07 control initializers | G05 | Resident activation dependency classification. Closed match/comprehension producers initialize once; live-only dependencies and effects do not replay. |
| R08 variable-cardinality consumers | G18 | Retain finite downstream concat/transpose 1x4 and changing-cardinality positive witnesses for target scope acceptance; implement the accepted layout capability under its resident owner. Correct current fixed-target rejection is not a seal. |
| R09 comprehension storage | G06 | Resident retained binding/capture/yield values across the closed scalar/structural cells. Preserve working primitive destructuring. |
| R10 structural match patterns | G09 | Canonical structural scrutinee, tuple/array/tag bindings, guards and match exhaustiveness. Preserve already-working compound arm outputs. |
| R11 composed control | G07 | Canonical nested match/comprehension block and shape ownership. No source rewriting by special case. |
| R12 computed patterns | G08 | Canonical lexical pattern evaluation blocks, captures, ordered filtering and rejected partial matches. |
| R13 nominal declarations | G10, G11 | Canonical type/declaration environment for aliases and enums, nominal identity, imported uses, duplicate/cycle errors and payload patterns. |
| R14 reified matrix kinds | G13 | Canonical dimensionless kind lowering into the existing declared dimension environment; exact closed-kind/bytecode identity and bounds. |
| R15 constrained-type contract | G12 | Review the six explicit decisions in CONTRACT-DECISIONS-TYPES.md before any constrained-type implementation. Acceptance pairs cover boundaries/domains/enforcement sites; silent erasure is excluded. |
| R16 pattern functions | G19 | Canonical callable pattern bodies, ordered first match, partial-match runtime failure, lexical formulas and homogeneous matrix/set lifting. Depends on control/value owners above. |
| R17 recursive calls | G20 | Bounded canonical runtime call/continuation storage; FUN07–FUN10 base/branch/tail/capture/limit/rollback cells. Recursion is already required, not a new language decision. |
| R18 FSM continuation | G14 | FSM01–FSM13 specified declarations, transition/capture/persistence, async timing/fairness and rollback. This is a semantic prerequisite outside compiler adapters. |
| R19 activation scopes | G15 | ACT01–ACT10 stable/pattern triggers, activation edge ownership, captures and rollback. Preserve the frozen v0.4 exclusion of context sends inside scopes. |
| R20 ordered graph identity | G16 | ProgramCompiler shared transitive explicit roots, caller output order, once-only provider planning and later-root failure rollback. |
| R21 authority retirement | G21 | Replace remaining tree compiler/cache/interactive/module-index authorities after their responsibility-specific cells qualify. No parser shim. |
| R22 browser adoption | G22 | Actual retained-document bootstrap, replacement/capture and REPL/documentation loading; served/bundled complete applications and negative stale/error cases. |
| R23 distribution closure | G23, G24 | Retarget deleted-parser tests/examples, reviewed certification assumption/hash updates, exact final feature matrix and all54 production consumer cells. |
| R24 Index ranges | G27 | Resident range cardinality, physical binding and execution for Index endpoints; exclusive/inclusive modes, increments, constant endpoints, portable bounds/overflow, and preserved live-endpoint rejection. Strict source/decoded witness and diagnosis are in [RECOVERY-FINDINGS.md](RECOVERY-FINDINGS.md#g27--index-range-resident-prerequisite-proposed-r24). |
| R25 compute read planning | G28 | Canonical mixed compiler stages compute/interface construction before coordinator sample/telemetry read planning. Scalar/nonsquare shapes and bytecode schemas, telemetry types, invalid paths, ordinary-provider ownership, and preserved imports/initializers; C separately proves configured browser source replacement. Details are in [RECOVERY-FINDINGS.md](RECOVERY-FINDINGS.md#g28--compute-output-schema-planning-precedes-its-interface-r25). |
| R26 canonical compute lowering | G29 | Canonical artifact-to-portable-kernel lowering for the shipped fixed-shape EKF region. Reconcile canonical operation IDs and selector/index storage with the fixed-shape lowerer, preserve transactional integrity predicates and derived-value materialization, and pass the maintained CPU/WebGPU EKF oracle without source rewriting or parser restoration. Details are in [RECOVERY-FINDINGS.md](RECOVERY-FINDINGS.md#g29--canonical-ekf-artifact-does-not-lower-to-the-portable-compute-kernel-proposed-r26). |


## Qualification ownership across both stacks

[The ownership crosswalk](qualification-ownership.tsv) assigns every compiler,
schema, rule, control, consumer, catalog, target and gap acceptance cell to one
primary boundary above. Its 341 relations include all 480 catalog candidates in
34 groups and all 18 compiler/schema constructions that still lack exact tests.
[Qualification ownership](QUALIFICATION-OWNERSHIP.md) explains supporting owners
and the explicit links for all 115 O03 observations. Missing tests belong to their
responsibility's E/R boundary; R23 integrates the final matrix. The linked ledgers
remain the authority for evidence and expected behavior. Run
`python3 docs/design/grammar-audit/s8-replacement-audit/verify-qualification-ownership.py`
to check complete membership and links without executing behavioral tests.

These are review boundaries, not an estimate that each takes one small patch.
Nominal declarations, constrained types, recursive execution and FSM continuation include substantial prerequisite work. The concrete coverage
ledgers attach finite positive, rejection and blocked obligations; they are not
waived by observational harness totals. New demonstrated root causes must amend
the register and owning boundary before implementation can resume. The original B comparison remains frozen.
Scope-dependent work still requires its explicit decisions; confirmed corrections proceed in separate PRs.

After scope acceptance the operating procedure remains: implement the bounded
slice, validate, reply to all relevant notes and resolve addressed threads, then
request another review. CI may run asynchronously; a seal requires its exact head.
