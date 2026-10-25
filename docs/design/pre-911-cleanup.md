# Pre-#911 cleanup ledger

This cleanup is stacked on #910 at
`bbd66acb9b3beacdc95356084df259ca172c8b98`. The refreshed #911 head is
`e070bec0f24ce78cb192d0c10380197bbc927376`. Neither stack is to be merged by
this work. #911 must remain on its existing base until the cleanup is qualified.

## Maintained owners

- Engine `native_scalar` owns scalar kinds, operator identities and signatures.
  Resident binder witnesses remain private. Native lowering and emitters use
  that vocabulary directly; the previous private/public enum conversions and
  duplicate signature matches are removed.
- Core `__mech_for_each_exact_binop_runtime_factory_for_type` owns the concrete
  binary factory grid. Canonical specialization and aggregate installers adapt
  its callbacks, preserving ordering and feature predicates. `VDRD` and `RDVD`
  remain independently linkable declarations and stay outside the established
  82-entry canonical/installer surface; the full grid remains 84 entries.
- Mech retains candidate build, generated-launcher, tamper/rebuild/recovery,
  runtime closure and hosted correctness gates. A compact local harness executes
  original/decoded × secured/trusted, compares complete output, validates report
  identity/phase/lifecycle, checks pins and compares accounting across all four.
- `mech-lang/benchmarks/tools/canonical-hosted-runner` owns reusable full-input
  and staged orchestration and its tests. `tools/benchmark-runner.toml` pins its
  revision. The CI verifier uses a sparse checkout of maintained tooling;
  ordinary Mech correctness tests have no archive/runner dependency.

Fast scalar execution is a primary maintained capability. The existing scalar
entry remains until a canonical Program scalar specialization demonstrates
equivalent correctness, latency and allocations. The initial diagnostic found
that the general Program Value interface has substantially higher invocation
cost; output equivalence does not authorize replacing the efficient entry.

## Coverage transfer

| Original selector/material | Obligation | Surviving selector/location | Evidence |
| --- | --- | --- | --- |
| `tests/ci/test_ci_full_contract.py::test_benchmark_status_keeps_unique_keys_and_both_cli_records` | Historical PR903 receipt, duplicate-key rejection and both publication records | Benchmark archive `test_historical_receipt.py::HistoricalReceiptTests::test_benchmark_status_keeps_unique_keys_and_both_cli_records` | One test executed successfully; all 78 receipt blobs match original Git objects |
| `program_runtime::setup_work_diagnostic::hosted::complete_secured_knucleotide_original_and_decoded_setup_debit_diagnostic` | Whole-source original/decoded hosted lifecycle, finite work, atomic denial and memory release | `canonical_hosted_artifact`: `scaling::complete_secured_knucleotide_handles_hundred_and_four_hundred_bases_without_lifting_limits`, `activation_node_work_denial_observes_nothing_and_releases_all_native_storage`, `phased_linear_native_activation_is_not_replayed_and_shares_the_final_work_limit` | Current-candidate execution pending; diagnostic source and observer hooks frozen in archive |
| `current_complete_source_selected_library_inventory` | Printed selected-site metadata; no executed behavior | Frozen `source/hosts/gpu/tests/support/canonical_hosted_scaling.rs`; adjacent executable scaling gate retained | Print-only diagnostic retired |
| `canonical_hosted_full*.rs` tests (23 cases) | Strict configuration, duplicate JSON, pins/mutation, four-way accounting, finite policies and failed/partial receipts | Pinned maintained runner, same test names; plus `runner_and_candidate_pins_are_required_before_dispatch` | 24 tests passed; CLI compiles |
| `full::comparison_control` in ignored linked policy gate | Four actual current-candidate executions before timings | Same ignored selector `linked::benchmark::complete_original_and_decoded_knucleotide_same_binary_policy_controls`, local `driver::qualify` | Actual generated-launcher execution pending; zero samples and no timing loop retained locally |
| Binary factory declarations/canonical/installer lists | Exact feature closure, callback order, 84 declarations / 82 installers, hygienic independent factories | `binop_factory_inventory::canonical_installers_keep_order_and_exclude_only_dynamic_cross_orientation`, `stdlib_macro_hygiene` and existing linkage gates | Reduced dynamic matrix/vector profile: both tests passed; complete shape grid pending |
| Scalar arithmetic/casts/control tests | Checked arithmetic, exact floats, errors, atomic publication, finite limits, resident/source/decoded agreement | Existing `canonical_artifact_native`, `canonical_artifact_i64`, `canonical_artifact_f64`, `canonical_artifact_cast`, `canonical_artifact_match`, `canonical_artifact_iteration`, `canonical_artifact_program` | Execution in progress; no predecessor cases deleted |
| #910 record compaction/reuse cases | Small buffer boundaries, separators/markers, CRLF/EOF, aliases, pointer reuse, interruption and I/O errors | Existing `hosts/gpu/src/sequence/runtime.rs` tests, unchanged | Execution pending; maintained implementation untouched |

The archive is available at immutable benchmark commit
`91f8a97a7f1af30b451a7864d68f767002505f0d` under
`archive/compiler/mech-pre-911-bbd66acb9/`. Its verifier passed for all 101 source
files against the exact original Mech Git objects before originals were removed.
Archive replay does not qualify the cleanup candidate.

## Execution and wire boundaries

Current CI still selects generated launchers and their negative controls. Runner
unit tests moved with their implementation and are explicitly executed by the
full linked-applications and native-plan hosted jobs. The factory inventory is
selected by the core ownership contract and managed-memory job. No existing
job or finite runtime resource limit is weakened or raised.

The old full/staged environment options now belong to the standalone benchmark
runner. Current Mech qualifiers reject them with migration instructions and
export `release-qualifier-results.json` and `small-launch.json` for the maintained
runner after completing their current-candidate controls. Production launchers
and runtime-only dependency closure remain unchanged.

Scalar enum wire names, variant order, identity tags, native image versions and
machine ABI are unchanged. Only type ownership and signature lookup changed.
Private validated proof objects, U128/I128 decoding and canonical `.mecb` are
untouched. #911's template ownership work remains downstream.

## Remaining work before completion

Finish optimized scalar measurements and current-head validation, including
actual ignored linked/hosted/policy execution and the complete linkage surface.
The scalar/Match/Iteration API families and legacy source-native paths have not
been retired: capability transfer and the scalar specialization must preserve
performance first. No single-owner retirement audit or downstream restack has
been qualified yet. This ledger deliberately does not claim completion of the
larger retirement objective.
