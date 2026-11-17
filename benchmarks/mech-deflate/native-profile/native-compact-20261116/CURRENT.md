# PR #993 current status

Maintained compiler source 1426ea50ba887590c0696f9e3c767a41c5cd847f is on PR #993 stacked on #973.
These measurements use Apple M1, macOS 15.6.1 and arm64.
[Machine-readable scorecard](scorecard.json)

## Current matched deployment build

| Apple M1 | Mech | Roc | libdeflate |
| --- | ---: | ---: | ---: |
| Source through final link | 2.54442 s | 2.99543 s | 0.48343 s |
| Source through strip | 2.55500 s | 3.00609 s | 0.49411 s |
| Encoder text | 71,192 B | 57,696 B | 37,080 B |
| Stripped executable | 292,568 B | 191,912 B | 155,752 B |

Mech builds in **0.849× Roc** and
**5.267× libdeflate** time. Its encoder text is
**1.234× Roc** and **1.920× libdeflate**. Compilers, allocator and fixed
deployment support are prebuilt. The exact worker carries 81/81 Roc and
61/81 libdeflate in its current full qualification. Promotions of new
runtime artifacts require the v3 floor of 62/81 in both orders, with
63/81 as the next target.
[Matched measurements](evidence/ssa-local-definitions/README.md) ·
[Runtime qualification](evidence/frame-copy-v2/README.md) ·
[Policy v3](qualification-policy-v3.json)

An earlier dedicated parser-phase comparison reported **813 ms Mech**
and **3 ms Roc**. Mech's interval includes indexing and its strict check.
[Earlier Mech and Roc parser-phase observations](evidence/sealed-ascii-graphemes/README.md)

## Whole-build budget

| Stage | Current | 2.0 s budget | 1.0 s budget | 0.45 s budget |
| --- | ---: | ---: | ---: | ---: |
| Parser | 0.786 s | 0.75 | 0.35 | 0.13 |
| Native preparation through object | 1.386 s | 0.95 | 0.48 | 0.22 |
| Source semantic frontend | 0.233 s | 0.18 | 0.09 | 0.04 |
| Catalog initialization | 0.058 s | 0.05 | 0.02 | 0.01 |
| Artifact, encoding, link and other work | 0.077 s | 0.07 | 0.06 | 0.05 |
| **Total** | 2.544 s | 2.00 | 1.00 | 0.45 |

