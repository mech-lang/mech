# E5 — Canonical compiler planning extraction

This extraction moves existing frozen S8B implementation into a reviewable runtime
planning change. It does not implement replacement-gap fixes or switch shipping
source entry points. The input is frozen S8B
`662d29b79df8ab05a25bbadb941a689fd5bd5aae`; the base is E4 `7d0db6ede`.
The full identities, exact frozen byte spans, line references, byte lengths and
SHA-256 hashes are recorded in `e5-symbols.json`. These extraction records live
on the audit branch; the E5 production branch contains only the five copied
production/test file deltas.

## Included dependency closure

| File | Copied frozen implementation |
| --- | --- |
| `compiler.rs` | Canonical compilation error and retained-document parse helper; document and interactive-document APIs; document artifact APIs with supplied inputs and detached initializers; static-symbol evaluation APIs; canonical source convenience APIs. |
| `compiler.rs` | `CanonicalDocumentPlanning`; document projection and resource discovery; validation through temporary activation; resource-effect payload preflight; detached named-output execution; context binding and resource-send declarations. |
| `value.rs` | `program_result_output_index`, including the unchanged surrounding frozen file. |
| `loading.rs` | Frozen caller of the shared result-identity helper. |
| `query.rs` | Only `MechRuntime::program_output_id` and its frozen documentation. |
| `tests.rs` | 21 complete existing canonical planning tests and their retained-document helper. Exact names and hashes are in the JSON manifest. |

The tests cover planning constants versus live inputs, detached defaults, static
selection, resource preflight and missing providers, matrix shapes, function
scope/imports, inactive document owners, portable selectors, implicit result
identity, integrity constraints, and bytecode parity. They are copied acceptance
evidence, not a claim that the replacement audit's remaining obligations pass.

Every existing `compiler.rs` method and test remains byte-for-byte identical to
E4. The frozen import block is copied as one dependency unit. `loading.rs` and
`value.rs` equal their complete frozen files. The query projection-refresh call
retains E4's signature; its later engine/lifecycle dependency is excluded.

## Deferred closures

Graph/root/resolved-root/import APIs, their `canonical_handoff.rs` helpers and
ordered-root tests belong to the next graph extraction. REPL session lifecycle,
the three engine output-refresh hunks and the query refresh caller move together
in the interactive extraction. Mixed compilation and compute activation/IR
remain in their respective extraction closures. The 12 canonical tests requiring
those APIs are listed in `deferred_test_names`; none is partially rewritten.

`canonical_interactive_uses_configured_resource_planning` needs the new REPL
session API, and `canonical_uncalled_functions_do_not_bind_resource_inputs` also
exercises canonical-root compilation, so both remain deferred despite their
planning-related names. The new five-route query test also remains intact for
the graph extraction. Browser/bundle changes are excluded.

## Verification and suggested validation

The extraction provenance checker verifies all copied spans against frozen git
objects and reconstructs each changed file from declared spans and its base:

```sh
python3 /private/tmp/mech-syntax-s8-replacement-audit/docs/design/grammar-audit/s8-replacement-audit/extraction-manifests/verify-e5.py "$PWD"
git diff --check
rustup run nightly-2026-03-03 rustfmt --check --edition 2024 --config skip_children=true \
  src/runtime/src/runtime/program/{compiler,loading,query,value,tests}.rs
```

These checks passed before handoff. Cargo validation is intentionally delegated
to the parent agent's serialized target; no Cargo result is claimed here.
Suggested exact-head commands, matching the existing runtime feature profile:

```sh
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,resident-routing-source \
  --lib runtime::program::tests::canonical_
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,resident-routing-source --lib
cargo +nightly-2026-03-03 check --locked -p mech-runtime \
  --no-default-features --features full_compiler,compute
```

The broad library run checks unchanged routes alongside the added tests. The
compute-enabled build guards the intermediate extraction boundary without
adding the deferred mixed implementation. Any failing existing semantic witness
must remain an audit obligation rather than being repaired inside this extraction.
