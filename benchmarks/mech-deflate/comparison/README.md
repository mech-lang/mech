# Complete-stream library comparison

Run the exact complete-stream inputs used by the Mech qualification through the
pinned reference compressor and two pure Rust compressors. This is a library
comparison, not an identical-algorithm or language-speed comparison. The Rust
libraries use their own match finders, parsers and allocation layouts, and can
produce different compressed sizes.

The reference remains at source revision
`671077457ff7ef3a3d44caf715a3fc595addf379` and compiler revision
`d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`, level12. All fourteen source hashes
are checked before building. The runtime entry argument selects the level, as in
the retained reference benchmark. Inputs are embedded in the reference binary;
Rust reads them before warmup. The inputs and expected streams come from the
maintained fixtures, including both prefix cases, both boundary cases and the
full 6627202-byte corpus file. No compressor implementation is modified.

Rust uses `miniz_oxide`0.9.1 at level10 and `zlib-rs`0.6.8 at level9, their
respective highest supported compression levels. These settings are not
equivalent search effort to reference level12. `zlib-rs` disables its default
features and enables `std`, selecting the Rust allocator. Neither Rust row calls
the C libdeflate or C zlib compressor. Both use the exact pinned mimalloc2.1.7
object used by the reference host, SHA256
`38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.
The runner verifies it before building. Raw DEFLATE is used, without a zlib or
gzip wrapper.

Each case and implementation gets a fresh process, at least200ms of warmup,
and a reused output reservation. The timer covers compression, including fresh
compressor scratch initialization and destruction on every call. Output
allocation, clear/prefix preparation, decoding, equality checks and printing
are outside the timer. Rust uses a conservative `zlib-rs::compress_bound` output
reservation; the reference uses its original `Deflate.compress_bound`.
The reference is built with `--no-cache --opt=speed`; Rust uses the locked
release profile at optimization level3. Three rounds rotate implementation
order, with eleven samples per small case and five per full-file case per
round:1134 samples across twelve cases and three implementations.

Every reference result must equal the pinned expected stream. Every Rust result
must equal its own warmed result, preserve the supplied prefix and output
backing pointer, and independently decompress with Python's zlib to the exact
input. The checked-in summary records whether each Rust output also equals the
reference; valid output is allowed to differ. Reference expected streams are
also independently decompressed before execution. On this macOS host, the
reference's `CLOCK_MONOTONIC` clock has1microsecond resolution. Zero-duration
samples are retained as below resolution, never reported as literally free
compression or replaced with invented positive values.

Example, after obtaining the pinned source/compiler, host objects and corpus:

```sh
RUSTUP_TOOLCHAIN=nightly-2026-03-03 python3 benchmarks/mech-deflate/comparison/run.py \
  --reference /path/to/roc-deflate \
  --roc /path/to/pinned/roc \
  --corpus /path/to/reymont \
  --full-reference /path/to/level-12.deflate \
  --out /private/tmp/deflate-comparison
```

This host/allocator setup currently targets Apple Silicon macOS. The optional
`--pause-pid` is for the already identified local window-boundary investigation
only; the runner checks its parent and command before pausing it and resumes
it in `finally`. It is not a generic process-control interface. No ongoing run
is restarted to obtain these comparison timings.

The existing Mech measurements use a System allocator with allocation
instrumentation, include caller initialization and run alongside other
investigations. They are useful slow baselines, but do not share this final
timing/allocator protocol. Do not label a ratio to them a controlled compiler
performance comparison. The compressor source, buffers, search limits and
NoLimits settings are unchanged by this harness.
