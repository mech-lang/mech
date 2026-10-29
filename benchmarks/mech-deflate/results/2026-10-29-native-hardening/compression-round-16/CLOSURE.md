# Ordinary aggregate metadata: completed review packet

Latest round base: `c6054fcf72be0a196d700899360283108a29ccb8`. Production compiler: `e982adbf6f7c060877bd253dbffe7b982dd9c67f`. Timing/profile commit: `361889600cdd86cfb780e2e04a0daf9e213f20a4`. The publication commit contains this closure and the remaining receipts; its exact SHA is recorded in PR #973 and the task response. PR parent: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`. Branch/worktree: `codex/level12-compression-consolidated`, `/private/tmp/mech-deflate-linear-stack`. The separate user's checkout is untouched.

## Measured result

**Roc=1×:141.428201×→127.863609×,9.591151% less elapsed,1.106086× speedup.** Three primary observations per control:40.872750ms→36.952583ms. Same fresh pinned reference median0.289000ms from20000 observations. Previous independently paired round15 after was142.091837×; this round's fresh before is141.428201×. Warm-ups and the separate109-repetition profile are excluded. No larger-input throughput or confidence interval is claimed. [Immutable paired packet](PRECHARGE-TIMING.md).

Exact ten-byte raw output, independent inflation, capacity13 and zero final bytes/nodes pass throughout. Logical work is exactly303735923752 on both sides. Output backing still changes; copies remain inside the timer. The pinned source/bytecode/25 components, algorithms/tables,262144 U16 tree,1500513 U32 cache,depth300/nice258,near-EOF behavior, six Huffman initialization sites, source allocation behavior, same mimalloc2.1.7 object and evaluate_into-only boundary remain unchanged. No larger full-compression case ran. Mech performs the compression itself.

Source revision remains671077457ff7ef3a3d44caf715a3fc595addf379; reference compiler d99c7b838fb05bb1047eb0c18d4d06d0f86f345d; reference executable SHA2563941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324. Caller source SHA256331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9; bytecode SHA256a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7. Immutable binary/dependency/source hashes and all observations are in the primary receipts.

## Bounded design

The preceding investigation measured53 generic record-field calls in this same input. Reached controller/Huffman/optimal-planner reads call `numeric::aggregate_access::fund_metadata`. That helper manually walked the complete immutable schema table on each dynamic aggregate closure, bypassing the existing prepared retained-layout summary.

It now calls the existing `ResidentBudgetMeter::precharge_schema_layout`, which is also available without the native-artifact feature. The batch requires a complete successful preview on the current account and actual resident turn. Its total is exactly the original visitor's successful compute debit. Missing summaries, checked overflow, denied previews and shared worker credit keep the original ordered walk, first error, spent prefix and accumulated estimate. The worker guard is local to this schema helper; unrelated preview consumers are unchanged.

No new representation, executor, ownership authority, stored current argument, successful-admission cache or fabricated runtime turn plan is introduced. Selected factories, field selection, current source/selector/schema/shape validation, closed-shape reconstruction, footprint rules, actual payload copying, coexistence/output/copy admission and publication/retry remain on their maintained paths. IR119/work93/ABI35/bytecode14 are unchanged because representation, eligibility, prices and resource behavior are unchanged. [Design](precharge/DESIGN.md).

## Current validation

**278 unique scoped Rust passes**; the10-test focused projection/recursion run is already included in the163-component cohort and is not counted twice:

| Scope | Executed | Result |
| --- | ---: | --- |
| Maintained component cohort plus dynamic projection/recursion |163|Pass; prior expected values/allowances unchanged |
| Actual Engine budget/worker/projection/frame test root |104|Pass, including3 new default-meter tests |
| Selected native scalar correctness |8|Pass |
| Integer bits |3|Pass |

Default-meter differential tests run inside scoped turn-plan fixtures against the old visitor across every nonzero contribution's exact/one-short policy, account and call limits, seeded work, arithmetic overflow and shared worker settlement. They compare results, typed errors, estimates, spent-work prefixes and cleanup. The existing projection tests exercise changing owners and extents, empty and rectangular arrays, selectors/factory evidence, original/decoded artifacts, input/output aliases and success→failure→retry with unchanged prior publication. Existing finite-denial, ordered errors, fuel, private-buffer and recovery coverage remains in the maintained component cohort.

All12 protected scalar timing/allocation pairs pass (max after/before1.012417440988≤1.05), exact requests/bytes and zero-versus64-iteration allocation slope. These System/probe scalar controls are separate from pinned-allocator primary. All12 architecture/catalog/inventory/unsafe/warning/format checks pass. Warnings remain errors. Engine also passes a non-native-artifact-feature library metadata compile with the exact other selected feature cfgs/dependencies retained; this is compilation evidence, not a runtime test count.

