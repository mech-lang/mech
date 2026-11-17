# PR #993 current status

The deployment comparison uses compiler source `fcb818e6420828a19cf019023f0f765a938c351e` on PR #993,
stacked on #973 at `9087b1f07727a9bfcb7c09059f48edaca6dc2301`.
The host is Apple M1, macOS 15.6.1, arm64.
[Machine-readable measurements and per-case changes](scorecard.json)

## Current matched deployment build

The eight balanced comparisons use prebuilt compilers and fixed host support.
Peer executables use the same CLI, allocator, target and strip rules.

| Measurement | Mech | Roc | libdeflate |
| --- | ---: | ---: | ---: |
| Source through final link | 2.80251 s | 2.98347 s | 0.47961 s |
| Source through strip | 2.81307 s | 2.99371 s | 0.49005 s |
| Generated encoder text | 71,768 B | 57,696 B | 37,080 B |
| Stripped deployment file | 292,576 B | 191,920 B | 155,760 B |
| Compiler peak RSS | 395,051,008 B | 392,110,080 B | 71,532,544 B |

The adjacent cleanup verification change reduces paired source-through-link
build time by **1.40%**. Native preparation through
object generation falls from **1.464 s** to
**1.425 s**. The matched time follows the same
link boundary at **0.941× Roc** and **5.842× libdeflate**.
Encoder text is **1.244× Roc** and **1.935× libdeflate**. Its checked-root
symbol extent is **51,264 B**. The bytecode, encoder object, metadata and
benchmark worker reproduce byte for byte. The normalized timing output name
changes the signing record by eight bytes.
[Matched compiler timings and identity checks](evidence/phi-verifier-reuse/README.md)
· [Previous matched compiler build](evidence/scoped-function-replay/README.md)
· [Checked-root symbol extents](evidence/scoped-function-replay/root-census.json)

Across four fresh compiler profiles, Mech reports
**1.020 s** for its
lossless parser and index construction. Roc reports **3 ms** for summed
tokenization and parsing with one compilation job. The reported intervals
are approximately **340×** apart.
[Separate reported parser phase comparison](evidence/phi-verifier-reuse/parser-phases/README.md)

The general-byte-read outline emits **71,420 B**. It keeps every peer win
from the paired control in both complete orders. Both workers score 60/81
against libdeflate in reverse when `prefix-periodic-1024` crosses parity.
[Checked byte reads](evidence/checked-byte-read-regions/README.md)

## Which build to use

| Role | Encoder text | Status |
| --- | ---: | --- |
| Historical rebuild default | 72,520 B | `rebuild.py --variant baseline`, compiler opt level 1 |
| Fixed comparison reference | **71,768 B** | Opt-in recipe below and exact linked worker `674d88602946…` |
| Latest fully timed size candidate | 71,420 B | Checked byte reads. Both reverse workers scored 60/81 libdeflate |
| Earlier owner-exchange size candidate | 71,288 B | Rejected by the v2 runtime gate |
| Current compiler source | **71,768 B** | Guarded parser replay and adjacent cleanup verification |

~~~sh
python3 -B benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py \
  /tmp/mech-reference-71768 --variant fresh-owner-counts-cold-tails-zero-branches \
  --compiler-opt-level 3 --direct-source --preselect-root-inlining --skip-refused-general-ssa
~~~

The fixed object is `7bf9df1aa610ba130490c1092db15fceeace77609c16a60d35ff04e47a48ab7b`.
[Exact recipe and build receipts](evidence/refused-general-ssa/README.md)

## Earlier build with the benchmark host

These measurements are pinned to the earlier evidence revision `dae46c686703560230150a93f36c44c4978ffdc7`.
They include the original benchmark executable and its larger host support.
[Earlier source-through-link comparison](evidence/refused-general-ssa/README.md)

| Benchmark dimension | Fixed Mech | Roc | libdeflate |
| --- | ---: | ---: | ---: |
| Historical benchmark source through final link | 3.73777 s | 2.98844 s | 0.48067 s |
| Generated encoder text | 71,768 B | 57,696 B | 37,080 B |
| Complete encoder object | 159,384 B | 79,160 B | 52,128 B |
| Linked executable text | 474,692 B | 150,416 B | 129,900 B |
| Benchmark executable file | 956,056 B | 230,600 B | 210,168 B |
| Compiler peak RSS | 401,498,112 B | 395,739,136 B | 71,499,776 B |

The fixed worker's measured private payload peak is 10,548,878 B. Its
checked root frame is 3,728 B and the full measured stack high-water is
5,280 B. A study that claims stack nonregression should pin its bound
explicitly in its plan. [Reference runtime receipts](evidence/bound-71768-runtime/README.md)

