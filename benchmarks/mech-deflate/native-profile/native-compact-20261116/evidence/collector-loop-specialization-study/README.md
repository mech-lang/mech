# Collector loop split: repaired admission, retained for further work

**Keep the qualified 63,804-byte default and the earlier 72,196-byte Huffman alternative, both with 81 Roc / 63 libdeflate wins.** The new source split is retained with its repair and adverse results. It does not establish a 64th win or replace either qualified build. The next milestone remains 64 C wins; the goal remains 81.

The repaired source uses 0.4–0.5% less time on random 1 MiB, but slows other inputs and loses the required 320-byte overlap win. Encoder text grows to **77,988 bytes** and source-through-final-link time to **2.880 s**. [Results and raw archive inventory](summary.json) · [Recipes](recipes.json) · [Portable verification](verification.json).

## Source change and admission repair

The original 2,719-line codec is unchanged. [generate.py](generate.py) makes separately pinned ordinary-Mech alternatives, with either the original sorter or the earlier qualified Huffman sorter. No LLVM dependency or native-byte patch is added.

The new collector has an outer search loop and a bounded, self-recursive skip run. Observation arrays and block decisions stay in the search loop. The skip loop carries the verified-prefix state; cache/header order, skip counts, observations, block boundaries and compressed output are preserved. The full Huffman combination is a separate **2,754-line** source.

The first split regresses badly: random 4 KiB takes **58.8–61.0% more time** and random 1 MiB **78.3–78.5% more**, against the qualified Huffman control. Its small inner kernels are not evidence of a whole-call improvement. The compiler admits inner search/skip phases but leaves the outer traversal outside that combined native phase. On random 1 MiB, the inner search routine is entered 406,968 times, versus 35 calls to the control's combined native collector.

Moving four observation-field projections before the nested skip call restores the combined outer phase. Adding a skip-count range assertion alone does not restore it; both controls are archived. After scalar-home proofs, the repaired phase has 28 packed input words, versus 29 in the control, and 19 output words, versus 17. This repairs admission, not every cost of the split. The compiler's sensitivity to the field-read order is a lead for improving general nested-loop proofs.

The temporary, read-only compiler inspection patch, its build pin and logs are archived. It reproduces all three qualified-control native artifacts exactly. No Rust compiler or runtime source change remains from this experiment.

## Both complete focused orders

Each order covers the same 21 declared cases and all 24 permutations of four actors: the qualified Huffman control, changed source, Roc and C. Fresh heap/context construction through private cleanup is included. Each call validates output, inflation and aliases. No samples, including timer-resolution zeros, are filtered. Ratios are geometric means of balanced block means; lower is faster.

| Input | Repaired source / Huffman control, forward / reverse | Repaired source / C, forward / reverse |
| --- | ---: | ---: |
| Overlap, 320 B | **1.063942 / 1.049647** | **1.021102 / 1.020588** |
| Natural 1 KiB | 1.026295 / 1.025859 | 1.269836 / 1.271314 |
| Natural 4 KiB | 1.011562 / 1.013428 | 0.990566 / 0.997721 |
| Natural 8 KiB | 1.034673 / 1.026421 | 0.962488 / 0.968159 |
| Random 4 KiB | 0.996351 / 1.023751 | 1.395373 / 1.375865 |
| Random 16 KiB | 1.030094 / 1.014686 | 1.371184 / 1.340577 |
| Random 1 MiB | **0.995334 / 0.995725** | 1.251214 / 1.252666 |
| Distribution change, 16 KiB | 1.043735 / 1.049927 | 1.247723 / 1.253273 |

Both repaired orders retain 21 Roc wins but only eight of the control's nine C wins. Overlap is the sole lost C win in this panel. Full 81-case qualification is not run for a candidate already known to miss a required win. Confidence is separate and unassessed. The initial regression and both repaired orders remain available for combination with later improvements.

## Actual final matcher work

Diagnostic clones count the actual final kernels, after layout, on the same nine cases. On random 1 MiB both collectors are entered 35 times. Checked controller fallback remains outside these counts.

| Random 1 MiB | Qualified Huffman control | Repaired split |
| --- | ---: | ---: |
| Instructions | 204,521,468 | 217,998,082 |
| Branches | 31,907,630 | 29,812,476 |
| Stack loads | 21,120,522 | 21,772,713 |
| Stack stores | 6,553,945 | 7,339,268 |

