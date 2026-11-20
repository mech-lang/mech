# Huffman replay and repaired full-codec source alternative

**The repaired source retains 81/81 Roc and 63/81 libdeflate wins in both complete 96-block orders, including every required named win.** No 64th C win is established. It takes **11.4–12.5% less time on random 4 KiB** and **10.0–10.8% less on random 16 KiB** than the selected build in those matrices.

This is a separately pinned ordinary-Mech alternative. The selected `m1-narrow-packet-words` recipe remains **63,804 B** with the original **2,719-line source** and canonical bytecode. The repaired alternative is **2,732 lines**, **72,196 B** text and **2.346 s** through final executable linking. Its added size and build cost are reported alongside the runtime gains. No LLVM emission, new LLVM dependency or shipping native-byte patch is introduced.

[Full repaired source](program-hybrid-u32.mec) · [Exact source generator](generate_codec.py) · [Native generation recipes](recipes.json) · [Portable verifier](verify.py).

## What changed

The original packed sorter collects nonzero frequency/symbol keys and uses Shell's gapped insertion sort. The first source candidate replaced that one body with the existing count/cumulative/place/heap helpers. It improved random-input time but lost the existing 320-byte overlap win in both focused orders, measuring **1.042628× / 1.044563× C**. That candidate and every adverse observation are retained.

The repair keeps Shell sorting for an alphabet below 64 symbols or fewer than 32 active symbols. It uses buckets for the remaining dense alphabets. These are algorithm choices based on the alphabet, with no benchmark-name or input-name checks. A second repair uses **U32 symbol counters**. Its complete count/cumulative/place recurrence is bounded by the alphabet's at most 288 symbols; the zero-initialized counters and cumulative placement indices therefore fit U32. Tree length counts keep their existing types. The default codec source is unchanged.

The new counter allocation is inside the bucket branch. Across all 162 success observations, the repaired U64 source adds exactly one **288 × 8 B** table per additional payload request, and U32 adds **288 × 4 B**. Every other success field, including output bytes, borrowed aliases, initialized-copy count, peak payload extent and empty private ownership at return, matches the original-source observation. These are requested initialization extents, not physical clearing measurements.

## Identical-input phase replay

The replay uses **212 actual frequency vectors** captured from pinned libdeflate on nine cases, including its fresh compressor construction and compression stages. Four standalone arms receive exactly the same vectors:

1. The pinned C bucket/heap sorter, extracted from source.
2. A C expression of Mech's packed Shell recurrence.
3. Current Mech Shell helpers, extracted unchanged into a small program.
4. Mech's existing bucket/heap helpers in a small program.

Each invocation includes fresh output scratch allocation, result copying into prepared caller storage and cleanup. The Mech arms include their checked entry and heap context. C uses its own local counter storage. These are standalone source-extracted phases, rather than the actual inlined shipping helpers. C's captured sequence includes calls that Mech can skip or reuse; **sequence sums are not Mech whole-call phase times or speed forecasts**.

All four arms produce the exact independently sorted packed keys on every vector, with unchanged output canaries. Timing uses all 24 actor permutations, 32 sequence repetitions per block and both nine-case orders. Separate counter builds record frequency reads, counter updates, key comparisons and output writes. Clock builds disable those counter increments.

| Captured sequence | Native C | C Mech recurrence | Generated Mech Shell | Generated Mech buckets |
| --- | ---: | ---: | ---: | ---: |
| Natural 1 KiB, forward | 6.46 µs | 6.29 µs | 16.88 µs | 10.32 µs |
| Random 4 KiB, forward | 5.31 µs | 14.81 µs | 46.48 µs | 10.51 µs |
| Random 1 MiB, forward | 87.80 µs | 44.43 µs | 169.19 µs | 165.79 µs |
| Overlap 320 B, forward | 6.40 µs | 3.26 µs | 9.13 µs | 9.29 µs |

The native C versus C-recurrence comparison shows that the preferred algorithm depends on the distribution. On small random input, count sorting avoids substantial Shell work. On large random input, overflowing frequencies make C's heap route more expensive. The additional Mech cost also includes checked lowering and context/array handling; this replay does not assign all of that difference to a single compiler pass. All reverse observations and work counters are in the archive.

## Complete codec results

Two focused campaigns compare the selected build, the source controls/repairs, Roc and C using all **120 permutations of five actors** per case in both 21-case orders. The first preserves the raw bucket overlap regression. The second shows that both repaired sources recover every observed current point win. U32 saves **224 B** versus the repaired U64 source and takes about **0.5–1.6% less random-4-KiB time** in those paired orders. Small slowdowns that retain the named wins are allowed by the selection policy.

The U32 source then completes both **81-case, 96-block** orders:

| Order | Roc wins | C wins | Required named wins missing |
| --- | ---: | ---: | ---: |
| Forward96 | 81 | 63 | 0 |
| Reverse96 | 81 | 63 | 0 |

These are point outcomes under the user's selection rule. Confidence analysis is separate and has not been performed for this new source alternative.

