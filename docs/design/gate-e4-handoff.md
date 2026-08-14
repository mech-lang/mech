# Gate E4 hardening handoff

E4 is the mandatory pre-merge correctness gate stacked on the final E3 head.
This document is a work queue, not authorization to create the E4 branch or
pull request during E3.

## Gate boundary

E3 removes migration scaffolding and reconciles permanent CI ownership. E4
fixes retained product and correctness failures. F0 refreshes evidence and
performs release qualification only.

No E1, E2, E3, or E4 pull request may merge before E4 satisfies the acceptance
rule below. E4 must not restore interpreter execution, add a compatibility
fallback, delete a retained host, weaken a checker, waive a retained test, or
regenerate benchmark evidence merely to obtain compilation.

The exact E2 comparison base is
`f16e6b5d8a01b226353aba914c7c8ac1fcc90e31`. The first E3 full run was
`31826043920`; it checked GitHub's synthetic merge commit
`022fdc3416160ca204e305cdbba1c7f6264f382a`, so it is discovery evidence and
not exact-head qualification. Corrective run `31835368918` checked exact
Commit 6 `75e359b3673b86338b4b8e2438a1e84b7f042f1e`: all 19 selected checks passed,
and the full graph completed 84 jobs with 60 successes, 23 retained-product or
product-contract failures, and one skipped aggregate path. The final E3 docs
head is recorded in PR #760 after Commit 7 is pushed.

## Active work queue

### A. Resident-external validation closure

- Commands:
  `cargo +nightly-2026-03-03 test --locked -p mech-runtime --test
  resident_external_gate_d3 --no-default-features --features
  runtime_bench_gate_d3` and
  `python3 scripts/generate-d2-contract.py --check`.
- Error: `runtime_bench_gate_d3` selects resident-external code without the
  resident-routing symbols (`ResidentRouteFailureClass`, `route_failure`, and
  active-program ownership). Compilation then reaches a non-exhaustive match
  for `ResidentExecutionError::InvalidOutputMaterialization` in resident
  recording.
- E2 reproduction: yes, on the exact E2 base.
- Permanent owner: the retained resident-external Gate D integration test and
  D2 contract generator.
- Acceptance: both commands pass; the preserved source fixtures execute through
  resident routing; no Gate B or Gate D measurement is regenerated simply to
  make the code compile.

### B. Bytecode producer/runtime boundary

- Command: `bash scripts/check-bytecode-compiler-boundaries.sh`.
- Discovery: a prior qualification attempt reported `mech-bytecode` in a
  runtime-only graph. The current local E3 tree passes the complete boundary
  script, including engine/runtime-only graphs and the isolated producer and
  consumer. The next exact-head full run decides whether an active platform or
  workflow graph remains.
- E2 reproduction: the graph manifests are byte-for-byte unchanged from E2;
  the strengthened E3 checker is not. Any exact-head recurrence must therefore
  record the precise `cargo tree` chain before changing a feature.
- Permanent owner: compiler-enabled source/producer profiles and runtime-only
  bytecode decode/resident execution profiles.
- Acceptance: `check-bytecode-compiler-boundaries.sh` passes in full CI;
  compiler profiles retain the producer; runtime-only profiles retain decoding
  and execution without `mech-bytecode`, `mech-core/compiler`, or
  `mech-engine/compiler`.

### C. Bytecode-v1 source-generator closure

- Commands:
  `cargo +nightly-2026-03-03 run --locked --manifest-path
  tests/fixtures/bytecode-v1-generator/Cargo.toml -- --write` and the same
  command with `--check`.
- Error: the retained synthetic-live source compiler reports module manifest
  `test-live` is not registered. The generated live application also rejects
  `synthetic-live-read.mecb` as `invalid type: map, expected a sequence`.
- E2 reproduction: yes. E2 first fails the removed actor source at
  `actor/state/id`; invoking the retained synthetic-live source reproduces the
  `test-live` failure independently.
- Permanent owner: the bytecode-v1 source generator, 20-fixture corpus, and
  native live application.
- Acceptance: `--write` and `--check` both succeed; all 20 committed fixtures
  reproduce byte-for-byte; the actor fixture remains absent; the retained live
  fixture executes.

### D. Generated fixed-matrix native closure

- Command: `cargo +nightly-2026-03-03 test --locked -p mech-build
  --all-features --test native_generated_end_to_end -- --nocapture` (the
  failing owner is `generated_native_fixed_matrix`).
- Error: the isolated generated engine closure lacks exact bool and `Vector2`
  type/shape support used by assignment factory macros, producing 152
  `FunctionRuntimeType`, `MatrixBool`, and `Vector2` errors.
- E2 reproduction: production engine/build sources are byte-for-byte unchanged
  from E2. A second complete E2 generated build was not run after the earlier
  reproduction consumed 74 GB; this limitation must remain explicit.
