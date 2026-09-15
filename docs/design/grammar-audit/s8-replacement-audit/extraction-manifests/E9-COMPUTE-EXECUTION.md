# E9 — Compute activation and execution extraction

E9 copies twelve complete files from frozen S8B
`662d29b79df8ab05a25bbadb941a689fd5bd5aae` onto E8 `2a091a128`.
`e9-symbols.json` records exact base/frozen identities, file hashes and the new
activation file. This manifest and its verifier remain on the audit branch.

| Files | Frozen responsibility |
| --- | --- |
| `src/compute/src/{activation,ir,shape,lib}.rs` | Shared activation initializer evaluation, common elementwise evaluator, remainder/assignment admission and concatenation/shape support. |
| `src/compute/src/fixed_shape.rs` | Separate retained derived publication storage. |
| `hosts/gpu/src/{lib,batched/mod,batched/jit}.rs` | Activation consumers, CPU/SIMD/JIT/WGSL exhaustive remainder handling, publication retention/commit, concrete source layout and port names. |
| `hosts/gpu/src/memory.rs` | Reuse the pinned submission's transfer authority during staged readback, with its existing regression. |
| `hosts/gpu/tests/{particle_source,parallel_ekf}.rs` | Existing activation/layout/publication/backend tests and the two frozen changes to earlier regressions. |
| `include/browser-compute.js` | Explicit physical-plan bind-group layout so reflection cannot omit publication read bindings used by planned ping-pong groups. |

The required struct-literal and exhaustive-match closure is complete:
`FixedShapeStoragePlan` has one construction site in `batched/mod.rs`, now copied
with `publications`; `BinaryOperation::Remainder` is handled by the shared scalar
operation, SIMD evaluator, JIT import/lowering and WGSL lowering. The activation
owner is `ComputeActivationValues`, not a new independent semantic frontend.

All new Rust tests use E8 mixed/document compilation or earlier APIs. None needs
the E10 bundle/render/WASM adapters, so no owned Rust test is deferred. New test
names are enumerated in the JSON: one memory test, six particle tests, two EKF tests.
The particle test also checks shared initializer allocation, two recurrence turns,
SIMD/JIT agreement and available native GPU outputs. The publication test checks
rejected candidates preserve all published values across registered backends.

Browser bind-group construction belongs with the backend's physical publication
plan and needs no new adapter API. The actual canonical browser smoke driver
requires the later WASM/bundle adapters and remains E10 qualification. The existing
`scripts/test-browser-compute-lifecycle.mjs` is useful lifecycle regression coverage;
it does not exercise the changed pipeline-layout construction.

## Verification performed

Exact-file reconstruction, `git diff --check`, and Rust formatting checks passed.
No Cargo was run by this extraction agent. Node was unavailable on PATH and in
the usual local install locations, so neither JS syntax nor lifecycle execution
is claimed as passed.

```sh
python3 /private/tmp/mech-syntax-s8-replacement-audit/docs/design/grammar-audit/s8-replacement-audit/extraction-manifests/verify-e9.py /private/tmp/mech-syntax-s8e9-compute
git diff --check
rustup run nightly-2026-03-03 rustfmt --check --edition 2024 --config skip_children=true \
  src/compute/src/{activation,ir,shape,lib,fixed_shape}.rs \
  hosts/gpu/src/{lib,memory,batched/mod,batched/jit}.rs \
  hosts/gpu/tests/{particle_source,parallel_ekf}.rs
```

## Exact validation commands and feature gates

`hosts/gpu/Cargo.toml` has no default features and no separate SIMD flag: `wide`
and scalar/SIMD implementation compile unconditionally. `jit` enables Cranelift;
`native` enables runtime-host, wgpu, pollster and bytemuck. The two integration
targets `particle_source` and `parallel_ekf` explicitly require `native`, even when
selecting a test which only runs CPU/SIMD/JIT. Omitting that feature can silently
skip the target and must not be recorded as a passing test.

Minimal build variants, matching the package feature declarations and the native
check used by `.github/workflows/ci-full.yml`'s managed-memory-runtime job:

```sh
cargo +nightly-2026-03-03 check --locked -p mech-compute
cargo +nightly-2026-03-03 check --locked -p mech-gpu
cargo +nightly-2026-03-03 check --locked -p mech-gpu --features jit
cargo +nightly-2026-03-03 check --locked -p mech-gpu --features native,jit
```

Concrete nonzero test filters (each exact named filter selects one source-declared
test; these counts are source inspection, not a Cargo `--list` result):

```sh
# New readback test; no GPU adapter or native feature required.
cargo +nightly-2026-03-03 test --locked -p mech-gpu --lib \
  memory::tests::readback_staging_reuses_the_submission_transfer_scope -- --exact
# Existing scalar/SIMD/JIT integrity test; native enables the test target.
cargo +nightly-2026-03-03 test --locked -p mech-gpu --features native,jit \
  --test parallel_ekf checked_cpu_backends_reject_candidate_and_keep_published_estimate -- --exact
# New shared activation and recurrence conformance, including available GPU.
cargo +nightly-2026-03-03 test --locked -p mech-gpu --features native,jit \
  --test particle_source canonical_activation_initializers_are_shared_and_run_only_once -- --exact
# New derived publication acceptance/rejection across registered backends.
cargo +nightly-2026-03-03 test --locked -p mech-gpu --features native,jit \
  --test parallel_ekf canonical_derived_publications_commit_without_becoming_recurrence_state -- --exact
# Changed GPU integrity regression, including two successful turns before rejection.
cargo +nightly-2026-03-03 test --locked -p mech-gpu --features native \
  --test parallel_ekf checked_gpu_rejects_candidate_and_keeps_published_estimate -- --exact
```

Tests with native GPU execution explicitly allow unavailable adapters. A green
Rust test therefore establishes GPU execution only when the adapter path actually
runs. For the complete added surface, run `--test particle_source canonical_`
(six tests) and `--test parallel_ekf canonical_` (two tests) with `native,jit`.
The million-element activation case is included in the first group.

Existing CI memory checks remain applicable:

```sh
cargo +nightly-2026-03-03 test --locked -p mech-compute
cargo +nightly-2026-03-03 test --locked -p mech-gpu --test r5_memory_plan
cargo +nightly-2026-03-03 test --locked -p mech-gpu --test r6_memory_runtime
cargo +nightly-2026-03-03 test --locked -p mech-gpu --features jit --test r6_memory_runtime
node --check include/browser-compute.js
node scripts/test-browser-compute-lifecycle.mjs
```

These are suggested validation commands, not recorded passes. Frozen failures
remain audit obligations and are not repaired during this extraction.