| Full-matrix case | Candidate / selected, forward / reverse | Candidate / C, forward / reverse |
| --- | ---: | ---: |
| Overlap 320 B | 0.995441 / 0.990902 | **0.967712 / 0.966298** |
| Natural 1 KiB | 0.994999 / 0.997992 | 1.235062 / 1.235610 |
| Natural 4 KiB | 0.990127 / 0.986313 | **0.959945 / 0.966359** |
| Random 4 KiB | **0.886398 / 0.874594** | 1.322873 / 1.343378 |
| Random 16 KiB | **0.900051 / 0.892061** | 1.327652 / 1.335609 |
| Random 1 MiB | 0.978003 / 0.978603 | 1.254215 / 1.255698 |
| Distribution change | 0.938038 / 0.932642 | **1.195996 / 1.198391** |

These compare the composite candidate to the selected recipe. Both source arms also use the earlier optional low-word carry and their own proved scalar homes/layouts. The first five-actor campaign isolates the all-bucket source change against a held original-source control; the second isolates U32 versus U64 in the repaired source. The distribution and large-random composite gains must not all be attributed to sorting.

The narrowest retained C win is overlap, with **3.23–3.37% less time**. The largest is empty input, approximately **148.4–148.6× faster** under the fresh-construction boundary. Among inputs at least 1 KiB, repeated 1 MiB takes **35.7–35.9% less time**, about **1.56× faster**. The closest remaining loss is distribution change: the worse order requires **16.55% less time** to reach C parity. Thus 64 is still a substantial next step.

## Complete build and size cost on Apple M1

| Actor | Encoder text | Source through final executable link | Stripped executable |
| --- | ---: | ---: | ---: |
| Selected Mech | **63,804 B** | **2.219190 s** | **276,072 B** |
| Original-source experiment control | 67,268 B | 2.215333 s | 276,056 B |
| All-bucket source | 71,060 B | 2.298131 s | 292,568 B |
| Repaired U64 source | 72,420 B | 2.349238 s | 292,568 B |
| Repaired U32 source | **72,196 B** | **2.346140 s** | **292,568 B** |
| Roc | 57,696 B | 2.986472 s | 191,912 B |
| libdeflate C | 37,080 B | 0.485813 s | 155,752 B |

Fourteen balanced serial fresh-process trials per actor include final linking. Compilers and fixed support are prebuilt equally; bootstrap and stripping are separate. Every Mech object, full metadata and bytecode matches its own recipe in every trial. Every stripped executable matches its actor's warmup identity. **All seven actual executables also pass all 81 input/inflation checks** after clocks finish; Mech and Roc outputs equal the selected stream for the same input.

The selected paired build ratios are **0.741637× Roc / 4.578823× C**. The repaired U32 ratios are **0.785557× Roc / 4.837380× C / 1.058165× selected**. Its text is **1.251× Roc / 1.947× C**. The selected recipe has no material compile-time change.

| Mech compiler phase median | Selected | Repaired U32 |
| --- | ---: | ---: |
| Parsing | 702.5 ms | 705.2 ms |
| Semantic lowering | 217.2 ms | 224.0 ms |
| Native pipeline | 1,226.4 ms | 1,343.3 ms |

Most added build cost is native compilation. The source alternative introduces 640 canonical actions; phase medians do not sum exactly. This is not progress toward C size/build parity.

## Correctness, errors, stack and source reconstruction

Each full source arm has 162 exact output/inflation/alias/cleanup observations, six checked allocation-error cases and 12 injected failures with exact recovery. The original-source control reuses the prior receipts only after proving the entire worker and all three native artifacts identical. The separate source arms map only named unchanged error actions in the common prefix/suffix. The newly introduced sort actions are not mapped to unrelated old actions. The original default error policy remains unchanged.

Actual candidate stack peaks are **4,688 B all-buckets, 4,720 B repaired U64 and 4,752 B repaired U32**, below the **5,280 B** cap. Earlier progress reports used the conservative **4,864 B** qualification-pair maximum, which includes the maintained control. The per-actor figures above come from the saved stack report.

Normal and checked register-allocation emission match all three artifacts. **Eight actual rebuilds using the public source/profile paths** reproduce the four source arms in both modes. Generation uses ordinary Mech and Cranelift; the shipping worker is always linked from authentic compiler output.

On matching Apple AArch64 hardware with a compiler built from this branch:

```sh
python3 reproduce_codec.py /path/to/compile_profile /tmp/mech-huffman --role hybrid-u32
python3 reproduce_codec.py /path/to/compile_profile /tmp/mech-huffman-checked --role hybrid-u32 --checked
python3 verify.py
```

The complete raw replay, both adverse focused orders, both repaired focused orders, both full matrices, full sources, native artifacts, generation commands, checks and all 14 build orders are stored in the four hashed archives. [Archive manifest](archives.json) · [Verification receipt](verification.json). The portable verifier uses Python's standard library. It retains timer-resolution zeros and recomputes positive block means and paired ratios; it does not filter inconvenient samples.

The larger source remains a qualified runtime alternative; the smaller default remains selected. The next work targets matcher state/lifetimes and producer facts across its private boundary, together with actual Mech semantic/frequency-input capture. **64 C wins is next; 81 remains the goal.**
