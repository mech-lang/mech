# Packed loads, population counts and scalar homes

This packet records the completed IR345–349 compiler work. Compact IR349 U32
improves against compact IR336 in both randomized comparisons, but remains
**1.008793× the fastest eligible Rust in the primary comparison and 1.008174× in
the independent confirmation**. Rust parity has not been reached. No new managed
latency gain was established; IR334 remains the selected managed checkpoint.

The primary compact U32 result is 1.247625 ms against Rust's 1.236750 ms. The
independent result is 1.192166 ms against Rust's 1.182500 ms. These are separate
same-machine cohorts, not measurements to pool or compare across cohorts.

The complete qualified IR349 compiler source is published at optional alias
`7a6686c6790b0d8e0a282a5619bb884f8ffe08d0`. Reconstruction requires only the earlier
published IR340 and IR338 commits below; no unpublished commit is required.
Publishing general compiler improvements does not select every generated
managed binary as a latency winner.

## Scope and counts

| Evidence | Exact outputs | Rounds × variants |
| --- | ---: | ---: |
| First packed query primary | 1,860 | 31 × 60 |
| First packed query resources | 406 | 7 × 58 |
| Scalar/packed primary | 1,736 | 31 × 56 |
| Scalar/packed resources | 399 | 7 × 57 |
| Independent compact confirmation | 468 | 13 × 36 |
| **Timed/resource total** | **4,869** | |
| Separate full+31-edge adapter qualifications | **1,856** | 58 × 32 |
| Separate allocation census diagnostics | **28** | 2 × 14 |

Qualification counts include fresh compact IR343 controls, compact IR345/348/349
original/decoded and resource adapters, managed IR346/347/348/349 adapters, and
three source-ladder attempts. They are correctness checks, not acceptance timing
samples. The fresh 5,001-iteration IR349 sampled profile is another diagnostic;
its iterations are not added to either output total.

`report.json` contains every cohort's timing and resource table, matching managed
work qualifications, source closure and selection limits. Raw run outputs,
orders, protocols, receipts, stdout/stderr, whole-process resources, concurrency
observations and exact Object/link audits are retained. Every observed worker
peak is accepted within 1–4 for four tasks and one batch. No sample is removed.
The historical unselected IR335 total-work variation is retained, with its exact
numeric-worker bill; its physical cause is not attributed.

## Compiler experiments and results

| IR | General compiler change | Result retained |
| --- | --- | --- |
| 345 | Apply the existing exact I64 SWAR population-count matcher in compact entries and numeric phases | Two scalar 15-instruction SWAR sequences become two four-instruction native count sequences. Standalone U32 gain did not independently confirm. |
| 346 | Emit direct unsigned I8/I16/I32-to-I64 loads and retain exact width facts | Redundant extension copies disappear. U32 query drops 48 static instructions; managed latency gain was not established. Wide stack growth is retained. |
| 347 | Reconstruct canonical Boolean merge values and rematerialize selected cold-home operands; bound inherited condition traversal | Original source homes/stores and semantic states remain. Smaller query and stack do not establish managed latency gains. |
| 348 | Compose unsigned loads with compact population counts | Standalone U32 load component did not independently confirm. |
| 349 | Compose IR348 with selected Boolean homes | Compact U32 gains against IR336 confirm; compact wide gains against IR348 also confirm. U32 gains against IR348 do not establish across both cohorts. No managed gain. |

IR349 U32 versus IR336 has median paired changes of approximately −1.0798% in the
primary comparison (95% interval [−1.9283%, −0.8487%]) and −1.6669% in confirmation
([−3.608%, −1.022%]). The exact unrounded comparisons and bootstrap seeds are in
the records. The compact-wide IR349/IR348 gain confirms in both cohorts. The U32
IR349/IR348 primary interval crosses zero. Component results are not inferred
from the net composition gain.

All three revisited source insertion ladders regress prospectively and remain
unselected. Their exact source diff, 6,336 bounded sorted-state equivalences,
qualification outputs, machine changes and failed timing attempts are retained.
Static instruction totals include cold code; neither static counts nor aggregate
hardware counters alone establish a faster critical path.

