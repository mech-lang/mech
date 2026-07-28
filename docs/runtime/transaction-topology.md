# Runtime transaction topology

Runtime transaction coordination has two intentionally separate layers:

- `src/runtime/src/transaction.rs` and `src/runtime/src/effect.rs` define
  public protocols and models.
- `src/runtime/src/runtime/transaction/` implements the private coordinator
  that joins the store, retained program, live runtime state, context,
  capabilities, modules, and effects into one atomic boundary.

The duplicate transaction and effect names at those layers are deliberate.
Private coordinator types must not leak into the public protocol layer merely
to make internal imports more convenient.

## Production ownership

`src/runtime/src/runtime/transaction/mod.rs` contains module declarations and
narrow internal re-exports only. Production behavior belongs to the following
owners:

| Module | Ownership |
| --- | --- |
| `envelope.rs` | Execution-transaction envelope, mode, state, program baseline, and active-envelope lookup. |
| `context.rs` | Transaction identity, context checkpoints, baseline restoration, and active transaction identity extraction. |
| `savepoint.rs` | Runtime-operation and retained-program-operation savepoints. |
| `health.rs` | Runtime health and poison records, including the existing public re-export. |
| `program.rs` | Atomic retained-program operation admission, savepoints, ownership, rollback, and poisoning coordination. |
| `reactive.rs` | Reactive-turn coordination and execution-service integration. |
| `effects.rs` | Private effect journal and effect lifecycle mechanics. Public effect traits remain in `src/runtime/src/effect.rs`. |
| `capabilities.rs` | Transaction-local capability overlay and usage journal. Public capability models remain outside this directory. |
| `modules.rs` | Transaction-local module and module-version journal. Module compilation and building remain in `runtime/module.rs`. |
| `commit.rs` | Store-commit construction, capability application, effect prepare/apply/commit, commit-event staging, durable publication, and indeterminate outcome classification. |
| `abort.rs` | Explicit abort, failed implicit cleanup, program/live and context restoration, effect abortion, staged-store discard, and abort-event emission. |

The coordinator preserves the existing commit ordering and poison policy.
File ownership is not permission to change effect protocols, capability
selection, snapshot APIs, store formats, event names, or transaction field
privacy.

## Test ownership

Private coordinator tests live below
`src/runtime/src/runtime/transaction/tests/` and are grouped by the invariant
owner:

```text
tests/
  store/
  program/
  reactive/
  effects/
  capabilities/
  modules/
```

`tests/mod.rs` only declares the store suite. Fixtures remain with the
subsystem suite that owns them.

The harness-path migration is complete under the following prefix map. Every
test function suffix is unchanged.

| Former harness prefix | Current harness prefix | Leaf modules |
| --- | --- | --- |
| `runtime::transaction::tests` | `runtime::transaction::tests::store` | `abort`, `begin`, `commit`, `context_identity`, `event_publication`, `indeterminate`, `store_failure` |
| `runtime::program_transaction::tests` | `runtime::transaction::program::tests` | `effects`, `explicit`, `extension_failures`, `implicit`, `integrity`, `poisoning`, `rollback`, `savepoints`, `support` |
| `runtime::reactive_transaction::tests` | `runtime::transaction::reactive::tests` | `coordination`, `finalization`, `rollback`, `service_borrow` |
| `runtime::effect::tests` | `runtime::transaction::effects::tests` | `after_commit`, `cleanup_failures`, `compensatable`, `savepoints`, `staging`, `support`, `transactional` |
| `runtime::capability::tests` | `runtime::transaction::capabilities::tests` | `grants`, `overlays`, `revocations`, `rollback`, `support`, `usage` |
| `runtime::module_transaction::tests` | `runtime::transaction::modules::tests` | `rollback`, `staging` |

For placement rules across private, public, compile-fail, and end-to-end
runtime coverage, see the
[runtime test topology](../testing/runtime-test-topology.md).
