# Mixed CPU/GPU particles

This prototype runs two ordinary Mech programs through two execution domains:

- `app.mec` remains in the normal CPU runtime with its transaction, scheduler,
  timer host, resource grants, and console host.
- `kernel.mec` is compiled from its typed artifact into WGSL and remains
  resident in the configured `gpu` host.
- A committed CPU timer turn sends `turn` to `gpu://particles/kernel`. The GPU
  dispatch is an after-commit effect, so GPU failure is reported but cannot
  roll back the already committed CPU turn.
- Named scalar controls are ordinary GPU-host inputs. The CPU graph writes
  them before `turn`; the host uploads them into typed resident buffers. There
  are no raw GPU pointers or source-level buffer offsets.
- Only adapter and timing telemetry cross back. Particle matrices are not read
  back on each turn.

The same source and `.mcfg` run on both platforms. `wgpu` selects Metal on
macOS and Vulkan or Direct3D 12 on Windows. Run from the repository root:

```text
cargo run --release --features gpu_executor_native -- \
  run examples/mixed-cpu-gpu-particles
```

PowerShell:

```text
cargo run --release --features gpu_executor_native -- `
  run examples\mixed-cpu-gpu-particles
```

The ordinary Mech graph always runs on the CPU. The config declares both
`wgpu` and `cpu` executors for the compiled numeric kernel. `auto` selects
`wgpu`, which is the mixed CPU/GPU run. Select either executor without changing
the Mech programs or `.mcfg`:

```text
cargo run --release --features gpu_executor_native -- \
  run --backend wgpu examples/mixed-cpu-gpu-particles
```

```text
cargo run --release --features gpu_executor_native -- \
  run --backend cpu examples/mixed-cpu-gpu-particles
```

The CPU selection keeps the graph-to-host boundary intact and runs the exact
same admitted artifact in the fused CPU executor. It is a backend comparison,
not a different CPU-only version of the application.

Run the managed-runtime benchmark against both configured executors:

```text
cargo run -p mech-gpu --release --features runtime-host \
  --example mixed_runtime_benchmark -- all 4096 10 200
```

The arguments are backend (`all`, `wgpu`, or `cpu`), particle count, warmup
turns, and measured turns. The benchmark loads this exact `app.mec` and
specializes only the `particle-count` declaration in this exact `kernel.mec`.
Every measured pulse enters through the timer input driver, advances the
transactional CPU graph, commits its resource writes, and dispatches the
configured kernel host. GPU timing performs one final queue synchronization
after the measured batch and no particle readback. The run is rejected unless
the compiled element count and completed dispatch count match the request.

Stop with Ctrl-C. The first committed turn creates the adapter, compiles WGSL,
and uploads initial state. Later turns reuse the same device, pipeline, and
buffers. GPU telemetry returns through ordinary live resource reads, so the
console output is produced by the same CPU graph that owns the timer and the
dispatch command. Runtime dispatch is nonblocking: `dispatch-ms` measures CPU
submission latency, while GPU queue ordering preserves input-write and
ping-pong-state order without stalling the CPU runtime for completion.

This proves cross-platform CPU-to-GPU scheduling, but not arbitrary automatic
region fusion. The boundary is one configured Mech kernel program. Named
live-in scalar values work; device-buffer sharing with a render host and
transactional double-buffer publication remain separate runtime contracts.
The example currently generates its force controls in the CPU graph. A real
interactive application should replace those expressions with reads from a
window/pointer input host declared in the same `.mcfg`.
