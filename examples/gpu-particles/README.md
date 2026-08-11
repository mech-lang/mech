# GPU particles

This is a complete Mech application. [`particles.mec`](particles.mec) defines
the two-million-particle initial state, simulation constants, and recurring
integration equations. [`mech.mcfg`](mech.mcfg) selects the GPU executor.

The browser boundary matches the analog clock and n-body examples:

- `particles.mec` is the application.
- `mech.mcfg` selects the executor and served files.
- `index.html` provides a canvas target for the generic `points2d` renderer.
- `/_mech/project.js` is the shared browser and WebGPU shim.

There is no particle-specific JavaScript and no Rust harness supplying particle
matrices or physics constants. The GPU compiler evaluates the source-defined
initialization, compiles the typed recurring graph, and rejects unsupported
programs with admission diagnostics.

## Browser

Build the browser GPU profile and server, then serve the example:

```text
./scripts/build-mech-gpu-browser.sh
cargo build --release --features gpu_executor_native
./target/release/mech serve examples/gpu-particles
```

On Windows PowerShell:

```text
.\scripts\build-mech-gpu-browser.ps1
cargo build --release --features gpu_executor_native
.\target\release\mech.exe serve examples\gpu-particles
```

Open `http://127.0.0.1:8081`.

The simulation advances every configured particle. To keep canvas rendering
from dominating the frame, the generic point renderer samples at most 250,000
particles for display. The page reports both counts and continues to calculate
throughput from the complete simulation state.

The compile readout separates source parsing, eager source initialization, and
artifact compilation. The current compiler still materializes the
source-defined initial matrices on the CPU before it lowers the recurring graph
to WebGPU. Successful plan registration does not render function debug output,
so large matrices are not formatted merely to prepare diagnostics. At this
scale, semantic artifact snapshotting and hashing are now the dominant startup
cost. GPU-side initializer lowering remains future executor work and would
avoid both the CPU materialization and the large initializer constants.

## Native executor

The same source can run through the native resident executor:

```text
./target/release/mech run examples/gpu-particles
```

Change only `run.executor.provider` from `"gpu"` to `"cpu"` to run the
generated resident CPU executor instead. The source program is unchanged.
