# Apple M3 native DEFLATE follow-up

No completed experiment in this checkpoint meets the requested M3 target of
81/81 wins against Roc and at least 61/81 against libdeflate 1.26 level 12 in
both complete orders. The reference remains the source-built 71,192-byte
encoder from the [M3 reproduction](../apple-m3-reproduction-20261117/README.md).
All experiments use the original Mech program and its unchanged canonical
bytecode, and the same frozen peer executables and fixture contracts.

The strongest small-input combination enables matcher spill forwarding,
preserves comparison conjunctions, and removes private write-prefetch hints.
Its complete comparison is:

| Order | Reference Roc wins | Candidate Roc wins | Reference libdeflate wins | Candidate libdeflate wins |
| --- | ---: | ---: | ---: | ---: |
| Forward | 69/81 | 68/81 | 59/81 | 60/81 |
| Reverse | 68/81 | 68/81 | 58/81 | 61/81 |

These contemporaneous reference counts belong to this campaign. They do not
replace the preceding reproduction's independently acquired counts. The
candidate passes exact outputs, independent inflation, borrowed aliases,
checked allocation failures, cleanup and the 5,280-byte stack ceiling.
It remains unqualified because it does not meet the complete runtime target.
[Every case in seconds and paired peer multiples](timings.csv) ·
[All completed screen decisions](screens.json)

## Compiler experiments

| Mechanism | Result |
| --- | --- |
| Matcher spill forwarding | Removes 29 instructions; insufficient large-input recovery |
| Preserve comparison conjunctions | Recovers one Roc case in its focused orders; complete recovery not established |
| Combine spill forwarding, conjunctions and demand writes | Improves weak 1 KiB inputs; complete results above |
| Demand reads, narrower arithmetic and repeated exit copies | Small-input gains; no recovery of the large Roc losses |
| Fold doubled halfword addresses | Correct native addressing; broader version raises stack to 5,328 bytes and fails the stack gate |
| Restrict doubled addressing to a single use | Correct independent checks; root stack still grows; not promoted |
| Fold masked shifts into UBFIZ | 40,128 independent native checks pass; almost no large-input gain, and stack rises to 5,296 bytes |
| Defer pure item construction until the winning ordinal is known | Correct full-program outcomes; natural-text screens regress about 1–3% |
| Qualify immutable input-packet loads | Focused packet tests and 162 exact calls pass; disabled build is byte-identical to the reference; enabled build fails the 5,280-byte stack gate, so no runtime cohort is acquired |

The first spill-switch trial is an explicitly retained no-op: a later existing
selector overwrote that switch and every emitted numeric body was identical.
The active-selector trial above is a separate artifact. No speedup is attributed
to the no-op.

The addressing and bitfield transformations operate on generic typed integer
expressions. Their patches are retained for investigation, outside the qualified
compiler. They are not selected by input bytes or benchmark names.
[Single-use addressing patch](scaled-address-single-use.patch.gz) ·
[Masked-shift patch](masked-shift-folding.patch.gz) ·
[Packet-load experiment](packet-load-qualification.patch.gz)

## Direct phase accounting

A diagnostic worker wraps the frozen Mech numeric phase bodies with monotonic
timers. Only symbol visibility and call destinations change; the phase bodies
retain the reference instructions. All 162 fixture checks pass after wrapping.
The diagnostic acquires 96 balanced three-actor observations per selected input,
with unmodified Mech and Roc controls. It does not supply qualification timings.

| Natural text, 65,536 bytes | Instrumented seconds | Share |
| --- | ---: | ---: |
| Optimal-path scoring | 0.0040361 | 51.5% |
| Match search | 0.0032552 | 41.5% |
| Other work | 0.0005477 | 7.0% |
| Complete instrumented call | 0.0078390 | 100% |

The unmodified Mech control measures 0.0078533 seconds and Roc 0.0076037
seconds in this diagnostic. The instrumented call is 0.9982× its unmodified
control. The controls' ratio is approximately 1.033× Roc. Phase intervals
include wrapper bookkeeping, and this acquisition uses three actors. The
complete-call interval retains heap construction and cleanup. These diagnostic
values must not replace the four-actor qualification results.

On 8 KiB natural text, scoring accounts for 53.2%, match search 28.9%, and
other work 18.0%. On random 1 KiB input, approximately 47% is outside the
numeric phases. The data support investigating both major phases and small-call
support costs. They do not identify a particular cache or branch mechanism.
Wrapping changes layout and adds call/timer overhead; very short phase intervals
also have limited clock resolution.
[All phase intervals](phase-accounting.csv) ·
[Totals, controls and instrumentation boundary](phase-accounting.json)

## Source integration in progress

The newer source-backed #993 experiments include smaller synchronous phase
packets, checked fallback reuse, output pruning and empty-route forwarding.
The 67,656-byte candidate reports a 4,672-byte stack peak on M1. Its original
Mech source and bytecode match this campaign. Six maintained source updates
and the complete experimental patch are verified against their published Git
blob and SHA-256 identities before a local M3 compiler rebuild.

No M3 result for that newer candidate is claimed in this checkpoint. M1 timings
do not qualify an M3 build. The requested M3 runtime floor is also distinct from
the newer M1 study's stricter 62/81 libdeflate promotion policy.
[Adopted source identities](upstream-source-adoption.json) ·
[Published candidate source pins](upstream-candidate-source.json) ·
[Published object, bytecode and recipe](upstream-candidate.json)

## Evidence

Together with the preceding execution study, **24,162,048 complete timing
calls** pass replay with the unchanged qualification verifier. The separate
phase-timer diagnostic is excluded from this count. Raw replay checks outputs,
inflation, aliases, cleanup, actor identities and balanced orders.
[Fifteen-campaign replay](raw-replay.json) ·
[Additional deferred-item replay](raw-replay-deferred-item.json)

[Evidence archive and SHA-256](evidence.json) identify a delta archive. Extract
the preceding execution-study archive first, then overlay this delta. The
manifest records every added or changed file. Compiler-helper executables and
the newer compiler build still in progress are excluded; sources, build
commands, completed native artifacts, failures, raw observations, proofs and
diagnostics are retained. The frozen peer/fixture foundation is linked by the
preceding study. Regenerating measurements requires fresh output directories,
relocated paths and the shared campaign lock.
