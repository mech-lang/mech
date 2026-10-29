# Current-Boolean numeric version proof: review packet

Round base: `8aefbd86685850d1d6558514ee059f80ffd8936c`, reviewed in [receipt6488884664](https://github.com/mech-lang/mech/pull/973#issuecomment-6488884664). Production compiler: `5cc14c28ccf786bceeb3ea0db7448530caed537c`. Final source/test checkpoint: `53908beb79b558aa4da2ab4a5faefb2ceef68c8b`; this differs only in the corrected cfg(test) fixture. The publication commit adds this evidence packet; its exact head is recorded in PR #973 and the task's final response. PR parent: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`; branch `codex/level12-compression-consolidated`, worktree `/private/tmp/mech-deflate-linear-stack`. The separate main checkout is untouched.

## Result

**Roc=1×:213.066031×→142.091837×,33.310891% less elapsed (1.499495× speedup).** Complete eight-byte compression medians61.576083ms→41.064541ms; same fresh pinned reference median0.289000ms from20000 observations. Three primary observations per control. Warm-up and the separate98-repetition profile are excluded. The result is a small-case baseline; no confidence interval or larger-input throughput is claimed.

All observations preserve exact ten-byte output, independent raw inflation, capacity13 and final zero bytes/nodes. Output backing still changes on both controls; those boundary copies remain inside the timer. There is no output-reuse improvement in this patch. Primary measurements use the pinned mimalloc2.1.7 object and unchanged evaluate_into-only harness, with no simultaneous compiler/test/benchmark. System/probe component/scalar measurements are separate.

Source revision remains `671077457ff7ef3a3d44caf715a3fc595addf379`; reference compiler `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`, executable SHA256 `3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`. Caller source SHA256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode SHA256 `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. The25 components, algorithm/tables,262144-entry U16 tree,1500513-word U32 cache,depth300/nice258,near-EOF limits, six Huffman initializers, allocation sites and timing boundaries are unchanged. Mech performs the compression. No larger full-compression input was run.

The real n288 cumulative witness drops11227 ordinary visits→10 and5540 allocation requests→1549 (1084797→170173 requested bytes). Original1.309125ms→0.078292ms; decoded1.306375ms→0.077667ms. Those are single component observations using System/probe, excluded from primary. The generic prefix witness is selected too, and produces independently checked results. Baseline and current logs retain alphabet19/32/64/288, zero/one/short paths, fresh arguments and aliases. No exact primary per-function cost share is inferred from these components or the41 static sites.

## Design and maintained architecture

`NativeProgramFacts` now owns two descriptive mechanical operations: one actual identity-Copy definition, and Bool branch edges normalized to(false,true), including true/false literals on either side. Text frontiers reuse them while retaining complete-use, unique-root, iteration and lifecycle policies. Numeric eligibility supplies its own current-traversal availability policy. The facts grant no execution or ownership authority and carry no runtime truth value.

The numeric version owner retains ordinary forward must-analysis. It first validates the full original acyclic traversal. On a freshness miss only, it considers one repeated Boolean at a time. A candidate and every identity-copy link must have one immutable definition in the same traversal and dominate its reader without following the latch backedge. Both truth values must independently pass the existing freshness transitions and every reached recursive/result boundary. Other branches keep the conservative meet. No exponential valuation product, arbitrary proof cutoff, source-name selection, new executor, owner pin, current-value cache or successful-admission cache is introduced.

The existing provenance/root checks, selected factories, emitter, current owner/shape/bit validation, first-change copy admission, errors, cleanup, publication and fuel paths remain. A phi alternative, unrelated predicate, discarded update or real old read cannot gain current-array authority. Every recursive visit reexecutes its Boolean definition with current scalar/frame arguments. Unselected EOF paths do not perform the checked step.

IR119 fingerprints changed native eligibility and rederives it on restored plans. ABI35, bytecode14 and work policy93 remain. The existing policy93 closed-tail envelopes fund newly selected regions; elided ordinary frame/library transactions cease to debit physical canonical construction. Whole work527732164684→303735923752; no work/memory/copy allowance was increased. Old native identities remain rejected by the existing versioned-image contract.

## Current validation receipts

**332 unique scoped Rust passes**, excluding the eight-test focused rerun already included in the153-component cohort:

| Scope | Actual executed | Result |
| --- | ---: | --- |
| Maintained program/component cohort |153|Pass; exact prior results/allowances retained |
| Actual Engine test root: text frontier/availability/retention, shared predicates, budget/worker/projection/frame recipes |168|Pass |
| Selected native scalar correctness |8|Pass |
| Integer bits |3|Pass |

The eight new focused tests cover generic and unchanged pinned selection in original/decoded artifacts; source aliases and false-first arms; alternating recursive Bool arguments and fresh repeated calls; stale reads/discarded updates; distinct predicates/phi alternatives; ordered late assertion/load/division/overflow failures; unselected EOF checks; preserved input aliases and prior output storage; exact/one-short work, first-copy denial, cleanup and success→failure→retry. The shared-facts unit additionally exercises actual Copy definitions after normal source alias elimination, duplicate definitions, both literal polarities/orientations and coincident successors. The previous cumulative-decline test now requires native selection with exactly its original expected result and inputs.

The153-test maintained cohort includes real-table state components, separate search and skip, Huffman, capacity behavior, ordinary argument transport, existing mixed/private result protection and public fuel/clone/publication/recovery tests. Broad-cohort diagnostics ran while the Engine unit root compiled; their elapsed times are test-run receipts, not protected performance observations. The earlier focused component measurements and primary/scalar measurements ran separately.

**12 fresh scalar latency/allocation pairs pass**, max after/before1.009924215 below1.05. Requests/bytes match exactly; zero-versus64-iteration allocation slopes remain zero. **12 architecture/catalog/inventory/unsafe/warning/format checks pass.** Warnings remain errors. The production build is nightly2026-03-03,aarch64mac,release,opt3,jobs1,incremental0,locked/offline, using `benchmarks/mech-deflate/native-profile/Cargo.toml` and only the five required arithmetic machine families. Exact dependency paths/hashes, feature cfgs and commands are retained. Production build450.535s; final actual Engine unit-root build212.895s,168-test run2.68s displayed. The separate component cohort ran153 tests in150.28s displayed. No global/default quota was changed.

Principal commands (full rustc arguments/features and wrappers are in the receipts):

```text
cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review
python3 /private/tmp/mech-predicate-versions-round15-20261029/primary/run.py
python3 /private/tmp/mech-predicate-versions-round15-20261029/full-components.py
python3 /private/tmp/mech-predicate-versions-round15-20261029/engine-units-final/build.py
python3 /private/tmp/mech-predicate-versions-round15-20261029/primary/qualify-selected.py
python3 /private/tmp/mech-predicate-versions-round15-20261029/primary/scalar-controls.py
python3 /private/tmp/mech-predicate-versions-round15-20261029/object-audit/build.py
python3 /private/tmp/mech-final-static-20261028.py
```

The complete Object and its full metadata pass the existing successful-private-CFG audit:190 selected regions,45 read-only(all parented),145 writes(142 parented),26 private mixed results. Successful backedges report zero helpers. All41 former first-recorded stale-return declines in this exact artifact now qualify; their equal static count is not a timing attribution. Other130 declines retain their original categories. Object SHA256 `2546a582ab601a30ef1375b4feb1346b6aee356c68f83bb362df75da501c6ae3`; full metadata SHA256 `7dfdbc9a9c96b334c71f71b5b8d19ea616f33da12d2a9a64fc930eeb22acc21f`. Full generated metadata is archived as `qualification/object-audit/complete-object.json.gz` to keep the review diff small; its decompressed hash is given above. This is Object/code-generation evidence, not a new linked executable run.

**Code-size tradeoff:** full Object7748568→11542304 bytes(+3793736,+48.960%). Actual `__text`7747836→11541572 bytes; constant link identity stays192. The new native paths increase generated code while the preserved ordinary paths remain. No additional code-size optimization is folded into this patch.

The1354-sample disjoint profile leaves561 other-helper,253 frame-action,232 ordinary-visit,191 selected-library and117 remaining weights. Frame-input admission is94. Prior1347-sample partition is independently reproduced. The largest category is now other invocation helpers(41.4%). Sample ancestry is descriptive, not exact removable time or exclusive named-source attribution. Pinned ARM64 reference disassembly was reexamined: ordinary checked load/add/store/backedge patterns support typed native execution, while compiler-hashed procedures are not claimed to map to a named cumulative helper. No reference source/binary was rebuilt.

## Commits and failure disposition

- `70a72003645a235f97518ba63c4139dc796de384`: preserve fresh baseline,32 functional observations and expected four-way selection rejection before the production change.
- `5cc14c28ccf786bceeb3ea0db7448530caed537c`: shared mechanical facts, bounded current-Boolean proof, IR119 and selection/failure/retry regressions.
- `193f87de471a975079ec8cf049086b41404619ec`: immutable paired timing/profile and focused receipts before any further optimization.
- `53908beb79b558aa4da2ab4a5faefb2ceef68c8b`: correct the new actual-Copy unit fixture after source alias elimination; production code is unchanged.
- Publication packet: all current qualification/source/binary hashes, full commands, counts and this closure.

Introduced test-construction failures are resolved and original failed receipts retained: ten Vec→boxed-slice errors in newly written test inputs, then one new shared-facts test expecting an alias Copy that normal lowering eliminates. The corrected test constructs an actual unique Copy and a duplicate-definition counterexample in the descriptive plan. Its final actual-root168-test run passes. The intentional baseline red selection test is green on the current compiler. No introduced compiler or scoped test failure remains.

**Inherited unresolved performance observation:** round10 decoded-U64 scalar ratio1.0526036 exceeded1.05. This round's passing12 controls do not erase that record; scalar retirement is not declared complete.

Skipped this round: larger/full-input timings; the all-certified-scalar-primitives test requiring catalog-omitted logic_not/logic_xor; rebuilding the optional generated runtime-only linked application; and the four GPU cfg(test) runtime-fuel units. Earlier completed linked/private-fuel evidence remains historical, not a new-head pass. JIT original/decoded execution, current public component fuel/clone/error/recovery coverage, Engine tests and current Object emission are the new-head evidence. Earlier assertion recording, corpus/CI migrations and parser milestones are not reopened.

The PR remains draft. New CI will be cancelled at the user's direction and a fresh exact `@codex review` comment posted. Cancelled CI is not represented as green; the publication receipt/final response records the exact head and run. There were no new environmental/resource denials in the protected campaign; expected default resident-limit and injected-unwind messages belong to passing tests.

## Next bounded investigation

Inspect remaining ordinary invocation helpers and repeated current-account/binding/descriptor preparation using this retained profile and a generic unchanged-operation witness. Rank their actual dynamic cost before changing code. Preserve current owners, bits, shapes, alias/failure behavior, finite denied prefixes and scalar controls. Native parent/child execution remains a later, larger ownership/control task. The pinned source stays unchanged; no optional tuning starts before this patch is reviewed. The whole-program1× goal remains active at142.091837× on the measured eight-byte case.
