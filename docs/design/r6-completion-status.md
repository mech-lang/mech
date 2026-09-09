# R6 completion status

This is the qualification ledger for PR #809. The normative architecture is
`r6-memory-runtime-cutover.md`; this file records the evidence attached to the
current candidate and deliberately makes no merge claim while an exact-head
gate is pending.

## Candidate identity

- R5 base: `941fbbce44712b7f738813170ca1561a313190db`.
- Latest completed implementation checkpoint before this ledger update:
  `59a14dd1ee07f55b8dee445d4e8359364ab61688`.
- Qualification candidate: the commit containing this ledger. The PR head,
  checked-out source, and CI head must be identical before approval; use
  `git rev-parse HEAD` rather than a self-referential hash embedded in this
  commit.
- Last completed exact-head normal CI before the current correction:
  [run 34386673015](https://github.com/mech-lang/mech/actions/runs/34386673015),
  green at `f1d0256889a4afb5513525934a38794d72da7f09` with 114 checks.
- Current-candidate CI: pending at the time of this ledger update. Its final
  run and conclusion belong in the PR qualification record, not in an amended
  implementation commit.

## Catalog denominator

The inventory test enumerates runtime entries, execution capabilities, named
specializers, and intrinsic specializers. It classifies concrete maintained
implementations, not only exported names, and fails on an unclassified entry.

| Profile | Runtime entries | Capability records | Named specializers | Intrinsics |
| --- | ---: | ---: | ---: | ---: |
| `standard_compiler` | 1,330 | 1,330 | 64 | 12 |
| `full_compiler` | 15,091 | 15,091 | 120 | 13 |

The fixed-shape qualification matrix separately compiles and executes
`matrix1`, `matrix2`, `matrix3`, `matrix4`, `matrix2x3`, `matrix3x2`,
`row_vector2`, `row_vector3`, `row_vector4`, `vector2`, `vector3`, and
`vector4`. Those slices remain isolated so one macOS link does not exceed the
local object-size boundary.

## Maintained-family evidence

| Family | Managed path and evidence |
| --- | --- |
| Numeric, logic, comparison, range, statistics, combinatorics | Fixed-width managed regions; catalog inventory and scalar/range/reduction Resident suites. |
| Matrix arithmetic, assignment, transpose, product, dot, solve | Managed fixed regions and R5 scratch; dynamic transpose growth/rebinding and rollback; fixed-shape qualification matrix. |
| String scalar/matrix operations | Prospective payload witnesses, sealed canonical construction, frozen publication; gather, assignment, transpose, growth and rejection regressions. |
| Aggregate access and assignment | Borrowed selection planning and call-bound canonical construction; tuple, record, map, table, and matrix selector suites. |
| Sets, tables, concatenation, comprehensions | Canonical finalize/sort scratch and frozen publication; set admission/atomicity and table join managed-frame regressions. |
| Literals and conversions | Fixed initialization or canonical construction; one selected `ConversionPlan` governs source and artifact execution; converted String payload bounds are target-aware. |
| Snapshots and canonical cells | Shared immutable root and one physical retained-payload ticket; independent domains retain separate logical plan admission without duplicating physical ownership. |
| Definitions, captures, state, registers, checkpoints | Managed publication and existing journal authority; published-invariant isolation and state/register tests. |
| External host/resource/module bridges | Finite marshalling construction, captured-result adoption, and failure-atomic publication; module output writes directly to the unpublished candidate. |
| Resident execution | `PreparedResidentTurn`, R5 transaction targets, real arena projection; numeric and String staging no longer creates an output-sized second buffer. |
| Native, WASM, compute, GPU, browser | Target-local realization and registered submission ownership; owner suites and required browser groups are part of the exact-head gate. |

## Current correction evidence

The correction after the last green baseline closes the bounded completion
review findings as families rather than call-site exceptions:

- Retired realizations now release region, revision, reuse, and initialization
  metadata after the final realization owner disappears. Failed candidates are
  retired without invalidating the active plan; stale object keys are rejected.
- Immutable snapshot imports share the payload ticket with the frozen data
  lifetime. Independent domains still account for their own logical plan, and
  an intentional deep copy receives independent physical ownership.
- Aggregate packing, table join, matrix reconstruction, and conversions resolve
  live inputs from the existing `KernelMemoryFrame` and consume its sealed
  construction authority. They do not create nested cells/domains or discard a
  frame snapshot and reread the source.
- Dynamic Resident modules write directly into the unpublished transaction
  candidate. String gather, transpose, and matrix assignment likewise avoid an
  output-sized second staging copy.
- Turn planning supplies the distinct published and candidate footprints to the
  core derivation once. It no longer patches semantic-hash work through a
  subtract-and-add adjustment that can underflow.
- Numeric-to-String conversion witnesses include a source-kind-aware bound for
  the exact `Display` representation selected by the conversion plan, including
  subnormal floats and complex components.

Local qualification on the correction tree:

- `mech-engine` library with `full_compiler,dynamic-modules,resident-artifact`:
  410 passed, zero failed.
- `mech-engine` R6 runtime integration suite: 16 passed, zero failed.
- `mech-core` R6 runtime suite under the prescribed narrow profile: 23 passed,
  zero failed. The obsolete file-wide `full` gate was removed, so this command
  executes the tests instead of reporting a zero-test pass.
- `mech-core` R6 safety suite under the same profile: 17 passed, zero failed.
- no-std core compile, formatting, diff validation, and the permanent R6
  architecture checker: passed.
- New mutation checks reject removal of retired-metadata cleanup, shared-data
  ticket ownership, frame-bound conversion construction, and direct Resident
  candidate staging.

## Remaining qualification work

No implementation family is intentionally deferred from R6 in this ledger.
The remaining gates are operational:

1. Complete required CI on one unchanged PR head and investigate any concrete
   failure without rerunning unrelated expensive jobs.
2. Answer and resolve the bounded review threads against that same head.
3. Obtain a bounded completion review of only the correction diff. Any new
   implementation work must be tied to a concrete finding from that review.

R7 remains release qualification; it is not a fallback for unfinished R6
memory-runtime cutover work.
