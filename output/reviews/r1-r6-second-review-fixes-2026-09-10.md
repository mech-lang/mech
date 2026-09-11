# R1–R6 integration correction report — 2026-09-10

## Disposition

**The implementation's local qualification passed; exact-head CI qualification
is still pending.** A first CI attempt exposed stale distribution fingerprints
outside the Rust profile assertions. The integration fixes remain in stacked
draft PR #811, targeting the R6 branch behind #809 rather than `main`. Preserve
that stack and keep the R6 branch unchanged until the correction PR qualifies.
The corrections
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
- Implementation correction: `adb8c0dc1450c3f049e9d48d7399fb9c3d3acc1d`.
- The CI integration follow-ups update this report. Resolve the
  candidate's exact SHA with `git rev-parse HEAD`; it does not embed its own hash.
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

## Distribution-fingerprint follow-up

Standalone Full CI run `34560842098` on `adb8c0dc1` found old expected counts
and digests in the static distribution shell gate and its frozen manifest.
The run was canceled promptly instead of spending the remaining full-suite
time on a known-red candidate. Its Windows aggregate failed because its
prerequisites were canceled, not because a Windows test failed.

The duplicate expectations now agree with the measured Rust profile results.
The complete static distribution shell gate passes. Product distribution
snapshots are measured separately using the exact no-dev Cargo feature graph;
their counts must not be inferred from a dev-feature-unified catalog test.
Both exact product probes completed successfully: standard has 1,698 runtime
entries and 70 specializers; full has 15,698 entries and 120 specializers.
Both checked-in snapshots match the generated results byte-for-byte, including
the graph fingerprints. Packaging, native-host contracts, and 55 CI/catalog/
native Python tests also passed during the follow-up.

## Exact-head CI integration follow-up

Normal CI run `34561935652` on `89634ab570a31387f4a835332f51d9f9e93542ef`
passed Linux, Windows, every static gate, and most owner suites, including the
corrected distribution checks. Five execution jobs failed from two shared
causes; browser/PR aggregation failures are not additional causes:

- GPU owner and browser-compute compilation still called the removed unscoped
  budget APIs. GPU backing is an aggregate physical projection, so both its
  budget evaluation and domain realization now use the aggregate APIs. Two
  regressions retain the separation from per-call limits and independently
  enforce exact/one-over storage limits even with an empty supplied violation
  list.
- Runtime owner, browser N-body reference, and standard browser source loading
  encountered the same `math/mul` ambiguity: symmetric broadcast candidates
  described one compatible output using fixed versus live dimension names.
  Inferred output dimensions now normalize under the admitted live-equality
  relations, while imported rigid dimensions, bounds, input conversion plans,
  and explicitly expected outputs retain their authority. Otherwise equal
  overloads prefer an already-proved shape relation over an extra deferred
  live check, preserving singleton and empty broadcasts.

The first runtime run passed 614 tests and failed seven through this shared
source-planning ambiguity. Existing N-body physics, trajectory, durability,
and no-fallback assertions are unchanged. The correction adds shared numeric,
comparison, String-equality, Boolean, fixed/live, operand-order, and expected-
output regressions instead of special-casing N-body or changing its source.

Local follow-up evidence: all 52 core catalog/solver tests, all six exact GPU
owner tests (three R5 and three R6), and all 621 runtime owner tests pass.
The seven previously failing runtime tests now pass unchanged, including both
the public N-body independent-reference test and the source/bytecode D2 test
for 4,096 accepted turns. All 35 R3 mutation tests, R3/R5/R6 architecture gates,
and formatting checks pass. Required exact-head normal and Full CI remain
pending; no completed-green claim applies to this revision.

## Work remaining before merge

1. Qualify the review-note and EKF shape follow-up in draft PR #811.
2. Validate the current full native shard set through the required Full CI
   workflow. Preserve failures as concrete correction work; do not substitute
   the inherited inventory or earlier green run for this result.
3. Record the immutable tested source SHA and exact-head CI results in the PR
   qualification record, without amending implementation merely to add a run ID.
