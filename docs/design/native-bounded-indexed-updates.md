# R305 — Measure bounded indexed updates at the phase entry

**Retain R304 with the R272 runtime.** The candidate shortens the selected literal update and changes paired elapsed -0.498% across the 27-case screen. All 27 pooled gates pass. The empty, one-byte and seven-byte cases miss the block gate. R304 supplies the retained score of 45/81 faster than Roc and the 3/81 matched libdeflate wins.

Measured and published parent 9283b56817d59042eb73420d0e05a3bc9ec2216f. PR base 92f75db84e2523693648084bea16cd52ad60777a. Compiler source base 8830db78789aee692d89583d25458bcc32f300b3 with the R304 closure. Apple ARM64, Cranelift 0.131.3, regalloc2 0.15.2, IR/work/ABI/bytecode 215/107/39/14. All runs use the pinned level-12 source and existing complete-call timing boundary.

## Compiler rule and linked evidence

A complete fixed-view numeric loop provides a current changing read-modify-write. The shared index analysis proves that its variable one-based index fits a finite nonzero interval. The compact caller adds that minimum length to its existing current flat-view admission after ownership and capacity. A shorter view uses the original root execution before any update. The same length fact lets the current traversal prove the zero-based read comparison after exact frame and scalar moves. The native store keeps its checked origin on every route outside the proven range.

Three loop headers select a floor of 256 for their current frequency view. The literal source selects an index from 0 through 255 and does a wrapping U32 increment. The match route keeps its dynamic length and offset table checks. The guarded writer uses the same typed address and store.

Seven of 347 bodies change. The six tally bodies go from 704 to 648 code bytes and have 192-byte frames. The root goes from 998,904 to 998,460 bytes with a 57,104-byte frame in both builds. The inspected literal update goes from 13 instructions to six. The case-specific measurements below show the effect on the whole call.

The matched Roc literal arm uses a bitfield placement and one load, add and store after extracting the packed literal. The pinned C packed-item arm forms the offset into its fixed frequency table and joins the shared load, add and store. C has a separate all-literal input byte loop. The safe Rust witness admits a 256-element slice once and uses a bitfield extract plus the same load, add and store. These are local assembly excerpts.

| Short input | Median block mean change | Pooled gate | Block gate |
|---|---:|---|---|
| empty | +10.239% | pass | miss |
| one-stored | +7.110% | pass | miss |
| seven-stored-prefix | +7.708% | pass | miss |

## Matched timing and counters

| Input | R304 / Roc | R305 / Roc | R304 / libdeflate | R305 / libdeflate | Paired screen change | Instruction change |
|---|---:|---:|---:|---:|---:|---:|
| eight-literals | 0.194324× | 0.195043× | 2.479308× | 2.488492× | +0.111% | -0.161% |
| prefix-natural-4096 | 1.009228× | 1.007239× | 1.705309× | 1.701949× | -1.089% | -0.139% |
| prefix-natural-1048576 | 1.403687× | 1.403809× | 1.133066× | 1.133165× | -0.149% | -0.021% |
| prefix-random-1048576 | 1.697277× | 1.650357× | 2.401835× | 2.335439× | -2.600% | -3.269% |
| prefix-periodic-1048576 | 0.740188× | 0.740009× | 1.256983× | 1.256678× | -0.103% | -0.040% |
| prefix-repeated-1048576 | 0.741060× | 0.740315× | 1.166111× | 1.164939× | -0.085% | -0.097% |

The reference ratios use the ten-case four-actor cohort with all 24 balanced orders. The screen change uses the original three-actor screen. Counter and heap results come from separate ten-case cohorts. All six tracked payload families agree for all ten.

## Qualification and profiles

- Both artifact forms have the same Object and metadata and pass 324 complete calls across all 81 fixtures. Output streams match exactly and independently inflate. Input and held output aliases pass, with zero final private owners.
- The 36 linked controls cover the exact floor, values on both sides of it, U32 wrap, zero trips, a later default read, a late bounds failure, lexical failure first and repeated recovery. The observer confirms that the 255-element case falls back before an update. The retained and candidate forms match the independent oracle, status and reached origin. Prior outputs and observed allocation families agree.
- The first proof removed only the store check. The final traversal proof removes the literal read branch as well. The first witness used a document assertion that the compact adapter rejects. The corrected function-local assertion runs in all three linked forms.

| Input | Running samples before / after | Literal tally before / after |
|---|---:|---:|
| eight-literals | 5461 / 5595 | 1 / 7 |
| prefix-natural-4096 | 5566 / 5407 | 102 / 85 |
| prefix-random-1048576 | 5412 / 5362 | 1042 / 946 |
| prefix-repeated-1048576 | 5485 / 5358 | 9 / 12 |

These samples describe running locations in each exact executable, including the harness and validation. The raw block data preserve all three short-input misses.

Put the same bound check in the outlined numeric function. Its successful branch uses the current length proof and the other branch calls the existing checked body. Qualify the unchanged root machine body before screening and retest the three short inputs along with the full performance frontier.

[Verified compiler patch, timing screen and profiles](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r305-tally-entry-bounds-dc859a429827.tar.gz). SHA-256 dc859a429827d4c60c0ae3fd03948a38408b495004b49e90ee41c5b2b32a24d1.
