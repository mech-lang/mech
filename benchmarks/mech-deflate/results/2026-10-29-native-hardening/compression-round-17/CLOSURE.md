# Prepared semantic descriptors: review packet

Reviewed base: `3615e260d96185ffbd3b7c57a9b7613742ac8207`, clean review [6492136036](https://github.com/mech-lang/mech/pull/973#issuecomment-6492136036). Production compiler: `1388b4da98174ca3493ca65cdafee3b44f876f7f`. Baseline evidence: a2e3fbe0b; timing/profile: `371a711e085530bcd72fcad7b9f01ffd68698121`. The publication commit adds this closure and receipts; its exact head is in PR #973 and the task response. PR parent eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b. Branch/worktree: codex/level12-compression-consolidated, /private/tmp/mech-deflate-linear-stack. The separate user's checkout is untouched.

## Result

**Roc=1×:128.891179×→113.747852×,11.748924% less elapsed,1.133131× speedup.** Complete eight-byte medians37.507333ms→33.100625ms, same fresh reference median0.291000ms from20000 observations.3 primary observations per Mech control; warm-ups and the separate121-repetition profile are excluded. Previous round16 after was127.863609×, compared with this fresh128.891179× control. No confidence interval or larger-input throughput is claimed. [Committed timing packet](TIMING.md).

Exact ten-byte output, independent inflation, capacity13 and final zero bytes/nodes pass throughout. Work is exactly303735923752 on both sides(delta0). Output backing still changes and copies stay timed. Same pinned mimalloc2.1.7 object; no simultaneous compiler/test/benchmark during primary or scalar measurements. Mech compresses itself. No larger full-compression input ran.

Source revision671077457ff7ef3a3d44caf715a3fc595addf379; reference compiler d99c7b838fb05bb1047eb0c18d4d06d0f86f345d; reference executable SHA2563941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324. Caller source SHA256331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9; bytecode SHA256a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7. The25 components, algorithm/tables,262144-entry U16 tree,1500513-word U32 cache,depth300/nice258,EOF limits,6 Huffman initializers, original source allocation sites and evaluate_into-only boundary are unchanged. Fresh source compilation also reproduces that exact bytecode and decoded reencoding; its receipt explicitly reports compression_executed=false.

## Architecture and semantics

Ordinary prepared calls previously rebuilt ResolvedValueDescriptors and normalized the same immutable schema types on each bind. The retained profile places165/1348 exclusive weights in descriptor construction,141 in type derivation, under198 binding weights. Reached controller/Huffman/optimal-planner record-field reads are the real requirement; this is a runtime compiler cost rather than a compressor algorithm change.

The existing NativePreparedLibraryCall now retains ordinary validated descriptors as immutable schema/type metadata. A neutral valid shape is only a preparation witness. It is never an argument, a buffer, a zero-filled path matrix or a permit. If the witness or semantic type is unavailable, the original live constructor remains. Core's16-line with_current_shape method checks all current bounds and body extents, then clones the already validated immutable schema/type. ResolvedType derivation depends on those declarations, not current shape values. Every live call still binds its actual current shape. No current Value, success, owner or argument is cached.

The same binder, selected factory, operation contract, current schema/selector/shape/owner checks, memory planner/kernel, payload copying, finite work/copy/output/coexistence admission, atomic publication and retry remain authoritative. The existing optional-cache headroom guard is untouched. Metadata retains its original conservative charge, which the whole-program and128-case witnesses confirm exactly. The build-image loader reconstructs the optional skipped field; no wire field or runtime ownership authority is added. IR119/work93/ABI35/bytecode14 remain. There is no alternate executor, source-name shortcut or scalar-emitter change.

## Validation

**424 unique scoped Rust passes**:

| Maintained scope | Executed | Outcome |
| --- | ---: | --- |
| Core type-system contract target |21|Pass, including3 new shape tests |
| Actual Engine budget/worker/projection/frame/text/collection test root |227|Pass |
| Component cohort, dynamic projections and binding pressure |165|Pass |
| Selected native scalar correctness |8|Pass |
| Integer bits |3|Pass |

The11-test focused rerun is included in165, not double-counted. Four new Rust tests comprise3 Core comparisons and1 source-derived128-case witness. Core compares refresh with normal derivation across all unsigned widths, changing/empty/rectangular/full-u64 extents, independent descriptors, dependent bounds, first errors, wrong parameter counts, extent/bound overflow and retry. The record witness covers4 widths×2 original/decoded forms×4 fresh geometries×4 selector visits:96 successes and32 expected missing-field rejections, with aliases, prior-output storage, cleanup and recovery. All128 after rows match the old executable's typed errors/work/metadata/results. Every row requests fewer allocations and bytes. Success medians2739→1862 requests and220032→179254 bytes; rejection1237→360 requests and69025→28252 bytes. These System/probe diagnostics are excluded from primary. The exact same opt3 driver source/hash and corresponding dependency hashes are retained.

Existing expected outputs and allowances are unchanged. Current finite binding-pressure, error-order, fuel/publication/private-buffer/retry tests pass. All12 architecture/catalog/inventory/unsafe/warning/format checks pass, with warnings as errors. Non-native-artifact Engine library metadata compilation passes; that is compile evidence, not a runtime test count. Current build-image rederivation checks pass within the maintained cohort. No new linked executable was built.

**Scalar timing qualification is recorded without erasing a failed observation.** Initial12 pairs have exact allocation requests/bytes and zero-versus64-iteration allocation slope.11 meet≤1.05; original zero-step scalar-loop is1.052454987, above the unchanged guard. Its decoded zero-step counterpart is0.971006 and uses identical bytecode. One predeclared fixed balanced follow-up uses unchanged binaries/source/limits/5 measured samples/100 invocations, with8 pairs per form. All16 paired ratios are≤1.05(max1.023239056); original median-paired1.007024021, decoded1.005408545. Pooled ratios1.004911688/1.006814434. Full plan, all192 rows, commands, order and64 raw process logs are retained. This does not justify labeling the initial suite all green. The first exceedance remains a new timing observation with an unidentified cause; the balanced evidence does not reproduce a persistent>5% regression. No compiler change or selective reroll was used to obtain that result.

The first Core-dependent selected-operation release rebuild takes1004.195s. A temporary helper extraction made during that build was restored; final qualification uses a fresh455.022s rebuild of the exact clean production commit. Nightly2026-03-03,aarch64mac,opt3,jobs1,incremental0,locked/offline, existing5 arithmetic families. Core integration root usesopt1. The actual semantic Engine unit root usesopt0, taking42.462s instead of the preceding opt1~225s; it runs227 tests in7.184s process/5.86s displayed. All production, primary, scalar and GPU-test dependencies remainopt3. This test-only compile setting is not a performance optimization claim. Component run71.355s process/71.34s displayed; non-primary test elapsed times may include concurrent qualification and are not benchmark comparisons.

Principal commands (full features, dependency and source/binary hashes accompany each receipt):

```text
cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review
python3 /private/tmp/mech-binding-descriptors-round17-20261029/core-tests.py
python3 /private/tmp/mech-binding-descriptors-round17-20261029/components/qualify.py
python3 /private/tmp/mech-binding-descriptors-round17-20261029/primary/run.py
python3 /private/tmp/mech-binding-descriptors-round17-20261029/engine-units/build.py
python3 /private/tmp/mech-binding-descriptors-round17-20261029/full-components.py
python3 /private/tmp/mech-binding-descriptors-round17-20261029/after-witness/build-record-witness.py
python3 /private/tmp/mech-binding-descriptors-round17-20261029/primary/qualify-selected.py
python3 /private/tmp/mech-binding-descriptors-round17-20261029/primary/scalar-controls.py
python3 /private/tmp/mech-binding-descriptors-round17-20261029/scalar-repeat.py
python3 /private/tmp/mech-binding-descriptors-round17-20261029/object-audit/build.py
python3 /private/tmp/mech-binding-descriptors-round17-20261029/engine-units/nonnative-check.py
python3 /private/tmp/mech-final-static-20261028.py
```

## Binary/profile evidence and next task

The full Object and complete metadata are byte-identical to the prior compiler:11542304 Object bytes,SHA2562546a582ab601a30ef1375b4feb1346b6aee356c68f83bb362df75da501c6ae3; metadata1704995 bytes,SHA2567dfdbc9a9c96b334c71f71b5b8d19ea616f33da12d2a9a64fc930eeb22acc21f.190 selected regions(45 read-only,145 writes),26 private mixed results, zero reported successful-backedge helpers. Current emission/identity receipts are retained; the identical full metadata remains archived in round15. The preceding code-size increase remains, with no new generated-code growth. Archived pinned ARM64 reference allocation/load/store/backedge excerpts were reexamined; no new named source-to-hashed-procedure or per-function reference attribution is claimed.

The new1356-sample disjoint partition is381 other helpers,334 frame actions,250 ordinary visits,241 selected library and150 remaining. Under validated current calls, binding drops198→35 weights; descriptor construction165→14, with no type-derivation samples in that path. Invocation remains168 and retirement/other8. Sampling categories are descriptive, not exact removable elapsed time. The reproducible parser also recovers the older1347-sample partition exactly.

The next larger investigation is the reached record producers/consumers across private-frame boundaries. Engine already implements buffer-record field admission without cloning the payload. Do not add that operation again: determine why the retained53 reached record reads still take canonical calls, using actual selected producer/consumer IDs and a nested-record/function-result witness. The existing private-frame route, lifetime, shape and result-escape decisions must establish the exact missed opportunity before any compiler expansion. No next semantic change precedes review of this patch.

## Failure disposition and omissions

No compiler or Rust test failure remains. Introduced diagnostic setup failures (a new driver &String/&str compile error and a provenance script rehashing already-rebuilt mutable artifacts) were corrected before the successful baseline and preserved. The temporary helper/build-overlap mistake is closed by the clean final rebuild. Expected injected-unwind and existing default-limit diagnostics belong to passing tests. Primary has no resource denial. New scalar timing exceedance and its non-reproducing fixed series are reported above, not hidden.

**Inherited unresolved observation:** round10 decoded-U64 scalar ratio1.0526036>1.05 remains tracked. The new passing direct controls and balanced series do not erase it; scalar retirement is not declared complete.

Skipped: larger/full-input timings; all-certified scalar primitives requiring omitted logic_not/logic_xor; rebuilding the optional generated runtime-only linked application;4 GPU cfg(test) private-fuel units. Prior receipts remain historical. Current Core/Engine/ordinary original-decoded component execution, selected scalar/bit correctness, public fuel/publication/recovery, image reconstruction and Object emission are new-head evidence. Earlier assertion/corpus/CI/parser milestones are not reopened.

PR #973 remains draft. New CI is cancelled by user direction and is not green; the publication/final response records the exact head/run and @codex review request. The complete1× objective remains active at113.747852× on the measured input.
