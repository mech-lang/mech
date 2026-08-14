# Gate E1 teardown log

This is the working handoff log for the forward-only E1 teardown. It records
what was removed, which retained v0.4 capability was checked before removal,
and what a subsequent hardening round still needs to inspect.

## Decision rule

- A surface with no resident owner is deleted with its callers, wrappers,
  tests, configuration, exports, and implementation.
- E1 does not invent resident semantics to preserve legacy-only behavior.
- A deletion stops only when it would break an explicitly retained v0.4
  resident capability: source or bytecode execution, native/WASM/browser
  routing, capability enforcement, resident external effects, replay, or an
  established shipping host.

## Completed slices

| Slice | Result | Retained owner or boundary checked |
| --- | --- | --- |
| Resident source compiler isolation | Added `ProgramCompilerSession`; removed duplicate planning services | Resident source-to-artifact compilation |
| Product entry points | Removed legacy interpreter product entry points, retired REPL/test runtimes, legacy loader fallback, and legacy-only examples/live fixture | Resident CLI, build, and serve entry points |
| Build and benchmark callers | Converted build planning tests to compiler sessions; removed legacy transaction and Gate B benchmark lanes | Resident artifact planning and current resident benchmarks |
| Interpreter facade | Deleted `runtime::legacy_interpreter` after migrating or deleting its remaining callers | No shipping resident owner |
| Console provider wrappers | Deleted interpreter-backed provider tests | Provider and capability owners remain independently tested |
| Browser DOM execution wrappers | Deleted two configuration tests that executed DOM assignments through the retired interpreter; retained the configuration test that proves filtered browser hosts and grants construct successfully | Browser configuration, authority, provider preflight/deferred-delivery tests, and generic resident authorization/effect suites |
| CLI module executor | Deleted the private interpreter executor and duplicate module-execution tests; retained path canonicalization and build-tool configuration tests | Resident `mech build` tests cover sibling/nested imports, missing-dependency atomicity, and caller root order |
| CLI legacy resource-alias execution | Deleted the interpreter-only CLI/browser URI alias suite and source execution from CLI configuration tests; retained input classification, runtime construction, reserved-name rejection, and exact effective-grant assertions | CLI configuration and capability-grant owners |
| Retired CLI feature edges | Deleted the dangling root `test` and `repl` features, removed them from `cli`/`full-cli`, deleted the empty WASM `repl` feature, and removed stale parser/dispatch assertions for the absent `mech test` command | Resident `run`, `build`, `serve`, `format`, and `bundle-web` commands remain; `run` still owns explicit unsupported-empty/REPL route handling |
| Full production feature closures | Removed `legacy-interpreter` from root `full_runtime` and runtime `full_runtime`/`full_source`/`full_compiler`; changed the production-routing guard so standard, full, and WASM product profiles all require its absence | Full value/operation/source/compiler selection remains unchanged; only the obsolete engine edge is removed. The explicit migration feature remains temporarily for legacy-only test target teardown |
| Root legacy feature forwarding | Deleted the orphan root `legacy-interpreter` feature after every product profile stopped selecting it | No root feature or shipping distribution referenced it; the runtime-local migration feature remains temporarily isolated to three integration-test targets |
| CLI REPL state propagation | Deleted always-false REPL fields from root flags, parsed/prepared run options, and execution plans; simplified target resolution now that no command or feature can request a REPL | Empty production runs still fail at the existing resident-target boundary; inline source, path/config resolution, durability, and live-drain behavior are unchanged |
| Materialized CLI config retention | Deleted the unused `RunExecutionPlan.loaded_config` copy after host settings, grants, runtime settings, paths, capability authority, and the config event are materialized into owned plan fields | Run behavior consumes only the materialized plan; no product path read the retained parse document |
| Obsolete WASM console/interpreter flags | Deleted the unreferenced `run_program`, output-value, document-console, `eval`, `whos`, `help`, `clc`, `clear`, and `code` crate features after their Rust implementations disappeared | Browser project construction, resident routing/source, served authority, and concrete browser host features remain unchanged; document-console commands implemented in the shipping JavaScript controller are not Cargo feature-gated |
| Retired command integration tests | Deleted the Linux non-UTF-8 `mech test` harness, the wholly REPL-gated Ctrl-C harness, and the Windows `mech test` module-graph case | The commands/features no longer exist. Windows resident `run` path/import/directory tests and `build` source-to-bytecode coverage remain in the same file |
| Retired command documentation | Deleted the dedicated REPL and `mech test` pages, removed their command/navigation/deployment entries, and changed the production-routing guard from requiring migration wording to requiring page absence | Resident run/build/serve/format/bundle documentation remains; language-level integrity constraints are retained separately |
| Validation command/report documentation | Removed `mech test` invocations and the deleted reporter's JSON/Mech schemas while preserving invariant syntax, rules, and examples | Runtime integrity-constraint semantics remain documented without advertising a deleted CLI owner |
| Root engine-interpreter suite | Deleted `tests/interpreter.rs` and its private catalog helper; the suite directly constructed `MechProgram` and executed source with `run_string` | No resident product entry point or artifact route used the suite. Resident compiler, artifact-equivalence, operation, and product fixture suites remain |
| Disabled CLI REPL-host coverage | Deleted the two REPL-only imports, stdin harness, and four tests gated by the already-removed root `repl` feature; left every resident run/config/capability/import assertion in the mixed file unchanged | The REPL product surface and feature no longer exist; the retained CLI-host suite continues to exercise resident execution and host effects |
| Legacy `whos` distribution surface | Removed the root `whos` feature from standard, full, and base language profiles and deleted its sole helper, which read symbols through `MechProgram.interpreter()` | Resident pretty-print/compiler features remain selected independently; no command, WASM surface, or shipping caller referenced `whos` |
| Root interactive-terminal helpers | Deleted uncalled prompt/help/list/symbol/tree/screen-clear helpers and their unowned root `crossterm`/`tabled`/`chrono` dependencies | Retained file formatting, resource loading, UUID generation, CLI commands, and crate-level pretty-print feature forwarding are unchanged |
| REPL command grammar | Deleted the syntax crate's uncalled `ReplCommand` parser and public re-export after every interactive-console caller and product feature was removed | Ordinary Mech source, Mechdown, formatter, and Mika syntax modules remain unchanged |
| Browser document developer-evaluation branch | Deleted the dormant controller probe, `.evaluate` call, and rendered-result path; non-command input now unconditionally fails while resident document commands remain | The shipped shim contract now requires the command-only marker and forbids both `state.document.evaluate` and `supportsInteractiveEvaluation`; reset, step, rendered symbols, errors, and output tabs are unchanged |
| Browser legacy-routing assertions | Deleted browser configuration/delegation assertions that constructed `PreferResident`/`LegacyOnly`; retained durability serialization and signature-tamper coverage | Browser product authority already accepts only required-resident routing, so rejected interpreter routing should become unrepresentable when the runtime enum is collapsed |
| Legacy routing configuration input | Removed `prefer-resident` and `legacy-only` from runtime configuration lowering; only `require-resident` can now be constructed from a shipping config document | The internal migration variants remain temporarily until their test/runtime callers are removed, but no textual product configuration can select or fall back to the interpreter |
| Artifact loader failure test | Moved malformed-bytecode coverage from the migration loader/options facade to `load_production_bytecode_program` and deleted the duplicate engine-selection default assertion | The shipping bytecode route still proves invalid bytecode fails closed without installing any program |
| Resident loader test ownership | Migrated every retained source, root, bytecode, authority, durability, lifecycle, and n-body assertion in `resident_program::tests` to the production loaders; deleted fallback selection, legacy-only load/unload, legacy-entrypoint exclusion, and private legacy-program-claim assertions | The suite now constructs only the shipping resident route, while preserving source/bytecode identity, import closure, activation, authorization, bounded retention, transaction exclusion, trajectory, and effect behavior |
| Migration program-loader facade | Deleted `load_source_program`, `load_root_program`, `load_bytecode_program`, `load_resident_with_options`, and `RuntimeProgramLoadOptions` after their last resident test callers moved to production loaders | Production source/root/bytecode loaders now pass durability directly into activation; execution-engine selection is absent from the program-load API |
| Browser DOM legacy source wrappers | Deleted the ten root integration assertions that reached the DOM provider through `MechRuntime::run_string`; kept all 14 direct binding, provider-dispatch, manifest-path, wildcard, capability-denial, and operation-scope assertions | Browser provider and capability-kernel behavior remains explicitly covered without retaining an interpreter execution wrapper or weakening those assertions into construction-only checks |

