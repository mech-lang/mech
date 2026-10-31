# Current executable profile and path-allocation diagnosis

**The retained compiler measures 13.696806× Roc on complete eight-byte level-12
compression. This round makes no production compiler change and claims no new
speedup.** Fresh elapsed is 3.9857705 ms against one shared fresh 0.291 ms
reference. Three independent preparations measure 13.674686×, 13.714991× and
13.695088×; all 1,536 steady calls are retained, with six warmup/profile calls
excluded. The reference contains 20,000 calls.

The previous longer round-51 result was 13.915864× against its own 0.286 ms
reference, with Mech at 3.979937 ms. The new Mech time is about 0.15% higher;
the different multiple does not establish another optimization. Preserve both
round-51 matched cohorts and their adverse instruction result.

| Separate diagnostic medians | Roc | Mech | Mech / Roc |
| --- | ---: | ---: | ---: |
| Instructions/call | 1,364,372 | 45,493,321.5 | 33.343781× |
| Cycles/call | 884,130 | 13,149,330 | 14.872621× |
| Allocation requests/call | 95 | 13,484 | 141.94× |
| Requested bytes/call | 9,021,923 | 98,480,739 | 10.92× |
| Prepared tracked heap bytes | 32 | 142,982,576 | — |
| Additional peak tracked heap bytes | 9,257,328 | 20,517,976 | 2.22× |
| Peak tracked heap bytes | 9,257,360 | 163,500,552 | 17.66× |
| Post-call resident bytes, CPU cohort | 14,794,752 | 319,651,840 | 21.61× |
| Post-call physical bytes, CPU cohort | 14,451,648 | 228,886,272 | 15.84× |

Hardware has 32 steady observations per label; allocation has 32 reference and
eight steady Mech observations. Counter boundary overhead is uncorrected and
all page-in counts are zero. Counter and allocation elapsed is excluded from
primary timing. The allocation counts/bytes, prepared heap, and tracked peak
agree with the retained round-51 after measurements. Instruction count is also
approximately unchanged. Cycles come from a separate cohort and are not an
explanation of a cross-round elapsed difference.

Tracked allocator high water includes retained compiler/catalog/decoded-image
state; it is not minimum standalone runtime RAM. Post-call OS readings are not
peak RSS. Both programs receive the whole input outside the timer. Eight bytes
cannot establish streaming or large-input copying behavior. Relocating realloc
has an unchanged 13,905-byte observed copy upper bound; total copy volume,
branch misses and cache misses remain unmeasured.

Fresh serial sampling covers 695 complete Mech calls and one 20,000-call
reference run, excluded from elapsed comparisons. Mech has 1,227 exclusive
sample weights; the reference has 1,519. Ancestry unions are:

| Mech ancestry group | Weights | Share |
| --- | ---: | ---: |
| Private frame actions | 345 | 28.12% |
| Tail cleanup | 63 | 5.13% |
| Canonical construction/admission | 67 | 5.46% |
| Projection accounting | 38 | 3.10% |
| Thread-local lookup | 41 | 3.34% |
| Schema closure | 17 | 1.39% |
| Frame copy actions, included in frame actions | 29 | 2.36% |
| Capacity-buffer calls | 15 | 1.22% |
| Memory move/copy routines | 26 | 2.12% |

These groups overlap and include surrounding harness work. They are sampled
ancestry, not additive time percentages, removable fractions, or predicted
gains. The reference's dominant opaque native symbol has 1,404/1,519 weights
(92.43%); it is not assigned an unsupported source name. The generated Mech
entry has 137 exclusive weights, the helper dispatcher 54, and the frame-action
helper 48. Within frame handling, call-limit scopes, memory-budget evaluation,
projection charges and frame copying remain reached costs.

## What the large allocations actually do

The historical round-37 trace already attributed large path reservations to
`copy_buffer_with_sublist`. The new matched current trace refines that finding:
35 × 2,442,056-byte requests are one initial U64 reservation, 32 fallback appends,
and two fallback clears. Together they request 85,471,960 bytes.

Each backing reserves 305257 U64 words, but the fallback appends reach lengths
258 through 265, four times each. From those observed lengths and the unchanged
primitive implementation, these 32 appends copy **66,944 initialized bytes**.
The initial reservation and clears copy zero elements. This is a derived count
for these fallbacks, not an instrumentation of total program copying. Requested
capacity must not be presented as bytes copied.

