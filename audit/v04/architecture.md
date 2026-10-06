# Mech v0.4 architecture and capability audit

Baseline: `c4777b7015fe8ff47fdfa48d18606c49aace7d97`, isolated audit checkout. This report describes the baseline. Each capability record identifies its inspection and execution evidence. Machine-readable records are in `data/capabilities.json`, `data/findings.json`, `data/subtraction.json`, and `data/review-coverage.json`. Line numbers below refer to this baseline. The census and extraction reports provide physical accounting, with each responsibility counted once across crate boundaries.

## Responsibility map

| Authority | Owners | Reason for the boundary |
|---|---|---|
| Revisioned source, lossless tree, recovery, diagnostic anchors, editor identities | `src/syntax/src/document` | Source remains UTF-8 text. Runtime allocation and operation selection belong to later execution stages. |
| Canonical schemas, values, kind inference, operation contracts, physical compatibility and managed storage | `src/core` | Stable semantic and ownership contracts are shared by compiler, resident execution and library implementations. Subsystem classification follows the several responsibilities within this crate. |
| Typed source lowering and immutable program artifacts | `src/engine/src/source_semantics`, `src/engine/src/artifact` | Strict syntax becomes semantic inputs, state, operations, schemas and constants; artifacts can exist independently of whether a selected resident target can activate them. |
| Resident planning and execution | `src/engine/src/resident/general`, `src/engine/src/memory_planner` | Activation selects concrete implementations and plans storage. Prepared turns own candidate state before accepted publication. The resident EKF control fixture has a separate benchmark/conformance responsibility. |
| Product compilation, resource/capability admission, transaction ordering, scheduling and effects | `src/runtime` | `ProgramCompiler` compiles; `MechRuntime` loads and executes; `ResidentReplSession` owns interactive state and replacement. Frontends adapt these owners. |
| Portable compute interface and backend selection | `src/compute` | Backend-neutral IR, ports, placement, capability descriptors and completion contracts. Dispatch evidence requires an observed completion through the backend contract. |
| Concrete CPU/wgpu execution and compute host integration | `hosts/gpu` | Scalar/SIMD/JIT/wgpu implementations, physical resources and completion handling. Stable application source paths currently use scalar CPU or wgpu; fixed-shape SIMD/JIT are narrower experimental paths. |
| Bytecode transport, native product construction, public ABI | `src/bytecode`, `src/build`, `src/abi` | Serialization and product closure construction are distinct from live resident ownership. ABI pointers have call-scoped external invariants. |
| Distribution selection and machine operations | `src/stdlib`, `machines/*` | Composition and machine implementations are extraction candidates. Engine resident arithmetic is another implementation path retained in Mech, alongside canonical types, language intrinsics and publication authorities (F-007). |
| Native CLI, WASM and browser providers/controllers | root entry points, `src/wasm`, `hosts/*` | Product adapters connect inputs, execution and presentation. The observed architecture hosts the Mech runtime in browser WASM. |

The maintained manifest separates runtime, source and compiler profiles. `tests/architecture/distributions/profile-contracts.json` records selected/full catalog closures, required and forbidden packages, feature layers and catalog digests. The audit compares build measurements against these checked-in expectations. In particular, runtime-only profiles forbid syntax/compiler dependencies. Native and WASM closures require their own builds; workspace-wide feature unification can conceal missing forwarding.

## Historical requirements and current scope

The comparison below uses ancestors of the preserved baseline, read directly from git. The reviewed sources are the listed architecture documents and their ancestor revisions. Additional issue discussions and off-repository decisions remain outside this review.

