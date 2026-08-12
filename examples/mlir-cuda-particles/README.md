# Mech to MLIR to CUDA: step 1

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

`particles.mec` owns the initial state and every arithmetic operation. The C
files only implement the small MLIR CUDA runtime ABI, allocate/copy device
memory, invoke the generated entry point, and check the returned state.

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
the adapter, 1,024 particle lanes, 2,048 resident `f64` values, and a maximum
absolute error no greater than `1e-12`.

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
