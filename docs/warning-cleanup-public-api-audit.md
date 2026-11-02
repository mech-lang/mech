# Warning cleanup public API audit

This audit compares `integration/value-executor-v0.4` at
`6d4c5e37f1e583f48cb43dc28cdae237f1de6b5e` with the revised PR #780 worktree.
It was generated with `cargo-public-api 0.52.0` and
`nightly-2026-03-03`. ABI, bytecode, core, build, runtime, compute, and engine
were inspected with all features enabled. Syntax and the root facade were
inspected with their default supported profiles because their historical
literal all-feature unions combine `no_std` with standard-library-only
features and are not valid configurations.

The raw line counts include repeated re-export routes, generated
implementations, and canonical type-path changes. They therefore describe the
size of the compiler-visible diff, not a count of source-breaking names.

| Crate | Base lines | Revised lines | Raw removed | Raw added | Removed public identifiers |
| --- | ---: | ---: | ---: | ---: | ---: |
| `mech-abi` | 47 | 59 | 0 | 12 | 0 |
| `mech-bytecode` | 8 | 8 | 0 | 0 | 0 |
| `mech-core` | 20,347 | 19,924 | 1,246 | 823 | 40 |
| `mech-build` | 766 | 766 | 0 | 0 | 0 |
| `mech-runtime` | 13,099 | 12,814 | 289 | 4 | 0 |
| `mech-compute` | 454 | 454 | 0 | 0 | 0 |
| `mech-engine` | 8,280 | 7,229 | 1,137 | 86 | 376 |
| `mech-syntax` | 1,016 | 1,016 | 2 | 2 | 0 |
| `mech` | 170 | 183 | 3 | 16 | 0 |

## Removal classification

### Preserved through another route

- 38 of the 40 `mech-core` identifier removals are re-export-path changes.
  Matrix types and errors remain under `mech_core::matrix` or
  `mech_core::structures`; `Kind` remains under `mech_core::kind`; and
  `CompileConst` remains through the canonical compiler and root re-exports.
  The removed paths were duplicated glob projections such as
  `mech_core::program::compiler::constants::CompileConst`.
- 369 of the 376 `mech-engine` identifier removals are re-export-path changes.
  Intrinsic implementation types remain under their canonical
  `mech_engine::intrinsics::{access,assign,convert,...}` modules and, where part
  of the supported facade, at the crate root. Catalog installers remain at
  `mech_engine::function::*` and the crate root.
- `mech-runtime` has no removed named item. Its raw changes are caused by the
  canonical core and engine paths above. It adds
  `InMemorySourceResolver::try_with_source`.
- The root facade replaces three glob-projection markers with sixteen explicit
  syntax exports. No named root item was removed.
- `mech-syntax::label_with_recovery` has the same name and callable shape; the
  output now makes the existing input lifetime explicit.

These paths are still source changes for callers that named a removed duplicate
route. They are retained here as deliberate facade consolidation, not described
as deletion of the underlying capability.

### Deliberately migrated

- `LegacyValue::from_kind` was an unconditional `todo!()` and
  `MechRecord::from_kind` called it for every field. Both public functions were
  unused, could not successfully construct a value, and were retired as broken
  legacy stubs. Supported construction uses explicit values or the schema/value
  migration boundary.
- Legacy `CompileConst` implementations and their duplicated nested projections
  were reduced as part of the semantic compiler's canonical bytecode encoding.
  The supported `CompileConst` trait routes remain.
- `VariableDefineMatrix::id` was replaced by `root_visible`. The old field held
  a recomputable name hash; the new field is required to distinguish root
  symbols from function-local bytecode symbols. This is an intentional compiler
  contract migration, not a warning-only edit.

### Public in Rust, but private/internal in contract

- `Frame`, `FrameState`, and `Stack` appeared twice in the extracted API through
  root and `function` re-exports. Their fields were private, they had no public
  constructors or methods, and no external caller could construct or inspect
  them. They represented a dormant interpreter call-stack sketch and were
  removed.
- Generated intrinsic implementation structs remain public where macros and
  native factory registration require public visibility. Their fields and
  compiler-only implementations are not supported downstream construction
  APIs; the catalog/factory interfaces are the supported boundary.

### Accidental removals found by review

There are no known accidental removals remaining. Review found and this
revision restores:

- every original `MechStatusV1` and `MechKernelKindV1` source name as a frozen
  compatibility alias;
- `CopyMat::copy_into_r -> usize`; and
- the completed `solve_result_overhead` benchmark.

## Downstream compatibility qualification

- `tests/fixtures/dynamic-status-module` is an independent Cargo workspace and
  compiles using the original V1 ABI names.
- `tests/public_api_compat.rs` compiles and executes the historical
  `CopyMat::copy_into_r -> usize` call shape from an integration crate.
- The same integration test proves the established infallible resolver builder
  does not panic on invalid caller data and that the new fallible builder
  returns the validation error.
- The repository's bytecode, native-build-owner, native-live-host, dynamic
  module, Windows generated-application, browser, and CLI fixture builds remain
  part of CI qualification.

The extractor outputs are intentionally not committed: the full lists contain
tens of thousands of repeated generated implementation lines. The table and
classification above are derived from those complete outputs, not from a
source-text grep.
