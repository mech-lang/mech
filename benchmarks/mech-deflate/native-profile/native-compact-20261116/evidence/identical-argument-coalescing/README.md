# Dominating identical SSA arguments and elapsed-time parity

**Selected: `m1-identical-shared-vector-linear`, 64,408 B, every named 81/81 Roc and 63/81 libdeflate win held in both full 81-case, 96-block orders.** Dominance-proved SSA argument coalescing with shared-Boolean lowering takes **23.506–23.508 ms versus 20.349–20.353 ms C** on random 1 MiB, paired **1.154592–1.155961× C**. About **3.16 ms, or 13.4–13.5% less Mech time**, is still needed for parity. No 64th C win is established. The original 2,719-line Mech source and canonical bytecode are unchanged.

| Qualified arm | Text B | Random 1 MiB Mech ms, forward /reverse | C ms, forward /reverse | Paired Mech ÷ C |
| --- | ---: | ---: | ---: | ---: |
| New default, shared-vector-linear | 64,408 | 23.506125 /23.507917 | 20.353292 /20.348604 | 1.154592 /1.155961× |
| New vector-linear alternative | 64,364 | 23.591333 /23.597542 | 20.348937 /20.350479 | 1.160017 /1.160308× |

The default saves **4 B** versus the preceding64,412-B selection. The alternative saves **48 B** and also retains all 81 Roc/63 Cwins in both complete orders. Default full-suite geometric means are **0.999440× /0.999564×** control; alternative **0.998341× /0.998738×**. These small broad differences are not an established large runtime gain. Small slower cases remain included; point wins and confidence are separate. The previously qualified **63,372-B compact recipe remains available**.

Matched complete builds take **2.199275 s new default /2.192953 s alternative /2.204447 s prior control /2.986321 s Roc /0.481612 s C**. Twelve balanced serial fresh-process orders include final executable linking with compilers/support equally prebuilt; no compile-time gain is claimed. Default paired time is **0.736279× Roc /4.563207× C**. Parse/frontend/native medians are **0.703250 /0.218701 /1.204642 s**. The stripped Mech image remains **276,072 B**; peak stack is **4,832 B**, below 5,280 B. The alternative has 4,736 B peak stack. Text is **1.116× Roc /1.737× C**; stripped image **1.439× Roc /1.772× C**.

The actual main default and explicit alternative reproduce all three artifacts exactly. **34 public normal/checked source replays**, **79 optimized scalar tests**, **7,360 new focused native executions**, all strict output/resource/error/recovery/stack checks and **486 actual timed stripped-image executions** pass. Portable verification checks **10,031,040 saved runtime calls**, including 1,450,135 retained zero samples, **144 actual training counter cases**, both profile derivations and all 12 complete-build orders. Compiler source is rebasable at `10a4b06f8`.

[All17 retained source recipes, timings and tradeoffs](README.md) · [All81 elapsed times and remaining parity gaps for both new arms](PARITY.md).

The next broad investigation is relational unsigned bounds: the existing selected-edge range code narrows variable comparisons only when one operand is constant. A true `x<y` can prove `x<MAX` even with an unknown upper bound, which may remove redundant checked increments. Count the actual remaining hot guards first, preserve width/signedness and boundary errors, and combine proved facts with the new SSA normalization. Constant private input-packet words, shared comparison ANDs, immutable byte-load reuse and the retained compact recipe are also recorded for follow-up. **64 C wins is next; 1× libdeflate across all 81 remains the goal.**

## Compiler proof and actual native work

`MECH_COMPACT_COALESCE_IDENTICAL_ARGUMENTS=root|vector|all` aliases a block formal only when every incoming edge supplies the same dominating SSA value, ignoring only a self recurrence. It checks types, attached SSA and dominance; excludes the function entry, unreachable blocks, egraph unions and debug/fallthrough metadata. Different actuals on two edges to the same destination remain distinct. No arithmetic, load, store or other effect is moved or duplicated. Native Bool/U64/I64/F64 original/decoded paths and private witness construction remain maintained. The generic compiler option is off by default; the selected M1 source recipe enables the qualified vector scope with shared-Boolean lowering. No new LLVM dependency, codec rewrite, generic operation dispatch or shipping native-byte patch is added.

The actual matcher proves **281** redundant formals; the optional caller scope proves **753**, Huffman 3 and the last vector 1. Caller-only cleanup grows text160B but lowers peak stack128B and leaves the three selected private bodies exact. It remains retained. New matcher register weights come from actual nine-case native counts, including all static heads and unvisited zeros. The old machine graph does not match the new matcher: its layout uses a rank guide through identical source VCode block IDs and successors, labeled as a heuristic. Huffman changes 26→27 machine blocks; all edge flows are uniquely determined by the actual block-count constraints on every training case, and its new layout is independently derived.

For random 1 MiB, selected matcher counts are **183,596,351 instructions /31,901,634 branches /11,213,235 stack loads /1,296,017 stack stores**, with35calls. The preceding Boolean-linear body has187,148,785instructions and12,265,616stackloads. These diagnostic counts describe bound actual bodies, not hardware cycles. Huffman executes662,043instructions with31calls. Elapsed times remain the primary performance evidence.

