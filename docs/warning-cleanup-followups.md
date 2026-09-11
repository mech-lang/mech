# Warning cleanup follow-ups

This document contains only current architectural decisions that survived the
canonical value-system cutover. Completed cleanup history belongs in Git
history and the design archive, not in this active handoff. The canonical
runtime value boundary is `Value` plus `ValueCell`; function-local visibility is
stored directly as `CanonicalVariableDefinition::root_visible`.

## R1 — Canonical contract and compatibility closure

R1 is closed by the permanent `scripts/check-r1-compatibility-closure.py`
contract and the same-head Full CI merge gate. Executable artifacts require
declared semantics; resident lookup uses canonical operation identities; the
resource-write command is bound once to effect identity and idempotency before
provider preparation; assignment and set validators are explicit declaration
data; and the exact root `no_std` profile is an owned CI product.

Matrix Boolean NOT is scalar-only. The unavailable math operations removed by
R1 are absent from implementation, features, catalogs, documentation, and
public API. Bytecode-v1, canonical encoding v1, stable operation IDs, native
linkage names, dynamic-module ABI v1, and program artifact versioning remain
intentional compatibility contracts.

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
