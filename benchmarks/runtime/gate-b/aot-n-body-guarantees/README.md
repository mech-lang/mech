# AOT n-body guarantee envelopes

This benchmark builds `examples/aot-n-body/n-body.mec` through the public
`mech build --aot` path, then runs each guarantee mode in five fresh processes.
The workload is the five-body advance step from the Computer Language
Benchmarks Game, with the standard initial momentum offset.

Apple M1 medians over five one-million-turn processes (2026-08-12):

| Mode | Median ns/turn | Median turns/s | Delta from fast |
|---|---:|---:|---:|
| fast | 139.088 | 7,189,676 | baseline |
| atomic | 138.896 | 7,199,644 | -0.14% (noise) |
| checked | 148.369 | 6,739,943 | +6.67% |
| receipt | 176.623 | 5,661,766 | +26.99% |

`atomic` computes into a second 30-value state buffer before publication.
`checked` additionally scans all 30 values for non-finite results. `receipt`
also hashes the 30 values before and after every turn and chains the receipt
through the timed loop. It is not the complete `MechRuntime` transaction stack.
All four modes report the same final state checksum; only `receipt` reports a
non-zero receipt checksum. The state checksum also changes between 10 and one
million turns, and the checked million-turn run completes without a non-finite
value. The fast/atomic difference is smaller than run-to-run timing variation.

Run with:

```sh
benchmarks/runtime/gate-b/aot-n-body-guarantees/run.sh 1000000 5
```
