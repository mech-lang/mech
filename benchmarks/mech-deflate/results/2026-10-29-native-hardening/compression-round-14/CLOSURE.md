# Ordinary private-frame admission review packet

Round base: `fae97122b0dff4506f8fcb93e7cee87b9a67e184` (reviewed round 13 checkpoint). PR stack parent: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`. Final measured production compiler: `5a46c85935279a61b934222331b643b81d254d6d`. Branch: `codex/level12-compression-consolidated`; worktree: `/private/tmp/mech-deflate-linear-stack`. The publication receipt commit adds only this packet to that production code. The exact final published head is recorded in PR #973 and the task's final response. The separate main checkout is untouched.

## Result and scope

Roc = 1×. The final paired complete eight-byte compression result is **254.339556× → 212.543257×**, **16.433% less elapsed** (0.073249792 → 0.061212458 seconds). Fresh pinned reference median: 0.000288 seconds from 20000 observations. Three primary observations per control, with warm-up and diagnostic observations excluded. No confidence interval or larger-input throughput is claimed; parity remains unachieved.

The same campaign separately measures prepared footprints with original shapes at 222.207174×. Direct shapes remove another 4.349% of that elapsed time. Earlier combined, isolated-footprint and shared-proof campaigns retain their original samples, medians and independent reference measurements. Their numbers are not substituted for this final paired comparison. Every timing/profile packet was committed before another optimization.

Exact ten-byte output, independent inflation, capacity13, final zero bytes/nodes and unchanged logical work 527732164684 pass for every observation. Output backing changes on every control; those boundary copies remain inside the evaluation timer. Algorithm, 25 components, tables, 262144-entry U16 tree, 1500513-word U32 cache, depth 300 / nice length 258, EOF rules, six Huffman initializers, allocation sites, source/bytecode hashes, pinned mimalloc2.1.7 object, diagnostic NoLimits policy and evaluate_into boundary are unchanged. No larger full compression input was run.

## Design

The existing `FrameGeometry` owns a reconstructed field-admission recipe. The budget/footprint owner reuses `BorrowedFootprintSchemaProof` and prepares exact scalar footprints through the maintained scalar measurement once. Runtime scalar words retain exact validity checks and return fixed logical footprints; packed fields read current metadata through Core, including actual retained capacity. Current schema IDs/keys, account membership, shapes, output bounds and coexistence remain checked. Runtime uses cached table-entry keys, not Schema hashing. No arguments, extents, successful admissions or owner references are cached.

After the original metadata preflight and normalization-envelope reservations, the existing budget meter previews a complete serial footprint debit. A success combines exactly the same compute/node work on the same account. Any unsupported layout, invalid input, overflow, tight allowance or shared-worker account keeps the incremental path, error precedence and spent prefix. Logical canonical footprints remain distinct from physical retained-byte reservations. Work prices, IR118/work93, ABI35 and bytecode14 are unchanged.

Only after that current flat-field success may `FrameShapeRecipe` compare current axes directly. A linear binding-time pass records parameter representatives. Constants, distinct/repeated parameters and current child arenas are checked; each output matrix retains its validated child's extents and checked element-count product. Unsupported/missed relations retain the original constructor. Scope restoration, factory selection, ownership, error/publication behavior and retry remain unchanged.

The change is in six Engine production files, with no new executor, compression shortcut, native-loop class, public ownership authority, ABI or serialized proof. The call-specific admission module is 85 lines; the shared footprint module includes its own differential tests. Image restoration rederives the private recipe through the existing prepared-call path.

## Evidence

**257 unique current scoped Rust passes**, without counting focused reruns twice:

| Coverage | Executed tests | Outcome |
| --- | ---: | --- |
| Actual Engine test root, optimized, current feature profile | 101 | Pass; budget, worker credit/unwind, projection, footprint and constructor parity |
| Maintained component cohort, original/decoded, unchanged allowances | 145 | Pass; includes the 12 focused frame tests |
| Selected native scalar correctness | 8 | Pass |
| Integer bits | 3 | Pass |

The frame test records 144 cases across original/decoded artifacts and fresh empty/small/real-table shapes: all 120 baseline work/error/retry observations agree exactly, and 24 competing compute/invalid-scalar/output errors pass. Typed scalar cases include widths, signed extrema, negative zero and a NaN payload. Packed footprint cases cover all four unsigned widths, current rectangular/zero extents, aliases and the actual table capacities. Each logical footprint and compute/node debit agrees with the maintained ordinary visitor. Constructor parity additionally covers repeated/constant axes, 2×3/3×2 changes, zero products with unequal axes and u64::MAX×0.

Existing component publication, alias, late-error, retry, real-buffer private-write/result, no-op, fuel-boundary and parent-argument protections pass. Expected printed resident default-limit denials and injected unwind panic messages belong to passing tests; no allowance was increased to erase them.

**12 fresh scalar performance/allocation pairs pass** against the retained round 13 control. Maximum observed ratio: **1.016561616**, below the unchanged 1.05 ceiling. Allocation requests/bytes agree exactly; zero-versus64-iteration allocation slopes remain zero. These probe/system-allocator controls are separate from the primary mimalloc timing and do not provide a primary cost breakdown.

**12 architecture/catalog/inventory/unsafe/warning/format checks pass.** Warnings remain errors. The release uses nightly2026-03-03, jobs1, incremental0, locked/offline selected-operation profile `benchmarks/mech-deflate/native-profile/Cargo.toml`, preserving the full compiler/value configuration and only the five required arithmetic machine families. Exact commands, cfg/features, dependency/source/binary hashes and elapsed build/test receipts are in `final/`, `qualification/` and the earlier phase packets. All primary/scalar measurements ran without another compiler/test/benchmark.

The complete Object emission, full metadata and successful-private-CFG no-call audit pass: **149 regions**, 45 read-only and 104 writes, including 13 private mixed results. Both Object bytes and metadata are **byte-for-byte identical to round 13**: Object SHA256 `eef36a03e58b00c5a85c3edff1cb0c67135bf908ec666690d395bb2ee6a488f2`, metadata SHA256 `a545defa312f3557b046a8e0656b4aaf9e59f6cf27c724c5d0b8a21096906294`. No loop eligibility or emitted-code change is being credited for this runtime improvement. The retained passive reference ARM64 evidence likewise remains pinned, without mapping compiler-hashed procedures to named source functions.

The disjoint sample partition reproduces the brief's prior 1347-sample categories and124/78/47/80 admission breakdown. Final frame-input admission is **135/1349 samples**, against 329/1347 before; the old generic footprint and shape-constructor roots disappear beneath that successful path. Other frame handling remains. Sampling categories describe execution ancestry, not exact removable time. Full original samples, parser scripts and partitioned results are retained.

## Commits

- `2404a0359`: maintained baseline regression and exact prior-production receipts.
- `c8b87b305`: initial combined admission/shape implementation.
- `aa93a9e3f`: initial paired timing and profile before further edits.
- `bf850c333`: isolate footprint/accounting from shape reconstruction.
- `2427adf7b`: commit the separate comparison and environment recovery receipts; published checkpoint received a completed clean review.
- `d6675140a`: consolidate through existing schema proof/footprint owners and prepare fixed scalar measurements.
- `d840ea837`: shared-proof timing and100-test Engine qualification packet.
- `5a46c8593`: separately justified direct shape relations and expanded constructor parity.
- `5a3bee0d8`: final paired timing/profile and101-test Engine qualification packet before any next optimization.

## Failure and coverage disposition

No introduced compiler/test failure remains in this scoped qualification. Two initial baseline-fixture errors (moving a borrowed value with its closure; comparing memory before/after a live work-account allocation) were corrected before the passing baseline. The first release attempt's unresolved error import was corrected to the Core owner. Those original receipts are retained. Profile reanalysis initially matched mangled closure substrings too broadly; exact admission/managed-root matching now reproduces the supplied retained analysis.

An environmental preflight failure occurred before any observation when the old temporary reference checkout lost its pinned source files and Git HEAD/config. Exact source blobs were recovered from its surviving object database and all 14 existing SHA256 pins verified. No reference binary or source revision changed; executable hashes remained correct. Exact deletion cause is unestablished. Recovery receipts and a deterministic source archive preserve reproducibility.

**Inherited unresolved timing:** round 10 decoded-U64 scalar ratio 1.0526036 exceeded 1.05. This round's passing controls do not erase that history; scalar retirement is not declared complete.

Skipped this round: full larger-input timings; the all-certified-scalar-primitives test requiring omitted logic_not/logic_xor; rebuilding the optional generated runtime-only linked application; and the four GPU cfg(test) runtime fuel units. Prior round 13 completed receipts remain historical evidence, not new-head passes. Current public component fuel/publication/recovery protections run, and current Object code/metadata and build-image restoration are audited. Parser, assertion recording, executable-corpus migration and CI-inventory findings are not reopened as work packages.

Intermediate published head 2427adf7b has a completed clean review (comment 6487999056) and CI run 40817963310 is completed/cancelled at the user's direction. The final publication will remain draft, cancel its new CI and request a fresh review; cancelled CI is not represented as green. Current qualification uses no CI wait.

## Next bounded step

Predicate-correlated current-version analysis for the real cumulative Huffman recurrence is next, as requested. First capture a fresh current-head generic/pinned witness and dynamic contribution; the 41 static stale-return declines are not equal runtime costs. Reuse/extract mechanical current-Boolean/copy/edge facts while keeping text and numeric eligibility separate. Preserve freshness across recursive visits, aliases, exact errors, ownership and fuel behavior. Commit its own paired small timing/profile before any next optimization. The whole-program 1× goal remains active.
