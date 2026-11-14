# Grouped histogram dispatch inside the phase

This report records the original descriptor and placement cohorts. The subsequent [shared-descriptor qualification](native-grouped-histogram-qualification.md) isolates the startup effect and verifies the retained compiler patch.

The phase-local variant preserves the [first pilot’s](native-grouped-histogram-pilot.md) random-input gain. Its default link misses the empty-input timing gate. The independently relinked control also misses the stored-case block gates. The equal-join compiler remains the retained build.

| Default link input | Candidate / control | Candidate / Roc | Candidate / libdeflate |
| --- | ---: | ---: | ---: |
| Random 1 MiB | **0.824345×** | **1.165650×** | **1.647776×** |
| Natural 8 KiB | 0.995178× | 1.023628× | 1.275006× |
| Natural 256 KiB | 1.001079× | 1.173319× | 0.949310× |
| Eight literals | 0.996935× | 0.171079× | 2.172707× |
| Repeated 65,537 bytes | 1.008639× | 0.571445× | 1.007463× |
| Empty | 1.086221× | 2.141769× | 0.016905× |

The repeated boundary case crosses from 0.998834× to 1.007463× libdeflate in the nine-case screen. The eight-literal result remains below the reference ceiling. Both peers, warmup, timers, allocator, input and output ownership, and the 1.05 gates use the existing comparison.

## Where the dispatch runs

The compact phase checks its current node and count spans before calling the grouped loop. The stored route bypasses both. The compiler requires the four view words to come from the phase entry snapshot. It refuses grouping across an iteration-boundary escape, a non-unit latch, incompatible load/store flags, another memory effect or a different checked cell. Entry checks use initialized lengths, checked byte extents and disjointness. A failed check enters the scalar body before a group can write.

The grouped bodies contain the same 800-byte code as the first pilot. The phase entry bodies grow from 596 to 720 bytes and keep their 192-byte frame. The root’s 956,596 bytes and 55,360-byte frame match the control Object exactly. The default link changes 3,470 root instruction words only in relocation immediates. The placement control gives the root and 1,269 other shared symbols the same addresses, leaving two tally entry addresses different. Its root has 179 differing relocation immediates. All 927 instructions reachable in the stored subgraph are identical in the linked placement pair. That inventory overapproximates the actual three runs.

| Separate cohort | Empty / control | One byte / control | Seven bytes / control | Random 1 MiB / control |
| --- | ---: | ---: | ---: | ---: |
| Default link | 1.086221× | 1.045094× | 1.044732× | 0.824345× |
| Matched root placement | 1.077264× | 1.055096× | 1.065305× | 0.822992× |
| Same placement pair, actor identities swapped | 1.105651× | 1.090340× | 1.063255× | 0.820742× |
| Identical executable in both actor slots | 1.007106× | 1.002069× | 0.991007× | — |

The swapped row reports the physical candidate divided by the physical control. The archive preserves the original actor labels and recomputes both raw gates. The three stored inputs each miss the block gate in the matched-placement comparison. Identical-executable controls pass all four cases. Root placement alone does not explain the small-case loss. The timing still includes the caller’s allocation, output transfer and cleanup, and this experiment does not isolate a particular remaining instruction or address.

## Counters, profiles and qualification

In the separate CPU cohort, random 1 MiB retires 481.98M to 458.80M instructions and takes 130.30M to 107.39M cycles. Lifetime physical peak changes from 50.92 MB to 50.96 MB. The empty candidate retires 97 more instructions and takes 5.9% more cycles in the instrumented worker. The counters and primary elapsed timings are separate cohorts.

The exact linked random candidate has 285 sampled PCs in the grouped tally and 3,833 in the matcher. The pinned peer binaries each have four write-prefetch instructions in their hot matcher symbols. Mech has none in the corresponding matcher. The next bounded experiment is one proved early bucket hint with a short address lifetime, compared against an equal-layout no-op. The rejected historical hint trial does not establish that benefit in this binary.

The exact source and revision-14 bytecode hashes match the first pilot. The compiler patch is applied to `467ef97a25a70c1e919d84d1ef408d50d616e143` under the same nightly release features and warnings-as-errors. Candidate Object SHA-256 starts `03f7edb2ba2e` and the linked worker `cb720a5c0244`. The four original linked workers passed 648 complete output, independent inflate, capacity, alias and cleanup checks. The focused JIT replay passed 51 differential and extent checks and nine refusals. The managed related workload passed all 88 output checks, with post-JSON ratios of 0.719931× and 0.733000× Rust. Its 13 generated bundle files match the retained bundle. Decoded compact emission and a second full matrix were skipped after the default-link gate miss.

[Patch, raw timing blocks, counters, profiles, assembly and replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-467ef97a25-grouped-histogram-local.tar.gz). SHA-256 `86e79b3dfcbd2ae092e220c4a530ebc88f7582ace69c1b968681c11ad15faf82`. Run `python3 -B verify-local.py` after unpacking.
