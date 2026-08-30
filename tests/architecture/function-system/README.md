# Function-system architecture contracts

The committed source cases and runtime-factory surfaces preserve operation
names, IDs, ownership, signatures, and distribution behavior after removal of
the retired universal value compatibility layer.

The former fixture generator depended on the removed value and function
adapters. It is intentionally gone. Permanent profile tests consume these
documents directly and fail when a canonical catalog or source boundary
changes unexpectedly.

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
