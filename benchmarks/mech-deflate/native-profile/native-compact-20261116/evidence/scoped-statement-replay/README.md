# Replay bounded statement alternatives

Reuse the existing guarded recipe for a winning `STATEMENT` in
`MECH_CODE_ALT`. The existing longest-match choice, admission limits,
frozen accounting and fresh-identity construction still apply.
The prior parser profile attributes 155 ms of exclusive work to this
family. [Selection profile](selection-profile/report.json)

Six balanced standalone orders move the median parser time from
**1.011 s to 0.914 s**
and the matched ratio to **0.905×**.
Capture-only measures **1.002×**
the control. The guarded recipe applies for 117 of 123 statement winners.
The six larger winners use ordinary execution.

Differential checks compare raw events, ordered cache publications,
logical resource accounting, poll partition and the identity-bearing
final syntax and indexes. The focused tests cover finite grants,
segmented UTF-8, and the enclosing raw-access witness. All 535 existing
syntax tests and three focused checks pass.
[Standalone observations](standalone/comparison.json)
· [Differential harness](validation/differential-tests.patch.gz)
· [Focused results](validation/focused.log)

| Eight balanced Apple M1 builds | Mech control | Mech statement replay | Roc | libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Source through final link | 2.81174 s | 2.70927 s | 2.98501 s | 0.47997 s |
| Source through strip | 2.82225 s | 2.71974 s | 2.99558 s | 0.49037 s |

Median Mech parsing in the matched full builds changes from
**1.015 s to 0.916 s**.
Source through final link takes **3.67% less** in the
matched pairs. Compilers and host support are prebuilt. All matched runs
reproduce the pinned bytecode, encoder object and metadata. The separately
relinked worker reproduces byte for byte and passes 162 exact linked calls.
[Plan](source-through-link-plan.json)
· [Raw builds](source-through-link-observations.json.gz)
· [Worker identity](worker.json)

Four separate phase captures give **0.918 s for
Mech** and **3 ms for Roc**, about
**306×** across the reported intervals. Mech
builds the lossless syntax tree and indexes. Roc sums tokenization and
parsing with one compilation job and its cache disabled.

The reported parser phases come from separate diagnostic runs. Each
pair reproduced the pinned artifacts.
[Phase plan](parser-phases/plan.json)
· [All four pairs](parser-phases/summary.json)
