# R15 fixed integer interval contract

R15 implements the first G12 constrained scalar form. A kind such as
`u8:1..10` denotes the values 1 through 9. The lower endpoint is included;
`..` excludes the upper endpoint and `..=` includes it. Equal endpoints are
valid only with `..=`. Descending and empty intervals are invalid.

This accepted fixed-integer contract supersedes the earlier post-v0.4 deferral
in issue #865 for this narrowly defined form. Qualification and sealing remain
separate from that scope decision. Completion of R15 does not claim arbitrary
scalar predicates, dynamic bounds, stepped membership, implicit interval
subtyping, or arithmetic result-bound inference; those need separate contracts.

The supported domains are the exact signed and unsigned integer widths 8, 16,
32, 64, and 128. Both endpoints must be closed decimal integer literals that
fit the declared width. Mixed-width endpoints, fractions, floating point,
complex, rational, NaN, infinity, and all noninteger scalar domains are
rejected at the range annotation. Three-operand ranges and their step semantics
are deliberately unsupported in this first form; they receive a positioned
diagnostic rather than an inferred interval meaning.

Endpoints are evaluated once at source admission. Names, effects, captures,
activation-dependent values, and turn-dependent values are not admitted as
endpoints. A constrained kind has distinct kind and schema identity. Its
canonical schema and reified-kind encodings include signedness, width, both
endpoints, and the upper-inclusion flag. Artifact transport must retain that
identity and revalidate values after decoding or rebinding.

An exact-base integer constant can enter its interval only after a membership
check. A live plain integer cannot be implicitly narrowed to an interval.
An interval value can be rebound to its exact base kind by a checked lower
layer, but source typing does not implicitly widen it. Different intervals
do not implicitly subtype one another, even when one contains the other.
Aggregate members and checked mutable or external snapshots are checked by the
schema-directed snapshot finalizer. Initialized fixed-width interval regions
are separately checked against their exact integer bounds under retained
storage authority before publication; initialization and shape alone are not
membership evidence. An out-of-range value must fail without publishing a
partial state change.

The following paired outcomes are the minimum acceptance evidence: lower and
last admitted value succeed; the value below lower and the excluded upper fail;
the included upper succeeds with `..=`; invalid width, empty interval,
unsupported domain, dynamic endpoint, and stepped expression fail at the
source location; reified identity, aggregate members, external input, state
update, and bytecode roundtrip retain the same interval and rejection behavior.

## Capability consistency correction (2026-10-03)

Review comment 4175038060 exposed a construction-path discrepancy: a directly
constructed `ResolvedType::new(KindExpr::IntegerInterval(...), ...)` did not
receive the equality, keyability, and ordering evidence already supplied by
the schema-derived path. The correction assigns `Equatable`, `Keyable`, and
`Ordered` to valid intervals in the shared intrinsic predicate classifier,
which also supplies generic constraint solving and structural child evidence.
The redundant schema-only interval rule is removed; enum payload evidence
remains schema-authoritative. This does not grant numeric, arithmetic, or
range-endpoint capabilities and does not change implicit narrowing/widening.

The four new regressions cover all ten signed/unsigned widths with both upper
endpoint policies; direct versus schema-derived types; option, matrix, set,
tuple, record, table, and map children; generic predicates; mixed direct/schema
binary inputs in both orders; and invalid interval rejection. Table and map
roots retain their existing non-keyable policy, and aggregates do not acquire
scalar ordering. Before the production correction, the first three regressions
ran: the two capability regressions failed and invalid-interval rejection
passed. They then passed with the correction; the mixed-origin case was added
and passed afterward.

Fresh local evidence for this correction:

- `cargo +nightly-2026-03-03 test --locked -p mech-core --features compiler
  --test type_system_builtin --test type_system_contract --test type_system_solver`:
  17 builtin, 15 type-contract, and 30 solver tests passed.
- `cargo +nightly-2026-03-03 test --locked -p mech-core`: all enabled unit,
  integration, and documentation suites passed, including 70 library and
  3 documentation tests. Feature-disabled zero-test targets are not coverage.
- `cargo +nightly-2026-03-03 test --locked -p mech-engine --no-default-features
  --features full_source,resident-artifact --test canonical_source_semantics`:
  all 138 source-semantics tests passed.
- Formatting, whitespace, warning policy, R3 type-system, and R4 type-cutover
  contracts passed. Builds use the repository's warning-denied configuration.

