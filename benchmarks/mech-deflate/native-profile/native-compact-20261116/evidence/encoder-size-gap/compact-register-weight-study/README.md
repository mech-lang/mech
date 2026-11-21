# Compact source with measured register spill weights

**Selected recipe: `m1-profiled-register-sqrt`, 63,612 B, preserving every named 81 Roc / 63 libdeflate win in both complete 81-case, 96-block orders.** It saves 192 B from the preceding 63,804 B default. The linear combination is also fully qualified, at 63,628 B. No 64th C win is established; 81 C wins remains the goal.

The original **2,719-line Mech source and canonical bytecode are unchanged**. This combines the generic measured register spill weights with the previously retained carried-low-cost phi proof. It adds no LLVM dependency or shipping byte patch. The register profiles must match the entire lowered VCode, including register types, operand constraints, clobbers and every block weight. Old recipes remain selectable.

## Full runtime qualification

Both candidates are compared with the preceding selected original-source build and pinned Roc/libdeflate 1.26 level 12. Each case uses all 24 actor permutations four times in each order. Calls include fresh context/heap setup through cleanup. Every previous named win is checked separately; confidence has not been reassessed and is not an additional selection gate.

The following are candidate/control time ratios, with values below 1 better. The two candidate campaigns are separate matched comparisons.

| Input | Linear forward / reverse | Square root forward / reverse |
| --- | ---: | ---: |
| 320-overlap | 0.983924× / 0.980810× | 0.985110× / 0.984316× |
| prefix-natural-1024 | 0.983826× / 1.002926× | 0.992287× / 0.989151× |
| prefix-natural-4096 | 0.998119× / 1.007575× | 0.990711× / 0.994742× |
| prefix-natural-8192 | 1.017405× / 1.014952× | 0.999420× / 0.995371× |
| prefix-random-4096 | 0.936722× / 0.944720× | 0.939601× / 0.961646× |
| prefix-random-8192 | 0.941651× / 0.947374× | 0.935020× / 0.922871× |
| prefix-random-16384 | 0.918515× / 0.924021× | 0.921084× / 0.937316× |
| prefix-random-1048576 | 0.915641× / 0.914518× | 0.925947× / 0.927928× |
| 16384-distribution-change | 0.979527× / 0.979843× | 0.967694× / 0.964550× |

Across all 81 inputs, square root weighting uses **3.14% / 3.07% less time by geometric mean**. It improves random 1 MiB about 7.2–7.4% and distribution change 3.2–3.5% in these full orders. Linear weighting saves about 8.5% on random 1 MiB in both focused panel orders. Small slowdowns remain included when their named peer wins survive.

Both full orders retain **81/81 Roc, 63/81 C, zero required names missing**. Square root natural 4 KiB is **0.972895× / 0.984128× C**; random 1 MiB remains **1.187866× / 1.188604× C**. The largest remaining C gaps are random 4–16 KiB; this is useful progress across a family, rather than an extra near-parity scoreboard point.

Four combinations (with/without carried phi facts, linear/square-root weights) also have complete 21-case, 24-block forward/reverse discovery campaigns. All four preserve every observed panel win. The two carried combinations alone have full 81-case qualification. Every sample, including timer-resolution zeros, is preserved.

## Native size and complete builds

| Encoder | Native text | × Roc text | × C text | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Prior selected | 63,804 B | 1.106× | 1.721× | 276,072 B |
| Weights alone, linear | 63,628 B | 1.103× | 1.716× | 276,072 B |
| Weights alone, square root | 63,612 B | 1.103× | 1.716× | 276,072 B |
| Carried + linear | 63,628 B | 1.103× | 1.716× | 276,072 B |
| Selected carried + square root | 63,612 B | 1.103× | 1.716× | 276,072 B |
| Roc | 57,696 B | 1.000× | 1.556× | 191,912 B |
| libdeflate | 37,080 B | 0.643× | 1.000× | 155,752 B |

All compact candidates measure **4,736 B** actual peak stack, under the 5,280 B cap. File alignment leaves the stripped executable unchanged. Native output, checked errors, resource observations, aliasing and cleanup retain the original source contracts.

