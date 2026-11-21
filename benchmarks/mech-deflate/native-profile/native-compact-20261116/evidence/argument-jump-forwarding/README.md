# Source-built argument forwarding and runtime parity

**Selected: `m1-argument-all-sqrt`, 64,412 B, every named 81/81 Roc and 63/81 libdeflate win retained in both full 81-case, 96-block orders.** Pure SSA argument transport forwarding with fresh profiles takes **1.49–1.59% less elapsed time across the full suite by geometric mean** than the previous 63,468-B selection. Its measured peak stack is 4,736 B. The **63,372-B compact alternative** also retains every named win in both full orders, saving 96 B and taking 0.87–0.90% less full-suite time; random 1 MiB costs 0.23–0.30% more than its matching control. Both retain the original 2,719-line source and canonical bytecode.

**The nearest remaining loss is random 1 MiB: 23.68–23.69 ms Mech versus 20.34–20.35 ms C, paired 1.164–1.165× C. About 3.34 ms, or 14.1% of Mech time, must still be removed to tie.** No 64th C win is established. Milliseconds below are per-call medians; ratios come from paired observations, so dividing displayed aggregate medians need not reproduce the paired ratio.

| Test | Mech ms, forward /reverse | C ms, forward /reverse | Paired Mech ÷ C | Mech time reduction still needed |
| --- | ---: | ---: | ---: | ---: |
| Random 1 MiB | 23.691354 /23.683562 | 20.353500 /20.343750 | 1.163816 /1.164937× | 14.08% /14.16% |
| Distribution change, 16 KiB | 0.579458 /0.587708 | 0.482479 /0.483042 | 1.200310 /1.209133× | 16.69% /17.30% |
| Random 4 KiB | 0.107542 /0.108750 | 0.076312 /0.076979 | 1.426908 /1.425898× | 29.92% /29.87% |
| Periodic 1 MiB | 12.004729 /12.004251 | 18.465896 /18.409959 | 0.649893 /0.652389× | 0.00% /0.00% |

Latest matched complete builds on **Apple M1** take **2.200294 s Mech /2.984815 s Roc /0.485399 s C**, paired **0.735180× Roc /4.521701× C**. Twelve balanced serial fresh-process orders include final executable link with compilers and fixed support equally prebuilt. Paired time versus the previous compiler/recipe is 0.998819×; **no compile-time gain is established**. Selected parse/frontend/native medians are 0.706155 /0.217352 /1.203106 s. Stripped executables remain 276,072 B Mech /191,912 B Roc /155,752 B C. Selected encoder text is 1.116× Roc /1.737× C; compact text is 1.098× Roc /1.709× C.

Sixteen public normal/checked source replays and both actual main source recipes, including the new main default, reproduce object, full metadata and bytecode exactly. Every candidate passes strict original output/resource/error/recovery checks and the 5,280-B stack cap. The portable verifier checks **6,801,408 saved runtime calls**, 935,509 retained zero samples, **45 actual training counter cases**, both profile derivations, all twelve complete-build orders and **972 saved stripped deployment streams** with independent inflation. The compiler source has **5 SSA regression tests and 79 optimized scalar tests passing**, including direct checked Bool/U64/I64/F64 original/decoded paths. Source implementation is committed and rebasable at `a0bc28d87`; no LLVM dependency or shipping native-byte patch is added.

[All eight source recipes, tradeoffs, raw measurements and verification](README.md) · [All 81 elapsed times and remaining parity gaps](PARITY.md).

The next broad proof investigates masked observation-counter indices and repeated read/write bounds in the actual matcher, reusing existing admission and word-range machinery. Confirm which checks remain before adding a pass; the existing bounded byte-histogram optimization is already enabled. Report elapsed milliseconds, paired ×C and the remaining time reduction for every target case. **64 C wins is next; 1× libdeflate across all 81 runtime cases remains the goal.**

## All retained candidates

Discovery panels have 21 cases and 24 blocks per case in each order. Full qualification has 81 cases and 96 blocks per case. Full scores are attached only to the two completed full campaigns. Small slowdowns remain included when named wins survive. Confidence is reported separately.

