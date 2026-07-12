# Runtime warning inventory

Captured after removing the crate-level `#![allow(warnings)]` from `src/runtime/src/lib.rs` and running:

```bash
cargo check -p mech-runtime --all-targets
```

## Resolved warnings

| Warning | Classification | Resolution |
| --- | --- | --- |
| `src/runtime/src/workspace/mod.rs`: `pub use self::discovery::*` re-exported no public items | resolved by removal of an empty re-export | Removed the empty public glob re-export. `mod discovery;` remains internal and workspace code continues to use the implementation. |
| `src/runtime/src/host/actor.rs`: unused `services` in `ActorMessageKindHostFunction`, `ActorMessagePayloadHostFunction`, and `ActorStateIdHostFunction` | trait-required unused parameter | Renamed only those trait-required parameters to `_services`; these operations intentionally read actor-turn data from `RuntimeContext`. |
| `src/runtime/src/host/arg.rs`: unused typed-value binding in `Value::Typed(value, _) => todo!()` | intentionally deferred typed-value behavior | Changed the pattern to `Value::Typed(..) => todo!()` without implementing typed host-argument conversion in this pass. |
| `src/runtime/src/runtime/errors.rs`: `RuntimeModuleExportLinkError` never constructed | resolved by deletion | Repository search with `rg 'RuntimeModuleExportLinkError|ContextAddressReadUnsupported' .` found no live production caller for this error type, so the stale error and its `MechErrorKind` implementation were deleted. |
| `src/runtime/src/runtime/errors.rs`: `ContextAddressReadUnsupported` never constructed | resolved by deletion | Repository search with `rg 'RuntimeModuleExportLinkError|ContextAddressReadUnsupported' .` found no live production caller for this error type, so the stale error and its `MechErrorKind` implementation were deleted. The existing smoke test that asserts this stale name is not surfaced remains valid. |
| `src/runtime/src/capability/kernel.rs`: `SharedCapabilityKernel::inner` never used | resolved by deletion | Repository searches for `SharedCapabilityKernel::inner` and `.inner()` found no live caller of this accessor; the private `inner` field remains as the kernel state. |
| `src/runtime/src/workspace/watch.rs`: `local_workspace_target_watch` never used by production code | resolved by deletion | Deleted the production singular wrapper and added a test-only `single_target_watch` helper that asserts exactly one watch before unwrapping. |
| `src/runtime/src/workspace/watch.rs`: `RuntimeWorkspaceWatcher::from_parts` never used in tests | resolved by deletion | Deleted the unused private test constructor rather than preserving unused test scaffolding. |

## Config-profile visibility cleanup

The broad public glob re-exports for config pipeline implementation phases were removed. The supported external entry point remains `parse_config_document(...)`.

Public config-profile types intentionally retained through explicit re-exports:

- `ConfigProfileOptions`
- `ConfigValue`
- `MechConfigDocument`
- `RuntimeConfigPatch`
- `RuntimeLimitsPatch`
- `DiagnosticsConfigPatch`
- `ServeHostConfig`
- `RunHostConfig`
- `ConfigCapabilityGrant`
- `ConfigCapabilityKind`

Pipeline implementation types such as `ConfigAnalyzer`, `ConfigCompiler`, `ConfigEvaluator`, `ConfigExtractor`, `ExtractedConfigProgram`, `ConfigProgram`, `ConfigItem`, `ConfigExpr`, `ConfigFunction`, `ConfigLet`, and `ConfigLowerer` are no longer publicly re-exported.

## Final command summary

- `rg '#!\[allow|#\[allow' src/runtime`: no matches.
- `cargo check -p mech-runtime --all-targets`: finished with zero warnings.

Final runtime warning count for `cargo check -p mech-runtime --all-targets`: **0**.