| Source through final executable link, Apple M1 | Median | Paired × Roc | Paired × C |
| --- | ---: | ---: | ---: |
| Prior selected | 2.230153 s | 0.745564× | 4.582912× |
| Weights alone, linear | 2.226440 s | 0.745530× | 4.569207× |
| Weights alone, square root | 2.233239 s | 0.746731× | 4.559446× |
| Carried + linear | 2.227917 s | 0.744578× | 4.564780× |
| Selected carried + square root | 2.236726 s | 0.746096× | 4.591963× |
| Roc | 2.988907 s | — | — |
| libdeflate | 0.490598 s | — | — |

Fourteen balanced serial fresh-process orders per actor include final linking, with compilers and fixed support equally prebuilt. Every object, full metadata and bytecode is checked in every trial; strip and compiler bootstrap are separate. **No compile-time gain is established.** All seven actually timed stripped images pass all 81 inputs: **567 executions**, with saved streams independently inflated by the portable verifier.

Selected phase medians: parsing **703.786 ms**, semantic frontend **217.863 ms**, artifact preparation **6.821 ms**, encoding **2.548 ms**, native compilation **1234.452 ms**, final link **35.693 ms**. Catalog work overlaps parsing; separate medians do not add.

Offline profile training is separate from codec-build timing. Complete commands, environments, every phase/trial, actual artifacts and compiler identity are archived.

## Source reproduction

From the repository root, the main source build path now defaults to the selected recipe on Apple ARM64:

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/mech-compact
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/mech-linear --variant m1-profiled-register-linear
```

From this evidence directory, portable source/profile replay also supports each combination:

```sh
python3 build_compiler.py /tmp/mech-register-compiler
python3 reproduce.py /tmp/mech-register-compiler/release/compile_profile /tmp/mech-sqrt --role phis-sqrt
python3 reproduce.py /tmp/mech-register-compiler/release/compile_profile /tmp/mech-sqrt-checked --role phis-sqrt --checked
python3 verify.py
```

All **eight** public normal/checked replays reproduce object, complete metadata and bytecode exactly. Both named recipes also pass the actual main `rebuild.py` path with the current source-built compiler. The recipes contain source and profile inputs; no prebuilt encoder is used. Compiler implementation and independently runnable profile derivation remain in the [measured register study](../../measured-register-weight-study/README.md). The compiled feature source is committed at `5b3a4af26`; its measured snapshot differs only in invalid-profile error-message formatting.

The old profiles are reused only after the compiler accepts the complete virtual-code identity and an explicit proof finds identical complete raw matcher bytes, extent and empty relocation lists. The verifier checks all four transferred atoms against both actual objects; source/profile hashes and normal/checked final artifacts are also checked.

Each candidate passes 162 exact output/inflation/alias/cleanup observations, six checked errors and twelve exact failure/recovery pairs. Resource observations and error source origins match the original codec without source-specific exceptions. The same compiler feature has **79 optimized scalar tests** passing; no Rust source changed in this combination. Bool/U64/I64/F64 direct native execution remains maintained.

The portable verifier checks **5,366,016 saved runtime calls**, every named win, every archive member, eight public rebuilds, both main recipes, four exact matcher transfers, fourteen balanced build orders and 567 actual stripped executable streams. All 704,012 zero-duration individual samples remain included.

## Broader follow-up

The actual optimized matcher contains 2,124 prepared blocks: 1,358 single jumps, including 1,232 with no parameters and 126 with parameters. The existing argument-free forward-jump pass reduces that to 1,147 blocks and 381 single jumps, with 126 parameterized jumps left. Its first untimed emission saves 80 B; its unit test and IR verification pass. This is a construction/liveness opportunity, not a runtime or compile-time claim. Native/runtime qualification and fresh profiles are separate.

The next experiment combines that generic cleanup with fresh register and machine-layout profiles. It tests random, natural, periodic, repeated and distribution-changing inputs rather than selecting only a close loss. Further work compares actual Mech/C matcher search/skip/tree work and descriptor/state traffic. Rebasing is already SIMD and constants already use readonly object data, so neither is presented as an unimplemented optimization. Keep partial gains and repair any named loss. **64 C wins is next; 81 C wins remains the goal.**
