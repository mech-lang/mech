# Cross-language n-body steady-state benchmark

This suite compares the standalone Mech AOT five-body program with portable
programs from the Computer Language Benchmarks Game and an equivalent NumPy
matrix formulation. It measures only the million-turn simulation loop.
Initialization, momentum offset, energy checks, native compilation, and Julia
JIT warmup are outside the timed interval.

Sources:

- Mech: `examples/aot-n-body/n-body.mec`
- Rust: Benchmarks Game Rust #3, portable safe implementation
- Lua and LuaJIT: Benchmarks Game Lua #2, same source under both runtimes
- Julia: Benchmarks Game Julia #5, including its fixed tuple/unrolled form
- Python: standard Benchmarks Game Python program
- NumPy: the Mech program's fixed pair table and incidence-matrix algebra

The Benchmarks Game sources are redistributed under
`BENCHMARKS-GAME-LICENSE.txt`. Timing and CSV telemetry are the only material
changes. Every language program is first checked at the official `N=1000`
checkpoint: initial energy `-0.169075164`, final energy `-0.169087605`, absolute
tolerance `1e-8`.

## Method

- five fresh processes per implementation
- one million turns per process
- implementation order rotates between samples
- Rust uses `-C opt-level=3 -C target-cpu=native -C codegen-units=1`
- Julia uses `-O3`, no startup file, the source-requested LLVM unroll threshold,
  and a one-turn JIT warmup on a disposable copy
- NumPy/OpenBLAS is restricted to one thread
- Python cyclic GC remains enabled and reports collection count
- Lua reports heap growth; Julia reports GC time and allocated bytes
- Mech uses the `fast` AOT envelope: one resident state buffer, relaxed
  fixed-shape math, no rollback, and no finite-value validation

Run with:

```sh
benchmarks/runtime/gate-b/nbody-languages/run.sh 1000000 5
```

This is a steady-state throughput comparison, not a command-startup benchmark.
The programs use the same five bodies and symplectic integration step, but the
Julia Game #5 reciprocal-square-root approximation and different accumulation
orders can produce small floating-point differences after long runs.

## Apple M1 results

Medians over five fresh one-million-turn processes on 2026-08-12. Lower is
better. The fast Mech result comes from this suite. The other Mech envelopes
come from the companion `../aot-n-body-guarantees` run of the same generated
artifact on the same machine.

| Implementation | Median ns/turn | Turns/s | Time vs Rust |
|---|---:|---:|---:|
| Rust Game #3 | 26.242 | 38.11 M | 1.00x |
| Mech AOT fast | 32.300 | 30.96 M | 1.23x |
| Julia Game #5 | 38.132 | 26.22 M | 1.45x |
| LuaJIT running Lua Game #2 | 105.358 | 9.49 M | 4.02x |
| Mech AOT atomic | 138.847 | 7.20 M | 5.29x |
| Mech AOT checked | 148.017 | 6.76 M | 5.64x |
| Mech AOT receipt | 171.157 | 5.84 M | 6.52x |
| Lua Game #2 | 1,918.660 | 0.521 M | 73.12x |
| Python Game | 5,346.931 | 0.187 M | 203.76x |
| NumPy matrix | 10,676.946 | 0.094 M | 406.86x |

The generated fast Mech path is 4.31x faster than its previous 139.236 ns
result. It is 23% slower than the portable Rust Game source and 15% faster than
Julia Game #5 in elapsed time. This is not a strict-guarantee result: fast uses
constant-power specialization and sparse constant-matrix simplification. The
atomic, checked, and receipt paths retain the explicit graph operations.

Fast and strict Mech energy agree to 12 decimal places after one million turns;
their state checksums differ because their floating-point operation order
differs. The reference-language runs reported no timed cyclic-GC collections;
Julia reported zero timed allocations, while LuaJIT retained 8,324 bytes after
warmup. NumPy's zero Python-GC count does not account for native ndarray
temporary allocation and reference-counted reclamation.

Raw samples are in `results/apple-m1-2026-08-12.csv`; exact tool versions and
hardware are in `ENVIRONMENT.md`.
