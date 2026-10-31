# Shared numeric scratch storage — round 51

**Retain.** Complete eight-byte level-12 initial timing improves **13.799399× → 13.774699× Roc (−0.17899%)**; independent longer confirmation **14.000365× → 13.915864× (−0.60357%)**. Each cohort uses one shared fresh reference for before/after, respectively 0.291 and 0.286 ms. Seven of eight prepared pairs improve. This is a small gain, not parity. Preserve both cohorts and the **0.59658% instruction increase**; do not multiply separate rounds or choose only confirmation. [Full numerical profile](PROFILE.md).

## Exact checkpoint

Published parent: `f81cbfdc67cc591ed54bd861d63d693c005706ad`; stack base `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`. Branch `codex/level12-compression-consolidated`, worktree `/private/tmp/mech-deflate-linear-stack`, draft PR #973.

Tests-first corrected fixture: `e980f82f1aa5eb4987899a37dd8a7db625040465`. Production: `526024b4a4c1480deec23f46aa18603de45d3b16`. Separate CI inventory: `e0c180a47fdeeabada8b6d8df0c57f3f5fdc3d6c`. Exact optimized GPU source: **e0c180a47fdeeabada8b6d8df0c57f3f5fdc3d6c**, built in **22.324 seconds** with retained opt3/feature/warnings-as-errors flags. All other seven direct libraries and their common/transitive dependency graph are frozen. Later code changes are test-only: `fa2cca2519011b50dbcdfc0080a3cb1eb33e2c21` relocates unsafe image diagnostics into the maintained test boundary without widening the allowlist. The exact diff is archived.

**IR/work/ABI/bytecode change 131/95/37/14 → 132/95/37/14.** IR132 versions the changed private word homes and owned-image fields. The external ABI and canonical source bytecode remain unchanged. Independently rederived image equality and the existing previous-version identity rejection tests retain strict boundaries; no fixture revision or general codec migration is bundled.

## Mechanism and lifetime proof

The unchanged compressor has 239 child-free numeric regions. Original independent shadows total 5,491 words while the largest needs 49. Closed-region admission rejects a second active loan; a native child contains no child loop. Parent segments reconcile/end before child entry and retain their own separate bank. No ordinary helper can enter another numeric region during an active loan. Materialization settles/resumes the same region. Thus numeric scratch lifetimes cannot overlap.

Assign an explicit shared numeric shadow base after all region state words. Keep all per-region readiness/fuel words private: global abort visits every region, so sharing the flags would let an inactive region export another region's carrier. Entry, SSA word selection, materialization, terminal/failed export and image reconstruction all use the explicit base. Non-numeric map/scalar regions retain their own control words; parent-segment scratch is not shared in this pilot. No canonical arena compaction, logical value-ID reuse, epoch reuse, new executor, dirty-slot tracking, source-ID rule or user-visible source rewrite is added.

Every entry refreshes its current scalar/array views; source definitions still dominate scratch use. An inactive region's stale scratch is never exported because its own readiness word is clear. Cold paths retain exact assertions/load/arithmetic precedence, current carriers, original finite fuel, failure cleanup, unchanged prior publication and retry. Original/decoded tests exercise two sequential regions with different array contents, current Booleans, counts and seeds; false assertions before invalid reads, actual invalid reads, arithmetic overflow, unselected loads, repeated fresh invocations and aliases. Existing real finite-fuel/mixed-result/write tests all pass. The image test independently restores the valid bank and rejects altered scratch/state offsets.

Fresh same-build descriptors and actual allocation receipts establish:

| Storage | Before | After |
| --- | ---: | ---: |
| Numeric regions | 239 | 239 |
| Physical numeric scratch words | 5,491 | **49** |
| Disjoint readiness/fuel words | 717 | **717** |
| Physical closed-tail words | 6,208 | **766** |
| Original logical closed-tail words | 6,208 | **6,208** |
| Direct native word-bank request | 483,344 B | **439,808 B** |

Physical bank savings are **43,536 bytes**. `Layout.logical_words` preserves the old independent-bank count for setup work and combined transport admission. `SetupGeometry` allocates compact physical words but charges the original logical count and keeps the original combined transport threshold. Actual memory demand falls, which can allow a smaller physical budget; it is not cached permission or renewed fuel. Owner-word rebasing is linear, so the same saved-word correction preserves the original logical envelope with map owners too. The exact real-compressor total remains **60,260,219,686** and comparison **47,612**.

## Measured outcome

Initial: **4.015625 → 4.0084375 ms**, 3,968 primary calls, three of four pairs / 41 of 64 descriptive blocks improve. Independent longer confirmation: **4.0041045 → 3.979937 ms**, 8,064 primary calls, all four pairs / 55 of 64 blocks improve. Warmup/profile calls add 32 observations excluded from these counts. Both cohorts and every raw block remain committed immediately; blocks within a pair are descriptive, not independent experiments.

