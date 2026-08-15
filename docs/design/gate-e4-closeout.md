# Gate E4 closeout

Status: exact-head stabilization is locally complete on draft PR #762; remote
exact-head CI and review remain required. This is not a merge authorization.

## Boundary

- Exact E3 base: `3610c66961dcc2fa23aed05833b4722ae34790c0`.
- E4 remains stacked on E3, draft, and unmerged.
- No retained host has been deleted.
- No interpreter fallback or compatibility alias has been restored.
- The exact final E4 head will be recorded in the PR evidence after E4I is
  committed; a commit cannot contain its own content-addressed SHA.

## Compiler-planning quarantine

The shipping executor is the resident `ProgramArtifact` route. Source
compilation may use `CompilerPlanningProgram`, `Interpreter`, and
`LegacyValue` only as private, short-lived semantic-compiler coordinates.
Only `ProgramCompilationProduct` escapes compilation.

Accepted for v0.4:

- private semantic-compiler implementation may use `Interpreter` and
  `LegacyValue`;
- exact provider/value adapters remain governed by the value-system boundary.

Not accepted:

- public `Interpreter` API;
- `MechProgram` API;
- runtime-only compilation of interpreter machinery;
- bytecode execution through the interpreter;
- shipping fallback;
- runtime-owned compiler-planning state.

Future and non-gating: replace the private planning internals with a direct
AST-to-artifact compiler.

## Ordered E4 commits

1. `6f8f9937715968d883aebd68c16b7d9494a427a9` — `refactor(core): separate semantic compilation from resident values [E4A]`
2. `d2f4b9ae54624db1152720cbc69d4dac557d8323` — `feat(engine): close retained resident operation semantics [E4B]`
3. `c45bace47dda5f18bc7a003945cc9d5302e857f1` — `fix(runtime): harden resident product execution [E4C]`
4. `d006b1c6c8d91682749ca0e5cf2e69111b2ad2a2` — `fix(build): close generated product and distribution contracts [E4D]`
5. `2773d69b703a50c069e8d0d7becfdeb744efc05d` — `test(ci): enforce permanent E4 qualification contracts [E4E]`
6. `eea620a8c7ddefeae5f2a2607681437f17d61982` — `fix(ci): close final E4 qualification gaps [E4F]`
7. `adccb7f325882bcb470f941e0603841249a680d6` — `refactor(engine): quarantine compiler planning and delete executor APIs [E4G]`
8. `f6555296d83ceb4578235bc0d0f1e57013875a6a` — `test(architecture): retire active migration projection and prove compiler quarantine [E4H]`
9. `[self-address recorded in PR evidence]` — `docs(architecture): close E4 and hand off evidence-only F0 [E4I]`
10. `32ac722ef3353a692f240c10adab0479a49daf53` — `fix(e4): close exact-head product regressions`
11. `[self-address recorded in PR evidence]` — `test(e4): reconcile deterministic generated artifacts and finalize closeout`

## Exact-head stabilization ledger

Only failures reproduced by full validation run `31910896507` at exact head
`62da5a762bb81a0211b2264fe55c97e616cc807b` remained active. Earlier E4
failures are resolved or superseded by that run.

