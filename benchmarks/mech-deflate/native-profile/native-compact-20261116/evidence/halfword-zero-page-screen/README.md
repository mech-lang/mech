# Large halfword zero-page screen

The exact reference worker allocates 60 payloads on `prefix-periodic-1024`.
Separate instrumentation records 59 ordinary allocations and one mapping.
The median mapped allocation and release total **1.79 µs**. Ordinary
allocation and release total **9.73 µs**. The profiler adds measurable
per-operation clock overhead. [Raw diagnostic](profile-observations.json.gz)

A fixed-encoder causal screen changes the Darwin mapping threshold for
2-byte-aligned payloads from 64 to 32 pages. Both workers link the exact
71,768 B encoder and match 162 output and resource observations and six
checked failures. Peak stack is 5,280 B for the reference and 5,264 B for
the variant. [Worker bindings](link-bindings.json) · [Checks](contract-checks/result.json)

| Periodic 1 KiB | Mech reference | Mech mapping candidate | libdeflate |
| --- | ---: | ---: | ---: |
| Forward | 44.12 µs | 63.12 µs | 45.29 µs |
| Reverse | 44.12 µs | 63.46 µs | 45.04 µs |

Both focused orders lose the 1 KiB and 4 KiB periodic peer wins and the
64 B and 1 KiB random peer wins against libdeflate. They both score 10/10
against Roc. The focused v2 decision is **Reject**. The 71,768 B reference
worker remains the deployment comparison.
[Predeclared plan](plan.json) · [Focused replay](focus-replay.json)
· [Runtime change](zero-pages.patch.gz) · [Runtime build receipt](runtime-build.json)
