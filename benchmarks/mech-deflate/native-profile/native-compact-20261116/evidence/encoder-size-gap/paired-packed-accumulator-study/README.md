# Paired packed accumulator study

**Keep this candidate for margin repair.** It retains every prior individual **81 Roc / 62 libdeflate win with confidence support in both full 96-block orders**. The new natural-4-KiB win is supported in forward but remains uncertain in reverse. The qualified default stays **63,612 B / 81 Roc / 62 C**; no qualified 63rd win is claimed.

The carried paired loop takes **1.50% / 1.59% less natural-4-KiB time** than its matching one-accumulator control in the two complete 21-case discovery orders. Both full 81-case/24-block screens win **81 Roc / 63 C**, preserving every previous named win. The stricter 96-block qualification then records:

| Full 81-case order | Roc point / precision wins | C point / precision wins | Natural 4 KiB / C | 90% paired interval, Mech time change vs C |
| --- | ---: | ---: | ---: | ---: |
| Forward96 | 81 / 81 | 63 / 63 | 0.984695× | −1.990% to −1.078% |
| Reverse96 | 81 / 81 | 62 / 62 | 1.000362× | −0.429% to +0.528% |

Reverse misses parity by **0.036%**. It loses no previously qualified case. This is a specific remaining margin problem, not a reason to discard the candidate. Every original observation, including the adverse reverse order and the initial inventory-replay error, is preserved.