| Current-head failure | Classification and permanent owner | Narrow correction and acceptance proof |
| --- | --- | --- |
| Runtime wildcard-import audit found `use super::*` in the resident string-output regression. | Test-boundary regression in `runtime::program::value`; no feature or production semantic change. | Import only `string_matrix_value`, `LegacyValue`, and `ResidentShape`. The complete runtime boundary audit and focused string-matrix materialization test pass. |
| Bytecode-v1 determinism stopped on a committed `manifest.json` mismatch. | Deterministic generated-evidence drift. Two independent fresh corpora were byte-for-byte identical; all 20 `.mecb` files and source fixtures were unchanged, while 19 derived native-plan hashes reflected the frozen E4 plan. | The checker now compares fresh run A with fresh run B before comparing either run with the repository. The committed manifest and its frozen digest were refreshed only after A equaled B. The format contract passes for all 20 fixtures and the corrected checker passes across two fresh child processes. |
| The served FizzBuzz document reached `ready` with an empty output block while the same WASM resident artifact proved `first-fifteen! == true`. | Formatted-document adapter identity mismatch: HTML addresses an output by its stable 64-bit source-block hash, while the artifact query accepts compact `OutputId` values. The compiler, modulo, comparison, string-matrix value, and resident execution results did not diverge. | `WasmDocument` records direct fenced-output hash-to-root-symbol relationships from the decoded source tree and renders that detached resident symbol. Reset rebuilds the mapping. Native mapping coverage passes, the exact WASM profile compiles, and the browser regression now requires the rendered FizzBuzz matrix to contain `✨🐝`. |
| Native application graph validation expected 17 projects but generation and the independent determinism contract produced 15; after removing the two dead actors, the checker still expected pre-resident runtime feature graphs. | Deterministic generated-contract drift. The actor entries had no remaining producer or caller, while every surviving generated product now loads emitted bytecode through `mech-runtime/resident-routing`; the emitted 15-plan set is the frozen architecture source of truth. | Remove the two obsolete actor expectations and reconcile the exact package/declared-feature projections for all 15 surviving plans. A compact independent comparison reports 15 expected, 15 actual, and zero mismatches; the full checker passes all exact direct-package, declared-feature, resolved-graph, forbidden-package, and serialized-plan checks. |

## Work-item disposition

| Work item | Disposition |
| --- | --- |
| Canonical resident numeric, matrix, composite, set, strict comparison, and snapshot semantics | Retained as permanent typed resident kernels with focused source/bytecode parity coverage. |
| Generated native products | Load emitted bytecode through the resident runtime; no generated executor facade remains. |
| Source compilation | Sole product owner is `runtime::ProgramCompiler`; only `ProgramCompilationProduct` escapes. |
| Bytecode products | Decode, validate, admit, activate, and execute through the resident loader. |
| General `MechProgram` execution | Deleted. |
| Public interpreter surface | Deleted; the module and exact imports are crate-private and `semantic-compiler`-only. |
| Interpreter bytecode execution | Deleted with its register file and old execution tests. |
| Runtime profiling and `Cycle Time:` output | Deleted. |
| Developer legacy-executor carve-out | Deleted; no `.legacy_interpreter()` exception remains. |
| Active resident migration projection | Replaced by deterministic structural owner and exact-target enforcement. |
| Retained hosts | Terminal, browser DOM, timer, time, console, scene, and robot-arm paths remain resident-routed. |

## Deleted executor-shaped surface

Deleted files include `src/engine/src/program/instance.rs`, the interpreter
bytecode module and register file, the interpreter bytecode test suite, and
three external tests that imported private executor internals. Their surviving
compiler-only assertions now live under the semantic-compiler unit-test owner.

Deleted types include `MechProgram`, `MechProgramConfig`,
`MechProgramEnvironment`, `ProgramSolveOutcome`, `BytecodeRegisterFile`, and
`BytecodeRegisterFileCheckpoint`. No aliases preserve those names.

Deleted methods include the general interpreter accessors; all `run_string`,
`run_tree`, `run_source`, `run_sources`, `run_program`, and interpreter
`run_bytecode` variants; `run_profiled_string`; output queries; generic solve
plans; configuration/environment mutation; and runtime profiling output.

## Feature and routing proof

- Engine `runtime`, `runtime_default`, and `resident-artifact` compile without
  selecting the interpreter or compiler-planning module.
- Engine `semantic-compiler` and `compiler_default` select the private planning
  implementation needed by `ProgramCompiler`.
- The runtime-only graph does not select `mech-bytecode` compiler features.
- Root bytecode tests compile ordinary source with `ProgramCompiler`, encode
  bytecode v1, load it through resident admission, execute the resident
  instance, and compare canonical output.
- Production routing and compiler-quarantine checkers reject public interpreter
  reachability, old executor names, the old instance module, old executor
  paths, and any shipping fallback.

## Permanent activation contract

`resident-activation-contract.json` is generated deterministically from the
exact permanent semantic target identities. Its checker proves the required
artifact, activation, loading, and resident-admission owner markers; exactly
one public `ProgramArtifact` authority; and absence of obsolete executor and
activation owners. It contains no occurrence count, migration status, or
implementation-phase field. The active `d0-migration-projection.json` and its
schema are deleted.

