# Generic forward-jump cleanup with fresh profiles

**Selected: `m1-forwarded-register-linear`, 63,516 B, preserving every named 81 Roc / 63 libdeflate win in both full 81-case, 96-block orders.** It saves 96 B from the preceding compact square-root selection, reduces measured stack 4,736 → 4,720 B, and takes 1.25–1.33% less random-1-MiB time in the full orders. No 64th C win is established. The original **2,719-line Mech source and canonical bytecode are unchanged**. No LLVM dependency or shipping byte patch is added.

The existing generic pass bypasses forward argument-free jumps with the same coldness, then removes unreachable blocks and verifies the IR. It preserves branch parameters, debug/value-label constraints and checked semantics. On the actual matcher it reduces 2,124 prepared blocks to 1,147. New register spill weights are derived from this lowered code, and new matcher/Huffman machine layouts are trained on their actual emitted bodies. The old register profile is not applied to changed virtual code.

## Runtime comparisons

Candidate/control time ratios below 1 indicate less time. The control is the preceding fully qualified compact square-root recipe; peers are pinned Roc and libdeflate 1.26 level 12. Calls include fresh heap/context setup through cleanup. Full qualification uses all 24 actor permutations four times per case in each order, checking every prior named win. Confidence has not been reassessed and is separate from selection.

| Input | Linear / control, full forward / reverse | Linear / C, full forward / reverse |
| --- | ---: | ---: |
| 320-overlap | 1.000816× / 0.997453× | 0.960959× / 0.957076× |
| prefix-natural-1024 | 0.994792× / 0.993577× | 1.231026× / 1.228823× |
| prefix-natural-4096 | 0.999438× / 0.996157× | 0.981748× / 0.967052× |
| prefix-natural-8192 | 1.004183× / 1.003233× | 0.948274× / 0.944690× |
| prefix-random-4096 | 0.976979× / 0.994094× | 1.471518× / 1.486532× |
| prefix-random-16384 | 0.989802× / 0.987047× | 1.380773× / 1.365682× |
| prefix-random-1048576 | 0.987505× / 0.986724× | 1.172281× / 1.174017× |
| 16384-distribution-change | 0.992058× / 0.992319× | 1.232375× / 1.223610× |

Both orders retain **81 Roc / 63 C, with zero required names missing**. The full-suite geometric mean is **0.997291× / 0.996089× control time**. Small slowdowns remain included when named wins survive. These are additional gains over the preceding register/carry combination, not a chained estimate against older cohorts.

Both linear and square-root models also have complete 21-case, 24-block forward/reverse campaigns. Every observed 21 Roc / 9 C panel win survives. The smaller **63,468 B** square-root alternative improves random 1 MiB about 1.4–1.6%, but slows distribution change **3.0–3.4%** and has no full81 qualification. It remains an explicit named recipe for repair and combinations.

## Actual final matcher work

Fresh diagnostic clones count the actual selected control and both final candidates on the same nine inputs. Random 1 MiB makes 35 calls to each matcher. These counts exclude checked controller work and do not forecast elapsed time.

| Matcher | Instructions | Branches | Stack loads | Stack stores |
| --- | ---: | ---: | ---: | ---: |
| Prior compact square root | 198,169,162 | 31,909,589 | 20,228,175 | 1,697,137 |
| Selected forwarded linear | 195,425,468 | 31,907,630 | 18,128,229 | 1,696,938 |
| Forwarded square root | 194,508,233 | 31,909,589 | 17,447,641 | 2,342,593 |

Linear removes about **1.38% of instructions and 10.38% of stack loads** on this input, with almost unchanged branches/stores. The square-root model executes fewer instructions but has more stores and a distribution slowdown; instruction counts alone do not select the build.

## Size and complete builds

| Encoder | Native text | × Roc text | × C text | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Prior compact square root | 63,612 B | 1.103× | 1.716× | 276,072 B |
| Selected forwarded linear | 63,516 B | 1.101× | 1.713× | 276,072 B |
| Forwarded square root | 63,468 B | 1.100× | 1.712× | 276,072 B |
| Roc | 57,696 B | 1.000× | 1.556× | 191,912 B |
| libdeflate | 37,080 B | 0.643× | 1.000× | 155,752 B |

Both new candidates measure **4,720 B** peak stack, below the unchanged 5,280 B cap. File alignment leaves the stripped executable unchanged.

| Source through final executable link, Apple M1 | Median | Paired × Roc | Paired × C |
| --- | ---: | ---: | ---: |
| Prior compact square root | 2.230269 s | 0.747058× | 4.607362× |
| Selected forwarded linear | 2.229091 s | 0.745744× | 4.595310× |
| Forwarded square root | 2.230658 s | 0.746313× | 4.594039× |
| Roc | 2.988087 s | — | — |
| libdeflate | 0.486367 s | — | — |

Ten balanced serial fresh-process orders include final linking, with compilers and fixed support equally prebuilt. Every object, full metadata, bytecode and stripped executable identity is checked in every trial. Strip, offline profile training and compiler bootstrap are separate. **No compile-time gain is established**, despite fewer prepared matcher blocks. All five actual timed stripped images pass all 81 inputs: **405 executions**, with saved output streams independently inflated by the verifier.

Selected phase medians: parsing **703.702 ms**, semantic frontend **218.423 ms**, native compilation **1233.689 ms**, final link **35.384 ms**. Separate medians do not add; catalog work overlaps parsing.

## Reproduction and verification

The main build path defaults to the selected recipe on Apple ARM64:

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/mech-forwarded
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/mech-forwarded-sqrt --variant m1-forwarded-register-sqrt
```

From this evidence directory:

```sh
python3 build_compiler.py /tmp/mech-forwarded-compiler
python3 reproduce.py /tmp/mech-forwarded-compiler/release/compile_profile /tmp/mech-forwarded-linear --role linear
python3 reproduce.py /tmp/mech-forwarded-compiler/release/compile_profile /tmp/mech-forwarded-linear-checked --role linear --checked
python3 verify.py
```

Four public normal/checked source replays and both actual main named recipes reproduce all three artifacts exactly. Source and profiles are inputs; no prebuilt encoder is used. Each candidate passes 162 exact output/inflation/alias/resource/cleanup observations, six checked errors and twelve exact failure/recovery pairs. Resource observations and named error origins retain the original source contracts without exceptions. The compiler feature has **79 optimized scalar tests** passing; no Rust source changes in this combination.

All fresh counter files bind the actual source object, canonical object, identical linked worker and every translated block record. The portable generator derives both register profiles from the archived pre-allocation VCode and nine-case block counts. Each case contributes an equal normalized share, with maximum4096 and floor1; folded labels share actual machine-block observations. The first verifier run caught a directory-glob bug; the corrected verifier and original failure log are retained.

Portable verification checks **2,683,008 saved runtime calls**, every named full-suite win, all archive members, **7 fresh source/counter bindings**, both register-profile derivations, machine-profile provenance, four public rebuilds, both main named recipes, ten build orders and all405 deployment streams. All 349,351 zero-duration individual samples remain included.

The prior [compact register combination](../compact-register-weight-study/README.md), [generic register feature and generator](../../measured-register-weight-study/README.md), and every adverse result remain available. The next broader target is the checked entry, which still occupies **46,572 B**. Revisit the retained wider input-packet qualification with the current compiler advances; compare matcher search/skip/tree work with C. **64 C wins is next; 81 C wins remains the goal.**
