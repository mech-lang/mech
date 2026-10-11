# Paper numerical backends

This harness runs the existing single-worker scalar/SIMD instruction evaluators,
scalar/SIMD JIT, and SIMD AOT APIs
from Mech commit `0d85319d60c460cdecb81e03c181d62ecfe1ef39`. It does not modify
the backend or switch the working repository's branch.

`prepare.py` extracts the dependency sources into a new temporary directory and
creates a Cargo manifest from `Cargo.toml.in`. It prints the generated manifest
path and writes `build-provenance.json`. Build that manifest in release mode:

```sh
python3 benchmarks/numpy-ekf/paper-driver/prepare.py
RUSTFLAGS='-D warnings -A dead_code' cargo build --release --offline \
  --manifest-path /path/printed/by/prepare/build/Cargo.toml
```

The narrow feature profile includes the numeric types, matrix operations, and
source compiler required by this EKF; it does not select unrelated standard
library operations. Release settings are optimization level 3, one codegen unit,
and thin LTO. The generated Cargo lockfile belongs with retained build evidence.

Run the resulting `numpy-ekf-paper-driver` with `--backend scalar`, `simd`, `jit`,
`jit-simd`, or `aot-simd`, and the common `--instances`, `--turns`, `--warmup`,
`--input`, and optional `--dump` arguments. The existing compiled SIMD APIs
require an instance count divisible by four; the harness rejects other counts
for those backends rather than padding the workload. All five primary Mech
lanes therefore use the same backend revision and numerical source.

The source is the shared `../mech-driver/ekf-numerical.mec`, without integrity
predicates or their supporting arithmetic. Input files contain three contiguous
little-endian f32 arrays: velocity, angular velocity, and bearing. Output files
contain C-order `state[N,3]`, then `covariance[N,3,3]`.

Compilation, SIMD packing, preparation, warmup, and recreation of initial state
are untimed. The timer covers the dependent turns plus final host-readable state
materialization. SIMD JIT defers its unpack until `state()`; SIMD AOT includes it
in `dispatch_turns`. Both costs are included. Canonical dump conversion, file
I/O, checksums, and JSON reporting are outside the timer. No parallel-worker API
is called.