4. Merge only through the stack: #811 → #809 (R6) → R5 → earlier phases.
   Before any authorized merge, verify the R6 remote head is still
   `47fdb6a8ef7e7ce286553c65c1b76782200dc4e4` and preserve intervening work.
   Qualification depends on completed gates, not the imported checkpoint's
   status. This report does not authorize a merge or direct branch promotion.

A read-only check during this continuation confirmed the R6 remote remained at
the expected head with green existing checks. All review threads on **#809**
were resolved; the complete paginated result contained no open threads. The
separate user-requested review of #811 is recorded below. No agent-requested
follow-up review was launched.

The reproduced integration defects and the additional identity, sparse-feature,
and Resident budget-scope defects found during this continuation are corrected.
No further concrete implementation blocker was found in the inspected paths.
The combined architecture remains coherent under this bounded qualification;
its documented witness limits are not an exhaustive language-closure proof.
The candidate is ready for exact-head CI, not for a completed integration or
merge claim before that CI and the fresh full native union are verified.

## User-requested review and EKF follow-up

The review of `89634ab570a31387f4a835332f51d9f9e93542ef` returned four notes.
Its GPU API migration note was already corrected in `65163f2d0`; all six local
GPU tests and the exact-head GPU owner job passed, and that thread was answered
and resolved. The other three corrections are:

- Re-evaluate each remapped call's output/work budget against its instantiated
  execution target, including Host overrides in mixed programs and Resident
  call attachment. Aggregate memory admission remains separate. Exact and
  one-over limits reject during realization, while independent calls do not
  acquire an accidental program-wide work quota.
- Restrict elementwise equality schemes to the maintained numeric, Bool, and
  String families. Recursive `Equatable` evidence does not authorize nominal
  or structural matrix broadcasts. Scalar whole-value equality remains
  independent. Positive tests span the 16 maintained element kinds; negative
  tests cover Atom, Id, Index, Option, and tuple broadcasts in both orders.
- Preserve an explicit separator before a NaN imaginary component, regardless
  of its sign bit. Tests cover 36 C32/C64 finite, signed-zero, infinity, signed-
  NaN, and pure-imaginary conversions without changing their canonical tokens.

Normal run `34563553993` on `65163f2d02a36f5fe9dde3b7f76bef6ddb4d5656`
passed 27 jobs, including Linux, Windows, every owner, all static gates, and
the N-body reference. Browser compute failed during EKF source planning; its
downstream aggregate failures are not separate defects. Full CI was correctly
skipped. Linear indexing had discarded its fixed singleton column axis, so
transpose could not prove the row-broadcast shape. Access results now preserve
independent fixed-axis guarantees and explicit element schemas for empty masks.

The maintained native WASM test loads the complete unchanged EKF document and
configuration without requiring a browser build. It exposed the later trail
update's independent live dimension expressions as an additional local
reproduction before another CI run. Equal-live-shape candidates now retain
checked compatibility obligations for independent axis expressions, rather
than requiring identical symbolic names or aliasing imported dimensions. They
remain subordinate to more-specific exact and broadcast schemes and preserve
the supported comparison element families. Core regressions retain rigid input
conversion identities, fixed-shape rejection, and same-kind explicit expected
output authority. A real-catalog regression checks the rolling path expression.

Resident activation also copied the input axes through canonical index
conversion, despite that conversion producing a flattened column. It now
derives checked cardinality × 1 for matrices and preserves scalar shape for
scalar conversion. A no-shape-hint artifact regression covers both vector
orientations, rectangular selectors, empty axes, and scalars. Final follow-up
local evidence so far: 62 core tests (24 catalog, 30 solver, eight conversion),
435 engine tests (402 library, 16 R5 planner, 17 R6 runtime), and all 840 accepted
turns of the complete EKF scene test passed. The temporary slot diagnostic
confirmed the failing conversion and was removed; the maintained WASM test
itself is unchanged. R3/R5/R6 checkers, the unsafe-boundary audit, and all 233
selected checker/catalog mutation tests passed. The exact full-profile catalog
gate passed all three tests and 2,345 generated witnesses. All 17 stdlib source
tests passed using that shipping-full feature graph, including both new real-
catalog EKF expressions. The exact standard-profile catalog gate also passed all
three tests and 1,592 generated witnesses.

