# Value-system architecture contracts

This directory preserves the reviewed value-system architecture while
`LegacyValue`, `ValueKind`, and semantic `Kind` are retired. It does not
implement immutable values or authorize new legacy behavior.

## Retirement contract

`current-inventory.json` is the frozen reviewed retirement ceiling. Its v6
inventory records the audited enums and variants, ordinary qualified variant
occurrences, aliases, high-risk mechanisms, source coverage, and the three
type-contract source groups. It is historical review evidence, not a file to
regenerate after each migration edit.

`migration.json` and `frozen-semantic-targets-v1.json` preserve the reviewed
historical semantic classification. The former assigns every reviewed variant
and occurrence to its migration family, role, and target. The latter freezes
target definitions and the reviewed occurrence-to-target projection so those
decisions cannot be rewritten during retirement.

The line and column values for ordinary occurrences are historical evidence.
Retirement validation compares the live and reviewed occurrence counts using
the key `(enum, variant, path)`; line and column are deliberately excluded.
Consequently:

- deleting a reviewed occurrence passes;
- moving an occurrence within the same file passes;
- adding an occurrence in the same file fails;
- moving an occurrence to another file fails because the destination grows;
- replacing one variant with another fails if the replacement's per-file count
  grows;
- adding a variant or changing a surviving variant definition fails;
- removing a reviewed variant passes.

This is a per-file shrink-only ceiling, not a repository-wide aggregate.
Equal global totals cannot relocate legacy debt. PR1 does not support deletion
of an entire audited enum or `src/core/src/value.rs`; final enum and module
removal belongs to the final-cutover PR.

High-risk mechanisms remain fingerprint-precise. `MutableReference`, `ValRef`,
`Ref` identity/address operations, transaction and journal mechanisms, legacy
aliases, and the C2 adapter allowance retain their exact scanner and boundary
checks. Canonical encoding, snapshot isolation, schema isolation,
semantic-module isolation, adapter coexistence, resident hot-path boundaries,
Gate A, Gate B, and frozen semantic targets remain enforced in retirement
mode.

## Contract documents

- `legacy-growth-baseline.json` is generated only from archived reviewed B2
  commit `d5e41f6fd43c9d21c5858d80dab50e7ce64e9a10`. The separate archived-source
  check proves it remains byte-identical to that source oracle.
- `canonical-encoding-v1.json` freezes `MechSnapshotEncodingV1`, SHA-256
  identities, exact schema, kind and dimension tags, framing, primitive widths,
  aggregate order, and `KeyOrder`.
- `canonical-encoding-v1-vectors.json` contains the independent positive and
  negative encoding vectors reproduced by
  `scripts/tests/canonical_encoding_v1_reference.py`.
- `gate-b-regression.json` freezes inherited efficacy requirements and the
  protected paths that require fresh evidence.
- The adjacent `*-schema.json` files define each machine-readable contract.

The immutable legacy scanner is version `c2-legacy-growth-v2`, implemented by
`scripts/value_system_legacy_scanner_v2.py`. Its frozen module SHA-256 is
`78529f2ffce2e3c3fc0d3ffabd55c8df1846ace2edd11500c095e82c8a12eed3`.
The scanner recognizes direct, qualified, aliased, raw-identifier, generic,
UFCS, and supported identity-wrapper forms without relying on partial Rust
name resolution.

Type-contract source records preserve the exact reviewed targets and
implementation gates. Exact mode additionally validates today's declaration
forms and field types, including the separation between semantic kind schemes
and runtime-representation metadata. Retirement mode uses the frozen
projection while continuing to scan live variants, ordinary occurrences,
aliases, and high-risk mechanisms.

## Operating modes

Normal migration PRs run retirement mode and do not edit or regenerate the
JSON contracts:

```sh
python3 scripts/check-value-system-contract.py --mode retirement
```

Exact historical validation remains available for the reviewed tree:

```sh
python3 scripts/check-value-system-contract.py --mode exact
```

`generate-value-system-inventory.py --check` is an exact-tree diagnostic. It
remains strict and byte-precise, but it is not part of normal retirement CI and
must not be used to bless migration edits by regenerating
`current-inventory.json`.

Verify the archived legacy baseline separately:

```sh
python3 scripts/generate-value-system-inventory.py \
  --git-ref d5e41f6fd43c9d21c5858d80dab50e7ce64e9a10 \
  --check-legacy-baseline
```

Run the focused Python suites with:

```sh
python3 -B -m unittest \
  scripts/tests/test_generate_value_system_inventory.py \
  scripts/tests/test_check_value_system_contract.py
```

The final deletion will replace this retirement ceiling with a rule requiring
zero production `LegacyValue` occurrences. Until then, ordinary occurrences
may only shrink and every permanent safety boundary remains active.
