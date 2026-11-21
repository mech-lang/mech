# Qualified word/complement current-input reads

**Select `m1-word-complement-current-reads`: 63,388 B, preserving all 81 Roc and 62 libdeflate wins in both full orders.** Every previous individual peer win remains supported by the configured confidence checks. This saves **144 B** from the 63,532 B reference. There is still no 63rd C win; the goal remains 1× libdeflate performance.

The original 2,719-line Mech source and canonical bytecode are unchanged. No LLVM dependency is added. The source/compiler basis is `546af95af3735591a77455932dc48920a8fc071d`. The actual timing worker was emitted by compiler `5bcc225bdcf5aea18dbe2c87b1d43284b98d3af804a2589875f9b78c03741157`; compiler `47ee11652095829e8e1e8f0427eb9513614ab4026ab39346444a844fd378c34b` built from the integrated source reproduces the object, complete metadata and bytecode exactly, in normal and register-allocation-checked modes. The source and both compiler overlays are preserved in the preceding [source study](../../vector-mismatch-20261119/README.md).

The recipe combines seeded previous-DP-cost forwarding and cleanup, proved immutable-byte reuse, complementary predicate simplification, scalar word comparisons, and source-bound block layouts. The source's existing scalar word recurrence supplies equality; it is not a machine-code patch or a codec rewrite. The saved-prefix strategy admits no cache interval in this codec, so no cache-reuse benefit is claimed.

## Runtime qualification

Two complete **81-case orders, 96 balanced blocks per case**, pass the v3 policy and retain every prior individual point and confidence-supported win. The 81 named cases represent 74 distinct contracts; C wins cover 56 distinct contracts. The exact worker is `fab303151eca902a95986488684102fc1707ec7f8df1324ebfd9c55543cfc368`. Both earlier 24-block full orders are archived separately; they also held all point wins and are not substituted for the 96-block qualification.

Natural 4 KiB is **1.034702× / 1.043348× C**, versus same-run control **1.063779× / 1.072741× C**. The candidate takes **2.73–2.74% less time** on that case. It still needs **3.35–4.15% less candidate time** to reach C parity. Earlier discovery's lower 1.023–1.033× figures are retained in the source study; they do not replace these full results.

There are **six established slowdowns above the 0.5% resolution**: one-stored, random-256, random-1024, prefix-random-1024, prefix-random-4096 and prefix-random-8192. Four comparisons remain uncertain against the control: empty, seven-stored-prefix, random-64 and prefix-random-33027. The policy permits slowdowns that retain peer wins; these observations are not hidden or used to reject the whole gain. Random 4 KiB remains an important repair target. No new C wins are claimed.

All **162 exact-output/inflation/alias observations, six checked failure cases, twelve injected failure observations and twelve successful recoveries** pass. Peak active stack is **4,864 B**, 16 B above the previous reference and below the unchanged 5,280 B cap. The allocation checker emits exactly the same three artifacts. Each full timing call additionally verifies its outputs and aliases, and each worker reports complete private cleanup.

## Complete build and size

| Actor | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Previous reference, same compiler | 63,532 B | 2.212654 s | 2.223596 s | 276,072 B |
| Selected word/complement | **63,388 B** | **2.214373 s** | 2.225233 s | 276,072 B |
| Roc | 57,696 B | 2.991040 s | 3.001820 s | 191,912 B |
| libdeflate | 37,080 B | 0.482117 s | 0.493106 s | 155,752 B |

Ten balanced serial fresh-process trials per actor include final linking. Compilers and fixed support are prebuilt equally; bootstrap is excluded for every actor and strip is separate. Every Mech trial checks the full generated object, metadata and bytecode. Selected paired ratios are **0.739845× Roc / 4.594875× C**, and 1.001752× the same-compiler previous reference. The small build difference does not establish a compile-time change. Deployment size stays 276,072 B; the 144 B reduction is encoder text. Encoder text is **1.098655× Roc /1.709493× C**.

Phase medians: parsing 704.721 ms, semantics 217.239 ms, artifact preparation 6.836 ms, encoding 2.532 ms, native pipeline 1,220.038 ms and final link 35.474 ms. Catalog work overlaps parsing; separate medians do not add. Build time is distinct from compression latency.

## Reproduce

From this directory, use a compiler built from the source pin above:

```sh
python3 verify.py
python3 reproduce.py /path/to/compile_profile /tmp/word-complement-replay
```

The source replay runs both normal and checked allocation modes and requires the exact object, complete metadata and canonical bytecode. The registered recipe is available from the benchmark root:

```sh
python3 rebuild.py /tmp/mech-word-complement --variant m1-word-complement-current-reads
```

This builds the compiler from the checkout and emits the native encoder without LLVM tools. Use the existing optional allocator argument to link and validate a worker. A rebuilt worker or different hardware requires its own runtime qualification; no score is inherited solely from an artifact-size match.

`raw-runtime.tar.gz` contains all four full point/qualification orders, balanced raw observations, actor pins, cleanup records and completion counts. `checks-and-builds.tar.gz` contains the actual 96-block plan/replay, original individually required wins, output/failure/recovery/stack checks, allocation checker result, complete-build measurements and acquisition/analysis scripts. Archive members, profiles and shipped artifacts are hash pinned. Verification independently reconstructs the full confidence result from raw observations, checks the old named win sets, and checks the exact artifact and worker identities. Both preceding qualified references remain available for comparison.

The relevant repository run now passes all **10 wide-equality tests**, including the first-word paths and unchanged-graph check when saved-state proof is absent. The other 1,350 tests were filtered out; the entire GPU suite is not claimed. The earlier disk-exhausted attempt remains retained. Exact test inputs and logs are in the [review follow-up](../../review-followup-20261119/README.md). The pre-existing missing allowlisted archived allocator still blocks the repository-wide unsafe audit. No ordinary scalar capability is retired.

The admitted first-word probe before vectors remains experimental. A new direct five-actor comparison keeps both finalists: vector/narrow is faster on natural 4 KiB and small random inputs, while word/complement is smaller and stronger on fragile overlap/periodic wins. Scorer winner frequencies and fresh histogram counts are recorded in the review follow-up. Broader collector predicate and geometry work is still needed for the large random losses.

Current source checkpoint: `00765f3be3f20c1a50efd931f3128718ea4c205a`. The disabled-probe compiler reproduces the exact selected linked worker, so its existing qualification transfers by identity. `current-source-relink.json` binds the source, development compiler build and selected worker. The enabled first-word probe remains experimental.

The latest current-source check uses eight balanced complete-build trials: current **2.240053 s**, previous compiler **2.229851 s**, Roc **3.007427 s**, C **0.487046 s**. Current paired ratios are **0.741226× Roc / 4.593452× C** and **1.001272×** the previous compiler producing the identical encoder. The paired time difference and confidence interval are in `current-builds-summary.json`; the current cohort is archived separately. These figures are separate from compiler bootstrap and the earlier ten-trial selection cohort.