The nine existing R15 commits were restacked from R26 `3f1f23772a` onto the
review-clean R26 `873ebe7cf3` without changing their patches. The correction
above is the only new R15 implementation delta. Historical qualification
counts belong to their earlier candidates; these local checks are not Full CI,
a new exact-head external review, or a seal. CI and new R15 review requests
remain paused under the existing instructions.

## Output-template correction (2026-10-03)

Review comment 4175643613 on `32327f6f36` exposed the next construction boundary:
`schema_body_from_resolved` could create an interval template, but the shared
`resolved_schema_body` matcher rejected it. The five-line correction admits
only equal `KindExpr::IntegerInterval` / `SchemaBody::IntegerInterval` pairs.
Signedness, width, both bounds, and upper inclusion remain exact semantic
identity; a primitive base-integer or different-interval template still fails.
Recursive aggregate materialization inherits this one leaf rule. No enum
authority, dimension-witness policy, allocation rule, or resident semantics
changes.

Five new core regressions cover all ten integer types with both endpoint
policies, nine scalar/aggregate families, `FromResolvedType`, `Declared`, and
`FromInput`, compound parameterized matrix witnesses, nested supported
lower-bound witnesses, dynamic Cartesian/powerset output rules, and rejection
of all interval-identity changes and erased base-integer templates. The
parameterized matrix uses a non-lower witness; the nested option case does not
claim arbitrary input-witness preservation beyond the existing policy.

The maintained registered-specializer tests exercise set construction and all
four algebra operations through the actual resolved runtime binder, asserting
complete schemas, shapes, values, and bound-call contracts. Public runtime
tests separately load source and decoded bytecode, check full identity at
publication, and execute another turn. The public frontend has a separate
schema-draft materializer and already handled intervals: those public cases
are regression coverage, not the failure-before oracle for this binder bug.

Failure-before evidence: all five focused core tests ran; three positive
materializations failed at the missing interval-template match and two
rejection tests passed. Both new registered-specializer cases also reported
the exact identical-interval/template mismatch. That parallel test process
aborted during panic teardown, so it is recorded as two executed
reproductions, not a normally completed two-test suite.

Fresh correction qualification with nightly `2026-03-03`, locked dependencies,
incremental compilation disabled, and two build jobs:

- Core `full` profile: 14 `r4_type_cutover` and 17 `type_system_builtin` tests
  passed, including all five new materialization regressions.
- Combined stdlib/runtime profile (`mech-stdlib/full_compiler`,
  `mech-runtime/full_source`, `mech-runtime/resident-routing-source`): all 14
  `r6_managed_functions` and 47 `canonical_constant_binding` tests passed with
  one test thread, including the four new registered-binding/public-loading
  cases. Invalid initial fixture spellings were corrected to established
  Unicode kind delimiters; those initial fixture failures are not qualification.
- Engine `full_source,resident-artifact`: all 138
  `canonical_source_semantics` tests passed.
- Formatting, whitespace, warning policy, R3 type-system, and R4 type-cutover
  checks passed under the repository's warning-denied configuration.

Historical results above remain attributed to their original candidates.
R15 review requests were subsequently authorized; another correcting-head
review is requested after publication. CI remains paused. These checks are
not Full CI or a seal.

## Live-value and transport acceptance (2026-10-03)

The boundary qualification starts from review-clean R15
`6ad5b16205fcfb4e18cbfca697275db64a6a905a` on R26 `efc049e9d`.
No new production correctness defect was reproduced, and no production code
changed. Tests extend existing owner suites rather than adding a transaction
framework or interval arithmetic.

Public runtime cases load original and decoded artifacts and exercise the same
activated scalar/matrix instance. Checked base-to-interval host entry admits 2,
rejects the excluded upper, then admits 3. Unconverted base snapshots are
separately refused at runtime input capture, preserving the no-implicit-live-
narrowing rule. Membership refusal and input-schema refusal are distinct checks;
invalid finalized Values are never manufactured. State, output, complete schema
and shape, published epoch, state revisions and state hash stay unchanged after
refusal, followed by valid same-instance execution.

The mutable ValueCell owner tests check candidate membership and the actual
replacement gate, aliased values, cell identity, descriptor and publication
revision. A nonsquare 2x3 matrix's raw rebuild candidate has five valid members
before an invalid sixth: refusal identifies MatrixElement(5), with no input or
registered-transpose output change. A valid retry commits on the same cell and
executes the same registered transpose, checking complete 3x2 values and the
retained interval element schema. This mutable adapter evidence is not replaced
by an immutable rebind test alone.