The broad native suites retain the same 16 inherited failure names: IR346 has
1,092 passes, IR347 1,093, IR348 1,094 and IR349 1,099, with 11 ignored and 35
filtered cases in each. Focused receipts are retained, including the IR348
wrapper/parser error: all original focused commands passed; an offline parser
correction recovered all 52 names without rerunning focused tests. Failed IR347
compilation, the compact JIT fixture's missing production frame flag, failed
static-audit assumptions and packet-development failures are preserved.

Standalone IR346 production uses the V3 backend, but its original focused/broad
test executable links the historical backend family. Those original tests are
not described as production-coherent. IR347/348/349 focused and broad tests use
the exact V3 backend family; the IR348/349 composed suites include the unsigned
load, capture and geometry contracts. Their coverage supports those compositions
without changing standalone IR346's provenance or requiring a historical rerun.

## Allocation and profile diagnostics

The two IR340 U32 allocation censuses preserve the ordinary Object. Each version
has 14 exact runs, prospective orders and exact phase sums, with no allocation
failure, underflow or overflow. Both agree on 725 requests and 5,198,427 requested
bytes in the measured scope. V2 partitions the prior 403 parent requests by
actual typed dispatch; 172 requests belong to histogram finishing/publication.
Only 60 of 189 combined parent/worker setup requests belong to fixed-bank
materialization. A possible authority-allocation improvement targets 30 requests,
with no measured latency saving. Detailed counts and limits are in the retained
census READMEs and partition audit.

These counters describe Rust GlobalAlloc requests and issued/retired layout
bytes, not RSS. Deallocations are attributed to their retirement phase. Successful
reallocations retire the old layout without a deallocation call. Fixed-bank tags
cover plan creation/materialization after budget selection. Failed-path cleanup
is not assigned the successful retirement phase by assumption. A case-insensitive
handoff filename collision and exact old-byte recovery are explicitly retained;
raw and prospective census artifacts were unchanged.

The exact IR349 full-path profile restores once, then performs 5,001 fresh JSON
decodes, admissions, ledgers, four-worker create/run/join, materializations and
outer destruction. Of 4,228 native non-wait sampled stack weights, generated code
has 3,039, query self 1,240, escrow 219, allocator self 59 and thread creation 63.
These are sampled stack weights, not elapsed-time percentages. Single-offset
query frames map to the retained ordinary Object; coalesced multi-offset frames
remain unresolved. Concurrent build/archive load and the extra destruction scope
make phase averages diagnostic only. No profile average selects an implementation.

## Offline verification

Run from a checkout containing the required published Git objects:

```sh
python3 benchmarks/related-post-gen/integration/packed-scalar-native-20261112/verify.py
```

`verify-fresh-git.py` additionally imports only the actual required published
commit/tree/blob objects into a new isolated Git object store and repeats the
verification. It creates no synthetic commit aliases and intentionally omits the
optional IR349 commit, proving that it is not a required dependency.

Required source authorities:

- IR338: `b939e42a6cbbb6e9cd5a747d6c5ba1ba09567164`
- IR340: `1ef5985a9c58fd2857bdf8eedc06a2b7b50ef622`

The verifier reads this packet and those Git objects only. Historical absolute
paths are provenance labels resolved through archived content hashes; `/private/tmp`
does not need to exist. It does not compile or execute native programs.

`records.json` maps logical evidence names to content-addressed gzip members.
One pinned standard-library-readable `records.zip` contains those exact gzip
bytes as ZIP_STORED members. The verifier checks the container hash, complete
unique member set, gzip hashes/sizes, all exact outputs, randomized orders,
medians, paired bootstraps, work/concurrency observations, qualification outputs,
inherited failures and source/backend closure. Executables and libraries are
represented by immutable build/link/hash receipts; Objects are included.

Candidate/test/diagnostic sources are minimal overrides over the two published
bases. The complete 246-file V3 vendor closure includes non-Rust ISLE and package
inputs, and all 164 literal includes are checked. If the optional published IR349
alias exists locally, the verifier additionally compares its full source tree
against the reconstructed qualified snapshot.