## Mandatory-stop ledger

| Attempted change | Caller inventory | Command/error | E3 reproduction | Permanent owner | Disposition and continued work |
| --- | --- | --- | --- | --- | --- |
| Remove the live legacy-executor oracle from the D2 n-body fixture while preserving the resident/raw/source-bytecode checks. | `tests/fixtures/d2-contract-generator/src/main.rs`: source compilation and 4,096-turn legacy trajectory; `src/gate_d.rs`: legacy timing lane; `scripts/generate-d2-contract.py`, `scripts/check-d2-contract.py`, and `scripts/run-gate-d-benchmarks.py`: frozen correctness and timing consumers. | `cargo +nightly-2026-03-03 check --locked --manifest-path tests/fixtures/d2-contract-generator/Cargo.toml` failed because `CompilerPlanningProgram` no longer has `run_string`, `interpreter`, or `root_symbol_value`. | No. E3 still exposed the executor-shaped methods. | Permanent D2 resident trajectory contract plus live historical legacy evidence. | Resolved without restoring any current-tree executor API: the current fixture runs live source, bytecode, raw-Rust, allocation, and structural lanes, while the generator and benchmark runner execute the exact D2 implementation commit `96fd051608f9d9df9eb4e9b345af7c23279c6c67` as the live independent legacy correctness and timing control. |
| Replace the active D0 migration projection with a structural permanent resident-activation owner contract. | `scripts/generate-resident-activation-contract.py`, `scripts/check-resident-activation-contract.py`, their unit tests, selected/full CI, and `tests/architecture/resident-activation/d0-migration-projection*.json`. | The first static-owner replacement was rejected because it discarded the mechanical source-of-truth projection instead of proving an equivalent or stronger structural contract. | Yes: the migration projection is intentionally present on E3. | Permanent resident activation generator/checker. | Resolved in two proof-preserving stages: first generate and compare the permanent exact target set, then delete the occurrence projection. The replacement now checks deterministic generation, exact semantic target identities, required owner markers, one public artifact authority, and absent obsolete owners. |
| Remove the historical ancestry, publication-order, and exact inventory proofs while retiring the active migration projection. | The resident-activation generator/checker, frozen boundary document, pinned ancestry commits, frozen inventory blob, and publication-order assertions. | Automated review rejected the combined edit because it removed independent architecture enforcement beyond the authorized projection retirement. | Yes: all historical proofs are active on E3. | Permanent resident-activation checker, with the historical proofs retained until an equivalently strong permanent representation is approved. | Only the active occurrence projection was removed. The historical checks remain exact; current-tree structural owner checks were added alongside them. Compiler quarantine, projection retirement, and documentation continued. |
| Apply effect-free resident write planning to the controlled D3 provider. | `D3SceneProvider` and `D3TransactionalProvider` in the shared controlled-evidence fixture. | The full runtime suite rejected `gate-d3://scene/output#frame` with `RuntimeResourceWriteUnsupported` before executing the D3 evidence lane. | No. E3 did not require providers to validate payloads during source planning. | The provider-owned semantic write contract plus side-effect-free `plan_write`; no delivery before publication. | Added exact base/path, `Send`, and scalar-`F64` planning validation to both providers. The focused 4,096-turn D3 evidence test and the complete runtime suite pass. |
| Preserve a handcrafted nullary generated-app output-seed case after authoritative artifact sections replaced legacy output constants. | `native_output_seed_arities` only; no semantic operation contract, resident binder, source example, or shipping host referenced nullary `set/comprehension`. | The generated app could no longer execute the nullary seed without recreating a legacy-only operation path. | No. E3 still allowed the old output seed to stand in for executable artifact semantics. | Retained resident output computation and output-seed poisoning for every implemented arity. | Deleted the unsupported nullary case. Unary, binary, ternary, quaternary, and variadic poison cases remain green and prove generated applications use authoritative resident artifact sections. |
| Compile the retained runtime-only host owners after the interpreter became private. | `mech-browser`, `mech-console`, `mech-scene`, `mech-time`, and `mech-timer` select the engine `source` marker but do not own semantic compilation. | Each host owner failed to compile because engine compiler-planning modules and function catalogs were still selected by `source` without `semantic-compiler`, producing missing `InterpreterExecution` errors. | No. E4 introduced the private compiler-planning boundary and therefore owns its exact feature closure. | The engine feature boundary plus exact host owner suites and the engine `source`-only compile check. | Gated compiler-planning modules, source intrinsics, and source function catalogs on `semantic-compiler`. The `source` marker alone is now interpreter-free; all five exact host owner suites and the engine source-only check pass. |
| Preserve initializer-only resident state without treating arbitrary mutable symbol metadata as a declaration. | Artifact compilation's mutable-symbol metadata, declaration markers, initializer instructions, and state-slot construction. | `program::bytecode_plan_topology_tests::composite_helpers_and_mutable_metadata_without_a_declaration_do_not_become_state` found that bare mutable metadata manufactured resident state. | No. The issue arose from E4's initializer-state closure and must be resolved inside that semantic boundary. | Declaration markers are the authoritative state-declaration owner; initializer-only declarations remain supported. | State collection now requires a declaration marker before mutable symbol metadata can produce a state slot. The exact regression and the complete 286-test engine owner suite pass. |
| Weaken the explicit multi-root regression after dependency caching hid a requested root's observable output. | `ProgramCompiler::compile_roots`, requested-root ordering, dependency instantiation caching, `ProgramArtifact` publication, and bytecode-v1 artifact outputs. | The focused regression first produced only `[value]` when `main` imported the later explicit `dep` root. Automated review rejected replacing the missing-output assertion with the weaker fact that an `Add` node existed. | No. Ordered multi-root publication is an E4 compiler/artifact closure. | Explicit requested roots publish their result registers in caller order; dependency-only roots remain private. | The shortcut was not taken. Compilation now distinguishes requested roots from dependency cache hits and publishes every explicit root result. The focused source artifact and decoded bytecode-v1 artifact both expose `[answer, value]`; complete engine and runtime suites pass. |

