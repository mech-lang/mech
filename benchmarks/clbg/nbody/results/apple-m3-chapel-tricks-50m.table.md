# Chapel-style optimization check

Apple M3, 50,000,000 steps, five shuffled fresh-process samples; median wall time. AOT compilation excluded; startup, initialization and both energy outputs included. Only Mech source counts toward its program size.

| Program / options | Seconds | Range (s) | Mech-only or upstream noncomment lines | gzip bytes |
| --- | ---: | ---: | ---: | ---: |
| rust-3 | 1.954217 | 1.953187–1.957038 | 196 | 1748 |
| chapel-3 | 1.705568 | 1.704299–1.708017 | 79 | 972 |
| mech-sqrt-fma | 2.028586 | 2.028052–2.032711 | 32 | 735 |
| mech-sqrt-fma-depth | 2.028100 | 2.022100–2.035845 | 32 | 735 |
| mech-power-fma | 2.029894 | 2.020234–2.047853 | 32 | 732 |
| mech-factored-fma | 2.067036 | 2.066199–2.068430 | 32 | 740 |
| mech-pairwise-strict | 2.184892 | 2.179484–2.201490 | 38 | 877 |
| mech-pairwise-fma | 1.983660 | 1.980298–2.002741 | 38 | 877 |
| mech-pairwise-fma-depth | 1.982504 | 1.978445–1.990452 | 38 | 877 |

Rust #3 is the fastest of five measured ARM-compatible Rust submissions in the earlier full cohort. Published Rust #9 requires x86 intrinsics and is excluded on this machine.

The pairwise Mech source shares displacement × magnitude and interleaves both velocity contributions per pair. It changes arithmetic/accumulation order, passes the benchmark energy checks and all 28 native full-state comparisons at 0, 1, 1,000 and 10,000 steps. Maximum scaled errors: 2.644e-12 versus resident Mech and 2.257e-12 versus independent Rust.

The depth scheduler emits identical hot-loop machine code for each tested source; its overlapping timing ranges do not establish a scheduling benefit.

| Hot loop | Instructions | Stack loads | Stack stores | Fused operations |
| --- | ---: | ---: | ---: | ---: |
| Mech original + FMA | 314 | 89 | 33 | 65 |
| Mech pairwise + FMA | 251 | 58 | 22 | 65 |
| Chapel #3 (earlier audit) | 204 | 25 | 9 | 44 |

Static code-generation counts are diagnostics, not an isolated causal attribution of runtime differences. All ten interactions remain: Mech emits five f64x2 square roots and divisions per step, with no hot-loop library calls.

Exact samples, commands, source hashes and compiler snapshot: [JSON report](apple-m3-chapel-tricks-50m.json).
