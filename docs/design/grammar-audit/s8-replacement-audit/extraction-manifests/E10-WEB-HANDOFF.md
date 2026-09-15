# E10 — Web, bundle and production adapter extraction

E10 copies nineteen complete files from frozen S8B
`662d29b79df8ab05a25bbadb941a689fd5bd5aae` onto E9 `199b42d5d`.
File identities, modes and hashes are recorded in `e10-symbols.json`.
This manifest and the provenance checker stay on the audit branch.

| Closure | Files |
| --- | --- |
| Bundle admission | Runtime `program/bundle.rs` and `program/mod.rs` declaration/re-export; WASM `project.rs`; `include/static-project.js` and its Node tests. |
| Bundle source/presentation | Top-level `browser_planning.rs`, `bundle_planning.rs`, `bundle_presentation.rs`, `bundle_web.rs`; runtime renderer and renderer test; resolver public path-candidate factoring. |
| Feature/module wiring | Top-level `Cargo.toml` and `src/lib.rs`. |
| Mixed production adapters | `src/cli/compute.rs`, `src/wasm/src/mixed_compute.rs`. |
| Qualification owned by these adapters | `src/build/tests/standard_host_source_planning.rs`, `scripts/smoke-bundle-web.sh`, `scripts/smoke-canonical-compute-browser.py`. |

The compile and caller dependencies move together:

- `bundle_web_core` enables the compiler, resident source route, syntax program
  types and standard catalog needed by the retained bundle producer.
- `CanonicalProgramBundle` is declared/re-exported under the same
  `resident-routing-source + serde` gate used by its callers. The served WASM
  profile already enables both through `browser_project_core`.
- WASM `fromServedBundle` now receives source and artifact maps separately;
  its four-argument JavaScript caller and bootstrap test arrive in this change.
- `bundle_planning::retained_sources` calls the public filesystem candidate helper,
  so the complete remaining resolver factoring lands with that caller.
- Shared configured browser planning uses the existing host provider manifests
  and effect-free planning snapshots. The root module gate covers both bundle and
  served products.
- Mixed CLI/WASM production callers use the E8 retained mixed compiler and E9
  compute backends. No later CI or documentation API is needed to compile them.

Owned new Rust tests cover provider planning, resource aliases, bundle dependency
values/identities/resolution/errors, configured clocks/browser assignments, prose,
root cardinality, stale bundle rejection, and document framing. The Node suite
has two bootstrap tests. All existing frozen tests and their changes are copied
without adjusting expectations to fit an intermediate head.

## Remaining E11 boundary

After E10, exactly seven files must differ from frozen B:

1. `.github/workflows/ci-full.yml`
2. `.github/workflows/ci.yml`
3. `docs/design/grammar-audit/s8-consumer-readiness.tsv`
4. `docs/design/grammar-audit/s8-removal-manifest.tsv`
5. `docs/design/syntax-s8-execution-rehearsal.mec`
6. `src/syntax/tests/source_parser_consumer_inventory.rs`
7. `src/syntax/tests/support/cutover_contract.rs`

Those changes reconcile routing with the frozen consumer census while keeping
deletion qualification separate, and attach final CI/documentation. The old
consumer-census enforcement on this intermediate head does not yet account for
the newly routed callers; its coordinated update belongs to E11, not an invented
exception in this production extraction. E11 can then reproduce the full frozen
tree with no exclusions.

## Verification and suggested validation

Exact frozen reconstruction, whitespace and Rust formatting passed. TOML parsing,
`bash -n scripts/smoke-bundle-web.sh`, and Python AST parsing of the new smoke
driver passed. No Cargo or browser test ran in this extraction task. Node remains
unavailable locally; JavaScript execution is not claimed as passed.

```sh
python3 /private/tmp/mech-syntax-s8-replacement-audit/docs/design/grammar-audit/s8-replacement-audit/extraction-manifests/verify-e10.py /private/tmp/mech-syntax-s8e10-web
git diff --check
```

Concrete package/feature checks for parent-owned serialized validation:

```sh
cargo +nightly-2026-03-03 test --locked -p mech-runtime --no-default-features \
  --features full_compiler,full_source,resident-routing-source \
  --lib runtime::program::bundle::tests::
cargo +nightly-2026-03-03 test --locked -p mech-runtime --no-default-features \
  --features full_source,resident-routing-source --test canonical_document_render
cargo +nightly-2026-03-03 test --locked -p mech --no-default-features \
  --features serve,bundle_web,formatter --lib bundle_web::
cargo +nightly-2026-03-03 test --locked -p mech-build --features full-hosts \
  --test standard_host_source_planning every_standard_provider_plans_canonical_source_to_bytecode -- --exact
cargo +nightly-2026-03-03 test --locked -p mech-build --features full-hosts \
  --test standard_host_source_planning canonical_context_alias_preserves_the_resolved_resource_owner -- --exact
cargo +nightly-2026-03-03 check --locked -p mech --no-default-features \
  --features run,compute_backends_native --bin mech
cargo +nightly-2026-03-03 check --locked -p mech-wasm --target wasm32-unknown-unknown \
  --no-default-features --features browser_compute_canary
node --test scripts/tests/static-project.test.mjs
node scripts/test-browser-compute-lifecycle.mjs
```

The renderer integration test explicitly needs `full_source`; the provider suite
explicitly needs `full-hosts`. The runtime bundle test also requires
`compiler_default`, supplied by `full_compiler`. Omitting these gates can produce
zero selected tests.

The existing supported browser build profiles and CI commands provide actual
adapter validation after compilation:

```sh
python3 scripts/build-wasm.py --profile browser-compute-canary
python3 scripts/smoke-canonical-compute-browser.py --software-adapter
python3 scripts/build-wasm.py --profile browser
cargo +nightly-2026-03-03 build --locked -p mech --bin mech \
  --no-default-features --features serve,bundle_web,formatter
MECH_BIN=target/debug/mech bash scripts/smoke-bundle-web.sh
```

The new canonical browser smoke requires both CPU and actual WebGPU publication
results for two turns; unlike optional-adapter Rust tests it fails if no adapter
is available. These commands are proposed checks, not recorded results.
