# Warning cleanup follow-ups

This log records code or validation surfaces that could not be removed safely
during the zero-warning cleanup. No unaudited warning suppression remains; the
small compatibility and benchmark exception set is pinned in
`scripts/warning-exceptions.json`. An entry stays here only when a deeper
architectural decision is required; ordinary cleanup work does not belong in
this file.

## Protected artifacts

### Mika animation catalogs

- Location: `src/core/src/mika.rs`.
- Preserved code: all MicroMika animation frame catalogs, including their legacy
  names and exact frame contents.
- Reason: these are artifacts whose existence is the requirement. They remain
  public static data without lint suppression; callers are not required.

## Open investigations

### Versioned Rust names in the open dynamic-module ABI

- Location: `src/abi/src/lib.rs`, specifically `MechStatusV1` and
  `MechKernelKindV1`.
- Preserved behavior: both remain transparent integer newtypes, keep every v1
  numeric value, continue to admit unknown values from independently built
  dynamic modules, and expose both the original CamelCase source names and the
  warning-clean uppercase aliases.
- Compatibility issue: the CamelCase associated-constant spellings are public
  Rust source API, but Rust diagnoses them under `non_upper_case_globals`.
  Their two narrowly scoped `expect` attributes are therefore frozen ABI
  exceptions with owners, reasons, occurrence counts, and expiry conditions.
  Turning the newtypes into enums would avoid that exception, but would make
  unknown FFI discriminants invalid and remove the forward-compatible
  `MechStatusV1(99)` representation.
- Deeper question: decide whether the Rust-facing API needs an explicitly
  versioned compatibility layer distinct from the open integer wire ABI. That
  decision should be made as an ABI/API migration, after which the audited
  exception can expire.

### Canonical value-system cutover

Completed. The universal mutable value representation and its adapter directory
were deleted after production execution moved to `Value` and `ValueCell`.
Future warning cleanup must not reintroduce a compatibility shim.

### Legacy matrix NOT factory surface

- Location: `machines/logic/src/not.rs`, specifically the public `NotV` factory.
- Finding: the current source and native NOT catalogs select only the Boolean
  scalar `NotS` factory. `NotV` still exposes public fields and implements the
  runtime and semantic-compiler traits, but no in-workspace catalog or module
  instantiates it.
- Cleanup performed here: matrix type imports now belong to the binary
  AND/OR/XOR features, so a NOT-only build no longer compiles unrelated matrix
  vocabulary or emits warnings.
- Why `NotV` was not deleted: although it is not selected by the current
  catalogs, it remains a constructible public Rust type. Removing it is a
  source-compatibility decision and should be paired with a product decision:
  either restore matrix NOT to the catalog with exact linkage tests, or remove
  the public factory as an explicit legacy API break.

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
  recorded as unfinished. The restored `solve_result_overhead` benchmark has
  two audited Clippy expectations because those exact representations are the
  contracts it measures; production code has no Clippy-wide exemption.

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

### Advertised `no_std` profile

- Locations: the root `no_std` feature and `mech-core`'s function, reactive
  transaction, and symbol-table modules.
- Finding: the root facade now correctly omits the std-only
  `print_err_report` export, but an exact
  `cargo check --no-default-features --features no_std` still fails inside
  `mech-core`. The core crate unconditionally compiles reactive planning code
  that imports `std`, uses allocation-backed strings without the corresponding
  `alloc` imports, and constructs hashbrown maps through the std-only default
  hasher API. These failures are already present on the integration baseline;
  they are not a consequence of narrowing the root facade.
- Deeper question: decide whether `no_std` remains a supported product. If it
  does, give the reactive execution surface an explicit `std`/`alloc` ownership
  model and add the exact profile to CI. If it does not, remove the advertised
  feature and its partial compatibility branches as one product-contract
  change. This cleanup does not claim that broken profile as warning-clean.

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
  capability design. Until then, the dedicated
  `scripts/check-unsafe-boundaries.sh` five-field allowlist prevents the
  boundary from expanding silently.

### Historical D2 dependency lock provenance

- Location: the archived D2 fixture at commit
  `96fd051608f9d9df9eb4e9b345af7c23279c6c67`, exercised by
  `scripts/d2_historical_evidence.py`.
- Preserved evidence: each run extracts the immutable historical source,
  materializes its dependencies, then executes with `--locked --offline`; the
  generated D2 projections remain frozen and are compared with the current
  executor.
- Limitation: the archived fixture did not contain a `Cargo.lock`, so its first
  dependency materialization cannot itself use `--locked`. Attempting that
  correctly fails because Cargo is forbidden to create the missing lockfile.
- Deeper question: preserve a reviewed historical lockfile or vendor snapshot
  as part of the evidence contract. Until then, execution is deterministic
  within each materialized run, but dependency selection begins from the
  historical manifest and the available registry index.

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

### Dormant documented math operations

- Locations: `machines/math/src/logarithm/ilogb.rs`,
  `machines/math/src/trig/hypot.rs`, and `machines/math/src/trig/sincos.rs`, plus
  their user documentation.
- Finding: none of these implementation files is declared by its parent module,
  and the frozen source catalog explicitly requires all three operations to be
  absent. `ilogb` and `sincos` do not have Cargo features; `hypot` has a feature
  included in the full distribution contract, but enabling it currently adds no
  implementation or catalog entry. The documentation nevertheless describes all
  three as available operations.
- Why they were not deleted here: deletion would preserve current compiled
  behavior but make an unresolved product-contract conflict permanent. Decide
  whether each operation is supported; then either reconnect its module,
  lowering, catalog entry, and tests as a unit, or delete its implementation,
  documentation, unsafe-boundary entry, and (where compatible) feature promise
  as a unit. This cleanup does not move the frozen catalog to accommodate either
  outcome.

### Function-local definition scope metadata

- Location: `src/engine/src/intrinsics/define.rs` and the semantic compiler's
  `var/define` specialization path.
- Preserved behavior: the compiler distinguishes root-visible definitions from
  function-local definitions so bytecode execution cannot leak local symbols
  into the program root. The specializer currently receives that distinction as
  a fourth compile-time `LegacyValue::Bool`; it is not emitted as a runtime
  function input.
- Deeper question: move specialization context such as symbol visibility out of
  the legacy runtime-value argument channel. A compiler-owned specialization
  context would express this metadata directly, but changing that interface is
  broader than warning removal and must preserve the repeated-function local
  scope regression test.

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
- Restored the completed `solve_result_overhead` comparison benchmark after API
  review determined that a benchmark is a retained engineering artifact even
  when it has no production caller. Its two intentional Clippy findings are
  exact, reasoned expectations in the audited exception contract.
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
