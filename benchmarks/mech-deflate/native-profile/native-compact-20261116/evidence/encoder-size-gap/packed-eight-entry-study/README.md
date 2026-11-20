# Guarded eight-candidate packed scorer entry

**Strict 63-win qualification: not passed. The qualified default remains 63,612 B / 81 Roc / 62 libdeflate until selection is separately updated. Keep the candidate and its measured gains.**

Compiler source: [`60e942f462`](https://github.com/mech-lang/mech/commit/60e942f46298e1d4b3ea446248596ae3e071853d). The original 2,719-line Mech codec and canonical bytecode remain unchanged. No LLVM dependency is added.

`MECH_ORDERED_PACKED_EIGHT_ENTRY=1` permits a paired packed reduction to begin at eight candidates. It requires the existing little-endian AArch64 packed-cost option, exactly two accumulators, no carried pointers, and an index-independent cost. The runtime packed-key best/count guard is reused at vector entry. Eight to fifteen candidates with wide costs return to the original short scalar path; the original wide vector loop retains its sixteen-element threshold. Counts below eight retain the same first scalar operation. Exact remainder, strict earliest-winner behavior, original wide item, checked errors and finite source accounting remain maintained.

Both head and tail forms are measured against their exact sixteen-entry workers, with the same declared eight-case layout profile policy. With this option disabled, **31 older recipes reproduce all three artifacts exactly**. This source change keeps the earlier head-group and wider-item machinery.

## Runtime campaign

All ratios are Mech / libdeflate; lower is faster. Discovery comprises four complete 21-case/24-balanced-block orders. The head candidate then receives two complete 81-case/24-block screens and two complete 81-case/96-block qualifications. Every observation is retained.

| Candidate | Encoder text | Discovery natural 4 KiB, forward / reverse |
| --- | ---: | ---: |
| `packed-eight-paired-probe-profiled` | 63,884 B | 0.998584× / 0.999310× |
| `packed-eight-head-probe-profiled` | 63,868 B | 0.994896× / 0.992036× |

| Full24 order | Roc wins | C wins | Natural 4 KiB / C |
| --- | ---: | ---: | ---: |
| forward | 81 | 63 | 0.989234× |
| reverse | 81 | 62 | 1.002393× |

| Full96 order | Roc point / precision wins | C point / precision wins | Natural 4 KiB / C | 90% paired time-change interval vs C |
| --- | ---: | ---: | ---: | ---: |
| forward | 81 / 81 | 63 / 63 | 0.993901× | -1.144% to -0.096% |
| reverse | 81 / 81 | 63 / 62 | 0.997675× | -0.818% to +0.388% |

Every original 81 Roc / 62 C named win remains precision-supported in both full96 orders. Only the new reverse natural-4-KiB comparison lacks confidence support, so win 63 is not yet qualified. The final gate remains 63 C wins, every prior named win plus natural 4 KiB, and the existing 90% / 8,000-resample confidence policy in both orders. The reverse full24 target at 1.002393× C was admitted within the existing 0.5% discovery resolution; that screening admission does not weaken the final gate.

Unsupported named wins are recorded for each complete96 order:

- forward: roc: none; libdeflate: none.
- reverse: roc: none; libdeflate: prefix-natural-4096.

The public verifier replays raw observations, cleanup and actor pins, both full24 screens, the complete full96 report and each required named confidence outcome. Before timing, all 81 temporary input/prefix/expected contents and shapes were reconciled with the canonical inventory. Only the canonical replay inventory digest is reconciled; original pins and all data remain intact.

## Untimed scorer counts

On natural 4 KiB, the head candidate removes **66,937 instructions and 36,501 branches** against the preceding sixteen-entry head worker. Stack traffic is unchanged. Aggregate instruction, branch and stack counts on random 1 MiB and distribution change are exactly unchanged; individual branch opcodes can differ. Natural 8 KiB executes more scorer instructions but retains its previous peer win; speed changes are allowed when prior individual peer wins survive. These counts are diagnostic and do not replace latency observations.

| Raw scorer | Natural 4 KiB instructions | Branches | Stack loads / stores |
| --- | ---: | ---: | ---: |
| Previous paired carried | 3,343,871 | 549,581 | 17,310 / 18,475 |
| Previous sixteen-entry head | 3,333,563 | 549,581 | 17,310 / 18,475 |
| Eight-entry paired | 3,286,504 | 513,080 | 17,310 / 18,475 |
| Eight-entry head | 3,266,626 | 513,080 | 17,310 / 18,475 |

## Source through final executable builds

Apple M1, macOS 15.6.1, arm64. Ten balanced serial fresh-process trials per actor include final executable linking, with compilers and fixed support prebuilt equally. Bootstrap and strip are separate. Every generated Mech object, full metadata and bytecode is hash checked. Build clocks start after the runtime campaign and its confidence replay finish.

| Actor | Encoder text | Source through final link | Stripped executable |
| --- | ---: | ---: | ---: |
| Qualified histogram | 63,612 B | 2.209806 s | 276,072 B |
| Eight-entry paired | 63,884 B | 2.215319 s | 276,072 B |
| Eight-entry head | 63,868 B | 2.215637 s | 276,072 B |
| Roc | 57,696 B | 2.983775 s | 191,912 B |
| libdeflate C | 37,080 B | 0.480913 s | 155,752 B |

Head encoder text is **1.107× Roc / 1.722× C**; paired build time is **0.742654× Roc / 4.604948× C**, **1.002503×** the qualified control. No compile-time gain is established. All phase medians and observations are in the archive.

Head build phase medians: parsing **701.408 ms**, semantics **215.081 ms**, artifact preparation **6.796 ms**, bytecode encoding **2.548 ms**, native pipeline **1226.934 ms**, final link **35.327 ms**. Catalog work overlaps parsing, and independently computed medians do not add to an exact total.

## Validation and portable replay

Nineteen optimized tests pass in three configurations: default; eight-entry paired/head/wide item with direct item and singleton; and the same with carried lanes plus latch testing. Each configuration includes **94,208 packed-key/loop native calls, 16,384 wide-item calls and 16,384 prior winner/item calls**. All 31 old recipes reproduce exactly with eight-entry disabled.

Both candidates pass 162 output/inflation/alias observations, six checked failures, twelve injected failures and twelve exact recoveries. Peak stack is **4,736 B**, below the 5,280 B cap. Normal and checked artifacts match exactly, as do portable normal/checked replays with relocated layout profiles. The full GPU suite is not claimed; the pre-existing missing archived allocator still blocks the repository unsafe audit.

```sh
python3 verify.py
python3 reproduce.py /path/to/compile_profile /tmp/eight-entry-replay --role packed-eight-head-probe-profiled
```

`checks-and-builds.tar.gz` contains compiler/test source overlays and commands, old recipe replays, checks, counts, full qualification records and complete-build observations. `raw-runtime.tar.gz` preserves all eight orders: four discovery, two full24 and two full96. Every member is pinned in `archive-files.json`.

The [head/item study](../packed-margin-repair-study/README.md) preserves the earlier 16 B saving, two exact no-ops and the deferred-item revisit. The [paired accumulator study](../paired-packed-accumulator-study/README.md) preserves the preceding reverse 0.036% miss. Keep all gains available for combination. If the new margin remains unsupported, revisit the earlier scalar pairing against this newer 4,736 B frame and inspect actual scalar-block counts; its earlier 5,280 B build increased spill loads.
