# Runtime test topology

Runtime tests are organized by the boundary whose invariant they prove. Keep
tests close enough to use the narrowest useful interface, while keeping public
API and compile-fail coverage outside the library implementation.

## Test categories

### Local unit tests

Small tests for one private algorithm or data structure may remain inline in
the production module. An inline test module should be short, should not define
reusable fixtures, and should not assemble a complete `MechRuntime` or register
providers, effects, capabilities, or stores.

### Private coordinator tests

Scenarios that exercise a private runtime coordinator live in a sibling
`tests/` directory selected with a test-only `#[path]` declaration. For
example, execution scenarios live under
`src/runtime/src/runtime/execution/tests/`, and transaction protocol scenarios
live under `src/runtime/src/runtime/transaction/tests/`.

Host-input scenarios are grouped by invariant under
`src/runtime/src/runtime/input_tests/`. These tests may inspect private runtime
state without widening the production API.

### Public integration tests

Tests that prove behavior available through the public `mech-runtime` API live
under `src/runtime/tests/`. They should consume the crate as an external user
would and must not depend on private implementation details.

### Compile-fail boundary tests

Tests that prove an API cannot be used or escaped live under
`src/runtime/tests/ui/sealed/`. The `src/runtime/tests/sealed_api.rs` trybuild
harness owns these fixtures and their expected `.stderr` output.

### Host conformance tests

Host-provider, manifest, grant, and resource-interface conformance belongs in
public integration tests such as `src/runtime/tests/generic_host.rs` and
`src/runtime/tests/resource_preflight.rs`. A fake host or provider that models
one contract stays in the file that exercises that contract.

### End-to-end transactional scenarios

A scenario that crosses admission, reactive execution, integrity validation,
effect staging, durable commit, and recovery lives at the highest boundary
that owns the invariant. Internal orchestration acceptance tests belong under
`src/runtime/src/runtime/input_tests/` or the relevant coordinator's sibling
`tests/` directory; public-only scenarios belong under `src/runtime/tests/`.
Runnable demo scripts may select a focused test, but should not duplicate its
scenario implementation.

## Placement rules

- Keep local algorithms inline only when the complete test module is small and
  has no reusable fixtures.
- Put private coordinator scenarios in sibling `tests/` directories.
- Put public API tests under `src/runtime/tests/`.
- Keep sealed API tests under `src/runtime/tests/ui/sealed/`.
- Put genuinely shared private fixtures under
  `src/runtime/src/runtime/test_support/`. This module remains test-only and
  must not become a production API.
- Keep a scenario-specific fixture with its scenario. A large or specialized
  fixture is not shared merely because it is reusable in theory.
- Do not create `misc.rs`, `common.rs`, `other.rs`, or a generic regression
  bucket. Place a test with the subsystem whose invariant it proves.
- Use behavior and invariant names, not development-round terminology, for
  tests, fixtures, symbols, resources, comments, and assertion messages.
- Do not build a universal test harness. Share only small fixtures that are
  genuinely repeated; keep meaningful setup and assertions visible in each
  scenario.
- Import dependencies explicitly. A narrow import from a parent test module is
  acceptable, but wildcard imports from `super` or `crate` are not.

When a script or CI job uses an exact test filter, preserve the test function
name. If it also depends on the fully qualified harness path, retain that path
with a thin wrapper and keep the scenario body in its owning module.

## Naming rule

Use:

```text
<subject>_<condition>_<expected_outcome>
```

The name should identify the owner, the relevant condition, and the observable
contract. Examples:

```text
transaction_store_failure_keeps_transaction_active
invalid_integrity_turn_aborts_staged_effect
foreign_context_cannot_stage_transaction_work
```

Do not rename an externally filtered test merely to improve its wording.

## Critical test paths

| Contract | Location and anchor |
| --- | --- |
| Staged receiver integrity acceptance | `src/runtime/src/runtime/input_tests/integrity.rs` — `integrity_invalid_host_input_aborts_staged_receiver_before_commit`. The shell and PowerShell transactional-integrity demos select this test by its stable function name. |
| Final explicit integrity revalidation | `src/runtime/src/runtime/program_transaction/tests/integrity.rs` — `final_explicit_commit_revalidates_without_consuming_transaction`. |
| Runtime-service reentrant-borrow recovery | `src/runtime/src/runtime/reactive_transaction/tests/service_borrow.rs` — `reentrant_runtime_service_borrow_returns_structured_error_and_recovers`. |
| Module graph rollback | `src/runtime/src/runtime/module/tests/rollback.rs` — the rollback suite includes `explicit_abort_discards_provisional_graph` and `retained_root_failure_rolls_back_graph_events_and_program`. |
| Indeterminate store commit | `src/runtime/src/runtime/transaction/tests/indeterminate.rs` — `store_commit_panic_is_indeterminate_and_never_rolled_back`. |
| Sealed affine-journal boundaries | `src/runtime/tests/ui/sealed/`, driven by `src/runtime/tests/sealed_api.rs`. Journal construction, participant escape, reuse, and multiple-operation cases remain compile-fail fixtures with checked diagnostics. |
| macOS workspace-watch regressions | `src/runtime/src/workspace/watch.rs` — `workspace::watch::tests`, including the macOS-only `macos_temp_directory_alias_handles_missing_watch_paths`. The macOS CI job filters this module directly. |

The stacked core/interpreter reorganization adds the companion
`docs/testing/core-interpreter-test-topology.md` guide for journal, reactive
plan, checkpoint, and bytecode test ownership.
