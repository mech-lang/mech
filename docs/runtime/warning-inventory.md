# Runtime warning inventory

Captured after removing the crate-level `#![allow(warnings)]` from `src/runtime/src/lib.rs` and running:

```bash
cargo check -p mech-runtime --all-targets
```

## Classification

| Warning | Classification | Resolution direction |
| --- | --- | --- |
| `src/runtime/src/workspace/mod.rs`: `pub use self::discovery::*` reexports no public items | delete | Remove the public glob or make only supported discovery items public. |
| `src/runtime/src/host/actor.rs`: unused `services` in actor host functions | behavioral bug exposed by warning | These functions currently validate arguments but do not use the service context; either connect them to real actor services or remove unsupported host calls. |
| `src/runtime/src/host/arg.rs`: unused `value` in typed-value conversion placeholder | behavioral bug exposed by warning | Implement typed host argument conversion or reject typed values explicitly without binding an unused value. |
| `src/runtime/src/runtime/errors.rs`: `RuntimeModuleExportLinkError` never constructed | delete | Remove the stale public error after repository caller verification. |
| `src/runtime/src/runtime/errors.rs`: `ContextAddressReadUnsupported` never constructed | delete | Remove the stale public error after repository caller verification. |
| `src/runtime/src/capability/kernel.rs`: `SharedCapabilityKernel::inner` never used | delete | Remove the internal accessor unless needed by a real caller. |
| `src/runtime/src/workspace/watch.rs`: `local_workspace_target_watch` never used | delete | Remove the stale helper or connect it to workspace watch setup. |
| `src/runtime/src/workspace/watch.rs`: `RuntimeWorkspaceWatcher::from_parts` never used in lib tests | move behind a test gate | Keep only if it supports tests; otherwise remove with the dead watch helper. |

Additional suppression search:

```bash
rg '#!\[allow|#\[allow' src/runtime
```

returned no remaining runtime warning allowances after this change.
