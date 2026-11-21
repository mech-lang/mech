# Compared-byte reuse and guard repairs, 19 November 2026

The active goal remains **63/81 libdeflate wins while retaining every existing peer win and 81/81 Roc**. The qualified selection is unchanged: **63,532 B, 81/81 Roc and 62/81 libdeflate**. These experiments have not earned the 63rd win. The qualified 63,532 B and 63,692 B artifacts remain available in the retained study.

Compiler implementation [`4ebb148ada9311ec9caa976bd7b87e92345c309f`](https://github.com/mech-lang/mech/commit/4ebb148ada9311ec9caa976bd7b87e92345c309f) adds opt-in reuse of an already compared pair of immutable bytes on an ordering edge. It includes the previous-DP-cost work from `96b1f1ebc`. The original 2,719-line Mech source and canonical bytecode are unchanged. No LLVM dependency is added. With the new mode disabled, this compiler reproduces the qualified object, complete metadata and bytecode exactly.

## Runtime results

Each variant runs the same nineteen-case discovery panel, twenty-four balanced blocks in both orders: tiny/overlap/periodic guards, natural 1/4/8 KiB, random 1/4/8/16 KiB, 32/64-KiB boundaries, random 1 MiB and distribution change. All observations and actor identities are archived. This is discovery, not full 81-case qualification or a precision-supported new floor. Ratios below one are faster. The following table extracts natural 4 KiB.

| Source variant | Encoder text, B | Mech/libdeflate, forward / reverse | Experiment/control, forward / reverse |
| --- | ---: | ---: | ---: |
| dp-private-complements-current-profiled | 63,532 | 1.040074 / 1.057797 | 0.975760 / 0.985751 |
| dp-vector-all-profiled | 65,180 | 1.045141 / 1.040072 | 0.979339 / 0.970889 |
| dp-hash32-arithmetic-profiled | 63,548 | 1.041286 / 1.047331 | 0.975515 / 0.985005 |
| dp-hash32-shifts-profiled | 63,532 | 1.065529 / 1.070077 | 0.994120 / 0.996442 |
| dp-hash32-complements-profiled | 63,532 | 1.037800 / 1.043226 | 0.978967 / 0.980907 |
| byte-edge-wide-profiled | 63,580 | 1.043016 / 1.043324 | 0.978497 / 0.978541 |
| byte-edge-narrow-profiled | 63,596 | 1.066714 / 1.073861 | 1.005392 / 1.000548 |
| byte-edge-complements-profiled | 63,612 | 1.039992 / 1.043637 | 0.980866 / 0.968697 |
| byte-edge-hash32-profiled | 63,564 | 1.062585 / 1.066086 | 0.993854 / 0.997055 |

The promising wide and complementary-predicate byte candidates remain approximately **1.04× C**, about 2–3% faster than the same-run retained control. Byte reuse does not demonstrate an additional whole-encoder win over the earlier DP recurrence alone. The narrower and hash combinations lose the existing natural-8-KiB C win in at least one order; narrow also loses 320-overlap in the reverse order. Those measurements remain included. No candidate is promoted.

At 1.040–1.044× C, parity requires approximately **3.8–4.2% less elapsed time** on natural 4 KiB. This is a candidate-specific gap, not an overall geometric-mean claim or proof of the 63rd win.

## What was tried

- The private immutable-view contract identifies fixed byte arrays from the exact current packet. A comparison's two byte loads must dominate the selected edge, with no intervening write, call or unknown effect. Exact modular address expressions identify the subsequent reads; address hashes are not used as proof.
- The rewrite bypasses only effect-free paths to two defaulted reads. It retains each later bounds predicate and zero default through a scalar choice. Current dynamic phi transport is normalized, and escaping values receive explicit join parameters. Unmapped values, effects, ambiguous paths and bounded-search exhaustion decline the rewrite. Resource admission, checked errors and ownership are unchanged.
- The actual codec admits one such pair in the initial failed-byte comparison. Untimed counters remove **3,668 payload byte loads on natural 4 KiB** and **950,688 on random 1 MiB**. They also introduce scalar choices; the smaller load count does not establish a speedup. Most natural-4-KiB comparisons initially agree and extend the prefix, so the initial-mismatch path covers only part of its ordering work.
- Fresh matcher layout profiles are built from multiple natural and random sizes. Unchanged scorer and sort bodies reuse their previously verified profiles; wide and narrow share the identical matcher body. Every timed object is generated directly from Mech source. Coordinate-normalized counter inputs relink to the identical source worker before diagnostic instrumentation. Counter clones are never latency inputs.
- Complementary predicates remove three additional current comparison branches. The matcher already ran its basic predicate pass; the earlier hypothesis that it lacked that pass was incorrect. A broader all-vector guard gate also changes unrelated functions and adds text. That prototype is archived and absent from the live compiler. Existing private-only and narrow-hash modes are tested separately.
- A premature guard timing campaign overlapped a unit-test build. It was interrupted and all its timing output was discarded. `guard-timing-interruption.json` records this. Only the separately named, clean `dp-guard-panel-*` campaign contributes eligible results here.

A next repair can target the final mismatch after prefix extension. It must preserve the intervening cache updates and checked exits; the current pass deliberately does not bypass them. The prior DP improvement remains available while investigating that path.

## Complete build times

Ten balanced serial fresh-process orders per actor include source compilation through final executable linking. Compilers and fixed support are prebuilt equally; compiler bootstrap is excluded and strip is separate. Every Mech object, complete metadata and bytecode hash is checked in every trial. This cohort uses source `4ebb148ada`; separate historical medians do not establish causal gains.

| Actor | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Current compiler, retained encoder | 63,532 B | 2.217253 s | 2.228193 s | 276,072 B |
| Byte reuse plus wide recurrence, unselected | 63,580 B | 2.215713 s | 2.226586 s | 276,072 B |
| Byte reuse plus complementary predicates, unselected | 63,612 B | 2.220901 s | 2.231876 s | 276,072 B |
| Roc | 57,696 B | 2.987624 s | 2.998468 s | 191,912 B |
| libdeflate | 37,080 B | 0.481916 s | 0.492730 s | 155,752 B |

Paired build ratios: retained **0.742268× Roc / 4.600109× C**; byte-wide **0.740978× Roc / 4.603086× C**; byte-complements **0.743135× Roc / 4.607021× C**. Versus this same compiler generating the retained encoder, byte-wide is **0.999029×** and byte-complements **1.001542×**. These small point differences are not evidence of an established compile-time change. They are separate from compression latency.

The complementary-predicate candidate's phase medians are parsing **703.327 ms**, semantics **218.199 ms**, artifact preparation **6.825 ms**, encoding **2.534 ms**, native pipeline **1,224.154 ms** and final link **34.938 ms**. Catalog work overlaps parsing; separate medians do not add. Its development Rust compiler build took 205.106 s using the existing build cache; that is not codec compilation time.

## Checks and provenance

The exact production byte-reuse module and its committed tests pass optimized verified native JIT comparisons over **6,912 combinations plus three refusal fixtures**. They exercise current dynamic phi forwarding, live values outside the bypass, later bounds/defaults, changed equal-edge indices, writes and absent immutable authority. The minimal harness copies the existing comparison helpers verbatim. It is not a full GPU test-suite result.

Every timed variant passes **162 output/inflation/alias observations**, **six checked failure cases** and an active stack of **4,848–4,960 B**, below 5,280 B. The byte-wide/narrow stack is 4,848 B; byte-complements/hash is 4,864 B. No new twelve-recovery campaign or complete 81-case qualification is claimed. Module layout, warning policy and diff whitespace checks pass. Exact JIT test boundaries are allowlisted. The repository-wide unsafe audit remains blocked by the pre-existing missing allowlisted file `benchmarks/mech-deflate/comparison/rust/src/pinned_allocator.rs`; it is not reported as passing. The earlier interrupted full GPU build remains uncompleted.

`verify.py` verifies **14 native emissions, 18 runtime orders and ten complete-build orders**, all archive hashes, committed source identity, native text sizes, canonical bytecode, the native-test receipt and every paired runtime ratio. `source-and-receipts.tar.gz` includes complete compiler overlays, recipes, profile inputs, normal objects/metadata, diagnostics, tests and failures. `raw-runtime.tar.gz` contains raw latency observations and actor pins. The compiler provenance map identifies the exact source for each compiler SHA; inherited experiment-family labels in individual receipts are not authoritative source pins. No archived compiler binary is required.

## Reproduce from the PR's compiler source

Build `compile_profile` from `4ebb148ada` using the recorded nightly 2026-03-03 release command, then run:

```sh
python3 reproduce.py /path/to/compile_profile /tmp/byte-control --variant control
python3 reproduce.py /path/to/compile_profile /tmp/byte-wide --variant wide
python3 reproduce.py /path/to/compile_profile /tmp/byte-complements --variant complements
python3 verify.py
```

`reproduce.py` restores hash-checked portable profiles and verifies all three emitted artifact hashes. The complementary-predicate reproduction has passed. Another machine must qualify runtime performance independently. The broader archived guard-gate prototype requires its own recorded source overlay; it is not part of the live compiler.

[Previous DP experiments and complete build cohort](../previous-dp-cost-20261119/README.md) · [Qualified full runtime evidence](../encoder-size-gap/immutable-current-read-study/README.md)
