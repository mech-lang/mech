# Carry the reverse-read address through ordered reductions

The checked ordered-reduction proof already admits every visit to a reverse window before entry. This change carries the previous element address into the scalar continuation. The first short visit computes its original address, and a vector remainder starts from the last vector element's address. The continuation subtracts the element width before reading the next low word. The cursor is local to the current window. Other windows still use their own checked geometry. Candidate order, wrapping costs, tie behavior, the logical index, charges, and the empty and singleton exits retain their existing semantics.

The emitter uses the cursor in the existing singleton ordered reduction under `MECH_REVERSE_REDUCTION_CURSOR=1` on AArch64. The AArch64 selector can fold a last-use address update into a plain, native-endian, nontrapping I32 pre-indexed load. The updated address stays live and the load result cannot alias the tied base register. Eight of 351 generated bodies change, all in ordered reductions. The root and matcher bodies are byte-identical. The four composed bodies keep their 1,344-byte code size and 224-byte frames. Their continuing scalar route goes from 17 to 16 instructions and has no recurring stack accesses in either build. The four ordinary copies grow from 1,472 to 1,504 bytes and from 256- to 272-byte frames. The sampled Roc parser uses an 11-instruction loop with a post-indexed reverse read and a countdown.

The control is `f3a628b73cb01a7a7d8ee034714ec02b41674a5f`. The seven measured production files match the published patch by SHA-256. Both source forms emit Object `f59892df855a673815a9804fd0eb86053b7dad08043869287a1dcca295bbaa70` and metadata `ab149342064dbd6e5c55346cea4de2322ef200009bdc61f632f745576fc73bff`. The exact linked worker is `1eecb33f66053097eb8dc2b89f6b0dfff4be8f8ec466c5ada41d88136adbb59d`. The benchmark source SHA-256 is `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.

| Full matched comparison | Control | Reverse cursor |
| --- | ---: | ---: |
| Faster than Roc | 47/81 (58.0%) | 47/81 (58.0%) |
| Faster than libdeflate | 11/81 (13.6%) | 12/81 (14.8%) |
| Faster than both | 2/81 | 3/81 |
| Faster than either | 56/81 (69.1%) | 56/81 (69.1%) |

| Input | Control/Roc | Cursor/Roc | Control/libdeflate | Cursor/libdeflate | Cursor/control |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.171296× | 0.171055× | 2.191130× | 2.188053× | 0.998596× |
| Natural 8 KiB | 1.045138× | 1.034741× | 1.302618× | 1.289660× | 0.990052× |
| Natural 32 KiB | 1.159687× | 1.146255× | 1.058307× | 1.046049× | 0.988417× |
| Natural 256 KiB | 1.183392× | 1.170076× | 0.961519× | 0.950699× | 0.988747× |
| Natural 1 MiB | 1.197318× | 1.180179× | 0.966973× | 0.953130× | 0.985685× |
| Repeated 128 KiB | 0.606798× | 0.588129× | 1.031064× | 0.999342× | 0.969234× |
| Repeated 256 KiB | 0.608538× | 0.607588× | 0.990946× | 0.989399× | 0.998438× |
| Repeated 1 MiB | 0.605141× | 0.603854× | 0.953467× | 0.951439× | 0.997874× |
| Periodic 1 MiB | 0.590936× | 0.590594× | 1.001987× | 1.001405× | 0.999420× |

The full paired geometric mean is **0.996594× control**. Both 1.05 gates pass on all 81 cases over 245,664 measured calls. There are 57 lower and 24 higher point estimates. The largest increase is repeated 64 KiB at 1.025974× control. All six allocation, copying, and zeroing measures match. Repeated 128 KiB adds a narrow libdeflate win. Repeated 32,767 bytes is the nearest remaining libdeflate miss at 1.000730×.

The nine-case screen measured 0.988393× control across 6,912 calls. Natural 8 KiB was 1.005110× Roc in that cohort. Natural 1 MiB increased to 1.021115× its pilot control. In the separate instrumented cohort, natural 8 KiB instructions fell from 15,455,284 to 15,270,962 and cycles from 3,816,851 to 3,757,930. Eight-literal cycles increased to 1.028912× control. Lifetime physical peak increased by up to 557,120 bytes in the counter cohort. The profile receipts map PCs from nine cases to the exact timed worker.

The release build used `cargo +nightly-2026-03-03 build --locked --offline --release -p mech-gpu -p mech-stdlib --lib --no-default-features` with the narrow native features in `build-library.json`. The linked control, candidate original, and candidate decoded each passed 162 exact-byte, independent-inflate, capacity, alias, and cleanup checks. The maintained selector witness passed 270 JIT calls and an endian refusal check with the register allocator checker and `-D warnings`. The managed Rust comparison passed 88 exact outputs in 31 primary and 13 confirmation pairs. Mech took 0.720928× and 0.706094× the respective Rust post-JSON times. The archive includes the original commands and profiles. The changed-file unsafe audit, formatting, and `git diff --check` pass. The repository-wide unsafe audit still reports the inherited unlisted measurement probe at `benchmarks/related-post-gen/check-measurement.rs`.

[Source patch, full timing packets, machine evidence and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-f3a628b73c-reverse-cursor.tar.gz). Archive SHA-256 `475980578f734354ec130d71723a6f708bf9f150bbb0f1df0176e7ecb01f29e6`.
