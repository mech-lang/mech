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
| Workspace-session explicit catalog assertion | Replaced its sole `run_string` call with production resident source loading through an explicitly injected shipping catalog | Workspace discovery/watch/snapshot behavior is unchanged, and the test still proves that `open_with_function_catalog` installs and uses the supplied catalog rather than merely accepting it |
| Runtime-builder interpreter assertions | Deleted bare/intrinsic `run_string` catalog assertions and direct retained/isolated `MechProgram` catalog inspection; kept runtime catalog identity, empty-catalog shape, and execution-mode assertions | Production resident catalog use is covered by the workspace session and resident-loader suites; builder coverage no longer assigns ownership to retained interpreter programs |
| Implicit retained-program transaction test | Deleted the single test module that asserted an implicit `MechProgram` operation committed its program symbols together with store/events | Generic transaction commit/event/store assertions remain, while resident publication and accepted-turn state are covered by the resident program suite; no surviving owner uses implicit interpreter program publication |
| Host-callback failure coverage owner | Added the returned-error containment assertion directly to the surviving runtime-managed host transaction suite, without source execution or a retained program | The test preserves the exact error kind and proves transaction/effect/program-operation guards are cleared while runtime health remains healthy |
| Legacy program host-callback wrapper | Deleted `transaction/tests/program/extension_failures.rs` only after its retained error-containment contract moved to the host transaction owner | The surviving test is stronger at the permanent boundary: it asserts returned error identity, transaction/effect/program-operation cleanup, and healthy runtime state without constructing `MechProgram` source |
| Transaction context identity owner | Moved task/actor/message/state identity mismatch coverage from an interpreter-program operation to the surviving store transaction context suite | The retained test now proves the same `RuntimeTransactionContextMismatch`, no durable write, continued transaction ownership, and owner cleanup without relying on `run_string` |
| Explicit retained-program ownership tests | Deleted five tests for provisional interpreter symbols, single-transaction `MechProgram` ownership, and failed interpreter-operation ownership release after moving their one generic context-identity contract | Store transaction ownership/context checks remain; resident program loading has its own active-transaction exclusion test and does not expose provisional interpreter symbols |
| Retained-program rollback tests | Deleted three tests whose observable state was interpreter symbols, interpreter plan identity, private live bindings, and `ProgramCompleted`/`ProgramFailed` legacy events | Surviving store abort tests own staged event/effect rollback, failed abort publication, ownership cleanup, aggregation, and poisoning; resident turns publish or abort candidate state without the interpreter journal |
| Retained-program poisoning tests | Deleted fault-injection tests for missing legacy program envelopes and irrecoverable interpreter-plan restoration, along with their now-unreachable test-only fault injector | Store abort tests retain poisoning on incomplete cleanup, and reactive/host suites retain unwind and health behavior; resident execution never restores a mutated published interpreter plan |
| Duplicate retained-program effect assertions | Deleted the program-wrapper copies of failed resource staging and post-store participant commit failure | Generic effect transaction tests already prove empty staging on failure, full participant commit ordering, indeterminate classification, committed transaction retention, event evidence, and poisoning without relying on interpreter symbols |
| Interpreter-program savepoint ownership | Deleted explicit-operation and structural-replacement rollback tests whose observable contract was provisional interpreter symbols, plan identity, retained `MechProgram` object identity, and private `program_transaction_owner` state | Generic effect, capability, module, store, and reactive savepoint suites retain their own rollback contracts; resident publication never replaces or restores the interpreter-owned program object |
| Legacy program-integrity transaction wrapper | Deleted six tests whose observable state was interpreter symbols, private program savepoints/ownership, `ProgramCompleted`/`ProgramFailed`, and legacy integrity-audit append behavior; removed their transaction-effect helper | Checked resident execution owns integrity admission: invalid candidates do not publish, valid retry matches a fresh instance, and checked/unchecked modes remain explicit. E1 does not recreate the interpreter event/audit lifecycle in the resident coordinator |
| Event-retention tests assigned to the program coordinator | Moved the active-transaction and outer-commit bounded-history assertions unchanged from the private program test module to the store event-publication owner | Both tests exercise only store transactions, event retention, cleanup, and Gate A no-snapshot probes; no interpreter or program operation is involved |
| Mixed generic-host target preservation stage | Added a legacy-free `generic_host_kernel` integration target containing the nine retained host-registration, manifest-instance, capability, and exact resource-identity assertions before touching the old interpreter-backed mixed target | This compile-checked intermediate makes the surviving owner explicit and avoids weakening execution assertions into mere construction checks: only the original nine direct kernel assertions are copied |
| Interpreter-backed generic-host target | Deleted the old mixed target after its nine retained direct assertions passed independently; removed robot/plotter source-execution cases and their private fake providers/factories together | The deleted tail asserted operations available only through `run_string`; E1 does not add resident provider semantics for those legacy-only fixtures. The retained kernel target continues to own host construction and resource identity |
| Cumulative value-system inventory refresh | Regenerated `current-inventory.json` after the generic-host replacement; the projection also catches the earlier E1 command/REPL/interpreter/example/benchmark/program-test deletions and the new compiler-session source owner | The generated delta is deletion-dominant (26 fewer workspace Rust files overall) and replaces `generic_host` with `generic_host_kernel`; no value-system production behavior was added by the refresh |
| Interpreter-backed sealed-snapshot target | Deleted three retained-program/module/bytecode mutation wrappers and one duplicate compile-fail assertion | Snapshot unit tests own detached reachable cells and independent clones; `sealed_api` already glob-compiles the same raw-access escape attempt. The deleted wrappers exposed only legacy execution state, not a resident product boundary |
| Legacy `module_smoke` integration target and migration feature | Deleted the 8,000-line interpreter-backed target and the runtime crate's final `legacy-interpreter` feature after naming its surviving owners | Resident root planning owns resolved import closure; shipping CLI tests retain sibling, parent-relative, missing-import, and filesystem-authorization behavior; build suites own source-to-bytecode planning. The legacy suite cannot repair resident product gaps and no shipping feature selected it |

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
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default,resident-routing-source service::workspace_session::tests::server_workspace_session_accepts_an_explicit_function_catalog`
  - Passed after moving the session's explicit-catalog execution assertion to
    the production resident source loader.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default runtime::builder::tests`
  - All three surviving builder assertions passed after deleting its retained
    program and interpreter execution coverage.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default --no-run`
  - The complete compiler-profile runtime library test target compiled after
    removing the implicit retained-program test module.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default transaction_commit`
  - Four surviving generic store/event transaction commit assertions passed.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default runtime_managed_host_error_stays_contained_and_cleans_transaction_state`
  - The new non-interpreter host-layer failure-containment assertion passed.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default runtime::transaction::tests::store::context_identity::transaction_context_identity_includes_task_actor_message_and_state`
  - The migrated store-layer task/actor/message/state identity assertion passed.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default runtime::transaction::tests::store::abort`
  - All eight surviving store/effect abort and rollback assertions passed after
    deleting interpreter-program rollback coverage.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default poison`
  - All 17 surviving runtime poisoning and poison-recovery assertions passed
    after deleting legacy program restoration faults and their injector.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default provider_commit_failure_after_store_commit_is_indeterminate`
  - The surviving effect-transaction owner passed after deleting the duplicate
    retained-program participant-commit wrapper.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default staged_host_effect_records_execution_session_transaction_snapshot`
  - The remaining Gate A execution-session snapshot assertion passed after the
    unrelated retained-program effect wrappers were removed.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default runtime::transaction::program::tests::savepoints`
  - All five surviving generic context, event-retention, budget, and effect
    savepoint assertions passed after deleting the two interpreter-program
    identity and rollback tests.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default runtime::transaction::program::tests`
  - All six remaining program-coordinator probes passed after deleting the
    legacy interpreter integrity/audit wrapper.
