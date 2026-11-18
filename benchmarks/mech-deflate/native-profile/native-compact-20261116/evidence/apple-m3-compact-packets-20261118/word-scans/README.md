# Word equality scans and current-guard reuse on Apple M3

The source-built encoder contains **64,512 B** of text and preserves the original Mech source and canonical bytecode. The initial campaign below recorded 60 libdeflate wins per order. A [complete confirmation](confirmation/README.md) of the identical retained binary records **81/81 Roc and 61/81 libdeflate in both orders**, meeting the M3 comparison target. Near-tied cases vary between campaigns; the additional net win is not attributed to a code change.

| Initial complete order | Roc wins | libdeflate 1.26 level 12 wins |
| --- | ---: | ---: |
| Forward | 81/81 | 60/81 |
| Reverse | 81/81 | 60/81 |

[Every case in seconds and comparator multiples](timings.csv) · [Complete counts](results.json) · [Plan](plan.json)

The unchanged peer binaries run serially in 96 balanced blocks per case. Each compression starts with fresh encoder state. The measured interval includes heap construction, final allocation transfer and private cleanup; source preparation, output validation and reporting are outside it. This is the existing diagnostic compact-native contract, rather than a production capability-startup benchmark. Ratios are medians of paired observations; values below one are faster. [Environment](environment.json) · [Contract](contract.json)

## Compiler changes

The equality option retains the original admitted U64 recurrence instead of speculative 64-byte and 16-byte probes followed by a scalar retry. Earlier retained matcher counts showed that almost every 64-byte probe mismatched. Those counts came from the source-pinned M1 trace; they are a control-flow motivation, not M3 timing evidence.

Complementary unsigned comparisons use the same current SSA operands and a dominating selected edge. Fresh loads and changed operands remain distinct. Equality and inequality decisions use the same polarity normalization. The pass preserves the earliest failing source check and keeps stores and other effects in their original order.

The dense scorer uses the source-pinned immutable-cost hoisting and checked continuation from [upstream #993](https://github.com/mech-lang/mech/commit/cebfd544a69880e537b05be21235df956a611f0e). Phase packet loads gain alignment, readonly and nontrapping flags only after the entire nonescaping load contract is proved. The selected recipe contains no benchmark-input substitution or foreign encoder call.

## Correctness and reproduction

The linked candidate passes 162 exact-output and independent-inflation calls, borrowed alias checks, six checked failure cases and cleanup probes. Stack high-water is **4,736 B**, below the unchanged 5,280 B ceiling. [Correctness record](correctness.json)

Tests cover 211 backend cases, 805,600 scan-oracle calls, 14,400 complementary-predicate oracle calls and the phase packet admission/refusal cases. The earlier published 67,648 B encoder and the 66,192 B direct-branch encoder reproduce byte for byte. The local runtime, allocator and peer binaries remain frozen.

Build the compiler at the pinned source revision, then run `reproduce.py COMPILER OUTPUT_DIRECTORY`. The recipe checks source, profile, object, metadata and bytecode identities. The linked-worker record identifies the frozen deployment support. Its build durations are diagnostic, not controlled compile-time comparisons. [Recipe](recipe.json) · [Source inventory](source-inventory.json) · [Local build](local-build.json)

All screened and full comparisons, including rejected variants, are retained in [the comparison index](comparisons.json). Complete observations, exact source snapshots, native workers and reproducible diagnostic scripts are preserved in the scan-wave evidence archive. [Archive record](../scan-wave-archive.json)
