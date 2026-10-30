# Bounded native parent/child composition

Status: **implemented, locally qualified, measured and retained for review**. The PR remains draft. Hosted CI is cancelled by user direction; no green-CI claim.

## Outcome and comparable timing

Complete protected eight-byte compression, pinned allocator and unchanged evaluation-only timer:

| Same-session result | Before | After |
| --- | ---: | ---: |
| Elapsed | 5.1046875 ms | 5.0451670 ms |
| Fresh reference = 1×, 0.279 ms | 18.29637097× | 18.08303584× |
| Fixed prior-round 0.300 ms ruler | 17.01562500× | 16.81722333× |

Elapsed reduction **1.16599694%**. All four independently prepared worker pairs improve, ranging from 0.74% to 1.79%; 55/64 descriptive blocks improve. Median block ratio is 0.99197288; geometric ratio 0.98656702. Both evaluation-order strata improve. The blocks share four pairs; they are not 64 independent experiments. This establishes a modest gain on eight-byte latency, not throughput or parity.

All 3984 observations are retained, including 3968 primary calls; fresh reference has 20000 observations. Every reference/worker-pair/profile/control packet was committed immediately. Other builds, tests, audits, profiling and JIT preparation did not overlap timed evaluation. Each observation checks the exact ten reference raw bytes, independent inflation, input aliases and contents, capacity 13, expected work and final zero cleanup.

The earlier `primary/` packet is **provisional and superseded**: one complete worker pair before the additional single-use continuation checks, then an interrupted second pair during preparation when the new brief arrived. It measured 5.081500 → 4.9922705 ms against 0.288 ms. All its observations and partial preparation are retained. It is not pooled into confirmation. The confirmation's faster 0.279 ms reference explains higher normalized figures even when elapsed time improves; cross-session normalized rows are not a paired series.

## Exact checkpoint and pins

- PR stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
- Reviewed starting head: `94e732bddb0a709a35cdb884e6d553141235f0cb`.
- First connected compiler checkpoint: `fe9d2a93b`.
- Production compiler with single-use continuation: **`2de2b627d1656c50adfc99ab75f4f3242e354c29`**. Later changes are tests, audit registration, formatting and evidence.
- Final semantic/audit checkpoint: `59950eda3`; final formatting `2a61d0dd5`.
- IR/work/ABI/bytecode revisions: **131/95/37/14**.
- Toolchain nightly 2026-03-03, aarch64 macOS, locked/offline, jobs 1, incremental disabled. Production GPU opt 3; maintained diagnostic/integration/unit roots opt 1, warnings as errors. The production six-feature GPU configuration and seven non-GPU library hashes agree between variants; see archived build commands.
- Program revision: `21f49ac3f2497247ed2e2fbe67344640392d84a19f780d8772addcd505d396c1`.
- Compressor source SHA256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
- Canonical bytecode SHA256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7` (4138627 bytes).
- Reference package `671077457ff7ef3a3d44caf715a3fc595addf379`, compiler `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`; executable SHA256 `3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`.
- Pinned mimalloc 2.1.7 Object SHA256 `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.

Algorithm, tables, search limits, depth 300, nice length 258, near-EOF limits, real 262144/U16 tree and 1500513/U32 cache, source allocation sites and output clear/rebind/timer boundaries remain unchanged. No complete larger-input run was attempted.

## Execution contract

Prepare qualified leaves first, then derive a bounded second pass for one immediate leaf child. Parent fragments are read-only with respect to array payloads: scalar operations, unsigned reads and flat frame transport. Writes stay in the existing replacement-only child. Multiple children, deeper compositions, append/capacity changes, maps, activation boundaries, ambiguous forwarding and unsupported escapes retain ordinary execution.

A fragment excludes child initialization and exit, parent lifecycle sites, and managed-projection boundaries. Its exact single-entry acyclic source graph, every frontier, current scalar words, managed source roots, result mapping, exported live frames and original ordered retirements are derived from canonical facts. The selected child receives its arguments through its original initializer; its original first-write, completion, exit admission/actions and successor remain authoritative. The suffix acquires the returned current array and index. Conditional bypass carries its own values; the child does not dominate terminal exit.

