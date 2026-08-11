# GPU Particle Field

This served example keeps a high-throughput particle simulation resident on the
GPU. Mech commits one small control record through the `gpu-particles` host;
particle buffers never cross the runtime boundary during steady-state updates.

Build the browser package and serve the project:

```sh
bash scripts/build-mech-browser.sh
./target/release/mech serve examples/gpu-particles
```

The browser must support WebGPU. The page reports a clear unavailable state
when no compatible adapter can be created.

The compute pass reads committed particle buffer A and writes candidate buffer
B. The render pass consumes B in the same command submission, after which B
becomes the next committed generation. The two buffers are allocated once at
the configured `max-particles` capacity.

The current example provides generation-level publication inside the GPU
pipeline. It does not yet run a validation reduction that can reject a bad
candidate generation and report that rejection to the Mech runtime.

## Benchmark

Add `?benchmark=all` to the served URL to run the resident benchmark at 100K,
500K, 1M, and 2M particles. The page reports two measurements:

- Compute only encodes many simulation steps, submits them together, waits for
  the GPU queue to drain, and excludes command encoding from the timed region.
- Compute and render measures one complete resident frame at a time, including
  JavaScript orchestration, uniform upload, command encoding, compute,
  rasterization, submission, and a queue drain.

Use `?benchmark=compute` to skip rendering. Seeding, buffer allocation, pipeline
creation, and two render warm-up frames are outside the measured regions. The
per-frame queue drain is intentionally conservative; the interactive loop can
keep work in flight and is also limited by the display refresh rate.
