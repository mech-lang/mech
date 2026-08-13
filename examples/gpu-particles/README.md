# Interactive mixed CPU/GPU particle field

This example is one Mech application. `particles.mec` contains both the normal
transactional CPU graph and the named `particle-field @ gpu` numeric region.

The browser is a host, not the application:

- pointer events enter Mech through `pointer://pointer/frame`;
- the unannotated CPU graph computes the inputs for the accelerated region;
- committed writes to `gpu://particles/kernel` trigger the GPU dispatch;
- positions and velocities remain resident in WebGPU buffers; and
- the browser renders the resident position buffer without reading it back.

There is no JavaScript particle simulation or handwritten particle kernel.
The GPU program is lowered from the ordinary matrix expressions in
`particles.mec`. Unsupported regions fail with compiler diagnostics instead of
silently moving to the CPU.

## macOS

Install `wasm-pack` once if it is not already available, then build the browser
runtime before building the server so the executable embeds the new WASM files:

```text
cargo install wasm-pack --locked
./scripts/build-mech-gpu-browser.sh
cargo build --release
./target/release/mech serve examples/gpu-particles
```

Open the printed URL in a WebGPU-capable browser. Press and drag in the field;
the pointer coordinates pass through a committed Mech runtime turn before the
GPU force inputs change.

## Windows PowerShell

Use a current Edge or Chrome build with WebGPU enabled. The application and
Mech source are unchanged:

```text
cargo install wasm-pack --locked
powershell -ExecutionPolicy Bypass -File scripts\build-mech-gpu-browser.ps1
cargo build --release
.\target\release\mech.exe serve examples\gpu-particles
```

Open the printed local URL in Edge or Chrome. WebGPU availability, adapter
limits, WGSL compilation, and every CPU-to-GPU binding are checked before the
simulation starts; failures are shown in the page instead of falling back.

## Full-size acceptance

These tests compile the unchanged one-million-particle source. They do not
replace the particle count with a smaller fixture:

```text
cargo test -p mech-wasm --features browser_project,browser_gpu_compiler served_million_particle_source_compiles_without_bytecode_serialization -- --ignored --nocapture
cargo test -p mech-gpu --release --features native --test particle_source served_particle_shader_matches_cpu_with_pointer_force -- --nocapture
```

The first test covers the compiler path used by the browser. The second runs
the generated shader on the system GPU, when an adapter is available, and
compares its complete output with the CPU backend.

## What is measured

`Particles` is the number updated by the generated GPU program each committed
turn. `Displayed` is the renderer's visual sample, capped at 250,000 points to
keep rendering from obscuring compute throughput. The full one million particle
position and velocity matrices are always updated on the GPU.

The particle count is the `particle-count` value in `particles.mec`. Startup is
reported as parsing, source initialization, artifact compilation, and GPU
lowering. The current eager source initializer materializes the million-element
matrices while constructing the artifact; GPU-side initializer lowering is the
intended fix for that startup cost.

## Current spike boundary

This proves one ordinary CPU graph, one named GPU region, explicit host I/O,
transaction-ordered dispatch, persistent GPU state, and direct rendering
through the cross-platform WebGPU browser API. The generated shader is validated
on macOS Metal; the Windows build and run path is provided but still needs a
physical Windows browser acceptance pass. Multiple GPU regions, GPU-to-CPU
readback, automatic placement, and GPU-side initialization remain separate
compiler and scheduler work.