The full suite establishes every named81 Roc/63 Cpointwin for both new arms. Source parity ratios are geometric means of paired block observations; milliseconds are per-call medians. Displayed medians need not divide to the paired ratio. Confidence is separate. The full GPU suite is not claimed: the new two tests and 79 scalar tests pass in the rebuilt optimized library; a standalone build of the exact proof/test sources also passes7,360native calls, covering changing outer values, integer/float widths and bit patterns, distinct branch actuals, observable effects, early failure and retry. An initial crate-visibility wiring error and the missing old reference VCode capture are retained with their repairs; neither was a semantic failure.

## All retained trials

Every arm passes162exact output/inflation/alias/resource/cleanup observations, six named checked errors, twelve injected-failure recoveries and the 5,280-byte stack cap. Discovery panels contain21cases and24blocks in each order. Full qualification is attached only to the two completed81-case, 96-block campaigns. The eight width-revisit arms all retain the panel's named wins, but none beats the retained target timing; fewer instructions alone do not establish a win. All slowdowns remain included.

| Role | Text B | Random 1 MiB Mech ms, forward /reverse | Paired Mech ÷ C | Full81 qualification |
| --- | ---: | ---: | ---: | --- |
| shared-all-sqrt | 64,648 | 23.581291 /23.575395 | 1.161276 /1.158505× | not run |
| root | 64,572 | 23.682271 /23.693229 | 1.166439 /1.167097× | not run |
| shared-all-linear | 64,616 | 23.523292 /23.504042 | 1.156429 /1.156864× | not run |
| vector-sqrt | 64,380 | 23.600062 /23.601084 | 1.162903 /1.161546× | not run |
| all-linear | 64,492 | 23.607916 /23.593583 | 1.162105 /1.160907× | not run |
| shared-vector-sqrt | 64,440 | 23.592021 /23.562813 | 1.162073 /1.159514× | not run |
| vector-linear | 64,364 | 23.601875 /23.596229 | 1.161385 /1.161090× | 81 Roc/63 C,both orders |
| shared-vector-linear | 64,408 | 23.502584 /23.504000 | 1.157185 /1.158210× | 81 Roc/63 C,both orders |
| all-sqrt | 64,508 | 23.585500 /23.598104 | 1.161766 /1.160267× | not run |
| revisit-shift-sqrt | 64,412 | 24.328083 /24.339521 | 1.197918 /1.199300× | not run |
| revisit-shared-shift-sqrt | 64,648 | 24.154312 /24.116312 | 1.190301 /1.188194× | not run |
| revisit-narrow-linear | 64,428 | 23.604354 /23.595688 | 1.162295 /1.160793× | not run |
| revisit-shared-narrow-linear | 64,696 | 23.552188 /23.541583 | 1.159191 /1.159785× | not run |
| revisit-shared-narrow-sqrt | 64,680 | 23.607980 /23.584250 | 1.161676 /1.159439× | not run |
| revisit-narrow-sqrt | 64,412 | 23.697188 /23.686771 | 1.167043 /1.165797× | not run |
| revisit-shared-shift-linear | 64,664 | 24.274396 /24.249583 | 1.191196 /1.194703× | not run |
| revisit-shift-linear | 64,396 | 24.387250 /24.361375 | 1.200139 /1.200065× | not run |

The complete-build cohort checks every generated object, full metadata and bytecode hash. All compilers and fixed support are prebuilt equally. No test, counter, profile, archive or Git work runs during primary runtime or source-through-final-link clocks; strip and compiler construction are separate. Compiler bootstrap227.687 s overlapped independent test construction381.554 s and is not an isolated bootstrap comparison. All76peer source inputs remain pinned. The34 public source replays use the current source compiler, including exact replays of the earlier width arms; their original runtime/count receipts retain the original compiler identity. Actual main source paths reproduce both new recipes, including the default.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/identical-argument-coalescing/verify.py
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/mech-identical
```

Use `--variant m1-identical-vector-linear` for the48-byte smaller alternative, or `--variant m1-argument-vector-neighbor` for the earlier 63,372-byte compact recipe. To rebuild any of the17 source arms directly:

```sh
study=benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/identical-argument-coalescing
python3 "$study/build_compiler.py" /tmp/mech-identical-compiler
python3 "$study/reproduce.py" /tmp/mech-identical-compiler/release/compile_profile /tmp/mech-identical-native --role shared-vector-linear
```

Add `--checked` for checked register-pair emission; output directories must be fresh. The matcher rank guide is not mislabeled as an exact native graph or actual edge-frequency sample. Original zero-duration runtime samples and every adverse panel are retained. [Raw summary](summary.json) · [All81 timings](PARITY.md) · [Source recipes](recipes.json) · [Verification](verification.json) · [Archive manifest](manifest.json) · [Native/build evidence](native-and-builds.tar.gz) · [Runtime evidence](runtime.tar.gz).