Mech needs **2.094 s** less elapsed time to reach the
0.45 s working budget. The phase medians come from the same ten matched trials.

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
[Current #993 checks](https://github.com/mech-lang/mech/pull/993/checks) · [Local format and contract checks](evidence/pinned-formatter-repair/README.md)
· [Related Posts binding audit](evidence/related-post-linked-binding/README.md)

## Runtime scorer study

The caller-address and layout experiment reduces encoder text by **144 B**
to **71,048 B**, or **1.231× Roc** and
**1.916× libdeflate**. Its full runtime orders
and checked-failure receipts are below. The caller-address candidate increases measured peak stack from **5,280 B to 5,296 B**, above the study's
nonregression bound. Both candidates preserve existing individual wins in their respective
screens. The measured deployment build uses the qualified frame-copy recipe.
The experiment reached the 62-win high-water mark in its forward order.
The v3 policy requires 62/81 in both full orders.

| Scope | Order | Control wins vs Roc/C | Candidate wins vs Roc/C | Control 8 KiB vs C | Candidate 8 KiB vs C |
| --- | --- | ---: | ---: | ---: | ---: |
| Focus | Forward | 15/12 | 15/13 | 1.0293× | 0.9941× |
| Focus | Reverse | 15/12 | 15/12 | 1.0427× | 1.0025× |
| Full | Forward | 81/61 | 81/62 | 1.0336× | 0.9976× |
| Full | Reverse | 81/61 | 81/61 | 1.0418× | 1.0055× |

[Raw runtime observations, checked stack and compiler patch](evidence/caller-address-scorer/README.md)

## Scorer and parser experiments

The direct scorer call arguments emit **71,000 B** and pass the linked
checked-failure probes. Peak stack is **5,296 B**, 16 B above the earlier
bound. Both conditional item-selection variants lost peer wins. The
current deployment recipe uses the qualified 71,192 B worker.
[Scorer timing, counter trace and checked stack](evidence/ordered-scorer-visit-costs/README.md)

The parser shortcut preserves all three native artifact hashes and passes
537 syntax tests. Its paired standalone parser saving is
**28.5 ms**.
[Parser shortcut and matched final-link results](evidence/sealed-ascii-literal-miss/README.md)

## Ordered reduction follow-up

A bottom-tested vector loop shrinks encoder text to **71,048 B**
and wins natural 8 KiB in both focused orders without losing a peer case.
The candidate wins 81/81 against Roc in both full orders.
It wins **61/81 and 61/81 against libdeflate**, with natural 8 KiB at
**0.99779× and 1.00436×**.

Restoring the qualified root layout emits **71,096 B**.
The candidate wins 81/81 against Roc in both full orders.
It wins **62/81 and 61/81 against libdeflate**, with natural 8 KiB at
**0.99286× and 0.99866×**.

Its reverse order misses `320-overlap` in both Mech slots.
The linked checks cover all 162 exact outcomes, six checked failures and
a measured peak stack of **5,280 B**.
[Bottom-tested loop build and observations](evidence/ordered-vector-latch/README.md)

The preceding vector-cursor screens eliminated repeated address or lane
work. They established an extra 43,020 scorer stack loads on natural
8 KiB for either address-carrying version. Lane-position deferral alone
kept 5,280 B peak stack and preserved its focused peer wins.
[Vector ablation and counts](evidence/ordered-vector-cursor-ablations/README.md)

## Native compile-time attribution

Incremental branch guard propagation emits exact artifacts and passes
17 guard tests and 162 linked checks. The matched probe is
**4.58 ms slower** than the
maintained compiler through final link. Four compiler profiles put
SSA construction, cleanup and constant-phi work among the larger remaining
native costs. Reusing Tail graph discovery saves **0.19 ms** in the trace.
[Guard propagation](evidence/native-guard-edge-closure/README.md) ·
[Compiler samples](evidence/ordered-vector-cursor-ablations/compiler-sample/)

## Overlap backing and SSA storage

The 32-page word mapping threshold takes about 0.5% longer on
`320-overlap` in both focused orders. Natural 8 KiB is **1.0047×**
libdeflate in reverse. The checked root and its runtime calls
account for about 59% of the sampled native encoder on `320-overlap`.
[Overlap profile and timings](evidence/runtime-overlap-backing/README.md)

The exact bottom-tested scorer repeated at **62/81 and 60/81**
against libdeflate in the complete orders. [Full repeat](evidence/runtime-overlap-backing/root-confirmation/README.md)

Inline single-definition storage measured a paired saving of
**5.88 ms** through final link with exact outputs.
The 90% interval is **0.86 to 17.41 ms**.
[SSA results](evidence/ssa-local-definitions/README.md)

## Short scorer visits

Scalar pairing and earlier four-lane entry use the same admitted windows and
preserve strict winner order. The linked validation covers the complete
81-case fixture set and six checked exits. The 71,192 B frame-copy encoder remains the maintained build. The next runtime mechanism is the repeated bound reload.
[Short-visit results](evidence/ordered-scorer-short-visits/README.md)

## Active work

- **Native compiler.** Price one provisional SSA value family using the
  root-lane census. Direct entry values require dominance and immutability
  on all relevant paths, including ownership, synthetic and failure paths.
  The bounded freshly built and post-phi graph reuse is complete.
  [Current SSA census](evidence/source-and-ssa-attribution/README.md) ·
  [Verified graph reuse](evidence/verified-flowgraphs/README.md)
- **Parser.** Reprice only the remaining outer identifier-choice and frame
  work after the literal-mismatch shortcut. Bound the failure-prefix
  experiment with the current parser and preserve parser event and allowance
  boundaries. [Earlier grammar census](evidence/verified-flowgraphs/parser-choice-census/report.json)
- **Runtime and emitted size.** Recover the `320-overlap` win with margin
  while holding natural 8 KiB. Natural 4 KiB is the next additional win.
  Keep both complete orders and checked carrier evidence under v3.
  [Latest scorer observations](evidence/ordered-vector-latch/README.md)
- **Source semantic frontend.** Use the current exclusive capture to
  select one measured lowering mechanism. Catalog startup, installation
  and validation have separate timings in the same report.
  [Source and catalog attribution](evidence/source-and-ssa-attribution/README.md)

- **Integration and qualification.** Compile and test the checked Related
  Posts producer binding. The 24-block control-drift diagnostic recovers the
  four affected peer wins in both orders. The archived reference has a v2
  runtime Retain decision.

Initial diagnosis is bounded to roughly 60–90 minutes and the first
implementation to half a day. Park a direction after two flat or adverse
iterations unless new evidence changes its expected benefit. Review
the portfolio after two experiments or a substantial decision.

## Completed and parked compiler work

| Area | Decision |
| --- | --- |
| Variable, function and bounded statement replay, including nested observation | Complete |
| Adjacent phi verification reuse | Complete |
| Fresh and post-phi flowgraph reuse | Complete for the bounded cleanup family |
| Inline single SSA definition storage | Retained with a paired 5.9 ms through-link saving |
| Shared immutable facts | Retained and closed at about 1 ms less materialization |
| Declaration snapshots | Closed at about 0.048 ms |
| Minimal deployment without std or JSON | Complete |
| Conservative scalar demand and mixed scalar transport | Parked. The strict model found zero removals. Reopen the 14 protected reads with evidence of material downstream savings |
| Narrow range-pass CFG reuse | Bounded to about 3.2 ms |
| Additional backend workers and initialization-proof rewrite | Deprioritized by worker activity and the 3.8 ms proof ceiling |

[Replay evidence](evidence/scoped-function-replay/README.md) ·
[Verification reuse](evidence/phi-verifier-reuse/README.md) ·
[Native preparation limits](evidence/checked-root-work/README.md) ·
[Minimal deployment](evidence/alloc-only-deployment/README.md)

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
