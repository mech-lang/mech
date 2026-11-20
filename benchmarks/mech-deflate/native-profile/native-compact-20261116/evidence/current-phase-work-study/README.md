# Current work attribution and the 64th-win investigation

**The selected build remains 63,804 B, 81/81 Roc and 63/81 libdeflate. The next milestone is 64 C wins, preserving every existing named win.** Both complete 96-block orders support the selected result. The eventual runtime target remains 81/81 C. [Selected recipe, complete builds and qualification](../encoder-size-gap/packed-word-packet-selection/README.md).

This study adds 45 current owner profiles, exact parsing-work comparisons on nine cases, C search counters, 212 real Huffman frequency vectors, current Mech matcher block counts, and the completed bounded cost-carry experiment. It does not promote that experiment or claim a 64th win. Original Mech source and canonical bytecode are unchanged.

## Equal parsing work, measured in both implementations

Separate diagnostic workers count actual executed work. The Mech counters cover the selected scorer before block scheduling, and the optional carried-cost scorer separately. Their generation receipts differ from the corresponding timed recipes only in the scorer's block-layout profile. Scalar visits count one length; SIMD visits count their actual four/eight candidate lanes. C counters come from a separate copy of pinned libdeflate `92e6a0db9fa848d742f9eb286c92afc60f2c3dda`, at level 12.

**All five measured quantities match exactly for all nine cases in both Mech bodies:** parsing passes, DP-node updates, literal-only positions, consumed match records and candidate-length evaluations.

| Input | Passes, each actor | Node updates, each actor | Literal-only positions, each actor | Match records, each actor | Candidate lengths, each actor |
| --- | ---: | ---: | ---: | ---: | ---: |
| Natural 1 KiB | 4 | 4,096 | 1,940 | 4,348 | 22,836 |
| Natural 4 KiB | 5 | 20,480 | 4,375 | 35,565 | 199,335 |
| Natural 8 KiB | 6 | 49,152 | 9,600 | 86,904 | 381,696 |
| Random 4 KiB | 3 | 12,288 | 12,285 | 3 | 3 |
| Random 16 KiB | 3 | 49,152 | 49,116 | 36 | 36 |
| Random 64 KiB | 3 | 196,608 | 196,320 | 288 | 288 |
| Random 1 MiB | 11 | 2,997,152 | 2,991,660 | 5,517 | 5,517 |
| Distribution change, 16 KiB | 9 | 73,682 | 50,637 | 25,228 | 25,363 |
| Overlap, 320 B | 4 | 1,280 | 1,272 | 16 | 1,256 |

These totals rule out extra parsing passes or candidate-length enumeration as the explanation for these cases. They do not prove equal matcher, Huffman, setup or output work. C pass purposes are captured; the Mech comparison currently aligns totals, not every pass's purpose. [Portable derivation](align-dp-work.py), [data and scope](summary.json).

Every C diagnostic output matches the pinned uninstrumented worker's digest and independently inflates. Each DP pass additionally checks that its chosen literals plus matched bytes cover exactly its block length. Mech diagnostic workers retain the existing exact-output, inflation and alias checks.

## Current owners and matcher work

The five current actors are the histogram reference, selected packet/narrow-word recipe, sequential packet candidate, carried-cost packet candidate and pinned C. Each has a fresh three-second capture on each of the nine inputs above: 45 captures total, with exact worker/sample identities and two verified calls per capture.

On random 1 MiB, the selected Mech encoder tree has **72.9% matcher / 15.0% scorer** exclusive samples. The carried-cost candidate has **75.4% / 12.3%**. These current profiles support broadening the investigation beyond the scorer.

**The roots have different scopes.** C's near-optimal body includes inlined matching, and its scorer includes histogram/Huffman orchestration. Construction, transfer and cleanup outside these roots are not fully represented. Sample shares are not aligned phase elapsed times, and are not multiplied by whole-call medians to predict gains.

