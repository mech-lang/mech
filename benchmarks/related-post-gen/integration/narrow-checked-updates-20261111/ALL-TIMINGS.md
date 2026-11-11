## First cohort

Fastest eligible post-JSON reference: `rust-ordered-mask-tables256-apple-m3`. All rows retain their own first samples. Preinitialized rows show only their ineligible raw timer; eligible rows show post-JSON. Restoration is excluded from post-JSON and included in whole process.

| Variant | Reported timer ms | × fastest eligible Rust | Whole-process ms | Restore ms |
| --- | ---: | ---: | ---: | ---: |
| `rust` | 8.724125 | 5.785866 | 29.645208 | — |
| `rust-preinitialized` | 1.864167 | ineligible | 22.691750 | — |
| `rust-mech-control-streaming` | 1.599041 | 1.060489 | 22.727625 | — |
| `rust-mech-control-u32-casts` | 1.593500 | 1.056814 | 22.841208 | — |
| `rust-mech-control-u8-ids-u32` | 1.599083 | 1.060517 | 23.249208 | — |
| `rust-ordered-scan-only` | 1.554708 | 1.031087 | 22.483250 | — |
| `rust-ordered-ordered` | 1.530542 | 1.015060 | 22.121209 | — |
| `rust-ordered-mask-reuse` | 1.537166 | 1.019453 | 22.529416 | — |
| `rust-ordered-ordered-mask` | 1.520000 | 1.008069 | 22.810500 | — |
| `rust-ordered-ordered-mask-tables` | 1.512209 | 1.002902 | 22.694375 | — |
| `rust-apple-m3` | 8.715750 | 5.780311 | 28.776500 | — |
| `rust-preinitialized-apple-m3` | 1.877417 | ineligible | 23.352875 | — |
| `rust-mech-control-streaming-apple-m3` | 1.607125 | 1.065850 | 22.536542 | — |
| `rust-mech-control-u32-casts-apple-m3` | 1.605625 | 1.064855 | 23.405500 | — |
| `rust-mech-control-u8-ids-u32-apple-m3` | 1.605167 | 1.064552 | 22.833958 | — |
| `rust-ordered-scan-only-apple-m3` | 1.608625 | 1.066845 | 22.516750 | — |
| `rust-ordered-ordered-apple-m3` | 1.596291 | 1.058665 | 22.987500 | — |
| `rust-ordered-mask-reuse-apple-m3` | 1.602458 | 1.062755 | 22.996917 | — |
| `rust-ordered-ordered-mask-apple-m3` | 1.591542 | 1.055515 | 22.514958 | — |
| `rust-ordered-ordered-mask-tables-apple-m3` | 1.601709 | 1.062258 | 22.619792 | — |
| `rust-ordered-mask-tables256` | 1.507959 | 1.000083 | 22.585625 | — |
| `rust-ordered-mask-tables256-apple-m3` | 1.507834 | 1.000000 | 22.758166 | — |
| `mech-ir308-numeric-owner-handoff-4` | 1.714834 | 1.137283 | 48.793416 | 21.765125 |
| `mech-ir314-baseline-4` | 1.707417 | 1.132364 | 47.721667 | 21.759209 |
| `mech-ir314-counts32-4` | 1.704125 | 1.130181 | 47.510875 | 21.507208 |
| `mech-ir314-postings32-4` | 1.701667 | 1.128551 | 47.180625 | 21.524292 |
| `mech-ir314-counts-postings32-4` | 1.723500 | 1.143030 | 47.534042 | 21.237084 |
| `mech-ir315-baseline-4` | 1.703250 | 1.129600 | 47.185958 | 21.790917 |
| `mech-ir315-counts32-4` | 1.707167 | 1.132198 | 46.575750 | 21.428417 |
| `mech-ir315-postings32-4` | 1.702917 | 1.129380 | 47.058500 | 21.525125 |
| `mech-ir315-counts-postings32-4` | 1.678834 | 1.113408 | 47.146291 | 21.205500 |

## Confirmation cohort

Fastest eligible post-JSON reference: `rust-ordered-mask-tables256`. All rows retain their own first samples. Preinitialized rows show only their ineligible raw timer; eligible rows show post-JSON. Restoration is excluded from post-JSON and included in whole process.

| Variant | Reported timer ms | × fastest eligible Rust | Whole-process ms | Restore ms |
| --- | ---: | ---: | ---: | ---: |
| `rust` | 8.631208 | 5.759576 | 29.941167 | — |
| `rust-preinitialized` | 1.897542 | ineligible | 22.674834 | — |
| `rust-mech-control-streaming` | 1.599375 | 1.067257 | 22.254625 | — |
| `rust-mech-control-u32-casts` | 1.594250 | 1.063838 | 22.607833 | — |
| `rust-mech-control-u8-ids-u32` | 1.587334 | 1.059223 | 22.665584 | — |
| `rust-ordered-scan-only` | 1.549792 | 1.034171 | 22.167375 | — |
| `rust-ordered-ordered` | 1.517208 | 1.012428 | 22.593208 | — |
| `rust-ordered-mask-reuse` | 1.539083 | 1.027025 | 21.954875 | — |
| `rust-ordered-ordered-mask` | 1.517000 | 1.012289 | 22.555500 | — |
| `rust-ordered-ordered-mask-tables` | 1.516209 | 1.011761 | 22.340417 | — |
| `rust-apple-m3` | 8.685833 | 5.796027 | 28.490250 | — |
| `rust-preinitialized-apple-m3` | 1.877500 | ineligible | 22.789334 | — |
| `rust-mech-control-streaming-apple-m3` | 1.602666 | 1.069454 | 22.717250 | — |
| `rust-mech-control-u32-casts-apple-m3` | 1.608125 | 1.073096 | 22.726417 | — |
| `rust-mech-control-u8-ids-u32-apple-m3` | 1.607750 | 1.072846 | 22.981250 | — |
| `rust-ordered-scan-only-apple-m3` | 1.595792 | 1.064867 | 22.151542 | — |
| `rust-ordered-ordered-apple-m3` | 1.607583 | 1.072735 | 22.265416 | — |
| `rust-ordered-mask-reuse-apple-m3` | 1.598709 | 1.066813 | 22.776167 | — |
| `rust-ordered-ordered-mask-apple-m3` | 1.598917 | 1.066952 | 22.410958 | — |
| `rust-ordered-ordered-mask-tables-apple-m3` | 1.607375 | 1.072596 | 22.798250 | — |
| `rust-ordered-mask-tables256` | 1.498584 | 1.000000 | 22.445166 | — |
| `rust-ordered-mask-tables256-apple-m3` | 1.510042 | 1.007646 | 22.630333 | — |
| `mech-ir308-numeric-owner-handoff-4` | 1.707916 | 1.139687 | 48.458292 | 21.614084 |
| `mech-ir314-baseline-4` | 1.709875 | 1.140994 | 47.420416 | 21.736625 |
| `mech-ir314-counts-postings32-4` | 1.713500 | 1.143413 | 47.040916 | 21.221542 |
| `mech-ir315-baseline-4` | 1.702167 | 1.135850 | 46.364125 | 21.647292 |
| `mech-ir315-counts-postings32-4` | 1.675500 | 1.118055 | 46.350500 | 21.170333 |

