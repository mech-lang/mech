# Vector mismatch forwarding and word-comparison repairs

**Qualified selection: 63,532 B, 81/81 Roc and 62/81 libdeflate. No new promotion or 63rd win is claimed.** The next milestone is an additional C win while preserving every previous individual peer win; the eventual goal remains 1× libdeflate performance.

Compiler source is committed at `546af95af3735591a77455932dc48920a8fc071d`. The opt-in mismatch handoff uses the first unequal word from an already loaded vector instead of reloading through the scalar header. The retained collector uses its 16-byte tier; its 64-byte tier is disabled. The original 2,719-line source and canonical bytecode are unchanged. No LLVM dependency is added. Disabled mode and the existing word strategy reproduce their previous artifacts exactly.

## Runtime discovery

Nineteen workloads, twenty-four balanced blocks and both complete actor/case orders per candidate. These are discovery point estimates, not full 81-case scores or precision-supported new wins. Every positive and adverse observation is retained.

| Candidate | Encoder text, B | Natural 4 KiB, Mech/C forward / reverse |
| --- | ---: | ---: |
| vector-mismatch-narrow-profiled | 63,628 | 1.025877 / 1.039387 |
| vector-mismatch-wide-profiled | 63,612 | 1.030333 / 1.042030 |
| word-current-complements-profiled | 63,388 | 1.023495 / 1.033400 |
| word-current-narrow-profiled | 63,468 | 1.054373 / 1.058893 |
| word-current-profiled | 63,420 | 1.065730 / 1.067686 |
| word-current-wide-profiled | 63,452 | 1.039021 / 1.045144 |
| word-reuse-complements-profiled | 63,388 | 1.031752 / 1.044031 |
| word-reuse-narrow-profiled | 63,468 | 1.048637 / 1.060597 |
| word-reuse-wide-profiled | 63,452 | 1.040411 / 1.034970 |

The 63,388 B word/complement candidate saves 144 B and improves natural 4 KiB by 2.84–3.61% against its same-run control. It still needs 2.30–3.23% less candidate time for C parity. Natural 1 KiB improves roughly 2.9–3.2%; natural 8 KiB, periodic 1 KiB and 320-overlap remain below C in both discovery orders. Random 4 KiB slows 1.49–4.11%; random 1 MiB is only about 0.24% faster. Keep the gain available and repair adverse cases rather than discard it.

The vector/narrow candidate reaches 1.025877× / 1.039387× C on natural 4 KiB and improves distribution change roughly 3.1–3.5% against its same-run control. Random 1 MiB improves only about 0.10–0.16%. The word-reuse/narrow variant loses the existing 320-overlap C win forward at 1.001690× C; reverse is 0.996478×. It remains available for repair, not promotion.

The new `word-reuse` strategy retains saved-prefix machinery while selecting scalar words. **This codec admits no saved interval under the existing scope proof.** Its changed entry structure and register allocation are measured, but no cache hit, reused prefix, or new cache proof is claimed.

## Source and validation

The private certificate still proves an admitted pure read-only recurrence. A temporary dominator analysis checks the extra mismatch path before mutation. Only reconstructible pure values travel through explicit continuation arguments. The handoff preserves the first unequal word, position, live values, scalar tails and limit exits. Source effects, independently observed word loads and unsupported escaping values refuse the optimization. Authority remains private and nonserialized.

The exact production module and committed tests pass **1,288,960 optimized verified native comparisons**, plus **60 comparisons across four refusal fixtures**. They cover every mismatch byte, low/high differing bits, unaligned offsets, nonzero starts, tails, masked/reversed conditions, shared continuations and live payloads. The minimal Certificate fixture supplies admitted physical windows; real production certificate construction is separately exercised by the source-built codec.

Every timed candidate passes **162 exact-output/inflation/alias observations, six checked failures and the 5,280 B stack cap**. Stacks are 4,848–4,864 B. No new twelve-recovery campaign or full 81-case score is claimed.