## Runtime and integration

| Campaign | Forward Roc/C | Reverse Roc/C | Runtime gate |
| --- | --- | --- | --- |
| Exact 71,768-byte worker vs 71,960-byte immediate control | 81/61 | 81/61 | v2 runtime Retain on the archived orders. Historical v1 Unresolved |
| Exact 71,768-byte worker as contemporaneous control in owner exchange | 81/61 | 81/60 | Absolute C floor missed in reverse |
| 71,288-byte owner exchange vs the fixed 71,768-byte control | 81/61 | 81/60 | v2 Reject, with one C win lost in reverse |

[Exact reference v2 replay](evidence/reference-v2/README.md)

The reference's historical case intervals and the candidate's slowdown
diagnostics have separate entries in the scorecard. The owner-exchange candidate
loses `random-1024` and gains `320-overlap` in reverse.
[Complete v2 candidate evidence](evidence/owner-exchange-71288-v2/README.md)

The journal module layout is repaired and all syntax unit tests pass. The
Related Posts after-4 binding has static proof and awaits compiled validation.
[Checked after-4 binding](evidence/related-post-linked-binding/checked-after-4/README.md)
The optimized compact path is diagnostic. Finite admission, cumulative
accounting and recovery under the ordinary entry remain required. The bounded control-drift check recovered all four affected periodic wins in
both orders. The earlier transient's cause remains open.
[Fixed-worker sentinel diagnostic](evidence/fixed-reference-drift/README.md)
[CI for the measured source and evidence](https://github.com/mech-lang/mech/actions/runs/44118007456)
· [Related Posts binding audit](evidence/related-post-linked-binding/README.md)

## Active work

- **Parser.** Function replay applies all 369 recipes in the pinned source. The standalone parser now takes 1.009 s.
  The post-variable-replay trace identifies current function-family cost
  separately from already-removed nested work. The next largest replay
  family in that trace is the enclosing statement at 165 ms inclusive.
  Measure the remaining cost and applicable bounds before extending it.
  [Post-variable profile and both ablations](evidence/scoped-function-replay/README.md)
  [Earlier variable replay](evidence/scoped-definition-replay/README.md)
- **Native compiler.** Attribute plan/semantic preparation, lazy facts,
  SSA construction, cleanup, Cranelift and worker waits separately. Share
  one immutable facts owner across read packing and branch analysis. Keep
  phase choices and publication order exact when omitting a body.
  [First matched source change](evidence/shared-program-facts/README.md)
  The exclusive trace measures 0.692 s in CFG cleanup and 0.280 s in SSA
  construction. The facts change removes one graph and 11 definition
  initializations, about 1 ms in the trace. The scoped demand census finds
  14 scalar sibling reads with protected boundaries. A checked range-proof
  chain exposes about 3.2 ms of CFG and dominator recomputation. Park both
  narrow changes and inspect the next exact-choice omitted-body case. [Scoped native diagnoses](evidence/checked-root-work/README.md)
  [Exclusive native cost and raw traces](evidence/native-exclusive-work/README.md)
  The adjacent cleanup now reuses verification when the phi sweep edits
  no formals. [Matched verifier change](evidence/phi-verifier-reuse/README.md)
- **Integration and qualification.** Compile and test the checked Related
  Posts producer binding. The 24-block control-drift diagnostic recovers the
  four affected peer wins in both orders. The archived reference has a v2
  runtime Retain decision.

Initial diagnosis is bounded to roughly 60–90 minutes and the first
implementation to half a day. Park a direction after two flat or adverse
iterations unless new evidence changes its expected benefit. Review
the portfolio after two experiments or a substantial decision.

## Parked work

| Item | Reopen when |
| --- | --- |
| Tiny-case padding or clock-resolution tuning | A measured peer win or material control drift depends on it under v2 |
| Unchanged stale-root batching | A distinct mechanism can account for the prior size and random-data costs |
| Equivalent-vector or cached-join follow-ups | An exclusive elapsed-time profile shows materially removable work |
| Rescreening the smaller historical objects | A specific source change can recover its measured peer losses |
| Additional parser replay families | The post-change profile identifies enough repeated work and conservative admission coverage |
| Wider Related Posts historical probes | The checked producer route is available and each archive identity can be preserved |
| Scorer lowering and table-slot partition selection | A runtime slot reopens after the compiler investigation, with the separate source and causal ablation ready. [Partition evidence](evidence/match-cache-partition-cause/README.md) |
