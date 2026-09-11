# R1–R6 integration correction report — 2026-09-10

## Disposition

**Local integration qualification is green; the candidate is ready for
exact-head CI. Preserve this implementation and keep the R6 PR branch unchanged
until that qualification passes.** The corrections
address reproduced failures at the boundaries between source schemes, concrete
factories, artifact identity, memory planning, and managed execution. The local
core/engine and safety results below are positive evidence for those boundaries.
The standard/full catalog, shipping-profile, and full bytecode checks now pass.
All eight exact native feature shards also pass. Complete CI qualification
remains pending for the final committed candidate.

This is an integration report for the complete R1–R6 tree, not an approval of an
individual phase or a proof that every source program is supported. No new
external review has been requested by this report.

## Source and evidence identity

- Repository: `mech-lang/mech`.
- Working branch: `codex/v0.4-stack-review-fixes`.
- Reviewed R6 base: `47fdb6a8ef7e7ce286553c65c1b76782200dc4e4`.
- Imported checkpoint: `131165e4a4c2d1ea56692b46658da44a8b8c7fc6`.
- The correction candidate is the commit containing this report. Resolve its
  exact SHA with `git rev-parse HEAD`; this report does not embed its own hash.
- Local qualification was completed during September 10–11, 2026, against
  the correction contents committed with this report.
- The handoff records 1,476 standard and 2,201 full generated witnesses passing
  at the imported checkpoint. The standard rerun passed all 1,476 cases during
  this continuation, and the full rerun passed all 2,201 cases.
- The prior 124-job green run, `34475259493`, qualified
  `04eda42150cbc0f1fe6f901647a5806c94e13f60`. It does not qualify either the
  imported checkpoint or the current follow-up.

The preserved architecture and earlier qualification history are documented in
`docs/design/r6-completion-status.md`. Its older counts and green runs must not
be read as current-candidate evidence.

## Concrete corrections and the invariants they restore

| Boundary | Reproduced defect | Corrected invariant |
| --- | --- | --- |
| Call versus program admission | Individually valid solves and outputs could exceed a per-call limit only after their costs were aggregated; input allocation capacities were also treated as output bytes. | `evaluate_call_memory_budget` and `evaluate_aggregate_memory_budget`, together with scoped `RuntimePlanView` constructors, distinguish operation output/work limits from aggregate memory admission. |
| Resident input versus produced output | Resident arena admission still rejected a 65,537-element external input or literal constant against the 65,536-element operation-output limit. | Per-call output checks use the value's producing node. External inputs and constants remain in aggregate memory accounting. The same rule applies during initial planning and footprint finalization; actual produced outputs retain their limits. |
| Required in-place rollback | Undo payload allocation followed the smaller prospective result rather than the larger existing mutation target. | Undo header, payload, alignment, capacity, and registration requirements derive from the aliased input that must be restored. |
| Broadcast schemes and concrete factories | Valid Boolean, comparison, arithmetic, and floating binary broadcasts lacked concrete bridges or carried exact-shape contracts. Broad elementwise schemes also admitted incompatible axes. | Shared shape construction supplies declarations, registration, exports, and implementations; semantic schemes encode the supported equal-shape, scalar, row, and column relationships. All eight floating binary families, including `atan2`, use that shared construction. |
| Existing runtime and native identities | Replacing per-shape floating implementations with generic types could rename established runtime IDs and native installer paths. | Existing same-shape signatures retain their established names, IDs, and installer spellings. Added broadcast signatures receive their own identities. Focused identity tests also exercise compilation without a source binding that could conceal reconstructed-name drift. |
| Minimal native feature builds | Shared arithmetic helpers were reachable through operation modules that are absent in narrowly selected native builds. | Managed binary/unary helpers and their feature gates are shared independently of an unrelated operation's module. Exact feature-closure qualification remains required. |
| Semantic inputs versus runtime ABI | Read/modify/write declarations include the destination as a semantic input, but the executable ABI carries it through the output port. A direct arity comparison rejected valid assignment bytecode. | Catalog validation maps the destination through the declared mutation contract before comparing executable arguments. Resident whole-value arithmetic assignment supports the maintained operation family and scalar broadcast. |
| Compiled operation identity | A maintained unary compiler reconstructed a factory name that differed from the selected runtime ID. | Compilation of a source-bound node retains the immutable ID from its `BoundCall`; the generated gate checks selected/emitted identity and memory-certificate agreement. |
| Set bytecode and schema rebinding | A set capacity was decoded as an exact current cardinality, rejecting nested subsets of different sizes; correcting that exposed dynamic-to-exact literal rebinding failures. | Decoding preserves the dynamic capacity bound. Rebinding to an exact target validates the actual payload cardinality, including a negative mismatch regression. |
| Canonical assignment publication | Named record fields were classified as positional selectors; aggregate assignment rejected the managed staging path, and temporary candidate schema identity could differ from the sink. | Named-member contracts and admitted canonical staging rebuild and rebind the candidate to the sink schema before publication. Fixed numeric selection keeps its maintained fixed-storage path. |
| Table-column Resident activation | A registered column binder received the output's `[0, 0]` placeholder rather than its declared shape. | Exact table row declarations establish `[rows, 1]` before layout and binding. Dynamic or unresolved row declarations are not silently accepted by this inference. |
| Maintained numeric semantics | Complex absolute-value/formatting declarations and the one-equation, multiple-RHS solve representation disagreed with maintained execution. | The corrections preserve the established complex result/formatting behavior and supply the valid 1×1 coefficient/row-RHS solve bridge. |