Untimed raw-source matcher counts show the limitation: natural 4 KiB vector/wide falls from 1,521,656 to 1,481,710 instructions while stack loads rise from 105,318 to 106,261. Random 1 MiB loses only 6,626 of 245,750,573 instructions and gains 1,045,801 stack loads. These are pre-layout diagnostic bodies, not timing workers or cycle forecasts. Counter source/object bindings are archived.

## Complete builds

Ten balanced serial fresh-process orders per actor include final executable linking. Compilers and fixed support are prebuilt equally. Bootstrap is excluded for every actor; strip is separate. Every Mech trial reproduces all three generated artifacts. All Mech build actors use the current 47ee compiler, including word/complement artifacts originally timed from equivalent 5bcc output.

| Actor | Encoder text | Source through link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Current compiler, qualified encoder | 63,532 B | 2.212654 s | 2.223596 s | 276,072 B |
| Word/complement, unselected | 63,388 B | 2.214373 s | 2.225233 s | 276,072 B |
| Vector/narrow, unselected | 63,628 B | 2.213084 s | 2.225030 s | 276,072 B |
| Roc | 57,696 B | 2.991040 s | 3.001820 s | 191,912 B |
| libdeflate | 37,080 B | 0.482117 s | 0.493106 s | 155,752 B |

Retained-build paired ratios are **0.739292× Roc / 4.588892× C**. Word/complement is **0.739845× Roc / 4.594875× C**; vector/narrow is **0.740267× Roc / 4.592154× C**. Against the same-compiler retained build, candidates are 1.001752× and 1.000576×. These small differences do not establish a compile-time change. Build time and compression latency remain separate.

Word/complement phase medians: parsing 704.721 ms, semantics 217.239 ms, artifact preparation 6.836 ms, encoding 2.532 ms, native pipeline 1,220.038 ms and final link 35.474 ms. Catalog work overlaps parsing; separate medians do not add.

After authorized cleanup, the cached no-std deployment archive was rebuilt outside all measured trials. Its final hash exactly matches original `6c29b7cb…`. The first offline attempt needed a deleted dependency download; a second inherited unrelated worktree Rust flags. Both are archived and unused. The final standalone-project configuration restores the original archive exactly. Fixed-runtime source and restoration receipts are included.

## Limits and replay

Formatting, module layout and warning policy pass. The repository-wide unsafe audit still fails on the pre-existing missing allowlisted file `benchmarks/mech-deflate/comparison/rust/src/pinned_allocator.rs`; its log is archived. The full GPU test build failed before running tests when the existing full-catalog math dev dependency exhausted disk space. It is not a passing full GPU suite. The successful exact native harness and actual codec checks are separate evidence.

The first timing campaign was interrupted for user-requested cleanup and is entirely ineligible. Only `vector-mismatch-clean-panel-*` observations contribute here; the interruption record is archived. Source, recipes, objects, full metadata, tests, counters, failures, raw observations and complete builds are hash pinned. Compiler binaries are excluded.

Run from this directory:

```sh
python3 verify.py
python3 reproduce.py /path/to/compile_profile /tmp/word-complements --variant word-complements
python3 reproduce.py /path/to/compile_profile /tmp/vector-narrow --variant vector-narrow
```

Build the compiler from the source pin above using nightly 2026-03-03, release opt-level 3, 16 codegen units, LTO off and one build job. Complete commands and overlays are archived. Reproduction rebinds profiles and checks the object, full metadata and bytecode; it never patches machine code. Hardware scores require fresh qualification.

## Next repair

Keep the smaller word/complement gain. Try an admitted first-word probe before the vector tier, forwarding that word directly on mismatch and retaining vectors for longer matches. Compare against both current candidates across the spectrum. Large random losses additionally need reductions in repeated collector predicates, packet geometry and live state; this mismatch pass removes too little work there. Saved-prefix reuse needs a real immutable-input scope proof before claiming cache benefits. Preserve every old peer win, failure/recovery/stack checks, both full orders and precision checks before promotion.
