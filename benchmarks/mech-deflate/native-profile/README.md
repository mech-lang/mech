# Focused native profiling build

Build the compression profiling tools as an ordinary consumer of the maintained
compiler and catalog. Building a `mech-gpu` example activates that package's
dev-dependencies, including `mech-stdlib/full_compiler`, even with
`--no-default-features`. That selects the entire math library.

This manifest selects five math operations (add, subtract, multiply, divide and
remainder), six comparisons, Boolean and/or, and inclusive ranges. Only the
math, comparison, logic and range machine crates are built. The other local
machine patches are needed for Cargo's resolution of optional dependencies;
they do not enable those crates. Compiler, syntax and value features retain the
previous configuration, including the Bool/U64/I64/F64 scalar controls.

From the repository root, use the same target directory on subsequent builds:

```sh
CARGO_TARGET_DIR=/tmp/mech-native-profile-target CARGO_INCREMENTAL=0 \
  cargo build --locked --release -j1 \
  --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --bins --message-format=json
```

The repository's warnings-as-errors configuration still applies. For a pinned
offline experiment, also select its toolchain and pass `--offline`. Record the
Cargo JSON output to identify the actual linked libraries; do not select an
arbitrary similarly named file from the target directory.

`native_frame_transport` uses the existing example source without a copy.
`artifact_review SOURCE ARTIFACT` compiles the full compressor source through
the ordinary catalog, requires byte-for-byte agreement with the pinned artifact,
and checks decoded re-encoding. Run it before using this profile for timing.
The timing harness, allocator, compressor source, canonical artifact and
execution boundary must remain pinned independently of this build selection.

This is a local profiling profile. The full library qualification profiles stay
available and unchanged. A first build may rebuild dependencies because Cargo's
feature graph changed; reuse this profile for subsequent iterations. Build time
is separate from compression runtime, and switching profiles must be disclosed
in before/after comparisons.

The nondefault `linked-qualification` feature adds `mech-build` and its process
supervision for the maintained linked/Object tests. It can change transitive
dependency features and rebuild compiler libraries. Run it separately, record its
Cargo artifacts, then restore the default profile before primary timing and
scalar controls. It is not part of the timing feature configuration.

`unit-qualification` additionally enables the runtime's compiler interface needed
by existing GPU unit-test helpers. It retains the selected operation catalog;
it does not activate the GPU package's full-catalog dev dependencies.

## Compile-time profiling

`compile_profile` measures parsing, the source catalog, checked source compilation,
canonical artifact construction/encoding and native Object emission separately.
The `benchmark` source is reconstructed from the maintained compressor components
with the exact two-input entry used by the complete level-12 comparison. Its
SHA-256 is checked before compilation. `fixture` selects the public application
entry, and a source filename selects another program. The output directory keeps
source, canonical bytecode, Object, metadata and the phase receipt.

```sh
CARGO_TARGET_DIR=/tmp/mech-native-profile-target CARGO_INCREMENTAL=0 \
  cargo build --locked --offline --release -j2 \
  --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --bin compile_profile

python3 benchmarks/mech-deflate/native-profile/profile_compile.py \
  /tmp/mech-native-profile-target/release/compile_profile \
  benchmark /tmp/level12-compile compact sample
```

The optional final `sample` argument records consecutive 20-second macOS CPU
samples at 5 ms intervals. Omit it for a compile without sampling. Stage logs are
stamped independently of the compiler's phase timers. Process time includes
startup, final output writes and teardown; the compile receipt ends after native
emission and before final output writes. `compact` selects the same diagnostic
Object API as the compression benchmark. `managed` selects the ordinary Object
API, while `frontend` stops after canonical encoding. These modes have different
compilation boundaries; do not pool their timings. Rust tool builds and linking
a generated host application are separate from every measured source compile.

The guard state, owner-read filter, numeric liveness solver and vendored SSA builder
have focused test targets. They
compare against the original analyses and independent read-before-definition
paths, including contradictory guards, joins, redefinition, cycles, sparse IDs,
shared-state independence, packed word boundaries, bounded dense scratch, sparse
fallback and SSA cycle-detection generations:

```sh
CARGO_TARGET_DIR=/tmp/mech-native-profile-target CARGO_INCREMENTAL=0 \
  cargo test --locked --offline --release \
  --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --test guard_state --test numeric_liveness --test ssa_builder \
  --test dimension_remap --test old_value_reads
```

The [Apple M3 compile-time report](../../../docs/design/native-compile-time-profile.md)
records the measured before/after work and exact output checks. Raw evidence is
archived under ignored `.experiments/`; only its compact receipt belongs in source Git.

The [sub-minute follow-up](../../../docs/design/native-compile-under-minute-profile.md)
records the guard/ownership and preparation optimizations, two fresh compiles of
the current qualified worker, unchanged native output and offline replay.

The [canonical parser profile](../../../docs/design/native-parser-compile-profile.md) records the grammar-preserving parser speedup and the fresh pinned C/Roc comparison.

The [parser execution-work follow-up](../../../docs/design/native-parser-work-amplification.md) records continuation/checkpoint counts and two rejected frontend schema screens.
