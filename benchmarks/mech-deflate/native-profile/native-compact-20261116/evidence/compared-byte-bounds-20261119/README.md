# Sharing admitted byte-read bounds, 19 November 2026

The active goal remains **63/81 libdeflate wins while retaining every existing peer win and 81/81 Roc**. The qualified selection remains **63,532 B, 81/81 Roc and 62/81 libdeflate**. This pass does not earn the 63rd win. The qualified 63,532 B and 63,692 B comparison artifacts remain available.

Measured compiler implementation [`7259e858f3fc4a979c89378d3c73f45af4e6151c`](https://github.com/mech-lang/mech/commit/7259e858f3fc4a979c89378d3c73f45af4e6151c) carries the original scalar read's logical-bounds proof into subsequent immutable byte reuse. The source is available in PR #995; the publication commit additionally clarifies Rust comments without changing behavior. The original 2,719-line Mech codec and canonical bytecode are unchanged. No LLVM dependency is added. Disabled mode and the older byte-reuse mode both reproduce their previous object, complete metadata and bytecode exactly.

## Runtime discovery

Each candidate runs nineteen cases with twenty-four balanced blocks in both orders: tiny/overlap/periodic guards, natural 1/4/8 KiB, random 1/4/8/16 KiB, 32/64-KiB boundaries, random 1 MiB and distribution change. These are point estimates from discovery, not full 81-case qualification or a precision-supported new floor. Ratios below one are faster. The table extracts natural 4 KiB.

| Source variant | Encoder text, B | Mech/libdeflate, forward / reverse | Experiment/control, forward / reverse |
| --- | ---: | ---: | ---: |
| byte-bounds-geometry-wide-profiled | 63,548 | 1.038151 / 1.051211 | 0.972805 / 0.979291 |
| byte-bounds-geometry-narrow-profiled | 63,564 | 1.035632 / 1.045428 | 0.981242 / 0.975262 |
| byte-bounds-geometry-complements-profiled | 63,532 | 1.046024 / 1.056532 | 0.974143 / 0.982209 |
| byte-bounds-geometry-hash32-profiled | 63,532 | 1.036243 / 1.051758 | 0.974910 / 0.985854 |
The best narrow candidate is **1.035632× / 1.045428× C**, leaving approximately **3.4–4.3% less candidate time** to parity on this workload. All variants remain slower than C here. The source simplification does not establish an additional whole-encoder speedup over previous-DP forwarding or byte reuse alone. The promising wide/narrow candidates remain available for combination with a larger change; no new runtime selection is made.

Wide, narrow and complementary-predicate candidates keep the tight natural-8-KiB, 320-overlap and periodic-1-KiB C wins in both discovery orders. Narrow's natural-8-KiB ratios are 0.966586 / 0.968967; 320-overlap is 0.994800 / 0.991690. This repairs the earlier narrow byte candidate's discovery losses, without proving a full new score. **The hash combination loses periodic 1 KiB in the forward order (1.029639× C)**; the adverse observation is retained. Large random 1 MiB remains about 1.36× C. Every case, not just these extracts, is archived.

## Mechanism and repair

The earlier byte pass reused loaded values but retained later bounds comparisons and zero choices. The new `MECH_REUSE_COMPARED_BYTES=proved` mode records an opaque, nonserialized hint immediately after an actual numeric scalar read, inside its original logical bounds check or existing current-window proof. The hint identifies the surviving load instruction and source array. The consumer independently requires the exact private phase proof, fixed immutable byte-vector geometry and current packet pointer/length.

At reuse, the actual surviving load address minus its current packet pointer yields the ordinal. The later predicate must compare that same exact modular ordinal against the same logical length. Only then can the already-proved true choice replace a later default. Unsupported identities retain their checks. The original admitted read still dominates the comparison, and the reused path remains effect-free. No extra read permission, weaker resource accounting, speculative out-of-bounds load or public metadata authority is introduced.

The first prototype retained temporary SSA index/length names. Cleanup invalidated some of them, and the real codec caught a compiler panic before producing a new-mode artifact. That failed source/compiler receipt and backtrace are archived. The repaired design avoids those temporary references by recovering current geometry from the surviving load and sealed packet. The prototype contributes no runtime observations. Its disabled and old-byte modes still reproduce their controls exactly.

On the actual codec, the proof removes **two later bounds decisions** in the initial mismatch path. Compared with the earlier raw wide byte candidate, untimed counters show:

| Workload | Fewer matcher instructions | Fewer comparisons | Fewer scalar choices | Added stack loads/stores |
| --- | ---: | ---: | ---: | ---: |
| Natural 4 KiB | 14,672 | 3,668 | 3,668 | 0 / 0 |
| Random 1 MiB | 3,802,752 | 950,688 | 950,688 | 0 / 0 |

These are instruction counts, not cycle savings or a runtime forecast. Counter inputs relink to the exact source-generated worker before instrumentation; diagnostic clones are never latency inputs. Fresh layout profiles train on multiple natural/random workloads. Unchanged scorer and sort bodies reuse their verified profiles.

After profiling, wide is **63,548 B**, narrow **63,564 B**, complementary predicates **63,532 B**, and hash **63,532 B**. Relative to their prior byte-reuse versions, the whole encoder saves 32 B, 32 B, 80 B and 32 B respectively. The qualified selected encoder remains unchanged.

A separate remaining opportunity is the existing vector equality scan's mismatch fallback: it reloads scalar words already read by the vector tier. Any direct continuation must preserve the exact scalar position, first mismatch, live values and all later cache updates. Scorer winner-frequency measurement is another pending experiment before considering cost-only reduction with conditional winner reconstruction. Neither is claimed implemented here.

## Complete source-through-link builds

Ten balanced serial fresh-process orders per actor include final executable linking. Compilers and fixed support are prebuilt equally; compiler bootstrap is excluded and strip is separate. Every Mech trial reproduces all generated artifact hashes. These measurements use source `7259e858f`; separate historical medians do not establish causal savings.

| Actor | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Current compiler, retained encoder | 63,532 B | 2.216112 s | 2.227292 s | 276,072 B |
| Proved byte reuse plus wide DP recurrence, unselected | 63,548 B | 2.219958 s | 2.231677 s | 276,072 B |
| Proved byte reuse plus narrow DP recurrence, unselected | 63,564 B | 2.219512 s | 2.232275 s | 276,072 B |
| Roc | 57,696 B | 2.990595 s | 3.001634 s | 191,912 B |
| libdeflate | 37,080 B | 0.487171 s | 0.498152 s | 155,752 B |

Paired build ratios: retained **0.741074× Roc / 4.602640× C**; wide **0.742270× Roc / 4.556253× C**; narrow **0.740755× Roc / 4.565731× C**. Wide and narrow are **1.000418×** and **1.000275×** their same-compiler retained build. These small point differences do not establish a compile-time change. Paired ratios and ratios of independent medians need not match. Build time is separate from compression latency.

Narrow phase medians: parsing **703.054 ms**, semantics **217.414 ms**, artifact preparation **6.847 ms**, encoding **2.548 ms**, native pipeline **1,221.073 ms**. Catalog work overlaps parsing and separate medians do not add. The development Rust compiler rebuild took 204.821 s using the existing build cache; this is not clean bootstrap or codec compilation time.

## Checks and reproduction

The exact production module and committed tests pass **20,736 optimized verified native comparisons plus three refusal fixtures**. Cases cover same/different logical lengths, original admission, dynamic phi transport, changed equal-edge indices, escaping values, later defaults, writes and missing immutable authority. Each timed candidate passes **162 output/inflation/alias observations**, **six checked failures**, and active stack **4,848 B** (wide/narrow) or **4,864 B** (complements/hash), below 5,280 B. No full 81-case qualification or new twelve-recovery campaign is claimed. The prior uncompleted full GPU test build is not reported as passing.

Module layout, warning policy and whitespace checks pass. The repository-wide unsafe audit remains blocked by the pre-existing missing allowlisted file `benchmarks/mech-deflate/comparison/rust/src/pinned_allocator.rs`, as recorded in the previous study. No new unsafe boundary is introduced beyond the existing exact JIT-test path.

`verify.py` verifies **12 source emissions, eight complete discovery orders, ten complete-build orders**, committed source identity, all archive hashes, native test provenance, native text sizes, canonical bytecode and paired runtime ratios. The two archives contain complete source overlays and recipes, raw timing observations and actor pins, profiles, diagnostic counter provenance, failed prototype logs, and the repaired test receipts. The control baselines remain in [the previous byte study](../compared-byte-reuse-20261119/README.md).

Build `compile_profile` from `7259e858f` with the archived nightly 2026-03-03 release command, then run:

```sh
python3 reproduce.py /path/to/compile_profile /tmp/bounds-control --variant control
python3 reproduce.py /path/to/compile_profile /tmp/bounds-narrow --variant narrow
python3 verify.py
```

The portable narrow reproduction has passed all three artifact hashes. Another machine must qualify its runtime independently. The original-source compiler output is the latency input; no machine-code patch is shipped.