## Current validation

- `cargo fmt --all -- --check`
- `cargo test -p mech-browser --lib --no-default-features --features provider`
  - 54 passed, 0 failed after deleting the browser execution wrappers.
- `cargo test --lib module_execution::tests`
  - 3 passed, 0 failed after deleting the private CLI module executor.
- `cargo test --lib cli::run::tests`
  - 12 passed, 0 failed after deleting legacy URI execution coverage.
- `cargo test --lib cli::app::tests`
  - The supported default distribution compiled after deleting the retired
    `test`/`repl` feature edges; 51 tests passed. The sole failure is the
    pre-existing multiple-root assertion recorded below.
- `python3 -B scripts/check-production-resident-routing.py`
  - Passed after changing the D5 migration assertions into E1 absence
    assertions for the deleted WASM evaluator/developer runtime and REPL
    product surface.
- `cargo check -p mech-runtime --lib --no-default-features --features full_compiler`
  - Passed with the full compiler profile no longer selecting the legacy
    interpreter.
- `cargo check -p mech --lib --no-default-features --features distribution-full`
  - Passed with the root full product distribution no longer selecting the
    legacy interpreter.
- `cargo test --lib cli::runtime_plan::tests`
  - 2 passed, 0 failed after removing unreachable REPL state.
- `cargo test --lib cli::commands::run::tests`
  - The command-level test build passed; this filter contains no tests on the
    current native target.
