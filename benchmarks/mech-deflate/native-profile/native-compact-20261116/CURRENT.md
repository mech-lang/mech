# PR #995 current status

## Current Apple M1 result

The smallest peer-qualified opt-in source recipe is **64,444 B**, or **1.117× Roc** and **1.738× libdeflate**. It saves **208 B** from the 64,652 B recipe and uses **4,656 B** peak stack. Both full orders record **81/81 Roc and 62/81 libdeflate point wins**, covering 74/74 and 56/74 distinct contracts. Reverse precision supports 61 C wins. The replay records two established control slowdowns and ten uncertain cases. The frame-copies control remains 71,192 B.

The named recipe is `m1-literal-frame-cleanup`, with `MECH_COMPACT_FINISH_CFG_CLEANUP=batched-routes`, built from `caa8b22ac5cd4d7c3cf30b5c97f78fd786fe5c16`. It reproduces the exact qualified encoder, bytecode, full metadata and archived worker. The public rebuild also links a new worker and passes 162 output checks, six injected failure cases and 12 recovery calls. Its new worker identity is recorded separately from the benchmark worker.

| Apple M1 build | Encoder text | Source to link | Source to strip | Source CPU | Source peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Heap-layout control | 64,652 B | 2.169865 s | 2.180720 s | 2.198015 s | 392.5 MB |
| Full final cleanup | 64,444 B | 2.280134 s | 2.290961 s | 2.309317 s | 392.8 MB |
| Batched final cleanup | 64,444 B | 2.263127 s | 2.273977 s | 2.291872 s | 392.8 MB |
| Roc | 57,696 B | 2.977284 s | 2.988122 s | 2.935612 s | 392.2 MB |
| libdeflate | 37,080 B | 0.480397 s | 0.491348 s | 0.442003 s | 71.5 MB |

Ten balanced fresh builds per actor use the same prebuilt deployment support. The selected recipe takes **0.7603× Roc** and **4.7158× libdeflate** build time. Batching saves a paired median **17.200 ms** from full cleanup, with 8/10 faster, but still costs **94.835 ms** versus 64,652 B. This is a size win with a compilation-time cost. The stripped deployment stays **276,072 B**.

Compiler phases are 0.706344 s parsing, 0.212691 s source semantics, 0.009339 s artifact/encoding, 0.000017 s catalog join wait and 1.272847 s native emission. The complete build is 0.263127 s above 2.0 s, 1.263127 s above 1.0 s and 1.813127 s above 0.45 s.

[Source, all timings and tradeoffs](evidence/encoder-size-gap/shared-literal-frame-cleanup/README.md) · [Raw build comparison](evidence/encoder-size-gap/shared-literal-frame-cleanup/cleanup-recovery/build-comparison.json.gz) · [Source rebuild checks](evidence/encoder-size-gap/shared-literal-frame-cleanup/source-rebuild.json)

## Apple M3 upstream integration

The compiler changes are rebased onto `ca0c778e58618c72e6bd13e7df03d3089d4ec2e3`. The combined encoder contains **63,868 B** of text and uses **4,768 B** peak native stack. In the fresh complete comparison, both the previous and rebased M3 executables record **81/81 Roc in both orders, 58/81 libdeflate forward and 60/81 reverse**. The original upstream deployment recipe reproduces its encoder object, complete metadata and bytecode byte for byte with the combined compiler.

[Complete integration results and reproduction](evidence/apple-m3-rebase-20261118/README.md) · [All timings in seconds and comparator multiples](evidence/apple-m3-rebase-20261118/timings.csv)

The latest shared compiler also contains the M1 heap-setup work and the M3 literal-allocation setup pass. It reproduces the named **64,652 B M1 encoder**, complete metadata and bytecode exactly. That common recipe passes the linked M3 correctness, failure, cleanup and stack checks; M3 timing is pending. Five further combined builds are ready. A single final configuration has not yet been qualified on both machines. [Shared-source verification and pending comparisons](evidence/apple-m3-win-recovery-20261118/integration/README.md) · [Recovery trials and completed later orders](evidence/apple-m3-win-recovery-20261118/README.md)

