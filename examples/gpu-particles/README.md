# Mixed CPU/GPU particle application

This spike runs one real Mech document through the ordinary runtime and a
native GPU host. [`particles.mec`](particles.mec) contains both sides:

- unannotated application code reads the timer host, stages GPU work, reads GPU
  telemetry, and writes the console host;
- `particle-field @ gpu` is compiled from the same syntax tree into one resident
  `wgpu` kernel;
- the runtime delivers the GPU dispatch only after the CPU transaction commits.

There is no Rust particle function, JavaScript simulation, second `.mec` file,
or configured direct-executor turn loop. The Rust GPU host implements the
general execution boundary: it receives a compiler-produced region and rejects
regions the current lowering cannot execute.

## Run

Build native GPU support and run a bounded number of live timer turns:

```text
cargo build --release --features gpu_executor_native
./target/release/mech run examples/gpu-particles --max-live-turns 120 --runtime-info
```

On Windows PowerShell:

```text
cargo build --release --features gpu_executor_native
.\target\release\mech.exe run examples\gpu-particles --max-live-turns 120 --runtime-info
```

`wgpu` selects Metal on macOS and an available native backend on Windows. Each
console row is `(timer tick, completed GPU turns, previous dispatch ms, adapter)`.
The telemetry is intentionally one transaction behind: the application reads
the last committed state before staging the next after-commit dispatch.

The spike explicitly uses the transactional legacy route for the CPU graph.
D4's resident finalizer still sees operations inside the GPU section when it
tries to build the whole CPU artifact; partition-aware resident finalization is
one of the design results this prototype makes concrete.

The particle count is the `particle-count` value in `particles.mec`; the checked
in end-to-end demonstration runs 100,000 particles. Larger source-defined fields
currently push the eagerly materialized source artifact toward D4's 64 MiB
bytecode read limit. GPU-side initializer lowering should remove that startup
artifact growth instead of teaching the demo to bypass the limit.

## Spike limits

The extraction and host boundary are real, but deliberately narrow. The native
path currently supports one selected GPU region, autonomous resident state, and
dispatch/telemetry I/O. Mutable CPU-to-GPU region parameters, GPU output
readback into the CPU graph, multiple GPU regions, and the browser host remain
future scheduling work and fail rather than silently changing placement.
