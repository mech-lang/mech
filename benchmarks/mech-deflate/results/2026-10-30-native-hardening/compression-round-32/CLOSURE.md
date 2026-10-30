# Compact call-cache storage: retained

Complete eight-byte compression improves **18.687430000× → 16.937986667× Roc**, a **9.361604744% elapsed reduction** (5.606229 → 5.081396 ms). All four independently prepared worker pairs improve; 63 of 64 within-pair blocks improve. Retain this generic compiler/runtime optimization. Parity and larger-input throughput remain unestablished.

## Revisions and review findings

- PR base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
- Source baseline: `aff5d1e7d544fb18ef8efa4d66766ae58ae57a83`, clean review 6515868269.
- Production compiler: `915f039cfd7e2647cc3a1d349ab1070c3f292d19`.
- Before inventory: `ae4c97390`; parent witness and protocol: `d01a39761`; verified after-constructor: `5e91a9ce0`; complete preflight: `374947979`.
- The earlier frame-read experiment remains removed; prepared retirement and shared-accounting improvements remain retained. This round addresses the separate compact-cache recommendation without changing them.

Exact commands, feature configurations, frozen binary/library hashes, source identity and all observations are retained in the packets below. IR/work/ABI/bytecode revisions remain **129/93/36/14**.

## Concrete mechanism

The old invocation allocated a 24-byte entry for every one of 41,039 control blocks and classified 40,836 as repeated, even when they had no prepared call. Actual factories occupy only 5,181 sites: 5,061 `LibraryCall` and 120 `PrepareU64Range`; 5,137 repeated and 44 once-only. These counts come from actual complete-program preparation and actual constructor execution, rather than a source estimate.

Prepare one immutable logical-site-to-dense-slot map from the complete `metadata.calls` inventory. Include all prepared actions and calls needed only by cold fallback. Independently rebuild this map on image loading. Each invocation borrows the map and owns only its dense entries and bound contents. A centralized accessor translates logical sites; string-span and map-span readers use it alongside ordinary invocation. Serialized input cannot supply the derived map. No source specialization, raw pointer, executor, alternate owner or per-visit allocation is added.

| Actual lane construction | Before | After |
| --- | ---: | ---: |
| Initialized entries | 41,039 | 5,181 |
| Initialized repeated entries | 40,836 | 5,137 |
| Physical entry backing | 984,936 bytes | 124,344 bytes |
| Admitted capacity and node units | 1,970,000 | 248,816 |
| Setup work | 15,760,128 | 1,990,656 |

This removes **860,592 physical bytes** and 1,721,184 admitted units per invocation, preserving the 24-byte entry, twice-byte envelope, 128-unit header and eight-work-per-admitted-byte price. Constructor admission, transport geometry, worker setup claims and caller-retained inventory all use the new exact count. Persistent state grows eight bytes for the borrowed descriptor reference; maintained tests establish that persistent 1,128 + invocation 1,280 = 2,408 bytes still fit the unchanged 2,432-byte envelope.

Preserve once-only root versus loop-repeated classification, current inputs, account/foreign-scope checks, failed-call metadata reconciliation, no refund, exact capacity checks before initialization, error order, aliases, publication and retry. Setup work falls by 13,769,472 because fewer entries are initialized, with no price reduction. Complete measured work changes **45,060,848,918 → 45,047,079,446**, exactly that difference. The unchanged native program still executes every algorithm operation.

## Protected timing and immediate receipts

The predeclared experiment uses four independent worker pairs. Each prepares baseline and candidate sequentially before measurement and reverses preparation order across groups. Each pair runs 16 fixed alternating-order blocks, with 31 primary calls per binary per block. Only one worker evaluates while its peer blocks on a stack byte outside the timer. The 64 blocks share **four** worker pairs; they are not 64 independent experiments.

All 3,984 observations are retained, including 3,968 primary observations. Leading warmups and trailing diagnostics remain archived and excluded. No JIT preparation, build, test, audit or other measurement overlaps timed evaluation.

| Worker pair | Before | After | Pooled elapsed reduction | Improving blocks |
| --- | ---: | ---: | ---: | ---: |
| 0 | 18.687986667× | 17.007430000× | 8.992711% | 16/16 |
| 1 | 18.651251667× | 16.873543333× | 9.531308% | 16/16 |
| 2 | 18.682291667× | 16.946875000× | 9.289100% | 15/16 |
| 3 | 18.737223333× | 16.897570000× | 9.818175% | 16/16 |
| Combined descriptive median | 18.687430000× | 16.937986667× | 9.361605% | 63/64 |

Median block ratio: 0.904284144; geometric block ratio: 0.906699861. No confidence interval or 64-independent-replicate claim. The fresh common reference median is 0.300 ms from 20,000 observations. These normalized values describe this matched experiment and must not be spliced with earlier reference campaigns.

Every cohort was committed before the next stage: reference `7ec5b3a3e`, group 0 `4f27ccd72`, group 1 `0cad088ae`, group 2 `5cd6197de`, group 3 `5f6dfa0d7`, summary `0dc81c650`. Raw stdout/stderr and all rows are in `primary/`.

### Pinned source, allocation and validation

The complete input remains eight bytes, `abcdefgh`. Mech performs compression itself.