Exact membership tests cover all ten widths: signed minimum/maximum, unsigned
maximum, inclusive extreme singletons, excluded upper, negative intervals that
exclude zero and odd exact 64/128-bit payloads beyond floating-point precision.
Both finalization and checked base rebind are tested. Codec tests roundtrip all
ten extreme intervals and retain ordinary integer schema tags/width encodings.
Malformed valid section JSON reaches InvalidIntegerIntervalV1 for out-of-width,
empty or descending bounds, and IntegerIntervalViolationV1 for invalid payloads;
an unrelated checksum/envelope failure is not accepted as evidence. Restoring
the original representation decodes successfully.

Fresh normal-thread qualification uses locked nightly-2026-03-03, warnings
denied, incremental compilation disabled and two build jobs:

- `cargo +nightly-2026-03-03 test --locked -p mech-core --features full
  --test snapshot_value_contract --test r4_type_cutover
  --test type_system_builtin --test type_system_contract --test type_system_solver`:
  10 snapshot, 14 cutover, 17 builtin, 15 contract and 30 solver tests passed.
- `cargo +nightly-2026-03-03 test --locked -p mech-engine --no-default-features
  --features full_source,resident-artifact,compiler
  --test program_artifact_contract --test canonical_source_semantics`:
  37 artifact and 138 source-semantic tests passed. The codec target explicitly
  requires compiler; a feature-disabled refusal is not counted as execution.
- `cargo +nightly-2026-03-03 test --locked -p mech-stdlib -p mech-runtime
  --no-default-features --features
  mech-stdlib/full_compiler,mech-runtime/full_source,mech-runtime/resident-routing-source
  --test r6_managed_functions --test canonical_constant_binding`: 16 managed and
  49 runtime tests passed with the ordinary test-thread invocation. The previous
  serial run is not a concurrency waiver.
- `cargo +nightly-2026-03-03 test --locked -p mech-stdlib --no-default-features
  --features standard_compiler --test r6_managed_functions`: 12 passed, but it excludes u8, so
  this is compatibility evidence rather than interval coverage. Standard plus
  the existing u8 feature (`--features standard_compiler,u8`): 15 passed,
  including both new mutable interval cases. Bare `compiler,u8` selects zero
  tests in this target and receives no coverage credit; the maintained preset
  must be selected.

The test/evidence patch `ca1bf1c13` and all eleven preceding R15 patches were
restacked onto review-clean R26 `0f1c8feabf87a0bb86f9a986d3e6329639ff8ee8`.
All twelve patches are identical by range-diff. Every successful command above
was refreshed with the same nonempty counts on resulting code/test candidate
`a96608b4529c2a13505cd59eb1e21c2f092863fd`. Formatting, whitespace, warning
policy, R3/R4 and the bytecode format's 21 deterministic fixtures also passed
there. Routing, quarantine and retired-value checks passed before the mechanical
restack; those historical runs are not relabeled as new-head execution.

The accepted narrow contract above supersedes issue #865's earlier deferral;
broader refinement features are not claimed. Historical counts remain assigned
to their original candidates. R15 CI remains paused pending authorization, and
local boundary qualification does not itself seal R15.

## Initialized-region publication correction (2026-10-04)

The source-traced review of `7b197ca5fbec577ecb3125c305219297d844d420`
identified a publication route that did not use the snapshot finalizer.
`InitializedManagedRegion` supplied checked initialization and shape but no
interval membership evidence. The owner regression reproduced this on the
unchanged implementation: an ordinary typed U64 region containing 10 reached
ReadyPublication for a valid `u64:1..10` cell holding 2. Both `--features full`
and the maintained CI `--all-features` invocation executed the focused test
and failed at that specific ready-gate assertion. Unexpected ready objects
were dropped before assertions; invalid bytes were never committed.

The correcting code/test candidate is
`114925449ac698e1e6b4c91380b6a16c6d819a47`, continuing from the reviewed head
on R26 `0f1c8feabf87a0bb86f9a986d3e6329639ff8ee8`. The earlier capability,
output-template, checked-value and codec acceptance corrections remain intact.

