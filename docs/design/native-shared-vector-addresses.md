# R269 — Share computed addresses across vector loads

**Retain R268.** The shared-address candidate finished the 19-case screen at 5 → 2 cases faster than Roc. The paired geometric change was +0.019%. Repeated 33,027 bytes stayed above parity and the eight-literal case missed both 1.05 gates. The retained 81-case result remains 26/81 (32.1%).

PR base 92f75db84e2523693648084bea16cd52ad60777a. Measured parent and retained compiler 1e6459bee97ef35b55794c95df9fb72cbf455bb2. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, IR/work/ABI/bytecode 215/107/39/14. The exact R268 Mech source, input bytecode, allocator, algorithm, capacities and output ownership conditions are reused. The measurement covers complete level-12 compression, scratch construction, output transfer and private cleanup. Ordinary managed admission and recovery remain outside this measurement boundary.

## Exact executable evidence

On repeated 33,027 bytes the exact retained matcher accounts for 3,185/5,568 running timer samples. After the selector change it accounts for 3,200/5,570. The reference matcher and surrounding setup occupy 4,006/5,355 samples in its own image. The reference uses eight-byte comparisons. Small C and Rust controls for the same guarded 64-byte comparison form the two addresses once and use paired 128-bit loads. Their machine code is retained as a control. Sampling includes setup and validation and gives CPU-time weights.

Each of nine widened comparisons in each matching body had six address adds inside its 64-byte group. The two-file AArch64 change prefers a shared two-register sum when the existing use analysis reports multiple uses and the positive displacement fits the scaled unsigned load encoding. Single-use, zero-displacement, constant, scaled and extended forms keep the original selector rules. The same eight loads retain their widths, flags, addresses and order. The bounds, match extension, fallible scalar fallback and loop-carried state are unchanged. All eighteen selected groups go from 24 to 20 instructions through the mismatch branch, with zero stack transfers in the inspected block. Both matching frames shrink 960 to 944 bytes. The vector body shrinks 41,576 to 41,356 bytes. The general matching body grows 51,868 to 53,280 bytes.

## Native timing screen

The 12 balanced blocks per case, matched warmup and verified outputs produce 56,700 measured calls. Both gates pass in 18/19 cases. The eight-literal pooled and block ratios are 1.08641× and 1.07225× retained. Ratios in the first two timing columns are the medians of each actor’s within-block ratios to Roc. The paired change uses the candidate/retained ratio directly within each block.

| Case | Retained / Roc | Candidate / Roc | Paired time change | Both gates |
|---|---:|---:|---:|---|
| empty | 1.97619× | 1.97619× | +0.000% | pass |
| one-stored | 1.97619× | 1.97619× | +0.000% | pass |
| seven-stored-prefix | 1.97619× | 1.97619× | +0.000% | pass |
| eight-literals | 0.40352× | 0.43609× | +2.140% | fail |
| 33027-periodic-window | 1.05285× | 1.05243× | -0.306% | pass |
| prefix-natural-32768 | 1.68608× | 1.67360× | -0.512% | pass |
| prefix-natural-1048576 | 1.75402× | 1.74663× | -0.409% | pass |
| prefix-random-32768 | 1.78470× | 1.76475× | -0.326% | pass |
| prefix-random-1048576 | 2.11562× | 2.11051× | -0.264% | pass |
| prefix-periodic-32767 | 1.04444× | 1.04154× | -0.948% | pass |
| prefix-periodic-32768 | 1.04273× | 1.04948× | +1.035% | pass |
| prefix-periodic-32769 | 1.04193× | 1.04297× | -0.196% | pass |
| prefix-periodic-33027 | 1.06259× | 1.05846× | +0.734% | pass |
| prefix-repeated-4096 | 0.72824× | 0.73723× | -1.457% | pass |
| prefix-repeated-32767 | 0.99616× | 1.00830× | +0.752% | pass |
| prefix-repeated-32768 | 0.99641× | 1.00116× | +0.715% | pass |
| prefix-repeated-32769 | 0.99138× | 1.00099× | +1.031% | pass |
| prefix-repeated-33027 | 1.02319× | 1.02218× | +0.059% | pass |
| prefix-repeated-65536 | 1.09911× | 1.09119× | -1.605% | pass |

## Independent counters

| Case | Retired instructions before → after | Instructions change | Cycles change |
|---|---:|---:|---:|
| eight-literals | 1,134,069.0 → 1,134,060.5 | -0.001% | +1.425% |
| prefix-natural-1048576 | 3,341,565,415.0 → 3,341,040,775.0 | -0.016% | +0.170% |
| prefix-periodic-32768 | 25,342,042.0 → 24,950,311.0 | -1.546% | -0.045% |
| prefix-repeated-32768 | 25,285,364.5 → 24,893,208.0 | -1.551% | +0.104% |
| prefix-repeated-33027 | 26,017,820.0 → 25,607,350.0 | -1.578% | -0.778% |

The candidate uses fewer retired instructions on the intended input. Its elapsed and cycle cohorts give mixed results. The selected vector address lowering is rejected at the native screen. This pilot covers the original linked image and five independent hardware-counter cases. R268 remains the checkpoint for decoded artifacts, heap qualification, the two full acceptance cohorts and the 81-case score.

The first ISLE regeneration required the selector predicate to be marked pure. Regeneration and warnings-as-errors builds then passed. The initial screen stopped before collecting any measurements because the restricted shell denied its idle-machine check. The complete measurement used the same predeclared inventory with that check enabled. All retained calls passed exact compressed bytes, independent inflate, live alias and capacity checks. Both Mech workers reported zero final private payload bytes and owner records.

Run the packet’s replay.py to validate the preserved observations and regenerate both numerical reports. Apply backend.patch to the backend source in the verified R268 packet to reconstruct the candidate. The packet includes the exact build, emit, link, timing and profiling receipts.

[Retained R268 results and compiler closure](native-thread-after-growth-isolation.md).

[Verified screen and compiler patch](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r269-shared-vector-addresses-e3fd1a6b06ba.tar.gz). SHA-256 e3fd1a6b06bad7212ec0cbb610cbbe1f5e35d2663b03e35b29b81fba26057843.
