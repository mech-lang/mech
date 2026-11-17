# PR #993 current status

The benchmark evidence is at review head `dae46c686703560230150a93f36c44c4978ffdc7`,
stacked on #973 at `9087b1f07727a9bfcb7c09059f48edaca6dc2301`.
GitHub's integration commit for that head is `ee91fed7880010209c0b166258b4d661b028bf13`.
The matched build and runtime host is Apple M1, macOS 15.6.1, arm64.
[Machine-readable measurements and per-case changes](scorecard.json)

## Which build to use

| Role | Encoder text | Status |
| --- | ---: | --- |
| Historical rebuild default | 72,520 B | `rebuild.py --variant baseline`, compiler opt level 1 |
| Fixed comparison reference | **71,768 B** | Opt-in recipe below and exact linked worker `674d88602946…` |
| Latest fully timed size candidate | 71,288 B | Shared owner exchange rejected by the v2 runtime gate |
| Current compiler investigations | — | Structural parser replay and native work attribution |

~~~sh
python3 -B benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py \
  /tmp/mech-reference-71768 --variant fresh-owner-counts-cold-tails-zero-branches \
  --compiler-opt-level 3 --direct-source --preselect-root-inlining --skip-refused-general-ssa
~~~

The fixed object is `7bf9df1aa610ba130490c1092db15fceeace77609c16a60d35ff04e47a48ab7b`.
[Exact recipe and build receipts](evidence/refused-general-ssa/README.md)

## Measurements

| Benchmark dimension | Fixed Mech | Roc | libdeflate |
| --- | ---: | ---: | ---: |
| Source through final link | 3.73777 s | 2.98844 s | 0.48067 s |
| Generated encoder text | 71,768 B | 57,696 B | 37,080 B |
| Complete encoder object | 159,384 B | 79,160 B | 52,128 B |
| Linked executable text | 474,692 B | 150,416 B | 129,900 B |
| Benchmark executable file | 956,056 B | 230,600 B | 210,168 B |
| Compiler peak RSS | 401,498,112 B | 395,739,136 B | 71,499,776 B |
| Stripped deployment file | 292,576 B | 191,920 B | 155,760 B |

The benchmark build spends 1.89649 s parsing and 1.46648 s in native
emission. The experimental deployment link uses the same encoder and a Rust
`core` and `alloc` host. Peer links use the same C driver, mimalloc, target
and strip rules. Generated text, linked text, file bytes, and RSS have
separate measurement boundaries. [Matched builds](evidence/refused-general-ssa/README.md)
· [Common deployment](evidence/minimal-deployment/README.md)

| Matched deployment build, prebuilt compilers and support | Mech core and alloc | Roc | libdeflate |
| --- | ---: | ---: | ---: |
| Source through final link | 3.73555 s | 2.99608 s | 0.48811 s |
| Source through strip | 3.74587 s | 3.00635 s | 0.49889 s |

The smaller Mech host is 1.245× Roc and 7.666× libdeflate in the paired
source-through-link runs. Its paired build time is 0.39% lower than the
std-backed deployment link and its final link takes 35.56 ms versus
42.75 ms. The normalized output name in these timing runs changes the
ad hoc signing record by eight bytes.
[Core and alloc source, link and checks](evidence/alloc-only-deployment/README.md)
· [All 32 deployment build measurements](evidence/alloc-only-deployment/measurements/source-through-link-timing.json.gz)

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
diagnostics have separate entries in the scorecard. The latest candidate
loses `random-1024` and gains `320-overlap` in reverse.
[Complete v2 candidate evidence](evidence/owner-exchange-71288-v2/README.md)

CI remains open. The journal file/directory collision has been repaired locally and all syntax
unit tests pass. The Related Posts producer binding remains open.
The optimized compact path is diagnostic. Finite admission, cumulative
accounting and recovery under the ordinary entry remain required. A bounded
control-drift sentinel check comes before another full promotion campaign.
[CI at the recorded integration commit](https://github.com/mech-lang/mech/actions/runs/44100619431)
· [Related Posts binding audit](evidence/related-post-linked-binding/README.md)

## Active work

- **Parser.** Measure the repeated winner execution in BestChoice and choose
  one expensive structural family for a conservative replay summary. Preserve
  event order and fresh IDs, diagnostics and recovery, longest-match and tie
  behavior, finite grants, streaming and segmented/UTF-8 cases. Recheck the
  parser after the replay change before allocating a tree project.
  The frozen parser trace attributes 1.142 s to BestChoice winner re-execution
  within 1.843 s of grammar work. The first family is variable definitions
  at 0.643 s exclusive. [Parser attribution](evidence/best-choice-structural-replay/README.md)
- **Native compiler.** Attribute plan/semantic preparation, lazy facts,
  SSA construction, cleanup, Cranelift and worker waits separately. Share
  one immutable facts owner across read packing and branch analysis first.
  Prototype demanded scalar payload fields only with a bounded fixed point
  and the original all-fields fallback. Keep phase choices and publication
  order exact while separating selection from body construction.
  [First matched source change](evidence/shared-program-facts/README.md)
- **Integration and qualification.** Finish the Related Posts producer
  binding and run the bounded control-drift diagnostic before any new full
  promotion campaign. The archived reference has a v2 runtime Retain decision.

The measured build spends 1.896 s parsing, 0.233 s in source semantics,
1.466 s on native preparation through object, and 0.063 s in catalog
construction. Artifact, encode, link and remaining work total about
0.079 s. Use these proportions to prioritize the next experiment.

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
| Parser implementation after the current exclusive profile | A slot opens and measured removable work justifies the build-time effect |
| Wider Related Posts historical probes | The checked producer route is available and each archive identity can be preserved |
| Scorer lowering and table-slot partition selection | A runtime slot reopens after the compiler investigation, with the separate source and causal ablation ready. [Partition evidence](evidence/match-cache-partition-cause/README.md) |
