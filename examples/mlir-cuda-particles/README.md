# Mech to MLIR to GPU: step 1

This example proves that the particle equations can be authored in Mech and
compiled into a real NVIDIA GPU kernel. There is no handwritten particle
kernel in the C support files.

The pipeline is:

```text
particles.mec
  -> Mech bytecode and typed resident numeric IR
  -> MLIR gpu.func, one thread per particle lane
  -> NVVM dialect
  -> PTX for the selected NVIDIA architecture
  -> CUDA Driver API launch
```

`particles.mec` owns six resident state vectors and all 31 arithmetic
operations in each particle turn. The support files only implement the GPU
runtime ABI, allocate resident memory, invoke the generated entry point, and
check the returned state.

## Generate PTX on macOS or Linux

Install LLVM 22 with MLIR, then run:

```bash
MLIR_BIN=/path/to/llvm-22/bin \
GPU_CHIP=sm_86 \
./examples/mlir-cuda-particles/build-ptx.sh
```

On an Apple Silicon Homebrew installation, `MLIR_BIN` is discovered at
`/opt/homebrew/opt/llvm/bin`. The command emits these inspectable artifacts:

```text
target/mech/mlir-cuda-particles/particles.gpu.mlir
target/mech/mlir-cuda-particles/particles.lowered.mlir
target/mech/mlir-cuda-particles/particles.host.ll
```

The first file contains `gpu.func @mech_turn`. The second embeds a PTX
`.visible .entry mech_turn`, and the third contains the host-side CUDA launch.

## Execute on an NVIDIA GPU from Windows

Use WSL 2 with NVIDIA GPU support, the CUDA toolkit headers, and LLVM 22 with
MLIR. From the repository inside WSL:

```bash
MLIR_BIN=/usr/lib/llvm-22/bin \
CUDA_HOME=/usr/local/cuda \
GPU_CHIP=sm_86 \
./examples/mlir-cuda-particles/run-wsl.sh
```

`sm_86` is the appropriate target for an RTX 3050 Ti. A successful run prints
the adapter, 65,536 particle lanes, 393,216 resident `f64` values, and a
maximum absolute error no greater than `1e-12`.

Set `MECH_OFFLINE=1` when all Cargo dependencies are already cached and the
build must not access the network.

## Current boundary

This is an AOT accelerator proof, not yet the managed Mech executor. The GPU
lowering intentionally rejects programs outside its proven subset. It accepts
resident `f64` row-vector state plus lane-wise broadcast, assignment, negate,
add, subtract, multiply, and divide. It rejects host inputs, reductions,
matrix operations, transcendental functions, nonuniform constants, and
non-lane-shaped state with an explanatory build error.

The target is explicit at build time:

```bash
mech build --aot --emit mlir --target nvidia:sm_86 \
  examples/mlir-cuda-particles
```

That avoids inventing a global `particles` host or hiding placement in a C or
Rust program. A later managed implementation should express accelerator policy
in project configuration while preserving the same backend-neutral numeric IR.

For inspection or a CPU comparison, the same KernelIR can also emit a fused
Rust `f32` loop:

```bash
mech build --aot --emit rust --target cpu:f32 \
  --out /tmp/particles-f32.rs examples/mlir-cuda-particles
```

## Execute on Apple Metal

The same `particles.mec` can be lowered through f32 SPIR-V-dialect MLIR and
executed on Apple Metal. Install LLVM 22 and SPIRV-Cross, then run:

```bash
brew install llvm spirv-cross
MECH_OFFLINE=1 TURNS=10000 ./examples/mlir-cuda-particles/build-metal.sh
```

The Apple path is:

```text
particles.mec -> Mech bytecode -> typed KernelIR -> SPIR-V MLIR
              -> SPIR-V binary -> generated MSL -> Metal
```

Apple GPUs do not expose f64 compute, so `apple:metal-f32` is an explicit
relaxed-precision target. The generated module contains both `mech_initialize`
and `mech_turn`; `metal_runner.m` owns the device, resident buffer, dispatch
loop, and post-timing correctness oracle, but it does not supply the executable
GPU equations or initial state. The small duplicated CPU recurrence is used only
to check the result after measurement. Metal source is compiled through the
runtime, so Xcode's optional offline Metal Toolchain component is not required.

## Representative resident benchmark

The benchmark compares the generated Metal `f32` kernel with generated Rust
`f32` and normal native Mech `f64` kernels. All three execute the same six-state
particle recurrence. Initialization, compilation, command encoding, and final
readback are outside the timed region.

```bash
MECH_OFFLINE=1 TURNS=10000 SAMPLES=5 \
  ./examples/mlir-cuda-particles/benchmark-metal.sh
```

On an Apple M1, the medians for 65,536 particles over 10,000 turns were:

| Backend | Precision | Time | Throughput | GPU speedup |
| --- | --- | ---: | ---: | ---: |
| Apple Metal | `f32` | 299.396 ms | 2,188.940 M particle-turns/s | 1.00x |
| Generated Rust | `f32` | 567.372 ms | 1,155.080 M particle-turns/s | 1.90x |
| Native Mech | `f64` | 1,286.048 ms | 509.6 M particle-turns/s | 4.30x |

The precision-matched GPU advantage is 1.90x. The larger 4.30x figure includes
both GPU execution and the explicitly relaxed `f32` precision. After 10,000
turns, the maximum sampled CPU `f32` versus GPU difference was about `5.2e-8`.

The current example uses 65,536 lanes because shaped scalar splats are still
serialized as dense constants in the program artifact. Larger resident arrays
need a compact splat representation rather than higher artifact size limits.
