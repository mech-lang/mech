# Bounded selected unsigned increments

**Selected: `m1-bounded-increment-vector-linear`, 64,264 B, every named 81/81 Roc and 63/81 libdeflate win preserved in both full 81-case, 96-block orders.** The compiler now proves that selected non-strict unsigned bounds can make a checked `+1` overflow impossible. The original **2,719-line Mech source and canonical bytecode are unchanged**.

| Random 1 MiB | Mech | Current Mech control | libdeflate | Paired Mech ÷ C | Mech time reduction to parity |
| --- | ---: | ---: | ---: | ---: | ---: |
| Forward | 23.434895 ms | 23.506791 ms | 20.393270 ms | 1.149165× | 12.98% |
| Reverse | 23.430417 ms | 23.504812 ms | 20.352646 ms | 1.152650× | 13.24% |

This saves **144 B** versus the prior 64,408-B selection. Paired random-1-MiB time is **0.996426× /0.996761×** control, about **0.32–0.36% less time**. Full-suite geometric-mean point estimates are **0.995072× /0.995243×**; no formal confidence claim is added. Small slower cases remain included. **No 64th libdeflate win is established; about 3.0–3.1 ms, or 13.0–13.24% of Mech's time, still needs to be removed on the closest remaining case.** The earlier qualified **63,372-B compact recipe remains available**.

Matched complete builds take **2.196434 s Mech /2.983926 s Roc /0.480779 s C**, paired **0.735454× Roc /4.565690× C**. Twelve balanced serial fresh-process orders include final executable linking with compilers and fixed support equally prebuilt. The matched prior Mech control takes **2.195559 s**; no compile-time improvement is established. Selected parse/frontend/native medians are **0.703131 /0.216515 /1.203295 s**; final link is **0.035262 s**. Bootstrap and strip are separate.

Encoder text is **1.114× Roc /1.733× C**. The stripped executable remains **276,072 B**, **1.439× Roc /1.773× C**; measured peak stack is **4,832 B**, below the 5,280-B cap. The standard main rebuild command reproduces all three measured artifacts exactly. All seven retained source arms pass strict output/resource/error/recovery/stack gates and public normal/checked source replays. **79 optimized scalar tests**, the native increment/range tests, **169,600 focused native calls**, and **486 actual stripped-executable checks** pass. Portable verification recomputes **3,400,704 saved runtime observations**, including 464,141 retained zero samples, **72 actual counter cases**, all profile derivations and all 12 complete-build orders. Compiler source is rebasable at **`b73335e1c`**. No new LLVM dependency, codec rewrite, generic operation dispatch or shipping native-byte patch is added.

[All 81 elapsed times and remaining parity gaps](PARITY.md) · [Raw observations and build summary](summary.json).

A selected `x <= y` makes an unsigned `x + 1` safe when the immutable bound of `y` is strictly below the I64 maximum. The existing increment proof already handles dynamic strict `x < y`; the measured missing case is the matcher's selected `counter <= 258`. Its overflow guard executes 516,964 times on random 1 MiB. The extended proof accepts integer comparisons and immediate comparisons, both selected branch directions and operand orders. It retains unique-edge, current SSA, simultaneous phi substitution, cycle/backedge and proof-budget restrictions. Unbounded/non-strict MAX, signed comparisons, changed or bypassed values, different branch actuals, opaque values and `+2` retain their necessary guards. No arithmetic, memory operation or observable effect moves.

`MECH_PROVE_BOUNDED_UNSIGNED_INCREMENTS=1` is optional and off by default in the compiler. The M1 recipe combines it with the previously qualified SSA argument coalescing and shared-Boolean lowering. A disabled compilation reproduces the previous object, full metadata and canonical bytecode exactly, including checked register-pair emission. Only the matcher changes in the raw native census; the selected Huffman and scorer bodies remain exact.

