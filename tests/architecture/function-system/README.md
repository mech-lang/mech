# Function-system compatibility fixtures

These fixtures were captured from `v0.4-beta` at the PR0 base commit:

```text
f7768e0c6bbde69d27410be6d1ecacbd08c238c5
```

From the repository root, regenerate the deterministic JSON snapshots with:

```bash
cargo +nightly-2026-03-03 run \
  --manifest-path tests/fixtures/function-system-baseline/Cargo.toml \
  -- \
  --write tests/architecture/function-system
```

Create the pre-rewrite bytecode corpus only as an intentional compatibility
fixture update:

```bash
cargo +nightly-2026-03-03 run \
  --manifest-path tests/fixtures/function-system-baseline/Cargo.toml \
  -- \
  --write-bytecode tests/architecture/legacy-bytecode
```

Validate the committed JSON without modifying it:

```bash
cargo +nightly-2026-03-03 run \
  --manifest-path tests/fixtures/function-system-baseline/Cargo.toml \
  -- \
  --check tests/architecture/function-system
```

The additive runtime-factory surface uses the standard linked profile with
dynamic matrix shapes only. Generate and validate it separately so the
fixture's Matrix2/Vector2 specialization cases cannot enlarge the runtime
surface:

```bash
cargo +nightly-2026-03-03 run \
  --manifest-path tests/fixtures/function-system-baseline/Cargo.toml \
  --no-default-features \
  -- \
  --write-runtime tests/architecture/function-system

cargo +nightly-2026-03-03 run \
  --manifest-path tests/fixtures/function-system-baseline/Cargo.toml \
  --no-default-features \
  -- \
  --check-runtime tests/architecture/function-system
```

Both runtime commands require exact factory name, ID, ownership, and
function-pointer equality between every explicit catalog fragment and the
composed standard catalog.

Run the complete native compatibility contract, including all standalone
standard machines and the compiler-free bytecode consumer, with:

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

CI invokes only validation commands. It never invokes `--write` or
`--write-bytecode`.
