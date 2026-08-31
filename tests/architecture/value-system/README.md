# Value-system architecture contracts

The final deletion-to-replacement disposition for mixed and compatibility-only
test suites is retained as archived review evidence in the
[value-system final-cutover test map](../../../docs/design/archive/value-system-final-cutover-test-map.md).

## Canonical runtime boundary

`Value` is immutable canonical data. Its validated `ValueData`, schema key,
schema table, and resolved shape describe semantic content without carrying
process-local identity. `ValueCell` is the mutable identity of a program
location. Its private `CellBinding` retains an exact scalar, matrix, or
canonical-value backing without exposing erased storage.

Canonical aggregate data is acyclic. Aggregate elements are owned
`ValueData`; they never contain `ValueCell`, `Ref<T>`, object identifiers, or
back-references. Computation graphs may still be cyclic because graph edges
connect `ValueCell` locations rather than being embedded in immutable data.

The retired universal value model and adapter directory have been removed.
Normal solve, source specialization, bytecode, residents, hosts, resources,
and public APIs all operate on canonical values and cells.

## Permanent contracts

- `canonical-encoding-v1.json` freezes `MechSnapshotEncodingV1`, SHA-256
  identities, exact schema, kind and dimension tags, framing, primitive
  widths, aggregate order, and `KeyOrder`.
- `canonical-encoding-v1-vectors.json` contains independent positive and
  negative encoding vectors reproduced by
  `scripts/tests/canonical_encoding_v1_reference.py`.
- `gate-b-regression.json` freezes inherited efficacy requirements and the
  protected paths that require fresh evidence.
- The adjacent `*-schema.json` files define each machine-readable contract.

`scripts/check-value-system-contract.py` validates those documents, reproduces
the canonical vectors, preserves schema isolation and validated construction
routes, and checks Gate B evidence:

```sh
python3 scripts/check-value-system-contract.py
```

`scripts/check-no-retired-value-system.py` permanently rejects the removed
files, crate-root modules, exact retired symbols, and conversion entry points:

```sh
python3 scripts/check-no-retired-value-system.py
```

The migration inventories, growth baseline, C2 allowance, and their generators
were intentionally removed after the final zero result. Git history retains
that review evidence; the working tree now carries only permanent canonical
contracts.

## Downstream migration

- Replace the removed universal value enum with `Value` for immutable data or
  typed canonical function ports for execution.
- Replace the removed value-kind and semantic-kind lookup models with
  `Schema`, `KindExpr`, `KindScheme`, and exact runtime representations.
- Replace universal mutable references with `ValueCell` or an exact typed
  backing retained behind a canonical port.
- No deprecated alias or compatibility shim is provided.