## Review findings

- The handcrafted nullary `set/comprehension` generated-app fixture had no semantic operation
  contract or resident owner, so E4 removed that legacy-only case instead of recreating its old
  execution path. Output-seed poisoning remains covered for every retained resident runtime arity
  (unary, binary, ternary, quaternary, and variadic), while preserving the authoritative artifact
  sections.

All six findings from the review of `2773d69b70` have local corrections and
focused regression coverage: generated engine applications use resident
bytecode loading; composite children validate exact schemas; bytecode-v1
manifest evidence is current; the full-source fixture uses resident routing;
strict comparisons preserve schema/shape identity; and set membership
preserves element schema identity.

The five findings from the exact-head review of `ccad6da17` are also corrected
with focused regressions: n-choose-k accepts and validates every valid `k`;
numeric change detection derives scalar-versus-matrix policy from semantic
schema rather than a physically scalar 1x1 layout; inclusive ranges revalidate
cardinality before writing; composite packing reconstructs canonical matrix
children; and explicit roots remain observable even when an earlier requested
root instantiated them as dependencies. Source and decoded bytecode-v1
multi-root artifacts publish the same ordered outputs. The complete engine,
runtime, and generated-product suites pass with these corrections.

The five findings from the exact-head review of `fcc87b5ed` are corrected in
E4H: the 6,333 value-system occurrences have current exact classifications;
inclusive ranges reject reversed endpoints before writing; n-choose-k rejects
source integers that cannot be represented exactly as `u128`; hold-state
requires exact schema identity; and an explicit root already compiled as a
dependency is published without recompiling it. The correction also closes the
ordinary FizzBuzz product surface with resident string access, assignment,
conversion, equality and transpose operations, plus retained modulo and vector
logic. The native and browser products execute the same source through the
resident route with zero fallback, and indexed string kernels use two-pass
validation without per-turn temporary index allocations.

## Interpreter-directory audit

The exact E3-to-E4 interpreter diff contains six paths:

| Path | Disposition |
| --- | --- |
| `src/engine/src/interpreter/bytecode/mod.rs` | Deleted; old bytecode execution module. |
| `src/engine/src/interpreter/bytecode/registers.rs` | Deleted; old bytecode register file and checkpoint machinery. |
| `src/engine/src/interpreter/tests/bytecode.rs` | Deleted; it tested the removed interpreter bytecode product path. |
| `src/engine/src/interpreter/mod.rs` | Reduced to private semantic-compiler planning machinery; shipping execution, bytecode execution, profiling, output queries, and generic executor entry points are removed. |
| `src/engine/src/interpreter/tests/checkpoint.rs` | Retained only as compiler-planning correctness coverage under `semantic-compiler`. |
| `src/engine/src/interpreter/tests/mod.rs` | Removes the deleted bytecode test module. |

No surviving change in this directory implements shipping source or bytecode
execution, fallback execution, runtime profiling, runtime output queries, or a
standalone interpreter product.

## Local validation

All commands below passed on E4H `f6555296d83ceb4578235bc0d0f1e57013875a6a`:

| Contract | Command/result |
| --- | --- |
| Complete runtime | `cargo +nightly-2026-03-03 test --locked -p mech-runtime --all-features`: 585 library tests, all integration and compile-fail guards, the controlled 4,096-turn D3 evidence lane, and doctests passed in one session. |
| Generated products | `cargo +nightly-2026-03-03 test --locked -p mech-build --all-features`: all 117 unit tests plus every generated materialization, host, scalar, fixed/dynamic matrix, arity, planning, registry, and doc-test target passed. |
| Engine owner | `cargo +nightly-2026-03-03 test --locked -p mech-engine --lib --no-default-features --features full_compiler,resident-artifact`: all 286 tests passed. `cargo +nightly-2026-03-03 check --locked -p mech-engine --no-default-features --features source` also passed and proves the source marker alone does not select compiler planning. |
| Retained host owners | Exact all-feature owner suites for `mech-browser`, `mech-console`, `mech-scene`, `mech-time`, and `mech-timer` passed after the semantic-compiler feature-boundary correction. |
| Retained operation regressions | Exact generated ternary inclusive-range, fixed-matrix addition, dynamic-matrix addition, poisoned output-seed arities, n-body 4,096-turn, public orbit-viewer, initializer-state, and frozen-EKF targets passed. |
| Retained profiles | Full-source runtime, full compiler, terminal all-features, engine all-features check, and WASM source profile passed. |
| Ordinary product canary | `mech run examples/working/fizzbuzz.mec` completed through the resident native source route and produced `bool` / `true`. The equivalent encoded document passed in headless Chrome through `mech-wasm`: 1 test passed, 15 filtered out. |
| Bytecode v1 | The regenerated corpus passed its deterministic check for 20 fixtures across five fresh child processes; the root resident bytecode suite passed 19/19. |
| Static architecture | `bash scripts/check-static-distribution-profiles.sh static` passed compiler-planning quarantine, production resident routing, module layout, source-catalog entrypoint, and the static distribution profile. Standard/full distribution, packaging, and native-host catalog checks also passed. |
| Value-system inventory | `python3 -B scripts/generate-value-system-inventory.py --check` reports the inventory is current. Exact occurrence classification passed for all 6,333 sites; the full checker reported only the controlled Gate B stale-evidence finding allowed until F0. |
| Formatting | `cargo fmt --all -- --check` passed. |
| Benchmark immutability | `git diff --exit-code 3610c66961dcc2fa23aed05833b4722ae34790c0 -- benchmarks/runtime` passed; E4 changes no benchmark source or evidence. |

## Change accounting

Against exact E3 base `3610c66961dcc2fa23aed05833b4722ae34790c0`,
the final E4 stack changes 298 files with 38,562 insertions and 38,737
deletions (net `-175`). The final stabilization commit contains only
deterministic contract reconciliation, guards, and closeout documentation; its
exact SHA is recorded in PR evidence because a commit cannot contain its own
content-addressed identity.

Remote selected/full CI results and the exact-head review disposition are PR
evidence and must be added after the pushed E4I head is known.