- `cargo test -p mech-engine --test resident_ekf_program_execution --no-default-features --features compiler_default,resident-artifact integrity`
  - All three resident integrity tests passed: checked/unchecked selection,
    failed-candidate non-publication, and valid retry equivalence remain owned
    by the resident executor.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default runtime::transaction::tests::store::event_publication`
  - All three store event-publication and bounded-history tests passed after
    moving the two generic retention assertions out of the program coordinator.
- `cargo test -p mech-runtime --test generic_host_kernel --no-default-features --features runtime_default`
  - All nine retained host-registration, manifest-instance, capability, and
    exact resource-identity assertions passed without the legacy interpreter
    feature or source execution.
- `cargo check -p mech-runtime --lib --no-default-features --features compiler_default`
  and `python3 -B scripts/check-production-resident-routing.py`
  - Both passed after deleting the interpreter-backed generic-host target.
- `python3 scripts/generate-value-system-inventory.py --check`
  - Passed after regenerating the cumulative deletion-dominant E1 inventory.
- `python3 -B scripts/check-value-system-contract.py`
  - Completed and failed on cumulative contract maintenance rather than the
    generic-host slice: stale Gate B evidence, six unclassified compiler-session
    `ValRef` aliases, two deleted REPL classifications, and two moved console
    provider classifications. Exact disposition is recorded below.
- `cargo test -p mech-runtime --test sealed_api --no-default-features --features source_default`
  - The full compile-fail glob passed, including the raw snapshot access case.
- `cargo test -p mech-runtime --lib --no-default-features --features compiler_default snapshot::tests`
  - All 18 snapshot isolation, cycle, borrow, collection, clone, and cell
    disjointness assertions passed after deleting the legacy execution wrapper.
- `cargo check -p mech-runtime --lib --no-default-features --features source_default`
  - Passed after removing the old `sealed_snapshots` target.
- `cargo metadata --no-deps --format-version 1`,
  `cargo check -p mech-runtime --all-targets --no-default-features --features compiler_default`,
  and `python3 -B scripts/check-production-resident-routing.py`
  - All passed after deleting `module_smoke` and the runtime-local
    `legacy-interpreter` feature.
- Focused retained import owners:
  - `resident_root_plans_the_resolved_source_import_closure_before_route_selection`
    passed.
  - `mech_run_file_missing_import_reports_dependency` passed.
  - Sibling and parent-relative product cases reproduced the recorded
    `OutputMustBeStateBacked` gap and remain intact.

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
| The auto-discovered resident EKF integration target lacks a usable default feature closure | `cargo test -p mech-engine --test resident_ekf_program_execution integrity` failed to compile because the default-empty engine profile exposed `artifact::requirements` without `mech_core`'s application-requirement helpers; the declared compiler/resident feature closure passed all three matching tests | No E1 regression. Resident integrity execution passes under the feature set that owns the target | Do not alter engine/core features during legacy integrity-test deletion; validate the retained owner with `compiler_default,resident-artifact` | E2/distribution hardening should add an explicit `required-features` declaration or gate the integration target's imports so an unsupported empty-profile invocation does not look like product failure |
| The runtime-only library test profile does not satisfy compiler-bound reactive test doubles | `cargo test -p mech-runtime --lib --no-default-features --features runtime_default runtime::transaction::tests::store::event_publication` failed while compiling unrelated `RuntimeStepProbe` tests because `MechFunctionCompiler` is absent; the same retained store tests pass under `compiler_default` | No E1 regression. The moved store tests themselves require no compiler behavior | Do not add compiler edges to `runtime_default` during test-owner cleanup; use the already-supported compiler test closure | E2/distribution hardening should gate compiler-bound reactive test helpers by `compiler` or define a complete runtime-only library-test closure |
| Cumulative value-system contract metadata is stale after teardown | The full checker reports: Gate B evidence affected by removed legacy benchmark files; six `valref-alias` sites in `runtime/program/compiler.rs` absent from the immutable baseline; deleted REPL `LegacyValue::Empty` classifications; and console `LegacyValue::String` classifications shifted from line 188 to 207 | The generic-host deletion adds none of these uses. Compiler planning is retained, while REPL behavior is deleted and console provider behavior is retained | Do not approve legacy growth, rewrite benchmark evidence, or silently move classifications during this E1 slice. Keep the generated live inventory and record the exact failures | E2 must classify the compiler-session aliases as an explicitly bounded planning boundary (or remove them), delete obsolete REPL classifications, re-anchor the console classifications by fingerprint, and decide when cumulative benchmark evidence is regenerated; E3 may retire migration-only enforcement where the Gate plan requires it |
| Product sibling and parent-relative imports compile but do not activate | Focused `mech_cli_host` cases fail with `ResidentRouteFailure::SemanticUnsupported` / `OutputMustBeStateBacked { slot: CellSlotId(0) }`; the focused missing-import case and resident root import-closure test pass | Yes: ordinary resident source/import execution is retained. This exact gap predates deletion of `module_smoke` and remains asserted by shipping product tests | Delete the interpreter-only suite and keep the failing product assertions. Do not restore fallback or change output semantics in E1 | E2 must give ordinary imported scalar outputs a supported resident publication contract (or explicitly freeze a state-backed source rule) and produce a green sibling/parent product session |
| The runtime `source` and `source_default` features are not complete lib-test closures | Both `cargo test -p mech-runtime --lib --no-default-features --features source ...` and the same command with `source_default` fail before running config tests because three reactive transaction fixtures do not implement `MechFunctionCompiler` | No E1 regression: runtime production code compiled, and routing config lowering does not touch those fixtures | Do not add compiler implementations to legacy reactive tests. The focused configuration suite passes all 10 tests under the declared `compiler_default` profile | E2 test ownership should gate those reactive fixtures by their real feature requirements or delete them with the retained-program executor |
| A minimal intrinsic-only workspace catalog has no resident literal implementation | Migrating the explicit-catalog session assertion from `run_string("result := true")` to production resident loading failed with `SemanticUnsupported ... core/literal: LegacyOpaque` | No retained capability requires an arbitrary incomplete catalog to execute; explicit catalog injection itself is retained | Do not add a literal implementation or fallback in E1. Exercise the same session API with the shipping resident source catalog and record the deleted interpreter-only behavior | E2 should document the minimum resident compiler/runtime catalog contract and make incomplete injected catalogs fail at a clear construction or compilation boundary |
| The private retained-program transaction suite cannot be deleted wholesale before ownership classification | Auto-review rejected deleting its ten files/1,869 lines in one slice because the suite names rollback, effects, integrity, poisoning, and savepoints, all of which also have retained non-program owners | The `MechProgram` coordinator is legacy-only, but generic transaction atomicity, effects, integrity evidence, health, and savepoints remain retained capabilities | Left the entire suite and its module declaration unchanged. Continue with source-level classification and small compile-checked deletions rather than retrying the wholesale removal | Map every assertion to either the private retained-program owner or a surviving store/reactive/resident transaction owner; cite the latter before deleting each legacy duplicate |
| Host-callback error containment cannot be dropped with its legacy program wrapper | Auto-review rejected deleting `transaction/tests/program/extension_failures.rs` because it is the only explicit assertion shown for a returned host-callback error staying inside the execution session and cleaning its transaction | Yes. Host failure containment and transaction cleanup are retained external-effect safety contracts | Left the test/module unchanged. Move the same observable error and cleanup assertions to the surviving host transaction layer before reconsidering the legacy wrapper | A focused non-interpreter host test must prove returned callback failures preserve their error kind, stage no program/effect state, and leave no active transaction; existing panic conversion tests alone are insufficient |

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
