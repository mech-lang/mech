# Value-system architecture contracts

This directory freezes the exact reviewed Gate B value surface and its Gate C
migration destinations. It does not implement immutable values.

- `current-inventory.json` is the regeneratable v5 inventory of `Value`,
  `ValueKind`, semantic `Kind`, exact line/column variant occurrences, aliases,
  high-risk legacy mechanisms, proven auxiliary Rust fixtures, and the three
  distinct type-contract source classes.
- `legacy-growth-baseline.json` is generated only from archived reviewed B2
  commit `d5e41f6fd43c9d21c5858d80dab50e7ce64e9a10`. It is the immutable growth
  boundary; regenerating current inventory cannot expand it.
- `migration.json` is the reviewed v3 semantic classification. Every generated
  occurrence appears exactly once with roles and one applicable structured
  target. Its 69 targets name the exact enum variants to which they apply.
- `frozen-semantic-targets-v1.json` is the hand-maintained projection of every
  target's ID, applicability, semantic category, representation,
  implementation gate, key semantics, and runtime storage. It prevents the
  migration manifest from silently redefining an accepted destination. Its
  occurrence projection also freezes the reviewed target of every exact
  production site, so targets cannot be swapped between two valid uses.
- `canonical-encoding-v1.json` freezes `MechSnapshotEncodingV1`, SHA-256
  identities, exact schema, kind and dimension tags, framing,
  platform-independent primitive widths, aggregate order, and `KeyOrder`.
- `canonical-encoding-v1-vectors.json` commits eleven positive value, five key,
  seven dimension-normalization, seventeen invalid-value, and eight
  invalid-schema vectors.
  `scripts/tests/canonical_encoding_v1_reference.py` independently reproduces
  them during contract tests.
- `gate-b-regression.json` freezes inherited efficacy requirements and the
  protected paths that require fresh evidence.
- The adjacent `*-schema.json` files define each machine-readable contract.

Production code must qualify every `Value`, `ValueKind`, and semantic `Kind`
variant with its canonical enum name. Glob, grouped, single-variant,
variant-alias, enum-alias, direct type-alias, raw-identifier, qualified-self,
angle-qualified, and generic/turbofish spellings are rejected. `Self::Variant`
and `<Self>::Variant` are rejected inside the audited enum's own `impl` blocks.
This convention keeps inventory token-based and auditable without partial Rust
name resolution.

The v5 inventory contains 49 `Value` variants, 32 `ValueKind` variants, 17
semantic `Kind` variants, 5,976 qualified production occurrences, nine
`KindExpr` sources, five `KindScheme` sources, and ten runtime-representation
sources. Runtime representation and native-lowering metadata never become
`KindScheme`. `use_classifications` replaces file-wide role unions: several
sites may share one record only when enum, variant, path, roles, and target are
identical.

Cargo metadata defines production and auxiliary target roots. The inventory
enumerates 859 Rust files: 576 are audited, 261 are proven auxiliary through
Cargo target/module reachability, and 22 are proven trybuild fixture roots.
Production reachability overrides either auxiliary proof. Auxiliary files
are excluded only when they are target/module-graph reachable from an
auxiliary Cargo target or are literal `trybuild` fixture paths resolved from
supported `compile_fail` and `pass` calls in the target root or any reachable
helper module. Dynamic paths, unmatched files,
production-reachable files, and paths that escape the repository remain
audited or fail discovery. The current inventory records four trybuild call
sites resolving to 22 fixture files.

The immutable legacy scanner is version `c0-legacy-growth-v1`, implemented
entirely by `scripts/value_system_legacy_scanner_v1.py`, with module SHA-256
`9624eb89c01085cc5e412506b30671f442ba53b057c228b3fbcd113cc77ad834`.
The inventory separates two removable legacy value aliases from two required
public compatibility aliases (`SymbolTableRef` and `InterpreterRef`) and
freezes their public re-export routes.

The high-risk `Ref` inventory recognizes direct, turbofish, path-qualified,
angle-qualified, nested-generic, local-type-alias, and imported-rename UFCS
calls. Trait-qualified calls, unrelated receiver types, and instance methods
are not counted. Audited enum aliases are rejected through transitive alias
chains and generic identity wrappers, while ordinary containers such as
`Ref<Value>` remain valid.

Type-contract source records include exact declaration forms and field types.
Runtime-representation declarations may not reference semantic KindScheme
types. Canonical dimension constant folding uses checked `u64` arithmetic;
overflow is invalid.

`Value::Empty` is partitioned among `source-empty-expression`,
`option-absence`, `execution-no-result`, `uninitialized-storage`,
`unspecified-extent`, and `generic-dispatch`. `Value::MatrixValue` is
partitioned among `matrix-construction-ir`, `homogeneous-matrix-snapshot`, and
`legacy-matrix-value-adapter`. The nonempty bytecode-v1 rejection arm is frozen
separately as `heterogeneous-matrix-rejected`; it must not be counted as a
homogeneous immutable snapshot.

Permitted role identifiers are:

```text
semantic-payload
mutable-storage
type-wrapper
constant
machine-argument
machine-output
host-input
host-output
serialization
diagnostic
reactive-identity
journal-discovery
selection-ir
compiler-type-data
temporal-payload
compiler-shape-hole
generic-dispatch
reified-type
binding-contract
```

Regenerate and validate with:

```sh
python3 scripts/generate-value-system-inventory.py
python3 scripts/generate-value-system-inventory.py --check
python3 scripts/generate-value-system-inventory.py \
  --git-ref d5e41f6fd43c9d21c5858d80dab50e7ce64e9a10 \
  --check-legacy-baseline
python3 scripts/check-value-system-contract.py
python3 -B -m unittest scripts/tests/test_generate_value_system_inventory.py
python3 -B -m unittest scripts/tests/test_check_value_system_contract.py
```

Generated current inventory, immutable baseline evidence, and reviewed
migration classification are separate. The checker never rewrites them.