Normal CI passed on `ffa8e5ee4297ed42d5dffc24cc7819950e5d3b43`, including
Linux, Windows, all package owners, static contracts, and both browser suites.
Automatically chained Full CI run `34566718048` found one concrete failure:
the generated D2 artifact fingerprint was stale after the semantic corrections.
The known-red run was canceled to avoid spending the remaining qualification
time on that head; its final results were 67 successful jobs, one failed job,
and 46 canceled jobs. The pinned Miri job passed all 20 safety tests. These are
partial results, not completed Full CI qualification.

The existing D2 generator was rerun without modification. It executed the
current and immutable historical fixtures, checked their matching trajectory
against the frozen platform trajectory, and regenerated all six projections.
The sole generated difference is `d2-nbody-artifact-v1.json`'s
`program_revision`: the canonical artifact fingerprint changes from
`6ee952ad3f165542c6dde921123eaf4dad64748c99b8b8b3158c8e25e239b4a2` to
`d0d18adc88edd1a4788772cbfea76dc113e0d936f607c5c6bf9ea34388b26cf4`.
All other artifact fields and all five other projections remain byte-for-byte
unchanged, including numerical hashes, zero-allocation assertions, resource
sizes, and publication behavior. The D2 architecture checker also passes.
Required normal and Full CI must qualify the corrected exact head; the
earlier partial run is not substituted for that result.

## Distribution-specific bytecode test follow-up

Normal CI passed again on `53194b69c89df1ab96f5d9993b745a8d87a6591c`.
Full CI run `34569378873` then exposed a test-profile mismatch rather than a
production execution failure: the standard/default bytecode test called
`assert_bool_matrix`, but that helper was restricted to `distribution-full`.
The same restored-overload test also contained complex-number sources that
require the full profile's complex syntax and `math_abs` capabilities.

The correction makes the Boolean helper available in both distributions and
limits only the complex-number source block to the full distribution. Boolean,
String, numeric, and broadcast cases remain enabled in the standard profile;
all complex assertions remain enabled in the full profile. No production
behavior, source semantics, expected value, or CI gate changes.

The corrected tests pass under both exact profiles: 21 default-profile
bytecode tests and 37 full-profile bytecode tests. The later language-job steps
were also executed locally rather than stopping at the first formerly failing
test binary: 373 root-library tests, 189 default-core tests, 270 compiler-enabled
engine tests, five formatter tests, and both isolated assignment-feature tests
passed. The isolated compiler and String-concatenation feature checks passed.
The bytecode package's test targets compiled successfully but executed zero
tests in that invocation; they are not counted as additional test coverage.
These runs used a fresh local target directory after the reused D2 fixture
build cache produced conflicting crate metadata. Dependencies and lockfiles
were not changed.

On the pushed `53194b69c` head, the full architecture gate now passes, including
the generated D1/D2 checks and R2–R4 conformance. The R6 runtime gate passed
20 normal and 20 debug-assertions-disabled release safety tests; pinned Miri
passed all 20 safety tests independently. Both generated catalog gates passed.
All 20 native surface jobs, the fresh coverage merge, and all eight exact native
closure shards passed. The fresh union reproduces 122,083 entries, the expected
`34db793ac637b3b1bc532978c6e8a6e0be1b8be63c33ec5f3043045f765616f5`
digest, and 34,916 closure inventories with zero missing linkage, signatures,
or contracts. This is fresh evidence for that pushed head, not a substitute
for final-head qualification after the test-profile correction. Complete
normal and Full CI on the final unchanged head remain required.

The run finished with 124 successful jobs and two failed jobs: the single
language-test profile failure above and its dependent PR aggregate gate.
Native-plan generation and every standard/full release-package job passed;
no other concrete failure was found. The final correction therefore remains
limited to the two test-feature gates and this qualification record.

## Follow-up on the review pinned to `65163f2`

The preceding candidate `b7b80535ebdbe75e8481fe77e584a73e5a66399a` completed
normal and Full CI in run `34575564549`: all 126 checks passed. That qualifies
that head, not this subsequent correction.

