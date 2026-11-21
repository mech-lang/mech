# Cleanup after SSA argument coalescing

**Selected: `m1-coalesced-compact-all-linear`, 63,048 B, every named 81/81 Roc and 63/81 libdeflate win preserved in both full 81-case, 96-block orders.** Repeating existing current-word and argument-transport proofs after SSA argument coalescing combines with the earlier compact recipe. The original **2,719-line Mech source and canonical bytecode are unchanged**.

| Random 1 MiB | Mech | Current Mech control | libdeflate | Paired Mech ÷ C | Mech time reduction to parity |
| --- | ---: | ---: | ---: | ---: | ---: |
| Forward | 23.419542 ms | 23.473645 ms | 20.398521 ms | 1.156924× | 13.56% |
| Reverse | 23.388208 ms | 23.431687 ms | 20.352874 ms | 1.152030× | 13.20% |

The broader target, random **4 KiB**, takes **108.438 /108.020 µs Mech**, versus **75.667 /76.042 µs libdeflate**. Paired Mech time is **1.440× /1.436× C**, leaving **30.57% /30.38%** to remove.

This saves **1,216 B** versus the prior 64,264-B selection and **324 B** versus the previous qualified 63,372-B compact recipe. Paired target time is **0.998227× /0.999471×** control; full-suite geometric-mean point estimates are **0.993651× /0.992991×**. The timing differences are small; no formal confidence claim is added. Small slower cases remain included. **No 64th C win is established; about 13.2–13.6% of Mech's time still needs to be removed on the closest remaining case.** Earlier qualified named recipes remain available.

Matched complete builds take **2.203537 s Mech /2.988196 s Roc /0.482477 s C**, paired **0.737236× Roc /4.560036× C**. Twelve balanced serial fresh-process orders include final executable linking, with compilers and fixed support equally prebuilt. The matched prior Mech median is **2.197920 s**; no compile-time improvement is established. Parse/frontend/native medians are **0.702318 /0.217598 /1.208162 s**. Bootstrap and strip are separate.

Encoder text is **1.093× Roc /1.700× C**. The stripped executable remains **276,072 B**, **1.439× Roc /1.773× C**. Measured peak stack is **4,736 B**, below the 5,280-B cap. The actual main default reproduces object, full metadata and bytecode exactly. **36 public normal/checked source replays**, **79 optimized scalar tests**, the current-word/SSA/increment tests, strict output/resource/error/recovery/stack checks and **486 actual timed stripped-executable checks** pass. Portable verification recomputes **4,836,096 saved runtime observations**, retaining 708,858 zero samples, **180 actual counter cases**, every profile derivation and all twelve complete-build orders. Compiler source is rebasable at **`c92e9e1a2`**. No new LLVM dependency, codec rewrite, generic operation dispatch or shipping native-byte patch is added.

[All 81 timings and parity gaps](PARITY.md) · [Raw runtime/build summary](summary.json).

`MECH_COMPACT_REFINE_COALESCED_ARGUMENTS=predicates|transport|widths|all` is optional and off by default in the compiler. It runs only in unobserved private vector bodies after identical-argument coalescing actually removes formals. The existing proofs and verifier remain authoritative: no new arithmetic assumption, permission, memory effect or generic operation dispatch is introduced. Current-word refinement removes six remaining matcher branches; transport cleanup removes 50 matcher jump blocks and two each in the Huffman and last vector. Width-only refinement removes nothing and reproduces the previous raw artifact exactly. The checked Bool/U64/I64/F64 original/decoded paths remain maintained.

The smaller combination reuses the earlier qualified `vector` scope for argument-jump forwarding. Its matcher, Huffman and scorer native bodies exactly match their noncompact counterparts, so register weights/layout inputs are reused only after native-body identity checks. Removing newly dead private input fields also reduces caller packet code. The previous compact recipe and all slower trials remain retained.

Fresh register profiles use nine actual mixed-input counter cases mapped through every static native head, including unvisited zeros. Layouts use exact block-flow constraints where determined, explicitly heuristic priorities for unresolved edges, and portable weighted-chain/segment refinement. The fast refinement computes the same exact adjacency-score changes using only affected edges; offline profile construction time is not Mech compilation time. The portable verifier independently recomputes all six layout profiles.

