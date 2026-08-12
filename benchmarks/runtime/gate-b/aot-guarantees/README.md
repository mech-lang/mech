# AOT guarantee envelope benchmark

This benchmark starts with [`examples/aot-ekf/ekf.mec`](../../../../examples/aot-ekf/ekf.mec),
runs the real `mech build --aot` path, and executes the resulting native binary.
Every mode calls the same generated EKF turn function.

Apple M1 Mac mini, macOS 15.6, release build, 1,000,000 turns per process, five
processes per mode (2026-08-12):

| Mode | Median ns/turn | Median turns/s | Delta from fast |
|---|---:|---:|---:|
| fast | 109.047 | 9,170,351 | baseline |
| atomic | 109.374 | 9,142,976 | +0.30% |
| checked | 109.785 | 9,108,730 | +0.68% |
| receipt | 111.829 | 8,942,198 | +2.55% |

The corrected harness marks both kernel entry points `#[inline(never)]`, passes
validation and hashing inputs through `black_box`, and chains each receipt into
the next turn. Disassembly confirms distinct kernel calls, a 12-value finite
scan, and two 12-value hash loops inside the timed turn loop.
The output reports the state and receipt checksums separately. Every mode reaches
the same final state; only `receipt` produces a non-zero receipt chain.

`receipt` is deliberately narrow: candidate-state isolation, finite-value
validation, and before/after receipt hashing. It does not claim the full
`MechRuntime` transaction, capability, dirty-graph, effect-outbox, or retained
ledger guarantees. Benchmarking that outer managed envelope around the same
generated function requires the mixed runtime/AOT executor boundary that this
prototype identifies but does not yet implement.

Run with:

```sh
benchmarks/runtime/gate-b/aot-guarantees/run.sh 1000000 5
```
