# Gate E2 closeout

This is the authoritative status source for PR #758,
`refactor/e2-consolidate-resident-architecture`. The PR remains stacked on
`codex/e1-delete-legacy-executor`; neither PR is merged or retargeted here.
The E1 base used for this closeout is
`73384717505b8e561509de21cd17e76f93176a5a`. The final E2 head and ordered
commit list are recorded in the PR description after the final push, because
the commit containing this document cannot name its own SHA.

## Governing boundary

Retained hosts are libraries and product capabilities. E2 supplies the
smallest resident semantic owner required by each retained host and removes
interpreter plumbing around it. No interpreter or fallback route is restored.

The retained external-host picture is:

- timer and time: resident observations;
- browser DOM: resident observation plus deferred effect;
- terminal env: driverless resident observation captured as a String input
  fact;
- terminal stdout/stderr: scalar String, at-most-once, after-commit effects;
- console and scene: resident after-commit effects;
- robot arm: custom resident effects whose declared command operation is
  preserved end to end.

## Reconciled decisions

- `--time` remains removed. Its interpreter-era `Cycle Time:` output is not a
  resident profiling contract. Any replacement is future structured
  observability work after F0.
- The unreachable CLI diagnostic renderer, Ariadne dependency,
  `WatchPathFailed`, and uncalled resource-registry helpers remain deleted.
- Browser DOM is retained. Its signed-envelope compatibility word remains
  fixed and authenticated through v0.4.
- `RuntimeLimits.max_memory_bytes` is advisory until allocator integration.
  `max_source_bytes` remains enforced at the current source boundary.
- Activation-scope context sends remain fail-closed with
  `ActivationScopeContextSendUnsupported`; top-level addressed sends are
  retained.
- `RuntimeBuilder::host_function` remains a direct embedding API. Source-callable
  host libraries require compiler-visible contracts; arbitrary registered
  callbacks are not made source-callable in E2.
- Existing product and generic resident tests jointly own String, f64 packet,
  and matrix observation conversion. No synthetic product host is added only
  for shape coverage.
- `mech build` retains multiple source roots in caller order. Bytecode builds
  accept one bytecode input and reject mixed source/bytecode inputs.
- Robot commands remain strict custom operations (`move`, `grip`, and `home`);
  generic `write` is not accepted as an alias.

## Reconciled E1 follow-up inventory

The historical E1 follow-up bullets now have these dispositions:

- `max_memory_bytes` accounting: future allocator/profiling work; not a gate.
- activation-scope context sends: deliberately unsupported in v0.4.
- runtime-registered host callbacks in source: direct embedding API only; not
  source-callable.
- `RuntimeExecutionMode` and planning builder: resolved. Compiler-only
  `build_compiler()` owns planning and attaches no drivers.
- non-f64 and matrix input coverage: resolved at retained generic/product
  owners.
- browser DOM: resolved in E2.
- terminal env/stdout/stderr: resolved in E2.
- static inventory refresh: completed in E2.
- bare runtime source test closure: completed in E2 without broadening
  `source_default`.
- dead-code warnings: not a gate; only mechanically unreachable code may be
  deleted, and warnings are not suppressed to hide reachability.
- ordinary `OutputMustBeStateBacked`: resolved by `SlotRole::Output`.
- multiple source roots: resolved and retained.
- engine integration required features: resolved.
- matrix example required features: resolved.
- generated native `MechProgram` route: resolved by the resident loader.
- full source/bytecode fixtures using `MechProgram` as a runtime: resolved by
  compiler plus resident loader.
- direct resource-binding facade: resolved by deletion.
- interpreter-shaped runtime query fallback: resolved by resident query
  ownership.
- robot generated fixture: resolved in E2 by custom operation preservation.
- terminal value metadata drift: resolved in E2 by inventory regeneration and
  coordinate/fingerprint re-anchoring.

## Phase ledger

### E2 — completed in PR #758

