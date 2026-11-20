# Paired scalar continuation under the modern scorer

**Keep the qualified 63,612 B / 81 Roc / 62 libdeflate default and the preceding eight-entry candidate. The restored paired scalar path needs repair: it loses the existing natural 8 KiB C win. No new qualified win is claimed.**

Compiler source: [`84628d979e`](https://github.com/mech-lang/mech/commit/84628d979e34b67b6450b9eb3e9b79a694b629e3). The original 2,719-line Mech codec and canonical bytecode remain unchanged. No LLVM dependency is added. The opt-in source and all observations are retained.

`MECH_ORDERED_PAIRED_CONTINUATION=1` revisits the earlier paired-scalar experiment with the current 4,736 B frame and packed eight-entry scorer. After the original first scalar visit, two admitted continuation visits are evaluated together. A strict comparison chooses the earlier member of a tie; another strict comparison preserves the entire incoming item on a tie with the best. The original scalar loop handles an odd remainder. Deferred-item reconstruction remains supported. The same source-derived immutable windows and biased-index witness admit both reads; no visits, budgets or error checks are removed.

A separate production helper keeps the pair control flow small and directly testable. The new native fixture covers both reverse widths, unaligned guarded storage, wrapping U32 costs, complete old I64 items, strict ties, multiple pairs and odd tails. The disabled option reproduces **all three artifacts for 33 older recipes exactly**.

## All eight runtime orders

Each entry is a complete 21-case/24-balanced-block acquisition. Times are Mech / libdeflate; lower is faster. Original head/tail variants compare against their exact eight-entry workers. Register-setting pairs compare against matching unpaired controls with the same setting and layout policy. The latter leave the two other phases unprofiled and prepare the same fresh eight-case scorer profile for both actors, so they do not isolate register preference from that policy change.

| Candidate | Text | Order | Natural 4 KiB / C | Natural 8 KiB / C | 4KiB time / held control | Missing old C wins in panel |
| --- | ---: | --- | ---: | ---: | ---: | --- |
| `scalar-pair-tail-probe-profiled` | 64,124 B | forward | 1.019542× | 0.999793× | 1.023024× | none |
| `scalar-pair-tail-probe-profiled` | 64,124 B | reverse | 1.014746× | 1.006689× | 1.022020× | prefix-natural-8192 |
| `scalar-pair-head-probe-profiled` | 64,108 B | forward | 1.008413× | 1.002231× | 1.019649× | prefix-natural-8192 |
| `scalar-pair-head-probe-profiled` | 64,108 B | reverse | 1.019633× | 1.009163× | 1.024407× | prefix-natural-8192 |
| `scalar-pair-root-probe-profiled` | 64,092 B | forward | 1.027865× | 1.010813× | 1.019472× | prefix-natural-8192, prefix-random-1024 |
| `scalar-pair-root-probe-profiled` | 64,092 B | reverse | 1.036251× | 1.018317× | 1.027371× | prefix-random-1024, prefix-natural-8192 |
| `scalar-pair-plain-probe-profiled` | 64,860 B | forward | 1.040318× | 1.008056× | 1.021257× | prefix-natural-8192, prefix-random-1024 |
| `scalar-pair-plain-probe-profiled` | 64,860 B | reverse | 1.037717× | 1.016466× | 1.024271× | prefix-random-1024, prefix-natural-8192 |

These discovery panels do not establish a full 81-case score. All previous observed Roc wins survive, but the named natural 8 KiB C loss rules out promotion now. Keep the source and investigate the cause. The preceding eight-entry candidate remains **63,868 B / 63 C point wins in both full96 orders**, with the new reverse natural 4 KiB confidence interval still crossing parity; the qualified score remains81/62.

## Work and register lifetime

Peak stack stays **4,736 B**. Reload frequency increases sharply even though the pair body itself does not load the stack. The outer literal-cost-table setup now reloads its base from `[sp,#0x90]` on 35,565 visits in natural 4 KiB. Both cost values remain live until the pair minimum is reconstructed, alongside the incoming best and item. Generated code also repeats two comparisons to reconstruct the index and item. This gives specific lifetimes and instructions to repair.

| Natural 4 KiB raw scorer | Instructions | Branches | Stack loads | Stack stores |
| --- | ---: | ---: | ---: | ---: |
| Eight-entry head | 3,266,626 | 513,080 | 17,310 | 18,475 |
| Paired scalar head | 3,589,068 | 509,199 | 105,720 | 19,650 |

The pair removes 3,881 branches but adds 322,442 instructions and 88,410 stack loads. Natural 8 KiB rises from 7,608,126 to 8,466,508 instructions and 43,074 to 259,920 stack loads. These are untimed diagnostic counts, not latency measurements.

Both existing register settings were attempted: preserved preference only for the root, and no preserved preference. **The raw scorer body hashes and aggregate work counts remain unchanged** for the matching paired/unpaired body in each setting. Other code changes: the two root-only workers emit 63,852 / 64,092 B; the two plain workers emit 64,620 / 64,860 B. All four setting workers pass the semantic gates. Their complete runtime orders above are retained; they do not repair scorer spills.

## Broad loss census

An additional diagnostic census covers eleven cases, using the exact preceding eight-entry raw scorer. It changes no shipping object, layout profile or latency data. Its published full96 references identify the next closest remaining C losses: natural 1 KiB at approximately 1.25×, random 1 MiB at 1.28× and distribution change at 1.28–1.29×. Every reference is checked against the preceding raw archive.

Natural 1 KiB executes 423,496 scorer instructions, 2,496 stack loads and 2,804 stack stores. Its leading blocks contain paired vector scoring, first scalar visits and scalar continuation. Random 1 MiB instead executes 69,418,301 instructions; its hottest 13-instruction literal-only block runs 2,986,191 times. That block loads the literal cost, constructs and stores the packed DP cost/item word, and advances the outer traversal. Two nearby four-instruction blocks handle descriptor extraction and the literal-index bound. This route needs separate optimization to close the larger random-input gap. The raw blocks and counts for all eleven cases are preserved, including the distribution-change mixture.

## Source through final executable builds

Apple M1, macOS 15.6.1, arm64. Ten balanced serial fresh-process trials per actor include final executable linking. Compilers and fixed support are prebuilt equally; bootstrap and stripping are separate. Every generated Mech object, full metadata and bytecode is hash checked. All runtime and diagnostic producers finish before these clocks begin.

| Actor | Encoder text | Source through final link | Stripped executable |
| --- | ---: | ---: | ---: |
| Qualified histogram | 63,612 B | 2.213505 s | 276,072 B |
| Paired scalar, tail | 64,124 B | 2.220747 s | 276,072 B |
| Paired scalar, head | 64,108 B | 2.219618 s | 276,072 B |
| Roc | 57,696 B | 2.986912 s | 191,912 B |
| libdeflate C | 37,080 B | 0.482204 s | 155,752 B |

Head paired build time: **0.743139× Roc / 4.606172× C**, **1.002866×** the qualified control. Text: **1.111× Roc / 1.729× C**. No compilation-time gain is established.

Phase medians: parsing **704.244 ms**, semantics **217.405 ms**, artifact **6.861 ms**, encoding **2.538 ms**, native pipeline **1225.512 ms**, final link **35.585 ms**. Catalog overlaps parsing; independently computed medians do not add exactly.

## Validation and reproduction

Twenty optimized tests pass in four configurations: default; paired scalar with eight-entry/head/wide item; the same with carried lanes and latch testing; and paired scalar with deferred-item reconstruction. Each configuration includes **22,528 new paired-scalar native calls**, 94,208 packed-key/loop calls, 16,384 wide-item calls and 16,384 prior winner/item calls. All 33 disabled recipes reproduce exactly.

All six workers pass 162 output/inflation/alias observations, six checked failures, twelve injected failures and twelve exact recoveries. Normal and checked artifacts match, as do all six portable normal/checked recipes with relocated profiles. Peak stack is 4,736 B, below 5,280 B. The full GPU suite is not claimed; the pre-existing missing archived allocator still blocks the repository unsafe audit.

The compiler build uses the checked-in native-profile manifest. From the repository root, this reproduces the recorded release configuration with a separate compiler target:

```sh
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 \
CARGO_PROFILE_RELEASE_OPT_LEVEL=3 CARGO_PROFILE_RELEASE_DEBUG=false \
CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
cargo +nightly-2026-03-03 build --locked --release \
  --config profile.release.package.libmimalloc-sys.opt-level=1 \
  --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml \
  --target-dir /tmp/mech-paired-scalar-compiler --bin compile_profile
```

The measured compiler rebuild took 226.216 s with existing dependency artifacts. It is separate from the codec source-through-link measurement and is not a clean compiler bootstrap.

From this evidence directory:

```sh
python3 verify.py
python3 reproduce.py /path/to/compile_profile /tmp/paired-scalar-replay --role scalar-pair-head-probe-profiled
```

The verifier checks the six source/artifact bindings, test overlays, 33 exact replays, semantic/recovery gates, work counts and their prior-study comparisons, all eight raw runtime orders, the eleven-case loss census and ten complete build trials per actor. Archives preserve every observation, including regressions; every member is pinned in `archive-files.json`.

Next, update the minimum and item after each scalar visit within an unrolled pair. This can shorten the first cost and incoming-best lifetimes and avoid reconstructing a pair winner. Test it against this exact paired candidate and the preceding unpaired worker; preserve the old paired mode, exact tails, all prior named wins and the qualified 81/62 default. Separately investigate the literal-only DP route identified by the broad census. These follow-ups are not implemented in this source epoch.
