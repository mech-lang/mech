# Generated source catalog closure gate

The native linkage inventory proves properties of factories that are already
registered. It cannot detect an advertised source overload whose factory is
absent, or exercise the compiler that chooses and emits a runtime ID. The
catalog closure gate adds a finite source-to-execution check alongside that
inventory, using the existing `FunctionCatalog`, source schemes, signature
features, compiler certificates, and resident preflight APIs. It changes no
runtime registration API or production execution path.

Run either exact shipping profile:

```sh
python3 scripts/check-native-linkage-coverage.py catalog-closure standard
python3 scripts/check-native-linkage-coverage.py catalog-closure full
```

The runner executes `tests/catalog_closure.rs` with the pinned toolchain and
locked dependencies. It requires a fresh, nonempty, profile-matching report
at `target/native-linkage/catalog-closure-{profile}.json`. Full CI runs both
profiles as required jobs and retains these generated reports. The reports
are evidence from the current catalog, not a second frozen factory list.

## First-phase witness universe

* Every enabled fixed source overload whose input and output kinds can be
  instantiated entirely as Booleans or Boolean matrices. This includes
  equatable/keyable kind parameters, so equality and strict-equality families
  cannot disappear behind an otherwise generic declaration. Arguments are
  generated from each `KindScheme`, respecting declared kind predicates,
  dimension bounds, and constraints. The finite shape basis
  includes row, column, square, rectangular, and extents beyond fixed-size
  matrix storage. Broadcasting
  witnesses use either live matrix dimension. Generation never queries whether
  the operation has runtime factories: removing all matrix `logic/not` factories
  therefore leaves its source-derived matrix witnesses in the gate.
  Internal strict-equality declarations use their public `===` and `!==`
  operator spellings through an explicit syntax adapter; internal canonical
  names are never mistaken for callable source exports.
* Every externally exposed, value-call compatible declaration also uses an
  `f64` representative derived from its schemes. This exercises numeric
  equality, arithmetic, ranges, Prelude functions, and module exports across
  the same finite scalar and broadcast shape basis without maintaining an
  operation-to-factory table. Module-only witnesses derive their required
  import from `FunctionExport`. Read/modify/write declarations remain deferred
  until their assignment-syntax adapter can supply a writable destination.
* Every enabled `SourceSchemeTemplate::TableJoin` operation, discovered from the
  source catalog. One structural table input recipe instantiates each template;
  no join operation names or factory IDs are enumerated in the generator.
* Table column projection for each active built-in scalar representation.
  Scalars come from `BuiltinScalarKind::ALL`, restricted by signature-derived
  representation features across the entire catalog. The single syntax adapter
  uses a table field selection, and requires the compiler to select
  `access/column`. Missing column factories or binders do not filter witnesses.
  This syntax-directed operation currently emits `TableAccessColumn` without a
  registered concrete runtime factory. Its report rows explicitly use
  `role: artifact_only_syntax` and `factory: null`: that concrete-factory edge
  remains deferred, while selected/emitted identity, certificate agreement,
  semantic artifact identity, resident binder closure, and activation are required.
  The adapter validates the catalog-backed instruction prefix before the final
  artifact-only instruction; that final instruction's direct-runtime ABI remains
  unqualified. This allowance requires the same operation, `SyntaxDirected`
  origin, and `DirectRuntime` compiler target that production permits. It cannot
  exempt a source-scheme operation or an unrelated missing helper factory.

For each witness the gate requires source compilation to succeed. For
catalog-backed source instructions it checks the selected `BoundCall`, exact
catalog entry, physical signature, operation identity, execution target, and
agreement with the R5 memory certificate's bound call. Every generated
operation with representative overloads must actually select at least one of
them. Closed literals can always prefer a more specific declaration over a
general dimension-variable overload; those visible but shadowed declarations
are recorded under `selection_deferred`. Runtime IDs and factory names in the report come from this
compilation and catalog lookup. The existing catalog-aware bytecode validator
also checks the emitted instruction stream's register seeds and argument/output
contracts; resident artifact activation alone would not validate those operands.
Auxiliary literal-construction calls can have no source certificate when the
compiler's `instruction_type_binding_requirements` explicitly permits it. The
report marks these as `compiler_helper`; their exact catalog factory, direct
target, and emitted runtime contracts are still checked. A witness's claimed
semantic operation must always occur in a source-bound instruction.

Bytecode is decoded, canonically re-encoded, and decoded again, preserving the
source artifact's semantic revision at both decode boundaries. Supported
witnesses require a concrete resident case for every artifact node, matching
node/operation identity, registered binder, and successful resident activation.
Table joins have an explicit first-phase `ResidentCpu` unsupported policy:
their rejection must identify the join node and the same semantic operation,
with a missing-binder reason. An unrelated or plan-level rejection cannot pass
as the expected join outcome. Adding join resident support requires qualifying
it and changing this template-level policy.

The small negative suite verifies that source witnesses survive absent runtime
registration, and that the checks reject missing factories, altered emitted
runtime IDs, corrupt register operands, and missing resident binders. Python
contracts check both profile commands, stale/malformed/empty/partial reports,
command failures, and required CI wiring. Source compilation failures are
accumulated in the report and always fail the test and runner.

## Limits and extension

This is a bounded first phase, not proof of closure for all source programs or
all runtime factories. The report lists deferred source operations and, for
partially covered declarations, deferred overload IDs. An operation appearing
under `covered_operations` means at least one witness exists, not that every
overload is covered. Numeric kinds beyond `f64`, other structural templates,
general syntax-directed intrinsics, host operations, every physical storage
variant, and every reactive shape transition remain outside this generator.
Its literal-derived dimensions exercise dynamic storage, not turn-varying shapes.
The shape basis does not exhaust all extents or matrix representations. The
gate exercises binding and initial activation; it does not provide independent
numerical or reactive-update oracles.

Extend the finite input adapters using scheme/template or representation
metadata, and record their deferred universe explicitly. Keep exact native
linkage feature-closure checks and focused semantic regressions: neither is
replaced by this source closure gate. Avoid adding a handwritten per-factory
expectation list or an inferred “unsupported” exemption for missing binders.

The exhaustive all-features runtime catalog is checked without building one
monolithic crate on a memory-constrained runner. Native linkage CI builds the
shipping full surface and scalar/subsystem shards independently, then forms
their deterministic union. The merge records that union's exact count and the
same `id_hex<TAB>name<LF>` digest used by the Rust stdlib profile test, and it
rejects any mismatch with the constants in `profile_contracts.rs`. The large
profile fingerprint therefore comes from current factory declarations instead
of a manually copied count or digest.
