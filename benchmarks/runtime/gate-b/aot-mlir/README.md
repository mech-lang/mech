# Mech AOT Rust versus MLIR prototype

This prototype lowers the same activated, fixed-shape Mech `KernelIr` through
two backends:

```text
examples/aot-n-body/n-body.mec
  -> Mech bytecode and resident activation
  -> backend-neutral KernelIr
     -> generated Rust -> rust-aot
     -> arith/math/memref/scf MLIR -> LLVM IR -> mlir-aot
```

The MLIR timing shim contains no simulation math. Generated functions report
the buffer sizes, initialize state, execute one turn, and execute a resident
multi-turn loop. The shim calls the multi-turn function once so the comparison
does not include a C ABI crossing per turn. Before timing, `run.sh` requires
the Rust and MLIR state checksums to match after 1,000 turns.

## Requirements

- a release Rust toolchain
- `mlir-opt` and `mlir-translate` from the same LLVM release
- a C compiler that accepts the emitted LLVM IR

On Apple Silicon with Homebrew LLVM:

```sh
brew install llvm
benchmarks/runtime/gate-b/aot-mlir/run.sh 1000000 5
```

Override `MLIR_BIN`, `MLIR_OPT`, `MLIR_TRANSLATE`, `CC`, or `BUILD_DIR` for
another installation. Generated and lowered artifacts are retained under
`target/mech/aot-mlir` for inspection.

## Scope

This is an optional comparison backend, not a runtime dependency. It currently
accepts one fixed-shape numeric turn and compile-time gather indices. Lifted
batches are rejected with a diagnostic. The emitted module uses standard
`arith`, `math`, `memref`, `func`, and `scf` dialects and exposes a
stable C ABI. Transactions, validation, receipts, hosts, scheduling, and GPU
lowering remain outside this prototype.

## Apple M1 result

Nine alternating-order, fresh-process samples of one million turns on
2026-08-12:

| Backend | Median ns/turn | Turns/s | Relative |
|---|---:|---:|---:|
| generated MLIR/LLVM | 31.700 | 31.55 M | 0.982x |
| generated Rust | 32.266 | 31.00 M | 1.000x |

Both backends report state checksum `2826371988739629776`. The 1.8% MLIR
advantage is narrow and should not be generalized beyond this kernel or these
compiler versions. It proves that the alternative backend is competitive and
that keeping the resident loop visible to the optimizer matters: without the
private kernel's `alwaysinline` attribute, MLIR measured 34.816 ns/turn.

Environment: Apple M1, macOS 15.6.1; Rust 1.96.0-nightly using LLVM 22.1.0;
Homebrew MLIR and Clang 22.1.8. Raw samples are in
`apple-m1-2026-08-12.csv`.
