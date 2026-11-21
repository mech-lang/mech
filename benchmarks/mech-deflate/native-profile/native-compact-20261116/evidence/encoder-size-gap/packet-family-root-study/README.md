# Retained packet proofs with generic checked-entry cleanup

**Selected: `m1-root-packet-combined`, 63,468 B, preserving every named 81 Roc / 63 libdeflate win in both full 81-case, 96-block orders.** It builds from the unchanged 2,719-line Mech source through final executable link in **2.199591 s**, versus **2.227699 s** for its control. The paired median is **0.986513× control time**, a 1.35% reduction. Native text saves another 48 B; measured peak stack is 4,752 B, below the 5,280 B cap. No 64th C win is established; confidence for runtime is separate.

This combines the retained broader input-packet qualification with the existing argument-free jump pass on the checked entry. The generic pass preserves cold boundaries, parameters, debug/value labels and preferred fallthrough, then removes unreachable code and verifies IR. The original codec and canonical bytecode remain unchanged. No LLVM dependency or shipping byte patch is added.

## Matched complete builds

The four Mech actors separate the packet proof from root cleanup. All use the same immutable source-built compiler and original source; only published compiler/profile options differ. Twelve balanced serial fresh-process orders include final executable linking, with compilers and fixed support equally prebuilt. Offline training, compiler bootstrap and strip are separate. Every generated object, full metadata, bytecode and stripped image is identity checked in each trial.

| Actor | Native text | Checked entry | Complete build | Native pipeline |
| --- | ---: | ---: | ---: | ---: |
| Selected forwarded-linear control | 63,516 B | 46,572 B | 2.227699 s | 1.232435 s |
| Broader packet proof | 63,196 B | 46,572 B | 2.229558 s | 1.233064 s |
| Root cleanup | 63,788 B | 46,848 B | 2.201067 s | 1.202541 s |
| Selected combination | 63,468 B | 46,848 B | 2.199591 s | 1.201935 s |
| Roc | 57,696 B | — | 2.992954 s | — |
| libdeflate 1.26 level 12 | 37,080 B | — | 0.490219 s | — |

The root pass reduces the measured native pipeline about 30 ms. It increases the checked entry 276 B and total native text 272 B; section padding accounts for the 4 B difference. The broader packet proof saves 320 B in helper functions, giving the combination a 48 B net saving. The large checked entry remains 46,848 B; this pass does not establish a large code-size reduction.

Paired source-through-link ratios for the combination are **0.734967× Roc / 4.521845× C**. Text is **1.100× Roc / 1.712× C**. Stripped executables remain **276,072 B Mech / 191,912 B Roc / 155,752 B C**. All six actually timed stripped images pass all 81 inputs: 486 executions, with output streams saved and independently inflated.

The paired median saving is **30.041 ms**, with a descriptive 90% percentile bootstrap interval **21.707–43.436 ms**, resampling the 12 complete balanced order pairs 8,000 times with seed 184920. The interval is not an extra selection gate. The portable verifier recomputes it exactly. Native phase medians and complete-build medians are separate and do not add.

## Runtime qualification

The combined candidate retains **81/81 Roc and 63/81 C in both full 81-case, 96-block orders**, with zero required names missing. Each case includes all 24 actor permutations four times. Calls include fresh heap/context setup through cleanup. Runtime is effectively flat against the control; small slowdowns remain included where named wins survive. Confidence support has not been reassessed.

| Case | Combined/control forward/reverse | Combined/C forward/reverse |
| --- | ---: | ---: |
| empty | 1.023554× / 1.022447× | 0.006813× / 0.006863× |
| 320-overlap | 1.006943× / 1.004875× | 0.963558× / 0.966604× |
| prefix-natural-4096 | 0.999672× / 1.004405× | 0.961163× / 0.979176× |
| prefix-natural-8192 | 0.993340× / 1.000580× | 0.937892× / 0.944222× |
| prefix-random-4096 | 0.988956× / 1.001040× | 1.471075× / 1.474859× |
| prefix-random-16384 | 0.991854× / 0.995141× | 1.364551× / 1.367788× |
| prefix-random-1048576 | 0.999540× / 0.999182× | 1.172233× / 1.171971× |
| 16384-distribution-change | 1.001290× / 0.998396× | 1.220179× / 1.227128× |

All three source combinations also complete both 21-case, 24-block discovery orders and preserve every observed 21 Roc / 9 C win. The 63,196 B family-only arm and 63,788 B root-only arm remain available with their native/panel evidence; they have no full 81-case qualification. The smaller family-only arm shows no broad runtime or build gain. All adverse observations are retained for repair and later combinations.

## Source reproduction

The main build path now defaults to the fully qualified combination on Apple ARM64:

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/mech-root-packet
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/mech-packet-only --variant m1-root-packet-families
```

From this evidence directory:

```sh
python3 build_compiler.py /tmp/mech-root-compiler
python3 reproduce.py /tmp/mech-root-compiler/release/compile_profile /tmp/mech-root-combined --role combined
python3 reproduce.py /tmp/mech-root-compiler/release/compile_profile /tmp/mech-root-combined-checked --role combined --checked
python3 verify.py
```

Six public normal/checked source replays and all three actual main named recipes reproduce object, full metadata and bytecode exactly. Source and profiles are inputs; no prebuilt encoder is used. The register profile still matches the complete matcher virtual code; the wider packet selection covers other functions. Every candidate passes 162 exact output/inflation/alias/resource/cleanup observations, six checked errors and twelve exact failure/recovery pairs. Peak stacks are 4,720 B family-only and 4,752 B root-only/combined. The original source resource observations and error origins are retained.

The compiler feature has 79 optimized scalar tests passing, maintaining direct Bool/U64/I64/F64 native paths. No Rust source changed in this combination. The source implementation and [register-profile provenance](../forward-jump-register-study/README.md) remain rebasable.

The portable verifier checks **3,041,856 saved runtime calls**, every full-suite named win, all archive members, six public source replays, three main recipes, twelve build orders, the paired bootstrap derivation and 486 saved deployment streams. All 413,283 zero-duration individual samples remain included.

## Broader continuation

The earlier [62,796 B packet-selection experiment](../conditional-winner-item-study/README.md) prompted this retest with current register/carry/CFG advances. Its 320 B saving is retained. The next larger investigation compares actual Mech/C matching, skip and tree work, then separates extra algorithm work from code-generation overhead. Existing VCode traces already map CLIF blocks to machine offsets; a source-site sidecar can provide the missing link while preserving emitted bytes. Deleted/merged blocks must not be treated as independent observed events. Prior DP work already matches C on nine cases. **64 C wins is next; 81 C wins remains the goal.**
