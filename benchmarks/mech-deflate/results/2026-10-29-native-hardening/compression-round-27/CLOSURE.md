# Scalar aggregate results in native numeric loops

Round base **67aa9a252718a9ddc3fd2ddd9e41081499677335**, clean automated review6510975393. PR parent **eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b**. Parent reproducer/design **3c002232f5fb904bd7561c46a999de7ffe9d24fe**; production **562a15efb08345b7d2daad3cb3deb43805d9c9b3**; immediately committed primary/profile **493738b96e9e38a082083ebc6c73c7c035fc9b57**; immediately committed scalar controls **79c9068461eb454ac3cb31ac8e7a9e15902a84d8**; final focused tests **47d670101**. IR/work/ABI/bytecode **129/93/36/14**. Engine/Core implementation is unchanged. All measured production source remains byte-identical to562a15efb.

## Result and scope

Protected complete **eight-byte `abcdefgh`** compression, level12: **27.748924138× →20.259555172×Roc**, **8.047188→5.875271ms** against the same fresh **0.290000ms** reference/20,000 observations. **26.989763381% less elapsed time**, **1.369670948× speedup**. Four alternating pairs all improve: after/before0.736181533/0.728135954/0.729903600/0.730207186. Twelve primary observations/control; all40 observations retained. Warmups and separate681-repetition profile excluded. This does not establish larger-input throughput or parity.

Same Apple M1, pinned mimalloc2.1.7 and unchanged evaluate_into-only harness. Every observation verifies exact10 raw bytes, independent inflation, aliases, capacity13 and final zero bytes/nodes. Output reserve/clear/rebind, compilation, verification and drop remain outside the timing boundary. Remaining execution copies stay timed. No compiler/test/audit/other benchmark ran concurrently with primary/profile or scalar measurements. Both complete raw packets were committed immediately, before further optimization. [Primary](timing/manifest.json), [scalar](scalar/manifest.json).

The attachment's managed named-record result work landed in round24, reserved appends in25 and structural alias retirement in26. This round addresses the next reached inconsistency: the native numeric result proof demanded an array-owning BufferFrames result even when NativeScalarAggregates already held the exact private scalar pair.

## Compiler design and execution evidence

`ResultRecipe::Aggregate` requires the existing connected private-use proof, exact flat scalar tuple/record schemas, no dimension parameters and the exact existing aggregate word count. Numeric body eligibility composes those same aggregate actions. Native emission calls the shared Pack/Copy/Project/Observe emitter through SsaWords, using the existing global word bank. The already funded exit/failure/header paths spill it. No runtime result owner, canonical Value, cache, executor, mutable pointer or ABI lane is added. Managed result/frame provenance, current-version validation, private-array ownership, publication and failure cleanup remain unchanged. Unsupported nested/enum results and managed flow retain the existing route.

Image loading independently rederives both the numeric proof and the serialized scalar word inventory. The latter comparison closes the otherwise inconsistent slot dependency exposed by the new regression. It preserves exact lane offsets even when a corrupt inventory keeps plausible counts. No source work price or finite allowance increases; IR changes because selection/proofs change.

| Iteration/header | Unchanged source function | Result | Existing result slot | Reached before/after frame-site helpers |
| --- | --- | --- | --- | ---: |
|142/13110|optimal-default-frequencies|(I64,I64)|22994,2|2524/0|
|424/38158|block-cost-literals, first call|(U64,U64)|23256,2|2311/0|
|425/38251|block-cost-literals, second call|(U64,U64)|23264,2|1799/0|
|426/38367|block-cost-lengths|(U64,U64)|23272,2|645/0|
|427/38521|block-cost-offsets|(U64,U64)|23280,2|546/0|
|429/38883|block-write-precode-lens|(U64,U64)|23292,2|0/0, unreached|

All six change from `numeric result frame escapes or lacks exact private fields` to eligible/selected. Native numeric regions233→239 (read-only45→51, writes188). Every emitted successful backedge has zero calls. All five reached containing iterations have zero ordinary admits/helpers after selection. Raw producers/consumers, exact schemas and source coordinates are retained in candidate-mapping/mapping.json in the archive; [summary](qualification/candidate-summary.json).

The actual checked complete-program census records visits **33497→14894**, helpers **19255→9500**, helpers at frame-labelled sites **14981→7161**, work **67079241202→45060848918**, comparison **48826→47856**, transport bytes **4757264→4758520**. The five old frame-helper subtrees total7825; five new ClosedTailBegin helpers on frame-labelled Initialize sites explain the net7820 decrease. This frame-site counter includes boundary helpers; it is not a direct count of buffer_frame_action invocations. Counts are not elapsed attribution. Each route's measured and diagnostic work totals agree, with unchanged prices.

Generated Object text **13561884→12923596 bytes** (638288 fewer); whole Object13562616→12924328; stack **105296→108032** (2736 more). [Sections](qualification/object-sections.json). Before/after separate samples have1294/1283 weights. After partition:337 frame,115 selected call,226 ordinary visit,272 other helper,333 remaining. These disjoint sample weights are not precise time percentages or promised removable fractions. [Profile](qualification/partitioned-profile.json).

