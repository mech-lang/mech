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