All 32 fallback appends have no static consuming candidates. In the first
append's source graph, site 12806 uses value 7335; the old-value query can reach
the old-source Copy at 12825 through a repeated conditional on predicate 7343.
The source-level `more` guard selects the append branch, while the old-source
return belongs to the other branch. The current conservative query does not
carry that Boolean fact. `graph-path.json` preserves the exact path and blocks;
it is a diagnostic illustration, not a new legality proof.

An isolated current-owner census additionally finds canonical owners 7335 and
7382 at the first append and no matching private-frame holdings there. Value
7382 is a forward structural projection of the earlier read tuple. Improving
the branch query alone therefore does not establish exclusivity or enumerate
every owner to retire. The existing alias candidate walk is backward through
structural transport. No arena-wide runtime scan or weaker ownership rule is
introduced into production.

The two failing clears do have static candidates. After existing retirement,
one still has two matching canonical owners and a frame owner; the other has a
frame owner. These are real remaining holdings, not evidence that they are dead
or may be discarded. The census covers only canonical arena and private-frame
holdings; it excludes the local source handle, caches and external holders.
Core's exclusive handoff remains authoritative. Both diagnostic overlays emit
the exact retained Object and image hashes and reproduce the same 35 requests.

This separates a candidate ownership improvement from a supposed 85 MB copy
bottleneck. The fresh capacity-buffer ancestry is 1.22%, while frame handling is
28.12%. Retain the alias opportunity, but do not predict a large speedup from
reserved-byte volume. The next bounded candidate is described in NEXT.md.

## Exact scope, validation and failures

Base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
Retained published compiler checkpoint:
`70847471ba06c9e395f9628eb1b19c7b11ea82f6`.
Optimized GPU source: `e0c180a47fdeeabada8b6d8df0c57f3f5fdc3d6c`;
the other seven direct libraries and transitive graph remain frozen from round
48. Exact flags/features, library hashes, allocator, consumer sources, and
same-build bindings are archived. Optimized consumers retain `-D warnings`.
IR/work/ABI/bytecode remains **132/95/37/14**.

Source SHA-256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
Bytecode SHA-256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Object SHA-256: `1a8bb46e9f78fec2147b3242fab5651e470f86ba0347f4f11c7f1aedd5008777`.
Image SHA-256: `1b4dbdd2fb0c1f9f51ae7e9dd0bc931bbc2804e6cb949653461d6348c31615c9`.
Generated text stays **13,046,668 bytes** and machine stack **108,368 bytes**.

Fresh complete-compression executions check exact raw bytes
`[75,76,74,78,73,77,75,207,0,0]`, independent inflate, input contents and aliases,
output capacity 13 with clear/reuse, work **60,260,219,686**, and final managed
budget/node cleanup zero. The previously qualified comparison work 47,612 is
not separately recounted by this refresh. Source, algorithm, real tree/cache/
path capacities, search limits, allocation sites, allocator and `evaluate_into`
timer are unchanged. No production implementation, ABI, resource policy,
architecture allowlist or CI inventory changes occur in this round. The prior
599-test Rust qualification remains prior evidence, not 599 new test executions.

Commands are retained as structured arguments in each receipt. The diagnostic
drivers are `probe/run.py`, `owners/run.py` and `owners/retry.py`. The current
profile drivers are `refresh.py`, `finish-refresh.py`, `profile-isolated.py`,
and `profile-reference.py`. `analyze.py` independently derives the numerical
JSON/CSV; `reconstruct.py` verifies archives and reproduces them without running
a compiler or benchmark. The completed replay verifies **20 archives / 168 file
occurrences / 156 unique paths**, preserves the one multi-version path, and
reproduces numerical JSON/CSV, profile JSON and the graph witness byte-for-byte.

Two interruptions are preserved and resolved:

- **Environmental:** the ownership trace consumer link failed with `errno=28`
  (disk full), after its GPU build and linked smoke succeeded. Completed
  diagnostic executables/duplicates were removed with hash inventories. The
  exact same link command then passed, followed by the three successful trace
  executions. No compiler limit or program size was changed.
- **Introduced diagnostic reader error:** the refresh driver initially expected
  current CPU counters on stderr's allocation prefix. The executable had exited
  zero with 34 valid JSON observations on stdout, including 32 steady samples.
  The reader was corrected, the original receipts retained, and that completed
  cohort validated without rerunning or discarding it.

There is no unresolved new runtime/test failure. No broad suite or all-features
build was rerun for these diagnostic-only changes. The inherited reduced-catalog
stats exclusion remains documented in round 51; it is not reopened. Hosted CI
is cancelled on publication by user direction and is not reported as passing.
The PR remains draft pending green CI and clean review. Parity remains open.
