# Remaining frozen-B extraction dependency closure

Read-only source review against these identities:

| Identity | Role |
| --- | --- |
| `b573284cb4587d4fb0085cc69a80a036830f1b76` | S8A extraction base |
| `8be3118e1` / `codex/syntax-s8e1-config-syntax` / PR #831 | Existing E1 configuration extraction head |
| `89403ba4b` / `codex/syntax-s8e2-syntax` | Existing E2 syntax correction extraction head |
| `732208864` / `codex/syntax-s8e3-resident-values` | Existing E3 resident primitive extraction head; current comparison base |
| `662d29b79` | Frozen production B reference, whose exact tree must be reconstructed |

This document describes dependency closure, not executed build results. No branch,
production source, Cargo invocation or commit was changed during this review.
Frozen-source line numbers below identify **function boundaries**; select complete
functions and their necessary imports, not an accidental unified-diff hunk boundary.

## Existing versus proposed extraction labels

The earlier `PR-STACK.md` was a proposal and its E-number assignments no longer
match the actual branches. Current E2 is syntax, current E3 is four resident/core
files, not the earlier proposed document PR. Do not use the old path ledger's E2
column as a script to copy whole files now.

Current E3 contains the frozen deltas of:

- `src/core/src/operation_contract/maintained.rs`;
- `src/engine/src/resident/numeric.rs`;
- `src/engine/src/resident/general/mod.rs`;
- selected resident execution value/shape hunks in `general/execution.rs`.

Remaining `execution.rs` differences are the three projection-refresh hunks at
base lines 498, 522 and 533. They change a public method signature and its behavior;
**all three belong with the runtime query caller change**, not with an otherwise
engine-only E4. Frozen line numbers after the change are approximately 498–620.

E4 may take the remaining engine/core frontend and product deltas as one engine
foundation, excluding those refresh hunks. That foundation exports everything
required by later runtime branches without depending on `mech-runtime`, GPU hosts
or the new compute activation evaluator. It includes:

- core catalog semantic-contract helpers and `SchemaTable` closed-body support;
- engine artifact/product construction (`ProgramCompilationProduct::from_canonical_artifact`
  and source-dependency metadata);
- canonical document/functions/imports/assignments/output projection and the
  frontend public entry points for resource, graph and mixed projections;
- engine resource-read/compiler-planning integration and source tests;
- the frozen EKF canonical artifact closure helper and its owned test.

The API exposes preparation capabilities for subsequent runtime PRs; it does not
claim the later production adapters or unresolved semantic families are sealed.

## Recommended dependency order after E4

Use responsibility labels below until branch numbering is finalized:

```text
E1 -> E2 -> E3 -> E4 engine canonical document foundation
                     |
                     +-> RUNTIME-A ordinary compilation/resource planning
                           |-> RUNTIME-B rooted/ordered graph ownership
                           |     |-> MIXED-A canonical mixed compiler
                           |     |     +-> MIXED-B compute activation/backends
                           |     |
                           |     +-----------------------> WEB bundle/host handoff
                           |                               ^
                           +-> INTERACTIVE lifecycle ------|
                                 (query/engine refresh)    |
                                     MIXED-A + MIXED-B ----|
```

A simple linear stack can choose RUNTIME-A, RUNTIME-B, INTERACTIVE, MIXED-A,
MIXED-B, WEB. INTERACTIVE does not technically require RUNTIME-B; its lifecycle
uses document compilation. MIXED-A's rooted wrapper does require RUNTIME-B.
The diagram distinguishes actual dependencies from the chosen review order.

## RUNTIME-A — ordinary canonical compiler and effect-free planning

`src/runtime/src/runtime/program/compiler.rs`, frozen symbols:

| Copy from frozen B | Lines | Required dependency |
| --- | ---: | --- |
| Canonical compilation error type/helper; `retained_compiler_document` | 59–94 | SourceDocument already exists at E3 |
| Public `compile_document`, `compile_interactive_document`, `compile_canonical_source` | 182–188,269–282 | corresponding view entry points below |
| Public document artifact/input/initializer/static/source-artifact methods | 310–385 | ordinary planning closure below |
| `CanonicalDocumentPlanning` | 647–653 | owned schemas/reads/writes/values |
| View document/interactive/artifact methods | 688–711 | common canonical artifact projection |
| `canonical_planning_context`, `canonical_planning_projection`, document initializer compilation, candidate preflight validation | 713–929 | E4 canonical binding/output projection; provider registry |
| `canonical_document_artifact`, `_with_projection` | 931–984 | E4 canonical frontend; resource helper below |
| `canonical_document_resources` | 1516–1674 | source index, canonical context bindings and existing inline-context materializer |
| `preflight_canonical_effect_payloads`, `execute_named_canonical_outputs` | 2742–2824 | prepared resident turn / provider effect-free hooks |
| `resolve_canonical_context_bindings`, `canonical_resource_send_operations` | 3349–3435 | source context inheritance and existing declared-send resolver |