| Scope | Disposition | Permanent proof |
| --- | --- | --- |
| Robot custom operation preservation | Completed; exact path declarations override wildcard declarations, conflicts fail closed, and `write` is never accepted as a robot command alias | Source artifact, decoded bytecode, provider rejection/state, and generated-native robot tests |
| Terminal/current value-system inventory reconciliation | Completed mechanically at the existing `string-snapshot` destination | Inventory `--check`; exhaustive checker has only `[C0-GATE-B-EVIDENCE-STALE]` |
| Runtime and syntax feature-test ownership | Completed without making `source_default` imply resident routing or broadening empty syntax defaults | Four runtime profiles and two syntax profiles |
| Dynamic-module supported profile | Completed on `distribution-standard dynamic-modules`; excluded providers use isolated offline lockfiles and locked builds | Complete `scripts/test-dynamic-modules.sh` session |
| Exact PR metadata | Completed after the final push | PR body names exact base/head and ordered commits |
| Required current validations | Completed in the final closeout run | Commands and outcomes below |

### E3 — migration scaffolding only

| Authorized scope | Required action | Prohibited expansion |
| --- | --- | --- |
| D3/D4 migration checkers | Delete or rewrite stale checkers and repository-contract/CI assumptions that require deleted intermediate interfaces | No product semantics or execution changes |
| Experimental actor migration surface | Atomically remove build-local feature, actor bytecode/native fixtures, frozen projections, generator clauses, and checker expectations | Do not retain a partial fixture closure |
| Migration-era CI ownership | Simplify workflow/path ownership only after permanent resident product checks are authoritative | Do not weaken permanent checks |
| Migration-only value projections | Retire coordinates/projections only where a permanent current-inventory/value contract remains | Do not remove current value-integrity guards |
| E1/E2 logs | Archive after references are updated | Do not reinterpret the signed browser compatibility word |

### F0 — final correctness and evidence

| Required scope | Required action | Exception policy |
| --- | --- | --- |
| Gate B/D evidence | Controlled refresh against the exact final stack | No benchmark-number refresh in E2/E3 |
| EKF fault injection | Correct the two assertions or the implementation they expose | Must be resolved, not waived |
| Engine/root bytecode integration | Resolve any failure that reproduces unchanged on the current E1 base | If it does not reproduce, fix the E2 regression |
| Exhaustive product validation | Run standard/full source, bytecode, generated native, WASM, browser, CLI, host, capability, replay, effect, and packaging suites | No skipped workflow may be called passing |
| Legacy reachability | Prove zero shipping interpreter/fallback reachability | No compatibility fallback |
| Browser reproducibility | Pin browser, driver, and toolchain where required | Record exact versions |

### Future — explicitly non-blocking

| Scope | Boundary | Gate status |
| --- | --- | --- |
| Resident profiling/observability | Structured compilation, activation, capture, execution, publication, effect, commit, and delivery measurements | After F0; not an E2/E3/F0 blocker |
| Allocator-integrated memory budgets | Integrate actual allocation accounting with `max_memory_bytes` | Future; current limit remains advisory |
| Activation-scope context sends | Propose semantics separately if desired | Deliberately unsupported in v0.4 |
| Arbitrary runtime callbacks in source | Define compiler-visible source integration separately | Direct embedding API remains supported |
| Additional shape-coverage hosts | Add only for product need, not solely to manufacture tests | Existing generic/product coverage is sufficient |

E3 must not add product semantics, change resident execution or bytecode,
modify/delete hosts, refresh benchmark numbers, weaken ordinary correctness
tests, remove the browser compatibility word, or remove current production
routing/value-integrity guards.

## Mandatory-stop record

- Attempted scope: propagate source context capabilities through general
  `RuntimeContextBinding` and `context_send`. Action: `apply_patch` against the
  runtime authorization path. Error: automatic risk review rejected a broad
  security-sensitive capability expansion. Base reproduction: not applicable;
  this was a proposed E2 design, not a failing base command. Retained owner:
  capability admission. Disposition: the runtime target was left unchanged;
  work continued with a compiler/artifact-only requirement transformation.
- Attempted scope: preserve custom operations through interpreter context
  metadata. Action: `apply_patch` against compiler/interpreter metadata. Error:
  automatic risk review rejected extending interpreter-shaped state. Base
  reproduction: not applicable. Retained owner: compiler-only source planning.
  Disposition: the target was left unchanged; the final fix post-processes the
  compiled requirement table, remaps instruction indices, finalizes artifact
  and bytecode, and gives planning the same source-declared operation map.