Single and batch readiness now validate initialized interval candidates before
locking any cell or entering the infallible gate. Validation uses exact typed
views of the candidate region and the existing signed/unsigned interval
membership functions. It neither constructs a canonical payload copy nor
changes primitive-output publication. Matrix checks use logical coordinates,
not padding or spare capacity; diagnostic indices are row-major and complete
shape geometry is retained. Empty geometry returns after validation without
iterating an empty large axis. All ten fixed integer widths remain exact.

Preparation alone is not a durable check: an admitted writer can change bytes
before readiness. The candidate scan therefore acquires a read lease retained
through ready commit or abort, preventing typed writes and host-arena
projections from invalidating membership evidence. The read-only private
authority retains the exact realization, region and incarnation even when a
sibling call has finished staging. A required in-place candidate instead
borrows its matching retained exclusive undo lease; failure keeps rollback
authority intact. No fallible check was added to final commit.

Six added cases in the existing `r6_memory_runtime` target cover:

- Raw scalar excluded-upper refusal and valid retry, with unchanged accepted
  value, alias identity, schema, shape and publication revision after refusal.
- Valid preparation followed by raw invalid mutation, overlapping-writer
  refusal, and typed-writer/host-projection exclusion while Ready is held.
- A parameterized nonsquare matrix with a late invalid member, exact
  `MatrixElement(5)` rejection, atomic refusal of an earlier valid sibling,
  stale preparation, complete unchanged bindings and valid batch retry.
- All ten exact integer widths and signed minimum/maximum bounds: 22 bound
  subcases, not 22 independently selected tests.
- Permitted empty and padded matrices: invalid padding is irrelevant, an
  invalid logical sixth lane is not, and a valid retry preserves full identity.
- A real required-in-place/undo fixture: raw invalid output is rejected at
  readiness, old bytes and bindings are restored, and a valid retry commits.

Deterministic negative controls were executed and then removed. Bypassing the
membership predicate made the scalar ready-gate test fail. Releasing the read
lease immediately after validation made the writer-exclusion test fail even
though membership validation still ran. Both tests drop unexpected
capabilities and abort ready objects before assertions; neither publishes an
invalid candidate to demonstrate the defect. Restored source is the candidate
qualified below.

During implementation, an additional all-features library diagnostic was not
green: 273 of 274 tests passed, while the existing
`cell_binding::tests::exact_interval_matrix_output_seeds_every_lane_inside_the_interval`
failed during construction with `ValueCellStorageContractViolation` / "storage
capabilities are opaque", before initialized-region publication. A separate
clean worktree at unchanged `7b197ca5f` executed that exact library test and
reproduced the same error. This is recorded as a pre-existing diagnostic
failure, not a passing suite or a regression caused by this correction. The
integration targets listed after that failing library invocation did not run
in that invocation and receive no credit from it. The maintained publication
CI route selects `--all-features --test r6_memory_runtime`; no workflow routing,
test suppression, storage-capability policy or earlier finding was changed.

Fresh qualification on `114925449ac698e1e6b4c91380b6a16c6d819a47` uses locked
nightly-2026-03-03, warnings denied, incremental compilation disabled, two build
jobs and ordinary test threads:

- `cargo +nightly-2026-03-03 test --locked -p mech-core --all-features
  --test r6_memory_runtime --test r5_memory_plan`: all 48 managed-memory and
  23 memory-plan tests passed. The six new cases were also separately executed
  and passed before the committed-candidate refresh. They are not added again
  to the full-suite count.
- Core `--features full`, with `snapshot_value_contract`, `r4_type_cutover`,
  `type_system_builtin`, `type_system_contract` and `type_system_solver`: all
  10 snapshot, 14 cutover, 17 builtin, 15 contract and 30 solver tests passed.
- Engine `--no-default-features --features full_source,resident-artifact,compiler`,
  with `program_artifact_contract` and `canonical_source_semantics`: all 37
  artifact and 138 source-semantic tests passed, retaining interval payload,
  malformed-bound and source/decoded artifact witnesses.
- Combined stdlib/runtime `--no-default-features --features
  mech-stdlib/full_compiler,mech-runtime/full_source,mech-runtime/resident-routing-source`,
  with `r6_managed_functions` and `canonical_constant_binding`: all 16 managed
  and 49 runtime-binding tests passed, retaining live refusal/recovery and the
  registered interval-matrix transpose.
- Standard `--no-default-features --features standard_compiler` managed-function
  target: 12 passed, compatibility evidence only. Standard plus u8: 15 passed,
  including the existing interval mutable-cell witnesses.