The split reduces branches but adds other instructions and stack traffic. The small measured runtime gain does not imply an instruction-count gain. Pre-layout counts and the first split's inner-only counts are also retained, with their different scopes.

## Matched complete builds on Apple M1

Twelve balanced serial fresh-process trials per actor include the final executable link. Compilers and fixed support are prebuilt equally. Compiler bootstrap and stripping are separate. Every generated Mech object, full metadata and canonical bytecode is checked against the measured recipe; each trial's stripped executable must match its actor's warmup image.

| Actor | Encoder text | Source through final link | Stripped executable |
| --- | ---: | ---: | ---: |
| Selected original-source default | **63,804 B** | **2.218982 s** | 276,072 B |
| Qualified Huffman source | 72,196 B | 2.351058 s | 292,568 B |
| Initial collector split | 75,476 B | 2.798123 s | 292,584 B |
| Projection-order repair | **77,988 B** | **2.880354 s** | 292,568 B |
| Roc | 57,696 B | 2.989072 s | 191,912 B |
| libdeflate C | 37,080 B | 0.490788 s | 155,752 B |

Paired complete-build ratios are **0.740956× Roc / 4.551324× C** for the selected default, and **0.962776× Roc / 5.943184× C** for the repaired split. All six actual stripped executables pass all 81 inputs: **486 executions**.

| Compiler phase median | Qualified Huffman source | Repaired split |
| --- | ---: | ---: |
| Parsing | 706.3 ms | 711.0 ms |
| Semantic lowering | 224.6 ms | 532.8 ms |
| Native pipeline | 1,346.3 ms | 1,549.2 ms |

The source split increases canonical lowering and native work. It does not deliver a compilation improvement. Phase medians do not sum exactly to the complete-build median.

## Native checks and reproduction

All four source arms pass 162 exact output/inflation/alias/cleanup observations; six named checked allocation errors; 12 injected failures followed by exact recoveries; actual stack measurement; and byte-identical normal/checked register-allocation emission. Stack peaks are 4,816 B original-sort split, 4,912 B Huffman split, 4,944 B original-sort repair and **5,008 B Huffman repair**, below the 5,280 B cap.

Only named allocation-error origins in unchanged source prefixes/suffixes are mapped. Changed collector actions are not arbitrarily equated. Against the original default, the Huffman arms' additional zero-initialized counter extents remain exactly one 288-element U32 table per added request. Other success observations match. Failure-owned extents are measured before caller Drop; the worker separately asserts failed calls publish no output and cleanup reaches zero.

On matching Apple ARM64 hardware, build the compiler from this branch and run:

```sh
python3 reproduce.py /path/to/compile_profile /tmp/mech-collector --role hybrid-project-first
python3 reproduce.py /path/to/compile_profile /tmp/mech-collector-checked --role hybrid-project-first --checked
python3 verify.py
```

The other roles are `original-split`, `hybrid-split` and `original-project-first`. Eight actual replays with these public source/profile paths reproduce every native object, full metadata and bytecode exactly. The portable verifier checks archive/member identities, actual files, all saved runtime observations, all 12 complete-build orders, executable identities, native gates and source generation. It does not claim the entire GPU test suite passed.

## Broader work next

Prioritize repeated work, memory traffic and emitted-code costs across input classes. The history already includes immutable defaulted-read grouping, current copy-phi proofs and pinned-versus-rematerialized load placement; do not rediscover these as new optimizations. A fresh exact-artifact IR dump confirms the qualified build already shares a canonical byte read before permitted backend rematerialization.

Investigate bulk matcher maintenance, including the lowering and ownership of lifted `rebase-node(nodes)`, against C's in-place ARM implementation. The pinned C matchfinder also uses 16-bit positions and window rebasing; wider C timestamps must not be assumed. A fresh diagnostic of phase 362 is archived but its call pattern does **not** identify it as the rebasing operation. Map and measure the actual work before changing its representation or forecasting a gain. A separately pinned explicit buffer-loop alternative can test whether ordinary Mech lowering leaves useful allocation or vectorization gains.

Keep promising alternatives for later combinations. The qualified **81 Roc / 63 C** builds remain intact throughout this work.