## Apple M3 comparison before upstream integration

The retained word-scan and current-guard optimization records **81/81 against Roc and 61/81 against libdeflate 1.26 level 12 in both orders** in its [complete M3 confirmation](evidence/apple-m3-compact-packets-20261118/word-scans/confirmation/README.md). Encoder text is **64,512 B** and native stack high-water is **4,736 B**, below the unchanged 5,280 B bound. The original Mech source, bytecode, runtime, allocator and peer executables are unchanged.

[All 81 timings, both orders, in seconds and comparator multiples](evidence/apple-m3-compact-packets-20261118/word-scans/confirmation/timings.csv) · [Source and reproduction recipe](evidence/apple-m3-compact-packets-20261118/word-scans/README.md)

## Earlier source-built Apple M1 references

PR #995's integrated compiler at `90d133253a` builds the three named M1 recipes. Each reproduces its archived encoder object, canonical bytecode and complete metadata. The fixed link inputs reproduce all three benchmark and stripped deployment executables. The named command rebuilds its host support and verifies each new worker using 162 success observations, six checked failure cases and 12 exact recovery calls. [Source-built commands](README.md#source-built-m1-reference-encoders) · [Identities and checks](evidence/m1-source-built-20261118/result.json) · [Exact archive worker links](evidence/m1-source-built-20261118/archive-workers-rebased.json)

| Apple M1 build | Encoder text | Source to link | Source to strip | Source CPU | Source peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Mech frame-copies control | 71,192 B | 2.321 s | 2.332 s | 2.361 s | 394.1 MB |
| Mech checked regions | 65,516 B | 2.172 s | 2.183 s | 2.204 s | 392.7 MB |
| Mech compact match equality | 65,276 B | 2.173 s | 2.184 s | 2.205 s | 392.7 MB |
| Mech shared release setup | 65,036 B | 2.176 s | 2.187 s | 2.207 s | 392.7 MB |
| Roc | 57,696 B | 2.983 s | 2.994 s | 2.941 s | 392.9 MB |
| libdeflate | 37,080 B | 0.480 s | 0.491 s | 0.442 s | 71.5 MB |

Twelve balanced fresh builds per recipe use prebuilt deployment support. Shared release setup takes **0.729× Roc** and **4.532× libdeflate** time. It saves a paired median **145.4 ms** against frame-copies. Its encoder text is **1.127× Roc** and **1.754× libdeflate**. The stripped executable is 276,056 B and peak native stack is 4,784 B. [Raw trials and compiler phases](evidence/m1-source-built-20261118/matched-rebased-observations.json.gz)

Shared release setup spends 0.706 s parsing, 0.217 s in source semantics, 0.009 s creating and encoding the artifact and 1.179 s in native emission. Exposed catalog wait is 18 µs. The remaining 0.062 s includes 0.035 s of final link. The complete build is 0.176 s above 2.0 s, 1.176 s above 1.0 s and 1.726 s above 0.45 s. [Phase accounting](evidence/m1-source-built-20261118/result.json)

Frame-copies remains the maintained control. All three named recipes remain opt-in. Each exact archived worker records 81/81 Roc and 62/81 libdeflate wins in each full order, covering 74/74 and 56/74 distinct contracts. The fresh comparison with frame-copies found four cases with established slowdowns in both orders for checked regions and six for compact match equality. Shared release setup still needs a direct runtime comparison with frame-copies. [Runtime qualification](evidence/m1-source-built-20261118/result.json) · [Paired intervals and matcher diagnosis](evidence/m1-source-built-20261118/maintained-runtime/README.md)