Separate medians: instructions **45,224,746 → 45,494,546 (+0.59658%)**; cycles **12,702,555 → 12,680,797 (−0.17129%)**. Allocation requests stay **13,484**; requested bytes **98,529,723 → 98,480,739 (−48,984)**. Other large requests retain their histogram; no complete copy-volume claim. Prepared tracked heap stays **142,982,576**; additional peak **20,575,320 → 20,517,976**; peak **163,557,896 → 163,500,552 (−57,344)**. Post-call OS memory rises slightly and is not peak RSS. The retained harness/catalog makes this different from minimum standalone runtime RAM. No branch/cache-miss attribution is made.

Actual emitted text shrinks **13,885,696 → 13,046,668 bytes**, stack **108,400 → 108,368 bytes**. New Object SHA-256 `1a8bb46e9f78fec2147b3242fab5651e470f86ba0347f4f11c7f1aedd5008777`; image `1b4dbdd2fb0c1f9f51ae7e9dd0bc931bbc2804e6cb949653461d6348c31615c9`. Both new payloads are committed losslessly in compiled-artifact, with raw/compressed hashes, source and configurations. Release-symbol checks exclude diagnostic and test-only hooks.

## Qualification and failure disposition

**599 fresh distinct Rust tests pass**: 322 runtime, 32 work/memory/value, two preparation, 234 component, eight connected and one path mechanism. Focused three/six/27-case runs overlap these totals. The unchanged Core retains its prior **413-test** qualification, production SHA `d108019406335c7627061c0bf46cfda6e96adfc05fb97dae652499f1bbcbb873`; 1,012 identified covered tests are not 1,012 newly executed tests.

All **12 linked original/decoded failure/recovery executions**, 12 scalar controls with the original ≤1.05 latency gate and exact pure-scalar allocation equality, eight frame-allocation cases with original zero slope/no request-byte-work increase, 132 Python workflow/inventory tests and ten architecture/catalog/warnings/format checks pass. CI lists and executes **27** tests under the strict equality guard. Component goldens and allowances are unchanged.

Introduced findings corrected before closure: the first new test build lacked the explicit maintained fixture-helper import; correcting that import establishes the intended tests-first result (one existing semantic witness passes, two new sharing/image assertions fail on the parent). The first architecture run correctly rejected new unsafe image-decode diagnostics outside its exact allowed module. Moving the unchanged image test into the existing session identity/image test module fixes it; no allowlist or gate is broadened. The final test root, inventory and all architecture checks pass. These original failure receipts are retained. No production change follows timing.

No runtime/build failure or resource denial remains for the measured case. The only inherited exclusion remains `complete_unchanged_knucleotide_body_uses_the_owned_emitted_map`, whose reduced selected catalog lacks its stats import. No all-features or hosted-CI pass is claimed. PR #973 remains draft until green CI and clean review; publication CI is cancelled by user direction.

## Fidelity and reproduction

Source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; MBC `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Preserve algorithms, tables, actual **262144 U16 tree / 1500513 U32 cache**, depth **300**, nice length **258**, original near-EOF limits, allocation sites, pinned mimalloc **2.1.7**, output capacity **13** clear/reuse, and evaluate_into timer. Mech performs compression; independent inflate stays outside timing. Exact raw bytes `[75,76,74,78,73,77,75,207,0,0]`, input contents/aliases and zero final cleanup agree in every cohort. No larger input is timed.

Apple M1 / 8 GiB / macOS 15.6.1 (24G90), Rust nightly-2026-03-03 `ec818fda361ca216eb186f5cf45131bd9c776bb4`, LLVM 22.1.0. Pinned Roc binary `3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`, compiler `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`, package `671077457ff7ef3a3d44caf715a3fc595addf379`. Each timing, counter, allocation and sample cohort is serialized and committed immediately with commands, flags, hashes and raw results. Only GPU is rebuilt; no math-library rebuild is needed.

`reconstruct.py` verifies every archived version before chronological overlay, then independently reproduces scorecard JSON/CSV and numerical profiles. It does not rebuild binaries or rerun benchmarks. Shared native bodies, canonical arena compaction, scalar/window facts and remaining ownership proposals stay separate. Current frame actions remain 24.27% of overlapping samples; the unchanged 35 requests of 2,442,056 bytes are a concrete allocation-profile target, not proof that they can be removed.

Independent numerical replay verifies **52 archives / 450 file occurrences / 378 unique paths**, preserving 5 multi-version paths. Scorecard JSON/CSV and profile-number JSON reproduce byte-for-byte.