- Core `--no-default-features --features functions,u8,u64,f64,string,matrixd,bool
  --test r6_memory_safety`: all 23 passed, including primitive fixed-width
  publication without a canonical copy, initialization/stride safety,
  borrow-unwind recovery and steady-state allocation checks.
- The same 23 memory-safety tests passed in release mode with
  `RUSTFLAGS='-D warnings -C debug-assertions=no'`. Debug assertions are not
  membership or publication authority.
- Default-profile `cargo +nightly-2026-03-03 test --locked -p mech-core`: all
  enabled library, integration and documentation suites passed, including 70
  library and 3 documentation tests. Feature-disabled zero-test targets are
  not interval or initialized-region coverage.
- Warning-denied core checks passed for isolated `functions`, `functions,u8`
  and `functions,i128` profiles. Formatting, whitespace, R3/R4/R6, warning
  policy, bytecode format (21 deterministic fixtures), production routing,
  compiler-planning quarantine and retired-value checks passed.
- The existing R3/R4/R6 and warning checker unit/mutation cases passed using
  `python3 -B scripts/run-python-unittest-shards.py --jobs 4` with
  `test_check_r6_memory_runtime.py`, `test_check_r3_type_system.py`,
  `test_check_r4_type_cutover.py` and `test_warning_policy.py`: all 197 cases
  passed across four balanced subprocess shards. The interrupted preliminary
  serial invocation is not credited as a completed run.

Historical results above remain assigned to their original candidates. CI
remains paused; local evidence is not exact-head Full CI, an external clean
review or a seal.

## PR CI and exact ordering closeout (2026-10-04)

The user restacked all fifteen R15 patches onto qualified R26
`8899be8bf7c81fd38a54eea5d0a031144a925be5`, producing `2ee7706ba`.
Range-diff confirms every patch unchanged. The empty, skip-free trigger
`6f2eb2e86` has the same complete tree as reviewed `814cce330d`.
Subsequent review 5406531421 on `2ee7706ba` nevertheless found an additional
ordering issue; identical trees do not make that finding disappear.

CI is now authorized for R15 through its branch PR. Normal PR run
`37206905754` selected Full validation but failed its architecture and core
owner gates. The separate workflow-dispatch run `37205943106` was cancelled
at the user's request and supplies no passing qualification.

The architecture failure was the missing `IntegerInterval` allowance for an
actual canonical-source test import. The exact import inventory is updated;
the checker is unchanged. The core failure was the previously recorded matrix
output-seed fixture: an allocation requires a concrete backing, not the opaque
`AnyStorage` pattern. The fixture now requests supported `Exact(MatrixD)` and
checks unchanged schema, complete shape and both admitted lower-bound lanes.
No production storage or interval validation rule is relaxed. The actual core
CI owner command includes the library and six integration targets; the earlier
focused 48-test publication pass did not establish that entire owner command.

Review comment 4177940006 exposed a separate source/runtime type boundary.
Intervals have `Ordered`, but the ordering schemes inherited `Number` and
`Promotes`, so the resident's existing exact comparison implementation was
unreachable from source. Before correction, the two new source tests ran:
the positive failed with `incompatible-comparison-kinds` because the interval
did not satisfy `Number`; the mixed-identity refusal test passed.

The correction uses the existing compiler-only source-template mechanism.
All four ordering operations retain their previous numeric, Index and String
schemes. Only an identical, valid interval pair appends concrete exact scalar
and maintained elementwise schemes. Scalar/matrix, row/column and compatible
matrix dimensions use the existing comparison owners. No runtime arithmetic,
numeric capability, implicit narrowing, interval subtyping or wire format is
changed. A generic `Ordered` prototype was discarded before publication
because it would admit unrelated unsupported layouts.

One core contract test verifies identity conversions, independent symbolic
axes, unchanged ordinary schemes and mixed-kind refusal. Two source tests
exercise all four relations, nonsquare ordering, both broadcast axes and
operand orders, all ten integer widths, adjacent exact values above floating
precision, signed minimum and U128 maximum, and mixed base/bounds/inclusion/
width refusal. Actual source and decoded-artifact execution checks published
schema, complete shape and values. Their subcases are not independent suite
counts.

Fresh local qualification of this correcting code, continuing from `6f2eb2e86`,
uses locked offline nightly-2026-03-03, warning denial, no incremental
compilation, two build jobs and ordinary test threads:

- Core `--all-features --lib --test type_memory_contract
  --test storage_capability --test operation_memory_requirement
  --test type_memory_boundary --test r6_memory_runtime --test r6_memory_safety
  --test type_system_contract --test type_system_builtin --test type_system_solver`:
  274 library, 7 memory-contract, 12 storage, 14 operation-memory, 9 boundary,
  48 managed-memory, 23 memory-safety, 16 type-contract, 17 builtin and 30
  solver tests passed. This refresh includes the previously failing CI owner.
- Engine `--no-default-features --features full_source,resident-artifact,compiler
  --test program_artifact_contract --test canonical_source_semantics`:
  all 37 artifact and 140 source-semantic tests passed.
- All nineteen commands from the static architecture job passed on the
  correcting source, including formatting, the exact import inventory,
  78 CI/browser Python cases, unsafe and R1-R6 contracts, and the bytecode
  format's 21 deterministic fixtures.

Historical evidence above remains attributed to its original code. Fresh
correcting-head review and the PR's complete selected CI remain seal gates;
neither the failed older PR run nor local results are relabeled as a seal.

## Managed positional-index correction (2026-10-04)

Review comment 4178697930 on `118fa783e` identified a remaining physical
dispatch mismatch. The shared selector validator admitted integer intervals,
but the maintained, publicly registered `access/index` factory selected its
typed input port by matching only primitive schemas. Its scalar and matrix
entry paths therefore rejected valid interval selectors during binding.

Before changing production code, ten new integer-family regressions failed at
the reported `CannotConvertToType` binder error. Two independent matrix-first
regressions also failed there, without a preceding scalar conversion. These
are actual public runtime-catalog binding tests, not an unrelated frontend
failure or an unsupported layout. Canonical source/resident selection can
bypass this factory; passing source examples alone would not reproduce it.

The five-line correction unwraps the interval only when choosing the
fixed-width backing in `ManagedIndexInput::bind`. The original input port
retains its exact interval schema, complete shape, logical identity and
publication authority. Existing width feature guards, membership validation,
portable one-based index limits, memory contracts and output `Index` identity
are unchanged. This does not introduce implicit widening, narrowing,
arithmetic promotion or another evaluator. A neighboring-owner source audit
found existing normalized physical dispatch in managed access/assignment and
interval-aware handling in resident selectors; no broader change was needed.

The twelve new cases cover all ten signed/unsigned widths, positive scalar
updates, asymmetric 2-by-3 selectors flattened to a 6-by-1 Index output,
repeated selectors, and valid recovery on the same bound instance. Zero and
negative interval members, plus portable-limit overflow in wider types, are
valid constrained inputs but invalid positional selectors. Their conversion
must refuse without changing output values, full descriptor, identity or
publication revision. A late invalid matrix member follows a distinct valid
prefix, so an uncommitted partial write cannot hide behind identical values.

Fresh qualification from `118fa783e` plus this correction used locked offline
nightly-2026-03-03, warning denial, ordinary test threads, two build jobs and
no incremental compilation:

- Engine `--no-default-features --features full_compiler,resident-artifact
  --lib --test r6_memory_runtime --test canonical_source_semantics
  --test program_artifact_contract`: 665 library, 31 managed-memory, 140
  source-semantic and 37 artifact tests passed, with no ignored or filtered
  tests in those selected targets. Suite counts overlap earlier evidence.
- The exact engine CI-owner command with `full_compiler` also passed 236
  library, 77 source-semantic and 28 managed-memory tests before strengthening
  the late-failure prefix assertion; the expanded run above includes that
  stronger assertion. Feature-disabled resident cases in the former profile
  are not additional coverage.
- Formatting, whitespace, canonical-evidence imports, warning policy, R3,
  R4, R6, unsafe boundaries and runtime-factory safety passed. No checker or
  admission limit was weakened.

The preceding comparison-template helper corrections remain attributed to
`118fa783e`: 26 core catalog tests and all 17 frozen specialization cases
passed locally. They are not newly executed core qualification for this
production correction. Historical publication, reduced-profile and transport
results above retain their original attribution.

At the user's request, normal PR CI run `37221898495` on `118fa783e` was
cancelled while closing this review finding. Its cancellation-derived failed
aggregate gates are not source/test failures or passing qualification. The
correcting candidate requires a clean exact-head review before PR CI is
allowed to finish; each subsequent correction must repeat that order. Local
results are not a seal or a fresh external-review result.