Keep the old tree/source compiler methods and services exactly at the current
base in this extraction; canonical entry points are additive at frozen B. This
is extraction of already accumulated code, not the later retirement cutover.
The large diff hunk beginning `@@ -350,51 +609,1068` spans ordinary planning,
all canonical graph helpers and resource gathering: **do not copy it whole**.

Imports that now become unconditional with RUNTIME-A include `ReactiveInstanceId`,
`activate_external`, `ActivationFacts`, `ResidentIntegrityMode`,
`CanonicalSourceFrontend`, `CanonicalSourceProgram`, `ProgramArtifact`,
`SourceContextBase` and `SourceDocument`. Leave `SourceContextCapabilityScope`
compute-gated. `ComputeRegionInterface=()` and `ComputeValue=()` already exist on
the E3 base and need no newly invented compatibility helper.

Also take the complete `runtime/program/value.rs` delta and `loading.rs` delta,
plus **only `query.rs::program_output_id`'s output-identity hunk**. They all use
`program_result_output_index`; ordinary and interactive canonical documents can
publish presentation aliases on either side of their implicit result. This is a
compiler output contract, not dependent on adopting canonical REPL lifecycle.
Leave `query.rs`'s new `refresh_output_projections(&candidate_artifact, ...)` call
for INTERACTIVE with its engine signature/body changes.

Owned tests in `runtime/program/tests.rs` include canonical resource inactive-owner
filtering; native sidecar completeness; trailing send result; nonempty planning
values/live defaults; static matrix detachment and output selection; local named
function binding/scope; resource-bearing function first use; tuple destructure;
function imports; constant ranges; dimensionless annotation shapes; provider matrix
shapes; static constraint projection and unbound-input pruning; uncalled function
resources; document interactive provider planning/preflight; portable selector
width; missing-provider error; full FizzBuzz output identity. Carry the helper
`canonical_planning_test_document` with the first of these tests.

Do not copy the three new mixed tests near line 1207, any `canonical_mixed_*`
test later in the file, canonical resolved/rooted/ordered tests, or replacements
of the old explicit-root tests until their owning API lands.

The new five-route test in `query_tests/source.rs` calls graph APIs on routes 2
and 3, so the **complete unchanged test belongs in RUNTIME-B**. Its shared provider
counter/`plan_write` additions can travel with it. Do not weaken its loop to make
RUNTIME-A build; RUNTIME-A has separate document-only planning tests above.

## RUNTIME-B — rooted and ordered retained graph ownership

Take these frozen `compiler.rs` functions together:

- public `compile_canonical_root`, resolved-root, ordered-roots, all interactive
  rooted variants and their options wrappers (190–267);
- `CanonicalGraphCompilation` and its constructor (627–645);
- `compile_canonical_roots` (986–1232);
- `compile_canonical_root`, `_resolved_root`, `_graph_document`,
  `_graph_module_identity` and `_graph_imports` (1234–1514).

They depend on RUNTIME-A's resource planning and validation, E4's
`CanonicalOrderedDocument/CanonicalOrderedImport` frontend and product source
metadata, and the **whole frozen delta of
`src/runtime/src/resolver/canonical_handoff.rs`**. The latter makes
`CanonicalResolvedImport<T>` generic, shares `canonical_import_values`, and
exposes `validate_canonical_import_uses`; assigning that file to WEB would make
RUNTIME-B fail to compile. No graph helper depends on the new compute evaluator.

Take `tests/canonical_declaration_handoff.rs` with this resolver change. Its
constant-binding test also exercises the E4 API. Take the two existing explicit-root
tests' conversion to canonical APIs, canonical resolved/rooted interactive tests,
canonical ordered-root scope/error tests, and the complete five-route provider
preflight test described above.

