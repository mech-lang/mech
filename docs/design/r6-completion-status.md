# R6 completion status

This document records migration evidence for PR #809. It is a status ledger,
not a replacement for the guarantees in `r6-memory-runtime-cutover.md`.
Unverified rows remain explicitly open.

## Candidate identity

- R5 base: `941fbbce44712b7f738813170ca1561a313190db`
- reconciled R6 head: `8fd275d19eb4d9166e648b49ce2ef99784cfc724`
- current packet: W0/W1 corrections are local and uncommitted
- latest remote normal CI: run 34278263488 on `8fd275d19`; all jobs except
  Browser Canary passed. The canary timed out after 30 seconds while evaluating
  the report-only 16,384-particle software-adapter smoke test.

No completion claim is attached to the local working tree. Exact candidate and
CI SHAs will replace this line after the packet is committed and pushed.

## Catalog denominator

The inventory test enumerates `runtime_entries()`,
`runtime_execution_capabilities()`, `specializer_entries()`, and
`intrinsic_specializer_entries()`. It classifies concrete factory
implementations, not merely exported source names, and fails on any
unclassified entry.

| Profile | Runtime entries | Capability records | Named specializers | Intrinsics | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| `standard_compiler` | 1,330 | 1,330 | 64 | 12 | classified; ordinary suite executes 9 tests on the current local tree |
| `full_compiler` | 15,091 | 15,091 | 120 | 13 | classified; ordinary suite executes 9 tests on the current local tree |

The fixed-shape qualification test also proves that the selected exact matrix
representation is installed in the runtime catalog and executes `math/add`
through the ordinary managed `FunctionInstance`. `matrix1`, `matrix2`,
`matrix3`, `matrix4`, `matrix2x3`, and `matrix3x2` have each been compiled and
executed as separate local feature slices. Combining all square representations
in one macOS link exceeded the local linker/object-size boundary, so Full CI
now isolates every fixed representation in its own bounded job. The six vector
and row-vector slices remain pending locally.

## Family inventory

Statuses distinguish catalog classification from behavioral qualification.
“Subset verified” never means the whole family is complete.

