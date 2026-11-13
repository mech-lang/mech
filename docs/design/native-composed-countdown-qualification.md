# Composed ordered-reduction qualification

The compiler reproduces the [isolated composed-reduction comparison](native-reduction-variant-isolation.md). Four composed reductions change. The four ordinary reductions preserve their measured control code. The complete 81-case comparison takes **0.993395×** paired control time, with both 1.05 gates passing for all cases. Mech beats Roc on **52/81 (64.2%)** and libdeflate on **16/81 (19.8%)**. Seven cases beat both peers.

| Input | Candidate / control | Mech / Roc | Mech / libdeflate |
| --- | ---: | ---: | ---: |
| Eight literals | 0.997940× | 0.170546× | 2.188515× |
| Natural 8 KiB | 0.981852× | 1.001108× | 1.245266× |
| Natural 256 KiB | 0.981276× | 1.148857× | 0.931645× |
| Natural 1 MiB | 0.977936× | 1.161237× | 0.935114× |
| Repeated 32 KiB | 0.995841× | 0.507890× | 0.997728× |
| Repeated 65,535 bytes | 1.001493× | 0.565938× | 0.999640× |
| Repeated 65,537 bytes | 1.002326× | 0.570395× | 1.008592× |

The candidate wins one additional libdeflate case in this cohort. Repeated 65,535 bytes stays below parity by 0.036%. Its two paired timing-order halves move in different directions. Periodic 128 KiB has the largest adverse paired estimate at 1.015264× control. All adverse blocks and the ordinary-only screen remain in the timing packet. The natural 8 KiB counter cohort retires 0.960743× instructions and uses 0.984981× cycles. At natural 256 KiB the corresponding ratios are 0.963739× and 0.977275×. The linked timer profile locates the natural-text change in the composed parser.

## Selection and execution

The opt-in compact compiler first builds the existing checked phase and its composed immutable-lookup variant. `MECH_REDUCTION_COUNTDOWN=1` lets it re-emit a complete reduction for that variant using a forward read ordinal, reverse cursor and remaining count. The candidate must prove the same composition and satisfy its own IR verification. The existing typed window facts still establish every reached read. The biased ordinal also supplies the selected item's low word only when the item has the required representation and the other fields are independent of the induction variable. Strictly lower cost still selects the item, which preserves ties and any original checked fallback.

The opt-in AArch64 `MECH_ARM64_LOOP_LIVE_RANGES=1` path reuses identical comparisons and sinks a pure increment until after the old ordinal's last use. It matches current canonical virtual operands, flag-preserving instructions, a decrement and a zero test in a recurring block. The rewrite runs before register allocation and declines blocks with heterogeneous source locations or functions with debug labels or stack maps. The measured continuing loop falls from 16 to 11 instructions, with three moves eliminated. The Mech program, revision-14 bytecode, output ownership and scratch behavior are pinned to the control.

## Build and verification

The measured source patch is `f9534e4b8e91ea195a30cee8b1db9e0aaabde2eec9613b3c5127ef2f9e73d9d0` over `4fff511238df193562e976dfa34fc3c354c60256`. Its review parent is `e9b80c8ce714c5e448904e383d7f24641098ca1c`, which adds the isolation results. Nightly 2026-03-03 builds the optimized producer with `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit` and `-D warnings`. The archive lists the commands and experimental flags. The original source invocation takes 641.832 s including parsing, semantic compilation, codec roundtrip and Object emission. The decoded invocation takes 622.577 s and also starts from the source file before selecting the decoded artifact.

Both builds emit Object `32a9faefa3b3d3673341659ebf269978261758755052fe62a0995660002466fd` and descriptor `960b5e7eb10d1cb9cdac81ff858379ce8312cc013d1fb669d1a42c7289ac0cef`. With the pinned runner and runtime, each links to the exact timed worker `f751d261e87596d360fe31998fb68c19bee34d3b53ae2a859280641dfdb30d32`. The exact and shared descriptors pass 648 complete linked calls with reference bytes, independent inflate, capacity, held aliases and zero live private owners at exit. The separately linked current runtime passes 162 more correctness calls. Its executable bytes differ from the frozen runtime link, which supplies the performance claims.

The direct ordinary and composed phase oracles pass 136,192 positions across both images. They check ties, wrapping values, scalar and vector tails, the same failure origin, and recovery. The managed post-JSON guard passes 88 exact results with all 13 generated bundle hashes matching control. Its primary and confirmation cohorts take 0.715374× and 0.709525× Rust. Changed-file formatting, compiler-planning quarantine, host catalog, resident routing and interactive architecture pass. The full formatting, warning-policy and unsafe audits report the same inherited paths as the parent. Qualification uses the local runs above.

[Compiler patch, receipts and verifier](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-4fff511238-composed-countdown-qualification.tar.gz). SHA-256 `4f3dc3ca78426ac1bfbbda60ec11a023582d22f9c1d1ba44649581a0ec5f059d`. Run `python3 -B verify-qualification.py` after unpacking.