The supplied review's EKF finding was already corrected in `ffa8e5e`.
`src/stdlib/tests/type_system_source.rs` checks the slice/transpose/broadcast
composition's singleton row, nonsymmetric 4×2 values, and incompatible matrix
subtraction. Both regressions passed in the preceding architecture job. The
unchanged browser compute job `103187161086` passed all five EKF scalar/GPU,
edit/no-edit cases and parity. No additional EKF or solver change is needed.

The live measurement finding remained valid. Before this correction, an
ordinary installed `access/scalar` consumer of a supplied 65,537-element F64
input failed with `ActivationKernel`, despite producing one scalar. The
corresponding unit tests identified `OutputElements` and `OutputBytes` as the
false rejections of borrowed existing values. The negative producer test
already rejected a 65,537-element result with `OutputElements`.

Borrowed measurement now contributes no prospective output elements or bytes.
It still uses call-scoped admission for traversal work, retained nodes, and
other supplied resource demands. Complete measured footprints still flow to
R5 for actual storage, old/candidate coexistence, and candidate-output
admission; switching wholesale to aggregate-only admission would incorrectly
drop the work guards. No output limit or memory policy was raised.

After the fix, all 404 engine unit tests and 19 R6 integration tests passed
under `full_compiler,resident-artifact`. The ordinary tests exercise supplied
inputs and constants at 65,536 and 65,537 elements, exact scalar values, actual
constant backing, and planned input backing audited during activation. They
check activation versus persistent byte accounting according to storage
lifetime, oversized-produced-output rejection, invalid-selector rollback of
the value/epoch/hash, and a subsequent successful turn. Unit tests retain
exact-limit/one-over-limit work and retention checks, bounded selector
traversal, and the distinction between measuring a large existing String and
admitting the same payload as a new output.

The existing `full_compiler` owner profile also passed all 229 unit and 17
integration tests with the Resident-only regressions correctly gated. Pinned
format checking, the R6 architecture checker, and the unsafe-boundary audit
passed.

The existing Full CI R6 integration invocation now enables `resident-artifact`
so the new ordinary Resident tests actually execute. Its existing non-Resident
owner profile remains supported. No new CI job, Python mutation suite,
allocator, executor, or broad review was added. Complete normal and Full CI
must qualify the corrected unchanged head before it is reported green.

## Configured aggregate-budget checkpoint and early native qualification

The security advisory on `84ed702e02` correctly identifies the absence of an
aggregate Resident allocation ceiling. The existing caller configuration
`RuntimeLimits.max_memory_bytes` supplies the policy; this checkpoint does not
invent a finite default or reuse the per-operation output quota as a program
limit. `None` remains unconfigured.

A shared managed-memory account now reserves validated unique arena/envelope
capacity and existing planned metadata before physical materialization. Its
non-cloneable charges follow actual allocation and payload-envelope owners,
including outstanding reservations, retained immutable roots, and old/new
program coexistence. Source and bytecode activation receive the runtime's
account; reactivation preserves it. Failure releases candidate ownership and
does not alter the previous program's publication. Aliases are not charged as
additional arenas and persistent/activation summaries are not added together.

Pinned local validation passed three core configured-budget integration tests,
two core account unit tests, one ordinary Resident integration test, and one
runtime test exercising source and bytecode loading. These cover exact limits,
one-over-limit rejection, overflow, shared-owner lifetime, old/candidate
coexistence, rejected replacement followed by successful old-plan execution,
and unload/retry. The R6 architecture checker and unsafe-boundary audit passed.
New-checkpoint release and Miri qualification have not yet run.

This is a checkpoint, not closure of the security advisory: Resident mutable
String/Snapshot lane construction and growth still need connection to the same
account before their materialization. Their existing per-call admission is
not a substitute for aggregate retained ownership. The review thread remains
open until that integration and its regressions are complete.

Run `34599294964` on `84ed702e02` finished with 123 successful checks and three
failures: Native plan and its two dependent aggregate gates. The native runner
reported a shutdown signal at 13:59:46 UTC and exit 143, not a failed Rust
assertion. One registry test completed successfully; its live/CTRL-C sibling
was interrupted without a result. The log does not establish who initiated
shutdown or whether the underlying cause was infrastructure or intervention.
No agent cancellation or superseding run was issued.

