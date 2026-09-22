# Recovery continuation after audit acceptance

## Live landing checkpoint — 2026-09-22, R19 published; R20 next

**Candidate:** the designated accumulated deleted-parser C is still draft #830
and has not been restacked or qualified on the current owner heads. The protected
target remains `integration/v0.4`; this is a continuation checkpoint.

**Completed:** R16 was published at `e56df7ed4` on #871 with 46/46 focused
semantic checks; three later findings are deferred to the next sealing pass by
owner direction. R17 was rebased and published at `be33847b5` on #873; its six
review findings were fixed, answered, and resolved, with 26/26 completion
checks and the bytecode format check passing. R18 was rebased onto R17 and
published at `8c6010684` on #874. All seven existing R18 review threads were
answered and resolved. Its focused semantic and completion suites passed 62/62
and 26/26, including downstream output readiness and explicit admission
rejection for unsupported nested suspended FSMs. The corrected R18 head also
passes the runtime compile check. No repeat review was requested.

**Completed next:** R19 was rebased onto exact R18 and published at `c13333ce9`
on #876. Its three remaining review notes were fixed, answered, and resolved:
ordinary matches reject sampled-input patterns, sampled computation follows its
activation owner, and recorded initial publication replays with activations
dormant. Focused checks pass 70/70 semantic and 26/26 completion tests, plus
replay, activation reuse, sampled-capture, and initial-publication runtime
checks. No repeat review was requested.

**In progress:** move R20 onto exact R19, then fix its own notes and conflicts.

**Remaining:** continue upward one PR at a time through R26, addressing each owner's
notes and conflicts, with focused product checks after coherent batches.
Scratch rebases currently reach R26, but those heads are not sealed or
published as the accepted stack. Restack the single C candidate after the owner
heads, run exact-head full qualification and review, merge through the protected
path, and verify the landed `integration/v0.4` result. Return to deferred R16
findings during the later sealing pass. Checkpoints are not completion.

**Status:** Checkpoint—not complete.

## Live landing checkpoint — 2026-09-22, R16 review repair

**Candidate:** the designated accumulated deleted-parser C remains draft #830 on
the R26 stack. This record is a continuation checkpoint, not qualification of C
or permission to merge. The protected target remains `integration/v0.4`.

**Completed:** the stack has been restacked through R14. Published exact heads
are R12 `c7a852b62`, R13 `6cf874391`, and R14 `f41df10ea`; R16 is the active
owner repair on top of R14. The R16 semantic suite passed 46/46 after addressing
the existing review findings for scalar pattern functions lifted over matrix
elements, declared output coercion per arm, and refutable enum payload coverage.
Repository formatting passed on the uncommitted repair. The bytecode corpus
regeneration exposed one further compile gap in runtime diagnostics for the new
shape-preserving comprehension kind; that owner fix is in the R16 worktree and
the isolated generator is being rerun. This is not an exact-head seal.

**Remaining:** finish deterministic corpus regeneration and frozen hash updates,
run the bytecode format and determinism seals, commit and publish exact R16,
request a new review, resolve any actionable findings, then rebase R17 and
continue upward one owner at a time. Run focused product checks after coherent
batches. Assemble the accepted stack into the single C candidate, qualify its
exact head with full protected CI and review, merge through the protected path,
and verify the landed `integration/v0.4` result.

**Current action:** finish the R16 isolated corpus build and seal; do not treat
the prior 46/46 semantic result as full distribution qualification.

**Next action:** publish and review exact R16 after its checks pass, then address
the existing R17 review notes on a rebase over that exact accepted head.

**External blocker:** none for R16 fixes, tests, review, or routine restacking.

**Status:** Checkpoint—not complete.

## Live landing checkpoint — 2026-09-17

**Candidate:** `codex/syntax-s8c-cutover` /
`90825ae35add8d0030831428ccf983808528d071`, published as draft #830 on
R26 `45eeb5dbb8e43c8b44057cef335c94e19867f402`. This is the designated combined
qualification candidate for the complete R01–R26 correction stack and targets
`integration/v0.4` through the protected stacked path.

**Completed:** the four review findings on preceding C `71ff3eec6` are fixed,
answered, and resolved. Static publication and WASM share the retained
`BrowserDocumentPayload` / `fromServedDocuments` contract; encoded transport
routes preserve logical document specifiers; comment-only evaluated rich output
retains a non-visible fallback; and document mounts retain their captured output
identity after console submissions. The preceding owner failures are corrected:
direct comment semicolons, ordered-root scope reuse, and numbered compute-region
names now have exact regressions.

Canonical native planning now consumes artifact requirements directly, validates
resource ownership and write payloads through the trusted providers, and avoids a
synthetic turn for read-only artifacts. Exact local evidence passes 397 root
compute-enabled tests, 128 syntax tests, 682 runtime tests, eight canonical host
planning tests, one fixed document-mount test, and three served-document tests.
Time and timer reads, scene writes, and the robot custom send build and execute as
generated native applications. The first eleven native graph generation cases
also passed before the now-corrected read-only planning failure. Formatting,
patch integrity, resident-routing, warning-policy, interactive-architecture,
compiler-planning-quarantine, and unsafe-boundary checks pass. The PR has zero
current unresolved review threads.

**Remaining:** obtain review of exact C `90825ae35` and complete protected CI on
that SHA. The local machine has no Node or Chrome executable, so the JavaScript
bootstrap and browser product probes remain assigned to CI. Inspect every actual
failed job, fix genuine failures in their owner, propagate the changed candidate,
and requalify it. Once exact-head review and qualification are clean, merge the
stack through the protected path and verify the landed target.

**Current action:** restore the missing GitHub check suite for the published exact C head, request review of that SHA, and monitor the early native plan and browser compute/product jobs that failed on the preceding head.

**Next action:** inspect the first completed exact-head jobs. If a genuine failure
appears, add its reproducer and fix it once in the owning C correction; otherwise
continue through full qualification and protected merge verification.

**External blocker:** none for review, CI triage, routine fixes, rebasing, or the
established protected merge path.

**Status:** Checkpoint—not complete.

## Live landing checkpoint — 2026-09-15

**Candidate:** `codex/syntax-s8c-cutover` / `daf69ffff909aa9c17bfa69e08718c8a5bf2664c`, published as draft #830 on native stack #859 and targeting `integration/v0.4` through R09 #858. The accepted correction chain is R01 #842, R03 #843, R04 #844, R05 #847, R25 #849, R08 #854, R07 #856, and R09 #858.

**Completed:** R09/G06 implementation content is `f357c0823`: schema-directed retained comprehension values now cover every canonical packed scalar kind and closed structural snapshots; pattern descent moves owned children, Dynamic elements unwrap for structural matching, parameterized binding drafts retain their source shape, packed outputs exclude standalone wrapper/root costs, and nested Set/Map finalization is preflighted. All five R09 review threads were answered and resolved. Evidence passes 67/67 canonical document-state tests, 11/11 lexical-collection tests, both completion witnesses, and 3/3 focused ownership/accounting tests.

R25 exact head `45680890c` now registers focused validation for R08 #854, R07 #856, and R09 #858. The selector/full-contract suites pass 50/50. That policy is propagated through exact heads R08 `fe7f8f02d`, R07 `e0840e3ef`, R09 `4b4291831`, and C `daf69ffff`. C remains the sole landing candidate and retains full qualification. Superseded R09 run 35042390841 and C run 35042537922 were canceled after their newer exact-head runs appeared.

Exact C execution at the current implementation content passes 67/67 canonical document-state tests and the real deleted-parser source product probe: 14 catalog cases plus the rooted source canary. The registry-only parent propagation does not change production code; the new exact C CI run owns remote qualification of `daf69ffff`.

**Remaining:** exact-head reviews and focused checks for R25/R08/R07/R09; accepted R10/G09 through R20/G16 prerequisites; R24/G27 Index ranges; R26/G29 canonical compute lowering; R21–R23 retirement, browser, and distribution closure; unresolved R02/G02, R06/G17, and R15/G12 decisions; one stable exact-C full qualification; protected merge and post-merge verification. No capability has been dropped or converted into an accepted rejection.

**Current action:** monitor and immediately address notes on R25/R08/R07/R09 while the focused checks run. R10/G09 canonical structural match patterns is the next dependency-ready implementation owner after R09 and will use its accepted tuple/array/tag binding, guard, fallthrough, and exhaustiveness cells.

**Next action:** confirm the selector marks #854/#856/#858 as review-only and C as landing; fix any actual failed focused job in its owner once. Then create the R10 slice on exact R09, implement G09 without widening R09, run focused source/decoded acceptance, request review, and merge the accepted batch into C for another product probe. R11/G07 composed controls follows R10.

**External blocker:** none for routine implementation, testing, review response, rebasing, or integration. The recorded R02/G02, R06/G17, and R15/G12 contract decisions remain explicit future blockers for their dependent work only.

