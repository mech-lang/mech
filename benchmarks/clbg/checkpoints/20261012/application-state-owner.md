# Application-state owner checkpoint

Recorded 2026-10-12 20:22 UTC. This is an unpinned handoff note, not a timing
manifest or authorization to repeat a phase. Re-read the actual reports and
process identities before acting. Preserve every frozen proposal, source,
report, failed attempt, and existing native process.

Workspace: `/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech`.
Bundled Python:
`/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3`.
Use `-B -u`. Builds, tests, runtime probes, validation, and hash/pin sweeps use
the common `/tmp/mech-clbg-timing.lock` and the reviewed owned-process guards.
Do not run a competing workload or interrupt a valid sample.

## Live APL continuation

Controller directory:
`benchmarks/clbg/r-apl-uiua-comparison/20261011/uiua-fann-compact-v5/controller_rev2/apl-argv-v1`.
Controller `continuation.py` SHA256:
`cfedca35b9770a35f707e1f40c1c35aca8f05c5e97f9ebdcf9890b04bd44dbc0`.
Use the exact manifest for verification.
Full manifest SHA256:
`cfc246e46aa4e67db659dae89009259d5d9188381b007f486b0266cc973ed1b8`.

Durable supervisor PID 17798 was launched at 19:44:14 UTC. Its authoritative
paths are `full-owner/launch.json`, `status.json`, `timings-apl.json`,
`supervisor.log`, and `attempts/`. The detached launcher session 86972 exited
normally after recording the durable owner. Do not launch it again.

At the 20:14 UTC identity snapshot, optimized k-nucleotide round 1 was active
as attempt 66, sampler PID 21962 and native PID/PGID 21969, started 20:01:47 UTC.
This is a valid actual full sample. Preserve it. The owner resumes only missing
APL samples across FASTA, k-nucleotide, trees, nbody and fannkuchredux; the
reviewed plan has 34 actual missing processes and preserves all 58 old attempts.
The original R/APL owner 52680 is terminal and must not be resumed.

The repaired FASTA group completed at 20:01:17 UTC with three exact full samples
per profile. Compact median: 332.738161375 seconds. Optimized median:
38.798114333 seconds. Matched Mech median: 0.581235 seconds. Ratios are
572.467524 and 66.751167 times Mech. Every output has 254,166,745 bytes and SHA256
`3fcf4f78104c8a65ef210fe1d469f4e473456c791225f2f1f9114f4986aa09fa`.
The original failed optimized FASTA attempt 58 remains explicit lineage.
This note does not publish or import the group.

Later observation: optimized k-nucleotide attempt 66 qualified at 20:14:37 UTC
in 769.057167708 seconds, with complete 259-byte output SHA256
`f8c042fe770dda8f870b967605ea40ca2e353f2bdf92e89a327209dd31d445c7`.
Its fresh matched Mech sample is still pending, so this is neither a median nor
a ratio. The next actual sample is compact k-nucleotide round 1, attempt 67,
sampler PID 27223 / native PID and PGID 27230, started 20:14:42 UTC. This
supersedes the earlier active-process snapshot above.

Read-only source finding for future reader qualification: both frozen APL K
forms retain only the first six header characters and compare that prefix to
`>THREE`; they consequently also accept `>THREEBAD`. Their record separators
are LF, with CR stripped only from sequence data, so CR-only records are not
recognized. The newer DNA oracle cases for Nim/Pony/V require exact
space/tab/end target-token boundaries and CR/LF/CRLF handling. No probe or edit
was performed for this finding. The current canonical full input has LF and
the exact THREE token, so this does not invalidate or authorize interrupting
its live canonical sample. Root should review any future isolated reader
revision and its qualification/admission scope.

Round 1 later completed: compact K attempt 67 took 3932.098400250 seconds,
optimized attempt 66 took 769.057167708 seconds, and fresh Mech attempt 68 took
0.538898208 seconds. Every output matched the complete 259-byte reference above.
Individual round ratios are 7296.551263 and 1427.091715 times Mech; these are
provisional actual full samples, not final medians or admission. Compact round 2
is attempt 69, sampler 44442 / native PID and PGID 44477, started 21:22:14 UTC.

## Terminal preparation phases

