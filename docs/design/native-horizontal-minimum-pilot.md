# Select ARM64 horizontal unsigned minima

Keep the [composed countdown baseline](native-composed-countdown-qualification.md). The trial takes **1.000126×** paired control time across all 81 cases. Both 1.05 gates pass everywhere. The Roc score stays at **52/81 (64.2%)**. Repeated 65,535 bytes loses its libdeflate win in both elapsed cohorts.

| Input | Trial / control | Control / Roc | Trial / Roc | Control / libdeflate | Trial / libdeflate |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 1.001663× | 0.173333× | 0.173621× | 2.183359× | 2.186990× |
| Natural 8 KiB | 0.999324× | 1.004509× | 1.003829× | 1.249498× | 1.248653× |
| Natural 32 KiB | 1.000679× | 1.121545× | 1.122307× | 1.020718× | 1.021412× |
| Natural 256 KiB | 0.997369× | 1.144936× | 1.141924× | 0.928178× | 0.925736× |
| Natural 1 MiB | 0.999332× | 1.155606× | 1.154833× | 0.931999× | 0.931376× |
| Repeated 65,535 bytes | 1.005712× | 0.563912× | 0.567133× | 0.997920× | 1.003620× |
| Repeated 65,537 bytes | 1.004383× | 0.568510× | 0.571002× | 1.004820× | 1.009225× |
| Held-out slides 65,537 bytes | 0.999344× | 1.154826× | 1.154069× | 1.008715× | 1.008054× |

In the full matched cohort libdeflate wins move from 16 to 15 for Mech; cases faster than both peers go from seven to six. Forty-four paired estimates improve. The worst increase is periodic 4 KiB at 1.014900× control. The 16-case screen takes 1.001487× control. Its natural 8 KiB result is 1.000016× Roc; repeated 65,535 and 65,537 bytes cross upward against libdeflate. Repeated 65,537 is already above parity on both sides of the full run. The two runs retain 56,640 and 245,664 timed calls and every adverse result.

The AArch64 selector recognizes an exact pair of little-endian, lane-preserving shuffles and unsigned minima over four U32 lanes. Each shuffle must select the proved input for every byte. The result is the same scalar minimum in all four lanes. A sole extraction uses `UMINv` and a register move. A shared vector result broadcasts the minimum once; scalar extracts then use that existing value. A bounded direct-user check declines the sole-extraction route when the result is shared. The first backend prototype duplicated a cost minimum; the measured prototype shares it. The existing [NEON witness](native-vector-minimum-experiment.md) has the same two horizontal minima, while the inspected Roc parser uses scalar comparisons. The source, range and capacity proofs, wrapping costs, stable tie selection and ordinary checked fallback are unchanged.

The linked composed vector finish uses two `UMINv` instructions and removes five instructions across cost and winning-index reduction. Sixteen of 357 Object bodies change, twelve of them retained as named linked entries. The composed path goes from 1,328 to 1,312 bytes and keeps its 224-byte frame. The ordinary no-growth path goes from 1,504 bytes/272-byte frame to 1,456/256. All 1,348 linked text symbol addresses and the root Object body stay the same. The repeated 65,535-byte profile reaches the ordinary no-growth path; it has no composed-parser samples. The observed slowdown does not establish that the new vector instructions cause it directly.

| Separate hardware counter cohort | Instructions before | Instructions after | After / before instructions | After / before cycles |
| --- | ---: | ---: | ---: | ---: |
| Eight literals | 626,825 | 626,773.5 | 0.999918× | 1.014879× |
| Natural 8 KiB | 14,708,275.5 | 14,678,155.5 | 0.997952× | 0.989496× |
| Natural 256 KiB | 565,724,973 | 565,266,843 | 0.999190× | 0.999605× |
| Natural 1 MiB | 2,019,238,960 | 2,017,805,780 | 0.999290× | 1.000933× |
| Repeated 65,535 bytes | 24,039,473.5 | 24,031,815 | 0.999681× | 0.998430× |

The exact executable timer profiles place 2,546/4,682 natural 8 KiB and 3,013/5,352 natural 256 KiB running samples in the composed parser. On repeated 65,535 bytes, the ordinary no-growth parser has 900/5,280 and the matcher has 2,813/5,280. These are timer samples, separate from the whole-call counters. The six private payload measures match on every case.

The compiler is based on `4de09acf2643f706110c0c747517080f11a90792` with patch SHA-256 `2927444bfdce44b584fd29112e4a0bda39b249b1f58907b31f7c95d61c7a7f17`. Nightly 2026-03-03 builds the backend and optimized six-feature native compiler with warnings as errors. The revision-14 decoded input emits Object `68abc4720add250c43af0285249cf15516cb83e46daa06c2e94d8bddb09ebf29` in 574.02 seconds. The timed worker SHA-256 is `d64802d27b0ee9269d421b020d8c1f3dc5100e4d8f95397ed21abcdc7a0ba182`.

The two JIT variants pass 48,020 scalar-oracle calls over ten scenarios, including all extraction lanes, shared users, unsigned boundaries, ties and refused patterns. A wrong expectation for the distinct-input shuffle was corrected against the unchanged control before candidate validation. The two descriptor launches pass 324 full linked checks. The direct phase oracle passes 68,096 positions, original rejection origin and retry. All 81 complete cases retain exact output, independent inflation, capacities, aliases and cleanup. Original source emission, the managed performance guard and full CI were skipped for this declined patch.

[Compiler patch, linked disassembly, timing and replay evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-4de09acf26-horizontal-minimum.tar.gz). SHA-256 `98dcb2a061143441c13ad3c7854af4696acfb68a430237caa67e30297c904e1e`. Run `python3 -B verify-round.py` after unpacking.