- Attempted scope: replace the computed terminal stdout source fixture with a
  console fixture to avoid the terminal String boundary. Action: `apply_patch`
  against `standard_host_source_planning.rs`. Error: automatic risk review
  rejected weakening retained terminal product coverage. Base reproduction:
  not applicable; this was a proposed fixture edit. Retained owner: terminal
  scalar-String resident effects. Disposition: the fixture was left on CLI
  stdout and changed to String concatenation, preserving the producer-reuse
  assertion and the terminal contract; robot and feature-ownership work
  continued independently.
- Attempted scope: run the required root-workspace matrix command. Command:
  `cargo +nightly-2026-03-03 check --locked -p mech-matrix --example
  steady_state_benchmark --no-default-features --features
  compiler,f64,matrixd,vectord,matmul,solve`. Error: `cannot specify features
  for packages outside of workspace`. Base reproduction: yes; the current E1
  base has the same root package topology and `mech-matrix` is not a workspace
  member. Retained owner: `machines/matrix/examples/steady_state_benchmark.rs`.
  Disposition: workspace topology was left unchanged. The materially
  equivalent pinned, locked, offline manifest-path check passed with an
  isolated lockfile, and all independent E2 validation continued.

Neither stop restored interpreter behavior, weakened a retained product test,
deleted a retained host, or moved required E2 work to a later phase.

## Current E2 proof

- Runtime production code has no interpreter/fallback route and
  `RuntimeExecutionMode` is absent.
- Compiler-only construction attaches no input drivers.
- Ordinary outputs are resident-owned.
- Browser and terminal hosts are resident-owned with source/bytecode parity,
  capability denial, and rejected-turn isolation.
- Robot `move` remains `move` through source planning, artifact construction,
  bytecode decoding, admission, provider preparation, and generated native
  execution.
- Bare runtime and syntax feature profiles are valid, and dynamic modules use
  the supported explicit profile.
- Current inventory, migration classifications, and frozen semantic targets
  match the E2 tree. `[C0-GATE-B-EVIDENCE-STALE]` is the only remaining
  value-system checker failure and is assigned to F0.

## Validation record

- Formatting and patch hygiene passed:
  `cargo +nightly-2026-03-03 fmt --all -- --check` and `git diff --check`.
- Terminal ownership passed: `cargo +nightly-2026-03-03 test --locked -p
  mech-terminal --all-features` (1 unit and 12 provider tests) and the
  `distribution-standard` `mech_cli_host` target (25 tests).
- Generated-native ownership passed: the full `mech-build` project generator
  target passed, including the robot-arm case; the focused generated robot
  case also proved that the provider receives `move`.
- Runtime feature ownership passed with 363 `runtime_default`, 438 `source`,
  488 `source_default`, and 571 `source_default,resident-routing-source` tests.
  Syntax ownership passed with 11 empty-profile and 14 `base` tests.
- Source/artifact/bytecode ownership passed: four standard-host planning
  tests, 16 engine bytecode-topology tests, and the root
  source-to-bytecode-to-native canary. Robot source artifact and decoded
  bytecode requirements both name `move`; all eight robot provider tests pass,
  including unchanged state after rejection.
- The matrix example passed the pinned, locked, offline manifest-path check
  described in the mandatory-stop record. The literal root `-p mech-matrix`
  command is not reported as passing.
- `bash scripts/test-dynamic-modules.sh` passed using the explicit
  `distribution-standard dynamic-modules` profile and isolated offline
  lockfiles for excluded providers.
- The value inventory `--check` passed. The exhaustive value-system contract
  reports only `[C0-GATE-B-EVIDENCE-STALE]`; no occurrence, frozen-target,
  matrix-adapter, classification, inventory, or immutable-growth failure
  remains.
- Permanent architecture checks passed: standard/full distribution contracts,
  distribution packaging, native-host catalog, production resident routing,
  unsafe boundaries, runtime-factory safety, value-execution boundary and its
  16 unit tests, bytecode-v1 format (21 fixtures), and the serial static
  distribution profile.
- Browser ownership passed at its surviving owners: 53 browser-provider tests,
  six compatibility tests, three focused WASM DOM source/bytecode/denial and
  rejected-candidate tests, all 31 native `mech-wasm` `browser_project` tests,
  the `wasm32` browser-project check, and the shipped document-shim checks. The
  deleted root `browser_dom` wrapper target is not described as a passing
  product test.
- A selected GitHub CI run is required after the final push. The full GitHub
  validation workflow has not run at this pre-push point and is not described
  as passing.
