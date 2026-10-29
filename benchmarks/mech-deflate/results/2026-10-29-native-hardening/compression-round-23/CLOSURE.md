# First-violation call budget checks: review packet

Base **545e17bf065028512c93a2bdc8677fbc3728a3fa**, PR parent **eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b**. Production **299c70f4ce21eb3cba94dcb47e4d37106cc14b29**; Core/code-generation evidence **3a6953921**; reference-only failed-attempt packet **e6a1749ee**; complete timing/profile committed immediately **2ad6e5021cdd35e60b7a16c7615dfdf9d276721b**. The PR metadata supplies the publication head. No production change follows299c70f4c. IR/work/ABI/bytecode remain125/93/36/14.

**Roc = 1×: 67.315086207× → 65.708479310×**,19.521375ms → 19.055459ms, fresh common reference0.290000ms(20000 observations). **2.386697% less elapsed;1.024451× speedup.** This is a modest gain and leaves a large gap to1×. All four balanced pairs improve; twelve primary observations per control. All40 process observations and the separate210-repetition sample remain in [the timing packet](TIMING.md). No compilation,test,audit orother benchmark overlaps protected timing/profile orprotected scalar controls. Every observation matches exact ten raw bytes, independent inflation,capacity13 andfinal zero bytes/nodes. Output backing changes;copies remain timed. **Logical work is identical at222049095608**. No primary resource denial orlarger-input throughput/confidence-interval claim. Largest complete input stays eight bytes.

The source,artifact,reference,allocator,harness and library hashes are in the primary manifest. Pinned source/bytecode,all25 components,algorithm/tables,262144-entry U16 tree,1500513-word U32 cache,depth300,nice258,near-EOF rules,source allocation sites andevaluate_into-only boundaries remain unchanged. Mech performs compression. JIT/setup/seed/clear/rebind/verification/drop remain excluded; same mimalloc2.1.7 anddiagnostic NoLimits. Previous67.879005245× used a fresh0.286ms reference; paired normalized series are separate.

## Bounded implementation and binary evidence

The observed cost is the shared budget evaluator under private frames and selected collection reads,lengths,writes andappend operations. Eight existing runtime sites requested evaluate_call_memory_budget(...).first(). They now use first_call_memory_budget_violation, an inlinable sink over **the same ordered core checks**. Complete-diagnostic callers still receive the complete sorted list; aggregate admission still excludes call-scoped output/work totals. Error owner,dimension,required,limit andpriority are preserved. The first-only path stops without constructing later diagnostics. Success never allocated an empty Vec before; no successful heap-allocation saving is claimed.

Current arguments,shape validation,account membership,call caps,work debits,owners,publication andcleanup remain at their existing positions. No cached current value,owner,visit grant orsecond executor is added. There is no algorithm-specific shortcut orsource change. This is a runtime lowering improvement, not a new source-language construct.

The optimized API witness emits the same six ARM64 instructions as clang -O3 C for two dynamic limits; the complete API wrapper has a304-byte stack frame andexternal call. This is mechanism evidence,not a compressor benchmark. Actual measured executables also show that NativeProjectionIndexContext::check no longer calls the complete evaluator. Its retained caller grows90 → 142 instruction lines as checks inline; no code-size reduction is claimed. Exact function disassembly,commands and executable hashes are in the qualification archive. Pinned reference bounds/load/store excerpts are retained without relabeling hashed symbols as named source functions.

The fresh sample has1315 disjoint weights:381 frame actions,280 selected-library calls,266 ordinary visits,202 other helpers,186 remaining. The complete budget evaluator falls58 → 18 exclusive weights in separately sampled profiles. These shares are not guaranteed removable elapsed time. The remaining frame/helper/control traffic is substantially larger than this isolated budget cost; the next optimization should target reached loop/call shapes and remove more repeated transport/dispatch, with a minimal witness before expanding compiler support.

## Qualification and disposition

**797 unique scoped maintained Rust tests pass:**349 Core,227 Engine,210 full component,8 selected scalar,3 bits. Focused/subset runs are not counted twice. Core's four budget tests include independent complete/first expected diagnostics for every2048 subset of11 limit dimensions and None/equality/u64/u32 maximum cases. All Core unit tests execute from the unmodified crate root. Engine tests execute from its actual crate root and cover finite accounts,nested call caps,unwind restoration,frame admission,projection demand,collection templates andworker debits. Printed injected panic messages are caught passing unwind fixtures.

