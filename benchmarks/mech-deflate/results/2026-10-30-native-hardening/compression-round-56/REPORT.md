# Private numeric publication: fewer allocations and instructions

The complete eight-byte level-12 computation is **13.189584× the fresh reference**, versus **13.244755×** for the retained parent in the same comparison: **0.416547% less elapsed time**. All 3,968 primary observations are retained. Three of four independently prepared pairs and 43/64 alternating blocks improve; pair 2 is 0.191492% slower. The small gain is retained.

| Complete-call metric | Pinned reference | Parent | Candidate | Candidate change |
| --- | ---: | ---: | ---: | ---: |
| Primary median | 0.290000 ms | 3.840979 ms | 3.8249795 ms | −0.416547% |
| Multiple of that reference | 1× | 13.244755× | 13.189584× | |
| Instructions, separate 32-call median | 1,364,372 | 43,117,063 | 42,996,202.5 | −0.280308% |
| Cycles, separate 32-call median | 930,675 | 12,139,909 | 12,075,044.5 | −0.534308% |
| Requests, separate heap cohort | 95 | 8,941 | 8,773 | −1.878984% |
| Requested bytes | 9,021,923 | 98,254,595 | 98,243,843 | −10,752 B |
| Incremental peak live allocator bytes | 9,257,328 | 20,484,504 | 20,482,072 | −2,432 B |
| Total peak live allocator bytes | 9,257,360 | 163,467,016 | 163,464,584 | −2,432 B |
| Post-call RSS | 14,761,984 | 319,651,840 | 319,586,304 | −65,536 B |
| Post-call physical footprint | 14,418,880 | 228,804,288 | 228,755,200 | −49,088 B |

The candidate executes about **31.51×** the reference's instructions and makes **92.35×** its requests. The remaining gap is substantial. Heap counters include retained catalog/image/runtime storage; post-call RSS is not peak RSS. Capacity requests are not initialized bytes copied. The eight-byte input does not establish larger-input scaling or streaming behavior. Reallocations remain 540, relocating reallocations 458, and their tracked copy upper bound 13,905 B; that bound excludes ordinary copies. All CPU cohorts have zero page-ins. Diagnostic elapsed observations and all sample windows are excluded from primary timing.

## Exact change and revisions

