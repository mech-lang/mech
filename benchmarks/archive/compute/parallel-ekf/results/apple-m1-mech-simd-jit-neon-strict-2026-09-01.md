# Mech resident SIMD-JIT optimization

Apple M1, 10,000 independent filters, 20 resident turns, one worker, three
steady-state samples. The benchmark keeps strict arithmetic in both modes;
"unchecked" only removes integrity publication checks.

| Runtime | Checked (M turns/s) | Unchecked (M turns/s) |
| --- | ---: | ---: |
| Mech Cranelift SIMD-JIT CPU | 41.20 | 49.79 |
| Futhark ISPC scalarized SIMD, strict one worker | 30.55 | 43.34 |

The optimized Mech path is 35% faster than the strict Futhark checked control
and 15% faster on the strict unchecked control. The checksum and maximum error
remain stable across the three runs. The optimization is generic resident SIMD
lowering: direct AArch64 NEON transcendental helpers and elimination of
redundant multiplication by literal `+1`/`-1`; no EKF-specific kernel is added.
