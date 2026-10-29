# Nested private-frame composition: review checkpoint

## Revisions and measured result

PR parent eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b; prior published/reviewed head96721e45d454f507595c54e445d18266eff05ced. Initial production e721febbcd557040ce8866e2ebbf61f09a9e4b5b; focused qualifications d9db1f2349151cb814336b668be7333cb05c138d; initial timing58fbf1b827f87a4adfe462faf3ac6e9586f7c871; fallback red/controls065d855ce; corrected production **aed697ad21ba1ffcf07da89b90c4c33571cf6ef2**; corrected timing/profile committed immediately **81850eddf69f563f339babb9964aa5fa9b02a7a9**. The publication head contains this packet and a comment-only clarification. IR123/work93/ABI35/bytecode14. Exact publication head accompanies the PR update.

**Roc=1×:111.196680070×→102.707533217×**,31.8022505ms→29.3743545ms against the same fresh0.286000ms reference(20000 observations). **7.634353% less elapsed;1.082654× speedup.** [Final timing](FINAL-TIMING.md) and raw receipts preserve all four alternating-order pairs, twelve primary observations/control, warm-ups, exclusions and separate137-repetition profile. No compiler/test/other benchmark overlapped protected primary. Initial99.868245734× at29.261396ms used a different fresh0.293ms reference and the earlier compiler with the subsequently repaired fallback bug; it remains a separately labeled experiment. Do not splice these normalized series or relabel the old compiler. Largest complete input remains8 bytes.

Pinned source671077457ff7ef3a3d44caf715a3fc595addf379 and reference compiler d99c7b838fb05bb1047eb0c18d4d06d0f86f345d are unchanged. All25 source components, algorithm/tables,262144-entry U16 tree,1500513-word U32 cache,depth300/nice258,near-EOF rules, allocation sites and evaluate_into-only boundary remain unchanged. Mech compresses the input. Fresh final source compilation and decoded reencoding reproduce all4138627 bytecode bytes from160857 source bytes; this check does not itself execute compression. Every primary/profile observation produces the exact10 raw bytes, independent inflation, capacity13 and final0 bytes/nodes. Output backing changes and those copies remain timed. Same pinned mimalloc2.1.7. Work307973385655→306669833472; existing route prices and allowances remain authoritative.

## Design

NativeBufferFrames now represents proved private structural children by bounded spans into the same scalar/managed leaf layout. Complete-component Pack/Copy/Project and escape analysis remains authoritative. A parent copies current child bits/owner references into its own admitted lanes; it does not point at mutable child slots. Projection restores current child lanes. Retirement clears the whole subtree. No intermediate canonical record payload is constructed along selected composition edges; no new ownership authority or executor is introduced.

Immutable prepared composition recipes retain only static source/schema/leaf/footprint relations. Every reached invocation rechecks current typed owners, scalar bits, current shapes and exact selectors, then charges finite admission and the original composite logical footprint. Existing ordinal validation is shared. Image restoration rederives the proof using the approved existing helper. Ambiguous or unsupported geometry and public escapes keep canonical constructor behavior. Numeric loops explicitly retain flat-field restrictions; multiple-array returns are not part of this change.

Current selection covers43 static mapped sites/53 calls in the retained historical trace, versus7/7. These are current graph selections mapped to old frequencies, not a new dynamic frequency measurement. The identical standalone wrapper witness(costs→selected→planned) has real-capacity steady success12926→304 requests and3596871→2485193 requested bytes. Current305257 U64 and6633832 U8 reservations are retained; before the actual U64 capacity was8. All rows remain, including failed visits and warm-ups. Component work increases156814977→205493208/215260920 because actual capacities and current route admission are now charged. Full compression work decreases as stated above. System/probe allocations are separate diagnostics from protected mimalloc timing.

## Qualification

**437 unique scoped maintained Rust passes:**227 actual Engine budget/worker/projection/frame/text/collection,199 maintained component/frame/record tests(including8 new focused tests),8 selected native scalar and3 bits. This is a scoped cohort, not all available root tests. Existing expected values/allowances are unchanged. Original/decoded coverage includes private child composition, scalar bit patterns and changing0/3/1 arrays, current arguments, fresh recursion, child aliases/shared parents, clear/append, exact logical output-byte admission, all six resource dimensions, assertion/arithmetic precedence, overflow, unchanged publication/aliases after failure, cleanup and retry. Real buffer reservations run separately from small resident-oracle witnesses.

