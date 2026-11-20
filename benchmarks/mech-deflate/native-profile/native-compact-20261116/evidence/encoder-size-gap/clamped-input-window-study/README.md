# Clamped input windows and late branch cleanup

**The qualified selection remains 63,612 B / 81 Roc / 62 libdeflate. There is no 63rd C win.** This pass produces a **63,180 B word candidate**, a **63,484 B probe candidate** and a **63,068 B packet-qualified candidate**. Each has 4,736 B peak stack. All previous peer wins observed in the twenty-one-case panel remain point wins in all six complete discovery orders. Full 81-case confidence qualification is outstanding.

The leading probe takes **1.04–1.53% less time on natural 4 KiB** and **1.61–1.62% less time on random 1 MiB** than its matching input-word/probe control. Natural 4 KiB remains **1.011199× / 1.009384× C**, needing about **0.93–1.11% less candidate time** for parity in these orders. All observations remain included.

## Source mechanism

Source **`3304169045a648907028e309aab098ef9b445751`** reuses the existing caller-admitted immutable input span. On the unit-stride loop's non-exit edge, `position < length`. This makes `length - position` nonwrapping. The compiler proves every incoming value of the clamped window is at most that remainder, then removes the overflow and input-extent predicates for:

```text
position + min(length - position, limit)
```

The proof follows the actual unsigned branch edges supplying a clamp phi. It also recognizes `base + (position - base)` as the same modular position. Clamp joins must occur inside the admitted cursor's body; a previous iteration's remaining length cannot be carried through the cursor header. Unsupported signed comparisons, changed positions/lengths and unknown shapes retain the existing checked graph. Proof work has fixed graph, step and recursion limits.

The first implementation at **`2e5949784`** replaced the two SSA comparisons with constants. Native traces exposed remaining conditional branches on materialized constants. The final implementation consumes the new facts with the existing constant-condition evaluator, including Boolean/zero-extension wrappers, and prunes unreachable paths before packing input words. The new metadata counter is omitted when zero, preserving old artifacts with the option disabled. This happens after readonly-load conversion, preserving the earlier load optimizations. No phase body is cloned.

Original 2,719-line Mech source and canonical bytecode are unchanged. No LLVM dependency is added. Enable `MECH_REUSE_ADMITTED_INPUT_WINDOWS=1` with the admitted-span recipe. The source-built compiler SHA is **`64a4f8c7c434f082a718a956e14f266e337331e6cb63c69a48b1dddb2014fa08`**. Its exact source inputs match the source commit above.

## Runtime discovery

Each row uses its own matching feature control, twenty-one cases, both orders and twenty-four balanced blocks per case. The packet control already enables immutable packet-load qualification. Differences between these separate cohorts are not direct causal comparisons between candidates.

| Candidate | Text | Natural 4 KiB / C, forward / reverse | Natural 4 KiB / matching base | Random 1 MiB / matching base |
| --- | ---: | ---: | ---: | ---: |
| Word | 63,180 B | 1.017035× / 1.022675× | 0.984551× / 0.987721× | 0.986707× / 0.986033× |
| Probe | **63,484 B** | **1.011199× / 1.009384×** | **0.989638× / 0.984740×** | **0.983906× / 0.983763×** |
| Packet-qualified | **63,068 B** | 1.022235× / 1.025639× | 0.979016× / 0.981948× | 0.984530× / 0.984221× |

The packet candidate remains available for repair, 416 B smaller than the probe. Packet qualification's earlier natural/distribution-change cost remains relevant; this new window pass improves that candidate without establishing parity. Every original observation is retained, including small slowdowns that preserve peer wins. These panels establish no new full score.

## Actual machine work

These are untimed **pre-layout collector** diagnostics, with all other recipe features held fixed. Shipping workers are emitted from source; diagnostic clones only add counters. Their counts are not a cycle forecast or the final profiled workers' instruction totals.

