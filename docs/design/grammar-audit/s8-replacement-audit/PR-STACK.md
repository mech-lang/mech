# Restoring review boundaries without changing implementation

Implementation remains frozen. This is an extraction and corrective PR specification,
not a claim that the proposed intermediate heads already build. The frozen B reference
must remain available. Do not turn the audit tests into a fix-by-fix implementation queue.

## First stack: extract the implementation already accumulated in B

Extract from S8A `b573284cb` to B `662d29b79`. The patch ownership ledger accounts for
all 91 changed paths. Shared files require responsibility-specific hunk extraction;
assigning the entire 1,802-line compiler delta to one PR would defeat the split.

| PR / proposed branch | Review boundary | Required evidence before requesting implementation approval |
| --- | --- | --- |
| E1 `codex/syntax-s8e1-config-syntax` | Restricted canonical configuration evaluator; numeric/literal and typed document syntax corrections supporting that evaluator. Preserve precise source errors and raw input. | Canonical config profiles, malformed/unknown field diagnostics, canonical rule certification. No general evaluator added to configuration. |
| E2 `codex/syntax-s8e2-values` → E1 | Existing canonical type/shape/value improvements, exact scalar change detection, resident arithmetic/selection primitives, output projection and artifact helpers. | Source/bytecode value, shape, composite memory and selected-assignment tests. Existing deficiencies G01/G02/G04/G05/G17/G18 remain labeled failing, not silently fixed during extraction. |
| E3 `codex/syntax-s8e3-document-semantics` → E2 | Existing document execution, local statement-bodied functions, declaration imports, document output identity and assignment lowering. | Retained document/state/scope/invariant tests and explicit function-binding tests. Missing function patterns/recursion, nominal declarations and control families remain upstream prerequisites. |
| E4 `codex/syntax-s8e4-compiler-planning` → E3 | ProgramCompiler ordinary/document/artifact/input/default/static APIs and shared resource planning. | Nonempty planning values, live defaults, grants, preflight failure, effect-free compilation, detached static results and projection. The 42-only dispatch probe is supplemental. |
| E5 `codex/syntax-s8e5-graph-interactive` → E4 | Resolved/rooted/ordered graph compilation plus accepted-candidate interactive lifecycle. | Root revision/options, import/export scope, source replacement/reset/clear, failed candidate preservation, provider ownership. G16 remains an explicit failing contract. |
| E6 `codex/syntax-s8e6-mixed-compute` → E5 | Mixed partitioning, activation initializer IR, backend output/readback and retained compute values, complete-source native/browser compute helpers. | Actual configured particle/EKF applications plus CPU/SIMD/JIT/GPU activation, publication and rejection. Region-only tests cannot seal this PR. |
| E7 `codex/syntax-s8e7-web-handoff` → E6 | Canonical bundle identity, transitive dependency stamps, configured offline compilation and prepared server/browser presentation. | Producer envelope/identity tests; exact remaining C browser transport blockers explicitly listed. No AST adapter. |
| E8 `codex/syntax-s8e8-qualification` → E7 | Existing CI wiring, ledger and generated-fixture reconciliation that crosses the above boundaries. | Union-of-patches check against frozen B, every intermediate build/profile needed for review, exact final-head results, and no test silently executing zero cases. |

The proposed branch names are reserved scope labels, not claims that remote PRs exist.
Creating the branches/draft PRs and moving #828 to an umbrella/reference role is the
next repository-only action after the extraction layout is reviewed. No semantic fixes
are prerequisites to that extraction. If an intermediate build requires another hunk,
move that prerequisite hunk into its owning earlier PR; do not add a shim or change
behavior. Record moves in the ownership ledger.

Extraction completion is executable: compare the final extracted tree to frozen B
(excluding audit-only files), and run each PR's stated dependency/profile checks.
An arbitrary chronological cut through the current commits is insufficient because
those commits interleave syntax, semantics, compiler, backend and browser changes.

## Second stack: explicit corrective prerequisites

These are new corrective PR scopes, each targeting the preceding prerequisite after
extraction. They may be authored independently, but a single dependency order keeps
qualification unambiguous. This table is an acceptance plan, not authorization to
implement its undecided language contracts.

| PR | Gap IDs | Closed scope and acceptance boundary |
| --- | --- | --- |
| R1 numeric contracts | G01, G02, G03 | c32 basic resident support; source/target signature admission; catalog exposure and missing min/max binding decision. Enumerate the supported signatures before editing kernels. All 17 scalar kinds remain in the matrix. |
| R2 selected updates and dynamic reads | G04, G17 | Occurrence-ordered RMW through mixed and nested selections; logical-read population/shape ownership. Validate noncommutative updates, repeats, masks, changing shapes, overflow and abort preservation. |
| R3 activation and variable-cardinality consumers | G05, G18 | Static control-backed initializer scheduling and runtime downstream collection layout. No general control-language expansion. Source/bytecode activation, two turns, empty result and state rollback. |
| R4 structural control values | G06, G09 | Closed scalar/composite control bindings/yields and structural match patterns. Include named rest/destructuring, guard bindings and codec/publication identity. |
| R5 composed and computed control | G07, G08 | Nested match/comprehension composition and executable computed patterns. Shared block/capture/shape ownership; lexical scope and partial-match rejection. |
| R6 nominal declarations | G10, G11 | Canonical alias/enum environment and nominal identity. Declarations, uses, imports, duplicate/cycle errors and variant patterns. |
| R7 constrained/reified types | G12, G13 | First write the constrained-schema and unsized reified-kind contract. Then implement or explicitly reject only what that reviewed contract excludes. No opportunistic special case. |
| R8 callable function semantics | G19, G20 | Pattern bodies plus a reviewed bounded recursion execution model. Depends on R4/R5. Factorial is a canary, not the entire acceptance surface. |
| R9 FSM/activation semantics | G14, G15 | A separately reviewed continuation/effect lifecycle design followed by declaration, transition, guard, async/fairness, activation and persistence implementation. Depends on the same control/value owners. This substantial prerequisite does not belong inside compiler adapter work. |
| R10 ordered graph correctness | G16 | Shared identity through transitive paths to explicit roots, caller output order, once-only provider planning, and later-root rejection rollback. |
| R11 real browser cutover | G22 | Replace actual document bootstrap/loader/REPL paths, then run complete bundle/served/browser positive and negative contracts, including stale dependency text. |
| R12 retirement and certification | G21, G23, G24 | Delete old tree compiler/cache authority; retarget the enumerated test/example/dependency manifest; repair certification assumptions; run final exact-head matrix and all 27 contracts. |

These 12 scopes are not an estimate of 12 equally small fixes. R7/R8/R9 require
acceptance/design decisions; R4/R5 span real semantic families. Until those decisions
and the extraction layout are reviewed, the scope is **not accepted as bounded enough
to resume implementation**. The six qualification packages O01–O06 remain explicit
work attached to these PRs; they cannot be waived by passing the observational harness.

Review order stays: implement an accepted scope, validate its required witnesses,
reply to every relevant note, resolve addressed threads, then request another review.
CI may run asynchronously, but no claimed seal may refer to a different head.
