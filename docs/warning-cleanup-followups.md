# Warning cleanup follow-ups

This log records code or validation surfaces that could not be removed safely
during the zero-warning cleanup. No warning suppression remains. An entry stays
here only when a deeper architectural decision is required; ordinary cleanup
work does not belong in this file.

## Protected artifacts

### Mika animation catalogs

- Location: `src/core/src/mika.rs`.
- Preserved code: all MicroMika animation frame catalogs, including their legacy
  names and exact frame contents.
- Reason: these are artifacts whose existence is the requirement. They remain
  public static data without lint suppression; callers are not required.

## Open investigations

### `LegacyValue` execution-model cutover

- Locations: the type currently lives in `src/core/src/value.rs` (published as
  `legacy_value`), with conversion boundaries in `src/core/src/legacy_adapter`.
- Current dependency surface: `LegacyValue` appears in 336 Rust files with
  10,199 references; its companion `ValueKind` appears in 88 files with 1,805
  references. It is still the active representation for function arguments,
  reactive cells, collections, engine evaluation, machine kernels, and runtime
  host/resource boundaries.
- Why it is named legacy: commit `4181cfc39` deliberately renamed the old
  mutable runtime `Value` to `LegacyValue` when the immutable, schema-validated
  snapshot `Value` was introduced. The accompanying comment says the physical
  `value.rs` path remains stable during migration and is deleted at final
  cutover. The name marks an active migration boundary; it does not mean the
  type is currently dead.
- Why it cannot be deleted in this cleanup: the newer snapshot `Value` is a
  detached/durable representation, not yet a replacement for mutable reactive
  execution values. Deleting or merely renaming `LegacyValue` would leave the
  execution model without its shared-cell and matrix/collection behavior.
- Narrower legacy surface: the adapter module has only two production consumers
  outside core (`engine/artifact/compiler.rs` and
  `runtime/program/external/value_adapter.rs`), but those are the current
  compiler and external-publication cutover boundaries. Once both consume the
  new value model directly, the adapter module and its contract tests can be
  removed as one unit.

### Clippy-wide `MechError` representation decision

- Location: `src/core/src/error.rs` and the public `MResult<T>` alias.
- Finding: a strict default-workspace Clippy run reports 649 diagnostics in
  `mech-core`; 498 are `result_large_err` because `MechError` is at least 176
  bytes. The remaining diagnostics are Clippy API/style/design lints rather
  than rustc or rustdoc build warnings.
- Why it was not changed here: shrinking the error requires boxing its public
  payload or changing every `MResult<T>` error to `Box<MechError>`. `MechError`
  exposes source ranges, annotations, tokens, compiler locations, source
  chains, and message fields publicly, so either route is a public layout/API
  and allocation-policy decision spanning essentially every crate.
- Deeper question: choose a compact error ABI (most likely one boxed private
  payload while preserving field access through methods), benchmark error-heavy
  compiler paths, then address the remaining Clippy design lints without adding
  lint exemptions. Rust compiler and rustdoc warning builds are clean today;
  `cargo clippy --workspace --all-targets -- -D warnings` is intentionally
  recorded as unfinished rather than suppressed.

### Mutually exclusive root feature profiles

- Location: root `Cargo.toml` distribution and platform feature selectors.
- Finding: literal root `--all-features` requests `distribution-standard`,
  `distribution-full`, hosted targets, and `no_std` together. Those selectors
  are intentionally incompatible, so that command is not a meaningful build
  product.
- Current verification: the maximal compatible root product enables every root
  feature except `default`, `distribution-standard`, and `no_std`; minimal,
  default, maximal-compatible, runtime-all-feature, and workspace-all-target
  products are warning-clean.
- Deeper question: model distribution and platform profiles outside additive
  Cargo features if a literal `cargo check --all-features` contract is desired.

### Cross-crate resident publication authority

- Location: `src/runtime/src/runtime/program/external/coordinator.rs`
- Preserved code: the sole unsafe implementation of
  `ResidentExternalPublicationAuthority`. It now has no lint exemption; the
  warning-policy contract pins this as the runtime crate's only unsafe boundary.
- Why it cannot simply be deleted: `mech-engine` deliberately makes this
  cross-crate marker trait unsafe so arbitrary safe consumers cannot authorize
  publication of an externally coordinated resident turn. Rust has no friend
  crate visibility with which to express that relationship directly.
- Deeper question: replace the unsafe cross-crate marker with an equally narrow
  capability design. Until then, `scripts/check-warning-policy.py` prevents the
  boundary from expanding silently.

### Legacy operation-contract and artifact projections

- Locations: `src/core/src/operation_contract/resolved.rs` and
  `src/engine/src/artifact/{compiler,model}.rs`.
- Preserved code: `LegacyOpaqueOperationContract`, the fallback contract
  attachment pass, and the ordinary-bytecode implementation-identity
  projection.
- Why it cannot be deleted yet: functions without a semantic operation
  declaration still need a closed, schema-bearing artifact contract, and the
  implementation-identity projection preserves the byte-for-byte encoding of
  existing v1 artifacts. Removing these requires complete semantic contracts
  for the remaining factories plus an explicit artifact-format compatibility
  decision.

### Compiler legacy-to-IR adapters

- Location: `src/engine/src/artifact/ir.rs`.
- Preserved code: the public conversions from parser pseudo-values and the
  heterogeneous matrix container into `ExpressionIR` and `MatrixLiteralIR`.
- Why it cannot be deleted yet: these are the implemented C3 boundaries for
  `index-all-selection-ir` and `matrix-construction-ir` in the frozen value
  migration contract. Their current in-workspace callers are contract tests,
  but deleting the exported boundary would reverse a recorded migration step.

### Resident resource-provider compatibility request

