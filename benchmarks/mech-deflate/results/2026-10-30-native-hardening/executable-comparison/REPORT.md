# Mech and Roc executable comparison

The current retained compiler takes **14.5609× Roc's elapsed time**, executes **34.4433× the instructions**, and makes **147.3053× the allocation requests** for the complete eight-byte level-12 case. The latest isolated change prepares fully funded record-lookup prefix charges: matched elapsed improves **1.98944%**, instructions **3.30552%**, cycles **1.98639%**. It retains exactly the original logical charges and paid failure prefixes.

| Per compression | Roc | Mech before | Mech after |
| --- | ---: | ---: | ---: |
| Elapsed | **0.289 ms** | 4.2935205 ms (**14.8565×**) | **4.2081035 ms (14.5609×)** |
| Instructions | **1,364,372** | 48,599,979 | **46,993,498.5** |
| Cycles | **920,543** | 13,551,678 | **13,282,488.5** |
| Allocation requests | **95** | 13,994 | **13,994** |
| Requested bytes | **9,021,923** | 98,586,843 | **98,586,843** |
| Additional peak tracked heap | **9,257,328** | 20,575,432 | **20,575,432** |
| Prepared tracked heap | 32 | 142,706,080 | **142,829,248** |
| Resident memory after | **14,778,368** | 319,668,224 | **319,799,296** |
| Kernel physical footprint after | **14,435,264** | 228,640,512 | **228,771,584** |

All four independently prepared pairs and 60/64 descriptive blocks improve; 1,984 primary calls/version and 20,000 reference calls. Keep the shared reference within this cohort; do not multiply independent earlier gains. Hardware uses 32 steady calls/implementation; allocator counts use eight Mech and 32 reference calls. Timings, counters, allocation traces and samples are separate serialized cohorts. Boundary overhead (about 5,010 instructions) is uncorrected; no CPU observation incurs a page-in.

Mech's extra immutable prepared lookup metadata is **123,168 bytes**. The whole Mech task retains compiler/catalog and decoded descriptor state, so whole-host memory is not a minimal native-runtime comparison. Heap, RSS and kernel footprint are different observations and must not be added. Requested capacity is not bytes copied; total copy bytes and branch/cache misses remain unmeasured. Input is fully loaded before timing; no streaming or larger-input throughput claim is made.

## What the fresh profiles show

The current actual primary worker contributes **1,204 exclusive weights** over 695 diagnostic calls and surrounding harness. Frame actions account for **310 (25.75%)**, ordinary control visits **183 (15.20%)**, other invocation helpers **174 (14.45%)**, selected library calls **133 (11.05%)**, remaining **404 (33.55%)**. These categories are disjoint. Narrower tail-cleanup ancestry is **72 (5.98%)**, canonical construction/admission **76 (6.31%)**, schema closure **19 (1.58%)**; those narrower groups overlap and must not be summed.

The fresh reference sample has **1,514 weights**, with **1,415 (93.46%)** in `roc__p5794f552e2e0fcba87a322629fdca7c0.fast`. The opaque symbol is not assigned an unsupported source name. Samples include harness work, are noisy proportions, and are not exact compression-timer percentages. See the [full fresh profile](../compression-round-46/PROFILE.md).

## Allocation evidence and next target

Two separate diagnostic traces reconcile **13,944 small requests / 1,457,255 bytes** and **50 large requests / 97,129,588 bytes**. Small requests split into **1,709 setup**, **12,232 inside generated entry** and **3 teardown**. The [top-five constructor/lifetime report](../compression-round-46/ALLOCATIONS.md) attributes 6,398 of those requests, including 1,010 registration/wrapper pairs and 1,010 lazy admission-mutex allocations. Unique-root eligibility is still to be measured; these are not promised removable counts.

The large backing-request histogram is unchanged: **35 path-sized / 5 tree-sized / 1 cache-sized requests**, compared with one of each for the reference. Extra path capacity requested is **83,029,904 bytes**; requested spare capacity is not initialized copying. The earlier unchanged-ownership witness identifies four 524,288-byte tree fallbacks; initialized bytes and reserved capacity remain distinguished. The observed relocating-reallocation copy upper bound is **13,905 bytes**, unchanged. Eliminating small wrapper requests and eliminating payload copies are separate mechanisms.

## Native artifact and reproducibility

The generated compressor Object remains SHA-256 `3f0c0e1954afe1083254d928a8ff66977ee9203e64bfd281326c75f9ff5f09e9`, with **13,885,696 text bytes / 108,400 machine stack bytes**. Whole reference text is 195,456 bytes: scope differs from the single generated Mech entry. The existing static instruction inventory is reused only after exact Object identity; static instruction counts include cold/error paths and do not predict dynamic branch/cache behavior.

The [round-46 packet](../compression-round-46/CLOSURE.md) now contains the actual Object/image once, selected real-binary disassembly, every matched timing/counter/profile observation, exact commands/features/library pins, 657 passing distinct Rust tests, linked recovery, scalar/allocation controls, CI inventory and all failure dispositions. `reconstruct.py` reproduces scorecards, constructor attribution and numerical profiles byte-for-byte. `compare.py` regenerates this directory's JSON/CSV from those checked-in measurements without executing a compiler or benchmark.

Baseline: `b12f6c5afb2c859f6cde29ba566ddf09e70d2f79`. Implementation: `fead50f19bcec08deacc4622c4ded05ee91b8383`. Optimized source: `6027e8016150f0510b23fcdda20430a40837e1da`. The source/bytecode, pinned allocator, algorithms/tables/search limits/real buffers, source allocation sites, capacity-13 clear/reuse, exact raw output, work **60,260,219,686**, comparison work **47,612**, input aliases/content and final zero cleanup are unchanged. The next isolated pilot is unique-root first admission; physical cleanup-domain analysis follows the new brief. Parity remains outstanding at 1×.
