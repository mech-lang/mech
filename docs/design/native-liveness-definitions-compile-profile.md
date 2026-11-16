# Materialize only definitions relevant to a selected liveness graph

Filtering unused definition facts reduces full compiler time **2.52%**, from
**29.94–30.89 to 29.44–29.86 seconds**, on top of the retained
[visitation optimization](native-thread-visits-compile-profile.md). Complete
process time improves **2.39%**, from **30.03–31.08 to 29.61–30.03 seconds**.
Every source, bytecode, native Object and complete metadata byte remains
identical to the qualified fast compressor.

Source `1682e13446f223ed85e1832a14befdcec213cd6c` retains each query's graph,
reads, predicates, tail stops and original ordered definition lookups. It
materializes kill facts only for values read somewhere in that selected graph
and definition sites belonging to that graph. Unread values cannot seed a
liveness fact, and an outside site cannot kill one on a selected edge.

Definition lookup/validation still happens for every originally selected value
in the original order, including unread values. Empty reads therefore suppress
no old error. Read IDs outside the declared value range retain their previous
solver behavior. No graph, semantic proof or liveness result is cached.

## The diagnostic identifies unneeded work

A separate temporary instrumented compile observes **790 distinct exact query
contexts**, with no exact duplicates. A result memo would not address these
observations. Queries share an immutable program, but vary their root, tail
stops, predicate facts and requested values.

| Diagnostic observation before filtering | Value |
| --- | ---: |
| Program blocks / values | 41,039 / 22,460 |
| Median selected graph size | 284 sites |
| Median materialized definition facts | 23,850 |
| Median returned live values | 9 |
| Total materialized definition facts | 11,737,288 |
| Total selected-site visits across queries | 11,983,473 |
| Query graph construction, summed | 2.842 s |
| Query liveness solving, summed | 2.239 s |

The query intervals are nested within native preparation. They exclude query
logging and are diagnostic, not primary timing observations. The diagnostic
preserves all four generated outputs. Its source, patch, build, query logs and
restoration hashes remain in the packet. Instrumentation is restored before
the functional edit. Diagnostic address labels are observations, never cache
or proof identities.

## Matched fresh complete compilation

Four serial ABBA observations use frozen compilers, the same **54 existing
runtime flags** and identical host process-statistics access. Bootstrap,
diagnostics, the separate screen and fixtures are excluded. No other builds,
tests, profiles, fixtures or archive work overlap primary timing.

| Measurement | Control | Candidate | Mean reduction |
| --- | ---: | ---: | ---: |
| Compiler source through Object | 29.944, 30.893 s | 29.443, 29.863 s | 2.52% |
| Complete process | 30.025, 31.083 s | 29.613, 30.034 s | 2.39% |
| Native preparation | 7.032, 7.433 s | 6.368, 6.450 s | 11.39% |
| Retired instructions | 466.168, 466.159 billion | 452.059, 453.227 billion | 2.90% |

Candidate maximum RSS is **1.06% higher on average**, with both candidates above
both controls. No memory saving is claimed. Main cleanup is slightly slower in
this cohort; all contrary counters and unchanged-stage observations remain in
the compact result. The complete-process ranges overlap slightly. Four compiles
are descriptive observations, not a statistical population, and variation
outside preparation is not causally attributed to this change.

## Qualification and evidence

All **11 liveness tests** pass. Added coverage compares dense and sparse solvers
with the original reference after removing irrelevant definitions on 256 cyclic
and branching graphs, with nine starts per graph, unused high IDs and outside
definition sites. All four outputs also match between subjects for six complete
array/enum programs and eight checked Bool/U64/I64/F64 scalar programs.

Every primary observation and the screen reproduce Object
`b125164d7e8ba9d4e98cb4b467b3194bb14a6d8cafe411600359c6f67943c703`
and the complete qualified metadata, including main code and numeric helpers.
No new compressor runtime latency campaign is claimed. Grammar, language,
current-visit/ownership checks, finite admission and optimization level are
unchanged. Full compilation still exceeds ten seconds.

The [compact result](../../benchmarks/mech-deflate/native-profile/live-definitions-compile-apple-m3-20261114.json)
and [archive receipt](../../benchmarks/mech-deflate/native-profile/live-definitions-compile-apple-m3-20261114.archive.json)
pin all 364 files. The [complete archive](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/mech-live-definitions-20261114.tar.gz)
is published in benchmarks. Its repository copy verifies every hash/Git blob and
replays byte-identically without a compiler or generated executable.
