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
- Mech uses the `fast` AOT envelope: one resident state buffer, no rollback or
  finite-value validation

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
| Rust Game #3 | 26.261 | 38.08 M | 1.00x |
| Julia Game #5 | 38.703 | 25.84 M | 1.47x |
| LuaJIT running Lua Game #2 | 106.604 | 9.38 M | 4.06x |
| Mech AOT atomic | 138.896 | 7.20 M | 5.29x |
| Mech AOT fast | 139.236 | 7.18 M | 5.30x |
| Mech AOT checked | 148.369 | 6.74 M | 5.65x |
| Mech AOT receipt | 176.623 | 5.66 M | 6.73x |
| Lua Game #2 | 1,943.605 | 0.515 M | 74.01x |
| Python Game | 5,356.442 | 0.187 M | 203.97x |
| NumPy matrix | 10,699.477 | 0.093 M | 407.43x |

The apparent 0.24% advantage for Mech atomic over fast is below run-to-run
variation; it is not evidence that copying is free or beneficial. Checked adds
finite-value validation and receipt also hashes and chains every state. The
reference-language runs reported no timed cyclic-GC collections; Julia reported
zero timed allocations, while LuaJIT retained 8,324 bytes after warmup. NumPy's
zero Python-GC count does not account for native ndarray temporary allocation
and reference-counted reclamation.

Raw samples are in `results/apple-m1-2026-08-12.csv`; exact tool versions and
hardware are in `ENVIRONMENT.md`.
