# Unique first-admission root reuse — round 48

Retain the measured resource improvement. **No elapsed speedup is established or credited toward parity.** The first timing cohort is −0.203624%; the independent longer confirmation is +0.075609%. Four of eight prepared pairs improve. All observations are preserved. The result remains approximately **14.5× the pinned reference** on complete eight-byte level-12 compression.

| Cohort | Before | After | Shared reference | Primary calls | Improving pairs / blocks |
| --- | ---: | ---: | ---: | ---: | ---: |
| Initial | 4.2052915 ms / 14.551182× | 4.1967285 ms / 14.521552× | 0.289 ms | 3,968 | 2/4; 46/64 |
| Confirmation | 4.1879795 ms / 14.391682× | 4.191146 ms / 14.402564× | 0.291 ms | 7,936 | 2/4; 56/128 |

These are separate cohorts, each with its own 20,000-call reference. The changed reference median affects the multiples; the lower confirmation multiple is not an additional optimization. Eight newly prepared pairs execute 11,904 primary calls, plus 32 warmup/profile calls. Blocks within a pair are descriptive, not independent experimental units. Do not pool selected favorable blocks, multiply earlier gains, or treat the initial −0.20% as confirmed.

## Exact revisions

- Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
- Previous published retained head: `b71c596bdd2f59fbb2893aa8847769f767660677`.
- Tests committed before implementation: `8c787809444103523751bd477aafbe8c7917eb42`; exact before witness build `acf11f0c7ed6d8949c30edf46c6577df65b452d7`.
- Ownership implementation: `d00a8cc0e2b1c540cdf945c937924dbc00b792d2`.
- Exact optimized after library source: `f2be19f4cf04663fd59d1d464894c04cf3523d4c`; CI-only scheduling commit `f2be19f4cf04663fd59d1d464894c04cf3523d4c` is included. The executable production change is Core first admission.
- Final test-only join fixture correction: `b666b3967ab91c9b890417d27c432fced480b743`. It changes no production library body, benchmark, timer, golden or allowance; the already measured coherent binaries remain authoritative.
- Diagnostic census is separately archived in round 47; no census counters/getters remain in production.
- IR/work/ABI/bytecode: **131/95/37/14**.

## Mechanism and ownership boundary

At first import, an already constructed Value previously acquired a registration and a second FrozenSnapshotStorage wrapper, then discarded its original wrapper. An exclusive, unadmitted root can instead receive the ownership after the existing fallible preparation. `Arc::get_mut` proves exclusivity; absent ownership prevents overwriting an existing admission. Shared, admitted, foreign and weakly held wrappers keep the replacement path. No public API, executor, metadata format or layout changes.

A unique wrapper does not imply private data. Both paths retain the existing shared-data registry gate and concurrent deduplication recheck. Byte/node validation and the original conservative admission envelope remain. Node retention and reservation splitting succeed before ownership/link publication. Shape/schema owners, frozen data identity, payload tickets and independently owned imports retain their existing semantics. This uses the same safe exclusive-root attachment pattern as PrivateUnsignedMatrix::finish.

The real census measured 1,010 wrapper paths, including 510 eligible roots, per steady complete call. The allocator verifies **exactly 510 fewer 112-byte requests**: 13,994 → 13,484 allocations; 98,586,843 → 98,529,723 requested bytes. The other 500 wrapper requests remain. All 50 large requests / 97,129,588 requested bytes are unchanged. This is a small-wrapper improvement; requested capacity is not a count of bytes copied.

Fresh paths intentionally have one fewer actual allocation failure point. Registration rejection remains; shared-source wrapper rejection remains. No dummy allocation/check preserves the removed point. The focused original/decoded join fixture now proves success with that point armed, rejection at the next actual join registration with the exact original origin, unchanged prior output, zero leaked admission and recovery. An initial fixture edit incorrectly expected constant setup to be the next registration; the observed join origin is now checked explicitly.

## Build and measurement coherence

Both selected-operation release graphs were freshly built with the same manifest, features, opt3, pinned nightly, warnings-as-errors, offline/locked dependencies and single Cargo job. Core-dependent transitive artifacts were copied into separate immutable directories before the next build. GPU uses the same retained direct-host rustc flags on both sides. Every primary/counter worker uses its respective exact coherent libraries and same-build Object/image. Core integration tests pin the exact after Core hash.

The common dependency audit includes **173 configurations / 313 artifacts**, keyed by package identity/target/features/profile rather than an ambiguous crate name. Every common artifact is Fresh in both Cargo logs, predates baseline linking, and hashes identically. Initial supplemental common hashes were captured while only Mech crates rebuilt. Source/MBC, allocator, Object/images, worker sources and dependencies are pinned in receipts. Cargo builds took 1,042.232 s before and 1,047.157 s after; these are build receipts, not an isolated compile-speed claim. Some correctness checks ran during the latter build, but all builds/tests stopped before performance measurement.