PR base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`. Previous published head: `7f7aad194537cdddb24d3a8b9f32c9a74ae0720d`. Production change: `b717a930396d3f694865f79149216581bedde2b5`. Candidate build source: `198b7818a0e0d0aac518678445a4bd31c08f3267`; retained parent build source: `13f319e7b0058c66f892e2566e0d56558831ab12`. Later changes only strengthen tests, update their CI count, or record evidence. The measured production files remain identical.

`PrivateUnsignedMatrix::finish` promised allocation-free publication but initialized a fresh platform Mutex in the payload's import registry. The full parent trace contains **168 allocations of 64 B** with this ancestry. Publication now initializes that unpublished registry through the existing exclusive gate helper, before exposing the weak wrapper owner. Destruction/consuming transfer also uses exclusive registry access when `Arc::get_mut` proves there are no strong aliases or weak data identities. Shared and weak cases retain the original locked path. This prevents merely deferring the lock allocation to retirement.

No public API, owner size, lock type, unsafe access, memory/work prices, schema, image or artifact revision changed. Every other allocation size/kind bin agrees exactly; the complete candidate census reconciles **8,773 events**, **4,799 stack/size/kind combinations** and **1,280 PCs** with the separate heap counters. Its only histogram delta is **−168 allocations of 64 B**.

The test was committed first in `dafe97c6fb49fdcb715afb4f9b26cffe5f64a50a`, corrected to use public admission in `82d8d31418c126986ccf2fbdd808b3070d52c310`, and strengthened in `a4a3a6c32daf7bcced6e7132dbf8c202c95b7deb`. The final source fails on the retained parent with publication requests **1 versus required 0**, then passes on the candidate. It independently observes publication and complete destruction, all four unsigned kinds, fixed/dynamic shapes, capacities 13/262144/1500513/305257, backing identity, retained aliases, exact injected `AllocationFailed`, same-account retry, and final zero bytes/nodes. CI's constructor inventory is separately updated **3→4** in `c4fdacb4c76f6ff47264006934553c7987ada2bb`.

## Fidelity and profiles

Source, bytecode, algorithm, tables, search limits, near-EOF behavior, source allocation sites, real buffer capacities, allocator and `evaluate_into` timing boundaries are unchanged. Output capacity stays 13 and is cleared/reused. Every complete invocation matches raw bytes `[75,76,74,78,73,77,75,207,0,0]`, independently inflates to `abcdefgh`, preserves input aliases, and reports logical work **60,260,219,686**. The separate comparison witness remains **47,612**. Final managed bytes/nodes are zero.

Object, canonical image and metadata are byte-identical to the parent; hashes are in `static-equality.json`. There is one fresh 20,000-call reference shared by the before/after primary cohort, four prepared pairs, and 16 alternating blocks ×31 observations per variant per pair. Comparing different rounds' ratios without their reference clocks is inappropriate.

Fresh serial samples retain weights **1,316/1,314/1,515** for parent/candidate/reference. Function-frame handling remains **334/1,314 (25.42%)** of candidate sample ancestry; cleanup is 53 (4.03%), projection accounting 43 (3.27%), and thread-local lookup 42 (3.20%). Ancestry groups overlap and include harness activity. The reference's dominant symbol is the opaque generated `roc__p5794f552e2e0fcba87a322629fdca7c0.fast` (1,428/1,515); no unsupported source name is assigned to it.

The path investigation maps all **35 capacity-sized U64 requests** (85,471,960 requested B) to actual canonical append/clear/creation sites. These copy only initialized elements. The 32 per-position appends currently receive no ownership permission because the conservative old-owner graph includes contradictory Boolean branches; safe improvement requires a current-visit proof plus private-frame alias inventory. Those proofs are unchanged here. Ordinary numeric replacement still accounts for **2,242 requests**, and generic selected-call binding/planning for **1,243** in mutually exclusive ancestry buckets. Together with frame handling, these are measured next targets; each further change needs its own witness and before/after run.

## Validation and commands

**1,025 distinct Rust tests pass:** Core 364, Core integrations 54, constructor allocation 4, native runtime 326, work/memory/value 32, preparation 2, components 234, connected components 8 and path mechanism 1. Overlapping qualification additionally executes 12 linked original/decoded success/failure/retry cases, 12 scalar controls, 8 frame-allocation cases, 31 CI-selected tests, and lists/executes 9 admission +5 numeric matrix +4 constructor cases. Scalar controls keep the original 1.05 limit (worst ratio **1.009121**), with unchanged pure-scalar requests/bytes and zero loop allocation slope. All 132 Python workflow/inventory tests and 10 final architecture/catalog/unsafe-boundary/warnings/format checks pass. Goldens and allowances are unchanged.

Representative commands, all executed serially from the authoritative worktree:

```text
python3 /private/tmp/mech-buffer-liveness-round56-20261031/build-matched.py after
python3 /private/tmp/mech-buffer-liveness-round56-20261031/complete-round.py
python3 /private/tmp/mech-buffer-liveness-round56-20261031/allocation-regression.py before-recovery
python3 /private/tmp/mech-buffer-liveness-round56-20261031/allocation-regression.py after-recovery
python3 /private/tmp/mech-buffer-liveness-round56-20261031/final-constructor-ci.py
python3 /private/tmp/mech-buffer-liveness-round56-20261031/final-architecture.py
```

The release command is `cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review --features mech-runtime/compiler --verbose`, pinned nightly-2026-03-03, opt3, warnings-as-errors, one job, incremental disabled. The final host uses the retained exact direct-rustc native feature profile against the newly frozen coherent graph. All **173** common dependency variants have identical configurations and artifact hashes. Core/GPU unit roots use the retained opt1/WAE configuration. Every exact argument list, environment, test count, source revision and outcome is in the receipt archives. See `source-provenance.json` for full source/compiler/allocator/reference pins.

## Failures and limits

- Expected parent regression: final test source yields 3 passed/1 failed because publication allocates a lock. Candidate: 4/4, including same-account recovery.
- Introduced diagnostic mistakes, corrected and preserved: an external inspector attempted a crate-private facts method; naming its driver `inspect.py` shadowed Python's standard library and re-entered the failed helper. Only the verified failed task helpers were stopped; the driver was renamed and public facts used. No compiler visibility changed.
- Introduced test mistake, corrected and preserved: the first external allocator test called a crate-private budget method. It now uses ordinary public admission and checks the exact injected error and recovery.
- Introduced orchestration mistakes, corrected and preserved: direct formatting initially used edition2021 against existing edition2024 let chains; the correct full check passes. A manual test commit collided with the archiver's live index lock; the helper completed, then the commit was retried under its shared flock. No lock file was removed or live build restarted.
- Introduced packet completeness error, corrected and preserved: the first numerical replay lacked the retained parent's linked metadata at its replay paths. Those exact inputs were archived; a separate fresh replay reproduces all four analysis outputs byte-for-byte. No timings were rerun.
- Environmental diagnostic denial: unprivileged `ps` was denied; the authorized process check was rerun with the required access.
- Inherited skipped coverage: `complete_unchanged_knucleotide_body_uses_the_owned_emitted_map` needs a stats import absent from this reduced selected catalog. No local all-features/no_std/Miri or hosted-CI pass is claimed. Hosted CI is cancelled by user direction and the PR remains draft pending green CI and clean review.
- Pair 2's adverse timing is retained. The earlier round's adverse cycle cohort also remains in its own packet. No replacement favorable cohort is substituted.

No unresolved compiler, production-test or architecture failure was introduced. Full-input timing and parity are not claimed.

Completed-build cleanup freed **436,650,888 B** from eight task executables. Current/previous benchmark workers, the reference, compiler graphs, Objects/images, unit roots, sources and all receipts remain.

## Numerical replay

`reconstruct.py` verifies every archived file/version before chronological overlay, then independently regenerates the scorecard JSON/CSV, profile totals and allocation attribution. It does not rebuild binaries or rerun timings:

```text
python3 benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-56/reconstruct.py /private/tmp/mech-round56-public-replay-final
```

The exact executed replay command, archive counts and byte-identical outputs are saved in `reconstruction.json`.
