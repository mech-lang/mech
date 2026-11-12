# Related-post guard on the first recovery compiler

The managed related-post program passes the original #990 Rust target on the compiler used for the first compression recovery. The fixed upstream `rust_con` comparator remains at `fc2982b0c929b1b701209d401a1dc30c773ef4ab`.

| Cohort | Mech post-JSON | Rust processing | Mech/Rust | Pairs faster |
| --- | ---: | ---: | ---: | ---: |
| Primary | 1.840625 ms | 2.692750 ms | 0.683548× | 30/31 |
| Confirmation | 2.003000 ms | 2.836625 ms | 0.706121× | 12/13 |

All 88 outputs agree exactly. The primary runner hit a missing display category when constructing its summary after saving all 62 successful calls. The fixed verifier replayed those calls unchanged and validated the 26 fresh confirmation calls. The gate reads the original measured timer fields directly. Whole-process times remain in the raw reports.

Compiler base `ef3beb1985bbbf8e17e9436d1013ebb4a7eb215f` plus patch SHA-256 `4153645f90ced5c2cd5f70e5ecaedeb34e2de591e96cdc6234529506bf0ae6ce`. Managed Object SHA-256 `0f5373352797aef2ffae5172fa7a345cbec465c9ade379d3b5424b8357affa9e`.

[Raw observations and offline verifier](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/related-post-first-recovery-12b0ecd95ebd.tar.gz).
