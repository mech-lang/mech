# R57: rejected borrowed-field experiment

The candidate is rejected. Matched elapsed increased 1.759147%; none of four independently prepared pairs improved. All raw observations are retained. Production and its experimental tests were restored to published R56 by `a486856be`; the separate CI inventory was restored by `e22b0ea1a`.

| Metric | R56 baseline | Candidate |
|---|---:|---:|
| Elapsed (ms) | 3.8133545 | 3.880437 |
| Multiple of shared Roc | 13.1949983 | 13.4271176 |
| Retired instructions | 42995549 | 43252598.5 |
| CPU cycles | 12044217.5 | 12354610.5 |
| Allocation requests | 8773 | 8773 |
| Requested bytes | 98243843 | 98243843 |

This compares complete eight-byte level-12 compression only, with one fresh shared reference per round, four prepared pairs, 3,968 primary observations, and 16 excluded warm/profile observations. CPU (32 per executable), heap (8 per Mech executable;32 reference), complete allocation census and sampling are separate serial diagnostic cohorts. Allocation size/kind histograms agree exactly. Samples are not instruction counts or exact timer percentages; post-call RSS is not peak RSS. Requested capacity is not copied byte volume.

Original/decoded focused tests pass 2/2 on the candidate. Every compression observation preserves exact raw bytes `[75,76,74,78,73,77,75,207,0,0]`, independent inflate, input aliases/content, capacity13 reuse, work60,260,219,686 and final managed bytes/nodes0. The unchanged source/bytecode, allocator, tables and Object/image are pinned in archived bindings. Full component, architecture and scalar qualification was not run for this rejected candidate; no hosted pass is claimed. Initial compile/test failures remain in their original receipts and were corrected before the measured build.

The R57 implementation introduced extra projection stages and increased whole-call instructions. R58 restored one-pass borrowing through disjoint current-owner census inputs: it reduced instructions by0.224313% but still increased elapsed by0.459862% and cycles by1.373633%. These outcomes do not establish a net performance improvement. R56 remains the accepted baseline. No independent cohorts are multiplied or substituted.

`scorecard.json`/`.csv`, `profile-numbers.json`, `attribution-comparison.json` and all timestamped receipt archives provide the numerical evidence. Run `python3 reconstruct.py /private/tmp/replay-r57-new` from this directory to verify every archived version and reproduce the four numerical outputs without rebuilding or timing.
