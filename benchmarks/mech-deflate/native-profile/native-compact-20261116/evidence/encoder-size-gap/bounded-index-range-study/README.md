# Guarded index ranges: smaller candidate, qualification pending

**The new word candidate is 63,388 B, with 4,720 B peak stack.** It saves 224 B and 144 B of stack from the retained histogram recipe. Word and vector/complement both preserve every previous individual winning fixture in two complete **81-case, 24-block point screens**: 81/81 Roc and 62/81 libdeflate. They also pass 162 output/inflation/alias observations, six checked failures, twelve injected failures and twelve recoveries. Normal and allocation-checked source emission reproduce all three artifacts exactly.

**Neither candidate is promoted.** The full 96-block individual confidence campaign remains outstanding. The retained qualified result is still the 63,612 B histogram worker, with its existing 81 Roc / 62 C confidence-supported wins. There is no 63rd C win. Point counts must not be presented as new qualified scores.

Source is integrated at **`2537362150d163fe6bc490ff84d283f7f3fc9639`**. The compiler was built from the preceding benchmark commit plus the exact five-file overlay committed there; source hashes and snapshots bind both forms. The original Mech codec and canonical bytecode are unchanged. No LLVM dependency or codec rewrite is introduced.

## Compiler mechanism

The private numeric caller can explicitly adopt additional fixed-view length floors from the optimized body. It guards them before any source action and falls back to the original checked continuation on refusal. Only uniquely owned, fixed writable U32 views participate in new small-table admission. Exact packet lengths and independently bounded unsigned indices supply the proof. Unknown ranges, wrapping offsets, changed lengths and signed comparisons retain their checks. Both graph size and recursive range analysis have bounds.

The actual collector admits **two removed bounds checks** and a ten-element floor for the observation table. This is separate from the already optimized 262,144-element history table. Other source bounds and the scorer's dense composition remain intact. The native tests compare original and optimized results, partial state and error origins across **7,560 calls**, including short-table fallback, U32 overflow, unsigned extremes and joined control-flow values. A newly unreachable failure path has its own regression case.

Adding the entry guard shifts the old numeric region hints. The first two raw-hint compilations failed dominance verification; both logs and compiler inputs are retained. Source-relative mappings resolve those boundaries correctly. The final minimal recipe uses numeric hints regenerated from that verified mapping, and the existing checked-region proof revalidates their CFG and ABI at compilation. The portable recipe needs no optional source-unit compiler feature.

## Runtime observations

A nineteen-case discovery panel covers natural data, random sizes and boundaries, distribution changes and fragile overlap/periodic wins, in both orders. All alternatives remain archived. The subsequent complete point screens use the **actual minimal-build workers**; their identities differ from the source-window discovery workers. Identical executable text sections are recorded, but runtime qualification is not inherited from that fact.

| Full 24-block point screen | Natural 4 KiB / C, forward / reverse | Random 1 MiB / C, forward / reverse | Point wins, both orders |
| --- | ---: | ---: | ---: |
| Word, 63,388 B | 1.032217× / 1.041121× | 1.326153× / 1.324606× | 81 Roc / 62 C |
| Vector/complement, 63,772 B | 1.030299× / 1.020286× | 1.325423× / 1.324836× | 81 Roc / 62 C |

Natural-4-KiB candidate/control ratios are **1.005915× / 0.996968×** for word and **0.998013× / 0.990424×** for vector/complement. These point observations do not establish a consistent word speedup. The earlier nineteen-case discovery estimates are retained separately; separate cohorts' C ratios are not causal comparisons. Vector/complement still needs about **2.94% less time** for C parity in its worse complete point order.

Unprofiled matcher counters expose a tradeoff: random 1 MiB executes 244,275,679 instructions in its control and 246,402,283 in word, with 2,132,949 additional moves, 2,770 fewer branches and nearly unchanged stack traffic. The hot copy/reload sequence and repair proposals are archived. These are diagnostic bodies before fresh layout, not whole-encoder latency forecasts. Smaller alternatives are retained for combination and repair.

## Source-through-final-link builds

| Actor | Encoder text | Complete executable build | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Same minimal compiler, index guard disabled | 63,612 B | 2.218834 s | 2.229846 s | 276,072 B |
| Word | **63,388 B** | **2.215846 s** | 2.226879 s | 276,072 B |
| Vector/complement | 63,772 B | 2.219227 s | 2.230216 s | 276,072 B |
| Roc | 57,696 B | 2.987852 s | 2.998831 s | 191,912 B |
| libdeflate | 37,080 B | 0.483498 s | 0.494421 s | 155,752 B |

Ten balanced serial fresh-process trials per actor include final executable linking. All compilers and fixed support are prebuilt equally. Bootstrap and strip are separate metrics. Every generated Mech object, complete metadata and bytecode is hash checked. Word's paired build ratios are **0.741457× Roc / 4.583660× C / 0.999347× disabled**. No established compilation-time gain is claimed from the small candidate/control difference. Encoder size is **1.098655× Roc / 1.709493× C**.

Word phase medians: parse 705.316 ms, semantics 217.555 ms, artifact preparation 6.841 ms, encoding 2.532 ms and native pipeline 1,221.518 ms. Catalog work overlaps parsing; separate medians do not add.

The intermediate source-window recipe measured 2.302 s. A second matched cohort isolates the optional compiler feature: minimal/raw 2.217957 s, feature/raw 2.304190 s, feature/source-window 2.305543 s. Minimal/raw and feature/raw emit identical artifacts. The feature/raw paired cost is 3.925%. The final minimal recipe avoids that cost; all three cohorts remain available. No unsupported cause inside the feature is asserted.

## Reproduce and continue

```sh
python3 verify.py
python3 reproduce.py /path/to/compile_profile /tmp/bounded-index-replay
```

From the benchmark root:

```sh
python3 rebuild.py /tmp/mech-bounded-indices --variant m1-bounded-index-ranges-experimental
```

The compiler source and generation recipe are in the PR. Runtime and complete-build measurements on new hardware require their own qualification. Diagnostic-entry results do not establish ordinary managed-entry admission and recovery. The entire GPU suite is not claimed; the two relevant range tests passed. The pre-existing missing archived allocator still blocks the repository unsafe audit.

The bitwise follow-up below proves the literal observation index lies in 0–7. Next, measure its common-path effect and investigate the remaining selected read predicates. Preserve these candidates and adverse counts, and complete full confidence qualification before promotion.

The separate bitwise follow-up is committed at `ddebb908bfef55b353bee24df00bb01cd3da534b`, behind `MECH_REUSE_ADMITTED_BITWISE_INDEX_RANGES=1`. Three admitted-range tests exercise 18,360 native calls, and three existing word-range tests pass. It removes four collector checks in total. Raw word/vector/vector-complement encoder texts are 63,372 / 63,884 / 63,724 B; all pass 162 outputs, six failures and the stack cap. Its disabled mode reproduces the earlier 63,388 B candidate exactly. These unprofiled variants have no runtime qualification yet; no promotion or 63rd C win is claimed. Their source, test records and admitted normal-emission receipts are in `bitwise-source-checks.tar.gz`.