**Status:** Checkpoint—not complete.

The review of `d25fbaad01f69c077cb26d4cb0aeadcb4dd1b117` accepts the reconciled
planning baseline and exact-preserving extraction. It does not seal replacement
implementation or accept any silent reduction in required behavior.

The original S8B remains frozen at `662d29b79`. The eleven extraction PRs
#831–#841 retain their original frozen-head exact final-tree preservation proof.
Their current heads now also inherit the shared CI coordination overlay; the
original whole-tree proof is not claimed for that changed tree.
The original audit observations remain evidence of that frozen baseline; they
are not rewritten as results from a corrective head.

## Confirmed corrective work

R01 / G03 production code is implemented at `7ffeea89f`; head `60e5d534c`
adds explicit feature-enabled CI execution. It is published in
[PR #842](https://github.com/mech-lang/mech/pull/842), targeting E11. Configured
canonical calls and imports now use the existing FunctionEnvironment. The
correction includes per-root callable isolation and the explicit EKF fixture
import required by its ModuleOnly catalog exports. Catalog-free semantic type
probes retain their existing declaration-only behavior; configured production
calls cannot use that fallback.

Validation: 714 runtime library tests, 76 engine integration tests and six new
visibility tests pass; the reduced engine source build and formatting checks pass.
The implementation review at `7ffeea89f` returned with no major issues. The latest-head review
at `60e5d534c` also returned with no major issues; full CI remains required before closure.
No open review threads were present on #831–#841 when R01 was published.

R03 / G25 is implemented at `d0eab6087` in
[PR #843](https://github.com/mech-lang/mech/pull/843), stacked on R01. The binder
uses the canonical schema builder for the union and relocates all schema and
constant references, including lexical match/comprehension declarations.

Validation: 42 strict binding tests, six visibility tests, ten declaration-handoff
tests and 103 engine tests pass; the reduced source build, compiler quarantine
and R3 architecture checks pass. The runtime library run passed 713 tests and
hit one filesystem-watch timeout in the sandbox. The same already-built test
passed outside the sandbox; the sandbox retry failure is retained as evidence.
Recorded outputs are in recovery-evidence/. The review at `d0eab6087` returned
with no major issues; this is not a whole-replacement seal.

R03 executes the previously untested F15 binding/error/remap witnesses, Q30 Id,
Q32 generic Enum, and Q31 Index identity and zero-index validation. Q31's range
endpoint positive remains blocked by the new demonstrated G27 resident
prerequisite, recorded before implementation in
[RECOVERY-FINDINGS.md](RECOVERY-FINDINGS.md). Its strict positive test remains red
on the audit branch. This finding does not reopen G25's schema-order mechanism.

R04 / G26 is implemented at `19abda12c` on
[PR #844](https://github.com/mech-lang/mech/pull/844), stacked on R03. The
review at `19abda12c` returned with no major issues. It preserves already-Dynamic
identity and wraps concrete payloads once. Three regressions exercise changed
payload schemas, concrete wrapping and preservation of existing nested depth.
All 714 runtime library tests, 45 binding tests, six visibility tests and ten
declaration-handoff tests pass. Thus all 31 original strict schema-audit cases
pass across the R03/R04 corrections. Repository formatting and 23 CI contract
tests also pass. This does not close the separate Index range prerequisite or
other replacement obligations. The earlier independent R04 full-CI requests were superseded by the combined C
qualification policy described below; canceled work is not passing evidence.
Exact-head [Full CI run 34993367803](https://github.com/mech-lang/mech/actions/runs/34993367803)
is queued; the ordinary PR CI run is 34993111754. These pending workflows are
not passing evidence. The reduced engine source build also passes at R04.

## Scope decisions still required

The accepted baseline does not choose the G02/G17/G18 target capability floor or
the six G12 constrained-type decisions. Their positive capability witnesses and
owning review boundaries remain open. Missing semantic families remain required
prerequisite work under their assigned owners. There is no new generic audit,
blanket exclusion, or resumption of catch-all development on the original S8B PR.

## Landing coordinator checkpoint — 2026-09-15, batch 1

**Candidate:** `codex/syntax-s8c-cutover` /
`873dad50cc439779df572f26d986553d8f276699`, published on #830. Local worktree:
`/private/tmp/mech-syntax-s8c-qualification`. GitHub native stack #846 preserves
S0–S8A, replaces frozen B's landing position with its existing E1–E11 review
boundaries, then R01 #842, R03 #843, R04 #844 and C. Final target remains
`integration/v0.4`. Frozen B #828 remains available as the original comparison.
C now targets R04 and GitHub reports no merge conflicts.

**Completed:** CI policy `164379213` is pushed through every E/R slice and C.
Exact registered review identities run affected owners and explicit regressions;
unknown paths retain conservative validation. C retains full qualification,
normal PR gates, browser/application checks and an exact-head deleted-parser
source product probe. All 45 Python CI tests pass. No duplicate manual full
dispatch was started. Obsolete runs 34984227048, 34993111754 and 34993367803
were canceled only after preserving completed logs; their final archives are
retained under `/private/tmp/mech-syntax-qualification/s8-ci-archive-20260915`.
Pagination captured all 127 jobs on the frozen B failed run. C's previous remote
head is preserved at `archive/s8c-before-qualification-20260915`.

R01/G03, R03/G25 and R04/G26 are assembled unchanged in C. Exact C execution:
6 visibility + 45 constant-binding + 10 declaration-handoff tests pass; the real
source-runtime fixture passes its 14 catalog cases and rooted source canary
with `parser.rs` and `document/lower/legacy` physically absent. Runtime library:
713 pass, 2 fail. This is combined evidence, not final qualification. The earlier
local command that omitted the routing feature executed zero tests and is excluded
from passing evidence. The feature-enabled run is retained in recovery-evidence.

The B/E11 certification import violation was fixed once in R03 by moving the
unchanged binding regression to canonical_source_review. Both the regression
and the unchanged canonical-authority gate pass. Current R01/R03/R04 heads are
`f8c8706b8`, `3563dc2fb`, `98d673856`; prior clean production reviews remain
recorded, and fresh reviews were requested after confirming zero open threads.
The old/new SHA mapping and combined logs are in recovery-evidence.

**Remaining:** All accepted gaps other than the demonstrated G03/G25/G26
corrections remain open for their assigned acceptance. Current C runtime failures:
`interactive_program_output_is_the_final_statement_without_a_fenced_output`
and `resident_matrix_comprehensions_feed_mutable_vertical_concatenation`.
Inspect their actual failure causes before changing implementation or expectations.
Deleted-parser engine test consumers, browser/application probes, certification,
exact-head full qualification, required review and protected merge remain open.
Canceled/skipped work and prior heads are not qualification of this candidate.

**Current action:** Investigate the two actual C runtime failures against their
accepted owners; prepare R05/G04's dependency-ready repeated-selection correction
in `/private/tmp/mech-syntax-s8r05-updates` on R04. Only this new corrective
branch is actively changing; existing corrections are under review.

**Next action:** Extract the two runtime failure bodies from
`recovery-evidence/c-batch1-runtime.log`; add and execute strict mixed/nested
occurrence-order regressions in R05's existing canonical_document_state suite.
Continue real C browser/application probes while the focused slice CI/reviews run.

**External blocker:** None for the current batch; the user approved native-stack
re-linking and it completed. The previously recorded G02/G17/G18 capability floor
and six G12 semantic decisions still require explicit disposition before their
dependent implementations. No capability has been dropped or converted into an
accepted rejection. This checkpoint is not complete and does not authorize merge
until exact-candidate qualification and review requirements are met.

## Current landing checkpoint — reviewed CI rollout

**Candidate:** `codex/syntax-s8c-cutover` /
`90defdf9ed0599316d7fb0d8f19e0d2ac36b35f9`, published to #830 on native stack
#846, targeting R04 `c8edc05f0`; ultimate target `integration/v0.4`.

**Completed:** The first batch remains integrated. CI review #831 found two
policy defects: unreachable `None` registration for E7 and missing minimal
configuration coverage. Both are corrected in `062bdddf1`, propagated through
all slices, with 47 passing CI tests and 39 passing source-only configuration
tests. The third note (standalone fixture lockfile) was already fixed in C; all
three received replies and were resolved before the next review request. R03's
focused CI explicitly runs its relocated regression and the unchanged authority
guard. Every restacked production diff and review commit count was checked.

At preceding C `109c55242`, the exact rerun passes 61 combined regressions and
the real source product fixture (14 catalog cases plus rooted canary). The
deleted-parser browser product builds successfully; actual Chrome CPU and WebGPU
publication both produce the expected matrices across two turns. Two static
project tests and the browser submission lifecycle probe also pass. Runtime
remains 713 passing / 2 failing: factorial reaches G19/R16's missing pattern
function body; mutable comprehension state reaches G05/R07's activation
classification. The expectations remain positive and unchanged.

C's standard/full distribution snapshots now account for removing nom@8 and
nom-unicode. Only dependency counts and their surface hashes changed; selected
hosts, operation features, runtime factory/specializer counts and semantic
surface digests remain identical. Both distribution contract checks pass. This
is a retirement expectation correction in C, not a capability exclusion.

**Remaining:** The accepted open gap queue, R05 completion, C's two runtime
failures, remaining browser/application and retired-test consumers, required
reviews, final exact-head full qualification and protected merge. At exact C 90defdf9e,
the combined suites execute 6 + 45 + 10 passing tests, the locked source fixture
reports 14 catalog cases plus rooted checks, and actual Chrome CPU/WebGPU
publication passes two turns for both backends. Runtime executes 713 passing /
2 failing tests with the same G19 and G05 failures. All 47 CI contract tests pass.
These focused results are not final full qualification. The single current PR CI
run is 34999106483; obsolete run 34997626903 is canceled. The exact-head standard/full distribution rerun also passed; its output is
retained in recovery-evidence/c-policy-final-distributions.log.
The obsolete 109c55242 CI and earlier slice runs were archived before the
reviewed rollout and cancellation requests; canceled work remains unqualified.

**Current action:** R05/G04 is published in #847 at `92fc1acb1`, based on
R04 and not yet incorporated into C. Both original wrong-result witnesses now
produce 14. Its 36 state regressions and 20 memory-runtime tests pass, including
nested/mixed occurrence order, per-occurrence canonical conversion, rational
power and failed-turn rollback. The reduced source build, both distribution
contracts, architecture checks, formatting and 47 CI contracts pass. Its exact
PR identity registers focused state/memory CI. Implementation review is pending.

C 90defdf9e CI has four actual failed jobs, with logs preserved in
recovery-evidence/c-90def-failures. Their owners and executable witnesses are:
- C/G23 fixture retirement: `cargo metadata --locked --offline --manifest-path
  tests/fixtures/native-build-owner-runner/Cargo.toml --format-version 1` fails
  because its lock still includes the deleted parser dependencies. Regeneration
  removes 36 lines only; no dependency upgrades or capability changes.
- CI coordinator: the C-only standalone fixture runs offline without fetching
  its separate lock graph; clean CI lacks aho-corasick 1.1.5. E1 now stages an
  explicit locked fetch before builds, with an ordering regression.
- C/G22 presentation: warning policy rejects conditional allow in
  canonical_presentation.rs. The owning presentation correction removes that
  suppression while preserving feature builds.
- C/G22 served browser product: actual particle probe exits at server startup.
  Root is reproducing it and inspecting server output; the failing smoke remains
  required and has not been replaced by a passing isolated compute probe.

Eight current C review notes are owned in G22/C, grouped by shared cause:
canonical source rendering must publish browser output mounts and documented shim
slots; raw formatting/docs must not demand completed results; CLI declaration-only
context source must not be mistaken for a slash-containing path; REPL must retain
one terminator; browser bootstrap must use the current canonical document for
source replacement and decoded initialization. Existing/new strict witnesses are
being corrected in their shared rendering and canonical session authorities.
Two workers own presentation and WASM/session fixes; root owns CLI/CI/fixture fixes,
integration and qualification. R05 is under review. No additional corrective PR
has been opened for these already-owned C corrections.

**Next action:** Finish the current CLI classification build, run the actual
particle product server probe, finish the two C review-owner corrections, then
run their focused product/regression tests. Address and resolve every C thread
before requesting its next review. Collect R05's independent and PR reviews;
fix demonstrated notes before accepting it into C. Propagate the tested shared
CI dependency-fetch correction once through the stack, integrate accepted R05,
and publish the next combined C SHA with actual product results. G05/R07 control
initializers remains the next dependency-ready semantic obligation after this
batch. Existing G19/G05 runtime failures remain positive and open.

**External blocker:** None for current accepted work. GitHub native-stack
re-linking was explicitly approved and completed. Outstanding capability/semantic
decisions remain as previously recorded, with independent accepted work continuing.
Checkpoint—not complete; coordinator owns qualification and verified protected landing.


## Publication checkpoint — requested review handoff

**Candidate:** `codex/syntax-s8c-cutover` / `ef57626067ec9abd9ac89d252632651ee6af572e`
on #830. It remains based on R04; R05 is not yet accepted or integrated.

**Completed:** All current work is pushed in separate reviewable commits:
C CLI source classification `8f0131426` (8 tests pass), renderer/shim correction
`5ed789941` (36 renderer tests pass), explicitly WIP canonical WASM controller
`7a724e6ad` (browser_compute_canary check passes with warnings denied), fixture
lock/server failure logging `adc04495b`, CI fetch integration `ef5762606`.
The CI fix is owned in E1 #831 at `a37c97dcc` (48 contract tests pass).
R05 #847 is pushed at `a2a73653f`: its strict Boolean-mask coordinate witness
reproduced 20 instead of 22; the fix passes all 37 state and 20 memory tests.
The review thread received a reply and was resolved after publishing evidence.

**Remaining:** R05's returned row/column broadcast note is relevant and open,
with a response explaining its owner and next regression. C controller tests
still refer to retired parser/tree transport and need canonical fixture migration;
its positive decoded initialization, source replacement and browser probes are
not yet validated. No C review thread is claimed resolved by partial WIP work.
Actual served particle probe now exposes the startup cause: annotated compute
heading rendering fails with canonical source range unavailable. That witness
is being reduced in the canonical renderer owner (G22/C). Existing G05/G19
runtime defects, all other accepted backlog obligations, required reviews,
restacking the new CI overlay, stable exact-head qualification and protected
merge remain open. Canceled/skipped work is not qualification.

**Current action:** Root took over saved worker edits after their execution
limits. Reproduce/fix the particle heading and R05 broadcast, then finish
canonical browser controller regressions and reply/resolve C notes.

**Next action:** Inspect `c-particle-heading-baseline.log`, fix the shared
heading renderer, rerun the real served particle probe; implement the strict
R05 broadcast witnesses in its existing canonical_document_state suite.
Update current CI archives and cancel superseded heads, then restack accepted
fixes and publish combined evidence. Do not request C rereview while notes remain.

**External blocker:** Worker execution quota is exhausted; root can still
perform local accepted work. No new user authority is needed. The previously
recorded capability/semantic decisions still precede their dependent work.
Checkpoint—not complete.


Latest publication update: C is now `e79a62c38a02229d2ea9c0f7980b8f3ffa5042cb`.
The actual particle server error was reproduced by the retained annotated-heading
fixture and fixed in the shared heading renderer. All 37 renderer tests pass;
the full served browser probe must still be rerun with the rebuilt server.
C's CLI note received its evidence reply and is resolved. R05's mask note is
resolved; its mixed row/column broadcast note is still open. No rereview was
requested over those incomplete notes. The worker changes and evidence are all
published. The existing CI overlay is published on E1 and C; intermediate slice
restacking is the next integration-maintenance step, before accepting R05 into C.

Current action/next command: rebuild C's server with the heading fix, run
`MECH_BIN=/private/tmp/mech-syntax-s8-extraction-target/debug/mech python3 scripts/smoke-gpu-particles-browser.py --backend gpu --particle-count 16384 --software-adapter`;
then finish the R05 broadcast witness/fix and canonical browser fixture migration.
Archive current runs in `s8-ci-archive-review-handoff` and cancel only obsolete
heads. Exact C90def CI is completed failed; its complete paginated job list and
run logs are being preserved, not counted as qualification of this new head.
External blocker: worker quota as above; no new approval required. Checkpoint—not complete.


## Current batch checkpoint — live selection geometry and admitted browser products

**Candidate:** `codex/syntax-s8c-cutover` / `50ee7a23bbfb3dee04ea82a42a810ee59c38c6e7`
(#830), based on R04 `907aeb1e9`. R05 remains separate pending focused review;
this is not yet the requested R05-integrated C batch or a seal.

**Completed:** E1's base-target guard `0bd98f07e` is propagated through every
extraction/corrective branch and C. All 50 CI selector/contract tests pass on C.
Each restacked production tree was compared with its previous published head;
only the four shared CI-policy files changed. All remote refs were published
atomically with explicit leases. Registered review slices require their exact
staging parent; retargeting to integration, another branch, or an absent base
uses ordinary conservative validation. C retains full qualification.

R05 #847 is now `e7595eb85e4ba0264634d3206c9321fc8e0d6292` (production identical
to tested `f4e9847fb`). Broadcast fixes retain complete source/decoded matrix
checks and rollback coverage. Three returned notes were independently reproduced:
composite matrix field selection, turn-varying Boolean-mask broadcast geometry,
and inflated 1x1000 rectangular admission. The fixes use recursive composite
assignment, maintained type resolution without a discarded gather, live selector
geometry in the addressed kernel, and checked axis-capacity products. All **42
document-state + 20 memory-runtime tests pass**, including zero/one/two selected
populations across nonsquare row/column/rectangle cases. Compiler quarantine and
R3/R4 checks pass. Every returned R05 thread has an evidence reply and is resolved;
focused rereview is requested on the current head before integration.

C's admitted-product handoff is implemented in `0f2455395` and preserved through
the CI-only restack. The standalone locked/offline producer fixture passes **14
catalog cases plus the rooted canary** and emits actual canonical bundles. The
rebuilt shipping WASM constructor probe passes for plain and imported documents:
initial artifact revision equals emitted bytecode; step executes; reset restores
artifact/state; a source edit compiles a different artifact and advances retained
2 to 5; a different bundle resets to its own revision and initial 3; malformed
replacement/reset preserves accepted source/revision/output; stale transitive
source is rejected. The native producer check with compute_backends_native and
WASM browser-compute-canary release build pass. The new probe is wired into C's
existing browser qualification job, not a separate full dispatch.

**Remaining:** R05 focused review/acceptance and incorporation into C; native/WASM
owner tests still using deleted APIs; engine retirement tests; the actual particle
product's shared browser-planning `compute` provider failure; other unresolved C
notes; accepted G05/G19 runtime positives and all remaining gap-register items;
required stable exact-head qualification, reviews and protected landing. The
rebuilt particle probe at the previous production head got through heading
rendering but failed server startup (`RuntimeHostProviderNotFound: compute`).
Browser loading, backend selection, and full particle execution were therefore
not reached. The separate successful canonical compute/browser probes are not a
claim that the particle application passes. No positive obligation was removed.

**Current action:** Finish browser/engine test retirement in the existing owner;
collect R05 review and integrate accepted corrections. CI archives preserve all
available completed logs and paginated job metadata before cancellation of obsolete
runs. Current heads retain their ordinary PR workflows; no manual full dispatch
was added. Cancellations/skips are not passing evidence.

**Next action:** Run/migrate `cargo +nightly-2026-03-03 test --locked --offline -p
mech-wasm --no-default-features --features browser_compute_canary --lib`; retarget
its maintained assertions to canonical documents/bundles. Inspect R05's rereview,
fix any relevant notes, restack C onto accepted R05 and run combined visibility,
constant-binding, declaration-handoff and affected runtime regressions. Then
begin R07/G05 control-derived state initialization without a new planning round.

**External blocker:** Worker quota is exhausted; root continues accepted work.
No additional routine authority is required. Previously recorded capability
choices remain open before their dependent work. Checkpoint—not complete.


## Current checkpoint — R05 integrated, retired browser tests executable

**Candidate:** `codex/syntax-s8c-cutover` /
`8a94aa97e66406370383d4800d3dd0321b5693ec`, published on #830 and based on
R05 #847 `f118f0b17a6313e54b97319e0d987dcf9853f8c6`. GitHub reports MERGEABLE.
Native stack #848 contains S0–S8A, E1–E11, R01, R03, R04, R05, C. The prior
checkpoint's statement that R05 is outside C is superseded. R05 is assembled
for qualification; its latest review is pending, not accepted or sealed.

**Completed:** C `83fdffd6f` integrated R05 and passed 6 visibility, 45 constant
binding and 10 declaration-handoff regressions. R05's next review reproduced a
false change report when a dense F64 selected update changes signed-zero bits.
The correction compares physical dense output bits; a strict test inspects the
actual bound kernel report and source/decoded reciprocal values across three
turns for row, column and rectangle updates. R05 passes 43 document-state tests
and 20 memory-runtime tests. Its first memory invocation accidentally selected
a disabled test target (zero tests); that is excluded and the subsequent
full_compiler invocation executes all 20. All six R05 review threads have replies
and are resolved; focused rereview was requested on f118f0b17.

C's new migrated tests replace deleted parser/tree construction with canonical
SourceDocument and real admitted bundle fixtures. Initial/reset bundle activation
remains exact-bytecode; stale source/resolution checks remain enabled. Project
source loading uses the canonical interactive product so public root-symbol
queries retain their outputs. Canonical title rendering now handles CRLF as
well as LF; its full FizzBuzz regression passes. The WASM owner suite compiles
and executes: 53 pass, 3 fail, no ignored tests. WASM32 browser_project tests
also compile successfully. Fixture mistakes were reconciled against existing
canonical contracts: named foo scope does not replace root output; inert and
named fences are not root mounts; dependency failures come from canonical
graph compilation; source and bytecode comparisons use the same interactive
product. The remaining positives were not changed to rejections or disabled.

At exact C 8a94aa97e, all 43 document-state and 20 memory-runtime tests pass,
including the migrated resident literal/memory-release test. The exact-head combined runtime/renderer rerun passes 6 visibility, 45 constant
binding, 10 declaration-handoff and 38 renderer tests: 162 passing tests across
these six suites. The rebuilt, locked/offline deleted-parser product probe passes
all 14 catalog cases and its rooted source canary at this same C head. The
exact-head browser_project owner rerun confirms 53 passing and the same three
failing tests; no ignored tests. Six fixed C review threads have evidence
replies and are resolved. The source-replacement/compute thread remains open
with a progress reply. No C rereview is requested over that incomplete work.

**Remaining:** The three executed browser-owner failures are: (1) FSM
specification lowering, assigned to existing G14/R18; (2) fixed original
document output becomes console result 42 instead of root result 41 after
append, assigned to G22's capture owner; (3) an evaluated expression inside a
trailing comment has no canonical output (expected two visible outputs, actual
one), requiring canonical comment/output ownership reconciliation under G22's
consumer contract before implementation. All are retained as positive tests.
The actual particle application still fails server-side compute-provider
planning; no browser/backend execution result is claimed. R07/G05 `023c3b2b1bfedac5cdd2e0bf14415528506e4484` is published on
`codex/syntax-s8r07-control-initializers`, based on current R05, with two
strict baseline regressions demonstrating InitializerUnavailableAtActivation
for closed match/comprehension state producers, with no production patch yet.
All other accepted open gap-register items, target/semantic decisions, required
reviews, exact-head full qualification and protected landing remain outstanding.

**Current action:** Qualify this published batch while R05 rereviews; finish
G22 document capture and configured mixed-browser handoff, and implement G05
closed control initialization in its resident owner. Only R05, C and R07
are changing responsibilities. R07 has no new PR yet.

**Next action:** Resume R07 at `/private/tmp/mech-syntax-s8r07-initializers`:
implement closed control-node classification and activation execution using the
existing control-region and memory-budget authorities; retain the two strict
state initializer tests and add live-input/effect non-replay negatives. In C,
finish fixed original-document output capture and route browser mixed producers
through the existing mixed compiler; rerun the real particle product after that
coherent fix. Inspect R05's pending review before another request. No local
validation process is left running from this checkpoint. Current C automatic
qualification run is 35014211166; it was pending when checked, not passing.

The archived E11 failure (35008027930/job104515257514) is the already-known
canonical certification import-allowance violation: the frozen extraction still
has the snapshot-construction regression in canonical_source_semantics. R03
moved it to canonical_source_review, and current C has no such import in the
certification-owned file. Do not count the old extraction's failure as a new
C defect or weaken its authority gate. Earlier review-slice qualification still
needs that owner correction propagated if required before its protected merge.

**External blocker:** No additional routine permission is needed. Worker quota
remains exhausted; root performs accepted local work. The previously recorded
capability/semantic decisions remain open for their dependent obligations.
Obsolete C83fd run 35011917078 was confirmed completed/cancelled after normal cancellation then force
cancellation; all 29 job records and available logs were preserved first in `s8-ci-archive-signed-zero`; cancellation is not
qualification. No duplicate manual full dispatch was created. Checkpoint—not complete.


## Current checkpoint — shared static failure fixed, R05 review clean

**Candidate:** `codex/syntax-s8c-cutover` /
`ac4388d4f888e39f13e133ac22d1b88d07bb2166` (#830), based on unchanged R05
`f118f0b17a6313e54b97319e0d987dcf9853f8c6`. Automatic C qualification run
35015781991 is pending, not passing. This C commit changes only its production
routing guard and two guard tests; product code is identical to 8a94aa97e.

**Completed:** R05's implementation review at f118f0b17 returned with no major
issues at 2026-09-15T19:36:40Z. All prior threads are resolved. C's static
architecture and static distribution jobs both failed on the same stale
`load_root_program` string requirement, after the browser project had moved to
`load_interactive_root_program` to retain public root-symbol queries. The single
owner fix now requires that interactive production seam. A mutation regression
proves reverting to the ordinary root loader fails the guard. All 18 commands
from the local static-contract sequence pass, including repository formatting,
52 CI/guard unit tests, all R1–R6 contracts, bytecode and unsafe-boundary checks.
No product test was weakened or changed. The 162 focused Rust tests, 14 catalog
product cases plus rooted canary, WASM32 test compilation, and browser 53-pass/
3-fail results belong to the preceding product-identical 8a94aa97e head; they
are not mislabeled as full exact-head qualification of ac4388d4f.

R07 is now published at `50ae6a64e5b709e7787cec75b735bd19b69f88af` on
`codex/syntax-s8r07-control-initializers`. Two new tests (four fixtures, each
source and decoded bytecode) prove live input/state-dependent matches and
comprehensions must remain unavailable as persistent-state initializers. Both
tests pass. The original two closed-initializer positives remain red. No R07
production implementation or additional PR has been published.

**Remaining:** R07's activation implementation, C's three enabled browser
failures, real particle mixed-compute handoff, all other accepted open gaps and
required decisions/reviews, exact-head full qualification and protected landing.
C still has its compute/source-replacement review thread open; no premature
rereview was requested. R05 clean review does not seal the whole candidate.

**Current action:** Implement R07 closed control initialization through shared
resident control execution, preserving the live-dependency rejection boundary.
Continue C qualification and handle actual failures in their existing owners.

**Next action:** In `/private/tmp/mech-syntax-s8r07-initializers`, extend the
activation schedule to execute closed match/comprehension producers and their
closed dependencies once. The existing kernel-only ActivatedOnceNode cannot
simply relabel controls: build_plan, control local storage, execution and
activation-derived shapes must all be consistent. Reuse the current control
executor and budget accounting; do not add a second evaluator or replay effects.
Run both closed positives and the live/state negatives before publishing a fix.
Poll C run 35015781991, including paginated jobs, and preserve failure logs.

**External blocker:** None for the current accepted work. Existing scope
decisions and worker quota are unchanged. Obsolete run 35014211166 has all
29 job records and completed logs preserved in s8-ci-archive-interactive-seam;
normal cancellation left it queued, so force cancellation was requested.
Canceled/skipped work is not qualification. Checkpoint—not complete.


## Current checkpoint — rich comments restored as required behavior

**Candidate:** `codex/syntax-s8c-cutover` / published
`5a8efdf99a09ced4338c434da10174176a78a349` (#830), based on R05 #847
`99595861195f745a59d88ad5b5b70cf34510fd09`. The C worktree has unpublished
capture/comment changes; they are not qualification of the published candidate.

**Completed:** R05's module-layout follow-up moved `numeric.rs` to
`numeric/mod.rs` with identical contents and updated four architecture-test
references. The unchanged module-layout guard, 60 mutation tests, and engine
build pass. R05's review at 995958611 returned clean at 2026-09-15T20:11:03Z.
Current C static distribution passes. C run 35017217645 is the automatic combined
qualification run; no manual matrix was dispatched. Completed logs are being
preserved under `s8-ci-archive-rich-comments/35017217645`.

R07 is pushed at `94ca8e2b4` on `codex/syntax-s8r07-control-initializers`, based
on current R05. It now executes closed match/set-comprehension initializers
through the shared resident control dispatcher before state initialization and
outside turn topology. Its full state suite at c88f8a9db's pre-restack equivalent
reports 47 passing / 2 failing / 0 ignored. The closed match, closed set and four
live/state-dependency negatives pass. Both matrix-derived initializer positives
remain enabled: downstream transpose reports UnsupportedLayout; downstream scalar
access reports InvalidShape. These expose the existing G18/R08 layout prerequisite
in the combined G05 witnesses; R07 is partial and is not incorporated into C.
A new memory-limit sweep reproduced lost MemoryRuntime errors inside activation
controls. The 94ca8e2b4 correction and source/decoded ownership-release regression
pass; the test records the earlier failure at budget 4608 in
`r07-control-budget-baseline.log` and success in `r07-control-budget-fixed.log`.

C's unpublished G22 capture correction retains the original prefix result in the
canonical semantic graph, with a separate current interactive result. The existing
console-overlay regression passes, and a new test passes three successive console
submissions for both plain source and a root fence, preserving original result 41
while console values advance through 42, 43, 99. The first overly restrictive test
filter executed zero tests; only the corrected exact-name invocation is counted.

The user explicitly confirmed comments are rich Markdown paragraphs, including
inline `ans`, and asked us to consult the blog posts. The existing Phase 2B port
note says rich semantics awaited paragraph-element completion; that deferral was
never removed. Required contract references:
- https://mech-lang.org/post/2025-11-12-mechdown/ (paragraph markup, links, reactive
  inline expressions, inert double-braced code, comments using paragraph features)
- https://mech-lang.org/post/2026-05-11-version-0.3/ (line-local inline ans and
  selection/inspection integration)
This is an implementation omission, not an accepted inert-comment capability.
The canonical comment continuation now calls the shared paragraph-element owner;
its specification/dependency reports are updated. Comment inline evaluations are
published without replacing the preceding ordinary/fence result. The original
trailing-comment output-order regression passes. The broader rendering regression
is still running; rich rendering, named scopes, streaming, and generated-gate
qualification must be completed before this batch is accepted.

**Remaining:** C's actual CI failures include engine libtests still depending on
the deleted parser, runtime G19/G05 prerequisites, WASM G14 and the two G22 cases
being corrected, browser product failures, and native-plan failure whose current
log must be inspected. All other accepted gap-register obligations remain open.
Exact-head full qualification, required reviews, protected merge, and post-merge
verification remain outstanding. R02/G02 has no PR: its 25 positive numeric
capability witnesses remain open, not absorbed or excluded.

**Current action:** Complete shared rich-comment rendering and line-local ans
qualification in C while preserving R07's partial owner work.

**Next action:** Poll runtime renderer regression session 54405 and CI archive
session 56426; finish canonical rich-comment HTML/text rendering for root/named
scopes, run comment scalar-cut/fuel tests and the affected browser/runtime suites,
then publish the coherent C batch and reply to its remaining review thread.

**External blocker:** None for rich comments; the user resolved the contract in
favor of full paragraph semantics. Previously recorded unrelated capability/type
choices remain open. No test was disabled and no cancellation is passing evidence.
Checkpoint—not complete.


## Current published checkpoint — C capture and rich comments qualified locally

**Candidate:** `codex/syntax-s8c-cutover` /
`c15f3e7835e40df62df3d8001463923ded6a9f74` (#830), based on R05
`99595861195f745a59d88ad5b5b70cf34510fd09`. Worktree is clean. Automatic
exact-head C run 35020590724 is queued; no manual full dispatch was started.
The prior run 35017217645 has completed with failure, so there is no live obsolete
run to cancel. Completed job metadata/logs were archived before publishing.

**Completed:** C now preserves the original document result separately from the
console result in the canonical graph. Both the original fixed-output regression
and three successive overlays against plain/fenced source pass. Comment parsing
uses the shared paragraph-element continuation. Rich comments preserve markup,
links, inert inline code, evaluated expressions and line-local ans. Their inline
values do not replace ordinary statement/fence results. HTML/text rendering uses
the retained owner and execution scope, including named fences. The user explicitly
confirmed this existing rich-paragraph contract; the earlier raw-token deferral
has been removed, not treated as an accepted capability exclusion.

Executed evidence for the published tree: 128 syntax library tests; 33 canonical
incremental/root/scope/property/certification tests; 40 renderer tests; 22 static
and generated grammar checks; and the rebuilt deleted-parser product probe with
14 catalog cases plus rooted canary pass. The renderer regressions cover
one-shot and character-streamed comments in plain/root-fence/named code and live
ans updates across three turns through source and decoded bytecode. Browser owner
suite: 56 pass / 1 fail / 0 ignored. Its two G22 failures are corrected; the
remaining FSM-lowering positive still fails under G14/R18.

R07 remains separately published at `94ca8e2b4`, with its closed-control execution
and memory-error/release correction. Its two matrix-derived positives remain red;
no R07 work was silently incorporated into C. R05's exact latest review is clean.
C's open review thread received the published evidence reply:
https://github.com/mech-lang/mech/pull/830#discussion_r4019975819
It remains open for actual compute adoption; no premature rereview was requested.

**Remaining:** Two selected Mechdown parser-parity integration targets cannot
compile because they still import deleted parser/lowerer APIs:
`canonical_mechdown_closed_rules` and `canonical_mechdown_lowering_parity`.
Their blocked invocation and exact errors are retained in
`recovery-evidence/c-rich-comment-document-qualification.log`. They remain G23
retirement migrations, not disabled tests or successful qualification. The five
independent canonical targets were separately executed (33 passing tests).
Current C CI also demonstrates G23 engine libtest parser dependencies, G19/G05
runtime prerequisites, G14 WASM FSM lowering, n-body server readiness timeout,
and missing compute provider registration. The native generated CLI project fails
`NativeRuntimeConfigUnsupported`: configured host instances/grants have no build
plan resource requirements. That product handoff belongs to the existing native
qualification owner (G23), not rich comments or numeric binding. All other accepted
open obligations, required reviews, final exact-head qualification, protected
merge and post-merge verification remain outstanding.

**Current action:** Publish this coherent G22 batch and advance the existing
product/retirement owners while its single combined CI runs.

**Next action:** Rebuild shipping browser WASM from c15f3e783 and extend/run the
existing canonical bundle browser probe for fixed output plus rich-comment/ans
selection. Continue configured mixed-compute handoff in C; migrate the two recorded
Mechdown parity targets to canonical payload/acceptance witnesses without reducing
their fixture or rejection coverage. Poll run 35020590724 with paginated jobs and
inspect actual failures. R07's matrix layout dependency remains assigned to G18.

**External blocker:** None for this batch. Unrelated accepted type/capability
decisions remain open. Prior greens, cancellations, and blocked test targets are
not full qualification. Checkpoint—not complete.


## Current checkpoint — shipping rich Markdown verified; compute handoff in progress

**Candidate:** `codex/syntax-s8c-cutover` /
`8370ced411a8ff3597315344d6fbaeb748be605e` (#830), based on R05
`99595861195f745a59d88ad5b5b70cf34510fd09`. Single automatic exact-head CI
35022454727 was queued after publication. No manual matrix dispatch.

**Completed:** Published `ffe308dd6` adds the actual shipping Chrome regression
for six scenarios: original admission/edit/reset for plain and imported bundles,
fixed document result across three console submissions for plain and fenced
documents, and rich comments plus separate Markdown paragraphs in plain/fenced
documents. Links, emphasis, inert code, three live inline values across three
turns and retained selection pass. The browser used the rebuilt shipping WASM
from c15f3e783; the two following commits change tests only. The source fixture
producer still passes 14 catalog cases plus the rooted source canary.

Published `8370ced41` migrates the two blocked Mechdown parity targets to canonical
contracts: all 53 acceptance/prefix fixtures and all payload/malformed fixtures
remain. No retiring parser, alternate AST or compatibility layer is restored.
`canonical_mechdown_lowering_parity` is replaced by `canonical_mechdown_payloads`.
Seven full-feature canonical document targets pass 48 tests, zero failures or
ignored tests. An earlier invocation without Mika failed the unconditional Mika
scope fixture; the supported full-feature rerun is recorded separately, not
counted as an implementation failure or a passing reduced-feature result.
Review evidence reply: https://github.com/mech-lang/mech/pull/830#discussion_r4020122546

Prior C run 35020590724 was archived (31 paginated jobs; 30 completed logs available)
before its obsolete remaining work was cancelled. Archive:
`/private/tmp/mech-syntax-qualification/s8-ci-archive-rich-browser/35020590724`.
Actual owner results: runtime 697 pass / 2 fail (G19 and G05/G18); WASM 56 pass /
1 fail (G14). Engine test-retirement, native-plan configuration, browser readiness
and compute-provider planning failures remain. Cancellation is not qualification.

**Remaining:** All accepted open gaps and required reviews/qualification/merge
remain as previously registered. R07 remains separate at 94ca8e2b4. C's compute
thread remains open; no fresh review was requested.

**Current action:** C worktree now contains an uncommitted G22 browser-planning
correction. The pointer provider/ingress implementation is being moved unchanged
from WASM into the browser host owner so native planning and live WASM use one
contract. Static and served compilation share a canonical mixed-root bundle
helper; compute host settings are validated before deferring materialization to
the compiled region. Pointer injection is registered rather than dropped. Shared
placement discovery delegates to the existing semantic section authority. This
is not yet validated or published and is not passing evidence.

**Next action:** Finish local sessions 59895 (configured mixed browser tests),
51400 (shared pointer provider tests), 10837 (WASM compute compile check). Fix
actual errors, run served/static product regressions and the real particle probe,
and publish only the coherent owner correction with actual results. Confirm
ordinary documents still use the ordinary canonical route. Keep C review open
until configured compute adoption is demonstrated.

**External blocker:** None for this batch. Previously recorded unrelated
capability decisions remain outstanding. Checkpoint—not complete.


## Current published checkpoint — served compute product passes

**Candidate:** `codex/syntax-s8c-cutover` /
`4c901a1eaf8282cb807039513ce01ab0b3dd83b1` (#830), based on unchanged R05
`99595861195f745a59d88ad5b5b70cf34510fd09`. Worktree is clean. The single
automatic exact-head workflow is 35024471044 (in progress at this checkpoint).
No manual full dispatch. Prior workflow 35022454727 was cancelled only after
archiving its 31 paginated jobs and 29 then-available completed logs under
`/private/tmp/mech-syntax-qualification/s8-ci-archive-compute-planning/35022454727`.
Unfinished/cancelled jobs are not passing qualification.

**Completed:** Published two reviewable correction commits: `94e39e77d` moves
pointer ingress/provider contracts from WASM to the browser-host owner and keeps
pointer hosts/grants in browser authority injection; `4c901a1ea` gives static and
served browser products one canonical mixed-root compilation handoff. Compute
settings are validated before materializing the compiled region. Shared region
discovery uses the existing semantic section authority. Retained resolver records
now preserve executable source kind as well as exact bytes and dependency hashes.

The real served particle product now passes in Chrome/WebGPU with 16,384 particles,
advancing frames and pointer -> Mech CPU transaction -> compute inputs. The actual
shipping compute WASM also passes the CPU/WebGPU numeric oracle for both turns.
The six rich-document/capture/selection Chrome scenarios pass on this rebuilt
compute profile. The rebuilt deleted-parser source product passes 14 catalog
cases plus the rooted canary. Owner results: 3 browser-planning tests, 2 pointer
contract tests, 60 static-bundle tests, and 90 server tests (89 in the first run;
the sole localhost-listener sandbox denial passed as a one-test escalated rerun).
All 22 static/generated checks pass, including the 52 CI/routing guard unit tests.
No production test was skipped or changed to rejection. The first native server
build raced the generated WASM directory replacement and failed at include_bytes;
the completed shipping WASM was then built first and the server rebuilt successfully.
Those transient invocation/build failures are not counted as passing evidence.

**Remaining:** C's open thread PRRT_kwDOCJ6M-c6ioDtu still needs the direct
configured WasmDocument compute source-replacement acceptance check. The standalone
particle presentation is not a substitute for that path. Its source handoff uses
the candidate canonical document already, but source changes/retained output
identity must be exercised through the actual configured document controller.
No fresh C review has been requested while this remains open. Engine libtest
retirement (G23), runtime pattern-function G19, matrix initializer G05/G18, FSM G14,
native-plan configuration, browser standard readiness, all other accepted gaps,
required reviews, stable exact-head full qualification, protected merge and
post-merge verification remain. R07 stays separate at 94ca8e2b4 and is not in C.

**Current action:** Finish the direct configured-document compute edit acceptance
check while the new C workflow runs. The published batch above is a checkpoint,
not S8 completion or closure of every G22 obligation.

**Next action:** Inspect `src/wasm/src/project.rs` configured constructors and
`build_document_repl_runtime_for_document`; extend the existing browser bundle
probe/fixture path with a configured compute document, submit a changed compute
body through the document controller, and assert changed execution/output identities
plus failed-edit rollback. Use the existing canonical mixed-root APIs for any
required dependency handoff correction. Keep pointer/live-driver contract ownership
in `hosts/browser/src/pointer.rs`. Inspect paginated failed jobs from workflow
35024471044 as they finish; group failures by the already assigned owner.

**External blocker:** None for this batch. Previously recorded unrelated
type/capability decisions remain open. Checkpoint—not complete.


## Active checkpoint — configured document sample read reproducer

**Candidate:** unchanged published C `4c901a1eaf8282cb807039513ce01ab0b3dd83b1`.
**Completed:** rich Markdown shipping results and served particle evidence above
remain recorded. A direct configured WasmDocument edit probe now reproduces G28
at server compilation, before browser execution. Its failure and finite ownership
are recorded in RECOVERY-FINDINGS.md before production implementation.
**Remaining:** G28 prerequisite, C/G22 edit thread, all other accepted open gaps,
required reviews and final exact-head qualification/merge.
**Current action:** R25 canonical mixed resource planning correction on R05;
C holds only the browser acceptance probe. R07 remains separate and idle.
**Next action:** Run the strict canonical_mixed_resource_planning reproducer, then
stage compute interface construction before coordinator read planning. Integrate
subject to required review and rerun the actual configured document probe.
**External blocker:** none for this correction. Checkpoint—not complete.


## Current checkpoint — R25 draft review; C integrated locally

**Candidate:** published C remains `4c901a1eaf8282cb807039513ce01ab0b3dd83b1`;
local `codex/syntax-s8c-qualification` is `6ade888fc5f8dc93cc6b3a496174f8fced7be7a0`,
restacked on R25 `a4555e3e027b4a4a20404c858088ec9a56ac81dd`. R25 is draft #849.
The local C rebase resolved the retained-result-boundary argument at the staged
coordinator call; no parser or alternate lowering authority was restored.

**Completed:** R25's strict baseline reproduces missing compute read planning.
The correction passes 5 focused resource planning tests (including nonsquare
matrix and scalar schema/bytecode identity, telemetry types, invalid paths and
ordinary provider ownership), 18 existing mixed tests, 50 CI tests, formatting,
compiler quarantine and warning policy. The restacked deleted-parser C separately
passes the same 5 + 18 tests. Exact R25 review was requested by comment:
https://github.com/mech-lang/mech/pull/849#issuecomment-5688422721
No open threads existed at the request. The PR is draft; keep all active S8
extraction/corrective PRs and C drafts while reviewing/testing. Do not use ready
status to trigger reviews. No readiness or merge claim is made.

**Remaining:** R06 has no corrective branch; R07 is partial at `94ca8e2b4`, with
matrix positives blocked by R08/G18. R08–R20 have no published corrective branches
or PRs. R21–R23 responsibilities are being implemented/qualified in C and remain
incomplete. R24/G27 has a strict failing witness but no implementation. R02's
positive capability obligations also remain open. R25 is a new demonstrated
prerequisite, not evidence that lower-numbered boundaries were completed.
Required C browser-edit acceptance, review, stable full qualification, protected
merge and post-merge verification remain open.

**Current action:** rebuild shipping compute WASM for the local C candidate,
then rebuild the native server sequentially and run the configured WasmDocument
source-edit/rollback probe. R25 review is running. R07 is idle and separate.
The published C workflow 35024471044 is still live; actual completed failures
are preserved in `/private/tmp/mech-syntax-qualification/s8-ci-archive-current/35024471044`:
engine retired-parser test imports; runtime 697 pass / 2 fail (G19, G05/G18);
WASM 56 pass / 1 fail (G14). These are unqualified failures, not cancelled passes.

**Next action:** finish build log `c-r25-shipping-wasm-build.log`, build `mech`
with compute_backends_native, run `smoke-canonical-document-bundle-browser.py
--fixtures /private/tmp/mech-syntax-qualification/browser-bundle-fixtures
--served-compute`, inspect actual results and address the owning cause. Review
R25 notes when they return before any further review request. Publish C after
combined product evidence, preserving old CI logs before cancellation.

**External blocker:** none for current product acceptance; accepted scope/type
choices remain recorded for dependent work. Checkpoint—not complete.

## Published stack checkpoint — R25 linked before C

**Candidate:** `codex/syntax-s8c-cutover` /
`6ade888fc5f8dc93cc6b3a496174f8fced7be7a0`, published on draft #830.
Native stack #850 preserves all 27 PRs in their established order with
R05 #847 → R25 #849 → C #830, targeting `integration/v0.4`. GitHub's append-only
stack update rejected an insertion, so stack #848 was unlinked and the same PRs
relinked in order. C's base and R25 ancestry are verified; neither PR was marked
ready. All R01–R25 definitions now appear together in PR-STACK.md.

**Completed:** 5 new + 18 existing combined mixed tests pass on this C tree;
shipping compute WASM rebuilt successfully. The old C run's 31 paginated job
records and 30 available logs were preserved before publication; it is now
cancelled, not qualified. New exact-head workflow 35026816791 is queued.

**Remaining:** R25 review; direct configured browser edit/rollback probe; C's
other failing owners and the full accepted corrective backlog recorded above;
final exact-head qualification, protected merge and post-merge verification.

**Current action:** native server rebuild is running in exec session 42052,
log `/private/tmp/mech-syntax-qualification/c-r25-server-build.log`. The WASM
build is complete (its session 28716 exited 0). No second build should be started
until the live server build is polled.

**Next action:** poll session 42052, then execute the configured document probe
with `--served-compute`; inspect R25 review notes and resolve them before another
review request. Preserve the positive acceptance assertions and ownership.

**External blocker:** none for the current batch. Checkpoint—not complete.


## Current checkpoint — tuple sampled-port follow-up in product qualification

**Candidate:** published C `8542fa7f58c5fad7346c8dd204aa97439185712b`;
local C `4ef7a01dd108a89b8ae2e128ff41532a15bbd177`, rebased without conflicts
onto R25 `425578fa7`. Draft PRs remain on native stack #850.

**Completed:** The shipping configured timer/compute edit probe passes all seven
scenarios and is required by C's compute canary. C's final open review thread was
answered and resolved after actual evidence, then review requested on 8542fa7f5:
https://github.com/mech-lang/mech/pull/830#issuecomment-5688603153
R25's prior exact head had clean review and green focused CI. The larger EKF
product revealed a tuple-port/lexical-producer mismatch, recorded in G28 before
implementation. Its strict reduced witness fails on the preceding R25 head.
The follow-up preserves retained leaf identity and validates even unread declared
ports; implicit/named/nested tuple and decoded-interface regressions pass.
All 7 focused and 18 existing mixed tests pass, as does formatting.
Review requested after confirming no unresolved R25 threads:
https://github.com/mech-lang/mech/pull/849#issuecomment-5688639907

**Remaining:** follow-up review and actual EKF qualification, all accepted open
R02/R06–R24 obligations in the live queue, required C reviews, final exact-head
full qualification, protected merge and post-merge verification. Do not infer
completion from the higher R25 identifier. C workflow 35027739951 was live at
this checkpoint. Prior C workflow 35026816791 is cancelled after preserving
30 then-visible paginated job records and 25 logs; cancellation is not success.

**Current action:** shipping WASM build session 98693 in the local C tree;
log `/private/tmp/mech-syntax-qualification/c-r25-tuple-wasm-build.log`.
No native server build has yet been started for this new tuple-follow-up tree.

**Next action:** poll session 98693; after success, rebuild native `mech` with
compute_backends_native, run the configured document probe and the scalar single-
filter EKF product command from the prior checkpoint logs. Inspect the current
R25/C reviews and respond before requesting any additional review.

**External blocker:** none for this batch. Checkpoint—not complete.


## Current checkpoint — R25 bounded; R08 begins

**Candidate:** draft R25 #849 is
`codex/syntax-s8r25-compute-read-planning` /
`dff504ec2d7ddf2a9f8796d1714d6b5944ad294e`. Its tree
`ae09e4c2eff9896f65d9230e683bb4459e7f9119` is identical to reviewed content
head `425578fa7aef9d5395e43bb32889e8b49c330420`. Draft C #830 is
`codex/syntax-s8c-cutover` /
`45946479d98440bdc6aae3cf0b84e0170f0694ad`; its tree
`2590d7872fb12f0e66cab6be614e2a4172e7f9bf` is identical to locally tested
`4ef7a01dd108a89b8ae2e128ff41532a15bbd177`. The changed SHAs are ancestry-only
stack merges. Native stack #850 still orders R05 -> R25 -> C.

**Completed:** R25's tuple-port correction has a clean review at `425578fa7`, no
review threads, 7 focused + 18 existing mixed tests passing, and the same exact
tree at the current head. The rebuilt shipping configured document browser probe
passes all seven scenarios, including compute result 1 -> 3, retained inline
selection identity and malformed-edit rollback. The rebuilt shipping WASM and
native server both completed successfully. The actual EKF probe now passes both
R25 handoffs and reaches the next backend boundary. Its distinct fixed-shape
lowering failure was recorded before implementation as G29/R26, with the exact
log and finite acceptance in RECOVERY-FINDINGS.md. C and R25 cancelled-run logs
from 35027739951 and 35028355759 are archived under
`/private/tmp/mech-syntax-qualification/s8-ci-archive-restack`; cancellation is
not success.

**Remaining:** current exact-head focused R25 workflow 35028724829; current C
workflow 35028724674; C review completion; R08/G18 and R07/G05; R02/R06-R24 and
new R26 accepted obligations; full product checks; stable exact-head full
qualification; protected merge; and post-merge verification. The EKF product is
red at G29/R26, not R25. The earlier C run also preserves known R16, R18, R21/R23
failures and does not qualify those owners.

**Current action:** reproduce R08's two retained positive layout cells on a clean
owner branch based on current R05/R25 ancestry, identify the shared resident
layout cause, and add a strict owner regression before production correction.
R25 and C workflows continue without duplicate manual full dispatches.

**Next action:** implement the bounded R08 variable-cardinality layout correction,
run its focused positives and neighboring fixed-layout regressions, request review
only after all notes are resolved, then integrate the accepted result into C and
resume R07's remaining initializer positives.

**External blocker:** none for current R08 implementation. Recorded G02/G17/G18
scope decisions remain open where their exact target floor is required; no
positive obligation has been waived. Checkpoint—not complete.


## Current checkpoint — R08 review corrections integrated and natively stacked

**Candidate:** R08 draft #854 is
`codex/syntax-s8r08-variable-layouts` / `c9f6ba16746f265af943acb6a94d70a03a06a3d1`.
Accumulated C draft #830 is `codex/syntax-s8c-cutover` /
`6947bec97aa57df292a760697b28adc7dfacc148`. GitHub native stack #855 targets
`integration/v0.4` and orders R25 #849 -> R08 #854 -> C #830 at positions
16–18. C now directly targets the R08 branch. Every PR remains a draft.

**Completed:** all three R08 review findings at the prior head were implemented
in their owning resident layer, answered with exact witnesses, and resolved.
The retained constructor plan accepts zero inputs and compatible fixed dense
Bool/Index/F64/String operands alongside variable snapshots, converts physical
column-major inputs to canonical row-major order, and completes admission before
allocating owned drafts. Boolean snapshot transpose uses the existing
element-agnostic executor. Seven focused source/decoded comprehension tests pass;
all 49 `canonical_document_state` tests pass; the direct nullary binder/executor
test passes; runtime empty-comprehension and mutable vertical-concatenation product
tests pass. Numeric owner tests pass 86/87; the sole indexed-assignment assertion
is the already-recorded failure reproduced unchanged at the R25 base and is not
counted as R08 success. The new review was requested only after the unresolved
thread count reached zero. Evidence is recorded in
`recovery-evidence/r08-review-corrections.log`.

**Remaining:** new R08 review and focused exact-head CI; R07's existing matrix
initializer follow-up; all later accepted implementation and decision cells;
C product closure; one stable exact-C full qualification; protected merge and
post-merge verification.

**Current action:** inspect the returning #854 review and current stack checks.
The corrected R08 head is already merged into C and published; no duplicate full
qualification is requested for the slice.

**Next action:** resolve any new #854 findings before another review request. If
the review is clean, resume `/private/tmp/mech-syntax-s8r07-initializers` from its
existing rebased state and finish the two remaining matrix initializer witnesses.

**External blocker:** none. Checkpoint—not complete.


## Current checkpoint — R08/R07 integrated; R09 begins

**Candidate:** accumulated draft C #830 is `codex/syntax-s8c-cutover` /
`8ec616a49a3661cd9456bac7200cf6433803eb57`, directly based on R07 draft
#856 at `718b4854d8ec55488f7566779d3aa5848f906c2d`, which is based on R08
draft #854 at `f4dc9bbeceefaa5f2e5732b2006f855c663861e3`. Native stack #857
targets `integration/v0.4`; every PR remains a draft.

**Completed:** R08's fourth review finding is fixed at the owning snapshot
transpose binder, answered and resolved. Direct Index/String and source/decoded
Index regressions pass, as do all 50 canonical document-state tests. R07 is a
clean five-commit corrective series: closed controls execute once at activation,
activation controls stay outside turn topology, live dependencies remain
rejected, dynamic shapes remain snapshot-backed, and resource failures leave
publication unchanged. Its focused initializer tests pass 5/5, all 56 canonical
document-state tests pass, and the affected snapshot-access and hold-state owner
tests pass 8/8 and 1/1. Both exact heads are mergeable and have no unresolved
threads; fresh reviews and focused CI are pending.

The accumulated candidate contains both corrections. Its exact deleted-parser
source product probe passed all 14 catalog cases and the rooted source canary.
This is batch product evidence, not final qualification.

**Remaining:** R08 and R07 review and focused CI completion; R09/G06 and the
accepted R10–R24 implementation queue; R02/G02, R06/G17 and R15/G12 decisions;
R26/G29; remaining product/retirement acceptance; stable exact-C full
qualification; protected merge; and post-merge verification.

**Current action:** implement R09/G06 on
`codex/syntax-s8r09-comprehension-storage`, based on exact R07. The first strict
source/decoded witnesses cover retained `i32`, String and tuple binding/yield
storage over two turns and preserve the declared element schema.

**Next action:** finish the shared schema-directed retained item path, run the
focused and neighboring comprehension suites, publish one draft R09 PR, obtain
review, and integrate the accepted head into C. Interrupt that work for any
actionable R08/R07 review finding.

**External blocker:** none. Checkpoint—not complete.

## Current checkpoint — R20 ordered graph identity published

**Candidate:** R20 draft #877 is `codex/syntax-s8r20-ordered-graph` /
`f2bd4855c6464af0dcdadba8c07868dfbdcba101`, directly based on the
published rebased R19 head `cd70463d488324b0523cd32416488e3b1af59091`.
R20 remains a draft; review handling is assigned to another agent.

**Completed:** G16 now retains one deduplicated canonical graph identity for
every reachable retained source, including transitive paths through non-root
modules. Dependencies lower once in topological order, only caller-requested
roots publish, program and presentation outputs retain caller order, and shared
providers plan once. The source and decoded two-turn witness publishes main
`101`, then `102`, with the explicit dependency publishing `1`, then `2`; the
intermediate root stays hidden. The provider witness records one plan, zero
compile-time reads and one live read per turn. A later-root rejection records
zero live reads, effect preparations or deliveries, and retrying the same
compiler produces byte-identical output to a fresh compiler. Existing ordered
root, live-export, provider-count and callable-visibility regressions pass.
Runtime all-features test compilation, format/diff checks and the 20-fixture
bytecode format contract pass. The unrelated pre-existing resident
`access/range` layout failure remains outside R20.

**Remaining:** accepted corrective boundaries R02, R06, R21, R22, R23, R24 and
R26; integration of accepted batches into the designated C candidate;
deleted-parser product probes after each coherent batch; one stable exact-C
full qualification; protected merge and post-merge verification. R15 remains
the post-v0.4 interval-unit issue #865. R25 is assigned to another agent and is
not duplicated here.

**Current action:** R21/G21 is initialized from exact R20 on
`codex/syntax-s8r21-authority-retirement`. Its executable retirement gate finds
the remaining compiler tree authority (`compile_tree` and
`plan_artifact_tree_with_services`) plus interactive `from_tree`; browser tree
ownership is separately G22.

**Next action:** remove the runtime tree compiler and interactive tree/cache
authorities, migrate their production callers to retained canonical documents,
and run the named interactive, compiler and module-index acceptance cells.
Then publish the R21 slice on R20 without absorbing G22 browser work.

**External blocker:** none. Checkpoint—not complete.


## Current checkpoint — R23 canonical distribution closure published

**Candidate:** R23 draft #881 is `codex/syntax-s8r23-distribution-closure` /
`c7342723cccc6a89ed9512ee8c922ebc248f9edd`, directly based on R22 draft
#879 at `e760db1c593a7a9ddb9aabbf949ffa4dc131089f`. The pre-sync R23 work is
preserved locally at `26dbfac06`; tree comparison established that the published
R23 head contains that work plus the physical-deletion and canonical consumer
migrations.

**Completed:** the retired parser, legacy lowering and formatter authorities are
physically absent. Remaining CLI format/run, serve, resolver, runtime profile,
stdlib and presentation consumers use retained canonical documents. Served run
roots publish canonical program bundles and dependencies/prose remain renderable
without becoming implicit roots. Static formatting derives browser presentation
addresses from retained syntax without enabling the semantic compiler. The
syntax-only address list matches compiled canonical output anchors. The duplicate
`productions.tsv` grammar authority and legacy/parity suites are removed with an
explicit migration ledger. Exact-head focused evidence is: canonical browser
address regression 1/1; formatter boundary 32/32; stock-shim contract 1/1; serve
boundary 90/90 serial; migrated stdlib suites 18/18; reduced formatter and serve
feature checks green; canonical grammar/dependency/rule/SCC generator checks and
format/diff checks green. The full syntax all-feature test surface and full runtime
all-feature test surface compile on this head, and the supported syntax `no_std`
profile plus canonical submission-terminal witness pass after the explicit `alloc`
import correction. The first focused CI run then exposed three cutover remnants:
the REPL still imported Mika presentation data from the deleted syntax module,
the migrated stdlib helper lacked a reviewed lint exception, and the standard
distribution contract still counted the removed parser dependencies. All three
are fixed at their owners on the current head. The standard Mika build, warning
policy, regenerated standard distribution contract, packaging, native-host
catalog and static distribution-profile checks pass locally.

**Remaining:** focused CI and review on corrected R23 head `c7342723c`, followed
by accepted R24/G27 and R26/G29. R25 remains assigned to another agent and must
be stacked when accepted. After the accepted queue is assembled, the designated
C candidate still requires deleted-parser product probes, one stable exact-head
full qualification, protected merge, and post-merge verification.

**Current action:** inspect focused #881 CI on corrected exact head `c7342723c`
while beginning the dependency-ready R24/G27 index-range boundary.

**Next action:** add the strict source/decoded Index-range witness at its resident
owner, implement physical range binding/execution without weakening live-endpoint
rejection, and publish R24 stacked on the corrected R23 head. Interrupt that work
for any demonstrated R23 owner failure without absorbing R25.

**External blocker:** none. Checkpoint—not complete.