Each fragment opens the existing single closed-region slot, validates current owners, funds a certified native envelope from the same invocation ledger, and closes before any ordinary helper or child. The regular source entry pays ordinary admission; the additional entry preview and independently checked native instruction envelope are charged once for the selected fragment. Reached retirement actions retain their original prices/order. Insufficient funding or unsupported ownership declines before native fragment execution, at its exact unexecuted source site. No parent fuel or view survives arbitrary child spending. Bounds errors reconcile before the original failing selected operation; arithmetic errors preserve original origins and prior reached retirement effects. Failure publishes no partial outputs.

One **eight-byte, invocation-local pending edge ID** is armed at admitted child initialization and consumed only at the exact successor after canonical child exit/actions complete. It contains no owner or fuel. It rejects duplicate/mismatched completion and is discarded with the failed invocation. Native-parent/ordinary-child/native-continuation is covered. All owners remain in the inventoried original frame/arena banks. Persistent 1128 + invocation 1296 = **2424 bytes**, within the unchanged 2432-byte base envelope; the complete view-envelope test also passes.

Only live external frame results materialize at fragment exits. Existing last-use retirements remain unchanged; one shared maximum scratch inventory adds 66 words, not per-site matrices. Shared word/frame/address emission is factored from the maintained numeric emitter. No new interpreter, recursive proof engine, mutable owner class, cross-helper pointer loan, bytecode operation, source specialization or benchmark rewrite is introduced. Image loading independently derives the layout and rejects supplied composition authority or an undersized argument bank.

## Mechanism and costs

Fresh complete callback census: **14894 → 13678 admissions**, **9500 → 9156 helpers**. Before census is retained R29 instrumentation whose generated Object/selection stayed identical through R32; after census is fresh. Six reached parent instances select: iterations 70,119,190,239,320,417. Parent 190 contributes **848 → 446 admissions**, **610 → 496 helpers**. Its 290 old helper calls disappear while 176 begin/end boundaries are added, a net 114 reduction. Exact source sites, operations, readiness and child boundaries are archived in `mechanism/`.

Generated emission contains **39 parent pieces across 13 structural parents**, zero successful interior helper calls. All existing **239 numeric loops** retain zero successful backedge helper calls. Text grows 12923596 → **13801616 bytes** (+878020); Object 12924328 → **13802344**; stack 108032 → **108560** (+528). Compiler JSON metadata grows 2191167 → **2411445 bytes**. JSON byte size is not a descriptor heap-footprint measurement; exact Object/descriptor hashes are archived.

The minimal handoff allocation witness establishes **three payload allocations in both variants**, with no added copy: 7904 additional cells add 23712 bytes and requests remain 320 in original/decoded forms. The candidate adds **96 fixed transport bytes** in this minimal program. An initial one-copy assertion was disproved on both baseline and candidate, retained as counterevidence, and replaced by a regression against the observed baseline slope. No inherited copy-removal claim.

All 12 scalar controls pass the unchanged 1.05 ceiling, maximum ratio **1.016148966**; pure scalar allocation counts/bytes agree. All eight managed-frame controls retain counts, bytes and zero 0/64-step slopes. Every profiled observation is excluded from primary: 695 complete eight-byte diagnostic calls, zero cleanup. Fresh profile has **1351 exclusive weights**: frame actions 348, ordinary visits 246, other helpers 270, selected calls 121, remaining 366. This is sampling evidence, not exact elapsed fractions. Frame admission, retirement and ordinary control remain the next investigation area; full source/error/funding contracts still govern any extension.

## Qualification and exact commands

**574 unique scoped maintained cases**: runtime 308; accounting 12 + transport 1; unchanged component cohort 232; eight new composition tests + selected-image regression 1; selected scalar 8 + bits 3; actual runtime-only linked recovery 1. These are scoped results, not full-suite/hosted-CI results.