The full component cohort passes210/zero failures with original expected values andallowances (77.99s test,78.857s process). Original/decoded/image coverage retains exact pinned compressor components,current arguments,scalar kinds,aliases,finite rejections,assertion/arithmetic/load order,publication,cleanup/retry andstrict image restoration. The selected scalar8/bits3 pass separately. All12 protected scalar latency/allocation controls pass unchanged≤1.05(max1.010512530971770),with exact requests/bytes andzero0→64-step allocation slope. No follow-up rerun selects a better guard observation.

All12 architecture/catalog/inventory/unsafe/warnings/format checks pass in195.964s. Warnings remain errors and the unsafe allowlist is unchanged. A separate non-native-artifact Engine metadata build passes, preserving the feature boundary. Cargo artifact inventories prove Core,Engine,GPU,Stdlib,Math,Compare andLogic feature sets identical to the parent default production profile. The rebuild takes1017.391s because Core changes invalidate dependent providers; it does not activate more math operations ormech-build.

Configuration: nightly2026-03-03,aarch64mac,locked/offline,jobs1,incremental0; default production dependencies opt3; external maintained test harness opt1; actual Core/Engine unit roots opt0 with the same optimized dependencies. The Engine unit build starts after Cargo reports completed Core/Syntax/Bytecode/Engine artifacts,while remaining provider crates build; its receipt records that producer state and exact dependency hashes. Full Cargo completion andall compiler/test termination precede protected timing. Core source tests compiled before the production commit; their recorded budget source hash matches the committed file. No source edit follows those tests.

```text
cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review
python3 /private/tmp/mech-budget-checks-round23-20261029/core-unit-build.py core-units-final
/private/tmp/mech-budget-checks-round23-20261029/core-units-final/core-unit-tests --nocapture --test-threads=1
python3 /private/tmp/mech-budget-checks-round23-20261029/engine-units/build.py
python3 /private/tmp/mech-budget-checks-round23-20261029/engine-units/nonnative-check.py
python3 /private/tmp/mech-budget-checks-round23-20261029/components/qualify.py
python3 /private/tmp/mech-budget-checks-round23-20261029/primary/qualify-selected.py
python3 /private/tmp/mech-budget-checks-round23-20261029/final-primary/run.py
python3 /private/tmp/mech-budget-checks-round23-20261029/final-primary/scalar-controls.py
python3 /private/tmp/mech-final-static-20261028.py
```

[Qualification manifest](qualification/manifest.json) maps each receipt to its compressed archive member/hash. [Pre-timing manifest](pre-timing/manifest.json) retains Core tests andcode-generation witnesses. [Primary manifest](final-primary/manifest.json) retains the exact protected commands,all raw outputs,build bindings andprofile. No binary orObject file is committed.

Two initial new test builds fail because the mask closure's unsuffixed integer literals infer incompatible types; the explicit u16/usize/u64 witness fixes test setup. The first timing attempt then fails before spawning any Mech process because copied executable mode was0644. It measures only the reference(20000 observations,0.284ms),and is retained andcommitted immediately in [the aborted-attempt packet](ABORTED-TIMING.md). Corrected copies preserve execute mode andthe exact qualified compiler/library bytes,then use a new reference andfour fresh pairs. Neither issue is a compiler semantics regression orresource-policy denial. No failed Mech observation is replaced;none exists in that aborted series.

No current introduced compiler/component/scalar/architecture/timing failure remains. The inherited resident public nested-record failure andhistorical scalar timing observations remain documented in [round22](../compression-round-22/CLOSURE.md) and [round20](../compression-round-20/CLOSURE.md). The absent-stats GPU test remains outside this selected catalog. No previous assertion/corpus/CI/parser package is reopened.

Skipped this round: larger/full-input timing; newly linked/AOT application ornew Object audit (no generated-program lowering change);all-certified scalar primitives needing omitted not/xor;the303 GPU internal tests last passed round22,including all four private-fuel tests;stats-dependent coverage outside the selected profile. Current broad Core/Engine/component andscalar receipts are not described as a repeat of all303 GPU units.

The parent545e17bf06 has a clean automated review(comment6504019242). A fresh exact @codex review is requested on publication. New CI is cancelled by user direction,not green; PR #973 stays draft. The1× goal remains active.