The bounded runtime inspection found the Resident input/output scope sibling
and corrected it. It found no additional concrete defect in the inspected undo
derivation, scope propagation, canonical assignment staging, set rebinding, or
bytecode identity paths. That statement is limited to the inspected paths; it
does not establish memory safety or whole-language closure by inspection.

## Validation completed during this continuation

The following local runs passed against the current correction work.
These counts record executed tests, not
the number of matching test names in source files.

| Gate | Executed result |
| --- | --- |
| Engine, `full_compiler,resident-artifact` | 429 passed: 399 library, 13 R5 planner, 17 R6 runtime. |
| Core focused integration and library coverage | 342 passed: 235 library, 23 R5 planner, 32 R6 runtime, 20 safety, 6 snapshot, 19 type catalog, 7 conversion. |
| Release safety with debug assertions disabled | 20 passed. |
| Pinned-toolchain Miri safety | 20 passed; 135.2 seconds reported. |
| Standard generated catalog closure | 3 tests passed, including all 1,476 generated witnesses. |
| Full generated catalog closure | 3 tests passed, including all 2,201 generated witnesses, with zero compilation failures. |
| Standard compiler profile | 2 passed; 1,414 runtime entries and 64 named specializers. |
| Full runtime profile | 2 passed; 9,716 runtime entries, matching the regenerated inventory. |
| Full source profile | 3 passed; 15,694 source-catalog runtime entries, 120 named specializers, and 9,717 source-enabled runtime entries. |
| Full compiler profile | 3 passed; 15,698 runtime entries and 120 named specializers; compiler catalog retains the runtime subset. |
| Floating factory identity | 3 passed in the combined float profile and 3 in the sparse `fmod,f64` profile, including public native installer invocation, preserved identities, and emitted bytecode IDs. |
| Full shipping bytecode suite | All 37 passed, including maintained numeric, selector, aggregate, target, and determinism regressions. |
| Exact native feature closure | All 8 shards passed: 34,916 inventories validated and 15 owner/named representatives compiled and executed through their public installers. |
| Architecture and CI contracts | All R1–R6/static boundaries passed; 66 Python tests passed initially, and the final 60-test CI/runner/native/catalog group passed after workflow and fingerprint updates. |
| Changed Resident planner file | Pinned rustfmt check and whitespace/diff validation passed. |

An initial engine command using `full_compiler` alone selected zero Resident
planner tests because that module requires `resident-artifact`. That run is
not counted as passing coverage. The corrected feature selection above ran
the module and includes the new Resident budget regression.

An optional additional stdlib R6 inventory invocation was canceled when it
started recompiling the large shared library. It is not counted as a pass;
the executed core and engine R6 results above remain the local R6 evidence.

The compiler remains `nightly-2026-03-03`; local Rust work is serialized with
`CARGO_BUILD_JOBS=1`. The earlier monolithic all-shapes builds in the handoff
exceeded the other machine's useful memory envelope and were stopped. Their
partial compilation is not validation evidence.

## Inventory provenance

The current frozen shipping surface was regenerated from a measured catalog,
not increased by an assumed factory delta. At the time of this draft it
contains **9,716 runtime entries**. Its file SHA-256 is
`cccc5a0bc5b06689e202504d226fe9ce02f12061e2af89ec6f3bdd36720cc397`.