Run order is serialized: primary timing; CPU; allocation; Mech sampling; reference sampling; independent timing confirmation; then qualification. Each measurement cohort is committed immediately. No full-size input or long compression job ran.

## Correctness and review receipts

**1,003 distinct Rust tests pass** in the exact selected configurations: Core 359 unit + 48 memory + 1 buffer capacity + 3 canonical snapshot + 2 serde; GPU 316 runtime + 31 work/value; components 234 + 8 connected + 1 path mechanism. The six new ownership tests are included in Core's 359, not added twice. They cover exclusive and shared roots/data, weak identities, metadata identity, same-account reuse, foreign imports, actual allocation failures, concurrent deduplication, exact/one-short byte/node bounds, overflow, unchanged claims, cleanup and retry. Three mechanism witnesses fail against the pre-optimization implementation; three controls already pass there.

All 12 linked original/decoded executions check current arguments, lexical assertion/load/arithmetic precedence, prior publication, aliases, cleanup and recovery. Twelve scalar controls retain the original ≤1.05 latency gate and exact pure-scalar allocation equality; eight frame-allocation cases preserve zero loop slope and no request/byte/work increase. Component goldens and allowances are unchanged. Existing CI selection lists and executes all 18 tests. New first-admission scheduling separately guards exactly six listed/executed tests. All 132 workflow/inventory Python tests and ten architecture/catalog/warning/format checks pass.

The sole inherited exclusion remains `complete_unchanged_knucleotide_body_uses_the_owned_emitted_map`: the reduced catalog omits its stats import. No all-features local execution or hosted-CI pass is claimed. PR #973 remains draft pending green CI and clean review; publishing CI is cancelled at user direction. Prior run 41168494224 is confirmed completed/cancelled.

## Failure disposition

- Environmental checkout/cache loss was repaired and archived in round 46. No source work was discarded. It forced a broad rebuild.
- Round 47's archive filename error was corrected by resuming the successful build; its rebuilt dependency hash differences are explicitly excluded from timing. Fresh matched round-48 builds resolve comparison coherence.
- The copied GPU test adapter referenced a nonexistent Compute directory. Its E0432 cascade is preserved and the adapter now names the matching frozen artifact. Production builds and every measured worker already used correct paths.
- The old join fault-injection fixture passes on the frozen parent and fails after the deliberately removed fresh-wrapper allocation. The replacement test verifies the changed real failure sequence, exact origin, publication and recovery. Original failures and corrected passing runs are archived separately. A failed text-edit attempt caused one redundant focused run of the unchanged fixture; it is preserved, not counted as new coverage.
- No unresolved introduced compiler/runtime failure remains. The inherited reduced-catalog exclusion and cancelled hosted coverage remain explicit.

## Pins and reproduction

Host: Apple M1 Mac mini, 8 GiB; macOS 15.6.1 (24G90), aarch64; Rust nightly-2026-03-03 (ec818fda361ca216eb186f5cf45131bd9c776bb4, LLVM 22.1.0). Reference binary SHA-256 `3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`, Roc compiler `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`, package `671077457ff7ef3a3d44caf715a3fc595addf379`. Pinned allocator object SHA-256 `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.

Source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. The algorithm, tables, depth 300, nice length 258, near-EOF limits, actual tree/cache capacities, source allocation sites, pinned mimalloc 2.1.7, output capacity 13 with clear/reuse and evaluate_into boundary are unchanged. Every observed call checks raw bytes `[75,76,74,78,73,77,75,207,0,0]`, independent decompression, aliases/content, work **60,260,219,686**, comparison work **47,612** in separate matching witnesses, and final zero cleanup.

The generated Object remains `3f0c0e1954afe1083254d928a8ff66977ee9203e64bfd281326c75f9ff5f09e9`, text 13,885,696 bytes, machine entry stack 108,400 bytes. The actual unchanged Object/image was archived once in round 46. Round 48 adds real host admission disassembly. The host admission function grows 1,119 → 1,168 static instruction sites; conditional physical allocation removal does not imply a smaller function.

Each stage archive includes exact commands, environment, revisions, stdout/stderr and SHA-256 manifest. `analyze.py` regenerates scorecards from raw receipts; `profile_analysis.py` regenerates sample summaries. Numerical replay does not rebuild binaries or promise byte-identical third-party compiler output. Frozen library pins and complete build commands describe the actual executions. [Numerical profile](PROFILE.md) records limitations and remaining targets.

The next compiler investigation is physical cleanup-domain proof. Its separate audit identifies an outstanding worker-capture/arena-generation proof obligation; no universal no-arena theorem or cleanup optimization is claimed. Shape witnesses, registry lifecycle, shared shadows, arena compaction, shared native code and numeric range facts remain separate candidates. Parity remains unfinished.

Independent replay verifies **74 archives, 555 file occurrences, 452 unique paths and nine paths with preserved multiple versions**, reproducing scorecard JSON/CSV and numerical profiles byte-for-byte. Exact invocation: `python3 reconstruct.py /private/tmp/new-empty-replay-directory`.
