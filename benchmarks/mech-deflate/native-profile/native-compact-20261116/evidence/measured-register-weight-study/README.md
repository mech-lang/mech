# Measured register spill weights across the matcher

**Both variants retain every named 81 Roc / 63 libdeflate win in both complete 81-case, 96-block orders.** The linear variant uses about **6.3% less random-1-MiB time** than the qualified HF-U32 control. The square-root variant uses about **5.1% less random-1-MiB time**, **4.1% less random-16-KiB time** and **2.3–2.4% less distribution-change time**. No 64th C win is established. These are point qualifications; confidence is reported separately and has not been reassessed here.

The compiler change is committed at [`5b3a4af26`](https://github.com/mech-lang/mech/commit/5b3a4af26c0d39775580ca868e62af36f0049ae2). Its optional register profile must match the complete lowered VCode, including instructions, register types, operand constraints, clobbers and every block. Stale, incomplete, zero and oversized frequency profiles fail compilation. The profile changes spill costs; loop structure and split heuristics retain their previous rules. The **2,732-line qualified HF-U32 codec source and bytecode are unchanged**. This adds no code-generation dependency.

The original **2,719-line / 63,804 B** default remains selected at **81 Roc / 63 C**. With the option disabled, the new compiler reproduces the default and qualified HF-U32 object, full metadata and bytecode exactly. The two measured alternatives are retained for their different gains, with no rejection for a small slowdown on a still-winning input.

## Matched runtime comparisons

Each variant has its own complete 21-case campaign, all 24 actor permutations and both case orders. The control is the qualified HF-U32 source; the peers are pinned Roc and libdeflate 1.26 level 12. Calls include fresh heap/context setup through cleanup. The table is candidate/control time, with values below 1 better. These are separate matched campaigns, not a direct paired comparison between the two candidates.

| Input | Linear forward / reverse | Square root forward / reverse |
| --- | ---: | ---: |
| 320-overlap | 1.002777× / 1.000548× | 0.996771× / 1.005287× |
| prefix-natural-1024 | 0.997323× / 0.999992× | 0.995949× / 0.993236× |
| prefix-natural-4096 | 0.990762× / 1.002769× | 0.988445× / 0.997196× |
| prefix-natural-8192 | 1.021818× / 1.010429× | 1.002082× / 0.999208× |
| prefix-random-4096 | 0.978064× / 0.977967× | 0.975157× / 0.982885× |
| prefix-random-16384 | 0.968438× / 0.967891× | 0.959504× / 0.958842× |
| prefix-random-1048576 | 0.937023× / 0.936580× | 0.949048× / 0.949057× |
| 16384-distribution-change | 1.003419× / 1.009818× | 0.977188× / 0.976053× |

Both complete panel orders retain all 21 Roc and 9 C wins. Linear weighting has 1–2.2% slowdowns on natural 8 KiB while retaining its win; those observations remain included. Every individual call, including zero-duration samples, is preserved.

The full 81-case qualification uses the smaller selected original-source build as control, checks every previously required named win, and runs all 24 permutations four times per case in each order. Both models pass **81/63 in each order**. The full comparison therefore includes the existing Huffman-source improvement as well as the new register weighting.

## Final native work and size

The following counts cover the actual final matcher kernel on random 1 MiB, across 35 calls, with separate diagnostic clones. They exclude checked controller work and are not elapsed-time forecasts.

| Matcher | Instructions | Branches | Stack loads | Stack stores |
| --- | ---: | ---: | ---: | ---: |
| Qualified HF-U32 | 204,521,468 | 31,907,630 | 21,120,522 | 6,553,945 |
| Linear | 197,022,324 | 31,907,630 | 18,676,474 | 1,696,938 |
| Square root | 198,169,162 | 31,909,589 | 20,228,175 | 1,697,137 |

Linear weighting removes **26.38%** of these stack accesses, and square-root weighting removes **20.77%**. This supports the measured broad runtime gains. Source semantics, native admission and the number of collector calls remain unchanged.

| Encoder | Native text | × Roc text | × C text | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Selected default | 63,804 B | 1.106× | 1.721× | 276,072 B |
| Qualified HF-U32 | 72,196 B | 1.251× | 1.947× | 292,568 B |
| Linear | 72,020 B | 1.248× | 1.942× | 292,568 B |
| Square root | 72,004 B | 1.248× | 1.942× | 292,568 B |
| Roc | 57,696 B | 1.000× | 1.556× | 191,912 B |
| libdeflate | 37,080 B | 0.643× | 1.000× | 155,752 B |

Linear saves **176 B** and square root saves **192 B** against their HF-U32 control. File alignment keeps both stripped executables at the same size. Actual measured peak stack remains **4,752 B**, below the 5,280 B cap.

## Complete builds

| Source through final executable link, Apple M1 | Median | Paired × Roc | Paired × C |
| --- | ---: | ---: | ---: |
| Selected default | 2.233524 s | 0.746380× | 4.609191× |
| Qualified HF-U32 | 2.356827 s | 0.789113× | 4.875382× |
| Linear | 2.358269 s | 0.789216× | 4.887660× |
| Square root | 2.362126 s | 0.790604× | 4.879356× |
| Roc | 2.985902 s | — | — |
| libdeflate | 0.483257 s | — | — |

Twelve balanced serial fresh-process trials per actor include final executable linking, with compilers and fixed support equally prebuilt. Strip and Rust compiler builds are separate. Every Mech object, full metadata and bytecode hash is checked in every trial. **No compile-time gain is established.** All six actually timed stripped executables pass all 81 inputs: **486 executions**, with their raw streams saved and independently inflated by the portable verifier.

These build timings use the checked-in profiles. Offline training and layout-profile construction are separate; changing the lowered function requires valid replacement profiles. The complete commands, environment, phases, every trial and compiler source snapshots are archived. The final compiler feature is rebasable source, not an input encoder binary.

## Reproduction and checks

From this directory on Apple ARM64, build the compiler from the checkout and then compile the supplied Mech source using the published profiles:

```sh
python3 build_compiler.py /tmp/mech-register-compiler
python3 reproduce.py /tmp/mech-register-compiler/release/compile_profile /tmp/register-linear --role hybrid-linear
python3 reproduce.py /tmp/mech-register-compiler/release/compile_profile /tmp/register-linear-checked --role hybrid-linear --checked
python3 reproduce.py /tmp/mech-register-compiler/release/compile_profile /tmp/register-sqrt --role hybrid-sqrt
python3 verify.py
```

The recipes reference the repository’s pinned [HF-U32 source](../huffman-sort-replay-study/program-hybrid-u32.mec). No private temporary directory or built encoder input is required by the reproduction commands. All four public normal/checked rebuilds reproduce the exact native object, full metadata and bytecode. The build manifest preserves the exact compiler source snapshot; the committed feature differs from that snapshot only in error-message formatting. Native reproduction checks the emitted artifacts rather than requiring identical host-compiler binaries.

The native gates pass 162 exact outputs/inflations/alias/cleanup observations per candidate, six source-mapped allocation errors, twelve failure/recovery pairs and the stack cap. The existing HF-U32 counter-allocation delta remains the sole resource difference from the original source. All **79 optimized scalar regression tests** pass, preserving the maintained Bool/U64/I64/F64 native paths.

The profile generator is independently runnable after extracting `native-proofs.tar.gz`:

```sh
python3 derive_register_weights.py /path/to/extracted/study/training-native-v5 /tmp/derived-register-weights
```

It derives both complete register profiles from nine mixed-case machine-block counts and the pre-allocation VCode capture. Each case contributes an equal normalized share; linear and square-root transforms have maximum 4096 and floor 1. Folded/empty labels share measured machine-block counts and are recorded explicitly; they are not independent observations. Some original diagnostic top-level hashes copied a reference alias. Binding files verify the actual source image, canonical image, identical linked worker and every translated block record. The original records and timed profile files are preserved. The initial observed-route/physical-register capture failures are also archived; the final capture uses the normal preparation route before emission rewrites registers.

The portable verifier checks all archive members, all **4,648,320 saved runtime calls**, every full-suite named win, profile derivation, native evidence, complete-build medians and ratios, all 486 saved deployment streams and four source rebuilds. Confidence support has not been reassessed and is not an additional selection gate.

Next, combine the same compiler mechanism with the smaller original-source recipe using its actual VCode identity or fresh capture. Then census transparent argument-forwarding blocks and compare remaining Mech/C matcher semantic work. The normal capture has 2,153 VCode blocks, 435 distinct label offsets and 202 machine basic blocks; this identifies construction/liveness work to investigate, not a promised speedup or proof that every block can be removed. [Broader plan](BROADER-PLAN.md). **64 C wins is next; 81 remains the goal.**
