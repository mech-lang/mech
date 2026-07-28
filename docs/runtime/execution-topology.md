# Runtime execution topology

Runtime execution keeps the `MechRuntime` façade stable while placing each
operation with the subsystem that owns its policy. The module directory is an
organizational boundary only: execution order, transactions, effects,
capabilities, module resolution, errors, and public return values are
unchanged.

## Ownership

| File | Responsibility |
| --- | --- |
| `execution/mod.rs` | Module declarations and the narrow re-exports needed by existing runtime callers. |
| `execution/source.rs` | String, source, tree, and bytecode entry points; source limits; program lifecycle and profiling events. |
| `execution/query.rs` | Read-only program, interpreter, output, symbol, and integrity-report queries. |
| `execution/reactive.rs` | Direct runtime reactive-step entry points and turn-duration enforcement. |
| `execution/context_preflight.rs` | The complete recursive context visitor, capability preflight, addressed reads, direct writes and sends, and their structured errors. |
| `execution/activation_effects.rs` | Internal activation barriers and persistent-send payload capture. |
| `execution/module.rs` | Module graph preflight and retained or isolated module execution. |
| `execution/module_environment.rs` | Import/export environments, address targets, overlays, and conflict validation. |
| `execution/source_reconstruction.rs` | Reconstruction of scoped and fenced module source. |
| `execution/live_registration.rs` | Retained versus isolated live-registration mode and live-binding queries. |
| `execution/host_input.rs` | Host-input admission and atomic reactive-turn application. |
| `execution/persistent_send.rs` | Accepted-turn persistent-send selection and staging. |
| `execution/input_drivers.rs` | Ingress queries, draining, closure, driver startup, cleanup, and shutdown. |

`context_preflight.rs` intentionally exceeds the usual production-leaf size
target. Its recursive read-rewrite and capability-validation passes share one
placement model and must remain a complete visitor; splitting individual AST
branches would obscure coverage and duplicate traversal policy.

## Stable façade

Existing `MechRuntime` methods retain their names, visibility, feature gates,
arguments, return types, and errors. The activation compiler symbols used by
the runtime host remain narrowly re-exported through `execution`, and
`RuntimeAddressedAssignmentUnsupported` remains available at its prior module
path.

## Placement

Add a new execution method to the file that owns its policy above. Put
cross-subsystem orchestration in the narrowest existing owner and share only
the specific item required by a sibling. Do not add execution implementation,
universal imports, generic helpers, or another manager type to `mod.rs`.

Execution tests remain under `execution/tests/` with their existing
`runtime::execution::tests::...` paths. See the
[runtime test topology](../testing/runtime-test-topology.md) for scenario
placement and the
[core and interpreter test topology](../testing/core-interpreter-test-topology.md)
for lower-layer ownership.
