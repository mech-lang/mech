# Shared comparison booleans: elapsed time and parity

**Two additional source-built variants retain every named 81/81 Roc and 63/81 libdeflate win in both full 81-case, 96-block orders. No 64th C win is established.** The faster random-1-MiB arm takes **23.53–23.54 ms versus 20.35 ms C**, paired **1.156–1.157× C**. It still needs about **3.19 ms, or 13.5% less Mech time**, to tie. Both retain the original 2,719-line Mech source and canonical bytecode. The current `m1-argument-all-sqrt` default and 63,372-B compact alternative remain maintained; these new arms are reproducible alternatives with their tradeoffs recorded below.

Milliseconds are per-call medians. Runtime ratios are geometric means of paired block observations; dividing aggregate medians need not reproduce those ratios. Confidence is separate from the named point-win requirement.

| Test and arm | Mech ms, forward /reverse | C ms, forward /reverse | Paired Mech ÷ C | Mech time reduction to parity |
| --- | ---: | ---: | ---: | ---: |
| Random 1 MiB, linear | 23.541563 /23.531354 | 20.354751 /20.346833 | 1.156161 /1.156749× | 13.51% /13.55% |
| Random 1 MiB, square-root | 23.590687 /23.584479 | 20.353000 /20.346312 | 1.159830 /1.160628× | 13.78% /13.84% |
| Distribution change 16 KiB, linear | 0.602646 /0.603646 | 0.482750 /0.483375 | 1.245134 /1.248104× | 19.69% /19.88% |
| Distribution change 16 KiB, square-root | 0.579646 /0.585604 | 0.483000 /0.482791 | 1.195978 /1.211382× | 16.39% /17.45% |
| Periodic 1 MiB, linear | 12.002042 /12.005416 | 18.459021 /18.407229 | 0.649718 /0.652990× | Already wins |

[All 81 cases for both arms, with actual milliseconds and parity gaps](PARITY.md) · [Machine-readable timings](parity.json).

The square-root arm takes **0.14–0.17% less full-suite time** by geometric mean than its exact current qualified control. Random 1 MiB takes **0.43–0.44% less time**. The linear arm takes **0.62–0.71% less random-1-MiB time**, but costs **3.83–4.43%** on the distribution-change case. Its full-suite geometric mean is **1.001378× /0.998416×** control, so there is no consistent broad improvement. Both arms stay available rather than discarding the targeted gain or hiding the slower cases. The default remains the smaller, previously qualified 64,412-B recipe.

| Same Apple M1 cohort | Encoder text B | Source through final executable link s | Stripped executable B | Peak native stack B |
| --- | ---: | ---: | ---: | ---: |
| Current qualified control | 64,412 | 2.196295 | 276,072 | 4,736 |
| Shared Boolean, square-root | 64,680 | 2.201176 | 276,072 | 4,640 |
| Shared Boolean, linear | 64,696 | 2.200958 | 276,072 | 4,640 |
| Roc | 57,696 | 2.984033 | 191,912 | — |
| libdeflate 1.26, level 12 | 37,080 | 0.481201 | 155,752 | — |

Twelve balanced serial orders include fresh-process source parsing, code generation and final executable linking. Compilers and fixed support are equally prebuilt; tests, counters, profiles, archive work, Git work and strip are outside those clocks. Paired complete-build time is **0.737435× Roc /4.573835× C** for square-root and **0.736903× Roc /4.575391× C** for linear. Relative to the preceding compiler and current recipe, paired ratios are **1.002530× /1.001444×**; **no compile-time gain is established**. Square-root parse/frontend/native medians are **0.704133 /0.218003 /1.206804 s**. Linear medians are **0.704895 /0.218064 /1.205362 s**. Native text grows 268/284 B against the current selection; measured stack drops 96 B. The compiler bootstrap took about 4m49s while test compilation ran in a separate target; it is not an isolated bootstrap comparison. A later 0.5s cached rebuild is not a clean bootstrap result.

## Compiler change and actual work

`MECH_ARM64_CCMP_SHARED_BOOLEAN=1` enables generic ARM64 lowering of an OR of two canonical integer comparison results. It uses CMP/CCMP followed by one CSET, preserving the shared 0/1 result for branches and later numeric uses. Comparisons can originate in dominating earlier blocks; later operands are materialized before flags are set. Internal comparison/cast nodes must have a single use. Raw integer ORs, shared leaves, phis and floating-point comparisons are refused. The option is off by default. No runtime dispatch, interpretation, per-operation allocation, new LLVM dependency or shipping native-byte patch is added.

