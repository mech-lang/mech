# Private multiple-array numeric returns: review checkpoint

Base **430efcbaa1f193b1c2c4186fff86828d6d176eb9**, on PR parent eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b. Reproducer **fe3cd78cde10e70a7b428ab1237cd239045a25b8**; production **337c0d307d370d0f278706d1d772739d1cafd174**; focused qualification **f56bf105095a30f7ceb1ffff0839c05d78997171**; timing/profile committed immediately **89d6a05fe29adabcdc8c8a267ea6531129f8b13a**. Final packet also updates corruption-test JSON paths and includes all returned associations. No production change followed the frozen build. Publication head is reported in the PR metadata. IR124/work93/ABI35/bytecode14.

**Roc=1×:102.667470175×→73.331359649×**,29.260229ms→20.8994375ms against the same fresh0.285000ms reference(20000 observations). **28.573910% less elapsed;1.400049× speedup.** All four alternating-order pairs improve. [Timing and raw profile](TIMING.md) retains all24 primary observations,8 warm-ups,8 excluded profile-tagged observations and the separate192-repetition sampled process. No compiler/test/other benchmark overlaps protected primary or its profile. Every observation has exact10 raw bytes, independent inflation,capacity13 and cleanup0 bytes/nodes. Output backing changes; copies stay timed. Work306669833472→222038733432 reflects selected routes under existing prices/allowances. Largest complete input8 bytes. No larger/full-input throughput or confidence interval is claimed.

The pinned source, bytecode, algorithm/tables,262144-entry U16 tree,1500513-word U32 cache,depth300/nice258,near-EOF rules,source allocation sites and evaluate_into-only timing boundary are unchanged. Mech performs compression itself. Source/artifact/reference/allocator/harness/dependency/binary hashes are retained in the timing packet. The unchanged frontend's fresh source/decoded byte-for-byte compilation was last checked at the parent checkpoint; this round executes the unchanged artifact. No source edit is used to obtain this result.

## Bounded design and blocker evidence

Exact source requirement: huff-place-symbols in hosts/gpu/tests/fixtures/native/deflate-huffman-level12.mec:82–86 returns counts<U64>,lens<U8>,a<U32>. Parent numeric selection rejects25 candidates with `numeric result tuple requires exactly one array owner`. The red test first establishes exact resident/independent results, aliases/failure recovery and ordinary-route observations, then fails on this restriction.

ResultRecipe::Frame now stores a bounded association for **every** managed result field: returned ordinal→exact carrier-array root. Each must have the exact schema. Records/nested composites, unknown roots and public escaping frames retain fallback. Repeated roots are legal aliases. The existing version lattice checks every returned ordinal at the terminal site on every feasible path; the existing repeated-current-predicate proof is reused. A stale second result cannot be justified by a current first result.

No runtime owner or executor is added. Existing PrivateNumeric storage already admits/freezes every carrier array, and terminal synchronization already restores every frame-array lane after freezing. This change enables that maintained route only after all per-field proofs succeed. Original/decoded build-owned image restoration rederives the result proof; corruption tests reject every returned ordinal/root association, frame mapping, offsets, prices and factory binding.

All25 former result-arity declines are now selected. Numeric regions190→215(45 read-only,170 writes); private tuple recipes26 one-array,12 two-array,13 three-array. Reported successful-backedge helpers remain0. Stale/unknown forwarding and unsupported composite declines remain.

## Qualification and allocation evidence

**216 unique scoped maintained Rust tests pass:**205 component/frame/record tests plus8 selected scalar and3 bits. The initial full component run has204 passes and one corruption-fixture failure because it referenced the old Frame/ordinal and Frame/array JSON fields. The repaired test now checks arrays/index/0 and arrays/index/1 for **every** returned field and adds the three-array program; its targeted rerun passes. Thus205 distinct tests have passing receipts; this is not claimed as one uninterrupted all-green full-cohort run. The six new focused tests are included in205. Other expected values/allowances remain unchanged; the existing duplicate-root fixture's source/values remain while its selection expectation becomes eligible.