The maintained composition fixture covers original/decoded values and resident agreement; independent before/after input contents; current returned arrays/indices; reordered child results; distinct logical roots sharing physical backing; fresh Boolean recursive visits; zero/unselected children; completed empty calls distinguished from skipped calls; partial-write failure and a later child rejecting after an earlier child succeeds; errors before/after the child; unchanged prior publication; aliases; cleanup and retry. Exact/one-short control, library compute, comparison, memory, nodes and transport boundaries pass. Actual callbacks verify selected fragments, child initialization/exit counts and ordinary empty-child fallback. Forged image metadata is rejected and actual linked/AOT restoration executes valid → assertion rejection → valid with original publication and successful recovery.

Commands and complete feature flags/library hashes are in the adjacent archived JSON receipts. Core commands:

```text
cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review --features mech-runtime/compiler
<actual-GPU-unit-root> native_artifact::program::runtime:: --skip complete_unchanged_knucleotide_body_uses_the_owned_emitted_map --nocapture --test-threads=1
<actual-GPU-unit-root> native_artifact::work:: native_artifact::memory:: --nocapture --test-threads=1
<maintained-component-root> <unchanged 232-case filter inventory> --nocapture --test-threads=1
<maintained-component-root> numeric_composition_qualification restored_parent_segments_rederive --nocapture --test-threads=1
<actual-GPU-unit-root> composition_tests parent_continuation_is_single_use native_runtime_consolidation --list
<actual-GPU-unit-root> composition_tests parent_continuation_is_single_use native_runtime_consolidation --nocapture --test-threads=1
```

The exact CI selection lists and executes **five** unit cases; the integration fixture is already in the maintained scheduled target. The generated linked project's runtime closure has only GPU runtime features, no parser/code generator. Its exact generated catalog/symbol prefix and unchanged maintained retry driver execute inputs 3,0,2 and publish 4, unchanged 4 on rejection, then 6.

Routing, source catalog, host catalog, build layout, Rust module layout, factory safety, warning policy, planning quarantine and CI inventory pass. The unchanged unsafe audit passes after registering the exact test-only observer boundary; formatting and diff checks pass. No check, resource allowance or warning policy is weakened.

## All unsuccessful commands and limitations

- Introduced initial selection red: maintained positive expectation fails on reviewed parent, then passes after implementation.
- Introduced ordinary/native instruction double-count in the *emission audit*: fixed by separately auditing the already-funded native body, keeping fixed entry guards in the unchanged ordinary audit. Native envelope still checks actual emitted instructions against its independent price; no bound increased.
- Introduced file/directory module collision: fixed with standard `composition/mod.rs` layout.
- Introduced type/import errors in work-in-progress code and test helpers: explicit types/imports corrected; all raw failures retained.
- Two invalid negative fixtures failed unchanged Engine artifact validation before GPU preparation: corrected to valid unsupported shapes, without an Engine change.
- Introduced stale ABI36 test expectations: updated to exact ABI37 while preserving old-identity rejection.
- Introduced unregistered test callback observer: exact file/invariant added to the unsafe boundary registry; audit remains strict.
- New harness mistakes (same-account alias admission, undeclared input, nonexistent detailed divide error variant, stale linked catalog prefix, old stderr path): corrected; no production workaround. Actual final consumers pass.
- One-copy assumption: false on the retained baseline as well, as described above.
- Clean-checkout timing guard caught remaining test formatting before any measurements: committed formatting, then restarted normally.
- One provisional timing campaign was intentionally stopped during second-pair preparation for the incoming review brief; raw partial evidence is retained and not pooled.
- **Inherited/environmental reduced-catalog gap:** whole knucleotide stats test reproduces the same missing-module rejection on the retained R32 binary. The same baseline exclusion is retained. Selected not/xor all-primitives case is also outside this catalog. Broader Core/Engine/full-feature suites and larger compression inputs were not rerun.

No new implementation or qualification failure remains unresolved. Hosted CI is cancelled by user direction; latest publication status is recorded in the PR, and it stays draft pending green CI and clean review. The next bounded implementation requires its own acceptance witness; no further semantic compiler expansion is included here.
