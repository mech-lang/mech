# Complete M3 confirmation

The retained **64,512 B** word-scan encoder records **81/81 against Roc and 61/81 against libdeflate 1.26 level 12 in both complete orders**. It runs as `mech-control` in this campaign. Its native object, metadata, linked worker and runtime are identical to the [initial word-scan build](../README.md). The experimental entry-selected prefetch challenger is rejected.

| Order | Retained / Roc | Retained / libdeflate | Challenger / Roc | Challenger / libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Forward | 81/81 | 61/81 | 80/81 | 60/81 |
| Reverse | 81/81 | 61/81 | 81/81 | 57/81 |

The initial complete campaign recorded 60/81 against libdeflate in each order. No code changed in the retained executable between these campaigns. Near-tied cases change sides, so the additional net win is measurement variation rather than a further compiler optimization. This confirmation meets the M3 comparison target; it does not claim the separate v3 promotion floor of 62/81 in both orders. [Cases that changed sides](near-ties.json)

[Every case: raw seconds and paired comparator multiples](timings.csv) · [Original campaign counts, both actors](results.json) · [Selected binary identities](selected-actor.json) · [Plan](plan.json) · [Raw-record replay](raw-replay.json)

Each unchanged peer runs serially in 96 balanced blocks per case. The existing in-memory diagnostic boundary includes native execution, heap construction, final allocation transfer and physical private cleanup. Preparation, output validation, reporting and process startup are outside that interval. Multiples are medians of paired observations; values below one are faster. [Environment](environment.json) · [Contract](contract.json)
