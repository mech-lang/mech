# Recovery continuation after audit acceptance

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