| Requirement origin | Baseline observation | Disposition |
|---|---|---|
| `63414c821`, incremental architecture §8: edits select the smallest restart root; accepted fragments splice; document-root parse is fallback | `incremental/reparse.rs:35` always invokes the full canonical document parser, then reconciles identities. Current §8 labels fragment optimization future work. | **Unresolved commitment, F-002.** The implementation performs full-document parsing; the original fragment-reparse requirement remains outstanding. |
| `a37dbc8ab`, retained-document streaming contract; later B2–B6 commits | `DocumentStream` is a distinct retained parser path with lifecycle/publication/work contracts and dedicated equivalence, recovery, resource and complexity suites. | New capability; executed evidence supplied by the streaming work packet. Editor identity reuse and streaming continuation have separate implementations and evidence. |
| `fa8d687df`, Type System v1 | The original semantics require closed results, storage-blind overload resolution, planned conversions and structured diagnostics. The current contract strengthens physical binding after the original R4 handoff. | Changed semantic authority. Core scalar registry checks passed; runtime compiler error conversion loses source anchors (F-001). |
| `d54feddf8`, named compute regions | Original document already limits execution to one GPU region and identifies the multi-kernel scheduler as future work. Current document also restricts application backends and compound bytecode packaging. | The historical contract already records the multi-region limitation. SIMD/JIT evidence applies to the fixed-shape experimental paths. |
| `b175945e1`, accepted interactive architecture | Session owns commands, replacement, state and output; browser/native code adapts it. Later commits add asynchronous ownership/terminal-response rules. | Inspect session and browser generations together when qualifying the application demo. Browser lifecycle qualification requires execution through the browser controller. |
| `04e8d4ae3`, artifact activation/arena semantics | Artifact construction and resident admission remain separate; current source contract explicitly rejects live cardinality ranges at activation. | Preserve stage distinction. Report an unsupported activation at the activation stage. |
| `f9965cb3d`, native build/bytecode documentation | Current builder and native closure fixtures retain artifact-specific construction. | Product closure and artifact-size claims require measured builds under identical settings. |

Reproduction examples: `git show 63414c821:docs/design/incremental-syntax-architecture.mec`; `git show fa8d687df:docs/design/type-system-v1.md`; `git show d54feddf8:docs/design/named-compute-regions.md`. The canonical specification remains the language authority; these architecture contracts describe product and implementation obligations. Fixed-integer intervals have their own later grammar contract.

## Walkthrough 1: source to activation

1. `SourceDocument::parse_resolved` is the retained product document boundary. `ProgramCompiler::compile_source` (`src/runtime/src/runtime/program/compiler.rs:292`) delegates to the view; its view parses once into a retained document (`:697`) and compiles that owner (`:718`). The syntax snapshot owns byte coordinates, diagnostics and lossless structure. A clean tree satisfies the syntax gate for semantic checking.
2. `CanonicalSourceFrontend::compile_document` (`src/engine/src/source_semantics/frontend.rs:675`) calls `reject_recovered_syntax` before document lowering. The engine owns semantic interpretation. `CanonicalSourceProgram` (`:96`) retains `SourceProgram`, schema table, constants, per-node contracts and source map. Its public source map preserves document/revision/range anchors (`:53–91`).
3. `CanonicalSourceProgram::compile_artifact` (`:312`) requires each ordinary operation's contract, copies source program metadata, maps external input names through the reversible transport encoding, and invokes `compile_source_program_with_control_contracts`. This produces an immutable `ProgramArtifact`. Source names such as `@ctx/path` retain their identities; resource grants are supplied through the runtime capability path.
4. `ProgramArtifactDraft::finalize` (`src/engine/src/artifact/model.rs:711`) is the artifact validation boundary. Bytecode-v1 encode/decode functions are at `artifact/bytecode.rs:456` and `:625`. Provider availability and supported resident shape are checked during admission and activation.
5. `MechRuntime::load_source_program` (`src/runtime/src/runtime/program/loading.rs:61`) enforces source byte limits and enters the production loading path. `resident::activate` (`src/engine/src/resident/general/mod.rs:1969`) requires an artifact, catalog and activation facts. Preflight (`:2027`) classifies nodes, checks state initializers, builds the schedule, completes shape facts, builds layout and binds a plan before mutable instance state is exposed.
6. A live-input range can have a correct inferred source type and serializable artifact yet fail this activation boundary because its cardinality cannot be accommodated by the fixed resident arena. `canonical_source_review.rs:457` owns that distinction. The demo must display the attained stage and the activation rejection.