In actual random-1-MiB matcher counts, the square-root arm reduces instructions **186,103,856 →184,006,766**, with 35 matcher calls. Branches stay **31,901,634**, stack loads **9,117,617 →9,121,597**, and stack stores stay **2,344,558**. These diagnostic counts describe the actual uninstrumented candidate bodies through a bound counting clone. They are not hardware cycles or a substitute for elapsed time. Initial unprofiled lowering saves instructions but adds roughly 646,000 stack loads/stores; fresh register weights repair that effect.

Before adding another bounds pass, inspection confirmed that observation-counter sites 2000/2059 already use unconditional loads/stores. Remaining observation guards occur in ten-bin merge loops. The separate bounded-byte-histogram proof was already enabled. The next broader experiments should revisit proved 32-bit hash arithmetic with fresh profiles, and investigate copied loop state, repeated immutable input loads and the remaining high-word guard. Dynamic counter evidence identifies these costs; their correctness and runtime gains remain to be proved.

## Retained trials and validation

The first matched unprofiled 11-case screen improves geometric mean by 0.66–0.69%, while random 1 MiB stays effectively flat. It is not a replacement for the qualified selection. The first profiled linear layout guide misses the historical natural-4-KiB C win in reverse at **1.004127× C**. Reusing the matching linear layout repairs that named miss; both full orders then retain the win at **0.985869× /0.985386× C**. The original guide, adverse observations and repaired arm are all retained. Source-label layout guides are labeled as heuristics, not actual edge frequencies. Final per-mode rebases require exact ordered nonpadding transfer graphs; register profiles retain their expected-VCode guard and are derived from all static heads, including unvisited zeros.

Each of the five public arms passes 162 exact output/inflation/alias/resource/cleanup observations, six named checked errors, twelve injected-failure recoveries and the unchanged 5,280-B stack cap. Ten public normal/checked source replays reproduce object, full metadata and bytecode exactly. The current qualified control and option-disabled compiler reproduce all three preceding artifacts; the control worker is exact. All 486 executions of actual timed stripped images pass independent inflation, with raw streams saved.

**79 existing optimized scalar tests pass** with the new rule enabled, preserving efficient checked Bool/U64/I64/F64 original/decoded execution. The two current focused native tests pass **103,424 executions** covering all 100 integer-condition pairs at four width combinations, boundary values, cross-block definitions, an intervening store, a shared leaf and raw noncanonical words. They are compiled against the fresh compiler's pinned backend libraries. The first focused assertion counted the final assembly and an appended register-allocation dump twice; its failure and correction are retained. No math/semantic failure was observed. The entire GPU test suite is not claimed.

The portable verifier checks **4,674,624 saved runtime calls**, including **585,974 retained zero-duration samples**, 54 actual training counter cases, both register-profile derivations, all twelve complete-build orders and 486 saved stripped deployment streams. Earlier bounds-study counts and captures are archived separately. The first layout-proof path was overwritten by the later per-mode repair; its separate recomputation checks the saved first-wave profiles exactly and is labeled accordingly.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/shared-comparison-booleans/verify.py
```

Build and reproduce either full-qualified arm from this PR's compiler source:

```sh
study=benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/shared-comparison-booleans
python3 "$study/build_compiler.py" /tmp/mech-shared-boolean-compiler
python3 "$study/reproduce.py" /tmp/mech-shared-boolean-compiler/release/compile_profile /tmp/mech-shared-boolean-native --role shared-raw-sqrt
```

Use `--role shared-raw-linear-legacy-layout` for the faster random-1-MiB arm, or add `--checked` for checked register-pair emission. Output directories must be fresh. The original codec, all profiles and compiler implementation are in the PR; another agent can rebase on the source implementation at `6430f299a232eb2e58d86760f4d06c6c87d12916`.

[All source recipes](recipes.json) · [Raw results](summary.json) · [Verification](verification.json) · [Archive manifest](manifest.json) · [Native/build evidence](native-and-builds.tar.gz) · [Runtime evidence](runtime.tar.gz).
