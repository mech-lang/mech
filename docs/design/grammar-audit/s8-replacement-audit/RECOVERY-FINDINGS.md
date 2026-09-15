# Findings from corrective qualification

This supplement preserves the accepted `d25fbaad0` baseline and its original
25 groups. It records new demonstrated causes before implementation, rather
than silently widening an existing correction. Current declared scope is the
baseline plus the following three findings: 28 groups and 26 review boundaries.
These are ownership counts, not an effort estimate or a completeness guarantee.

## G27 — Index range resident prerequisite (proposed R24)

**Owner:** resident range cardinality, physical binding and execution. R03 owns
schema relocation; it does not own implementing new range kernels. Q31's Id-like
Index identity/binding and zero-index validation are qualified by R03, while
Q31's positive range endpoint subcase remains blocked by this prerequisite.

The canonical frontend and artifact codec admit `lo<ix>..hi<ix>` with detached
Index constants 1 and 3. Decoding succeeds. Activation returns
`InvalidDependency { node: NodeId(0) }`: `canonical_range_cardinality` calls
`snapshot_range_number`, whose match admits integer and floating-point ValueData
but omits Index. This is independent of G25 schema ordering and G26 wrapping.
The binder has already removed both live inputs; live-range shape rejection
cannot explain this constant-endpoint failure.

The strict positive test is
`src/runtime/tests/s8_recovery_index_range.rs::bound_index_range_has_exact_source_and_bytecode_values`
on the audit branch. It asserts exclusive output `[Index(1), Index(2)]` and
inclusive output `[Index(1), Index(2), Index(3)]`, with exact 1×N Index schema and
two turns through source and decoded artifacts. It intentionally fails until the
prerequisite is implemented; there is no success-on-rejection mode or ignored
positive test. This also corrects the initial exploratory fixture's exclusive
endpoint/column-orientation expectation before recording the final witness.

Run on the audit branch (or copy the one test file into a corrective checkout):

```sh
CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 \
  cargo +nightly-2026-03-03 test --locked -p mech-runtime \
  --no-default-features \
  --features full_compiler,full_source,resident-routing-source,compute \
  --test s8_recovery_index_range \
  bound_index_range_has_exact_source_and_bytecode_values -- --exact
```

R24's finite acceptance must cover both range modes, explicit increments,
constant endpoints, portable Index bounds/overflow, and the existing rejection
of changing live endpoints. Index is not an arithmetic alias for u64. The
existing Index catalog/type contract must govern the physical implementation.
No production Index range implementation is included in R03 or this supplement.


## G28 — Compute output schema planning precedes its interface (R25)

**Owner:** canonical mixed ProgramCompiler resource planning, with the canonical
semantic frontend supplying staged partitions (the existing E8/E4 responsibilities).
R25 is a targeted correction; C/G22 owns the consuming browser edit acceptance.
This does not change language scope or introduce a compute-provider substitute.

The actual shipping server at C `4c901a1ea` fails to compile a configured mixed
document whose coordinator reads `@compute/sample/result`. It reports
`RuntimeResourceProviderNotFound` for scheme `compute`: canonical resource reads
are planned before the compute artifact/interface exists. Existing interface-aware
`plan_compute_read` already defines the sample shape and telemetry contracts; the
canonical mixed route does not reach it. Earlier particle tests did not contain a
coordinator sample read, so their passing result did not exercise this dependency.

Executable product witness (C worktree):
`python3 scripts/smoke-canonical-document-bundle-browser.py --fixtures
/private/tmp/mech-syntax-qualification/browser-bundle-fixtures --served-compute`.
The positive probe starts the shipping server then exercises WasmDocument edits;
it currently fails at server compilation, before any browser assertion. Baseline
log: `recovery-evidence/c-configured-compute-edit-baseline.log`.

