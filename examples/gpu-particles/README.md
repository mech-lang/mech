# Standalone GPU particles

This directory is a complete Mech application. [`particles.mec`](particles.mec)
owns the initial particle matrices and the recurring integration equations.
[`mech.mcfg`](mech.mcfg) selects the GPU executor and the number of turns; no
Rust harness or host-provided values are required.

Build the executor and run the directory on macOS or Linux:

```text
cargo build --release --features gpu_executor_native
./target/release/mech run examples/gpu-particles
```

On Windows PowerShell:

```text
cargo build --release --features gpu_executor_native
.\target\release\mech.exe run examples\gpu-particles
```

The command compiles the `.mec` source into a typed program artifact, asks the
configured executor to place it, and then runs the resident state loop. The GPU
host infers the two device-resident state matrices and fuses the supported
matrix operations into WGSL. If the program contains unsupported operations,
the command rejects it with the source node and the reason it could not be
placed.

To run the exact same Mech source through the generated CPU executor, change
only this line in `mech.mcfg`:

```text
provider: "cpu"
```

The small nine-particle dataset keeps the standalone example easy to inspect.
[`particle-kernel.mec`](particle-kernel.mec) contains the equivalent scalable
kernel used by the benchmark harness, which explicitly supplies large input
matrices before compiling the same state recurrence:

```text
cargo run -p mech-gpu --release --features native \
  --example particle_benchmark -- 2000000 2 2 120
```

Both native GPU paths use `wgpu`, including Direct3D 12 and Vulkan on Windows.
