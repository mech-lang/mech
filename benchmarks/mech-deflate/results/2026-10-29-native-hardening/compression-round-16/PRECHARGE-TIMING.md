# Aggregate metadata precharge: paired eight-byte timing

Base: `c6054fcf72be0a196d700899360283108a29ccb8`. Measured production compiler: `e982adbf6f7c060877bd253dbffe7b982dd9c67f`. Previous production control: `5cc14c28ccf786bceeb3ea0db7448530caed537c`; later control checkpoints contain only tests/evidence. PR parent remains `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`, draft PR #973, branch `codex/level12-compression-consolidated`. This immutable timing packet precedes any subsequent optimization.

| Complete eight-byte compression | Before | After |
| --- | ---: | ---: |
| Median evaluate_into | 40.872750 ms | 36.952583 ms |
| Multiple of the same fresh reference (Roc=1×) | 141.428201× | 127.863609× |

**9.591151% less elapsed; 1.106086× speedup.** Fresh pinned reference median 0.289000ms,20000 observations; three primary observations per Mech control. Warm-ups and the separate 109-repetition profile are excluded. Previous round's independently paired after result was142.091837×; the fresh control here is141.428201×. This is a small-case measurement, with no confidence interval or larger-input claim.

Every observation preserves exact ten-byte output, independent raw inflation, capacity13 and final zero bytes/nodes. Logical work is exactly303735923752 on both sides (delta0). Output backing still changes; copies remain timed. Same pinned mimalloc2.1.7 object, unchanged 25 components/source/bytecode/algorithm/tables/262144 U16 tree/1500513 U32 cache/depth300/nice258/EOF limits/source allocation sites/six Huffman initializers, and unchanged evaluate_into-only boundary. Mech compresses itself. No larger compression input ran. Runtime NoLimits and every allowance remain unchanged.

Ordinary dynamic aggregate closure now reuses the immutable SchemaTable layout summary through the existing budget helper. A successful same-current-account/real-turn preview batches exactly the old debit. Missing summaries, arithmetic overflow, denied previews and shared workers retain the original ordered traversal and spent prefix. Current records, selectors, owners, dimensions, payload copying and publication are not cached or changed. IR119/work93/ABI35/bytecode14 remain.

Before timing: warnings-as-errors selected-operation release passed (458.173492s),104 actual Engine budget/worker/projection/frame unit tests passed, and10 maintained dynamic projection/recursion tests passed (four projection plus six recursion cases). New differential tests compare default-meter results, errors, accumulated estimates and account settlement across every contribution's exact/one-short policy/account/call boundary, seeded work, overflow and shared workers. The release receipt records the precommit dirty source; its production contents exactly match the measured commit. Commands/features/dependency/source/binary hashes are retained. Twelve architecture/catalog/inventory/unsafe/warning/format checks passed; an initial new-test formatting discrepancy was corrected, with its original diff retained.

The fresh1348-sample profile partitions into479 other-helper,263 frame-action,239 ordinary-visit,227 selected-library and140 remaining exclusive weights. Frame admission is111 of those weights. The script reproduces the prior1347-sample partition exactly. These disjoint ancestry counts describe sampling, not exact removable times. Ordinary helper work remains the largest category and needs narrower attribution before another compiler change.

Broader component/scalar/Object/non-native-feature closure follows in a separate receipt commit. No further production optimization precedes that closure and review. Inherited unresolved round10 decoded-U64 scalar ratio1.0526036>1.05 remains explicit. CI is cancelled by user direction, not green. The full1× goal remains active at127.863609× on this input.