**Finding on this path:** `SourceSemanticError` owns a precise anchor (`frontend.rs:603`); its `Display` prints code/message. `ProgramCompiler` repeatedly converts it with `to_string()` (for example `compiler.rs:757`) and constructs an error lacking a source range (`:120`). This finding applies to the inspected `SourceSemanticError` conversions.

## Walkthrough 2: changed input to accepted publication

1. An activated instance accepts typed input through `ReactiveInstance::prepare_turn_values` (`src/engine/src/resident/general/execution.rs:900`). The prepared turn retains a working epoch and candidate outputs. Reading `copied_output` (`:240`) accesses candidate data after readiness. Publication follows through a separate prepared-turn operation.
2. Concrete operations consume validated schemas, shapes and operation contracts. Memory planning remains separate from type inference: `ResolvedValueDescriptor` certifies semantic agreement before storage compatibility and implementation selection. Allocation, reuse and lease policy operate within that resolved type.
3. The general managed storage boundary validates complete candidates before visibility. In `src/core/src/memory_runtime/transaction.rs:285`, a publication batch validates every interval candidate and retains the read leases before locking any cell. Single-cell readiness performs the corresponding validation at `:754`. These core storage transactions provide a validation layer beneath the resident epoch mechanism.
4. `PreparedResidentTurn::publish` (`execution.rs:306`) refuses external plans. `publish_external` (`:325`) requires the coordinator's publication authority after provider and receipt/outbox preparation. `publish_inner` (`:342`) commits continuation candidates and releases the accepted epoch. Dropping or aborting a prepared turn preserves the previously accepted epoch.
5. Compute host writes enter an effect queue. Their delivery occurs after ordinary transaction acceptance (walkthrough 4). A display should identify accepted state and candidate failures separately.

**Executed feasibility:** six `mech-core` interval publication tests under `--features full` passed. They cover readiness revalidation with writer exclusion, late matrix-member rejection, exact integer widths, empty/padded geometry and in-place restore/retry. This establishes those cell-level behaviors. Public runtime, bytecode transport and browser input remain separate required demonstrations.

## Walkthrough 3: editor edit to identities and diagnostics

1. `DocumentSession` (`src/syntax/src/document/incremental/session.rs:10`) owns the accepted `SyntaxSnapshot`, ID generator, parse limits and interpretation counter. `try_apply_edits` (`:41`) returns a structured source error for invalid byte ranges.
2. `reparse` (`incremental/reparse.rs:27`) calls `TextSnapshot::apply_edits`, constructs a `ChangeMap`, and invokes `parse_canonical_document_with_ids` on the **entire updated snapshot** (`:35`). Fresh canonical syntax determines the tree, recovery and diagnostics.
3. The reconciliation budget is derived from old/new element and diagnostic work (`:38–72`). A range/structure index selects old candidates; successful reconciliation reuses their immutable green nodes and IDs (`:73–75`). Hash matching alone is insufficient. If optional work is exhausted, fresh canonical identities remain valid.
4. The new `SyntaxSnapshot` gets reconciled diagnostic anchors and a restart index (`:76–89`). Preserved IDs are counted separately (`:90–100`). `ReparseStats` (`:101`) explicitly reports full-document parser work as fallback/total work and reports reconciliation work separately.
5. `try_apply_edits` replaces its accepted snapshot only after this succeeds and returns `DocumentUpdate` containing old/new revision, changed range, reused/reparsed roots and diagnostic deltas. Views must match semantic results to the current owner and revision.

The demonstration should graph parser steps, reconciliation work, ingestion elapsed time and view/export time separately. Preserved subtrees measure identity retention. Parser-work counters measure parsing work. The historical fragment-reparse requirement is listed explicitly as F-002.