The working native coverage summary contains 9,716 full entries, 121,897
extended entries, and a deterministic union of **122,083 distinct entries**.
The union's Rust-compatible `id_hex<TAB>name<LF>` digest is
`34db793ac637b3b1bc532978c6e8a6e0be1b8be63c33ec5f3043045f765616f5`.
The summary currently reports 34,916 exact feature-closure cases.

Provenance is mixed and must remain explicit: the shipping full, floating,
and logic surfaces were measured from current sources; other surfaces came
from baseline inventories whose relevant construction was verified unchanged.
The merge validates identity, signature, feature, installer, and contract
consistency. It does **not** turn inherited baseline compilation into a
current-candidate compilation result. Required Full CI must regenerate every
current shard and reproduce the same union before final qualification.

The standard compiler, full runtime, full source, and full compiler assertions passed
with measured counts and digests. The generated inventory merge also passed,
with zero missing linkage records. All eight exact native feature shards passed
against this inventory. Earlier handoff counts
such as 9,374 or 120,833 describe an intermediate surface and are not the
current denominator.

## What the generated witnesses establish

The source gate generates a finite set from enabled schemes, built-in kind
predicates, dimension relationships, export metadata, and explicit syntax
adapters. It checks Boolean/f64-compatible scalar and matrix representatives,
strict-equality syntax, table-join templates, and table-column projections
across the active scalar kinds.

For catalog-backed instructions, a successful witness establishes source
resolution, concrete catalog selection, selected/emitted runtime identity,
physical operand validation, R5 certificate agreement, canonical artifact
round trips, and required Resident binder/preflight/initial activation. The
negative controls reject missing runtime edges, altered IDs or operands, and
missing Resident edges. Missing implementations are not a rule for filtering
source witnesses out of the universe.

The generated count is not an exhaustive semantic proof:

- An operation with a witness is not necessarily covered at every overload.
  Generation-deferred and selection-deferred overloads remain visible in the
  report; closed literals can select a more specific overload instead.
- Boolean and f64 representatives do not exhaust integer widths, rational or
  complex arithmetic, structural kinds, physical layouts, or all extents.
- Read/modify/write source declarations remain excluded from the f64 witness
  generator until writable-destination syntax adapters are supplied. Separate
  focused assignment regressions do not make them generated coverage.
- Literal-derived dynamic storage does not test every reactive growth or
  dimension transition. Initial activation is not an independent numerical
  oracle or a multi-turn execution proof.
- Table joins retain the declared unsupported Resident policy, with rejection
  required at the identified join operation. Those rows are not successful
  Resident execution.
- `access/column` retains an explicit artifact-only syntax edge: its concrete
  direct-runtime factory/ABI edge is deferred. Its selected/emitted identity,
  artifact, memory certificate, and Resident activation are still checked.
- Native inventory closure proves the enumerated registration/linkage
  relationships. It does not show that every advertised semantic scheme has
  witnesses or that every registered kernel is numerically correct.

Focused semantic, atomicity, safety, resource, bytecode, and reactive tests
remain necessary alongside the generated gate.

## Work remaining before promotion

1. Commit and push the completed local correction to the review branch without
   changing its qualified source contents.
2. Validate the current full native shard set through the required Full CI
   workflow. Preserve failures as concrete correction work; do not substitute
   the inherited inventory or earlier green run for this result.
3. Record the immutable tested source SHA and exact-head CI results in the PR
   qualification record, without amending implementation merely to add a run ID.
4. Before any R6 branch promotion, verify its remote head is still
   `47fdb6a8ef7e7ce286553c65c1b76782200dc4e4` and preserve intervening work.
   Promotion depends on completed gates, not the imported checkpoint's status.

A read-only check during this continuation confirmed the R6 remote remained at
the expected head with green existing checks. All review threads were resolved;
the complete paginated result contained no open threads. No new review was requested.

The reproduced integration defects and the additional identity, sparse-feature,
and Resident budget-scope defects found during this continuation are corrected.
No further concrete implementation blocker was found in the inspected paths.
The combined architecture remains coherent under this bounded qualification;
its documented witness limits are not an exhaustive language-closure proof.
The candidate is ready for exact-head CI, not for a completed integration or
merge claim before that CI and the fresh full native union are verified.
