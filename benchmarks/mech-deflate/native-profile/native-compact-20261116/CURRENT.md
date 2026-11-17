# PR #993 current status

Maintained compiler source `204656da92013ce24dabb115685a75fc930c7c46` is on PR #993, stacked on #973 at
`9087b1f07727a9bfcb7c09059f48edaca6dc2301`. These results use Apple M1, macOS 15.6.1 and arm64.
[Machine-readable scorecard](scorecard.json)

## Current matched deployment build

| Apple M1 | Mech | Roc | libdeflate |
| --- | ---: | ---: | ---: |
| Source through final link | 2.57808 s | 2.99395 s | 0.48620 s |
| Source through strip | 2.58878 s | 3.00439 s | 0.49685 s |
| Encoder text | 71,192 B | 57,696 B | 37,080 B |
| Stripped executable | 292,568 B | 191,912 B | 155,752 B |

Mech saves a paired **33.8 ms** through final linking and builds
in **0.861× Roc** and **5.297× libdeflate**
time. Its encoder text is **1.234× Roc** and
**1.920× libdeflate**. Compilers, allocator and fixed
deployment support are prebuilt. The identical worker carries the existing
**81/81 Roc** and **61/81 libdeflate** runtime results in both full orders.
[Reused flow analyses and all matched builds](evidence/verified-flowgraphs/README.md) ·
[Runtime qualification](evidence/frame-copy-v2/README.md)

The preceding dedicated parser-phase comparison reports **813 ms Mech**
and **3 ms Roc**. Mech's interval includes indexing and its strict check.
[Parser-phase plan and observations](evidence/sealed-ascii-graphemes/README.md)

## Whole-build budget

| Stage | Current | 2.0 s budget | 1.0 s budget | 0.45 s budget |
| --- | ---: | ---: | ---: | ---: |
| Parser | 0.815 s | 0.75 | 0.35 | 0.13 |
| Native preparation through object | 1.386 s | 0.95 | 0.48 | 0.22 |
| Source semantic frontend | 0.233 s | 0.18 | 0.09 | 0.04 |
| Catalog initialization | 0.066 s | 0.05 | 0.02 | 0.01 |
| Artifact, encoding, link and other work | 0.074 s | 0.07 | 0.06 | 0.05 |
| **Total** | 2.578 s | 2.00 | 1.00 | 0.45 |

Mech needs **2.128 s**
less elapsed time to reach the 0.45 s working budget. The measured compile
change carries over the existing score. A runtime gain is needed to restore
62/81 against libdeflate.

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

## Runtime scorer study

The caller-address and layout experiment reduces encoder text by **144 B**
to **71,048 B**, or **1.231× Roc** and
**1.916× libdeflate**. Its full runtime orders
and checked-failure receipts are below. The caller-address candidate increases measured peak stack from **5,280 B to 5,296 B**, above the study's
nonregression bound. Both candidates preserve existing individual wins in their respective
screens. The measured deployment build uses the qualified frame-copy recipe.
The forward result sets 62/81 as the next libdeflate floor. The next target
is 63/81 while holding 81/81 Roc.

| Scope | Order | Control wins vs Roc/C | Candidate wins vs Roc/C | Control 8 KiB vs C | Candidate 8 KiB vs C |
| --- | --- | ---: | ---: | ---: | ---: |
| Focus | Forward | 15/12 | 15/13 | 1.0293× | 0.9941× |
| Focus | Reverse | 15/12 | 15/12 | 1.0427× | 1.0025× |
| Full | Forward | 81/61 | 81/62 | 1.0336× | 0.9976× |
| Full | Reverse | 81/61 | 81/61 | 1.0418× | 1.0055× |

[Raw runtime observations, checked stack and compiler patch](evidence/caller-address-scorer/README.md)

## Active work

- **Native compiler.** Close graph reuse for freshly built and post-phi
  checks. Look for compilation work that can be avoided before constructing
  specialized bodies and their proof contexts. The current measurements give
  88.8 ms for tail layout and 77.7 ms for guard derivation.
  [Graph census](evidence/verified-flowgraphs/diagnostics/summary.json) ·
  [Native preparation timings](evidence/native-pass-attribution/README.md)
- **Parser.** Derive a bounded failure prefix for identifier-symbol choices
  from their canonical definitions. The census identifies 134,387 eligible
  identifier-symbol arms. Preserve exact parser work, limits and cache order
  and measure the ordinary parser separately from instrumentation.
  [Parser census](evidence/verified-flowgraphs/parser-choice-census/report.json)
- **Runtime and emitted size.** The nearest missed win is natural 8 KiB,
  about 4% away from parity. Price repeated work in the existing composed
  scorer. Retain the existing checked carrier and validate both 81-case
  orders for any changed object. [Case and atom bindings](evidence/verified-flowgraphs/runtime-target.json)
- **Source semantic frontend.** Attribute declaration registration,
  body/name/overload lowering, schema and constant finalization, and
  requirement canonicalization by the 2.0 s milestone review.

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