- Permanent owner: fixed-matrix generated-native application planning.
- Acceptance: the fixed-matrix end-to-end test passes with a sufficient,
  minimal generated feature set; an indiscriminate full engine profile and an
  interpreter route are not introduced.

### E. WASM source distribution closure

- Command: `bash scripts/check-static-distribution-profiles.sh wasm-source`.
- Error: `WASM source forbids 'mech-combinatorics v'`.
- Dependency chain: `mech-wasm/browser_project` directly selects
  `combinatorics_default`, which selects
  `mech-stdlib/combinatorics_n_choose_k` and the optional
  `mech-combinatorics` package.
- E2 reproduction: yes, with the same command on exact E2. The WASM manifest is
  byte-for-byte unchanged from E2.
- Permanent owner: the retained browser source distribution profile.
- Acceptance: the profile contract passes without broadening the allowed WASM
  surface; browser source compilation and resident execution remain green.

### F. EKF fault-injection correctness

- Command: `cargo +nightly-2026-03-03 test --locked -p mech-engine
  --no-default-features --features 'compiler_default,resident-artifact' --test
  resident_ekf_program_execution -- --test-threads=1`.
- Error: the injected arithmetic failure is not observed, and the
  `AlwaysChanged` policy does not increase reported `dirty_nodes` as asserted.
- E2 reproduction: yes. The exact E2 command passes 9 tests and fails these
  same 2 assertions. The identical failures were also recorded on E1 head
  `8cdd5b513154c579c5e09aa7e0140183dab4f700`.
- Permanent owner: resident EKF execution, fault observation, and dirty-node
  policy accounting.
- Acceptance: both assertions are resolved according to the real execution
  contract; ordinary trace, abort/integrity isolation, two-buffer storage, and
  source/bytecode execution remain green; neither failure is ignored.

### G. Root bytecode integration correctness

- Command: `cargo +nightly-2026-03-03 test --locked --test bytecode`.
- Errors:
  `ordinary_set_elements_round_trip_through_bytecode` reports
  `ReadModifyWriteSchemaMismatch`;
  `tuple_source_constant_is_encoded_by_bytecode_v1` reports
  `MissingInstructionRole`; and
  `outer_join_option_columns_compile_through_bytecode_v1` reports
  `UndefinedKind`.
- E2 reproduction: yes. All three focused commands fail on exact E2 with the
  same `ReadModifyWriteSchemaMismatch`, `MissingInstructionRole`, and
  `UndefinedKind` classifications while surrounding retained bytecode
  assertions pass.
- Permanent owner: root source-to-bytecode compilation and bytecode-v1
  round-trip execution.
- Acceptance: the complete root bytecode target passes; no test is deleted or
  weakened and bytecode v1 remains the sole pre-launch format.

### H. Full source factory surface

- Command: `bash scripts/check-static-distribution-profiles.sh full-source`;
  focused reproduction:
  `cargo +nightly-2026-03-03 test --locked -p mech-stdlib
  --no-default-features --features full_source --test profile_contracts
  selected_source_matches_the_frozen_source_surface -- --exact --nocapture`.
- Error: runtime factory count is 9,011 while the frozen exhaustive surface is
  9,010.
- E2 reproduction: yes, on exact E2 with the focused command.
- Permanent owner: the exhaustive full-source function catalog.
- Acceptance: identify the additional factory and either remove unintended
  reachability or deliberately update the permanent surface with architectural
  evidence; `full-source` and `full-compiler` then pass.

### I. Reduced CLI, browser, and host feature closures

- Commands:
  `cargo build --bin mech --no-default-features --features run`;
  `cargo test --lib --no-default-features --features run
  cli::commands::run::tests`; and the Project browser feature-boundary commands
  in `.github/workflows/ci-full.yml`.
- Error: the reduced graphs omit retained resident-routing symbols,
  `source_catalog`, terminal-provider ownership, source loading, execution-info
  APIs, and resident turn outcome types. The Project browser graph similarly
  omits routing APIs used by `mech-wasm`.
- E2 reproduction: yes. Bare root `run` and the focused
  `browser_project_runner` WASM command fail on exact E2 with the same missing
  resident route, execution-info, and root-program APIs.
- Permanent owner: supported CLI run and browser project feature profiles.
- Acceptance: every retained reduced profile either closes its actual resident
  product surface or is replaced in product documentation by one explicitly
  supported profile without deleting its test coverage.

### J. Reduced core `no_std` closure

- Command: `cargo check -p mech-core --no-default-features --features no_std`.
- Error: `snapshot/data.rs` cannot find `Vec`; `execution.rs` and snapshot code
  cannot resolve `to_owned` under the reduced prelude.
- E2 reproduction: yes. The exact E2 command produces the same six missing
  `Vec` uses and four missing `ToOwned` resolutions.
- Permanent owner: the retained reduced core/no-std profile.
- Acceptance: all three reduced core commands in Cargo language tests pass
  without broadening the profile to a default/full build.

