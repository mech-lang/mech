# Poster syntax verification

This correctness check implements the two source-language additions that were
previously deferred while editing the poster:

```mech
+> math/*, logic/all
variance! := all(Σ₊[[1 5 9]] > 0)
```

`logic/all` reduces a Boolean scalar, vector or matrix to a scalar Boolean.
Empty Boolean matrices produce true. Non-Boolean arguments are rejected rather
than converted. The ordinary resident executor uses no additional scratch space.
Compute lowering uses the existing comparison and conjunction instructions,
with a balanced reduction tree and linear element-processing work.

For a 3×3 covariance matrix, linear indices 1, 5 and 9 select the diagonal.
This constraint requires all three diagonal variances to be positive. It does
not establish finiteness, symmetry or positive semidefiniteness. The archived
performance benchmarks continue to use their original three-guard contract;
this change does not replace that contract or revise recorded measurements.

## Coverage

- Module import lists, groups, aliases, comments, fenced Mechdown, malformed
  separators, and formatter round trips. Formatting currently emits one import
  per line while preserving import order and trailing comments.
- Boolean scalar, row-vector, column-vector, rectangular-matrix and empty-matrix
  reduction; rejection of numeric inputs and invalid arity.
- The compact f64 guard in an ordinary resident graph, including rejection,
  unchanged published state/epoch, and recovery on a later accepted input.
- Explicitly typed f32 guards on scalar CPU, SIMD CPU, scalar/SIMD JIT,
  scalar/SIMD AOT, Metal and WGPU. Each selected element is independently made
  false in the final lane; rejection must leave the entire published batch
  unchanged. Both scalar-broadcast directions and matrix-to-matrix comparisons
  are covered, as is artifact serialization.
- Rectangular matrix constant ordering and Boolean host-value round trips.
- Existing embedding, saved-dylib, EKF, particle, and memory-plan regressions.

## Integration fixes exposed by the tests

Generic compiler interfaces previously used a floating-point-only conversion
for Boolean values. They now use `RuntimeHostInputValue::from_value`;
`from_numeric_value` retains its numeric-only behavior.

WGPU readback now releases the completed submission's transfer scope before
opening host staging views. The old ordering could fail with `TurnInFlight`.

The elementwise GPU path also transposed matrix initializers and constants
that were already canonical row-major snapshots. The redundant transposes
were removed, retaining shape/count validation. A standalone particle test
exposed this older bug, and two small rectangular-matrix regressions cover it.
The fixed-shape EKF benchmark uses a separate lowering path.

## Reproduction

Run from the repository root on macOS with an available Metal device for the
native GPU checks. These are correctness tests, not throughput measurements.

```sh
cargo test --offline --profile kernel-bench -p mech-syntax --features full --test module_imports --test formatter --lib -j 2
cargo test --offline --profile kernel-bench -p mech-logic --lib -j 2
cargo test --offline --profile kernel-bench -p mech-core --features full --test type_system_catalog -j 2
cargo test --offline --profile kernel-bench -p mech-engine --no-default-features --features full_compiler,resident-artifact --lib resident::numeric::tests -j 2
cargo test --offline --profile kernel-bench -p mech-runtime --no-default-features --features full_compiler,compute --lib input::tests -j 2
cargo test --offline --profile kernel-bench -p mech-gpu --no-default-features --features embedding,aot,metal-native --lib --tests -j 2
cargo test --offline --profile kernel-bench -p mech-stdlib --no-default-features --features full_compiler --test profile_contracts -j 2 -- --nocapture
python3 -m unittest -v scripts/tests/test_native_linkage_profiles.py scripts/tests/test_catalog_closure.py
```

## Recorded results

Verified on September 25, 2026, on macOS 15.6.1 (aarch64), using the repository's
`nightly-2026-03-03` Rust toolchain and `kernel-bench` profile.

| Test selection | Passed |
| --- | ---: |
| Syntax library, formatter and module imports | 70 |
| Logic library | 4 |
| Core type-system catalog | 25 |
| Resident numerical kernels | 87 |
| Runtime host inputs | 12 |
| GPU library and integrations | 145 |
| Full compiler catalog contracts | 4 |
| Native-linkage and catalog-closure Python tests | 24 |
| **Total distinct tests** | **371** |

Reruns are counted once. The GPU total includes 61 library tests, 21 embedding
tests, 9 reduction tests, 21 EKF tests, 26 particle tests and 7 memory-plan/runtime
tests. The particle tests were verified in two batches: the 25-test final rerun
and the longer served-shader CPU/GPU equivalence test, which also passed.

An isolated `all,source` consumer also passed compilation without enabling matrix
storage features. A separate minimal native-link consumer emitted the expected
`logic/all` metadata and successfully invoked its native installer. The complete
all-features native-linkage inventory and every distribution profile were not
rebuilt in this check; their historical evidence is preserved and explicitly
distinguished from current catalog diagnostics.

The full compiler catalog reported 15,699 runtime factories and 121 named
specializers. Its current canonical runtime-surface SHA-256 is
`9bda6b7d138562427a3d97880c63ec2bca2d698e86cb6a282137f613689c27dc`.
