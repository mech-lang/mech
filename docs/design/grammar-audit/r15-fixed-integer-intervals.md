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
Aggregate members and mutable or external values are checked by the
schema-directed snapshot finalizer before publication; an out-of-range value
must fail without publishing a partial state change.

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
- Combined `mech-stdlib/full_compiler`, `mech-runtime/full_source` and
  `mech-runtime/resident-routing-source`, selecting existing
  `r6_managed_functions` and `canonical_constant_binding`: 16 managed and
  49 runtime tests passed with the ordinary test-thread invocation. The previous
  serial run is not a concurrency waiver.
- Reduced Standard compiler managed target: 12 passed, but it excludes u8, so
  this is compatibility evidence rather than interval coverage. Standard plus
  the existing u8 feature: 15 passed, including both new mutable interval cases.

The accepted narrow contract above supersedes issue #865's earlier deferral;
broader refinement features are not claimed. Historical counts remain assigned
to their original candidates. R15 CI remains paused pending authorization, and
local boundary qualification does not itself seal R15.