## Walkthrough 4: named compute region to completion

1. Section annotations enter canonical document semantics. `CanonicalMixedSourcePrograms` and preparation (`src/engine/src/source_semantics/frontend.rs:159–184`) derive coordinator, compute and initializer programs from one retained syntax owner. `ProgramCompiler` exports immutable artifacts, typed interface, initializers and dependency identities (`src/runtime/src/runtime/program/compiler.rs:219–252`). The source compiler retains ownership of its planning state.
2. `ComputeBackendRegistry::resolve` (`src/compute/src/registry.rs:338`) filters configured candidates by platform, hard placement and kernel capability, then calls each factory's support check. An explicit incompatible request returns `ExplicitBackendRejected`; auto selection tries its deterministic preference order (`:405`). Application qualification must also establish that the source compiler produces a supported kernel form.
3. The concrete bundle registers factories (`hosts/gpu/src/compute_backends.rs:32`); the source application adapter lowers its compute artifact before factory compilation. `ComputeHostFactory` connects the session to runtime resources and grants. Inspection before execution records the selected backend identity.
4. On committed effect delivery (`hosts/gpu/src/compute_provider.rs:956`), the host checks its phase and logical turn, chooses demanded output ports, records `InFlight`, then dispatches. A `Submitted` report returns without speculative telemetry/sample publication (`:993–999`). CPU completion can finish synchronously; browser GPU work completes later.
5. `ComputeHostCompletionTarget::complete` (`:522`) upgrades a weak state owner and verifies the exact in-flight turn (`:535–543`). Completed samples are materialized only for completed turns. Integrity rejection and transport failure take separate outcomes. Retired owners and mismatched completions are rejected, preventing an obsolete generation from publishing into a new owner.
6. Only values read after this completion boundary can support numerical comparison. Requested selector, selected descriptor, completed report, adapter/device identity and independent expected values are separate evidence fields. GPU qualification records the actual adapter and device availability.

## Quality findings and priorities

The detailed records include exact locations, consequences, proposed actions, contracts and acceptance checks.

- **F-001:** preserve semantic diagnostic anchors across the product compiler boundary. This directly affects the requested source-linked errors.
- **F-002:** retain the original partial-reparse discrepancy. Resolution requires an implementation and qualification effort or an explicit scope decision.
- **F-003:** consolidate manually duplicated scalar/optional-scalar conversion tables using existing semantic registry authority, subject to nested-option and Dynamic tests.
- **F-004:** replace selected positive source-spelling gates with stronger API/behavioral authority checks while retaining useful forbidden-boundary scans.
- **F-005:** record test counts and explicit feature closures. Two initial no-default-feature targets succeeded with zero tests; the logs remain as counterexamples to exit-code-only evidence.
- **F-006:** correct remaining comments that describe the completed canonical cutover as pending.
- **F-007:** reconcile the composition contract with engine-owned resident arithmetic; qualify machine factories and resident arithmetic through their respective execution paths.
- **F-008:** retain the documented interval-arithmetic rejection as a capability boundary. Original conversion rules and the predicate correction exclude implicit widening and numeric capabilities.

The physical accounting separates production, embedded tests and later production helpers within large files. The frontend is 10,218 physical lines, including embedded tests and later production helpers; the general resident module is 23,873 lines with a large embedded test region and later production code. Both files therefore require line-level classification. Reviewed responsibilities include source lowering, error transport, activation preflight and selected publication paths. The coverage register identifies the remaining unreviewed spans.

The sampled managed-view code localizes unsafe access at memory/ABI boundaries with safety comments. Whole-repository unsafe review and Miri execution remain unverified. The allowlist and architecture scans identify boundaries; pointer extent, lifetime, alignment and alias invariants require behavioral validation.

## Subtraction and remaining work

