# Finite native application qualification

This example compiles the maintained, pinned complete compressor through the
public source frontend and application API. It uses the normal process stack,
shared Object emission, independent descriptor restoration, and the runtime-only
application shell. `original` and `decoded` select the artifact supplied to the
same generator. Neither uses the diagnostic unrestricted compilation mode.

From the repository root, using nightly-2026-03-03 and the locked dependencies:

```sh
cargo build --locked --offline --release -j1 -p mech-gpu \
  --no-default-features --features canonical-native-program \
  --example compression_application
target/release/examples/compression_application /tmp/compression-original \
  benchmarks/mech-deflate/public-application/compiler-policy.json original
target/release/examples/compression_application /tmp/compression-decoded \
  benchmarks/mech-deflate/public-application/compiler-policy.json decoded
```

Each destination must be new. When `CARGO_TARGET_DIR` is set, use its reported
example executable. The generator writes the exact source, bytecode, Object,
descriptor, identity, policies and nine small input/expected-output cases.

The generated project is a separate runtime-only Cargo root. For example:

```sh
cp Cargo.lock /tmp/compression-original/Cargo.lock
cargo generate-lockfile --offline --manifest-path /tmp/compression-original/Cargo.toml
cargo build --locked --offline --frozen -j1 \
  --manifest-path /tmp/compression-original/Cargo.toml \
  --target-dir /tmp/compression-runtime-target --message-format=json
python3 benchmarks/mech-deflate/public-application/qualify.py \
  /tmp/compression-original /tmp/compression-runtime-target/debug/compression-application \
  benchmarks/mech-deflate/public-application/launch-policy.json \
  /tmp/compression-original-eight --case eight-literals
```

Use Cargo's `compiler-artifact.executable` for the binary path. Build and qualify
the decoded project the same way. The documented debug runtime is for
correctness qualification; its whole-process elapsed time is not an optimized
compression benchmark. Omitting `--case` requests all nine cases, which can be
expensive. Selected cases and incomplete corpus coverage are explicit in the
receipt. The checker compares the actual canonical output to the expected raw
bytes and independently inflates that output with zlib. The application also
requires zero managed bytes/nodes after releasing inputs and outputs.

`compiler-policy.json` allows 128 MiB for one definition table, 131072 generated
control blocks, 16 MiB code and 64 KiB native stack, with 4096 inputs/outputs.
The complete program used 120170976 definition-table bytes, 100256 blocks,
2999140 emitted code bytes and 416 native stack bytes. This table allowance is
not a bound on total compiler memory. Generic defaults remain unchanged.

`launch-policy.json` is independent runtime authority: 512 MiB managed memory,
128 MiB transport, 1 MiB native stack, finite execution work, and all seven
finite library dimensions. Compilation limits cannot grant runtime authority;
both JSON formats reject unknown or missing fields. The existing numeric managed
launch arguments remain available. For resource-hosted public CLI applications,
`mech build --backend cranelift --native-compile-limits compiler-policy.json`
forwards the same compilation policy; hosted launch policy remains separate.

Fresh source/decoded generation, linked execution, dependency closure, hashes,
commands and limitations are retained in the
[public-path receipts](../results/2026-10-28-native-hardening/public-application/).
Only the eight-literal case was freshly executed in each deployed form. The
remaining eight cases, complete-program boundary inputs, repeated slides and
full corpus are separate qualification gates.
