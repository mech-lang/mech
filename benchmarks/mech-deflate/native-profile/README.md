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