- `cargo check -p mech-wasm --lib --no-default-features --features browser_project`
  - Passed after deleting obsolete, unreferenced WASM console/interpreter
    feature flags.
- `cargo test --test windows_source_paths`
  - The retained cross-platform target compiled after removing its Windows
    `mech test` case; no tests execute on the current macOS host.
- `cargo test --lib --no-run`
  - Passed after deleting the root engine-interpreter suite and its private
    catalog helper.
- `cargo test --test mech_cli_host`
  - The retained target compiled after deleting only blocks gated by the
    already-absent `repl` feature. Six resident assertions passed and 29
    failed on pre-existing resident product gaps: undefined `@out`/`@env`
    compiler contexts, missing CLI provider semantic contracts, non-state-backed
    outputs, and `LegacyOpaque` operations. The deleted blocks could not have
    affected this result because their feature no longer exists.
- `cargo check --lib` and
  `cargo check --lib --no-default-features --features distribution-full`
  - Both shipping root profiles passed after removing the legacy `whos`
    feature and its direct interpreter access.
  - Both profiles also passed after deleting the uncalled interactive-terminal
    helper cluster and its root-only dependencies.
- `cargo test -p mech-syntax --lib --features base`
  - 14 passed, 0 failed after deleting the REPL command grammar; the root
    shipping library also continued to compile.
- `bash scripts/check-shipped-document-shims.sh` and
  `cargo test --test mech_format_shims`
  - The structural command-only browser contract passed and all five shipped
    formatter-shim tests passed after deleting the dormant evaluation branch.