Finite compiler acceptance: scalar and nonsquare matrix sampled outputs use the
compiled interface's exact shapes; documented telemetry paths retain their types;
unknown sample/telemetry paths reject; ordinary provider reads remain provider-owned;
rooted imports and compute initializers keep their existing shared authority.
The same retained document must be partitioned once, compute and its initializers
compiled first, and coordinator compilation completed with interface-derived read
schemas. No second parser, guessed scalar schema, or eager runtime host is permitted.
Focused regression target: `canonical_mixed_resource_planning` on branch
`codex/syntax-s8r25-compute-read-planning`. The required C product continuation is
changed-kernel execution, retained inline identity, and failed-edit rollback.


### G28 follow-up — tuple sample ports and producer publication

The actual EKF product at C `8542fa7f5` gets past provider planning but fails
`source-semantics/unknown-published-binding` for `result.0`. The resource path is
a flattened compute-interface port, not a lexical document binding. Passing
that full port name into the semantic frontend's requested binding publication
confuses the two identities. The owning boundary remains R25's mixed compiler
handoff, supported by the canonical semantic frontend; no new PR is needed.
The actual product log is `recovery-evidence/c-r25-ekf-cpu-single.log`.

Strict minimal acceptance added before a production correction:
`canonical_mixed_resource_planning::canonical_mixed_tuple_sample_paths_publish_their_producer`.
It covers implicit result and named tuple producers, including nested tuple ports;
assertions preserve the exact retained leaf names and coordinator schemas. The
correction must retain the existing exact port validation, rejecting nonexistent
leaves even when their producer exists, and must not retain every sibling merely
because one leaf was requested. Source/decoded interface identity remains required.
This expands the explicit G28 sample-shape acceptance to the existing tuple-port
contract exposed by the shipping EKF application; it does not alter that contract.


## G29 — Canonical EKF artifact does not lower to the portable compute kernel (proposed R26)

**Owner:** the canonical artifact-to-portable-fixed-shape lowering contract shared
by E8 mixed planning and E9 compute execution. This is downstream of R25's
interface/read planning and distinct from G02's finite resident numeric-kind
capabilities. C/R23 retains the complete configured CPU/WebGPU product acceptance.
No corrective PR exists yet, and this finding does not enlarge R25.

At C tree `2590d7872fb12f0e66cab6be614e2a4172e7f9bf`, after R25's provider and
tuple-port corrections, the shipping single-lane CPU EKF reaches portable kernel
lowering and is rejected. Elementwise lowering reports 128 diagnostics; generic
fixed-shape lowering reports 22. Demonstrated classes include canonical
`matrix/matmul` operation IDs where the lowerer currently dispatches
`matrix/multiply`, range/access selector nodes and non-f32 selector schemas,
Bool integrity results rejected as ordinary storage, derived concat/broadcast
sources that are not materialized, and the transactional integrity contract.
These are one boundary mismatch exposed by one complete configured region, not
128 independent defects. The earlier semantic-only test
`canonical_mixed_shipped_ekf_region_compiles` stops before either portable
lowerer and therefore did not qualify this handoff.

Executable witness on the accumulated C tree:

```sh
MECH_BIN=target/debug/mech \
MECH_EKF_COMPUTE_BACKEND=cpu-scalar \
MECH_EKF_FILTER_COUNT=1 \
MECH_EKF_RESULT_FILE=/private/tmp/mech-syntax-qualification/c-r25-tuple-ekf-cpu-single.json \
bash scripts/smoke-served-resident-ekf-browser.sh
```

The exact failure is preserved in
`recovery-evidence/c-r25-tuple-ekf-cpu-single.log`. It proves that R25 now hands
the configured artifact to the backend; it is not passing EKF evidence.

Finite acceptance is the checked-in `ekf-batch @compute` section through the
same canonical mixed compiler and portable lowering API, with exact source and
decoded-artifact operation/interface identity. All selection, transpose,
concatenation, matrix product, 2x2 solve, comparison/logic, retained state and
three integrity predicates must lower without a second operation-name map or
source rewrite. A rejected predicate must roll back state and publication. The
shipping one-lane CPU probe must pass first; then the existing full CPU scalar
and WebGPU continuity/parity matrix in `.github/workflows/ci.yml` must pass on
the same accumulated C head. Existing unsupported-target negatives remain
errors and are not converted into accepted success.