- Location: `src/runtime/src/resource.rs`.
- Preserved code: conversion of `RuntimeResidentResourceWriteRequest` to the
  older `RuntimeResourceWriteRequest` accepted by active provider
  implementations.
- Why it cannot be deleted yet: resident external coordination currently adds
  effect identity and idempotency metadata, then delegates provider preparation
  through the established request trait. Removing the conversion requires
  migrating every resource provider to the resident request contract first.

### Stable-assignment contract validator dispatch

- Location: `src/engine/src/intrinsics/assign/catalog.rs`.
- Finding: the old `assign_contract_validator!` dispatch matched unsuffixed
  family names such as `Assign1D`, but the factory traversal passed concrete
  suffixed names such as `Assign1DS`. Every specialized arm was therefore dead;
  generated slice-assignment factories have actually used the two-axis fallback.
- Cleanup decision: removed the unreachable dispatch arms, their unused helper
  functions, and the isolated helper test while preserving the factory metadata
  behavior that shipped before this cleanup.
- Deeper question: redesign the traversal so axis semantics are explicit data
  attached to each generated factory, then restore correct linear, row-only,
  column-only, and no-index validation with end-to-end catalog tests.

### Set contract validator dispatch

- Location: `machines/set/src/catalog.rs`.
- Finding: `declare_set_runtime_factory!` captured each factory name as a
  `literal` fragment and forwarded it to `set_runtime_contract!`. Rust macro
  fragments are opaque when forwarded, so none of the literal-specific arms
  could match; every set factory shipped with the final `no_matrix` fallback.
- Cleanup decision: removed the unreachable specialized arms, validator helper
  stack, Cartesian-product validator, and tests that called those helpers
  directly. This preserves the runtime catalog behavior that was actually
  installed while removing misleading dead validation code.
- Deeper question: make the desired schema-validator callback explicit data in
  `for_each_set_runtime_factory!`, then add end-to-end catalog tests proving the
  installed contracts reject mismatched element and output schemas before any
  validator is restored.

### Unimplemented binary math compiler lowering

- Locations: the two-argument `copysign`, `fdim`, `fmod`, `nextafter`,
  `remainder`, `jn`, `yn`, and dormant `hypot` factory generators in
  `machines/math/src`.
- Preserved code: their `MechFunctionCompiler` implementations still terminate
  with `todo!()` when semantic compiler lowering is requested. The unused
  context binding is now an anonymous parameter pattern rather than a disguised
  unused variable.
- Why it cannot be deleted blindly: removing the implementations changes which
  runtime factory types satisfy the compiler-facing trait, while inventing
  lowering here would require bytecode operation IDs and execution kernels that
  this warning cleanup cannot infer safely. These operations need an explicit
  support decision: implement and test their lowering, or feature-gate the
  compiler trait implementation and catalog metadata out together.

### Best-effort cleanup and failure-path telemetry

- Locations: runtime transaction failure handling in
  `src/runtime/src/runtime/transaction/{commit,operation}.rs`; destructor and
  callback cleanup in runtime/host/CLI test support.
- Preserved behavior: some audit-event writes occur while another commit,
  compensation, or abort error is already authoritative, and destructors cannot
  return cleanup errors. These outcomes are now discarded explicitly with
  `drop(...)` or handled as an already-stopped channel state; none use
  `let _ = ...`.
- Why it cannot be removed blindly: making secondary telemetry failures replace
  the primary transaction error would lose the causal failure, while making
  destructors panic would turn recoverable teardown into process failure.
- Deeper question: introduce a structured multi-error/diagnostic sink for
  cleanup and audit failures so every secondary error remains observable without
  changing the primary `MResult` contract.

## Removed legacy code

- Removed the unused interpreter frame/stack/checkpoint subsystem; no frame was
  ever constructed or pushed by a production caller.
- Removed the shadowed rational-to-f64 conversion and the unreachable specialized
  stable-assignment validator dispatch while preserving the behavior that was
  actually selected.
- Removed duplicate direct-registration implementations from the stats and range
  machines, plus math's legacy `atan2` installer and roughly 500 lines of old
  assignment registration macros, after confirming their generated native
  declaration traversals own the active runtime catalogs.
- Removed the dormant math exponential source tree. Its nonexistent
  `exponential` umbrella feature meant it could never compile, and the frozen
  catalog contract explicitly requires `exp`, `exp2`, `exp10`, and `expm1` to
  remain absent. The existing public Cargo feature names remain no-op as they
  were before this cleanup.
- Removed the completed `solve_result_overhead` comparison benchmark. It existed
  only to compare an old void dispatch, the current result dispatch, and a typed
  split experiment, and required two Clippy suppressions to retain intentionally
  lint-triggering representations.
- Replaced dependency-feature-sensitive catch-all matches in integrity, source
  indexing, and config compilation with feature-stable control flow. Their three
  `unreachable_patterns` suppressions are gone.
- Removed the stale `matrix_multiply` example, which targeted deleted APIs and
  had no valid feature/product route.
- Replaced the broad root crate override table with explicit local dependency
  paths. Only the nine machine crates remain patched because each deliberately
  owns a standalone workspace; this also removes Cargo's unused-patch warnings
  from generated compile-fail crates.

## Baseline observations

- Baseline branch: `origin/integration/value-executor-v0.4` was initially
  inspected at `c53e92156`; the cleanup branch was then fast-forwarded to the
  merged upstream head `6d4c5e37f`.
- The first `cargo check --workspace --all-targets` exposed approximately 982
  `mech-engine` library warnings and 292 `mech-syntax` library warnings before
  duplicate test-target diagnostics were removed.
- The same check initially failed because
  `src/engine/tests/resident_ekf_program_execution.rs` did not copy the artifact's
  `compute_regions` into a reconstructed `ProgramArtifactDraft`.
