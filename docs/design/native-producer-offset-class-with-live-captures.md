# R332 — Measure producer classification with live captures

**Keep R327 with the R272 runtime.** The corrected producer-time pilot restores all four native parser phases. On 256 KiB natural text the matched comparison changes +1.168% and the primary screen changes +1.102%. Matched ratios move from 1.312091× to 1.327417× Roc and 1.056715× to 1.069059× libdeflate.

Measured compiler ba8f02f61b3735a2160cc6177e76c6f7cc59cf7b. Published parent c5e7595c7c64619d7ed963bc2f5ad0fc2c892cf9. PR base 92f75db84e2523693648084bea16cd52ad60777a. Same backend and runtime, IR/work/ABI/bytecode 215/107/39/14. The source experiment runs through the compact diagnostic entry.

## Representation and restored admission

The two existing match writers add the offset class to five spare bits. Lengths through 258 fit in the remaining nine low bits, with the original offset in the high half. Headers and the carryover copy retain their packing and geometry. Each read uses the current cost table for the saved class. There is no additional cache array. The unchanged 1,500,513-word cache, first-winner order, match limits and writer remain part of the complete compression.

R331 kept an unused slot table in the recursive parser arguments. Numeric proof preparation requires one operation witness per carried array. That proof declined all four parser copies. This witness removes only the forwarded slot table from the three parser signatures. All four no-growth bodies reappear. The inspected parser shrinks from 1,552 to 1,504 code bytes and 256 to 224 stack bytes compared with R327. The linked assembly records the cost-class extraction and current cost load. It uses a right shift and five-bit mask, and removes the parser’s byte-table load. The matcher grows from 3,068 to 3,126 static instructions and 543 to 548 static stack reads.

The separate 256 KiB CPU cohort changes instructions -3.205% and cycles -0.071%. The restored parser has 7 of 5,357 running samples. The matcher has 1,940. The profiles include validation. All six heap counters agree on 16/16 cases.

| Input | R327 / Roc | R332 / Roc | R327 / libdeflate | R332 / libdeflate | Primary elapsed change |
|---|---:|---:|---:|---:|---:|
| empty | 2.132521× | 2.128202× | 0.016812× | 0.016778× | +0.000% |
| one-stored | 1.994152× | 2.061236× | 0.017774× | 0.018372× | +0.000% |
| seven-stored-prefix | 2.194868× | 2.301121× | 0.018411× | 0.019302× | +0.000% |
| eight-literals | 0.182872× | 0.182730× | 2.578410× | 2.576401× | -0.080% |
| reymont-prefix-32769 | 1.294220× | 1.308747× | 1.174345× | 1.187527× | +1.176% |
| heldout-window-32768 | 1.298733× | 1.307747× | 1.179688× | 1.187875× | +1.381% |
| prefix-natural-32767 | 1.289623× | 1.304756× | 1.170923× | 1.184663× | +1.114% |
| prefix-natural-32768 | 1.297643× | 1.311937× | 1.174710× | 1.187649× | +1.112% |
| prefix-natural-65537 | 1.313133× | 1.312556× | 1.104888× | 1.104402× | +0.950% |
| prefix-natural-131072 | 1.304115× | 1.315870× | 1.072652× | 1.082321× | +0.995% |
| prefix-natural-262144 | 1.312091× | 1.327417× | 1.056715× | 1.069059× | +1.102% |
| prefix-natural-1048576 | 1.312319× | 1.332374× | 1.063864× | 1.080121× | +1.533% |
| prefix-random-16384 | 1.068097× | 1.061985× | 2.858942× | 2.842583× | -0.890% |
| prefix-random-1048576 | 1.595588× | 1.589130× | 2.256191× | 2.247060× | -0.075% |
| prefix-periodic-1048576 | 0.708248× | 0.711270× | 1.201931× | 1.207058× | +0.872% |
| prefix-repeated-1048576 | 0.710946× | 0.711547× | 1.118221× | 1.119167× | -0.047% |

154,752 balanced timed calls. The pooled and block gates pass 16/16 and 16/16. Original and decoded compiler output and metadata are byte-identical. All 324 linked calls match the reference stream, inflate independently, preserve aliases and capacities, and clean up. The original and decoded class witness separately checks 65,536 actual generated outputs each against the original slot table and wrapping expression.

## Eight-literal allocation attribution on retained R327

The diagnostic reconciles 111 requests at 93 call-site and size groups and 9,024,420 requested bytes in every call. Separate clocked backing and owner windows total a median 12.92 µs across 24 samples. Allocation and release window medians are 4.08 and 6.02 µs. The two mapped numeric buffers account for about 4.8 µs of backing release. Both coexist with the path buffer for a median 58.9 µs. The trace clocks helper windows. Later page faults and output deallocation are outside those windows.

R330 showed the cost of repeated threshold and bounds checks in the source composition. The next bounded implementation can use one fixed prepared cost table in the native phase, one index selection, and a single proved range. Setup belongs inside complete-call timing. Keep the ordinary phase as the sparse and resource-refusal route.

[Verified patch, linked machine code and pilot observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r332-produced-match-class-live-2789c51d2678.tar.gz). SHA-256 2789c51d2678f88d56f8e7b6f6ef8f5831cf4905c283dc6ae78e5a7737e10905.
