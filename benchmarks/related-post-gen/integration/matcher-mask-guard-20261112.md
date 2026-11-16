# Related-post guard on the matcher compiler

The managed related-post program passes the original #990 Rust target on the compiler used for the matcher bounds recovery. The fixed upstream `rust_con` comparator remains at `fc2982b0c929b1b701209d401a1dc30c773ef4ab`.

| Cohort | Mech post-JSON | Rust processing | Mech/Rust | Pairs faster |
| --- | ---: | ---: | ---: | ---: |
| Primary | 1.768542 ms | 2.464917 ms | 0.717485× | 30/31 |
| Confirmation | 1.781875 ms | 2.468334 ms | 0.721894× | 13/13 |

All 88 outputs agree exactly. The preparation adapter initially stopped on a missing display category. Both cohorts ran after that configuration was restored from the prior verified confirmation report. The fixed verifier checked all 62 primary calls and 26 confirmation calls and reads the original timer fields directly. Whole-process times remain in the raw reports.

Compiler base `59bf914ce249d74ea3f2f1ebd3b4db2d1ee90a82` plus patch SHA-256 `d8263537791901f8cc348b3f798ad1252a9189c0cc06541fb3bf0aac5f9f068b`. Managed Object SHA-256 `19899c35adbfbd9cec77506e437afa8d3cf2cc15ba8b661eef9c2746893b4ff5`.

[Raw observations and offline verifier](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/related-post-matcher-mask-d9f129ee0867.tar.gz).