Compiler source: [`31910aff67`](https://github.com/mech-lang/mech/commit/31910aff67d9f902da9f6f5239c7efc403c82b01). Enable `MECH_ORDERED_PACKED_COST_INDEX=1` and `MECH_ORDERED_PACKED_ACCUMULATORS=2`; `MECH_ORDERED_VECTOR_CARRY_LANES=1` selects the carried variant. The original 2,719-line Mech codec and canonical bytecode are unchanged. No LLVM dependency is added.

The existing U16 best/absolute-stop guard admits the bounded key, with the original optimized vector loop on a miss. Two four-candidate groups update separate packed minima before advancing by eight. One optional group of four is evaluated before the paired prefix. Its reads are immutable and already admitted; packed ordinals preserve earliest original source order even when this tail is visited first. The existing scalar remainder still processes zero to three elements. All traversals, errors, source charges and finite accounting are preserved. No unsafe production code is added.

All three artifacts reproduce exactly for **23 older recipes**, including all seven preceding packed/profile variants. The two new workers pass **162 output/inflation/alias observations, six checked failures, twelve injected failures followed by twelve exact recoveries**, normal/checked identity and the **5,280 B** stack cap. Peak stack is **4,736 B**. Eighteen optimized tests pass with packing off, paired packing on, and paired packing with carry plus latch testing; each configuration includes **61,440 key-fixture calls** and **16,384 previous winner/item calls**. The direct fixture exercises the actual paired emitter with/without carry and header/latch testing, 0/4/8/16/20/24/28-element boundaries, wide costs, sentinel ties, cross-group/optional-tail ties and the 65,535 boundary.

## Discovery and executed work

Both candidates use the same declared eight-case profile policy as their matching one-accumulator controls. All prior wins present in all four complete 21-case/24-block orders survive. Times are Mech / peer, lower is faster.

| Variant | Encoder text | Natural 4 KiB / C, forward / reverse | Natural time / matching control |
| --- | ---: | ---: | ---: |
| `paired-key-probe-profiled` | 63,836 B | 1.005587× / 1.001764× | 0.986590× / 0.992672× |
| `paired-key-carry-probe-profiled` | 63,852 B | 0.993819× / 0.999276× | 0.984993× / 0.984129× |

Untimed raw-scorer counts on natural 4 KiB:

| Scorer | Instructions | Branches | Stack loads / stores |
| --- | ---: | ---: | ---: |
| One packed accumulator | 3,379,620 | 573,275 | 17,305 / 18,470 |
| Two packed accumulators | 3,359,267 | 556,013 | 17,310 / 18,475 |
| One accumulator, carried indices | 3,331,665 | 555,990 | 17,305 / 18,470 |
| Two accumulators, carried indices | 3,343,871 | 549,581 | 17,310 / 18,475 |

The carried paired version removes 6,409 branches while adding 12,206 instructions and five stack loads/stores. Its measured latency improves despite the larger instruction count. Separate accumulators and unrolling target dependency/branch costs; no hardware-counter attribution to either mechanism alone is claimed. Original baselines and the preceding layout repair are in the [packed-key study](../packed-cost-ordinal-study/README.md).

## Complete executable builds

Apple M1, macOS 15.6.1, arm64. Ten balanced serial fresh-process trials per actor include source compilation through final executable linking. Compilers and fixed support are prebuilt equally. Bootstrap and strip are separate. Every generated Mech object, complete metadata and canonical bytecode is hash checked.

| Actor | Encoder text | Source through final executable link | Stripped executable |
| --- | ---: | ---: | ---: |
| Qualified histogram | 63,612 B | 2.209647 s | 276,072 B |
| Paired packed probe | 63,836 B | 2.216417 s | 276,072 B |
| Paired packed carried probe | 63,852 B | 2.215453 s | 276,072 B |
| Roc | 57,696 B | 2.985657 s | 191,912 B |
| libdeflate C | 37,080 B | 0.481600 s | 155,752 B |

The carried probe is **1.107× Roc / 1.722× C encoder text**. Paired complete-build ratios are **0.742850× Roc / 4.606302× C**, and **1.002289×** qualified control. No compile-time gain is established.

Its phase medians are parse **701.636 ms**, frontend **214.650 ms**, artifact **6.850 ms**, encode **2.557 ms**, native **1227.220 ms**, final executable link **35.211 ms**. Independent phase medians do not sum exactly to the overall median. The minimal compiler release rebuild took **228.069 s**; it is excluded from codec-build timing and is not a clean-bootstrap comparison.

## Replay and qualification scope

The qualification uses the existing v3 90% coverage and 8,000-resample method, with a stricter target: all original 81 Roc / 62 C named wins plus natural 4 KiB, with C floor 63. Every required original case remains precision-supported in each order; only reverse natural 4 KiB is unsupported. Small slowdowns remain allowed when prior peer wins survive. The stricter result is not promoted.

The initial replay found that temporary inventories call the prefix file `prefix`, while the frozen inventory names it by its SHA256. It stopped after both timing runs had completed. The correction verifies all 81 input/prefix/expected files byte-identical, including sizes, raw bytes and capacities, preserves original runtime pins, and reconciles only the canonical replay inventory digest. This is the existing `full-timing.py` reconciliation method. No clock was rerun or original data changed. Both the failed replay log and corrected replay/source are archived.

`checks-and-builds.tar.gz` contains compiler/test sources and commands, 23 exact older recipe replays, worker/stack/recovery checks, full24/96 plans, inventory reconciliation and full build observations. `raw-runtime.tar.gz` contains all four discovery orders, both full24 orders and both full96 orders. Every member is hashed in `archive-files.json`. Both portable recipes reproduce all three artifacts exactly in normal and checked emission, recorded in `portable-replays.json`.

```sh
python3 verify.py
python3 reproduce.py /path/to/compile_profile /tmp/paired-replay --role paired-key-carry-probe-profiled
```

The verifier checks source/object/worker bindings, exact prior replays, tests, errors, stack, recovery, all raw discovery/full24/full96 calls, actor cleanup, inventory reconciliation, both confidence reports and complete builds. Its success means the study is reproduced, including the failed new-win gate; it does not claim 63 qualified C wins. The full GPU suite is not claimed. The repository unsafe audit remains blocked by the pre-existing missing archived `pinned_allocator.rs`.

Next, retain the paired carried loop and repair its margin by (1) peeling the optional four-element group at the beginning, so the main loop can use the existing end bound, and (2) widening the packed ordinal before subtracting its bias, so item reconstruction can fold the offset. Test both changes separately and together against this exact candidate. Neither change is implemented in this checkpoint. Preserve both qualification orders and every named old win.
