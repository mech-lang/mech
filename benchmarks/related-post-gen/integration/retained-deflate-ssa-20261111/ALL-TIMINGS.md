All rows come from the same 31-round randomized first cohort. The continuous post-JSON timer includes setup, admission, allocations, worker creation/run/join, cleanup and materialization. Preinitialized rows have ineligible raw timer scopes. No latency confirmation was run.

| Variant | Timer ms | × fastest eligible Rust | Whole-process ms | Restore ms |
| --- | ---: | ---: | ---: | ---: |
| `rust` | 8.760458 | 5.784707 | 30.421000 | — |
| `rust-preinitialized` | 1.858959 | ineligible | 23.255667 | — |
| `rust-mech-control-streaming` | 1.609750 | 1.062950 | 23.328000 | — |
| `rust-mech-control-u32-casts` | 1.605000 | 1.059814 | 23.475458 | — |
| `rust-mech-control-u8-ids-u32` | 1.608250 | 1.061960 | 23.588458 | — |
| `rust-ordered-scan-only` | 1.556041 | 1.027485 | 23.009291 | — |
| `rust-ordered-ordered` | 1.538500 | 1.015902 | 23.297958 | — |
| `rust-ordered-mask-reuse` | 1.545459 | 1.020498 | 23.508084 | — |
| `rust-ordered-ordered-mask` | 1.531292 | 1.011143 | 22.756500 | — |
| `rust-ordered-ordered-mask-tables` | 1.518917 | 1.002971 | 23.360875 | — |
| `rust-apple-m3` | 8.775333 | 5.794529 | 29.925833 | — |
| `rust-preinitialized-apple-m3` | 1.937208 | ineligible | 23.810292 | — |
| `rust-mech-control-streaming-apple-m3` | 1.607541 | 1.061492 | 23.348292 | — |
| `rust-mech-control-u32-casts-apple-m3` | 1.616458 | 1.067380 | 23.030083 | — |
| `rust-mech-control-u8-ids-u32-apple-m3` | 1.603458 | 1.058796 | 23.453708 | — |
| `rust-ordered-scan-only-apple-m3` | 1.608542 | 1.062153 | 23.525625 | — |
| `rust-ordered-ordered-apple-m3` | 1.618167 | 1.068508 | 22.975958 | — |
| `rust-ordered-mask-reuse-apple-m3` | 1.605291 | 1.060006 | 23.737708 | — |
| `rust-ordered-ordered-mask-apple-m3` | 1.609416 | 1.062730 | 23.461292 | — |
| `rust-ordered-ordered-mask-tables-apple-m3` | 1.617333 | 1.067958 | 23.844208 | — |
| `rust-ordered-mask-tables256` | 1.514417 | 1.000000 | 22.982709 | — |
| `rust-ordered-mask-tables256-apple-m3` | 1.517542 | 1.002064 | 23.168667 | — |
| `mech-ir308-numeric-owner-handoff-4` | 1.713000 | 1.131128 | 49.713042 | 21.892583 |
| `mech-ir315-counts-postings32-4` | 1.683458 | 1.111621 | 48.521833 | 21.341458 |
| `mech-ir317-retained-ssa-4` | 1.675333 | 1.106256 | 47.630125 | 21.298708 |
