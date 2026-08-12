# AOT EKF prototype

Build the high-level Mech EKF into a native executable:

```sh
cargo run --release -- build --aot examples/aot-ekf \
  --workspace-root . --offline --out target/mech/aot-ekf
target/mech/aot-ekf --turns 1000000 --guarantees all
```

The executable reports CSV for four envelopes around exactly the same compiled
turn. `receipt` adds validation and chained receipt hashing. It is not the
complete `MechRuntime` transaction stack.
