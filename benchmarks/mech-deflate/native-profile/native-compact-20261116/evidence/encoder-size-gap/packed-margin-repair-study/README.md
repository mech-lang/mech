# Packed scorer head-group and item-width study

**Keep the head-group saving and continue optimizing the paired scorer. The qualified default remains 63,612 B / 81 Roc / 62 libdeflate. No new qualified C win is claimed.**

Head-group processing emits **63,836 B**, saving **16 B** and **10,308 executed scorer instructions** on natural 4 KiB against the exact 63,852 B paired-carried candidate. It measures **0.998497× / 1.000382× libdeflate** in the complete forward/reverse 21-case discovery orders; time against its matching held control is **1.000019× / 0.998029×**. The reverse gap remains. All prior wins present in all eight complete discovery orders survive. These panels do not establish a new full score or confidence qualification.

Compiler source: [`2b6ed311d1`](https://github.com/mech-lang/mech/commit/2b6ed311d19b24794bde87b9893301ca4e56dc1c). The original 2,719-line Mech codec and canonical bytecode are unchanged. No LLVM dependency is added.

`MECH_ORDERED_PACKED_HEAD_GROUP=1` moves the optional four-element group to the beginning. The paired loop then reuses the existing end value. The same admitted immutable elements are visited, and packed ordinals retain strict/earliest source choices. In this source epoch the paired entry still requires sixteen elements, so an optional head leaves at least sixteen for the loop. Scalar remainders, wide-cost fallback, checked errors, direct scalar execution and exact source accounting are unchanged.

`MECH_ORDERED_PACKED_WIDE_ITEM=1` widens the ordinal before removing its bias. A full-width offset can fold into that subtraction; unchanged zero ordinal becomes I64 −1 and preserves the entire old item. Its pure total extension is discarded when no winner exists. The feature applies only to a carried full-width item; an index-only result retains the original I32 representation.

For this codec, existing item narrowing already performs the useful fold. **Wide-only reproduces all three original paired artifacts exactly; head+wide reproduces all three head-only artifacts exactly**, both before and after block layout. They receive no separate timing claim. Revisit the older `MECH_ORDERED_PACK_WINNER_AT_EXIT=1` option to expose a real width effect: widening removes **2,955 instructions** from that deferred candidate on natural 4 KiB. The deferred mode still adds instructions overall and does not establish a latency gain. All comparisons and no-ops remain preserved.

## All distinct-code discovery orders

Each row contains complete 21-case/24-balanced-block forward and reverse acquisitions. Every candidate compares against the same exact 63,852 B paired-carried worker, with the same declared eight-case profile policy. Times are Mech / peer; lower is faster.

| Candidate | Encoder text | Natural 4 KiB / C | Natural time / held paired control |
| --- | ---: | ---: | ---: |
| `packed-head-group-probe-profiled` | 63,836 B | 0.998497× / 1.000382× | 1.000019× / 0.998029× |
| `packed-defer-item-probe-profiled` | 63,868 B | 1.007740× / 1.006656× | 0.994591× / 1.016306× |
| `packed-defer-wide-probe-profiled` | 63,868 B | 0.990675× / 1.004507× | 0.995584× / 1.000126× |
| `packed-head-defer-wide-probe-profiled` | 63,852 B | 1.000653× / 1.002794× | 0.995953× / 1.002861× |

Untimed raw-scorer counts on natural 4 KiB:

| Candidate | Instructions | Branches | Stack loads / stores |
| --- | ---: | ---: | ---: |
| Previous paired carried | 3,343,871 | 549,581 | 17,310 / 18,475 |
| Head group | 3,333,563 | 549,581 | 17,310 / 18,475 |
| Deferred item | 3,354,596 | 549,581 | 17,320 / 18,480 |
| Deferred + wide | 3,351,641 | 549,581 | 17,320 / 18,480 |
| Head + deferred + wide | 3,341,333 | 549,581 | 17,320 / 18,480 |

The wider representation removes one instruction per admitted packed reduction in the deferred version. It does not change the eight-case counts without deferred reconstruction. Counts are diagnostic, not latency claims. Zero-call cases retain their original zero weights.

## Source through final executable builds

Apple M1, macOS 15.6.1, arm64. Eight balanced serial fresh-process trials per actor include final executable linking. Compilers and fixed support are prebuilt equally. Bootstrap and strip are separate; every emitted Mech object, full metadata and bytecode is hash checked.

| Actor | Encoder text | Source through final link | Stripped executable |
| --- | ---: | ---: | ---: |
| Qualified histogram | 63,612 B | 2.216508 s | 276,072 B |
| Head-group candidate | 63,836 B | 2.222766 s | 276,072 B |
| Roc | 57,696 B | 2.988164 s | 191,912 B |
| libdeflate C | 37,080 B | 0.482030 s | 155,752 B |

Head-group encoder text is **1.106× Roc / 1.722× C**; paired build time is **0.744078× Roc / 4.610974× C**, **1.001371×** the qualified control. No compile-time gain is established. Phase medians and all observations are in the build archive.

## Validation and portable replay

- Nineteen optimized tests pass in four configurations: default; both new options with header testing; both with lane carry and latch testing; and both with the older deferred-item option. Each configuration includes **94,208 loop/key calls, 16,384 wide-item calls and 16,384 earlier winner/item calls**. The fixtures cover strict ties, complete old I64 items, total offset wraparound, both loop forms, all 16-bit boundaries and unaligned guarded storage. Composition tests preserve source remainders, wrapping, denial, publication and retry.
- All three artifacts reproduce exactly for **25 older recipes**. All six new workers pass 162 output/inflation/alias observations, six checked failures and twelve injected failures followed by twelve exact recoveries. Normal/checked artifacts match exactly. Peak stack is **4,736 B**, below the 5,280 B cap.
- All six portable recipes reproduce exactly in normal and checked emission with relocated profiles. The public verifier checks actual artifact equality against the preceding published study, not only recorded equality booleans. The full GPU suite is not claimed; the existing missing archived allocator still blocks the repository unsafe audit.

```sh
python3 verify.py
python3 reproduce.py /path/to/compile_profile /tmp/head-group-replay --role packed-head-group-probe-profiled
```

`checks-and-builds.tar.gz` preserves compiler and test source overlays/commands, all replays, six candidate workers and checks, instruction counts, negative equivalence results and full build observations. `raw-runtime.tar.gz` preserves all eight distinct-code orders. Every member is pinned in `archive-files.json`. The preceding paired candidate and both complete 96-block outcomes remain available in the [paired accumulator study](../paired-packed-accumulator-study/README.md); its reverse 0.036% miss is not discarded.

Next: test entering the paired vector loop with eight candidates only when packed cost/index bounds are met. The arithmetic needs one eight-candidate group, while the current sixteen-element entry is a performance heuristic. Short wide-cost intervals must retain their scalar path, and wide vector intervals retain their sixteen-element entry. Compare both head and tail forms; repeat the full 81-case confidence gate before claiming win 63. That follow-up is not implemented in this source epoch.