Mechanism-only C/Rust checked two-accumulator folds compile to direct loops and x0/x1 returns, with cold trap/panic differences explicitly retained; zero control executions or elapsed comparisons. Inspection of the actual pinned reference binary records a typed register loop at100018044–100018064. Its exact anonymous source function was not established; no exact scalar-pair mapping is claimed.

## Qualification and failure ledger

**546 fresh maintained Rust passes**: GPU runtime301 + exact transport-admission1 + closed-fuel1; component cohort232; selected scalar8; bits3. Final focused7 overlaps those tests. The232 cohort ran before moving image checks into the existing audited fixture; all changed paths pass the final focused7. Production source is unchanged. Engine227 and Core353 earlier passes are not counted fresh. No existing component expected value or allowance changes.

New tests cover scalar/pair control, tuple/record schemas, all seven scalar kinds and exact float bits, fresh recursive arguments and toggled Bool, repeated/changing calls, zero allocation/visit slope, mutation then assertion/arithmetic/load failure, order precedence, aliases, unchanged prior output, cleanup/retry, exact and one-less finite work, original/decoded execution, image roundtrip and corrupt proof/lane offsets. The generic pair's visits become constant10; record11 through256 iterations. Allocations remain721 requests with a fixed116322 bytes, versus115098 on the parent: **1224 extra setup bytes**, not zero allocation.

All12 protected scalar controls pass the existing≤1.05 criterion; maximum direct ratio1.002309817. Requests/bytes are identical and loop allocation slopes zero. All12 architecture/catalog/inventory/unsafe/warnings/format/diff checks pass. Unsafe allowlist and warnings-as-errors are unchanged.

Exact command/configuration receipts are in the [205-file archive manifest](qualification/manifest.json). Production builds use `cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review`; test dependencies add only `--features mech-runtime/compiler`. Nightly2026-03-03, aarch64mac, jobs1/incremental0. Rebuilds71s/~72s, runtime-test dependencies6.39s, outside timing. All seven production feature sets agree; only the GPU library changes from round26. Maintained integration bodies compile opt1 and actual GPU unit roots opt0 with-Dwarnings, exact Cargo artifacts and source hashes. The diagnostic wraps actual checked callbacks and delegates unchanged; it is outside the runtime test selector and adds no performance observations.

Failed attempts, retained without overwriting observations:

- Clean-parent new requirements:0 passes/2 intended failures; scalar selection passes, pair eligibility and visit slope fail. Values/aliases/resident agreement pass before rejection.
- First implementation focused check:4 pass/2 fail. New bit witness used small-width arithmetic absent from the focused catalog (MissingSemanticWitness Node11); it now transports those bits directly. New image-slots corruption exposed the inherited missing inventory comparison; production closes it. The omission is visible in the unchanged parent loader; no separate parent corrupted-image execution is claimed.
- Expanded pinned resident comparison:6 pass/1 fail, default resident helper ComputeWork required16806089>16777216. This is the inherited finite reference policy, not native execution denial. Focused resident agreement now uses the original terminal boundaries; full length/offset/default computations still execute under the unchanged independent-oracle components and protected complete program. No allowance increases.
- First architecture run rejects unsafe loader tests in a new file. Tests now extend the existing audited canonical_program_buffer_frames fixture; allowlist unchanged, final12 pass.

No introduced failure remains unresolved. Known inherited resident public nested-record Node2InvalidInput and alternate geometry Node5WitnessTypeMismatch remain as previously qualified on the clean parent; not re-opened or freshly retested. Existing matchfinder resident default-budget denials remain recorded by its unchanged test expectations. Skipped this round: larger/full-input timing, new linked/AOT execution, all-primitives not/xor omission, absent-stats whole-knucleotide unit, Engine/Core reruns and tests outside these cohorts. CI is not green: publication remains draft, new CI is to be cancelled by user direction and an exact-head review requested.

## Source pins and next step

Compressor revision **671077457ff7ef3a3d44caf715a3fc595addf379**; reference compiler **d99c7b838fb05bb1047eb0c18d4d06d0f86f345d**. Source/artifact SHA256 **331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9** / **a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7**. Fresh source compilation reproduces all4138627 measured bytecode bytes; decoded reencoding agrees. Algorithms, tables, depth300/nice258, near-EOF rules, real262144/U16 tree,1500513/U32 cache, source allocation sites and timing boundaries are unchanged. Mech performs compression.

The profile and reached census now point to remaining parent/child boundaries. The top reached frame-helper iterations25/21/190/168/170/220 all reject that boundary; static76 alone is not the justification. [Reached candidates](qualification/next-hot-candidates.json). Next bounded work is to map one real child boundary and establish a minimal parent/child witness before extending the existing native suspension/ownership/funding protocol. No next optimization has started. The1× objective remains active.