- Source SHA256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
- Artifact: 4,138,627 bytes; SHA256 `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
- Reference package: `671077457ff7ef3a3d44caf715a3fc595addf379`.
- Reference compiler: `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`.
- Reference executable SHA256: `3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`.

Algorithms, tables, depth 300, nice length 258, near-EOF limits, the real 262,144-entry U16 tree and 1,500,513-word U32 cache, source allocation sites and output reserve 13/clear/rebind remain unchanged. The allocator remains mimalloc 2.1.7 and the `evaluate_into` timing line is byte-identical. Every observation verifies ten exact raw bytes, independent inflation, aliases, capacity 13 and final cleanup to zero. Two earlier complete qualification calls are archived separately and excluded from primary timings.

## Regression evidence

**561 scoped maintained tests pass:** runtime 304 (including nine cache tests), work 12, exact transport 1, the unchanged component cohort 232, selected scalar 8/bits 3, and actual runtime-only linked recovery 1. The focused component six and cache nine overlap these totals.

The parent density witness fails with four allocated entries versus zero prepared calls. After the patch, original/decoded witnesses establish the complete inventory, absence at non-call/out-of-range sites, root/repeated kinds and real non-`LibraryCall` actions. Existing tests exercise changing inputs, account isolation, actual byte/node/work/transport exact and one-short boundaries, excess backing rejected before initialization, unchanged publication and aliases on rejection, cleanup and retry. Forged serialized cache data is rejected. All expected program outputs and allowances remain unchanged; exact geometry assertions now follow the smaller inventory.

Actual generated linked Object/descriptor and the maintained recovery driver execute 3 → assertion rejection at 0 → 2. Assertion precedes division, published 4 survives failure, and recovery publishes 6. The GPU runtime closure has only `canonical-native-program-runtime` / `canonical-native-runtime`, without parser or code generator. Source, decoded and image consumers are exercised.

All **12 scalar controls** pass the unchanged ratio limit of 1.05; maximum observed ratio is 1.003666882. Pure scalar allocation counts/bytes are identical. Scalar-loop counts do not increase, requested bytes fall 2,404,800 → 2,344,800 per 100-call cohort, and allocation slopes remain zero at 0/64 steps. **Eight managed-frame controls** retain counts and zero slopes; requested bytes fall 373,050 → 366,570 (exclusive) and 353,680 → 347,200 (aliased) per ten-call cohort. These intentional savings are recorded; any request/byte increase is rejected. Scalar/frame receipts were immediately committed as `525114573` / `8128aa1b4`. Allocation-control timing contributes no compression speed claim.

All **12 architecture/catalog/inventory/unsafe/warning/format checks** pass. Warnings-as-errors, unsafe allowance and existing architecture checks are preserved. Configuration: nightly 2026-03-03, aarch64 macOS, locked/offline, jobs 1, incremental disabled; production opt 3, maintained integration bodies opt 1, actual GPU unit root opt 0. All seven production feature/profile sets and all seven non-GPU directly linked library hashes match. Only GPU production code changes. Broad unchanged Core/Engine suites are not claimed fresh.

### Generated binary and fresh profile

Fresh candidate emission is byte-identical to the independently emitted retained-parent **Object and compiler metadata**: 12,924,328 Object bytes, 12,923,596 text bytes and 108,032 stack bytes. All 239 selected numeric regions retain zero calls on successful native backedges. Object SHA256: `97152f81cfe0270df3ac255a0c6da360dfb9e0ffb0bc075f7033e9549c44205b`. The improvement changes host runtime bookkeeping without changing or growing generated program code.

The fresh profile (`ea3990c84`) was immediately committed; all 695 observations are excluded from primary timings. Its 1,306 exclusive weights remain concentrated in frame actions (340), ordinary visits (239) and other helpers (310), using disjoint ancestry categories. Distinct cache-constructor samples fall 51/1,292 → 0/1,306 against the separately recorded parent profile. **Zero samples do not mean zero cost**: inlining and sampling can change attribution. Actual constructor counts establish the mechanism; paired elapsed results establish the net gain. Full symbol attribution and hashes are in `qualification/profile-analysis.json`.

## Failures, omissions and next bounded task

The expected parent-red density failure is fixed. No implementation, qualification or architecture command remains failing. Expected finite-denial paths remain exercised, including the existing resident `extend-35` default compute denial (16,842,945 required versus 16,777,216 allowed); no allowance was increased. No source program, pointer loan or older semantic package changes in this round.

Skipped coverage: all-primitives `not`/`xor` outside the selected catalog; absent-statistics whole-knucleotide execution; tests outside the scoped cohort; fresh broad Core/Engine suites; larger-input timing. Hosted CI is cancelled by user instruction, so there is no green-CI claim and the PR stays draft.

Next, establish a minimal witness and explicit generic parent/child contract around reached `huff-assign-lengths → huff-assign-symbols`. Specify parent suspension/resumption, current-version array handoff without overlapping mutable owners, shared finite funding and failure/fallback cleanup **before compiler code**. The fresh profile still identifies ordinary frame/control/helper traffic as the main frontier. Do not remove a guard merely because the child qualifies independently.

Evidence packets: `inventory/`, `parent/`, `after-inventory/`, `preflight/`, `primary/`, `profile/`, `scalar/`, `frame-allocation/` and `qualification/manifest.json`. Publish this independent result before the next production change.