### K. Retained native and browser product assertions

- Native command: the Project native analog-clock smoke command in
  `.github/workflows/ci-full.yml`.
- Native error: resident activation fails at node 14,
  `core/composite-pack`, with `UnsupportedConstruction`.
- Browser command: `wasm-pack test --headless --chrome src/wasm
  --no-default-features --features browser_project,set_union,set_element_of`.
- Browser error: 15 of 16 tests pass; the inline encoded-document test expects
  production admission to reject a document that is currently admitted.
- Hosted command: `cargo +nightly-2026-03-03 test -p mech --features build
  --lib cli::app::tests::build_`.
- Hosted error: the full-validation run reaches the multi-root success
  assertion and unwraps `MultipleRootsUnsupported`. A focused exact-E2 replay
  currently stops earlier because `module_execution.rs` still calls
  `module_runtime_config` with five arguments after the helper was reduced to
  four. Both failures belong to the same retained CLI/build test closure; E3
  changes neither helper nor root policy.
- E2 reproduction: the hosted focused target reproduces the obsolete helper
  call on exact E2. The analog-clock and browser failures were observed in the
  first full-validation discovery run and their production owners are
  unchanged from E2; exact-E2 focused replays remain required in E4. None is
  an E3 checker or workflow-name defect.
- Permanent owners: native project activation, WASM resident document
  admission, and hosted resident build root policy.
- Acceptance: each behavior and assertion agree on one retained v0.4 contract,
  with product-level tests preserved and no fallback route.

### L. Windows and native-live qualification

- Commands: the standard/full Windows package jobs, Windows serve contract,
  and `cargo +nightly-2026-03-03 test -p mech-build --features
  standard-hosts --test registry_generated_project -- --nocapture`.
- Errors: both Windows executables build, then `package-distribution.py` exits
  nonzero while its wrapper suppresses the child diagnostic; the reduced
  Windows serve profile cannot find `mech_stdlib::source_catalog`; native live
  rejects the stale synthetic fixture as a map where bytecode v1 expects a
  sequence.
- E2 reproduction: packaging requires a Windows reproduction; the live fixture
  and reduced-profile failures use product surfaces unchanged by E3.
- Permanent owners: release packaging, Windows serve, and native live bytecode
  execution.
- Acceptance: the packager preserves its child diagnostic and succeeds for
  standard/full packages; Windows serve compiles and tests; the regenerated
  20-fixture corpus makes native live green.

### M. Native-linkage summary reconciliation

- Commands:
  `python3 scripts/check-native-linkage-coverage.py surface full` followed by
  all 13 extended surface shards and
  `python3 scripts/check-native-linkage-coverage.py merge`.
- Error: every exact surface generates successfully, the `full` surface
  validates 9,010 linked entries, and the merger then reports
  `tests/architecture/native-linkage/coverage.json` stale.
- E2 reproduction: the linkage catalog, owner manifests, engine factory
  sources, and frozen summary are unchanged from exact E2. The E2 workflow
  requested invalid shard `standard`, so it stopped before the permanent
  merger could expose the stale summary. E3 changed only that CI request to
  the checker-owned `full` shard.
- Permanent owner: the complete full-plus-extended native-linkage inventory,
  signature invariants, and exact closure shards.
- Acceptance: generate the exact report diff, relate it explicitly to the
  9,011/9,010 full-source disagreement, and reconcile only the proven product
  surface. All owner surfaces, the merged summary, and every exact-closure
  shard must pass; do not refresh the snapshot merely because the corrected
  matrix finally reaches it.

## Mandatory acceptance rule

E4 is complete only when:

```text
selected PR CI passes
full validation passes
all retained product tests pass
all distribution profiles pass
all source/bytecode generators pass
all generated-native owners pass
all browser/WASM owners pass
all EKF and root-bytecode correctness owners pass

The only controlled remaining exception is:
C0-GATE-B-EVIDENCE-STALE

That exception is accepted only through the narrow CI option added in E3.
The strict checker must still expose it for F0.
```

No E1-E4 pull request may merge before this state is reached. Every additional
exact-head product failure must be appended with its exact command, exact
error, exact E2 reproduction result, permanent owner, and acceptance test;
“make CI green” is not an actionable entry.

## Evidence-only F0 scope

F0 contains only:

1. Refresh controlled Gate B evidence on the exact E4-complete stack.
2. Refresh controlled Gate D evidence where required.
3. Remove the controlled stale-evidence option from the CI invocation.
4. Prove the strict value-system checker is fully green.
5. Pin browser, driver, Rust, wasm-pack, and other validation tools.
6. Run final release, packaging, distribution, source, bytecode, native,
   WASM, browser, host, capability, replay, and effect qualification.
7. Produce the final zero-interpreter/fallback reachability proof.
8. Record the exact final SHA and immutable evidence manifests.

F0 is not the first owner of any known product correctness fix.
