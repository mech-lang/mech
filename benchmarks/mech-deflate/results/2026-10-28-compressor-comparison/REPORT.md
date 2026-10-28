# Complete-stream compression comparison

Base: `a82d50823e4140edd862886b0c6b5f813a81411c` (#970). The Mech compiler and compressor components are unchanged. This packet contains measurements requested after the first complete-program baseline.

Selected series: `deflate-library-comparison-isolated`, exit0 in53.937131882seconds. All builds completed before timing. No compiler or compression process was present before or after, and no other task work ran during the selected series. The machine remained available to normal user applications; this is not an isolated laboratory machine.

Apple M1, macOS15.6.1, pinned Roc compiler `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d` with `--no-cache --opt=speed`; Rust nightly2026-03-03, release optimization3. Roc and both Rust implementations directly use the same pinned mimalloc2.1.7 object. `zlib-rs` has only its Rust allocator feature path. The reference is level12; miniz_oxide0.9.1 is level10; zlib-rs0.6.8 is level9. These are different compression algorithms/search effort, so compare output sizes as well as elapsed times.

Each entry below is **median microseconds / raw compressed bytes**. Prefix bytes are preserved but excluded from raw size. Each small entry has33 samples, each full-file entry15, across three rounds with rotating implementation order:1134 samples total. All results independently decompress to the exact input. All reference outputs reproduce the pinned exact bytes; Rust results may differ.

| Input case | Bytes | Roc12 | miniz_oxide10 | zlib-rs9 |
|---|---:|---:|---:|---:|
| empty | 0 | <1 / 5 | 6.292 / 2 | 8.250 / 2 |
| one-stored | 1 | <1 / 6 | 6.542 / 3 | 5.792 / 3 |
| seven-stored-prefix | 7 | <1 / 12 | 6.416 / 9 | 9.708 / 9 |
| eight-literals | 8 | 286.000 / 10 | 6.541 / 10 | 9.500 / 10 |
| eight-overlap | 8 | 294.000 / 5 | 6.667 / 5 | 7.875 / 5 |
| 320-overlap | 320 | 301.000 / 7 | 7.042 / 17 | 9.875 / 7 |
| eight-literals-prefix | 8 | 289.000 / 10 | 5.458 / 10 | 9.583 / 10 |
| 128-biased-dynamic | 128 | 287.000 / 65 | 7.500 / 70 | 10.125 / 70 |
| 32-high-entropy-stored | 32 | 286.000 / 37 | 4.250 / 38 | 9.916 / 37 |
| 16384-distribution-change | 16384 | 790.000 / 11394 | 191.875 / 13384 | 204.458 / 13384 |
| 33027-periodic-window | 33027 | 1243.000 / 151 | 37.375 / 150 | 37.583 / 151 |
| full-reymont | 6627202 | 937136.000 / 1696488 | 555926.000 / 1825107 | 226522.709 / 1826397 |

The reference clock resolution is1microsecond. Its zero medians for0/1/7-byte bypass cases mean below measurement resolution, not zero execution cost. `samples.json.gz` preserves every observation without modification. Reusable output allocation, clear/prefix setup, validation and logging are outside the timer. Fresh compressor state creation and destruction remain inside on every call. The Rust output backing pointer is checked unchanged.

For16384 bytes, Roc takes0.790ms and produces11394bytes; the Rust libraries take about0.192ms and0.204ms but each produces13384bytes. For the full6627202-byte file, Roc takes0.937136s and produces1696488bytes; miniz_oxide takes0.555926s and produces1825107bytes; zlib-rs takes0.226522709s and produces1826397bytes.

The previous Mech16384-byte measurement is3051.9263705seconds. Its caller initialization, System allocator, allocation probe and concurrent work differ from this series, so a ratio is not a controlled compiler comparison. The separate33027-byte Mech window case also passes exact bytes/decompression/aliases/cleanup in5050.469946709seconds of wall time, including approximately53seconds deliberately paused during the earlier comparison. The full-file Mech attempt was deliberately stopped; no full-file result exists and no long job remains running.

Run the checked-in comparison harness to reproduce. `build.json` pins inputs, all fourteen reference sources, compilers, dependencies, objects and executables; `run.json` pins the actual runner. Generated applications, all108 raw per-process logs,1134 observations,36 summaries and successful build receipts are retained. Small Rust streams are preserved compressed; full Rust stream hashes/sizes and observed independent verification are retained without duplicating3.65MB of binary output.

Earlier receipts are included honestly: the first outer runner incorrectly rejected below-resolution zero durations; that check was fixed. A later pre-timing PID guard found the window job had already completed and aborted before any timing; its successful final builds were reused only after exact source, input, lock and executable checks. The selected unchanged-binary series then completed in full with no active benchmark jobs. These are harness/environmental observations, not compressor/compiler failures. No component qualification expectations, allowances, algorithms, tables, buffers or timing code in the Mech program changed.

CI remains canceled and the comparison PR remains draft pending review. No production compiler optimization is included.