| Probe collector metric | Natural 4 KiB, before → after | Random 1 MiB, before → after |
| --- | ---: | ---: |
| Instructions | 1,427,925 → **1,387,046** | 228,068,355 → **219,297,833** |
| Branches | 285,161 → **277,690** | 45,613,459 → **44,562,962** |
| Stack loads | 117,827 → **101,457** | 24,343,895 → **20,093,897** |
| Stack stores | 30,645 → 30,639 | 6,554,019 → 6,553,945 |

The first comparison-to-constant version removed just 4,135 natural-4-KiB instructions and no dynamic branches. Its original object, counts and profiles are preserved. The final cleanup removes the remaining constant routes and changes register allocation. The word alternative's natural-4-KiB stack stores increase from 24,311 to 30,639 while its overall time improves; this adverse count is retained.

Fresh source-bound layouts train on the same six declared random/natural cases. All current workers pass semantic and stack gates. Actual optimized IR inspection preserves the exact profiled object, complete metadata and canonical bytecode.

## Complete builds on Apple M1

| Actor | Encoder text | Source through final executable link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Qualified reference, same compiler | 63,612 B | 2.211302 s | 2.222104 s | 276,072 B |
| Window word | 63,180 B | **2.212604 s** | 2.223531 s | 276,072 B |
| Window probe | 63,484 B | **2.213320 s** | 2.224169 s | 276,072 B |
| Window packet-qualified | 63,068 B | 2.215269 s | 2.226315 s | 276,072 B |
| Roc | 57,696 B | **2.984480 s** | 2.995272 s | 191,912 B |
| libdeflate | 37,080 B | **0.481384 s** | 0.492309 s | 155,752 B |

Twelve balanced serial fresh-process trials per actor include final executable linking. Compilers and fixed support are prebuilt equally. Every emitted Mech object, complete metadata and bytecode must match its recipe. Strip and compiler bootstrap are separate. No runtime acquisition, test build, counter, profile preparation or archive compression runs during this cohort.

Probe paired build ratios are **0.742111× Roc / 4.602746× C / 1.003198× the qualified reference**. Word ratios are **0.741734× Roc / 4.599976× C**. This study establishes no compile-time gain.

Probe phase medians are **701.969 ms parsing**, **216.204 ms semantics**, **6.832 ms artifact preparation**, **2.537 ms encoding**, **1,223.726 ms native emission**, and **35.062 ms final linking**. Catalog work overlaps parsing; independent medians do not add. The 207.3-second Rust producer rebuild is a separate record.

## Validation and reproduction

The source passes **23 relevant repository tests**, including **47,552 native span/window/histogram calls**. The new fixture verifies that three branches disappear and covers full-word boundaries, stale-window refusal, original errors, fallback and partial state. The entire GPU suite is not claimed. The existing missing archived allocator still blocks the repository-wide unsafe audit.

All three profiled candidates pass **162 output/inflation/alias observations, six checked failure cases, twelve injected failures with twelve exact recoveries, and the 5,280 B stack cap**. Normal and allocation-checked compilers reproduce identical artifacts. Disabling the new option reproduces the qualified histogram, previous word, previous probe and previous packet artifacts exactly. No scalar path, error origin, ownership check or resource accounting is retired. These checks use the documented diagnostic entry and do not establish ordinary managed-entry admission/recovery.

Use a minimal compiler built from source `3304169045a648907028e309aab098ef9b445751`. Its command, configuration and exact source inputs are archived here.

```sh
python3 reproduce.py /path/to/compile_profile /tmp/mech-window-probe
python3 reproduce.py /path/to/compile_profile /tmp/mech-window-packet --role window-clean-packet-profiled
python3 verify.py
```

The verifier recomputes all six raw paired orders, instruction and branch totals, artifact/source bindings, repository-test receipts, worker/error/stack/recovery gates and twelve complete-build trials per actor. Portable relocated profiles reproduce the probe and packet candidate exactly in normal and checked modes. Different hardware or support requires fresh runtime qualification.

The next input-window target is the remaining four-byte read at `position + 1`: the already selected `window >= 5` edge proves that read fits. That proof is not included in this checkpoint.
