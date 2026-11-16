# Restore the checked append path

**R355 remains the retained comparison.**

The append candidate beats Roc on 47/81 cases (58.02%) and libdeflate on 3/81. R355 wins 47/81 and 8/81 in this run. The paired candidate-to-R355 geometric mean is 1.006549×. The archive preserves all 38 paired point estimates above 1×, 6 above 1.05× and 6,475 zero timer observations.

| Input | R355/Roc | Candidate/Roc | R355/libdeflate | Candidate/libdeflate | Candidate/R355 |
| --- | ---: | ---: | ---: | ---: | ---: |
| eight-literals | 0.187781× | 0.182594× | 2.413115× | 2.346457× | 0.972376× |
| reymont-prefix-32769 | 1.190339× | 1.225799× | 1.083843× | 1.116131× | 1.029791× |
| prefix-natural-32768 | 1.188205× | 1.226317× | 1.080756× | 1.115422× | 1.032075× |
| prefix-natural-131072 | 1.198545× | 1.260130× | 0.984203× | 1.034775× | 1.051383× |
| prefix-natural-262144 | 1.198422× | 1.264267× | 0.970981× | 1.024330× | 1.054943× |
| prefix-natural-1048576 | 1.212941× | 1.271851× | 0.977794× | 1.025284× | 1.048568× |
| prefix-random-1048576 | 1.482731× | 1.465534× | 2.097946× | 2.073613× | 0.988402× |
| prefix-periodic-1048576 | 0.639351× | 0.645395× | 1.080427× | 1.090641× | 1.009454× |
| prefix-repeated-1048576 | 0.648177× | 0.642438× | 1.016668× | 1.007666× | 0.991146× |

## Direct comparison with the previous recovery

A separate three-actor screen measures the exact previous recovery and the append candidate in the same blocks with R355. The fixed seven cases retain 41,832 complete calls.

| Input | Previous/R355 | Append/R355 | Append/previous |
| --- | ---: | ---: | ---: |
| eight-literals | 0.971383× | 0.973743× | 1.002429× |
| seven-stored-prefix | 1.156627× | 1.129727× | 0.976742× |
| prefix-natural-262144 | 1.028088× | 1.054699× | 1.025884× |
| prefix-periodic-262144 | 1.183791× | 1.013763× | 0.856370× |
| prefix-periodic-1048576 | 1.182891× | 1.014634× | 0.857757× |
| prefix-repeated-1048576 | 1.148403× | 0.992050× | 0.863852× |
| prefix-random-1048576 | 1.111129× | 0.990084× | 0.891061× |

## Counters and sampled time

| Input | Retired instructions / previous | CPU cycles / previous | Matcher samples | Composed parser samples | Other parser samples |
| --- | ---: | ---: | ---: | ---: | ---: |
| prefix-natural-262144 | 1.013867× | 1.026354× | 37.1% | 56.7% | 0.0% |
| prefix-periodic-1048576 | 0.928039× | 0.853039× | 63.2% | 14.5% | 2.4% |
| prefix-repeated-1048576 | 0.929346× | 0.863594× | 57.5% | 13.3% | 2.2% |
| prefix-random-1048576 | 0.920777× | 0.890039× | 59.5% | 0.0% | 9.4% |
| eight-literals | 0.999687× | 1.010382× | 13.5% | 0.0% | 0.2% |

The hardware counter companion uses the exact candidate Object and a separate set of 20 balanced calls per actor per case. The profiles sample the exact elapsed-timing worker in a separate acquisition. Sample percentages include validation.

Natural 256 KiB spends 56.75% of sampled running time in the composed parser and 37.07% in the matcher. Its direct elapsed ratio is 1.025884× the previous recovery, with 1.013867× retired instructions and 1.026354× cycles in the separate counter cohort. The periodic and repeated parser shares move back toward R355, and their elapsed time improves by 14.2% and 13.6% in the direct comparison. The exact generated body comparison localizes the code changes to parser variants.

## Compiler change and evidence

The checked append trace now recognizes the merged backend’s fused unsigned U32 reads before the previous-word load. It also resolves the low bit of a path-local I8 constant through the selected incoming edge. The append and exit edges carry distinct values. Original-width reads, the final U64 previous-word check, current geometry, source store order and fallback guards retain their existing checks.

Of 351 generated bodies compared, only the four ordinary and four composed parser bodies change. The root and matcher bodies are byte-identical. The matcher retains its 60 wide loads. All four real phases prove four carried words, thirteen packet reloads and two entry guards. Eight focused controls cover the old plain reads, fused widths and offsets, invalid extending and post-store loads, canonical masks, and independent incoming-edge values. The earlier fused-only attempt reached the masked join and declined all four phases. The exact phase probe isolated the rejected branch before the full candidate was built.

Compiler base `ef3beb1985bbbf8e17e9436d1013ebb4a7eb215f` plus patch SHA-256 `5f46bd68b69204a5c8fb3e3851fdc718988cf5e3f50cb0aec0114321d18c7b18`. Benchmark source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`. Original Object `d3b2455e88b397511d2d3364c1018144bf069ea77dc6fb797eeb4cd99ba2a3a4`. Linked worker `c248e224c47e2e4c0620aeb21e4cd77205ae5f612152001822cd8577fd878f1e`.

The original linked artifact passed 162 checked calls with exact output, independent inflation, alias preservation and cleanup. The 81-case timing uses 24 rotated actor orders with at least 200 ms warmup per actor per case. Both Mech actors use their qualified compact descriptors and time full compression, scratch construction, output transfer and private cleanup. The decoded image and repeated recovery qualification belong to the subsequent retention decision.

[Raw paired observations, counter and profile receipts, and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/compression-append-recovery-954739b7efd4.tar.gz).
