# Warning cleanup follow-ups

This document contains only current architectural decisions that survived the
canonical value-system cutover. Completed cleanup history belongs in Git
history and the design archive, not in this active handoff. The canonical
runtime value boundary is `Value` plus `ValueCell`; function-local visibility is
stored directly as `CanonicalVariableDefinition::root_visible`.

## R1 — Canonical contract and compatibility closure

R1 removes compatibility boundaries that still compete with canonical semantic
contracts. It does not change bytecode-v1, canonical encoding v1, stable
operation IDs, native linkage names, dynamic-module ABI v1, or program artifact
versioning.

### Opaque operation contracts and artifact fallback

- Locations: `src/core/src/operation_contract/{resolved,validation,encoding}.rs`
  and `src/engine/src/artifact/{model,validation,compiler}.rs`.
- Current state: `LegacyOpaqueOperationContract`,
  `ResolvedOperationContract::LegacyOpaque`, fallback contract attachment, and
  the ordinary-bytecode implementation-identity projection remain active.
- Required state: every executable operation supplies complete semantic
  metadata for schemas, access, aliasing, output construction, change
  detection, and external interaction. An operation ID may remain a lookup key,
  but must not substitute for semantics.

### Compiler pseudo-value-to-IR adapters

- Location: `src/engine/src/artifact/ir.rs`.
- Current state: public parser/container conversion helpers construct
  `ExpressionIR` and `MatrixLiteralIR`.
- Required state: retain and rename legitimate parser-to-IR boundaries; replace
  any helper that reconstructs semantics from a heterogeneous runtime
  container; delete migration-test-only exports.

### Resident resource-request compatibility bridge

- Location: `src/runtime/src/resource.rs` and active providers under `hosts/`.
- Current state: providers accept `RuntimeResourceWriteRequest`, while resident
  coordination constructs `RuntimeResidentResourceWriteRequest` and converts
  it to the older request.
- Required state: providers accept the resident request carrying effect
  identity, idempotency, and transaction metadata; the resident-to-old-request
  conversion is deleted.

### Matrix NOT product-surface decision

- Location: `machines/logic/src/not.rs`.
- Current state: public matrix `NotV` factories exist, but source and native
  catalogs expose only scalar Boolean NOT.
- Required state: either catalog, link, test, and document matrix NOT completely,
  or remove its implementation, features, catalog promises, documentation, and
  public factory together.

### Assignment contract metadata

- Location: `src/engine/src/intrinsics/assign/catalog.rs`.
- Current state: generated assignment families use fallback validation after
  the ineffective factory-name dispatch was removed.
- Required state: declarations carry explicit axis and shape-validator metadata
  for linear, row-only, column-only, and no-index families. Do not restore
  factory-name string matching.

### Set contract metadata

- Location: `machines/set/src/catalog.rs`.
- Current state: set declarations use fallback validation after ineffective
  macro-fragment dispatch was removed.
- Required state: declarations carry explicit validator metadata, with
  end-to-end catalog tests for element and output schema mismatches.

### Cataloged compiler `todo!()` paths

- Locations: `machines/math/src/{arithmetic,bessel,trig}` and the compiler-facing
  paths cataloged in the R1 starting audit.
- Current state: several advertised factory implementations terminate in
  `todo!()` when semantic lowering is requested.
- Required state: each advertised operation has a cataloged, linked, tested
  lowering, or its compiler implementation and every product promise are
  removed together.

### Dormant or falsely documented math operations

- Locations: `machines/math/src/logarithm/ilogb.rs`,
  `machines/math/src/trig/{hypot,sincos}.rs`, their Cargo features, catalogs,
  native linkage declarations, and user documentation.
- Current state: implementation files and product claims disagree; `hypot` is a
  no-op advertised feature, while `ilogb` and `sincos` are documented but not
  active catalog entries.
- Required state: each operation is fully implemented, cataloged, linked,
  tested, and documented, or absent from all five surfaces.

### `no_std` support-or-removal decision

- Locations: the root `no_std` feature and `mech-core`'s function, reactive
  transaction, and symbol-table modules.
- Current state: the advertised exact profile does not build; the R0 audit
  reports 24 compiler errors from unconditional `std` imports, missing `alloc`
  traits, and std-only default hasher construction.
- Required state: either make the exact profile build in CI with explicit
  `std`/`alloc` ownership, or remove the root feature and partial conditional
  branches as one product-contract change.

## R2/R6 — Type–memory boundary and memory runtime

### Storage and allocation abstraction

- Locations: `src/core/src/{types,cell_binding}.rs` and resident arena/storage
  implementations in `src/engine/src/resident/`.
- R2 owns the semantic boundary between type information and storage choice.
  R6 replaces allocation backing behind `ValueCell` without changing canonical
  value semantics.

### Public physical identity or pointer exposure

- Location: public `Ref<T>` pointer/address methods in
  `src/core/src/types/mod.rs` and their execution consumers.
- R2 decides which stable logical identities are public contracts. R6 removes
  physical-address authority from public reasoning and confines any necessary
  unsafe access to implementation-owned storage boundaries.

### Resident publication capability design

- Locations: `ResidentExternalPublicationAuthority` in
  `src/engine/src/resident/general/mod.rs` and its sole runtime implementation
  in `src/runtime/src/runtime/program/external/coordinator.rs`.
- R2 defines an explicit, non-forgeable publication capability. R6 replaces the
  current cross-crate unsafe marker without broadening publication authority.

### Memory accounting and transaction storage

- Locations: `RuntimeLimits::max_memory_bytes`, transaction journals/snapshots,
  and cell backing.
- Current state: the memory limit is advisory and transaction rollback retains
  snapshot-oriented storage.
- R2 defines the accounting and ownership contract; R6 integrates allocation
  accounting and replaces transaction storage behind canonical cells. R5's
  planner must consume, not redefine, that contract.

## R7 — Release qualification

### Truthful feature and distribution surface

- Reconcile mutually exclusive additive Cargo selectors, no-op feature names,
  and the supported standard/full/browser/native product matrix.
- Every shipped profile must have an exact build and test owner; unavailable
  combinations must not be advertised as products.

### Documentation and packaging reconciliation

- Align reference documentation, examples, package contents, release metadata,
  and the root package version with the product surface qualified by R1–R6.
- The root package remains `0.3.6` until this phase qualifies `0.4.0`.

### Release-facing platform support

- Record and validate the supported Linux, macOS, Windows, browser/WASM, and
  native-module surfaces. Remove claims for platforms or profiles without an
  owned release gate.

## Post-v0.4 ownership

### Dynamic-module ABI v2

- Location: `src/abi/src/lib.rs`, including the versioned Rust spellings for
  `MechStatusV1` and `MechKernelKindV1`.
- Preserve dynamic-module ABI v1 until a separately versioned ABI/API successor
  is designed and shipped.

### Clippy-wide `MechError` representation

- Location: `src/core/src/error.rs` and the public `MResult<T>` alias.
- Choose and benchmark a compact error representation before changing public
  layout or allocation policy; do not hide the decision behind broad lint
  suppression.

### Historical D2 dependency provenance

- Location: the archived D2 fixture consumed by
  `scripts/d2_historical_evidence.py`.
- Preserve a reviewed historical lockfile or vendor snapshot if stronger
  first-materialization provenance is required.

### Structured secondary-error telemetry

- Locations: runtime transaction failure handling and destructor/callback
  cleanup paths.
- Add a structured diagnostic sink that retains cleanup and audit failures
  without replacing the authoritative primary error or panicking in teardown.
