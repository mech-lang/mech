# Connected level-12 compressor qualification

Compiler base: `115188e17032e26341dd7983e0634589a9a490ab` (#967, reviewed without findings). Reference source: `671077457ff7ef3a3d44caf715a3fc595addf379`; reference compiler: `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`.

The maintained source entry now connects the existing matchfinder and block collector to cost initialization, iterative path optimization, final path/model selection, block writing, state carryover and stream draining. Compression executes as one canonical native Mech program. `deflate-compress-into` selects stored handling for inputs of at most seven bytes and the near-optimal path otherwise. This patch adds source composition and qualification; it adds no compiler optimization or alternative compressor.

The controller allocates and carries the actual 262144-entry U16 tree, 1500513-word U32 cache, 305257-entry U64 path reservation and caller-owned output. It retains depth 300, nice length 258, ten optimization passes, the original near-EOF limits, block heuristics, tables and allocation sites. Six original Huffman workspace initialization sites are present before optimization. All configurable native compilation/execution/library/managed-memory allowances are selected by `NativeProgramLimitMode::NoLimits`.

## Observed execution

Seven golden cases pass for source-derived original and decoded artifacts, including stored empty/one/seven-byte streams, eight literal bytes, overlapping matches, an initial output prefix, and 320 repeated bytes. Every output is compared byte-for-byte with the pinned producer and independently decompressed by miniz_oxide. Input aliases and complete managed-account cleanup are checked. The normal maintained test always parses and compiles source; the ignored saved-artifact investigation strictly decodes bytecode and requires its adjacent source to equal the current assembled source.

The accepted source is 160992 bytes, SHA256 `345324ce551aaa2485dcd0fe4262da96e660d6b31f5f772bff89b034e9b1997f`; its bytecode is 4140067 bytes. The source control graph has 4867 arms and 4997 blocks. `validated-fixtures.json` identifies all composed fixture files.

The first optimized-host source run was built before the final configurable compiler-analysis API: its fixed graph was admitted under the temporary 512 MiB experiment, while all dynamic budgets were maximum. This is explicit in the partial receipt. It completed all fourteen small-case evaluations. The 320-byte case took 47.960433416 seconds (original) and 47.887294375 seconds (decoded), with 28801269 allocation requests and 8286259411 cumulative requested bytes each. These byte totals are neither peak memory nor copied bytes.

That same process is evaluating the complete 6627202-byte Reymont input. At the latest observation it had used over 31 minutes of CPU time, approximately 95 MiB resident memory, and had reported neither an execution error nor a completed result. Completed positions are not exposed mid-invocation. No full-file byte agreement or timing result is claimed. The expected reference stream is 1696488 bytes, SHA256 `e94512f0a11fffd64dbff8d3db270104554f02648a56ee3c4496afd12af5e0c5`; input SHA256 is `0eac0114a3dfe6e2ee1f345a0f79d653cb26c3bc9f0ed79238af4933422b7578`.

The fresh qualification against the published compiler passes: fourteen complete evaluations (seven original, seven decoded), all exact bytes, independent decompression and cleanup. The normal maintained writer suite also passes three Rust tests with twelve original/decoded/path/sequence evaluations. Compiler preparation accepts 41042 native blocks, 22464 values and an estimated 120170976 analysis bytes under `u64::MAX`. The final 320-byte measurements are 50.806926291 seconds original and 50.392539125 seconds decoded; the requested-byte/allocation counts remain identical to the earlier run. The new artifact bytecode is identical to the one used by the active full-input job. The process running the older admitted graph is preserved. The external focused release driver copies the maintained qualification helper, removes test attributes, resolves fixture paths, and invokes its entry from `main`; its exact feature manifest, lockfile and source hashes are preserved as build evidence. It uses the same canonical source frontend, bytecode codec, native compiler, catalog and runtime. The maintained Rust test remains the code owner; no duplicate compressor or maintained driver framework is added here.

## Measurement boundaries and remaining checks

These are execution investigations. Source/native compilation and argument preparation are outside each timer; the qualification's initial output reservation and prefix setup are inside. The host allocator is System. This is not the final matched benchmark boundary, allocator or repetition loop.

No known compressor function is still unwritten. Larger/multiple-block end-to-end correctness, complete Reymont agreement, output reuse at the reference timing boundary and a fair complete benchmark remain unverified. The separate block qualification covers fixed, dynamic and consecutive block writers through both payload representations. Do not infer complete multi-block compression correctness from those component checks. Optional performance tuning remains deferred.

## Failure disposition

The first external release-driver rebuild failed because its copied multiline JSON include still used a relative fixture path. Resolving the copied path fixed that build; both receipts are retained. A one-line Rust formatting difference in the new stack wrapper was corrected before final checks. The first processor-name read was denied by the local environment; the read-only retry identified Apple M1. These are investigation-scaffold/environment failures, not compression or generated-code failures.

The retained earlier full-input receipt records a library `OutputBytes` admission rejection: required 22241990 versus limit 16777216. That limit was accidentally retained by the investigation scaffold. #967 explicitly enumerates every budget at maximum, and the current full run has passed that admission. The earlier receipt also predates the six restored initialization sites and is retained as historical evidence, not as validation of the current source. No execution allowance was reduced and no golden value was adjusted to obtain a pass.

## Repeating the maintained qualification

```sh
RUSTUP_TOOLCHAIN=nightly-2026-03-03 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 \
  cargo test --release --locked -j1 -p mech-gpu --no-default-features \
  --features canonical-native-program --test canonical_artifact_program \
  complete_level12_compression_matches_pinned_bytes -- --nocapture --test-threads=1
```

The recorded fast-turnaround release invocation uses the focused manifest captured here; it does not report a libtest test count. The maintained full-program test compiles as part of the passing host test binary, but its unoptimized execution was not repeated. The three writer tests above ran through the actual maintained libtest harness. Warnings remain errors. CI is cancelled at user request, so no green CI result or cross-platform full-program coverage is claimed.
