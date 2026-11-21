# Histogram counter capacity and ordered visit grouping

**Selected: `m1-histogram-capacity-grouped`, 63,192 B, preserving every named 81/81 Roc and 63/81 libdeflate win in both full 81-case, 96-block orders.** A proved entry counter-capacity bound permits four ordered byte histogram visits per iteration. The original **2,719-line Mech codec and canonical bytecode are unchanged**.

| Runtime case / order | Mech | Matched Mech control | libdeflate | Paired Mech ÷ C time | Mech time reduction to parity |
| --- | ---: | ---: | ---: | ---: | ---: |
| Random 1 MiB, forward | 23.024375 ms | 23.382167 ms | 20.340292 ms | 1.131646× | 11.63% |
| Random 1 MiB, reverse | 23.033958 ms | 23.393646 ms | 20.342875 ms | 1.132528× | 11.70% |
| Random 4 KiB, forward | 0.109104 ms | 0.110250 ms | 0.076916 ms | 1.428573× | 30.00% |
| Random 4 KiB, reverse | 0.108521 ms | 0.109459 ms | 0.075750 ms | 1.448058× | 30.94% |

Paired 1-MiB time is **0.984931× /0.984668×** the matched 63,048-B control, about **1.51–1.53% less time**. The grouped candidate adds **144 B** and remains 180 B below the earlier 63,372-B compact recipe. The original neutral scalar-capacity and SIMD trials remain retained. No formal confidence claim or **64th libdeflate win** is added; the closest remaining case still needs about **11.6–11.7% less Mech time**.

Fresh-process source through final executable linking takes **2.202398 s Mech /2.984846 s Roc /0.480850 s C**, paired **0.738298× Roc /4.582297× C**. Twelve balanced serial orders use equally prebuilt compilers/fixed support. Parse/frontend/native/link medians are **0.702117 /0.218718 /1.209371 /0.035314 s**. Bootstrap and strip are separate; no build, test, profile, counter, archive or Git work overlaps primary clocks.

Encoder text is **1.095× Roc /1.704× C**. Stripped executables remain **276,072 B Mech /191,912 B Roc /155,752 B C**. Measured peak stack remains **4,736 B**, below the 5,280-B cap. The actual main default reproduces object, full metadata and bytecode exactly from the integrated compiler source. All five source arms pass strict output/alias/resource/error/recovery/stack checks and **10 current public normal/checked source replays**. **40,725 focused native calls**, 79 maintained optimized scalar tests and the current-word/SSA/increment families pass. Portable verification checks **6,630,336 saved runtime observations**, retaining 995,679 zero samples, all twelve complete-build orders and **486 actual stripped-executable outputs**. The full GPU suite is not claimed.

[All retained trials and profiling](README.md) · [All 81 elapsed times and parity gaps](PARITY.md). Compiler implementation and stricter admission recognition are integrated at `c41789c828b601512735f89b1476396a89b239d2`. The optional flags remain off globally; only the named qualified source recipe enables the selected combination.

Fresh five-second native samples put matching at **74.3%**, scoring at **13.9%**, the remaining packed histogram at **7.5%**, and the optimized byte histogram at **1.0%** of encoder samples on random1MiB. On random4KiB, matching is **43.4%**, packed histogram **13.5%**, Huffman **11.9%**, scoring **11.1%**, and byte histogram **3.9%**. These encoder-root exclusive shares guide the next broad optimization; they are not exact phase elapsed times. [Sampling derivation and correction](PROFILE.md).

The existing specialized histogram hoists four bounds checks but retains a U32 overflow check on every input byte. The new option first proves a fixed source loop: pure invariant entry setup, one selected read/increment/write per visit, a single unit-stride backedge, and no route back through the entry prefix. Unknown calls, additional stores, changed stored values or addresses, extra backedges and restart paths keep the original checks.

The runtime admission checks the existing whole-input and 256-bin extent bounds, `remaining = end - cursor <= U32MAX`, and every existing bin `<= U32MAX - remaining`. Empty loops need no scan. No zero initialization is assumed. A miss enters the original checked source loop, preserving defaulting reads, partial writes and error origins. The selected fast path executes four byte-read/counter-read/write visits in original order, then uses the original admitted loop for the last zero to three bytes. Aliased views remain supported; tests include updates that change the next input byte.

The alternative SIMD admission scans 64 groups of four U32 counters using unsigned vector maximum. It does not consistently improve elapsed time, so the selected recipe uses the smaller scalar admission plus ordered visit grouping. Existing matcher, Huffman and scorer register/layout profiles are reused only after actual native body bytes and relocations match the qualified control. No new LLVM dependency, source rewrite or generic operation dispatch is introduced.

| Timed panel arm | Text B | Random 1 MiB Mech ms, forward /reverse | Paired Mech ÷ C | Blocks /case |
| --- | ---: | ---: | ---: | ---: |
| capacity-first | 63,096 | 23.403708 /23.398125 | 1.152511 /1.152813× | 24 |
| capacity-grouped | 63,192 | 23.035604 /23.038354 | 1.133525 /1.133812× | 96 |
| capacity-vector-grouped | 63,240 | 23.033125 /23.024166 | 1.134260 /1.134365× | 96 |
| capacity-vector | 63,144 | 23.386062 /23.397000 | 1.152090 /1.152408× | 96 |

Only `capacity-grouped` has both complete 81-case qualifications. Every variant passes 162 exact output/inflation/alias/resource/cleanup observations, six named checked errors, twelve injected-failure recoveries and the stack cap. Current public normal/checked replays come from the reviewed compiler; the earlier compiler, optimized tests, neutral scalar study and all initial replays remain archived. Tightening admission recognition leaves every measured codec artifact exact, so the timed worker identity is preserved. Maintained optimized original/decoded scalar execution, finite private witnesses and direct Bool/U64/I64/F64 emission remain required.

The compiler used for codec build timings is the release-optimized compiler. The reviewed optimized test library uses a package opt-level=1 override; generated JIT entries still use the speed optimizer and register-allocation checker. Bootstrap construction overlaps between separate Cargo targets and is not an isolated compiler-bootstrap comparison.

Fresh native sampling records both Mech and C on random 1 MiB and random 4 KiB, with all 200,800 diagnostic outputs checked and saved. Sampling is outside all primary clocks. Compiler function boundaries differ, so sampled phase shares are guides, not exact matched phase elapsed times.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/histogram-counter-capacity/verify.py
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/mech-histogram
```

To reproduce any retained arm from compiler source:

```sh
study=benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/histogram-counter-capacity
python3 "$study/build_compiler.py" /tmp/mech-histogram-compiler
python3 "$study/reproduce.py" /tmp/mech-histogram-compiler/release/compile_profile /tmp/mech-histogram-native --role capacity-grouped
```

Add `--checked` for checked register-pair emission. Source options are `MECH_HISTOGRAM_ENTRY_COUNTER_CAPACITY`, `MECH_HISTOGRAM_GROUP_BYTE_VISITS` and `MECH_HISTOGRAM_VECTOR_COUNTER_SCAN`. All remain off globally. [Native/build/sampling archive](native-and-builds.tar.gz) · [Runtime observations](runtime.tar.gz) · [Manifest](manifest.json) · [Portable verification](verification.json).

**64 libdeflate wins is next; 1× libdeflate across all 81 remains the goal.** Fresh native CPU sampling is preserved separately from the primary timing and will guide the next broad optimization. All slower cases and unselected candidates remain available.
