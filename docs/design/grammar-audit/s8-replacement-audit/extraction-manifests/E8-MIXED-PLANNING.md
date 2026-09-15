# E8 — Mixed compiler extraction

E8 copies the remaining mixed compiler closure from frozen S8B
`662d29b79df8ab05a25bbadb941a689fd5bd5aae` onto E7 `634f45620`.
All three changed files now exactly equal their frozen versions:

| File | Remaining frozen delta |
| --- | --- |
| `src/runtime/src/runtime/program/compiler.rs` | Canonical mixed source/document/root/resolved-root methods; mixed partition planning; detached initializer and activation-input capture; declared compute input/output names; `source_dependencies` and its entries in both existing tree constructors. |
| `src/compute/src/port.rs` | Decode canonical artifact source input names before constructing public compute ports. |
| `src/runtime/src/runtime/program/tests.rs` | Seven complete mixed compiler tests and restoration of frozen declaration order after earlier extraction stages appended their tests. |

The seven tests cover ordinary/compute partition ownership and typed initializers,
shared transitive imports/dependency identities, batched activation values, inlined
local function graphs, tuple-destructured function outputs, shipped EKF compilation,
and shipped particle initialization. Their exact names are in `e8-symbols.json`.
All 134 existing top-level test/helper function bodies are unchanged; the larger
textual test diff reflects restoring their original order, not additional behavior.

The dependency closure uses E4's mixed canonical frontend, E5's resource planning,
named static output evaluation and preflight, E6's graph/import ownership, and the
existing compute interface assembly. `decode_source_input_name` is already present
in E4. Both existing mixed tree constructors receive the frozen new field entry
with the type addition. There is no dependency on the deferred compute activation
evaluator, compute IR/publications changes, or host/browser implementation.

`e8-symbols.json` records base/frozen identities and hashes for each exact file.
The provenance checker also verifies the unchanged test bodies and exact seven-test
addition. These audit files are not part of the production branch.

```sh
python3 /private/tmp/mech-syntax-s8-replacement-audit/docs/design/grammar-audit/s8-replacement-audit/extraction-manifests/verify-e8.py /private/tmp/mech-syntax-s8e8-mixed
git diff --check
rustup run nightly-2026-03-03 rustfmt --check --edition 2024 --config skip_children=true \
  src/runtime/src/runtime/program/{compiler,tests}.rs src/compute/src/port.rs
```

These checks passed before handoff. No Cargo was run by the extraction agent.
Suggested validation for the parent's serialized target:

```sh
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,resident-routing-source,compute \
  --lib canonical_mixed_
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,resident-routing-source,compute \
  --lib canonical_rooted_mixed_compilation_shares_transitive_imports_and_initializers
cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features --features full_compiler,resident-routing-source,compute --lib
cargo +nightly-2026-03-03 check --locked -p mech-runtime \
  --no-default-features --features full_compiler,resident-routing-source
```

These are suggested checks, not claimed results. A frozen semantic failure remains
an audit obligation and does not authorize a repair inside this extraction.
