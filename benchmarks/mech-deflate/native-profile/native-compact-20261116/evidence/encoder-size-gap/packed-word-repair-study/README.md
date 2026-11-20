# Packed-word repairs and current 81/63 selection

**Working selection: `m1-narrow-packed-words`, 63,852 B, 81/81 Roc and 63/81 libdeflate point wins in both complete 96-block orders.** Every required named point win is retained. Natural 4 KiB is **0.984194× / 0.981922× C**, with both 90% intervals below parity. Reverse `320-overlap` has a confidence interval crossing parity while retaining its point win; confidence is reported separately under the user's working-selection rule.

The selected build takes **2.224476 s** through final executable linking on Apple M1, paired **0.740614× Roc / 4.531483× C** in 18 balanced trials. Roc takes **3.005122 s**, C **0.490298 s**. The previous selected build in this same cohort takes **2.225267 s**; no material compile-time gain is established. Encoder code is **1.107× Roc / 1.722× C**; stripped executable **276,072 B**; peak stack **4,736 B**. All compilers and fixed support are prebuilt equally; final executable linking is included, compiler bootstrap and strip are separate.

The compiler removes proved zero high fields from narrow costs while keeping full packed words for stores and item uses. This saves **16 B** and **5,977,851 scorer instructions (8.6%) on random 1 MiB**, but the random runtime is essentially unchanged. All source and adverse observations are preserved. [Twelve recipes, all 24 discovery orders, both full 81-case/96-block outcomes,18 complete-build trials, checks and portable source reproduction](result.json).

On Apple AArch64, `rebuild.py` defaults to this recipe; select it explicitly with `--variant m1-narrow-packed-words`. The actual named recipe reproduces all three artifacts exactly in normal and checked emission. Twenty-one optimized tests pass in four configurations, including 101,376 new native differential calls per configuration; all 53 older recipes reproduce exactly. Original 2,719-line Mech source and canonical bytecode remain unchanged. No LLVM dependency is added. The existing missing archived allocator still blocks the repository-wide unsafe audit.

The goal remains **1× libdeflate across all 81 cases**. Every 81 Roc and 63 named C point win must survive future changes. Smaller and paired candidates remain available for repair; the previous eight-entry recipe remains explicit as `m1-packed-eight-entry`.


## Full81-case runtime decision

| Order | Roc point wins | C point wins | Required named wins lost | Natural4KiB / C |90% interval versus C |
| --- | ---: | ---: | ---: | ---: | ---: |
| forward96 |81 |63 |0 |0.984194 |-2.105% to-1.070% |
| reverse96 |81 |63 |0 |0.981922 |-2.333% to-1.240% |

The point rule passes in both directions with no raw correctness, cleanup, identity, inventory or host-validation finding. The confidence report is retained unchanged. It is false only because the reverse320-overlap named comparison is not precision-supported; the point win remains and does not veto the user-selected working build. No extra confidence gate is applied.

## Every discovery observation

All rows are21 cases and24 balanced blocks in each order, compared against the exact previous81/63 worker. Ratios across different cohorts do not establish causal differences; paired comparisons to that cohort’s Mech control do. These panels do not infer full81-case scores.

| Role | Text bytes | Natural4KiB / C F / R | Natural8KiB / C F / R |
| --- | ---: | ---: | ---: |
| phase-preserved-unpaired-probe-profiled |63,868 |0.984982 / 1.009976 |0.960829 / 0.966144 |
| phase-preserved-legacy-probe-profiled |64,108 |1.018619 / 1.016502 |1.001039 / 1.009490 |
| phase-preserved-loop-probe-profiled |64,092 |1.012348 / 1.021247 |1.011072 / 1.012413 |
| phase-preserved-once-probe-profiled |64,060 |1.028918 / 1.020991 |1.007877 / 1.014381 |
| packet-eight-probe-profiled |63,804 |0.998207 / 0.995460 |0.960860 / 0.975423 |
| packet-once-probe-profiled |64,012 |1.023611 / 1.016782 |0.992801 / 1.001858 |
| packet-reg-eight-probe-profiled |63,804 |0.986285 / 0.986644 |0.961842 / 0.970196 |
| packet-reg-once-probe-profiled |64,012 |1.018443 / 1.022200 |0.996019 / 1.004813 |
| narrow-eight-probe-profiled |63,852 |0.974993 / 0.996344 |0.948756 / 0.947979 |
| narrow-packet-eight-probe-profiled |63,804 |0.979726 / 1.003934 |0.944248 / 0.953728 |
| narrow-once-probe-profiled |64,060 |1.006998 / 1.009913 |0.980366 / 0.990993 |
| narrow-packet-once-probe-profiled |64,012 |1.004559 / 1.012465 |0.983286 / 0.997476 |

The sequential/one-visit scalar repair now regains the existing natural8KiB C win in both narrow-folding discovery orders, while natural4KiB still needs repair. The packet combination preserves64B of code saving and remains available for a separate full96 run. Actual phase register preference renames registers without reducing aggregate work; every adverse outcome is retained.

## Compile time and artifact sizes

| Actor | Encoder text | Source through final executable link | Stripped executable |
| --- | ---: | ---: | ---: |
| Selected Mech |63,852 B |2.224476 s |276,072 B |
| Roc |57,696 B |3.005122 s |191,912 B |
| libdeflate |37,080 B |0.490298 s |155,752 B |

All18 trials for nine actors completed before a reporting-label KeyError: the helper expected mech-control-reference, while this cohort names its control mech-selected63. The archived report is reconstructed from the original saved actor result.json files. No clocks or observations were rerun or changed; original error, source driver and reconstruction script remain included.

## Reproduce and verify

Compiler source basis for the selected recipe: `9b1460933a88837bfd688344c312081b441666cd`. The compiler is prebuilt for source replay; source and flags for all12 roles are pinned independently. The compiler-source build is separate from the measured codec build.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/mech-narrow --variant m1-narrow-packed-words
python3 reproduce.py /path/to/compile_profile /tmp/mech-narrow-replay
python3 verify.py
```

The verifier replays all source/artifact bindings, compiler/test source pins,45/53 old recipes,12 portable normal/checked replays, worker and code identities,162 output/inflation/alias cases per worker, six checked failures,12 failure/recovery cases,4,736B stack, raw counters, all 24 discovery orders, full81-case point/confidence decisions and18 build trials. Actual named-recipe flag assembly is verified separately in working-selection-replay.json. No full-GPU-suite or repository-unsafe-audit success is claimed.