`data/subtraction.json` separates moves, simplifications and retained authorities. Null line estimates indicate unmeasured work. The frontend mapping candidate has a measured reviewed span. Net line estimates require a coherent replacement prototype. The resident EKF private control/benchmark implementation has an explicit ownership comment permitting later relocation with the workload; the general resident executor is essential language/runtime machinery.

Moving the standard library must retain type, artifact and publication contracts. Integration tests proving external library admission belong with Mech; implementation-private operation tests can follow external ownership. The extraction packet supplies the boundary arithmetic and experiment. Every adapter/fix/demo added later must be included in the final census delta.

Executed browser evidence now covers source streaming/editor behavior, exact type/publication cases, CPU and hardware GPU compute, and the maintained WASM application path. The capability register links each measured scope to its artifact, environment and expected-value checks. Experimental SIMD/JIT application paths and fragment reparsing remain outside these completed claims.

## Executed types demonstration

The recorded WASM artifact passes thirteen deterministic browser cases: interval bounds and rejections, present and absent options, optional-record nesting, peer-constrained input inference, a two-by-two matrix, named argument order and the exact u128 maximum. Executable cases perform bytecode-v1 encoding and decoding before resident activation. The inferred-input example reaches activation and awaits an external input.

On one activated state instance, inputs 9, 10 and 3 produce acceptance, rejection and acceptance. The rejection preserves output, publication epoch and accepted state hash. Three new-instance cycles restore the initial value 2. A semantic diagnostic after a Unicode prefix selects the exact source text `10` in the browser; the product compiler retains its typed byte identity and presentation range. Artifact hashes, feature closure, browser version, inputs and results are in `evidence/types-browser.json` and `site/artifact.json`.

The initial synthesized optional-tuple fixture was rejected by strict syntax validation. Its result is preserved in `evidence/types-browser-initial-fixtures.json`; the corrected demonstration uses the maintained optional-record fixture from `src/engine/tests/canonical_source_structures.rs:1033`. The initial JavaScript harness failure is preserved separately. Three native adapter tests passed, including five exact scalar comparisons. The same five sources agree with browser results. One additional native product-boundary regression passed for exact semantic identity and Unicode source-range projection; see `evidence/types-native-browser-comparison.json` and `evidence/canonical-semantic-diagnostics.log`.

## Incorporated recurrence proof

PR829 at `9ae5c00e0184fe341d2ff2bdc590ede712009d2f` supplied a recurrence proof using earlier parser/planner interfaces. The absorbed example uses the retained canonical document parser and semantic frontend. Its operation identities remain `math/mul`, `math/add`, `compare/lt` and `core/assign`; the current compiler emits an additional assignment node that snapshots prior state. An exact five-node sequence assertion preserves the bounded operation claim.

The native run publishes `5,13,29,61,125,253,509` through both source and decoded artifacts, with equal receipts. A source constant change produces `7,25,79` and a changed artifact revision. The next original candidate, 1021, fails its integrity condition and preserves accepted epoch, hash, state and output. Syntax, semantic typing, empty-catalog activation, truncated bytecode, modified artifact identity and one-byte-budget checks also pass. The browser extension checks first-turn values 5 and 7 and syntax/type/integrity rejections through the recorded WASM artifact.

Two old representation assumptions required explicit adaptation: the pre-turn output is now a derived slot, and semantic state roles identify state independently of shared physical storage. The original failures, exact two-commit patch, per-file absorption hashes and adaptation patch remain in `evidence/pr829-absorption.json` and its linked files. The original initial-output-alias assumption is retained as a documented difference.

The additional inventory workload uses base i64 arithmetic with explicit nonnegative-input, stock and capacity invariants. Nine recorded candidate updates produce equal native/browser results; rejected candidates preserve the accepted snapshot and later valid candidates succeed. The heat-diffusion workload completes six CPU/hardware GPU runs on 4×4 and 8×8 grids, with changed diffusivity and heater inputs, against an independent flux-sum reference. Their capability records preserve exact source, artifact and execution scopes.