Coverage: U16/U32/U8 owners and scalar fields, pinned Huffman branch updates, stale later result rejection, actual shared backing among writable roots, current arguments, assertion-before-division, short later array after an earlier write, prior publication/aliases unchanged, cleanup/retry, all six library dimensions plus total work denial, original/decoded artifacts and image reconstruction. Generic64-step visits2930→12, requests4862→2253,bytes542347→249803; native1/64 steps have the same requests/bytes/visits. The one-step request count increases2216→2253 because preparation has a fixed cost. All rows remain; no small-loop allocation gain is claimed.

Real-buffer internal consumption verifies exactly one initial payload each: real-minus-small6524612 bytes=2×(262144−288)+4×(1500513−288). Positive-step real requests409,bytes6592441,visits81,unchanged1→8 steps. Public canonical tuple materialization is separately retained with two payloads each, even at zero steps; its real-minus-small13049224 bytes is not attributed to private result transport. Positive-step public tuple requests2261,bytes13293600,visits37,unchanged1→8 steps. The initial public-tuple expectation of only one payload failed; the revised test keeps that case and adds the internal caller to isolate materialization. These System/probe diagnostics are separate from protected primary.

All12 protected scalar controls pass unchanged≤1.05(max1.028675864861620), with exact requests/bytes and zero0-versus64-step allocation slope. No follow-up rerun. Inherited observations from round10(decoded-U64 ratio1.0526036) and round17(initial zero-step1.052454987; not reproduced in balanced follow-up) remain recorded.

All12 architecture/catalog/inventory/unsafe/warnings/format checks pass(198.993s). Warnings remain errors and the unsafe allowlist is unchanged. Nightly2026-03-03,aarch64mac,opt3,jobs1,locked/offline,incremental0; same selected five math families. Frozen release build444.898s; test harness opt1 with production dependencies opt3. Actual commands/configurations/hashes and all outcomes are in [qualification manifest](qualification/manifest.json). Main executed orchestration:

```text
cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review
python3 /private/tmp/mech-multi-results-round21-20261029/components-final/qualify.py
python3 /private/tmp/mech-multi-results-round21-20261029/primary/run.py
python3 /private/tmp/mech-multi-results-round21-20261029/primary/scalar-controls.py
python3 /private/tmp/mech-multi-results-round21-20261029/full-components.py
python3 /private/tmp/mech-multi-results-round21-20261029/corruption-corrected/qualify.py
python3 /private/tmp/mech-multi-results-round21-20261029/primary/qualify-selected.py
python3 /private/tmp/mech-final-static-20261028.py
```

Initial generic narrow-division setup lacked an existing native witness; the final witness uses established wide arithmetic and narrowing. An initial assertion read the helper count from proof metadata rather than compiled metadata(Null versus0); the correct compiled inventory still requires0. These and the corruption/public-constructor fixture corrections are test setup failures, not production fixes. Raw failed receipts remain. No current native compilation/component/scalar/architecture/primary failure remains. The inherited resident public nested-record failure from [round20](../compression-round-20/CLOSURE.md) remains unchanged and is not reopened. Primary has no resource denial.

Skipped: larger/full-input timing; unchanged Engine227(last run round20),Core21(last round17),non-native-artifact Engine compile(last round20),a new linked runtime-only application,all-certified scalar primitives requiring omitted logic_not/logic_xor,and4 GPU cfg(test) private-fuel units. Previous assertion/corpus/CI/parser work is not reopened.

## Code and profile

Object10844664→12924928 bytes,text10843932→12924196,stack86208→101152. Text SHA b9a9e2e8c013e7792480c5bb5f8e5970b2ef88e3ebc39483db177b7056648e61. The25 newly emitted loops increase code size; no instruction-cache benefit or newly linked application is claimed. Metadata and section hashes are archived.

Fresh profile1307 disjoint exclusive ancestry weights:358 frame actions,301 ordinary visits,182 other helpers,283 selected library,183 remaining. Frame-input admission114. Ordinary visits still occupy about23% of weights. Sample shares are not guaranteed removable elapsed time. The next authorized step is ordinary scalar-path grouping using the existing acyclic frontier proof and admission machinery, with checked arithmetic/source errors, funded entry or ordered fallback, helpers/loops/activation/ownership/publication as boundaries, and no additional executor.

The parent430efcbaa1 received a clean automated review(comment6500104696). This packet gets a fresh exact @codex review request on publication. CI is cancelled by user direction, not green; PR #973 remains draft. The1× goal remains active at73.331359649×.