`resolver/file.rs::source_path_candidates` is **not** a canonical graph prerequisite:
old file resolution already searches the paths internally. Its public factoring
is consumed by bundle linking and can remain in WEB. The malformed-retained-source
test correction at the bottom of that file is different: E2 made the old particle
fixture valid, so its deliberate-error replacement belongs at the first available
intermediate validation repair (prefer E4), alongside
`runtime/module/tests/source_document_transfer.rs`'s fuel-limited invalid fixture.
Those fixture corrections do not require the public path helper or a graph redesign.

## INTERACTIVE — accepted retained candidate lifecycle and projection refresh

Copy the entire frozen `src/runtime/src/interactive.rs` delta, including its
owned tests. It adds finalized-stream admission, retained-document construction,
submit/replace/reset/clear handling and canonical mutation detection. Its compiler
requirement is `compile_interactive_document` from RUNTIME-A; no graph API is used
by its production lifecycle or test factory.

Copy these **inseparable** pieces in the same PR:

1. the remaining three `engine/resident/general/execution.rs` hunks adding the
   artifact argument, revision check, published state-candidate map and refresh
   behavior;
2. `runtime/program/query.rs`'s call passing `&candidate_artifact`;
3. the retained-state replacement/projection tests in `interactive.rs`.

There is one production caller of the changed refresh API in frozen source. A
compiler-only E4 with the signature hunk but the base runtime caller will not
build. Conversely a runtime-only new caller without the engine change will not
build. The published-candidate logic must not be peeled off merely to satisfy the
signature: its purpose is preventing a migrated state transition from running a
second time during presentation refresh.

`value.rs`, `loading.rs` and `query.rs::program_output_id` are already in RUNTIME-A
in this proposed split; do not duplicate or silently revert them here.

## MIXED-A — canonical mixed compiler and port names

Copy the frozen mixed-only compiler additions:

- public `compile_mixed_source`'s change to retained parsing, new
  `compile_mixed_document`, and canonical mixed rooted/resolved wrappers
  (488–531);
- `MixedProgramCompilation::source_dependencies` and the new field in **both**
  existing old-tree mixed constructors (127,2028,2068 in frozen source);
- `compile_canonical_mixed_resolved_root`, `compile_mixed_document`,
  `compile_mixed_document_with_imports` (1836–2003);
- `capture_canonical_compute_activation_inputs`, canonical declared input/output
  capability helpers (2827–2913).

The new field is a struct-literal closure: adding it to the type while leaving
one old constructor behind breaks builds even though production prefers the new
path. Do not remove an old API or constructor during this extraction.

Dependencies are RUNTIME-A preflight/output capture, RUNTIME-B's exact graph
imports and dependency identities, and E4's `compile_mixed_document_with_planning_contract`.
Also copy the small `src/compute/src/port.rs` delta that decodes canonical source
input names. Without it the canonical artifact's internal encoded names become
external compute port names, breaking declared input/default correspondence.
That method `mech_engine::decode_source_input_name` exists in E4's frontend.

The frozen runtime functions `assemble_compute_region`, `compute_initializers`,
`plan_compute_read`, `plan_compute_write` already exist at E3 and do not call the
new `ComputeActivationValues`. Leave their bytes unchanged. Likewise the existing
compute feature gates and placeholder aliases do not need alteration.

Take the three new mixed tests near frozen line 1207 and the later mixed local
function, tuple-destructure, EKF and particle region tests with MIXED-A. Region
compiler tests establish the prepared API only; they do not seal the complete
application/backend contract. Keep downstream CLI/WASM callers for WEB or a small
subsequent adapter PR until backend execution is available.

## MIXED-B — compute activation, IR and backend consumers

There is **no `ActivationBoundary` symbol** in frozen B. The added canonical
activation owner is `mech_compute::ComputeActivationValues`. Do not confuse its
backend state-initializer job with MIXED-A's coordinator effect capture and compute
input-default projection; these are different dependency boundaries.

The smallest safe whole-file extraction closure is:

- `src/compute/src/activation.rs`, `ir.rs`, `shape.rs`, `lib.rs`, `fixed_shape.rs`;
- `hosts/gpu/src/lib.rs`, `batched/mod.rs`, `batched/jit.rs`;
- their owned `hosts/gpu/tests/particle_source.rs` and `parallel_ekf.rs` changes;
- `hosts/gpu/src/memory.rs`'s reset/readback preservation change and its test;
- `include/browser-compute.js` if those host tests include the browser retained
  output behavior; its JS tests/actual browser check stay attached.

