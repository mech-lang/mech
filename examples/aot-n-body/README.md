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
