# E6 — Retained graph compiler extraction

E6 extracts the remaining ordinary graph compiler closure from frozen S8B
`662d29b79df8ab05a25bbadb941a689fd5bd5aae` onto E5 `0c8a95794`.
It contains no new production behavior relative to frozen B. The manifest and
provenance checker live on the audit branch, outside the production extraction
stack. `e6-symbols.json` records full identities and exact frozen byte spans,
SHA-256 hashes, and reconstruction operations for every changed file.

| File | Exact frozen closure |
| --- | --- |
| `runtime/program/compiler.rs` | Public canonical root, resolved-root, ordered-root, interactive-root and options APIs; `CanonicalGraphCompilation`; graph document lowering, import ownership, detached dependency exports and module identity. |
| `resolver/canonical_handoff.rs` | Complete frozen file: generic resolved imports, shared import-value environment construction and import-use validation. |
| `runtime/program/query_tests/source.rs` | Complete frozen file: provider payload counters/preflight hook and the intact five-route resource-planning test. |
| `tests/canonical_declaration_handoff.rs` | Complete frozen file: existing declaration tests plus sealed imported constants acceptance. |
| `runtime/program/tests.rs` | Four complete added graph tests; the two existing explicit-root tests' frozen updates to exercise both compiler routes. |
| `resolver/file.rs` | Only the existing malformed-retained-source test replacement. The public filesystem path helper stays deferred. |
| `runtime/module/tests/source_document_transfer.rs` | Only its frozen fixture correction, represented by the complete frozen file: a valid ordinary source with an intentionally fuel-exhausted canonical document. |

The graph APIs depend on E5's ordinary canonical resource planning and E4's
ordered-document frontend/product metadata. The complete canonical handoff file
supplies `canonical_import_values`, `validate_canonical_import_uses` and generic
`CanonicalResolvedImport<T>`; the existing resolver module already exports them.
They have no dependency on the deferred compute activation implementation.

The four added program tests cover unused function resource admission, supplied
versus newly resolved revisions across all rooted API variants, retained resource
authority and reusable compiler error handling, and ordered-root bindings/cycle
and missing-export rejection. The two existing explicit-root tests retain their
frozen loops over ordinary and canonical entry points. The five-route query test
is copied without reducing its route loop or changing its payload oracle.

The malformed fixture corrections are copied from frozen B because E2 makes the
previous particle example valid. The file resolver fixture is deliberately
syntactically incomplete; the transfer fixture deliberately exhausts parse fuel.
These preserve their intended rejection contracts without production changes.

After this extraction, `compiler.rs` differs from frozen B only in the deferred
mixed compiler methods, `source_dependencies` field/constructor entries and
compute activation/capability helpers. REPL lifecycle and engine/query projection
refresh remain deferred together. `canonical_interactive_uses_configured_resource_planning`
therefore remains with that lifecycle extraction.

Verification performed before commit:

```sh
python3 /private/tmp/mech-syntax-s8-replacement-audit/docs/design/grammar-audit/s8-replacement-audit/extraction-manifests/verify-e6.py /private/tmp/mech-syntax-s8e6-graph
git diff --check
rustup run nightly-2026-03-03 rustfmt --check --edition 2024 --config skip_children=true \
  src/runtime/src/resolver/{canonical_handoff,file}.rs \
  src/runtime/src/runtime/module/tests/source_document_transfer.rs \
  src/runtime/src/runtime/program/compiler.rs \
  src/runtime/src/runtime/program/query_tests/source.rs \
  src/runtime/src/runtime/program/tests.rs \
  src/runtime/tests/canonical_declaration_handoff.rs
```

All three passed. No Cargo was run by the extraction agent; root owns serialized
build validation. Suggested checks at the intermediate head are:

```sh
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,resident-routing-source,compute \
  --lib runtime::program::tests::canonical_
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,resident-routing-source,compute \
  --lib explicit_
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,resident-routing-source,compute \
  --lib runtime::program::query_tests::source::
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,resident-routing-source,compute \
  --test canonical_declaration_handoff
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,resident-routing-source,compute --lib
```

The full library run includes both malformed-source regressions and unchanged
runtime routes. These commands are suggested validation, not recorded passes.
