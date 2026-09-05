# Mech compute backends

`mech-gpu` provides the current elementwise product lowerer and the physical
CPU and wgpu implementations of Mech's v0.4 compute subsystem. The compiler
owns source parsing and region partitioning. `mech-compute` owns the typed
neutral program and backend contracts. `mech-runtime` owns transaction
ordering and host activation.

One Mech document may contain one executable region marked `@compute`, `@cpu`,
or `@gpu`. `ProgramCompiler` returns partitioned coordinator and compute
`ProgramArtifact`s plus detached typed initializers. The product integration
then calls `mech_gpu::lower_elementwise_compute_program` to construct the
neutral `ComputeProgram` accepted by a configured `compute` host. The host
resolves a compatible backend and keeps state resident behind `ComputeSession`:

```text
Mixed Mech source
  -> ProgramCompiler mixed semantic compilation
  -> coordinator ProgramArtifact + compute ProgramArtifact + typed initializers
  -> product-owned elementwise lowering
  -> mech-compute ComputeProgram
  -> ComputeBackendRegistry
  -> ComputeSession
  -> transactional publication and telemetry
```

The stable mixed-application backends are `cpu-scalar` and `wgpu`. Stable
product selectors are `auto`, `cpu`, `gpu`, `cpu-scalar`, and `wgpu`. Native
`mech run` additionally accepts `cpu-simd` and `cpu-jit`: selecting either
requests the compiler-owned fixed-shape kernel form before backend resolution.
The browser product keeps the stable selector set because those native
executors are not available in WASM. The browser and native products use the
same registry and session interfaces with platform-specific adapters. An exact
backend request never falls back. `@cpu` and `@gpu` are hard class
requirements; `@compute` accepts any compatible backend.

The backend library also contains fixed-shape SIMD, Cranelift JIT, and wgpu
implementations. Native source compilation emits that form when a fixed-shape
backend is explicitly selected. The compiler rejects a program that cannot be
lowered to the requested form instead of silently running a different kernel.
`auto` and the browser product continue to use the portable elementwise form;
automatic dual-form planning is a separate optimization.

On macOS, the opt-in `metal-native` feature exposes a direct Metal resident
session for native-kernel measurements. It translates the same generated
scalar IR to Metal Shading Language and uses a Metal command encoder directly;
it does not create a WGPU device. The EKF benchmark selects that lane with
`MECH_METAL_ONLY=1`:

```text
cargo build -p mech-gpu --release --features native,jit,metal-native \
  --example parallel_ekf_benchmark
MECH_METAL_ONLY=1 ./target/release/examples/parallel_ekf_benchmark \
  500000 40 1 40
```

This is a macOS benchmark lane, not a replacement for the portable `gpu`
selector. WGPU remains the cross-platform path for Windows and macOS browser
or native applications.

The current product scope is one configured compute host, one executable
region, fixed-shape `f32` values, persistent state, declaration defaults, live
input updates, requested output readback, and integrity rejection before
publication. Multiple regions, dynamic shapes, cross-region values, multiple
devices, and cost-based placement are deliberately rejected or deferred.

Mixed applications are source products in v0.4. Compute-region metadata and
artifacts round-trip through bytecode v1, but one root `.mecb` cannot yet carry
and activate the coordinator artifact, compute artifact, typed interface,
detached initializers, and backend metadata as a compound application package.
Ordinary resident applications continue to support source and `.mecb` input.

## Layout and publication

Compute-interface tensors are canonical row-major values. Mech's fixed matrix
kernels use column-major storage, so fixed-shape backends convert at session
ingress and output readback rather than in the steady-state turn. Scalar values
remain distinct from `1 x 1` tensors, and every update is checked for kind,
element type, rank, dimensions, layout, and element count.

Fixed-shape integrity predicates are compiled from ordinary Mech operations.
A failed candidate is not published, the previous state remains live, and the
session retains only a fault count and latest named fault. The compute host
reports backend ID, completed turns, dispatch time, fault count, and last fault
through normal runtime ingress.

The browser wgpu bridge gives each dispatched command a monotonic identity.
Every command consumed by JavaScript is either acknowledged after
`GPUQueue.onSubmittedWorkDone` or rejected when upload, encoding, submission,
or completion fails. `completed turns` advances only for acknowledgements; a
rejection is returned to the next resident dispatch as an execution error. Adapter admission and
`requestDevice` use the same required storage-buffer and workgroup limits.

## Runtime configuration

The product host uses provider and scheme `compute`:

```text
compute://<instance>/kernel/input/<name>
compute://<instance>/kernel/turn
compute://<instance>/kernel/backend
compute://<instance>/kernel/turns
compute://<instance>/kernel/dispatch-ms
compute://<instance>/kernel/fault-count
compute://<instance>/kernel/last-fault
```

`mech run` and `mech serve` always use the ordinary resident runtime.
`--backend auto|cpu|gpu|cpu-scalar|wgpu` overrides configured stable product
selection. Native `mech run --backend cpu-simd|cpu-jit` selects the fixed-shape
compiler path and then runs through the same resident host, transaction, input,
fault, and publication machinery. Those two selectors are intentionally not
offered by browser `serve`.

## Validation

```text
cargo test -p mech-compute --all-features --locked
cargo test -p mech-gpu --features native,jit --locked
cargo test -p mech-wasm --features browser_project,browser_compute --locked
cargo check -p mech --features compute_backends_native --locked
```

The end-to-end particle application is under `examples/gpu-particles`.
Historical prototype notes and performance data live under
`docs/research/compute` and `benchmarks/archive/compute`; benchmarks are not
correctness gates.