Nim session 89510 / PID 13686 stopped with exit 1 at 19:36:36 UTC.
Report: `benchmarks/clbg/nim-comparison/20261012/qualification/private-v4/report.json`.
All 48 new production cases passed; with the exact 106 old cases, all twenty
production binaries have 154 distinct retained output records. Failed attempt
49 is the separate pure-BigInts ABI contract: it expected `(-17) div 5 == -3`,
but the pinned pure library documents floor division, giving -4. GMP's separate
`mpz_tdiv_q` contract still correctly expects -3. Do not change production
arithmetic or repeat production outputs to repair the primitive assertion.

Pony session 3013 / PID 13702 stopped with exit 1 at 19:38:43 UTC.
Report: `benchmarks/clbg/pony-comparison/20261012/qualification/private-v4/report.json`.
Twelve new compact-k-nucleotide outputs passed, bringing retained production
evidence to thirteen binaries / 110 cases. Optimized K then failed compilation:
its U64 shifts received USize variable shift amounts. The additive v5 source
adds only the two lossless `.u64()` casts; nineteen other declarations stay
unchanged. Seven remaining production profiles still need actual fresh builds.

FP session 17502 / PID 13726 stopped with exit 1 at 19:44:12 UTC.
Report: `benchmarks/clbg/interpreter-algebraic/20261012/revisions/compile-input-v3/preparation/report.json`.
The strict production executable and actual LLVM/ARM codegen were retained;
binary SHA256 `cb82aac9fb7cceec8d46fed068227f2f0eb20a0ee7473670e77883d22eb946d2`.
The opt-in conformance build failed E0433 because resident is under
`sequence::runtime`, not directly under sequence. Zero conformance, CLI or
algebraic native cases ran. Module-bridge-v4 adds the feature-gated forwarding
through the actual module nesting, with fresh copies/targets and no default
arithmetic change. Data's bounded read-only review passed. There is no v4
native or metadata launch authorization in this note.

## Additive drafts and coordination

Nim v5 guard/reuse/source_proofs/Pi observer and Pony v5 guard/reuse/fixtures now
exist in their cohort directories. Their author was being confirmed with root
and the LOC agent; this restored turn made no edits to those guards. Do not draft over
them concurrently. Read-only review found Nim's declared fixture count 17 did
not match the selected ten plus eight methods; its revision document says 18.
Later bounded read confirms the guard now declares 18; that finding is resolved.
Actual work, worker,
allocation, canonical LOC, full manifest reviews, and three canonical samples
with fresh Mech controls remain separate gates for the new languages.

Data owns the V remaining-eight cohort, session 8433, and Crystal archive-v6.
V's six metadata fixtures passed; retained regex/revcomp records are not rerun.
Crystal metadata waiter 88471 was reported interrupted with exit 130 at its
blocking flock before zero cases/logs/receipts/native work. Neither this owner
nor Data sent a signal or restarted it. Preserve that pre-trial interruption
and coordinate any replacement with root. All existing native owners remain
untouched. A later inventory found that `archive-v6-fixtures.json` and its log
already exist with a six-pass receipt dated 20:14 UTC; reconcile that actual
receipt rather than repeat the fixture phase because an older tool wait ended.

A later read-only process inventory found already-live metadata owners:
Nim v5 sealer PID 28539, parent 1, using guarded_prepare_v5.py with
`--seal --proposal-file .../proposals/private-v5.json`; Crystal v6 sealer
PID 27883, parent 1058; Pony v6 sealer PID 29077, parent 1, using
`--seal --proposal-file .../proposals/private-v6.json`; and V cohort PID 20372.
Pony v6 repairs only a mock missing its required execution_fingerprint; its v5
32-pass/one-error fixture failure remains recorded with zero compiler/native work.
None of these controllers was newly launched in this
restored turn. Confirm their identities and ownership before a replacement or
any concurrent edit. Do not infer that an interrupted tool session means no
durable or separate controller remains. No signal or cancellation was sent here.

Later seal reconciliation: Nim private-v5.json exists and the original log
records 18 passed methods in 0.240 seconds; Pony private-v6.json exists and its
log records 33 passed methods in 0.164 seconds. Crystal
private-input-archive-filter-v6.json also exists. The three sealer PIDs are now
absent; preserve and review their actual terminal receipts/proposals before a
native phase. No mocked check was repeated in this restored turn, and no full
proposal hash or pin sweep was performed outside the mutex. V cohort 20372
remained live at the last process snapshot.

The generic arrow-body mutation fix was already selectively committed and
pushed as `c3396db09a2a9b8dc3e7866b3d5aeec1038b8a81`. Its focused 28 checks,
322 unchanged compatibility sources and 240 canonical PBMs passed. Do not
repeat those finished checks or mutate frozen compiler/study inputs.
