# AOT guarantee envelope benchmark

This benchmark starts with [`examples/aot-ekf/ekf.mec`](../../../../examples/aot-ekf/ekf.mec),
runs the real `mech build --aot` path, and executes the resulting native binary.
Every mode calls the same generated EKF turn function.

Apple M1 Mac mini, macOS 15.6, release build, 1,000,000 turns per process, five
processes per mode (2026-08-12):

| Mode | Median ns/turn | Median turns/s | Delta from fast |
|---|---:|---:|---:|
| fast | 108.838 | 9,187,950 | baseline |
| atomic | 109.108 | 9,165,273 | +0.25% |
| checked | 109.061 | 9,169,177 | +0.20% |
| transactional prototype | 109.686 | 9,116,878 | +0.78% |

The atomic and checked differences are within run-to-run noise on this machine.
Receipt hashing is measurable but remains below one percent for this fused EKF.

`transactional prototype` is deliberately narrow: candidate-state isolation,
finite-value validation, and before/after receipt hashing. It does not claim the
full `MechRuntime` transaction, capability, dirty-graph, effect-outbox, or
retained-ledger guarantees. Benchmarking that outer managed envelope around the
same generated function requires the mixed runtime/AOT executor boundary that
this prototype identifies but does not yet implement.

Run with:

```sh
benchmarks/runtime/gate-b/aot-guarantees/run.sh 1000000 5
```