All12 protected scalar controls pass the unchanged≤1.05 ratio guard; maximum1.035284426408066. Requests/bytes agree and0-versus64-step allocation slope remains zero. No follow-up rerun. Initial controls also remain. Inherited observations round10 decoded-U64 ratio1.0526036 and round17 initial zero-step1.052454987 remain recorded; round17 balanced follow-up did not reproduce>5%.

All12 architecture/catalog/inventory/unsafe/warnings/format checks pass, including final185.059s receipt. Warning-as-error policy and unsafe boundaries remain. Non-native-artifact Engine metadata compilation passes(compile evidence only). Nightly2026-03-03,aarch64mac,opt3,jobs1,locked/offline,incremental0; existing five selected math families. Corrected frozen release build443.759s. Engine semantic units useopt0; component/performance dependencies remainopt3. Exact commands, features, library/binary/source hashes and outputs are in [qualification manifest](qualification/manifest.json). Main commands:

```text
cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review
python3 /private/tmp/mech-nested-frames-round20-20261029/fixed-qualify-all.py
python3 /private/tmp/mech-final-static-20261028.py
```

Individual exact executed primary/scalar commands are authoritative in archived receipts; script labels above summarize the orchestration. No larger/full-input timing, newly generated runtime-only linked application, unchanged Core21(last run round17), all-certified scalar tests requiring omitted logic_not/logic_xor, or4 GPU cfg(test) private-fuel units ran. Prior assertion/corpus/CI/parser packages are not reopened.

## Failures and disposition

An introduced optional-selection bug rejected private children beside dynamic canonical record siblings with `private composition has unsupported shape relations`. The retained red test reproduces it on the initial compiler; aed697ad adds a static shape compatibility filter and restores canonical constructor fallback. Eight focused final tests pass. Initial timing stays labeled with the initial compiler.

Test setup errors(invariant literal-specialized shape, square versus vector extent, and default resident compute allowance for real reservation) were corrected without changing existing test expectations or increasing allowances. Small resident-oracle capacity16 and separately qualified real native capacities are explicit. Metadata mismatches, omitted copied reference files/driver, a wrong historical Object comparison, and initial profile labels were tooling/setup failures; corrected receipts are retained. Initial results metadata copied an old logical-work explanation; the correction is archived without changing observations.

**Inherited resident failure remains:** the public nested-record escape witness returns `Kernel { node: NodeId(2), error: InvalidInput }` at0/3/1 rows in original/decoded resident execution. Clean source-pinned parent96721e45 and current compiler produce identical six failures and byte-identical artifacts. Native canonical fallback agrees with the independent expected literal. Parent-control source/dependency hashes and both executions are archived. This is not claimed as resident agreement and is not expanded into an unrelated work package. The first replay attempt used overwritten libraries and is explicitly marked invalid as parent evidence; the later clean-source parent control establishes inheritance.

No current native compilation/component/timing/scalar/architecture failure remains. Primary has no resource denial. CI is cancelled by user direction, not green; PR stays draft. Last completed clean automated review names96721e45. Publication requests a fresh exact @codex review.

## Generated code and next steps

Object10844664 bytes and text10843932 bytes are unchanged. Executable text is byte-identical to the parent(SHA25636e3f9f6487dfc38f728dc0a0a8c28b329a7e6ab4b41c5c5db88d424b9e064fb); stack86208 bytes,117 scalar aggregates,190 numeric regions(45 read-only/145 writes),26 private mixed results,zero reported successful-backedge helpers remain. Whole Object identity changes to12153df953c38940adb362e59f6bb52719af8f820a155ad832ab424b330ae153. This is metadata/helper-routing evidence, not a newly linked application or direct body-code improvement.

Fresh final profile partitions1307 disjoint exclusive ancestry weights:392 frame actions,305 ordinary visits,185 other helpers,263 selected library,162 remaining. Frame-input admission171. Shares are not promised removable elapsed time. Reference ARM64 memory/load/store/allocation/backedge excerpts were reexamined; no new named-source-to-hashed-function timing is claimed.

The user has authorized both next upgrades: multiple-array numeric return proofs, then grouped admission for ordinary scalar paths using the existing frontier machinery. Each gets a separate small protected timing/profile and immediate timing commit. The1× goal remains active at102.707533217×; unchanged compressor source remains the workload.
