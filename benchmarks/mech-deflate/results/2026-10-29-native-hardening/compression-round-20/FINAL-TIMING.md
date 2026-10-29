# Fixed nested private-frame composition: final timing

Previous published head96721e45d454f507595c54e445d18266eff05ced. Initial productione721febbcd557040ce8866e2ebbf61f09a9e4b5b; final production **aed697ad21ba1ffcf07da89b90c4c33571cf6ef2**. IR123/work93/ABI35/bytecode14. The introduced mixed canonical/private geometry rejection is now a constructor fallback, with eight passing original/decoded focused tests. No production source changed during the successful443.759s selected release build or measurement.

**Roc=1×: 111.196680070× → 102.707533217×.** Mech medians **31.802250ms → 29.374354ms**; same fresh pinned reference **0.286000ms**(20000 observations). **7.634353% less elapsed; 1.082654× speedup.** This is a modest observed whole-program gain. Parity and larger-input throughput remain unestablished.

The initial separately committed series was106.602389×→99.868246×, using its fresh reference0.293ms and after median29.261396ms. This final series uses0.286ms and after median29.374354ms. They are separate paired experiments, not a continuous normalized comparison or an isolated timing of the fallback repair. Both are retained; the earlier result is not relabeled as the fixed compiler.

Fixed four alternating-order pairs, three primary observations per control/process, twelve per control. All40 process observations remain:24 primary,8 warm-ups,8 excluded profile-tagged. The separate137-repetition sampled process is excluded. Same pinned mimalloc2.1.7 and evaluate_into-only harness. No compiler/test/other benchmark overlapped primary. JIT/setup/seed/clear/rebind, validation and drop stay excluded.

|Pair|Order|Before ms|After ms|After/before|
|---|---|---:|---:|---:|
|1|before→after|32.362625|29.083666|0.898680685|
|2|after→before|30.694416|29.583708|0.963814004|
|3|before→after|32.514417|29.228500|0.898939692|
|4|after→before|31.078667|29.753042|0.957346144|

Every primary/profile observation agrees with the exact10-byte reference output, independent inflation, capacity13 and cleanup0 bytes/nodes. Output backing changes; those copies remain timed. Work307973385655→306669833472. Existing prices/allowances remain; executed routes and actual current capacities determine debits. No equal-debit claim.

Unchanged source/bytecode, algorithm/tables,262144-entry U16 tree,1500513-word U32 cache,depth300/nice258,near-EOF rules,source allocation sites and timing boundaries. Mech performs compression itself. Largest completed input8 bytes. No large/full-input run. Exact commands, source/artifact/dependency/binary hashes, all observations and the fresh executable sample accompany this packet, committed before further qualification or optimization.
