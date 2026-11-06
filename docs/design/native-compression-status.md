# Native compression: current status

**R147 is closed for draft review. The earlier milestone, its five published PR heads and the R146 closure branch remain frozen. No implementation task is active.**

| Checkpoint | Value |
|---|---|
| Base | `ab8cfed2524c2183414c122531e122863c5a7c42`; R146 documentation-only closure |
| Before / retained compiler | `c03bdfc7ffca8ffdf4f2207729536d8b4a5d8dbd` / `c4759eb2f5b26e8e883cdc317b57cdc7a218ba8d` |
| IR/work/ABI/bytecode | **173/107/39/14**; prior emission revisions rejected strictly |
| Bounded change | Existing native parent pieces cover ordinary numeric-phase descendants at their full source price; exact cold child markers restored |
| Frozen stack | [#973](https://github.com/mech-lang/mech/pull/973) → [#976](https://github.com/mech-lang/mech/pull/976) → [#977](https://github.com/mech-lang/mech/pull/977) → [#978](https://github.com/mech-lang/mech/pull/978) → [#979](https://github.com/mech-lang/mech/pull/979) |

One final cohort of 20 cases: geometric after/before **0.969855561036 (-3.014444%)**; 15 improving / 5 adverse medians. Periodic improves; every unchanged 1.05 guard passes. All 3,968 primary / 4,048 total calls preserve exact raw bytes, independent inflate, aliases, prefixes, capacities/clear-reuse and zero final managed data. Source/MBC, algorithm/tables/depth/nice/EOF, allocation sites/real scratch sizes, allocator and complete `evaluate_into` timing remain pinned. Development is separate; no pooling or guard repeat.

| Input | Before→after ms | Mech / Roc before→after | Change |
|---|---:|---:|---:|
|eight-literals|1.358855→1.342666|4.7679×→4.7111×|-1.1913%|
|16384-distribution-change|6.236499→6.281604|7.8943×→7.9514×|+0.7232%|
|33027-periodic-window|21.519979→12.967520|17.1406×→10.3286×|-39.7419%|
|random-32769|8.014458→8.141313|7.4380×→7.5557×|+1.5828%|
|repeated-32769|9.570583→9.607417|7.2068×→7.2345×|+0.3849%|
|reymont-prefix-32769|35.399062→35.236354|8.1452×→8.1078×|-0.4596%|
|reymont-prefix-16384|18.501312→18.425125|8.7229×→8.6870×|-0.4118%|
|random-1024|1.305625→1.274750|4.2667×→4.1658×|-2.3648%|
|repeated-1024|1.588312→1.585813|4.9480×→4.9402×|-0.1574%|
|reymont-prefix-1024|1.929021→1.910292|5.7929×→5.7366×|-0.9709%|
|320-overlap|1.359229→1.350104|4.5920×→4.5612×|-0.6713%|
|random-256|1.336416→1.314438|4.4997×→4.4257×|-1.6446%|
|128-biased-dynamic|1.146271→1.134125|4.0220×→3.9794×|-1.0596%|
|random-64|1.273416→1.263458|4.4370×→4.4023×|-0.7820%|
|32-high-entropy-stored|1.266521→1.246958|4.4596×→4.3907×|-1.5446%|
|eight-overlap|1.321250→1.315917|4.6037×→4.5851×|-0.4037%|
|eight-literals-prefix|1.366000→1.348042|4.7762×→4.7134×|-1.3147%|
|seven-stored-prefix|0.121355→0.121062|clock-limited reference|-0.2406%|
|one-stored|0.121000→0.121041|clock-limited reference|+0.0343%|
|empty|0.122292→0.122333|clock-limited reference|+0.0335%|

Five adverse medians are retained: distribution 16 KiB +0.723234%, random 32,769 bytes +1.582821%, repeated 32,769 bytes +0.384856%, one-byte +0.034298%, empty +0.033526%. Matched ratios support this change; cross-round absolute differences have no established cause.

Separate counters: periodic 380.158221M→225.340706M instructions (−40.724495%) and cycles −40.018678%; 8 bytes, instructions +0.027670%. All 20 allocation counts/reallocations/copy upper bounds/incremental usable peaks match; requested bytes rise 112 bytes/call. Logical-work totals differ under unchanged selected-representation formulas and are archived per side; they are not hardware instructions. Text 7,888,192→8,848,408 B; stack 7,504→12,544 B. No memory or code-size reduction claim.

Actual diagnostic: three periodic calls show 90 target sites, 89,280→0 ordinary callbacks, and 16 U8 read sites, 15,872→0; all ordinary source callbacks 177,743→13,048 per call. New segment 2735 admits 1,031 times at the full compute charge of 476,640 under the ancestor token; 39 paths do not reach the reads. Fourteen of 48 new static pieces actually admit. All 188 prior entries/source inventories and 27 word packs remain. The actual Object contains 16 target byte loads; no segment word-pack fusion was added.

Current four-case actual-binary CPU-time profiles show 58.50–82.72% generated-exclusive weights. Periodic is 59.11% generated / 34.72% other callbacks; these are sample weights, not instruction shares. Primary, counters, heap and sampling cohorts remain separate.

Qualification: GPU 570 pass / 7 same exclusions; CI 145 listed/guarded/executed; 132 workflow tests; 15 architecture checks; runtime-only/formatting pass. New focused 5 and 24 existing targeted controls pass. Original/decoded/direct/restored execution covers current owners/arguments, paid-overlap refusal, full unpaid charge, error order, aliases/publication/cleanup/retry. Finite ChildHeader restores the exact marker and consumes ordinary child return, then later Work denies; no whole-program finite success claim. Unsupported mode 6 grandchild witness is excluded.

Unchanged source-derived 236 components / 8 composition / 1 real-capacity path pass; 138 support files match the parent. Scalar 12 pass unchanged 1.05 gate (five adverse, maximum 1.005012675759); frame 8 pass request/byte/work controls. Warnings remain errors. Core/Engine/math/dependencies stay frozen; only GPU / required roots rebuilt. Initial obsolete selection-test assertions, state-variant eligibility and a test-only type mismatch were corrected; all raw failures are retained. Existing semantic/resource/argument-bank expectations and component allowances were preserved. Read-only review found no blocking correctness or architecture issue.

| Work board | Disposition |
|---|---|
| Completed | R130 reconciliation and native capabilities through the prior frozen milestone; R147 bounded descendant pieces closed |
| Rejected/preserved | R131 snapshots, R141 early refusal, R143 stores; contrary evidence remains archived |
| Active | None; no further capability enters this increment |
| Next | Integration review; separately assigned image-reconstruction usability follow-up |
| Deferred | Further generated-state/instruction work, child funding, source rewrites, ownership/allocation/SIMD |

Known costs remain explicit: R140 introduced 8 bytes +1.322924% is a dispositioned historical regression. R136 cold image admission rose roughly 17 s→35–37 s; R142 reduced its matched median 1.426206% to 34.580817458 s, leaving a separate usability limitation. This round has no new matched startup cohort. Code/stack growth above is accepted for the measured bounded gain, not hidden by compression-only timing.

No unresolved introduced functional failure. Seven inherited profile exclusions, full dependency/all-features graph, matched startup remeasurement, multi-MB timing and the separate ten-workload suite remain skipped. Ordinary hosted CI is cancelled under standing instruction; **draft until green hosted CI and clean external review**. Exact final sources/commands/pins/raw observations and failure dispositions are in the [experiment index](../../benchmarks/mech-deflate/experiments.json), R147. Some early uncommitted preparatory witness variants retain logs/commands rather than complete standalone source snapshots; no acceptance or performance claim depends on them.

Review follows the [current architecture](native-compiler-architecture.md). Large evidence stays external; current status and architecture describe the maintained system. The next optimization requires a separate bounded task after review. Performance parity remains the longer-term objective.
