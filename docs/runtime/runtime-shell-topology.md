# Runtime shell topology

`src/runtime/src/runtime/` is the private host-facing runtime shell. Its root
module declares the shell subsystems and preserves the existing public
re-exports; implementation belongs to responsibility-owned files.

This split changes neither public paths nor runtime behavior. It keeps
construction order, resource and transaction boundaries, event timing, limit
accounting, and shutdown cleanup unchanged.

## Ownership

| Module | Ownership |
| --- | --- |
| `builder.rs` | `RuntimeBuilder`, defaults, setters, validation, dependency assembly, extension wrapping, and runtime construction. |
| `live_state.rs` | Live-context templates and snapshots, live registration mode, persistent sends, and live-state validation, commit, snapshot, restoration, and turn-context construction. |
| `state.rs` | `MechRuntime`, its sibling-visible fields, scoped runtime state, debug formatting, identity, configuration, retained-program access, and runtime health access. |
| `resources.rs` | Resource binding models, binding validation, providers, authorization, reads, writes, grants, and failed implicit-operation cleanup. |
| `components.rs` | Component accessors and the existing controlled source-resolver and scheduler-policy replacement methods. |
| `operation_context.rs` | Runtime, task, actor, actor-turn, and historical-transaction contexts plus runtime-identity validation. |
| `limits.rs` | Default budgets, source-byte accounting, turn-duration enforcement, and in-memory event retention. |
| `events.rs` | Event sequencing, construction, immediate or transactional emission, and persisted-event publication. |
| `lifecycle.rs` | Explicit shutdown and drop-time input-driver cleanup. |

`ModuleInstance` belongs to isolated and retained module execution in
`execution/module.rs`. Module import-edge validation belongs to the existing
private module coordinator in `module.rs`.

The subsystem directories below the shell keep their existing ownership.
Private transaction coordination remains documented separately in
[`transaction-topology.md`](transaction-topology.md).

## Root-module rule

`runtime/mod.rs` contains only:

- module documentation;
- module declarations;
- deliberate public re-exports;
- test-only module declarations.

It is not an internal prelude. Production runtime modules import their own
dependencies explicitly, and `MechRuntime` fields are visible only to sibling
runtime modules with `pub(super)`.

## Enforcement

`scripts/check-runtime-wildcard-imports.sh` rejects production
`use super::*`, repeated-parent wildcard imports, and `use crate::*` below the
private runtime tree. Test-owned `tests`, `input_tests`, and `test_support`
trees are excluded.

`scripts/check-runtime-boundaries.sh` runs that audit before the remaining
runtime boundary checks. `scripts/test-runtime-boundaries.sh` exercises
accepted explicit imports, each rejected wildcard form, and every allowed test
tree. The required CI runtime-boundary job runs both scripts.