The hard compile edges are concrete:

| Addition | Required peers |
| --- | --- |
| `BinaryOperation::Remainder` (`compute/ir.rs`) | exhaustive SIMD match in batched/mod.rs; JIT lowering/import/function declarations in batched/jit.rs; WGSL lowering in hosts/gpu/lib.rs |
| `FixedShapeStoragePlan::publications` and `FixedShapePublicationStorage` | BatchCompiler construction, dead-instruction pruning, retained-buffer admission and fixed-shape backend consumers in batched/mod.rs |
| `ComputeActivationValues` | module export in compute/lib.rs; `source_dimensions` visibility in shape.rs; shared `ElementwiseInstruction::evaluate_into` in ir.rs; elementwise and fixed-shape compiler fields/initialization in hosts/gpu/lib.rs and batched/mod.rs |
| shared concatenation shape helper | shape.rs implementation and host compile-time concatenation callers |
| derived publication storage | retained state/publication count, double-buffer commits, demand readback and tests; treating it as recurrence state changes semantics |

Adding the enum variant or required struct field as a standalone “IR-only” PR
without these consumers creates compile failures under maintained GPU/SIMD/JIT
features. If a smaller separation is wanted, `evaluate_into` plus the activation
module can be extracted additively **without** Remainder/publications, followed by
their exhaustive consumers, but that requires hunk-level proof and offers little
review benefit over this coherent backend responsibility. No shim or default
publication field should be invented to bridge intermediate heads.

## WEB and remaining presentation/qualification files

After the prior responsibilities, these frozen whole-file deltas form the browser
handoff boundary:

- runtime `program/bundle.rs` plus both cfg/module/re-export hunks in
  `program/mod.rs`;
- runtime renderer/body-framing delta and `canonical_document_render` test;
- remaining `resolver/file.rs` public path-candidate factoring;
- top-level browser/bundle planning/presentation, `bundle_web.rs`, and the
  corresponding module exports in `src/lib.rs`;
- `wasm/project.rs`, `wasm/mixed_compute.rs`, `cli/compute.rs`, static-project JS,
  bundle smoke scripts, browser compute smoke driver and their owned tests.

Bundle encoding depends on E4's product dependency metadata; truthful transitive
values are supplied by RUNTIME-B/MIXED-A. The WASM helper uses the mixed compiler
and whole-source retained document APIs. A compiled bundle producer does not
complete G22's still-retiring actual document loader; extraction must preserve
that audit blocker.

Move final cross-cutting CI/manifest/readiness docs only after exact patch-union
comparison. Syntax-generated files already landed in E2 must not be copied again
from the obsolete proposed E8 ledger. Cargo changes belong at their first actual
caller: runtime's typed syntax dependency already landed with E1, engine feature
forwarding in E4, top-level bin/module feature/test wiring with the affected WEB or
qualification build. Do not postpone a necessary feature edge merely because its
old ledger row said E8.

## Verification for each intermediate head

This is the required execution plan; no command result is asserted here:

- E4: engine canonical source/document/state tests and retained source feature
  check, with the unchanged runtime refresh caller still compiling.
- RUNTIME-A: runtime source compiler tests with compute disabled and enabled;
  its nonempty planning/default/preflight/static/output-identity tests.
- RUNTIME-B: canonical declaration handoff, rooted/resolved/options/ordered
  compiler tests and the five-route provider preflight test.
- INTERACTIVE: complete interactive session tests, migrated projection tests,
  non-source resident build verifying the refresh API remains usable.
- MIXED-A: canonical mixed compiler tests with configured catalogs, port names,
  retained activation inputs and rooted transitive identity.
- MIXED-B: affected elementwise/fixed-shape CPU, SIMD, JIT, WGSL/WebGPU compile
  profiles; source-to-backend activation/publication tests, not kernel-only smoke.
- WEB: bundle/served/browser adapters plus the exact known G22 retirement blocker
  retained in review; no seal claimed from a producer-only unit test.

After extracting each function/file closure, compare it against the corresponding
frozen symbol bytes and retain unextracted pieces from the current parent. At the
final extracted head, `git diff --exit-code 662d29b79 <head> -- <production paths>`
must be empty; audit-only files and intentional PR metadata are separately accounted
for. An intermediate compile error is a missing extraction dependency, not approval
to change behavior or repair an unrelated semantic gap.
