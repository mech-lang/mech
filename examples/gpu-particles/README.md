# Interactive selectable CPU/GPU particle field

This example is one Mech application. `particles.mec` contains both the normal
transactional CPU graph and the neutral `particle-field @compute` numeric
region. The same Mech source can select a resident CPU or GPU executor.

Mixed compute applications are source products in v0.4. This example is not a
claim that the coordinator and compute region can already be packaged and
activated from one root `.mecb`; compute-region artifact metadata does round
trip through bytecode independently.

The browser is a host, not the application:

- pointer events enter Mech through `pointer://pointer/frame`;
- the unannotated CPU graph computes the inputs for the accelerated region;
- committed writes to `compute://particles/kernel` trigger the selected executor;
- under `gpu`, positions and velocities remain resident in WebGPU buffers;
- under `cpu`, the same lowered artifact remains resident in WASM memory and
  positions are uploaded to WebGPU for rendering; and
- the page reports the backend that was actually selected.

There is no JavaScript particle simulation or handwritten particle kernel.
The compute program is lowered from the ordinary matrix expressions in
`particles.mec`. Hard `@cpu` and `@gpu` constraints conflict with the
opposite command-line backend instead of silently changing placement.

Select a backend without changing `particles.mec`:

```text
mech serve examples/gpu-particles --backend cpu
mech serve examples/gpu-particles --backend gpu
mech serve examples/gpu-particles --backend auto
```

`auto` is also the configured default. It selects GPU when a compatible adapter
is available and CPU otherwise. This particular visual example still requires
WebGPU for its renderer, so a browser without WebGPU cannot display either
compute mode; use `--backend cpu` to compare CPU compute on a WebGPU-capable
browser.

The backend flag changes compute placement only. Rendering uses WebGPU because
`index.html` explicitly declares a `points2d` WebGPU canvas for `result.0`. The
CPU backend therefore uploads each computed position matrix for display, while
the GPU backend renders the resident position buffer directly. A first-class
Mech `render://` host declaration is outside this spike.

The project configuration explicitly selects `../../src/wasm/pkg`. This keeps
the GPU-capable JavaScript/WASM pair together and prevents `mech serve` from
falling back to a differently profiled WASM module embedded in an older native
executable.

## macOS and Linux

Install `wasm-pack` once if it is not already available, then use the canonical
product build before starting the server:

```text
cargo install wasm-pack --locked
./scripts/build-mech.sh
./target/release/mech serve examples/gpu-particles --backend gpu
```

Open the printed URL in a WebGPU-capable browser. Press and drag in the field;
the pointer coordinates pass through a committed Mech runtime turn before the
compute force inputs change.

## Windows PowerShell

Use a current Edge or Chrome build with WebGPU enabled. The application and
Mech source are unchanged:

```text
cargo install wasm-pack --locked
powershell -ExecutionPolicy Bypass -File scripts\build-mech.ps1
.\target\release\mech.exe serve examples\gpu-particles --backend gpu
```

Open the printed local URL in Edge or Chrome. WebGPU availability, adapter
limits, WGSL compilation, and every CPU-to-compute binding are checked before
the simulation starts; failures are shown in the page instead of falling back.

The product scripts build the native CLI/runtime and the mixed CPU/WebGPU WASM
package only. For a component-specific browser rebuild, use
`python scripts/build-wasm.py --profile browser-compute` on every supported platform.

## Browser acceptance

Run the full end-to-end gate from the repository root. It builds the selected
browser profile and native server, starts the real project, launches Chrome or
Edge with WebGPU, waits for the one-million-particle runtime, advances compute
frames, sends pointer input, and verifies that input passes through a committed
Mech CPU turn into the selected executor:

```text
# macOS/Linux
python3 scripts/smoke-gpu-particles-browser.py --build --backend gpu
python3 scripts/smoke-gpu-particles-browser.py --backend cpu

# Windows PowerShell
py scripts\smoke-gpu-particles-browser.py --build --backend gpu
py scripts\smoke-gpu-particles-browser.py --backend cpu
```

The command exits unsuccessfully on a profile mismatch, startup exception,
missing WebGPU adapter, wrong particle count, stalled frames, or broken pointer
transaction. Failure artifacts are retained in the printed temporary folder.
This acceptance command builds and runs the application; it is separate from
the canonical product-only `build-mech` scripts above.

The command defaults to the unchanged shipped one-million-particle source.
`--software-adapter` selects the same dedicated Chromium WebGPU SwiftShader
test adapter used by CI. A bounded software-GPU run can be requested with
`--particle-count`; it copies the same project under `target`, changes only the
canonical particle-count declaration, and leaves the checked-in application
untouched. CI uses 16,384 lanes (256 workgroups) because GitHub's software
adapter accepts but does not finish its first one-million-lane dispatch within
the browser gate's two-minute deadline. The CI run retains the same two-turn
lifecycle, backpressure, input, zero-readback, error, and disposal assertions.

## Full-size acceptance

These tests compile the unchanged one-million-particle source. They do not
replace the particle count with a smaller fixture:

```text
cargo test -p mech-wasm --features browser_project,browser_compute served_million_particle_source_compiles_without_bytecode_serialization -- --ignored --nocapture
cargo test -p mech-gpu --release --features native --test particle_source served_particle_shader_matches_cpu_with_pointer_force -- --nocapture
```

The first test covers the compiler path used by the browser. The second runs
the generated shader on the system GPU, when an adapter is available, and
compares its complete output with the CPU backend.

## What is measured

`Particles` is the number updated by the generated compute program each committed
turn. `Displayed` is the renderer's visual sample, capped at 250,000 points to
keep rendering from obscuring compute throughput. The full one million particle
position and velocity matrices are always updated by the selected executor.

The particle count is the `particle-count` value in `particles.mec`. Startup is
reported as parsing, source-to-artifact compilation, and compute lowering. The
source-to-artifact phase includes eager source initialization: today it
materializes the million-element matrices while constructing the artifact.
GPU-side initializer lowering is the intended fix for that startup cost.

## Current spike boundary

This proves one ordinary CPU graph, one named portable compute region, explicit
host I/O, transaction-ordered dispatch, selectable persistent CPU/GPU state,
hard placement conflicts, and rendering through the cross-platform WebGPU
browser API. Stable product backends are `cpu-scalar` and `wgpu`. SIMD, JIT,
and fixed-shape wgpu remain backend-library experiments until the common
product compiler emits their kernel form. Mixed `.mecb` packaging, multiple
compute regions, general GPU-to-CPU graph edges, cost-based automatic
placement, and GPU-side initialization remain separate compiler and scheduler
work.
