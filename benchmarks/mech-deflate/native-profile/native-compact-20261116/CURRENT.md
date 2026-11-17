# PR #993 current status

The compiler source is `033d17ce9cfd66b185e8fc45655575a971a86af9` on PR #993, stacked on #973 at
`9087b1f07727a9bfcb7c09059f48edaca6dc2301`. The comparison uses Apple M1, macOS 15.6.1, and arm64.
[Machine-readable scorecard](scorecard.json)

## Current matched deployment build

| Measurement | Mech | Roc | libdeflate |
| --- | ---: | ---: | ---: |
| Source through final link | 2.70063 s | 2.98958 s | 0.48058 s |
| Source through strip | 2.71108 s | 3.00001 s | 0.49107 s |
| Encoder text | 71,192 B | 57,696 B | 37,080 B |
| Stripped executable used in timing | 292,568 B | 191,912 B | 155,752 B |

The current build through linking is **0.903× Roc** and
**5.621× libdeflate**. Mech encoder text is
**1.234× Roc** and **1.920× libdeflate**. Both fixed
host support and the compilers are prebuilt.
[Ablation, build times and exact outputs](evidence/unused-failure-returns/README.md)

The 71,192 B worker records forward 81/61, reverse 81/61 in the two full orders against Roc/libdeflate. Runtime policy v2 records **Retain** for the
frame-copy candidate. The 71,420 B read outline still has corrected contract
checks and pending runtime qualification.
[Frame-copy evidence](evidence/frame-copy-v2/README.md)
· [Corrected read-outline checks](evidence/checked-byte-read-regions/README.md)

The new phase profile attributes **883.5 ms**
to grammar execution and **34.0 ms**
to indexing the syntax tree. The earlier paired phase captures measured
**918 ms Mech** and **3 ms Roc** under their respective parser boundaries.
[Mech phase attribution](evidence/post-statement-parser-profile/README.md)
· [Paired comparison with Roc](evidence/scoped-statement-replay/parser-phases/summary.json)

## Which build to use

| Role | Encoder text | Status |
| --- | ---: | --- |
| Historical rebuild default | 72,520 B | `rebuild.py --variant baseline`, compiler opt level 1 |
| Fixed comparison reference | **71,768 B** | Opt-in recipe below and exact linked worker `674d88602946…` |
| Checked-read size candidate | 71,420 B | 5,328 B peak stack. Runtime qualification pending |
| Earlier owner-exchange size candidate | 71,288 B | Rejected by the v2 runtime gate |
| Current qualified size recipe | **71,192 B** | Frame copies with compiler source below |

~~~sh
python3 -B benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py \
  /tmp/mech-frame-copies-71192 --variant frame-copies \
  --compiler-opt-level 3 --direct-source --preselect-root-inlining --skip-refused-general-ssa
~~~

The fixed reference object is `7bf9df1aa610ba130490c1092db15fceeace77609c16a60d35ff04e47a48ab7b`.
The frame-copy object is `96696eeed083f0f11b51a8b26a01fe9077735821f65e7f08decea56881c6329b`.
[Frame-copy worker verification](evidence/frame-copy-v2/README.md)
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
| 71,192-byte frame-copy worker vs the fixed 71,768-byte worker | 81/61 | 81/61 | v2 Retain |
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

- **Parser.** The current diagnostic samples put base-rule evaluation at
  19.8% and literal scanning at 16.1%. Expected-side segmentation takes
  1.65 million Unicode calls across 164 fixed literals. The next candidate
  can reuse that invariant work while charging the original allowance.
  [Parser profile](evidence/post-statement-parser-profile/README.md)
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
