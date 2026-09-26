# Function-system architecture contracts

The committed source cases and runtime-factory surfaces preserve operation
names, IDs, ownership, signatures, and distribution behavior after removal of
the retired universal value compatibility layer.

The former fixture generator depended on the removed value and function
adapters. It is intentionally gone. Permanent profile tests consume these
documents directly and fail when a canonical catalog or source boundary
changes unexpectedly.

The PR2 source and runtime JSON files retain their historical `base_commit`.
`src/stdlib/tests/profile_contracts.rs` checks `logic/all` as one explicit
addition: runtime and operation ID `00335e33bdc2f430`, one Boolean reduction
factory, and one module-only `logic/all` export. The tests then compare every
older entry against the original fixture or profile digest. Legacy profile
counts therefore increase by one runtime factory and, for source profiles,
one named specializer and module export; the prelude is unchanged.
`distribution_size_report_catalog_counts` still reports the actual complete
catalog count and digest, including this addition.

Run the complete native compatibility contract, including all standalone
standard machines and distribution boundaries, with:

```bash
bash scripts/check-function-system-contracts.sh
```

Run the shared cross-target corpus in headless Chrome with the contract-only
set operations enabled explicitly:

```bash
wasm-pack test \
  --headless \
  --chrome \
  src/wasm \
  --no-default-features \
  --features "browser_project,set_union,set_element_of" \
  -- \
  --nocapture
```

The shipped `browser_project` feature remains unchanged; the two set-operation
features above are part of this test configuration only.

CI invokes only validation commands. It never invokes `--write`.