That native job began about 50 minutes after the workflow started because Full
CI waited for normal CI. Required PR validation now launches the same native
qualification after impact detection, concurrently with normal CI. Its two
registry tests run first in a separately reported step, and only those already
executed tests are excluded from the later broad invocation. Full CI delegates
that one job to the caller; the final PR gate requires its success. Standalone
Full CI still executes and requires the native job itself. No test assertion or
required gate is removed. All 37 CI contract/impact tests pass, including shell
execution proving failed, cancelled, skipped, or missing early native results
cannot make a required PR gate green.

## Aggregate payload ownership completion — 2026-09-11

This correction completes the payload integration explicitly left open by the
preceding checkpoint. It uses the same optional configured account and R5
turn estimates; it does not replace per-call output/work guards, introduce a
finite default, or claim to limit compiler allocations or process RSS.

The existing Resident typed arena now retains its additional String capacity
with a payload owner. Cold input construction, captured inputs, kernel staging,
read/modify/write seeds, retained state, effect copies, and output projections
admit candidate/temporary capacity before materialization. Initial envelope
capacity is credited once. Tracked capacity follows the actual String buffers,
including shrink/replacement, while failed candidates leave publication intact.
Invariant fixed-width execution retains its cached allocation-free path.

Canonical imports consume an admitted reservation into a new immutable ownership
wrapper. The wrapper retains the actual shared data and schema owner, and its
charge survives export, runtime unload, and session close. A weak registration
shares true same-account/same-schema imports without changing caller-owned Value
clones. Cross-account imports retain independent obligations. Dropping a rejected
candidate removes its claim even while the caller retains the original value.
Both fallible wrapper/registration construction points have rollback tests.
No raw pointer identity, unsafe Send/Sync, or new unsafe allowlist is introduced.

Owning output/effect exports admit their complete copy/finalization peak and
transfer retained ownership to the returned Value. A borrowed output read needs
no extra owning export. Projection and migration copies stage before switching
publication tags; failure does not advance their versions or require fallible
rollback. Input/candidate coexistence remains charged to the same runtime account.

The bounded final check also found that parameterized Value clones copied
their shape-parameter Box, and that the old schema-clone byte multiplier
undercounted wide structural schemas. Immutable Values now share the admitted
shape owner with their frozen data, including across exact rebinding and
exports. The general semantic ShapeInstance API and its encoding are unchanged.
The existing R5 finalization witness accounts the concrete shared-owner headers;
the schema clone witness traverses actual boxed children, dimensions, names,
parameter declarations, and retained encoding buffers with checked arithmetic.
These are part of the same complete metadata-ownership correction, not new
default limits or a separate planning policy.

Local candidate evidence (pinned nightly, locked dependencies):

- Engine: all 408 library and 20 R6 integration tests passed, including
  parameterized owning export at an exact account limit.
- Core safety/runtime profile: all 155 unit, 41 R6 runtime, and 23 safety tests
  passed. This includes exact structural clone witnesses and 128 parameterized
  immutable clones performing zero allocations while retaining one charge.
- New ordinary runtime regressions passed for aggregate String outputs,
  source/bytecode budget propagation, String grow/shrink/rejection/retry,
  canonical import failure/retry with unchanged caller/publication, and exported
  String/canonical ownership after unload. A broader reduced-feature invocation
  also selected three unrelated feature-dependent tests and failed; it is not
  reported as a clean full runtime suite.
- Release safety with debug assertions disabled: all 23 tests passed.
- Pinned Miri safety: all 23 tests passed, including the shared parameterized
  shape regression (140.20 seconds executing the final suite).
- Complete runtime owner profile (`full_compiler,resident-routing-source`): all
  626 library tests passed on the frozen final correction, including the new
  budget regressions and maintained N-body source/bytecode paths.
- Final formatting, whitespace validation, R1 compatibility and R2–R6
  architecture checks, and the unsafe-boundary audit passed. D1/D2 checks also
  passed earlier in this correction; their exact final qualification remains
  part of the required normal/Full CI.
  Complete normal/Full CI and review must qualify the pushed exact
  head; the earlier clean reviews of `551622578` do not qualify this correction.
