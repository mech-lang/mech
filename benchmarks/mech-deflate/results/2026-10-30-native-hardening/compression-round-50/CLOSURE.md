# Completed-child physical cleanup — round 50

**Retain.** Complete eight-byte level-12 compression improves **14.639474× → 14.065937× Roc** using one shared fresh 0.285 ms reference: **4.172250 → 4.008792 ms (−3.91774%)**. All four freshly prepared pairs / 63 of 64 descriptive blocks improve. There are 3,968 primary calls, 1,984 per version, plus 16 warmup/profile calls. The before is the retained round-49 executable measured again; do not multiply independent cohorts or compare multiples across different reference clocks. Parity is unachieved.

## Checkpoint and exact change

Published parent: `5a519cf059619a840af2ed26dd4fa1a3d34d90d9`. Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`. Worktree `/private/tmp/mech-deflate-linear-stack`; branch `codex/level12-compression-consolidated`; draft PR #973.

Tests first: `311ff37e1ab49add994f6731211be90f2e11478d`. Production: `9d90db331329c351eed0475c33de15c3582b04d3`. Initial added proof/recovery tests: `795e1fa74129cca48137a1398d42367bd2c88929`. Separate CI inventory: `7dd730f543b036efdadda86686ef65bb42d44b51`. Separate final test-layout repair: `361fda2ce528481d50f06b408cf0b729a9673240`. The exact optimized GPU source is **52ecb3520b7f82cc00b33a8a8d37c8b6e6428965**; rebuild took **22.062 seconds** using the retained opt3/features/warnings-as-errors flags. All seven other direct libraries and the common/transitive dependency graph stay frozen. Later executable-source changes are tests plus one cfg(test)-only module-path attribute; `release-source-after-tests.diff` records that exact boundary. No production code changes follow the measurement.

The real compressor repeatedly returns from nested collector/search/skip/parser/Huffman calls. Original lexical inventories include each managed local in its owner and every ancestor; successful child cleanup therefore leaves many entries empty before a parent boundary. Round 49 retained the original per-slot scan and physical frame clearing. This change prepares original logical prices separately from a shorter list of physical effects.

A same-plan descriptor layout selects only strict-descendant Tail locals other than the owning child's returned value. Current-scope values, returns, collection/iteration crossings and uncertain physical aliasing retain their original effects. The original immutable inventory and admitted invocation backing remain unchanged. There is no dirty-slot state, executor, second ledger, permission cache, allocation epoch reuse or source-ID rule.

## Proof and execution

Current-visit validation closes every producer in its lexical owner and checks all structural child entry/exit edges, including dormant arms. A parent can resume only after successful child exit or without entering it. The ordinary successful Tail exit clears every nonreturned managed local; closed native children reconcile through that same ordinary exit. A failed child/cleanup aborts the invocation instead of resuming the parent. There is no later producer outside the owner to revive a nonreturned local. Fresh invocation banks start empty, including never-entered child scopes.

Canonical arena homes remain dedicated to logical slots. The derivation explicitly checks every selected frame word/owner lane for overlap and declines aliasing candidates; a future physical reuse pass must provide allocation-epoch proof. Shared immutable payload aliases are distinct from shared physical frame homes. Child returned values remain in the parent effect list until their ordinary copy/release. Collection exceptions stay conservative. Owned-image restoration rederives this metadata from the same immutable plan, original inventories and rederived frame recipes; no new serialized field or policy version appears.

At a reached boundary, the original scan charge is paid first. Prepared preview validates original arena/account identity even if all effects are omitted, required frame backing, and the remaining current entries/generations. The existing noncloneable synchronous paid-retirement loan funds the original complete total. Original prices for empty child slots are consumed on that loan without a physical effect. Remaining effects retain their order and round-49 storage-domain checks. Original frame/arena owner release is unchanged where storage exists. Insufficient funding, worker budgets, active/nested/foreign scope or uncertain preview keeps the original ordered path, first error, spent prefix and partial cleanup.

The focused real Huffman test fails on the parent (original work 91,452,918 but no omitted child effects) and passes after implementation with that exact work unchanged. New original/decoded tests cover original prices, current locals and returned-value exceptions, unentered/empty children, repeated calls, changed arguments, arithmetic failure after a write, unchanged published roots and input aliases, cleanup and retry. Existing finite control/library/comparison/memory/node/transport boundary tests and every original retirement contribution boundary remain passing. Test builds additionally assert actual emptiness of every omitted frame word/owner and arena entry.

## Actual mechanism and fresh profile

A separate coherent instrumented GPU source overlay builds its own matching library, Object/image and linked worker. It checks all skipped storage rather than inferring emptiness from source IDs. Eight steady complete calls consistently report:

| Counter | Per call |
| --- | ---: |
| Paid batches | 590 |
| Original slot attempts | 26,565 |
| Batches with prepared child summaries | 311 |
| Omitted completed-child slots | **14,091** |
| Remaining slot effects | 12,474 |
| Ordered fallback batches in this input | 0 |
| Unexpected nonempty omitted slots | **0** |
| Omitted frame containers | 7,673 |
| Omitted canonical slots | 6,418 |

The instrumented time is excluded from every performance comparison; production symbol checks exclude both diagnostic and test-only hooks. Work stays **60,260,219,686**, comparison work **47,612**, capacity 13, raw output/inflate and aliases/content all agree, final cleanup is zero. Source allocation sites and all 50 large requests / 97,129,588 requested bytes are unchanged.

Separate hardware medians improve **46,770,314 → 45,222,691.5 instructions (−3.30898%)**, **13,263,236.5 → 12,658,163 cycles (−4.56204%)**. Allocation traffic remains 13,484 requests / 98,529,723 requested bytes per call. Prepared tracked heap grows **153,328 bytes**, 142,829,248 → 142,982,576; additional execution peak remains 20,575,320 and absolute peak grows 163,404,568 → 163,557,896. Retain this measured space/time tradeoff explicitly. Post-call OS memory is not peak RSS; copy volume/cache/branch misses remain unmeasured. See [fresh numerical profile](PROFILE.md) for the reference comparison and overlapping sample weights.

Generated Object SHA-256 remains `3f0c0e1954afe1083254d928a8ff66977ee9203e64bfd281326c75f9ff5f09e9`, image `c34838f0b42997bcdcbfb86df1eddad94303fb3083c34c89829ab4a8f8ae9a05`; actual payloads are archived once in round 46. Text remains **13,885,696 bytes**, machine stack **108,400 bytes**. Host cleanup disassembly grows 676 → 763 static sites while dynamic instructions fall. IR/work/ABI/bytecode stay **131/95/37/14**.

## Qualification and all failure dispositions

**596 fresh distinct Rust tests pass**: 319 runtime, 32 work/memory/value, two preparation, 234 component, eight connected and one path mechanism. Repeated focused 24-case selections overlap this total. The unchanged Core retains its prior **413-test** qualification, exact production hash `d108019406335c7627061c0bf46cfda6e96adfc05fb97dae652499f1bbcbb873`; 1,009 identified tests are covered across cohorts, not 1,009 freshly executed tests.

All **12 linked original/decoded failure/recovery executions**, 12 scalar controls (original ≤1.05 latency gate and exact pure-scalar allocation equality), eight frame-allocation cases (original zero slope/no request-byte-work increase), 132 Python workflow/inventory tests and ten architecture/catalog/warnings/format checks pass. CI lists and executes **24** tests, preserving the strict equality guard. Component goldens and allowances are unchanged.

Introduced findings repaired before closure: the initial private driver guessed 27 rather than 24 selected tests (Rust already passed all 24); a missing baseline control hash sidecar stopped before any control execution; a repeated receipt label correctly refused to overwrite an immutable earlier archive. These private setup errors are recorded and corrected without weakening any gate. An added source fixture incorrectly expected a recursive call inside a comprehension to reach native preparation; the frozen unchanged Engine rejects it with `source-semantics/recursive-comprehension`. The corrected test explicitly preserves that existing rejection and does not invent a native plan or expand compiler scope. The new file/directory test-module collision failed the architecture contract; separately moving only the test file fixes it, and the final test root, CI inventory and all architecture checks pass. Original failure receipts and correction metadata are retained.

Initial independent numerical replay also identified omitted archive inputs (derived profile numbers and the unchanged baseline binding/metadata); they were archived unmodified, and the final independent replay passes. The only intentional red acceptance test is the tests-first parent witness. No runtime/build failure or resource denial remains in the complete measured case. The only inherited exclusion remains `complete_unchanged_knucleotide_body_uses_the_owned_emitted_map`, whose reduced selected catalog lacks its stats import. No local all-features or hosted-CI pass is claimed. Publication CI is cancelled by user direction; the PR remains draft pending green CI and clean review.

## Fidelity and reconstruction

Source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; MBC `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Algorithms, tables, actual **262144-entry U16 tree / 1500513-word U32 cache**, depth **300**, nice length **258**, original near-EOF limits, allocation sites, pinned mimalloc **2.1.7**, capacity-13 clear/reuse and evaluate_into timing boundary are unchanged. Mech compresses itself; independent inflate is outside the timer. Raw output remains `[75,76,74,78,73,77,75,207,0,0]`. Only complete eight-byte compression is timed.

Apple M1 / 8 GiB / macOS 15.6.1 (24G90); Rust nightly-2026-03-03 `ec818fda361ca216eb186f5cf45131bd9c776bb4`, LLVM 22.1.0. Pinned Roc binary `3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`, compiler `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`, package `671077457ff7ef3a3d44caf715a3fc595addf379`. Each timing/counter/allocation/sample/mechanism cohort is serialized and committed immediately with commands, feature flags, revisions, hashes and raw results. Only the GPU library is rebuilt; no math-library rebuild is required.

`reconstruct.py` verifies every preserved receipt version before chronological overlay and reproduces scorecard JSON/CSV and profile-number JSON byte-for-byte. This numerical replay does not rerun binaries or benchmarks. Completed private build/diagnostic payload cleanup is separately recorded; primary matched workers, libraries, Object/image and all source/commands/receipts remain. Shared numeric-shadow storage, native-body sharing, scalar/window facts and other ownership work remain separate experiments. Frame actions remain 22.64% of overlapping fresh samples; no future speedup is presumed.

Independent numerical replay verifies **57 archives / 489 file occurrences / 414 unique paths**, preserving 3 multi-version paths before chronological overlay. Scorecard JSON/CSV and profile-number JSON reproduce byte-for-byte.