| ID | Registration and implementation owner | Storage lifecycle and shared path | Production evidence | Status |
| --- | --- | --- | --- | --- |
| F01 | Numeric, logic, comparison, range, statistics, and combinatorics machine factories; generated scalar/matrix macros and handwritten range/reduction leaves | Fixed-width managed region; range/combinatorics result geometry is prospective | `maintained_scalar_add_uses_the_ordinary_managed_function_entry`; catalog classification under standard/full | Subset verified; widths, empty/growing shapes, and handwritten leaves remain W2 |
| F02 | `machines/matrix`, math assignment factories, transpose/matmul/dot/solve | Fixed-width managed region plus R5 scratch objects; region-evidence publication | `bound_ordinary_transpose_follows_actual_cell_growth_and_rejected_update`; scratch-class catalog assertions | Relocation/rollback verified; complete scratch and algebra matrix pending W2 |
| F03 | `machines/string` scalar/matrix factories and String transpose branch | Prospective payload witness, sealed canonical build, frozen-value publication | typed String gather/assignment tests; Resident String atomicity/amplification tests; ordinary String concat grow/reject/recover | Shared access/assignment and ordinary String replanning verified locally; broader conversion/transpose rows remain W2 |
| F04 | `intrinsics/access/*`, `CanonicalAccess`, record/map/table/tuple access, swizzles | Borrowed selection count/footprint; admitted canonical build for payload results; fixed region for numeric results | `typed_string_gather_uses_prospective_managed_admission`; `map_access_uses_canonical_key_equality`; tuple/record/table payload access test; selector-family engine suite | Typed and source helper paths covered locally; complete mode/native representatives pending W2 |
| F05 | `intrinsics/assign/*` and core `ValueCell` replacement | Complete candidate witness; admitted canonical build or initialized fixed stage; one atomic publication | `typed_string_index_assignment_uses_complete_candidate_admission`; multi-cell publication and owned growth/shrink tests | Shared String/fixed paths verified; full assignment mode matrix pending W2 |
| F06 | `machines/set`, constructors, table operations, horzcat/vertcat, comprehensions | Borrowed set/aggregate bounds, canonical finalize/sort scratch, frozen publication | set admission/atomicity unit tests; catalog memory-class assertion | Catalog classified; production-path operation matrix pending W2 |
| F07 | Owned constructors, literals, conversions, artifact constants/import | Fixed initialization or admitted canonical construction; pinned storage only for explicit external `Ref` | ordinary source literal session test; explicit external registration tests | Core ingress subset verified; decoder/artifact constant matrix pending W2/W3 |
| F08 | `snapshot/*` and managed canonical cell adapter | Retained immutable root; explicit transforming rebind/deep-copy only | `ordinary_dynamic_cell_snapshots_retain_the_frozen_root_after_close`; canonical shared-root safety tests | Nested Dynamic ordinary snapshot verified locally; cross-table transform/export coverage pending W2 |
| F09 | Variable/invariant definitions, captures, state/register/checkpoint | Retained immutable or published invariant; state publication uses existing journal | `ValueSet` and `CanonicalVariableDefinition` policy checks; `maintained_set_definition_preserves_its_specialized_frozen_output_without_write_access`; `published_invariant_policy_cannot_obtain_output_write_authority`; state journal tests | Published-invariant output isolation and folded semantic planning inputs verified locally; full state/register inventory pending W2 |
| F10 | External host/resource/module/ABI adapters | Capture once, borrowed measurement, `ExternalAdoption`, atomic publish | `external_resource_adoption_replans_each_captured_result_once` | Grow/shrink, one-read, rejection, and recovery verified locally; module/ABI pending W3 |
| F11 | Resident executor, numeric/String/Snapshot lanes, real arena projection | `PreparedResidentTurn`, R5 transaction target, retained arena owner | Resident projection ownership test; Resident String/set regression suites; owner test command | Substantial existing coverage; lane-by-lane inventory and actual reuse proof pending W3 |
| F12 | Artifact/native/WASM/compute/GPU/browser adapters | Target-local realization; registered buffer/submission ownership; planned bridges | compute and GPU R6 suites; prior exact-head owner/normal CI; browser canary diagnostics | Local packet qualification and exact-head backend CI pending W3/W5 |

## Current packet evidence

Executed successfully on the uncommitted working tree descended from
`8fd275d19`:

- `mech-core --all-features --test r6_memory_runtime publication`: 3 passed.
- sibling-call atomic batch test: 1 passed.
- `mech-core --all-features` compile check.
- `mech-engine --features full_compiler` compile check.
- standard stdlib R6 suite: 9 passed, zero filtered out.
- full stdlib R6 suite: 9 passed, zero filtered out.
- full and standard catalog inventory classification: both passed with the
  denominators above.
- isolated managed fixed-shape execution for each of `matrix1`, `matrix2`,
  `matrix3`, `matrix4`, `matrix2x3`, and `matrix3x2`: 1 passed per feature.
- published-invariant false-writer regression: 1 passed; value and content
  version remained unchanged after rejected writer acquisition.
- R6 architecture checker and all 67 architecture mutations: passed.
- CI workflow/impact contract suite: 31 passed; the fixed-shape coverage set is
  enforced as twelve one-feature jobs to bound compiler, linker, and disk use.

Earlier focused tests in the same working tree established typed String
access/assignment, canonical map/tuple/record/table access, borrowed fixed
footprint measurement, shared Dynamic snapshot ownership, and external
small/large/small capture. They must be rerun as part of the packet suite after
the current edits settle.

## Remaining completion work

- Run all fixed-profile shards and correct any feature-isolation defects.
- Complete F01-F09 production representatives and scratch reconciliation (W2).
- Complete external/module/Resident/native/WASM/GPU/browser integration rows
  (W3).
- Make CI ownership enforce standard/full/fixed coverage and close the
  checker/coverage gate (W4).
- Qualify one unchanged SHA through normal CI, Full CI, release safety, and
  Miri, then obtain the bounded completion review (W5).

No item in this section is delegated to R7.
