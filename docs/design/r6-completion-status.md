# R6 completion status

This is the qualification ledger for PR #809. The normative architecture is
`r6-memory-runtime-cutover.md`; this file records the evidence attached to the
current candidate and deliberately makes no merge claim while an exact-head
gate is pending.

## Candidate identity

- R5 base: `941fbbce44712b7f738813170ca1561a313190db`.
- Latest completed implementation checkpoint before this ledger update:
  `04eda42150cbc0f1fe6f901647a5806c94e13f60`.
- Qualification candidate: the commit containing this ledger. The PR head,
  checked-out source, and CI head must be identical before approval; use
  `git rev-parse HEAD` rather than a self-referential hash embedded in this
  commit.
- Last completed exact-head normal CI before the current correction:
  [run 34475259493](https://github.com/mech-lang/mech/actions/runs/34475259493),
  green at `04eda42150cbc0f1fe6f901647a5806c94e13f60` with 124 checks.
- Current-candidate CI: pending at the time of this ledger update. Its final
  run and conclusion belong in the PR qualification record, not in an amended
  implementation commit.

## Combined R1–R6 review corrections (2026-09-10)

The integration review assessed the complete R6 tree with every R1–R5 ancestor.
This correction closes all eight findings at that combined boundary:

- Shaped matrix equality outranks whole-aggregate equality without changing
  scalar, tuple, or strict-equality semantics.
- Boolean AND, OR, and XOR retain scalar, row, and column broadcasts in both
  operand orders through semantic resolution and source specialization.
- Rational power preserves its exact `r64, i32 -> r64` input signature.
- Program and turn plans retain distinct transaction header and payload
  allocations, refresh payload bytes and registration bounds, and keep scoped
  call geometry consistent with the final turn arenas.
- Reuse placement reserves the maximum capacity and alignment of the entire
  group before assigning any member's offset.
- Runtime admission independently checks temporary allocation capacities over
  their closed lifetimes before reservation, even when the supplied semantic
  demand summary underreports them.
- Fixed Resident/device audits reject missing current storage and logical
  elements while preserving deferred future capacity and variable-payload bounds.
- Retained R2 storage tests assert immutable sharing and mutation isolation, and
  all four R2 targets now run in both owner CI and the full conformance gate.
  The related Resident workspace assertion now checks reservation-only
  construction authority, and Full CI retains its nine live admission tests.

Local validation includes the complete core integration/unit corpus (522 tests),
the engine R5 planner and R6 runtime suites (13 and 16 tests), internal planner
tests (11), Resident budget/live tests (18), and full source/R4 tests (18).
The standard bytecode suite passes all 21 tests, including 22 matrix/Boolean
regression cases; the full profile passes all 24 source/bytecode/Resident cases,
including rational power. R2/R5 checker mutations, CI contract tests, R2–R6
architecture checks, warning policy, and the unsafe-boundary audit pass.
Independent review of runtime admission and the planner corrections found no
remaining concrete blocker. The new commit still requires its own CI result;
the earlier green run does not qualify these corrections.

## Catalog denominator

The following historical denominator belongs to the earlier R6 checkpoint, not
the integration follow-up. Current measured counts and pending qualification
are recorded in the [2026-09-10 integration report](../../output/reviews/r1-r6-second-review-fixes-2026-09-10.md).

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

## Earlier R6 correction evidence

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
- Raw byte initialization and mutation are now limited to explicitly untyped
  scratch/transfer objects. Planned Boolean storage remains accessible only
  through the typed Boolean codec, so safe callers cannot manufacture an
  invalid Rust `bool` representation.
- Successful cold realization promotion, failed/abandoned function candidates,
  and ordinary managed cell replacement now collect retired ownership after
  scopes and staging owners unwind. The invariant fixed-width turn remains free
  of a full-domain collection scan.
- Canonical and external-marshalling construction workspace uses the explicit
  `ReservationOnlyWorkspace` backing. It retains finite R5 authority and a
  collectible runtime record but materializes neither a host arena nor a fake
  persistent payload registry; numeric scratch and ABI bridges remain
  contiguous.
- Fixed-width conversions execute the selected `ConversionPlan` directly over
  managed input and candidate output views. Table joins allocate their row,
  match, column, and nested-value containers through the same sealed
  construction capability before cloning or finalization work begins.

Local qualification on the correction tree:

- `mech-engine` library under the owner `full_compiler` profile: 225 passed,
  zero failed.
- `mech-engine` R6 runtime integration suite: 16 passed, zero failed.
- `mech-core` all-feature library suite: 230 passed, zero failed.
- `mech-core` R6 runtime suite under the prescribed narrow profile: 24 passed,
  zero failed. The obsolete file-wide `full` gate was removed, so this command
  executes the tests instead of reporting a zero-test pass.
- `mech-core` R6 safety suite with `bool` enabled: 19 passed in normal mode and
  19 passed with release debug assertions disabled. The Boolean validity case
  also passed under pinned-toolchain Miri.
- The maintained scalar String concatenation regression completed 24 alternating
  large/small replans plus injected failure and recovery without manual
  collection; live allocation and revision metadata remained at one steady
  shape after warm-up.
- Formatting, warning policy, unsafe-boundary audit, R5 planner checker, and the
  permanent R6 architecture checker: passed.
- New mutation checks reject removal of retired-metadata cleanup, shared-data
  ticket ownership, frame-bound fixed conversion, reservation-only construction
  backing, typed-byte validity, ordinary cold-path collection, and direct
  Resident candidate staging.

## Remaining qualification work

No implementation family is intentionally deferred from R6 in this ledger.
The remaining gates are operational:

1. Complete required CI on one unchanged PR head and investigate any concrete
   failure without rerunning unrelated expensive jobs.
2. Finish the integration follow-up's generated catalog, native feature, profile,
   and bytecode qualification and answer any existing applicable review threads.
3. Preserve the current stabilization instruction: no new review loop. Promote
   the qualified follow-up only after rechecking the expected remote R6 head;
   earlier green CI does not qualify the correction candidate.

R7 remains release qualification; it is not a fallback for unfinished R6
memory-runtime cutover work.
