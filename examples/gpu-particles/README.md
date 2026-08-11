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

## Browser particle field

The served example compiles [`particle-kernel.mec`](particle-kernel.mec) in the
browser, passes its typed artifact to the GPU host, and uses the generated WGSL
and binding manifest as the WebGPU compute pipeline. JavaScript owns browser
device setup, buffers, dispatch, rendering, and timing; it does not contain a
handwritten compute shader.

Build the browser compiler package and server, then serve the directory:

```text
./scripts/build-mech-gpu-browser.sh
cargo build --release --features gpu_executor_native
./target/release/mech serve examples/gpu-particles
```

On Windows PowerShell, use the equivalent commands:

```text
.\scripts\build-mech-gpu-browser.ps1
cargo build --release --features gpu_executor_native
.\target\release\mech.exe serve examples\gpu-particles
```

Open `http://127.0.0.1:8081`. The initial compile establishes capacity for two
million particles. The 100K, 500K, 1M, and 2M controls change the active GPU
dispatch without rebuilding the program. Use the Benchmark button for the full
matrix, or open these URLs to start it automatically:

```text
http://127.0.0.1:8081/?benchmark=compute
http://127.0.0.1:8081/?benchmark=all
```

The benchmark reports resident compute separately from compute plus rendering.
Shader compilation, state allocation, initial upload, and readback are outside
the resident per-turn measurement.