| Role | Encoder text B | Source through final link s | Panel geometric mean ×control, forward /reverse | Panel random 1 MiB ×C, forward /reverse | Full 81 qualification |
| --- | ---: | ---: | ---: | ---: | --- |
| root | 64,364 | 2.193698 | 0.998718 /1.000891 | 1.179455 /1.174986 | not run |
| vector-default | 63,820 | 2.203335 | 1.031758 /1.030331 | 1.259906 /1.258218 | not run |
| vector-linear | 63,548 | 2.196647 | 0.997348 /0.999236 | 1.162070 /1.160459 | not run |
| vector-sqrt | 63,532 | 2.199504 | 0.996205 /0.993027 | 1.166201 /1.167099 | not run |
| vector-neighbor | 63,372 | 2.203338 | 0.999330 /0.999857 | 1.177753 /1.177739 | 81 Roc /63 C, both orders |
| all-linear | 64,428 | 2.197508 | 0.999943 /0.996745 | 1.161342 /1.161505 | not run |
| all-sqrt | 64,412 | 2.200294 | 0.990907 /0.991003 | 1.167564 /1.166546 | 81 Roc /63 C, both orders |
| all-neighbor | 64,252 | 2.194705 | 0.996665 /0.997983 | 1.173059 /1.178361 | not run |

The linear discovery arms reach about 1.161× C. All-linear takes 23.59–23.61 ms versus 20.30–20.32 ms C; its full 81 qualification is outstanding. All-sqrt has the broader measured gain and full named-win qualification. The vector-default arm misses the historical natural 4 KiB C win in reverse, even though its matching control also crosses parity. It remains retained with all adverse observations. No arm earns the 64th C win.

## Implementation and limits

`MECH_COMPACT_FORWARD_ARGUMENT_JUMPS=root|vector|all` composes SSA arguments through pure jump-only blocks. It preserves each edge independently, including differing arguments to the same branch destination. Type/arity, aliases, escaped parameters, cold boundaries, entry blocks, debug/fallthrough metadata and unextracted egraph unions are checked. Forward layout order makes the composed plans acyclic; ordinary loop backedges remain valid. CFG cleanup and ISA verification run after an actual rewrite. The existing argument-free pass and efficient scalar path remain maintained. Disabling the new flag reproduces the previous selected object, full metadata, bytecode and exact worker.

Prepared matcher construction falls from 1,176 to 1,127 blocks. Unweighted argument forwarding grows matcher code and slows the broad panel. Fresh register and machine-layout profiles repair it; the prior 1,176-block profile is correctly rejected for the new 1,127-block VCode. Corrected linear/square-root weights use complete static heads and preserve unvisited zeros before the frequency floor. The compact neighbor heuristic intentionally fills from nearby visited heads and is labeled **not actual block frequencies**. The 45 counter cases describe actual diagnostic training objects and are not relabeled as final-candidate timings or hardware cycles.

The three selected private native bodies (matcher, Huffman and composed DP) are identical between each vector arm and its combined all arm. The small last-vector body is folded away in the canonical object and is not counted as a fourth surviving body. The throughput arm adds 944 B versus the previous selected encoder; its stripped executable size is unchanged. The compact arm saves 96 B and keeps both full 81 scores. Every partial, slowdown and failed attempt is retained.

## Build measurement and reproduction

The 12 actors run serially, one per position in each of 12 balanced orders, after warmup. Source parsing, native code generation and final minimal executable link are included. Compiler construction, fixed-support construction, strip, tests, profiles, counters, Git work and archive work are outside those clocks. A first warmup stopped on missing temporary Roc source; a separate preflight found a missing C header. Exact published sources were restored under the pinned archive hash, 76 source files were saved and bound to the final commands, and both original encoder hashes were reproduced before the entire cohort restarted. Failed warmup/preflight records are retained separately and excluded from the 12 measured orders.

The compiler rebuild took 226.229 s while independent test compilation ran in another target directory. This is not an isolated compiler-bootstrap comparison. All source-through-link clocks use immutable prebuilt compilers. The entire GPU test suite is not claimed; the 5 SSA tests and 79 scalar tests are recorded under the all scope.

Portable verification on a current Python:

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/argument-jump-forwarding/verify.py
```

Build the compiler and regenerate any original-source arm on Apple arm64:

```sh
study=benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/argument-jump-forwarding
python3 "$study/build_compiler.py" /tmp/mech-argument-compiler
python3 "$study/reproduce.py" /tmp/mech-argument-compiler/release/compile_profile /tmp/mech-argument-native --role all-sqrt
```

Add `--checked` for checked register-pair emission. The main M1 default is exercised by `rebuild.py OUTPUT`; select the compact arm with `--variant m1-argument-vector-neighbor`. All output directories must be fresh. The source recipe verifies all three native artifacts; the complete minimal-deployment link inputs, commands, timings, images and saved inflated streams are archived separately.

[Raw summary and every observation](summary.json) · [All 81 runtime/parity rows](PARITY.md) · [Machine-readable parity](parity.json) · [Source recipes](recipes.json) · [Verification result](verification.json) · [Content-hash manifest](manifest.json) · [Native/build evidence](native-and-builds.tar.gz) · [Runtime evidence](runtime.tar.gz).