Selected random 1 MiB matcher counts are **182,039,777 instructions /31,382,768 branches /11,213,259 stack loads /1,296,017 stack stores**, with 35 calls. The prior selected matcher has 183,596,351 instructions and 31,901,634 branches. This removes 1,556,574 instructions and 518,866 branches while adding 24 stack loads. These diagnostic counts describe actual bound native bodies, not hardware cycles. Elapsed timing remains the primary evidence.

Fresh register weights come from nine actual mixed-input counter cases mapped through every static native head, retaining unvisited zero counts. Fresh machine layouts use exact flow constraints where determined and explicitly label unresolved endpoint-based priorities as heuristics. The two retained source-label guides map prior qualified ranks through surviving source block labels, with fresh flow priorities for unmatched heads; source labels, successors and machine graphs are not claimed identical. Normal/checked public source replays and strict gates cover all seven arms. Three preparatory arms have no elapsed-time claim; the four timed layouts have both 21-case, 24-block orders. Full 81-case qualification is attached only to the selected arm. All slowdowns remain included.

| Timed layout | Text B | Random1MiB Mech ms, forward /reverse | Paired Mech ÷ C | Full 81-case qualification |
| --- | ---: | ---: | ---: | --- |
| bounded-sqrt | 64,280 | 23.520374 /23.503771 | 1.158998 /1.157964× | not run |
| bounded-sqrt-guide | 64,376 | 23.706999 /23.725708 | 1.167013 /1.168589× | not run |
| bounded-linear-guide | 64,360 | 23.686104 /23.701167 | 1.174291 /1.168433× | not run |
| bounded-linear | 64,264 | 23.440792 /23.430709 | 1.155047 /1.153662× | 81 Roc/63 C, both orders |

All seven arms pass 162 exact output/inflation/alias/resource/cleanup observations, six named checked errors, twelve injected-failure recoveries and the 5,280-B stack cap. The rebuilt optimized library passes 79 scalar tests, three native increment tests, three word-range tests and the source-composition error/retry test. The focused build compiles the actual proof/test sources and exact support excerpts against pinned backend crates; its 169,600 native calls cover 36 modes, width boundaries, zero budgets, different branch arguments, saved loop predicates, observable memory, early errors and retry. The complete GPU suite is not claimed. Initial passing test logs are retained; execution was repeated after primary clocks to record the full test environment.

The complete-build cohort compares the previous selected compiler/recipe, new compiler with the option disabled, selected linear and preparatory square-root arms, Roc and libdeflate. All generated object, full metadata and bytecode hashes are checked in every order. Compilers/fixed support are equally prebuilt, final link is included, and all 486 actual stripped deployment streams are saved and independently inflated. Compiler/test bootstrap took 230.409 /382.843 s while constructed concurrently; these are not isolated bootstrap comparisons. No build, test, profile, counter, archive or Git work ran during primary runtime or source-through-final-link clocks.

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/bounded-unsigned-increments/verify.py
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py /tmp/mech-bounded
```

To build an isolated compiler from the supplied source snapshots and reproduce any retained arm:

```sh
study=benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/bounded-unsigned-increments
python3 "$study/build_compiler.py" /tmp/mech-bounded-compiler
python3 "$study/reproduce.py" /tmp/mech-bounded-compiler/release/compile_profile /tmp/mech-bounded-native --role bounded-linear
```

Add `--checked` for checked register-pair emission. The main source compiler change is rebasable at `b73335e1ccbe6f89dac60d249ae9c989eeaf8ef6`; it is compiler source, not only built artifacts. Original source/bytecode and all named wins remain maintained. The main default changes only on Darwin arm64; earlier named alternatives remain available. [Native/build receipts](native-and-builds.tar.gz) · [Runtime observations](runtime.tar.gz) · [Archive manifest](manifest.json) · [Portable verification](verification.json).

The next broad experiment revisits current-word, known-width and argument-transport cleanup after the new SSA coalescing. Count whether it can remove the remaining hot high-word guards. Private input-packet specialization also needs current SSA or all-callsite bounds. Shared comparison ANDs, immutable byte-load reuse and the retained compact recipe remain recorded for later combinations. **64 C wins is next; 1× libdeflate across all 81 remains the goal.**