- `cargo test -p mech-browser --lib --features delegation_signing`
  - 64 passed, 0 failed after removing browser test construction of legacy
    routing policies; retained configuration, delegation, durability, signing,
    and authority coverage remains green.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default config::profile::lower::tests`
  - 10 passed, 0 failed after making legacy routing values invalid in runtime
    configuration documents.
- `cargo test -p mech-runtime --lib --no-default-features --features runtime_default resident_program::artifact_tests`
  - The malformed-bytecode production-loader assertion passed after removing
    its dependency on the migration options facade.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default,resident-routing-source runtime::resident_program::tests`
  - All 27 retained resident program tests passed after migration to the
    production loaders and deletion of interpreter-route assertions.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default,resident-routing-source runtime::resident_program`
  - All 28 resident-program tests passed after deleting the uncalled migration
    loaders and options type.
- `cargo test --test browser_dom`
  - All 14 surviving browser binding/provider/capability assertions passed
    after deleting the ten interpreter-backed source wrappers.

## Stop conditions and weakened or missing contracts

| Discovery | Exact evidence | Retained capability affected? | E1 disposition | Later proof needed |
| --- | --- | --- | --- | --- |
| Browser DOM assignment has no resident semantic admission | Resident activation rejected `Assign` with `ProviderContractMismatch`: the browser provider does not declare `semantic_write_contract` | No. The exercised source operation was owned only by the retired interpreter; browser configuration/routing remains | Deleted the two interpreter-backed execution wrappers. Did not add a browser semantic contract | Prove the surviving browser host can still be configured, routed, authorized, and used by every explicitly retained resident browser operation |
| Deleting the browser test helper also removed a surviving construction assertion | `browser_runtime_injection_filters_non_browser_run_grants` constructed filtered hosts/grants through `RuntimeBuilder` using the shared helper | Yes. Browser routing/configuration is retained | Replaced only the helper with a minimal capability-layer factory; kept the host/grant construction assertion | Keep the configuration test green in both its intended feature profiles; do not replace it with source execution |
| Non-state-backed source output fails resident activation | A simple `answer := 42; answer` source route reproduced `OutputMustBeStateBacked` on the exact D5 base | Not established as an E1 regression | Recorded as pre-existing and excluded from teardown redesign | Verify the retained source fixtures, including normal state-backed programs; open separate work only if an explicitly retained v0.4 capability lacks coverage or fails |
| No-legacy compilation exposes large dead-code clusters | Compiler warnings identify retained-program execution, live state, private program transactions, ledger/outbox, and related helpers with no active caller in narrow profiles | Potentially. Some ledger/outbox/effect types have independent resident owners | Do not delete by warning alone; use production reachability and focused retained-capability tests for each slice | Produce a final reachability inventory showing which surviving resident component owns every remaining cluster |
| The root `build` feature is not an independently complete product profile | `cargo test --lib --no-default-features --features build ...` cannot see resident-route failure types, compiler entry points, selected host catalogs/factories, or the standard source catalog | No E1 regression established; the standard distribution profile supplies those closures and the focused tests pass there | Validate build teardown in a shipping distribution profile; do not add unrelated feature edges during this deletion slice | Decide in distribution hardening whether `build` is intended to be standalone. If so, freeze that closure explicitly; otherwise enforce only supported root profiles |
| A context-addressed AST detector remains in CLI run code with no observed caller | `rg` finds only the detector definitions; the no-legacy build reports every helper as dead code | No runtime use was found, but CLI routing is retained and the deletion guard requested stronger proof | Leave the detector untouched during the current E1 slice; it does not keep the interpreter or its tests reachable | Resolve in E2 API/routing cleanup after tracing every `RunInputMode` consumer and freezing the retained input-classification contract |
| Route-policy variants cannot be removed before their runtime callers | Auto-review rejected removing `PreferResident`/`LegacyOnly` while legacy load and route code still compiled | No retained behavior requires the variants, but that edit order would create a broken intermediate build | Reverse the order: delete the caller/feature closure first, then remove the variants | Final E1 search must show no legacy load API, route, or config value before deleting the enum variants |
| `generic_host` mixes retained kernel coverage with legacy source execution | The first ten tests directly cover host registration and capability identity; later robot/plotter tests call `run_string` | Yes for the first group; no resident owner is established for the interpreter-only group | Do not delete the target wholesale. Split or migrate the retained group after the legacy runtime caller closure is gone | Keep host registration, base-URI identity, and capability tests green without the interpreter feature |
| `sealed_snapshots` mixes duplicate sealing coverage with runtime-boundary assertions | `sealed_api` already glob-compiles `runtime_value_snapshot_raw_access`, while three tests use the retained program/module/host executor | Snapshot isolation is retained even though those particular execution paths are legacy | Do not delete the target wholesale before mapping the resident snapshot-boundary owners | Cite the surviving snapshot deep-copy, resident host-boundary, and compile-fail tests before pruning this target |
| Runtime diagnostic binaries are legacy-only, but still designated validation owners | `module_smoke`, `address_target_diagnostics`, and `source_import_dependency` call the legacy module executor; `scripts/run-runtime-source-targets.sh` invokes all three | Their execution path is not retained, but the source-target script currently assigns them module/import/diagnostic coverage | Auto-review rejected deleting targets and script calls in one slice. Leave them unchanged and continue elsewhere rather than ending E1 | Map each observable assertion to resident build/compiler/authorization tests, change the validation-owner list explicitly, then delete the three legacy harnesses |
| The legacy `module_smoke` integration target cannot yet be deleted independently | The target imports the deleted `LegacyInterpreterTestExt` and calls `run_module`/`run_string`, but Auto-review rejected removing its Cargo entry and source while module/import coverage is still assigned to the active diagnostic validation owner | The interpreter execution path is not retained; equivalent resident module/import coverage has not yet been documented at the validation-owner boundary | Left the target and Cargo entry unchanged after the rejection. Continued E1 on an independent dead-feature slice instead of retrying or ending the operation | Remap every assertion owned by `scripts/run-runtime-source-targets.sh` to named resident build/compiler/authorization tests, update that owner list, and only then delete the stale integration and diagnostic targets |
| The workspace service has one legacy execution assertion inside otherwise retained coverage | `ServerWorkspaceSession` is used by `mech serve`; `server_workspace_session_accepts_an_explicit_function_catalog` alone calls `run_string` | Yes for workspace loading/watch/events and explicit catalog injection; no for the interpreter call itself | Keep the service and its retained tests. Defer changing the mixed assertion until an equivalent catalog-use owner is identified or the legacy runtime API is removed with its caller closure | Preserve product-level workspace open/load/watch/event coverage; prove catalog injection through the resident compiler/activation path rather than weakening the test to construction only |
| Migration load APIs mix resident tests with obsolete fallback tests | `load_source_program`, `load_root_program`, and `load_bytecode_program` are test/legacy-gated, but dozens of resident admission, durability, source/bytecode-equivalence, and product n-body tests still call them with `RequireResident`; a smaller group explicitly tests `PreferResident`/`LegacyOnly` | Yes for the resident assertions; no for fallback/legacy-only selection | Remove production feature reachability first. Migrate resident assertions to production loaders and delete fallback assertions in a dedicated test-owner slice before deleting these APIs | The final suite must preserve resident source/bytecode identity, admission, durability, limits, atomicity, and n-body behavior without exposing engine-selection options |
| `MechProgram` has two different owners that must be separated | `ProgramCompilerSession` creates short-lived compiler planning objects; `MechRuntime` still stores a retained `program` and private program transaction/live-state coordinator | Yes for compiler planning; no shipping route should own the retained legacy executor | Do not delete the engine type or compiler session. Remove the runtime field together with execution/session/program-transaction/live-state callers in compile-checked slices | Final reachability must show `MechProgram` only in compiler/bytecode planning or explicitly retained non-runtime tooling, never as `MechRuntime` state |
| The generic-host ownership split was not accepted as a generated truncation | The intended split kept lines 1–472 (the nine direct host/capability tests), removed the legacy extension import, deleted the robot/plotter execution tail, and dropped the target's legacy feature requirement; Auto-review reported it could not verify that the resulting target omitted every helper/import/test | Yes for the nine direct tests; no for the interpreter execution tail | Leave the file unchanged and continue elsewhere. Do not retry the same generated truncation indirectly | Perform an explicit source-level split in E2 or a later E1 pass: compile the retained target without the legacy feature before deleting the old execution tail |
| A CLI build test still expects multiple resident source roots | `build_multiple_source_roots_in_caller_order` fails with `ResidentRouteFailure(MultipleRootsUnsupported)` under the default distribution; the dangling-feature patch changes no build routing or source behavior | Potentially: this is a source-build product-contract mismatch, but it predates this E1 slice | Do not change build behavior or delete the assertion as part of retired `test`/`repl` feature removal | Decide in E2 whether the retained v0.4 build contract supports ordered multiple roots or explicitly rejects them, then align the product test and documentation |
| The retained CLI-host product suite is not resident-clean after the legacy cut | `cargo test --test mech_cli_host` compiled, then reported 6 passed and 29 failed. Failures group into `UndefinedContext` for `@out`/`@env`, missing provider semantic contracts for CLI read/send, `OutputMustBeStateBacked`, and resident `LegacyOpaque` operations | Yes. CLI run, source/import routing, capability enforcement, and CLI host effects are explicitly retained | Do not restore the deleted REPL or legacy route, and do not invent these semantics during E1. Keep the resident tests and record the full failure as a hardening input | E2 must map each failing observable assertion to compiler context declaration, provider operation contracts, state-backed output policy, or operation lowering; establish a green retained CLI-host session before final F0 |
| Narrow syntax library test profiles lack the Mechdown formatter closure | `cargo test -p mech-syntax --lib` fails because Mechdown tests import `Formatter`; adding only `formatter` then fails because that module also requires `mika` and `variable_define`. The declared `base` profile passes all 14 tests | No E1 regression: the removed `ReplCommand` module defines none of these types or features | Do not add feature edges during the REPL grammar deletion. Validate with the declared `base` profile and keep the narrower closure mismatch visible | E2/distribution hardening should gate Mechdown/formatter tests by their actual features or declare a dedicated standalone test closure |
| The runtime `source` and `source_default` features are not complete lib-test closures | Both `cargo test -p mech-runtime --lib --no-default-features --features source ...` and the same command with `source_default` fail before running config tests because three reactive transaction fixtures do not implement `MechFunctionCompiler` | No E1 regression: runtime production code compiled, and routing config lowering does not touch those fixtures | Do not add compiler implementations to legacy reactive tests. The focused configuration suite passes all 10 tests under the declared `compiler_default` profile | E2 test ownership should gate those reactive fixtures by their real feature requirements or delete them with the retained-program executor |

Auto-review stops are recorded here and work proceeds on another dependency
slice. They are not treated as requests to restore legacy behavior, reopen D5,
or add resident semantics.

When a proposed deletion actually breaks a retained capability, add a row here
before changing code. Record the smallest failing command and error, identify the
surviving product owner, and distinguish a real E1 stop from a legacy-only test
that should be removed.

## Follow-up inventory

These are observations to inspect in later E1 slices or the subsequent
hardening round. They are not requests to restore legacy behavior.

- Determine the remaining production reachability of browser DOM
  `read`/`preflight_write`/`prepare_write`. The provider has no resident
  semantic admission contract; delete any operation path that is reachable
  only from the old executor while preserving retained browser routing and
  host configuration.
- Remove the remaining `legacy-interpreter` feature closure and its
  test-only CLI/runtime callers.
- Delete `MechRuntime`'s retained `MechProgram` owner and its private program
  transaction coordinator once their remaining callers are removed. Keep the
  public store transaction and shared resident effect journal.
- Classify legacy integration targets (`module_smoke`, `generic_host`, and
  `sealed_snapshots`) by surviving contract. Delete legacy-only coverage; move
  the sealed raw-access assertion to the surviving sealed API suite.
- Re-run static architecture inventories after the production surface is
  settled. Snapshot churn during intermediate deletion slices should not drive
  runtime redesign.
- The no-legacy feature build currently exposes substantial dead-code warnings
  in old execution, ledger, outbox, live-state, and retained-program paths.
  Use reachability, not warning suppression, to decide the remaining deletion
  order.
- A simple source program whose final output is not state-backed previously
  reproduced `OutputMustBeStateBacked` on the D5 base. Do not broaden E1 to fix
  that pre-existing activation behavior; verify retained product fixtures and
  record any actual retained-capability break separately.

## Final E1 proof still required

- No production import or feature enables the legacy interpreter.
- No shipping entry point selects a legacy program route.
- Retained source and bytecode fixtures pass on native and WASM/browser paths.
- Capability enforcement, resident external effects, replay, and established
  shipping hosts pass their focused suites.
- Static distribution, value-system, production-resident, and packaging
  contracts match the final intentionally reduced surface.
