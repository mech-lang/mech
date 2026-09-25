# Rust-hosted Metal control

This is the Metal comparison's Rust ecosystem control. A Rust host compiles
and dispatches a hand-written MSL EKF kernel through `metal-rs`. Stable Rust
does not itself compile Rust kernels to Apple Metal, so the result must be
labeled **Rust host + MSL**, not “Rust compiled to Metal.”

The kernel uses the same broad physical strategy as the direct Mech Metal
backend: resident structure-of-arrays state, one GPU thread per filter,
separate checked and unchecked entry points, two publication buffers, a
two-word shared fault status, 64-thread Metal threadgroups, one command buffer
and host wait per turn, and five untimed warmup turns followed by a state
reset.

```sh
cargo run --manifest-path benchmarks/iros-2026/rust-metal/Cargo.toml \
  --release -- 500000 40 checked
cargo run --manifest-path benchmarks/iros-2026/rust-metal/Cargo.toml \
  --release -- 500000 40 unchecked
```