The current admitted Mech matcher kernel executes **204,859,867 instructions and 21,120,522 stack loads on random 1 MiB**, across 35 calls. Its hottest blocks repeatedly reload table hashes, bases and collector state. These counts cover the actual unchanged selected matcher body; checked controller fallback is outside this kernel. C search/tree counters are also captured, but comparable total Mech search/skip/tree work still needs mapping. No equality or causal speed estimate is asserted for those phases.

The allocation counters retain their actual meaning: `zeroed_bytes` is requested initialized extent, not physical clearing or measured first-touch traffic.

## Completed cost-carry experiment

Compiler source [`89dd67efb`](https://github.com/mech-lang/mech/commit/89dd67efb617e44e72aaa0710101e566661e8600) adds the opt-in `MECH_FOLD_NARROW_PACKED_WORDS=phis` proof. It carries the separately proved low cost through joins while retaining full packed stores. The source, native fixtures, four configurations of 22 optimized tests, 57 exact older-recipe replays, two candidates' output/error/recovery/stack gates and actual assembly difference are included.

The packet candidate retains **63,804 B** text and **4,736 B** stack. Instruction/branch counts and stack traffic are unchanged; the literal-cost dependency changes in the generated code. Two complete discovery orders use 21 cases and 24 balanced blocks, with the exact selected worker as control.

| Input | Candidate time / selected, forward / reverse | Candidate time / C, forward / reverse |
| --- | ---: | ---: |
| Natural 1 KiB | 0.996420 / 1.000711 | 1.236122 / 1.238602 |
| Random 1 MiB | **0.974945 / 0.975790** | 1.253263 / 1.254482 |
| Distribution change | **0.989551 / 0.979760** | 1.266384 / 1.266998 |

The direct control ratios are medians of per-block candidate/control ratios from the raw observations. They are not quotients of two peer-ratio medians. All observed control wins survive; no 64th C win appears. This candidate has not received full 81-case qualification or a new complete-build cohort, so the selected recipe stays unchanged. Its selected flag-1 counterpart's matched source-through-final-link build remains **2.222473 s**, paired **0.742624× Roc / 4.538331× C**.

## Verification and continuation

```sh
python3 benchmarks/mech-deflate/native-profile/native-compact-20261116/evidence/current-phase-work-study/verify.py
```

The portable verifier checks every archived member hash, reclassifies all 45 raw profiles, replays both nine-case DP alignments, checks C trace invariants and validates every saved discovery observation. [Verification receipt](verification.json).

The archives contain original producer commands/receipts, generated diagnostic source, counters, all per-block timing observations, frequency vectors, nine input/prefix/expected fixtures and the proof annotations. The frequent counters run only in separate diagnostic workers; their elapsed times are not speed results. The failed LLDB attempt is recorded as a noninteractive macOS debugger-permission limitation; source/binary diagnostic counters completed the work without it.

To rebuild the C diagnostic on an Apple AArch64 machine, extract `c-semantic-traces.tar.gz` and `current-owner-captures.tar.gz` into one directory. Run from that directory so the staged fixture paths resolve. `SOURCE_PINS` checks the original translation units before making local diagnostic copies:

```sh
python3 /path/to/trace-libdeflate.py \
  --libdeflate-root /path/to/libdeflate-92e6a0db9fa848d742f9eb286c92afc60f2c3dda \
  --fixtures fixtures.json --current-profiles owners --output rebuilt-c-traces
```

The next work is comparable matcher search/skip/tree/prefix counts, setup/mapping/retirement attribution, Mech frequency capture, and identical-input C/C-exact-Mech/Mech phase replay. Use those results to choose a targeted ordinary-Mech source experiment and a compiler alternative. Coarse aligned phase elapsed times remain outstanding. Size and complete-build work also remain separate: the checked entry is still larger than C's entire encoder. The 64th runtime win has not yet been achieved.