Nightly2026-03-03,aarch64mac,release,opt3,jobs1,incremental0,locked/offline, selected-operation manifest `benchmarks/mech-deflate/native-profile/Cargo.toml`; the large math library is reused. Actual Engine test root usesopt1 with the exact release dependencies. Release458.173s, Engine-unit build225.704s and run1.143s process/0.12s displayed; component run74.218s process/74.20s displayed. Non-primary component/test elapsed times include concurrent qualification activity and are not performance comparisons. Protected primary/scalars run without competing compiler/test/benchmark processes.

Principal commands (full commands, feature lists and hashes are retained under `precharge`):

```text
cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review
python3 /private/tmp/mech-record-precharge-round16-20261029/engine-units/build.py
python3 /private/tmp/mech-record-precharge-round16-20261029/components/qualify.py
python3 /private/tmp/mech-record-precharge-round16-20261029/primary/run.py
python3 /private/tmp/mech-record-precharge-round16-20261029/full-components.py
python3 /private/tmp/mech-record-precharge-round16-20261029/primary/qualify-selected.py
python3 /private/tmp/mech-record-precharge-round16-20261029/primary/scalar-controls.py
python3 /private/tmp/mech-record-precharge-round16-20261029/object-audit/build.py
python3 /private/tmp/mech-record-precharge-round16-20261029/engine-units/nonnative-check.py
python3 /private/tmp/mech-final-static-20261028.py
```

The complete Object and metadata are **byte-identical** to round15:11542304 Object bytes,SHA2562546a582ab601a30ef1375b4feb1346b6aee356c68f83bb362df75da501c6ae3; full metadata1704995 bytes,SHA2567dfdbc9a9c96b334c71f71b5b8d19ea616f33da12d2a9a64fc930eeb22acc21f. There are still190 selected regions(45 read-only,145 writes),26 private mixed results and zero reported successful-backedge helpers. Existing code-size tradeoff remains; no new generated-code growth or size optimization occurs. Full identical metadata remains losslessly archived in the round15 packet; current emission commands, receipt and byte-identity comparison are retained. This is current Object/code-generation evidence, not a rebuilt linked executable.

## Profile and next action

The new1348-sample disjoint partition is479 other invocation helpers,263 frame actions,239 ordinary visits,227 selected library calls and140 remaining. Under the ordinary cached-call entry,339 weights still take the uncached validated path:198 in `bind_values`,138 in ordinary invoke and3 in retirement. The prior values were422=192+225+5 over1354 samples. These disjoint subtree subdivisions are not inclusive sums or exact elapsed-time attribution. Original and current raw sample/analysis are retained.

Repeated current-call binding is now the stronger concrete target:198/1348≈14.7% of sampled ancestry. Investigate immutable descriptor/schema preparation at the existing prepared-call owner with a fresh-record/value/shape/finite-denial witness before changing it. Preserve factory choice, current values, admission and cleanup; do not bypass the conservative optional-cache headroom guard or change NoLimits. Wider parent/child fusion remains a larger later task. No further semantic optimization precedes review of this patch.

The previously retained pinned ARM64 reference disassembly shows ordinary typed load/add/store/backedge patterns; those general patterns remain context for moving immutable metadata out of execution. This packet adds no named source→hashed-procedure mapping or new reference per-function timing claim.

## Failures, omissions and publication

No introduced compiler or scoped-test failure remains. An initial formatting discrepancy in newly added unit cases was corrected before qualification; its diff is retained. Expected injected-unwind diagnostics and the existing extend-35 default-resident compute denial(required16842945,limit16777216) are passing regression cases, not newly exceeded benchmark allowances. Primary has no resource denial and keeps its existing maximum diagnostic policy.

**Inherited unresolved observation:** round10 decoded-U64 scalar ratio1.0526036>1.05 remains explicit. These passing fresh controls do not erase it; scalar retirement is not declared complete.

Skipped: large/full-input timing, all-certified-scalar-primitives requiring catalog-omitted logic_not/logic_xor, new generated runtime-only linked application, four GPU cfg(test) private-fuel units and rerunning unrelated text-frontier units unchanged by this slice. Prior round15 text/linked/history receipts remain historical. Original/decoded execution, actual Engine budget/frame tests, public component fuel/recovery, selected scalar/bit tests and current Object/metadata are current-head evidence. Prior assertions/corpus/CI migrations and parser packages are not reopened.

Draft status remains. CI is intentionally cancelled by user direction and is not green; the exact publication SHA, cancellation run and new `@codex review` request are recorded in the publication/final response. The full1× goal remains active at127.863609× for this eight-byte case.
