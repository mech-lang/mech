# AOT n-body guarantee envelopes

This benchmark builds `examples/aot-n-body/n-body.mec` through the public
`mech build --aot` path, then runs each guarantee mode in five fresh processes.
The workload is the five-body advance step from the Computer Language
Benchmarks Game, with the standard initial momentum offset.

Apple M1 medians over five one-million-turn processes (2026-08-12):

| Mode | Median ns/turn | Median turns/s | Delta from fast |
|---|---:|---:|---:|
| fast | 32.328 | 30,932,778 | baseline |
| atomic | 138.847 | 7,202,183 | +329.5% |
| checked | 148.017 | 6,755,967 | +357.9% |
| receipt | 171.157 | 5,842,605 | +429.5% |

`fast` mutates one state buffer in place and permits fixed-shape algebraic
simplification. The compiler replaces selected constant powers such as
`distance-squared^-1.5` with multiplication and square root, removes products
with known zero, and removes multiplication by known one. These transformations
preserve the finite n-body calculation to the measured tolerance, but are not
strictly equivalent for every IEEE-754 NaN, infinity, or signed-zero input.

`atomic` computes the explicit Mech graph, including `powf`, into a second
30-value state buffer before publication. `checked` additionally scans all 30
values for non-finite results. `receipt` also hashes the 30 values before and
after every turn and chains the receipt through the timed loop. These modes do
not use relaxed math. They are not the complete `MechRuntime` transaction stack.

The fast checksum differs because its legal reassociation changes low floating-
point bits. At the official 1,000-turn checkpoint and after one million turns,
fast and strict energy agree to 12 decimal places. Maximum state difference is
`5.329e-15` after 1,000 turns and `9.612e-9` after one million. Atomic, checked,
and receipt report the same strict state checksum; only receipt reports a
non-zero receipt checksum.

Run with:

```sh
benchmarks/runtime/gate-b/aot-n-body-guarantees/run.sh 1000000 5
```
