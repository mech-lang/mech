# MLIR GPU prototype findings

The first GPU AOT prototype lowers the same `KernelIr` used by the native Rust
and CPU MLIR backends. It proves this path without adding CUDA concepts to Mech
source operations:

```text
Mech source -> bytecode -> activated numeric KernelIr -> MLIR GPU -> NVVM -> PTX
```

## What the current representation already preserves

- Typed numeric operations survive compilation and resident activation.
- State storage, offsets, element types, and matrix shapes are recoverable.
- A row-vector recurrence can be recognized as independent materialized lanes.
- Assignment and broadcast semantics can be lowered without a handwritten
  domain kernel.

These are enough to generate a real PTX entry point from an ordinary `.mec`
file. CUDA architecture selection is a build target, not an operation encoded
inside the portable bytecode.

## Design pressure exposed by the prototype

1. **Lane intent must become explicit.** Inferring parallel lanes from a row
   vector is sufficient for the proof, but too accidental for a general GPU
   contract. The compiler IR or build plan needs a typed batch/lane dimension.
2. **`f32` must reach resident values and `KernelIr`.** The current numeric IR
   has `f64` and `Index`. Consumer NVIDIA GPUs strongly favor `f32`; a serious
   particle backend cannot rely only on `f64`.
3. **Initializers need a splat representation.** The source currently contains
   a 1,024-element literal because a typed state broadcast initializer does not
   survive the current planning path. Bytecode/artifact state should preserve a
   compact fill operation instead of materializing repeated elements.
4. **Residency is an executor concern.** A managed path needs device buffers,
   dirty ranges, transfer decisions, and asynchronous completion dependencies.
   Copying the entire state around every turn would erase the GPU benefit.
5. **Guarantees need an explicit boundary.** The prototype mutates resident GPU
   memory in place and offers no rollback. A transactional executor can use a
   second device buffer or defer publication until successful completion; a
   performance mode can choose in-place failure semantics.
6. **Capability rejection is part of the backend contract.** Unsupported
   operations must report the responsible node and operation. Silent CPU
   fallback would make placement and performance unpredictable.
7. **Toolchain versions must be pinned.** The prototype is verified with LLVM
   22.1.8. MLIR's GPU lowering pipeline and runtime ABI are not a stable binary
   interface, so generated build projects must record their LLVM version.

## Next vertical slice

Keep buffers resident across multiple turns, add per-lane inputs and `f32`, and
make accelerator selection available in `mech.mcfg`. Then place the compiled
region behind the executor's normal dependency boundary with two guarantee
modes: staged transactional publication and unchecked in-place mutation.