The 64,668 B shared-heap trial missed one Roc win in the forward sweep.
Its **64,652 B** successor passes both full M1 sweeps at **81/81 Roc and
62/81 libdeflate**. The previously lost case runs at 0.969× Roc forward
and 0.967× reverse. A fresh source-to-final-link build in the recovery
cohort takes **2.173 s**, or **0.728× Roc** and **4.522× libdeflate**.
Both heap recipes are opt-in. [Original trial](evidence/encoder-size-gap/shared-heap-call-screen/README.md) · [Smaller qualified worker and build timings](evidence/encoder-size-gap/shared-heap-layout-recovery/README.md)

The current native profile attributes the largest traced intervals to root CFG cleanup and phase construction. Retirement-transition caching reuses 3.1% of transitions and adds a paired median 4.7 ms to the complete build. It stays experimental. [Native spans and paired prototype results](evidence/m1-source-built-20261118/native-attribution/README.md)

The public CLI, hosted native applications and same-binary native policy controls passed on the integrated source. Static architecture contracts still fail on the Related Posts linked-module binding in `hosts/after-4.rs`. The earlier hosted checkout timeout occurred before compiler validation. [CI at 90d133253a](https://github.com/mech-lang/mech/actions/runs/44344708598)

## Earlier shared-release matched deployment

PR #993 tracks the qualified LLVM-free MechDeflate build and is stacked on #973. The measurements below use Apple M1, macOS 15.6.1 and arm64.

| Apple M1 | Mech | Roc | libdeflate |
| --- | ---: | ---: | ---: |
| Source through final link | 2.175 s | 2.977 s | 0.480 s |
| Source through strip | 2.186 s | 2.988 s | 0.490 s |
| Encoder text | 65,036 B | 57,696 B | 37,080 B |
| Stripped executable | 276,056 B | 191,912 B | 155,752 B |

Mech builds in **0.732× Roc** and **4.553× libdeflate** time. Encoder text is **1.127× Roc** and **1.754× libdeflate**. Full orders record 81/81 and 81/81 Roc wins, with 62/81 and 62/81 libdeflate wins.

[Matched measurements and source replay](evidence/encoder-size-gap/shared-release-setup/README.md) · [Scorecard](scorecard.json) · [Policy v3](qualification-policy-v3.json)

An earlier dedicated parser-phase comparison reported **813 ms Mech**
and **3 ms Roc**. Mech's interval includes indexing and its strict check.
[Earlier Mech and Roc parser-phase observations](evidence/sealed-ascii-graphemes/README.md)

## Earlier encoder code-size studies

In the earlier 65,612 B checked-address build, the checked entry accounts for **48,220 B**. The match collector adds **6,224 B**, and the dense scorer adds **1,104 B**. The matched libdeflate object emits **37,080 B**.
[Qualified encoder](evidence/encoder-size-gap/select-checked-default-address/README.md) · [Historical byte attribution](evidence/encoder-size-gap/README.md)

An experimental capacity refusal reuses the checked continuation and
emits **69,528 B**. Both focused orders preserve their peer wins.
The candidate's peak stack is **5,296 B** and natural 8 KiB still loses
to libdeflate.
[Source patch and measured results](evidence/encoder-size-gap/shared-growth-refusal/README.md)

Collector-only packet packing and the shared growth fallback emit
**68,840 B**, with **4,624 B** measured peak stack. Both focused
orders preserve their peer wins. Natural 8 KiB measures **1.00064×**
and **1.00262× libdeflate**. The maintained qualification remains
on the 71,192 B worker.
[Compiler patch, exact replay and comparisons](evidence/encoder-size-gap/scoped-phase-packets/README.md)

Reusing the checked continuations for two more geometry refusals emits
**68,136 B** with **4,768 B** peak stack. The complete orders measured
**81/81 and 81/81 Roc wins** and **61/81 and 61/81 libdeflate wins**.
Natural 8 KiB measured **1.00077×** and **1.00198× libdeflate**.
[Exact replay and complete comparisons](evidence/encoder-size-gap/checked-layout-refusals/README.md)

Keeping the sorter interfaces and trimming other phase results emits
**67,464 B**. In the complete orders it recorded
**81/81 and 81/81 Roc wins** and
**61/81 and 61/81 libdeflate wins**.
[Source replay and results](evidence/encoder-size-gap/checked-layout-refusals/README.md#preserve-the-private-phase-output-interface)

Forwarding 1,837 empty source jumps removes 1,568 blocks before native layout. The exact paired build drops from **2.427 s to 2.387 s**.
The object emits **67,656 B** with **4,672 B** peak stack. In the
complete orders it records **81/81 and 81/81 Roc wins**
and **61/81 and 61/81 libdeflate wins**.
[Compiler patch, build measurements and complete runtime results](evidence/encoder-size-gap/empty-source-routes/README.md)

The dominating-definition experiments emitted an encoder as small as **67,272 B**. The fastest matched source-to-link time was **2.305 s**.
Scalar and non-owning field definitions measured 81/81 and 81/81 Roc wins and 62/81 and 62/81 libdeflate wins in complete orders. The v3 runtime decision was **Retain**.
[Source replay and measured results](evidence/encoder-size-gap/dominating-definitions/README.md)

Keeping the checked returns available in the caller restores two shared continuation sites and emits **67,672 B**.
The 67,664 B encoder won 81/81 and 81/81 Roc cases and 62/81 and 61/81 libdeflate cases in complete orders. V3 returned **Reject**.
[Compiler replay, size and runtime measurements](evidence/encoder-size-gap/shared-continuation-returns/README.md)

The matcher layout emits **67,648 B** with the shared continuations. Complete orders recorded 81/81 and 81/81 against Roc and 62/81 and 62/81 against libdeflate. V3 returned **Retain**.
[Source replay and runtime results](evidence/encoder-size-gap/continuation-matcher-layout/README.md)

The single dense scorer with a checked fallback emits **66,240 B**. It won 81/81 and 81/81 against Roc and 62/81 and 61/81 against libdeflate. The v3 decision was **Reject**.
[Source patch, replay and complete results](evidence/encoder-size-gap/dense-scorer-only/README.md)

Preparing the admitted scorer first skips two unused native bodies and reproduces the qualified 67,648 B encoder object and worker. The source-to-link median is **2.283 s**.
[Source replay and matched builds](evidence/encoder-size-gap/early-scorer-preparation/README.md)

The dense scorer with direct branches emits **66,192 B**. Complete orders recorded 81/81 and 81/81 Roc wins, with 62/81 and 62/81 libdeflate wins. V3 returned **Retain**.
[Source replay and complete observations](evidence/encoder-size-gap/dense-scorer-direct-branches/README.md)

Folding raw grapheme frames and replaying longer statements builds the exact qualified 66,192 B encoder in **2.195 s** through final link.
[Parser source and matched evidence](evidence/sealed-grapheme-and-statement-replay/README.md)

Sharing checked byte reads emits **65,932 B** and builds in **2.198 s** through final link. Both full orders retained 81/81 Roc wins and the 62/81 libdeflate floor.
[Source replay and benchmarks](evidence/encoder-size-gap/shared-defaulted-byte-reads/README.md)

## MechDeflate compile time by encoder build

Each row measures fresh MechDeflate source through final executable
link with the same fixed deployment support prebuilt. Ratios use
Roc and libdeflate trials from the same session.

| Build | Encoder text | MechDeflate build | × Roc | × libdeflate |
| --- | ---: | ---: | ---: | ---: |
| [Collector-only packets](evidence/encoder-size-gap/scoped-phase-packets/deployment.json) | 68,840 B | 2.434 s | 0.814× | 5.046× |
| [Shared layout refusals](evidence/encoder-size-gap/checked-layout-refusals/deployment.json) | 68,136 B | 2.427 s | 0.814× | 5.053× |
| [One-body preparation](evidence/encoder-size-gap/checked-layout-refusals/single-body-preparation/summary.json) | 68,136 B | 2.421 s | 0.811× | 5.046× |
| [All-phase output pruning](evidence/encoder-size-gap/checked-layout-refusals/single-body-preparation/summary.json) | 67,512 B | 2.426 s | 0.814× | 5.059× |
| [Private sorter interfaces](evidence/encoder-size-gap/checked-layout-refusals/non-private-outputs/deployment.json) | 67,464 B | 2.434 s | 0.813× | 5.051× |
| [Forwarding disabled (control)](evidence/encoder-size-gap/empty-source-routes/deployment.json) | 67,464 B | 2.427 s | 0.813× | 5.032× |
| [Forward empty source jumps](evidence/encoder-size-gap/empty-source-routes/deployment.json) | 67,656 B | 2.387 s | 0.798× | 4.953× |
| [Register-preserving free wrapper](evidence/encoder-size-gap/empty-source-routes/deployment.json) | 67,992 B | 2.447 s | 0.817× | 5.060× |
| [Scalar definitions](evidence/encoder-size-gap/dominating-definitions/scalar/deployment.json) | 67,576 B | 2.422 s | 0.810× | 5.034× |
| [Scalar definitions with forwarded jumps](evidence/encoder-size-gap/dominating-definitions/scalar/deployment.json) | 67,272 B | 2.375 s | 0.794× | 4.939× |
| [Non-owning field definitions](evidence/encoder-size-gap/dominating-definitions/fields/deployment.json) | 68,488 B | 2.323 s | 0.777× | 4.834× |
| [Scalar and non-owning field definitions](evidence/encoder-size-gap/dominating-definitions/fields/deployment.json) | 68,424 B | 2.305 s | 0.771× | 4.787× |
| [Shared cold returns](evidence/encoder-size-gap/shared-continuation-returns/deployment.json) | 67,672 B | 2.304 s | 0.770× | 4.796× |
| [Shared cold returns with one additional continuation family](evidence/encoder-size-gap/shared-continuation-returns/deployment.json) | 67,664 B | 2.302 s | 0.770× | 4.794× |
| [Shared continuations with matcher fallthroughs](evidence/encoder-size-gap/continuation-matcher-layout/deployment.json) | 67,648 B | 2.301 s | 0.771× | 4.789× |
| [Dense scorer with early admission and single-body preparation](evidence/encoder-size-gap/dense-scorer-only/deployment.json) | 66,240 B | 2.295 s | 0.768× | 4.699× |
| [Prepare the admitted scorer first](evidence/encoder-size-gap/early-scorer-preparation/deployment.json) | 67,648 B | 2.283 s | 0.764× | 4.757× |
| [Checked dense scorer with direct branches](evidence/encoder-size-gap/dense-scorer-direct-branches/deployment.json) | 66,192 B | 2.274 s | 0.762× | 4.721× |
| [Folded grapheme frames and longer statement replay](evidence/sealed-grapheme-and-statement-replay/deployment.json) | 66,192 B | 2.195 s | 0.737× | 4.578× |
| [Shared checked byte reads](evidence/encoder-size-gap/shared-defaulted-byte-reads/deployment.json) | 65,932 B | 2.198 s | 0.737× | 4.587× |
| [Select the checked default address](evidence/encoder-size-gap/select-checked-default-address/deployment.json) | 65,612 B | 2.205 s | 0.740× | 4.588× |
| [Forward match collector spills](evidence/encoder-size-gap/matcher-spill-forwarding/deployment.json) | 65,516 B | 2.201 s | 0.740× | 4.586× |
| [Index checked region block order](evidence/encoder-size-gap/indexed-checked-regions/deployment.json) | 65,516 B | 2.190 s | 0.735× | 4.577× |
| [Use 16-byte equality groups in the compact matcher](evidence/encoder-size-gap/compact-match-equality/deployment.json) | 65,276 B | 2.184 s | 0.736× | 4.573× |
| [Skip empty parser transactions for resolved terminal misses](evidence/encoder-size-gap/empty-terminal-rewinds/deployment.json) | 65,276 B | 2.182 s | 0.733× | 4.545× |
| [Share owner-release call setup in the checked entry](evidence/encoder-size-gap/shared-release-setup/deployment.json) | 65,036 B | 2.175 s | 0.732× | 4.553× |

Parser, lowering, native compilation and link timings are in the
machine-readable scorecard.

## Earlier shared-release build budget

| Stage | Current | 2.0 s budget | 1.0 s budget | 0.45 s budget |
| --- | ---: | ---: | ---: | ---: |
| Parser | 0.707 s | 0.75 | 0.35 | 0.13 |
| Native preparation through object | 1.183 s | 0.95 | 0.48 | 0.22 |
| Source semantic frontend | 0.213 s | 0.18 | 0.09 | 0.04 |
| Catalog join wait | 0.000 s | 0.05 | 0.02 | 0.01 |
| Artifact, encoding, link and other work | 0.073 s | 0.07 | 0.06 | 0.05 |
| **Total** | **2.175 s** | 2.00 | 1.00 | 0.45 |

That shared-release build needs **1.725 s** less elapsed time to reach the
0.45 s working budget. The phase medians come from the matched measurements above. Catalog work runs during parsing and the table lists the join wait.

## Which build to use

The 64,668 B trial lost one Roc case in the forward sweep. Its V3 result is Reject. [Runtime, timing and replay](evidence/encoder-size-gap/shared-heap-call-screen/README.md)

| Role | Encoder text | Status |
| --- | ---: | --- |
| Historical rebuild default | 72,520 B | `rebuild.py --variant baseline`, compiler opt level 1 |
| Fixed comparison reference | **71,768 B** | Opt-in recipe below and exact linked worker `674d88602946…` |
| Checked-read size candidate | 71,420 B | 5,328 B peak stack. Runtime qualification pending |
| Earlier owner-exchange size candidate | 71,288 B | Rejected by the v2 runtime gate |
| Maintained frame-copy recipe | **71,192 B** | Compiler source below |
| Smallest v3-qualified opt-in build | **64,444 B** | [Compiler source, timings and tradeoffs](evidence/encoder-size-gap/shared-literal-frame-cleanup/README.md) |

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
[Current #993 checks](https://github.com/mech-lang/mech/pull/993/checks) · [Related Posts after-4 failure](https://github.com/mech-lang/mech/actions/runs/44204215687/job/134466751590) · [Local format and contract checks](evidence/pinned-formatter-repair/README.md)
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

## Scalar entry literals

The checked root bypasses provisional SSA lookups for **1,389**
immutable scalar literal lanes in the measured build.
A paired **40.63 ms** saving brings the matched M1 build to **2.495 s** from source through final link.
The 90% interval is **33.08 to 47.17 ms**.
The frame-copy recipe uses this path.
[Exact build and paired measurements](evidence/immutable-root-literals/README.md)

## Scorer cost length

The shorter scorer uses the caller's admitted minimum and preserves
the full-length check for larger offsets. Natural 8 KiB falls from
**43,074 to 54** stack loads in the placed scorer. The **71,128 B**
encoder wins the forward 8 KiB comparison at **0.99426× libdeflate**
and finishes at **1.00101×** in reverse. The focused comparison keeps
both overlap wins and every other peer win. The candidate remains
experimental on the v3 runtime gate.
[Scorer patch, phase counts and paired observations](evidence/scorer-admitted-cost-length/README.md)

## Source catalog overlap

The owned syntax iterator saves **5.87 ms**
in semantic lowering. Its isolated through-link interval spans zero.
Starting catalog construction during parsing brings the matched build
to **2.427 s**. Enabling overlap in the same compiler saves
**60.88 ms** through final link.
The direct frame-copy build uses the measured recipe.
[Catalog overlap and exact build](evidence/inline-syntax-child-traversal/catalog/README.md)

## Active work

- **Native compiler.** The scalar entry literal comparison is complete.
  Price the remaining repeated SSA work with the root-lane census.
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
