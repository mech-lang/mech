# Native compression: current status

**R151 is retained for draft review. Prior published heads remain frozen; parity is not achieved.**

| Checkpoint | Value |
|---|---|
| Published base | #981 at `89f2449cab6fd5b22e57f95c7d2f4006e2677453` |
| Evidence base | `054f5a5f82622d83d1da23d4b8d35b64ac6b9d50` (R149/R150 evidence only) |
| Before / retained compiler | `d9fd2855a2b3aef7a7f843f051466abd39f7ef25` / `dd3253562ee9d33451a66f4ef9b9808577dc4c1e` |
| Final focused test revision | `5da7d244be992c9d192369820c3bdc1ac803a898` |
| IR / work / ABI / bytecode | **175 / 107 / 39 / 14**; old IR173/174 rejected |
| Bounded change | Remove the second complementary case check after an exact current comparison chain |

One final 20-case cohort improves **0.373222% geometrically**: 13 faster / 7 slower medians, worst +0.772157%; every unchanged 1.05 guard passes. All **3,968 primary / 4,048 total** complete calls preserve exact bytes, independent inflate, aliases, prefixes, capacities and zero final managed storage. The development screen remains separate.

| Input | Current ms | Mech / Roc before→after | Change |
|---|---:|---:|---:|
|eight-literals|1.416397|5.0900×→5.0950×|+0.0972%|
|16384-distribution-change|6.271770|8.0253×→7.9895×|-0.4455%|
|33027-periodic-window|10.378709|8.3764×→8.3096×|-0.7971%|
|random-32769|7.935083|7.6979×→7.6593×|-0.5010%|
|repeated-32769|9.560500|7.2785×→7.2100×|-0.9407%|
|reymont-prefix-32769|35.147042|8.1868×→8.0928×|-1.1484%|
|reymont-prefix-16384|18.362479|8.7229×→8.6534×|-0.7972%|
|random-1024|1.246228|4.1867×→4.1541×|-0.7796%|
|repeated-1024|1.551020|4.7069×→4.6859×|-0.4480%|
|reymont-prefix-1024|1.904437|5.8889×→5.8240×|-1.1024%|
|320-overlap|1.349583|4.6829×→4.6861×|+0.0664%|
|random-256|1.324375|4.6659×→4.6551×|-0.2307%|
|128-biased-dynamic|1.141854|4.1564×→4.1222×|-0.8215%|
|random-64|1.323000|4.7015×→4.6749×|-0.5653%|
|32-high-entropy-stored|1.287354|4.5567×→4.5813×|+0.5402%|
|eight-overlap|1.405458|5.1396×→5.0556×|-1.6345%|
|eight-literals-prefix|1.419334|4.9594×→4.9977×|+0.7722%|
|seven-stored-prefix|0.125666|clock-limited reference|+0.7181%|
|one-stored|0.123104|clock-limited reference|+0.5275%|
|empty|0.120104|clock-limited reference|+0.0692%|

Hardware counters are a separate cohort. Instructions: Reymont 32,769 B **−2.424060%**, distribution **−0.917999%**, periodic **+0.654508%**, repeated **+0.723585%**. Distribution cycles are +0.783660%; all contrary observations remain. The local branch removal is not a universal instruction reduction. All 20 raw heap distributions (requests, bytes, reallocations, copy upper bounds and incremental usable peaks) and logical work match exactly. OS footprint/RSS remain separate.

Fresh four-case CPU-time profiles show **58.31–82.53% generated code / 16.21–37.97% host callbacks**; periodic is 73.56% / 24.32%, with 2.12% outside. These are time samples, not operation coverage or instruction attribution. This patch adds no lowering coverage.

The compiler proves a fresh comparison→empty literal case→empty complementary case chain, using existing descriptive Bool polarity facts plus its own current-visit proof. Only the second test becomes its existing success edge. Unknown input/joined predicates, actions and entry/exit/debit boundaries keep checks. Source sites, charges, checked arithmetic, errors, owners, aliases, publication and recovery remain. No new runtime cache or serialized permission.

Actual Object: three profiled duplicate branches disappear; the corresponding interval is **45→37 static instructions**. Across selected regions 1,305 cases fold statically, not a dynamic visit count. Generated code **8,848,408→8,726,056 B**; entry stack **12,544→12,192 B**. Source/MBC, algorithm/tables/depth/nice/EOF, real scratch sizes, source allocation sites, output capacity 13 with clear/reuse, allocator and complete evaluate_into timing remain pinned.

Qualification: **574 GPU pass / 7 same exclusions**; final focused 6 pass; 149 CI tests listed/equality-guarded/executed; workflow 132, architecture 15, runtime-only and formatting pass. Unchanged 236 components / 8 composition / 1 real-capacity path pass. All 138 GPU and 2 Core support sources match the parent. Scalar 12 pass (max 1.004688, 5 adverse medians); frame 8 pass. Nightly 2026-03-03/aarch64, warnings denied, library opt3 and unit/component roots opt1; Core/Engine/math/dependencies frozen and rehashed.

Exact **560 parent work/error observations** match: 304 existing reads + 256 Boolean cases. Coverage includes both literal orders, checked-add overflow before the branch, changing recursive arguments/windows, input/carried and merged predicates inside actual generated regions, original/decoded/direct/restored artifacts, finite denial, aliases, prior publication, cleanup and retry. Proof-only frontier mutations are not executable handoff claims. The broad suite ran at 8d758ae; the final metadata-only non-vacuity assertion passes the final focused run at 5da7d244be, with no execution change.

| Work board | Disposition |
|---|---|
| R151 comparison-case proof | Retained; local qualification complete, draft publication/review checkpoint |
| R149 exact-PC diagnosis / R150 geometry placement | Closed; R150 rejected and reverted because machine text did not change |
| Next compiler task | Select one causal runtime or generated-code cost from the fresh profile after review |
| Cold image validation | Inherited residual near 35 s remains open; no new matched startup cohort |
| Historical R140 eight-byte +1.322924% | Disposition retained; cross-round timing changes have no established cause |

The initial test-only borrow error is fixed and archived; no unresolved introduced functional failure. Same seven profile exclusions, full dependency/all-features rebuild, multi-MB timing and separate ten-workload suite remain skipped. Ordinary hosted CI is cancelled under standing instruction; **draft until green hosted CI and clean external review**. Exact commands, revisions, raw observations, failures and replay material are in the [experiment index](../../benchmarks/mech-deflate/experiments.json). No next implementation begins in this increment.
