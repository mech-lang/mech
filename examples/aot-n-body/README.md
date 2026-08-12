# AOT n-body prototype

Build the matrix/indexed Mech simulation into a native executable:

```sh
cargo run --release -- build --aot examples/aot-n-body \
  --workspace-root . --offline --out target/mech/aot-n-body
target/mech/aot-n-body --turns 1000000 --guarantees all
```

This is the Benchmarks Game five-body system. The ten-pair table is fixed
program data, and two incidence matrices sum pairwise velocity changes into
body rows. That is algebraically equivalent to indexed `+=`/`-=` updates but
avoids the current bytecode contract validator bug for differently shaped
indexed inputs and destinations.

Guarantee modes deliberately select different native lowering contracts:

- `fast` mutates one state buffer and permits fixed-shape constant folding and
  algebraic power specialization.
- `atomic` preserves the explicit operation graph and publishes a candidate
  state buffer only after the complete turn.
- `checked` adds finite-value validation before publication.
- `receipt` also hashes the before/after state into a chained turn receipt.

Fast math is intended for finite, validated workloads. Use `atomic` or stronger
when strict operation semantics and rollback behavior are required.