| Timed arm | Text B | Random 1 MiB Mech ms, forward /reverse | Paired Mech ÷ C | Full 81 qualification |
| --- | ---: | ---: | ---: | --- |
| predicates-sqrt | 64,072 | 23.492708 /23.473604 | 1.156762 /1.155445× | not run |
| all-linear | 64,040 | 23.392854 /23.381521 | 1.152036 /1.152774× | not run |
| compact-all-linear | 63,048 | 23.379959 /23.392625 | 1.151994 /1.152355× | 81 Roc/63 C, both orders |
| predicates-linear | 64,104 | 23.429500 /23.439562 | 1.154414 /1.152655× | not run |
| transport-linear | 64,232 | 23.425187 /23.430479 | 1.153137 /1.153816× | not run |
| all-sqrt | 64,024 | 23.418959 /23.416834 | 1.154956 /1.153926× | not run |
| transport-sqrt | 64,248 | 23.492645 /23.514166 | 1.157189 /1.158204× | not run |
| compact-predicates-linear | 63,048 | 23.438105 /23.420917 | 1.154788 /1.152964× | not run |

All eighteen source arms pass 162 exact output/inflation/alias/resource/cleanup observations, six named checked errors, twelve injected-failure recoveries and the 5,280-B stack cap. The six weighted preparatory arms and four raw arms have no elapsed-time claim. The eight final alternatives each have both 21-case, 24-block panel orders; only the selected compact combination has full 81-case qualification. The expanded native tests compare the composed passes against original execution across changing loop values, arbitrary integer/float bit patterns, opaque loads/calls, observable effects, failure prefixes, saved predicate snapshots and distinct same-destination branch arguments. Ten current-word tests, two SSA tests, four increment tests and 79 optimized scalar tests pass; test families overlap and the complete GPU suite is not claimed.

Native CPU sampling uses separate uninstrumented workers with exact output/alias checks, after primary clocks finish. Five-second 1-ms reports and all 200,800 diagnostic observations are saved for Mech and C on random 1 MiB and random 4 KiB. Shares are normalized to encoder-root samples; C inlines work differently, so no exact matched phase times are inferred. Mech is dominated by matching on 1 MiB, while histogram work grows substantially on 4 KiB. The hot high-word predicate selects cached input; it cannot be erased as an impossible overflow check.

Every source-through-final-link order checks generated object, full metadata, bytecode and stripped executable hashes, using equally prebuilt compilers/support. All 486 actual stripped deployment streams are saved and independently inflated. Rust compiler/test-library construction took 228.643 /384.226 s while constructed concurrently; these are not isolated bootstrap comparisons. No build, test, counter, profile, archive or Git work overlaps any primary runtime or source-through-link clock.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/coalesced-argument-refinement/verify.py
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/mech-coalesced
```

To build the compiler from this checkout and reproduce any retained arm:

```sh
study=benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/coalesced-argument-refinement
python3 "$study/build_compiler.py" /tmp/mech-coalesced-compiler
python3 "$study/reproduce.py" /tmp/mech-coalesced-compiler/release/compile_profile /tmp/mech-coalesced-native --role compact-all-linear
```

Add `--checked` for checked register-pair emission. Compiler implementation is committed at `c92e9e1a2`; the source and recipe are both integrated. The main default changes only on Darwin arm64. [Native/build/sampling receipts](native-and-builds.tar.gz) · [Runtime observations](runtime.tar.gz) · [Archive manifest](manifest.json) · [Portable verification](verification.json).

Separate native CPU sampling puts the matcher at **73.3%** and scorer at **13.7%** of encoder samples on random 1 MiB. On the larger relative loss at random 4 KiB, the matcher is **44.1%**, with **21.1%** in two histogram kernels. Code boundaries differ across compilers; these are phase guides, not matched phase timings. The next broad target tests a max-bin/remaining-span entry proof for the histogram's per-byte overflow check, keeping the checked loop on a proof miss and preserving partial writes and error origins. **64 C wins is next; 1× libdeflate across all 81 remains the goal.**
