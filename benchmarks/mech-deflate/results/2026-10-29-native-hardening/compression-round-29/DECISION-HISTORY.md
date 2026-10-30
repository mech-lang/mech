This earlier disposition is superseded by CLOSURE.md: the measured accounting change is now retained after the user clarified that small verified gains should accumulate. All measurements below remain original.

# Copy diagnosis and accounting experiment: no retained speedup

The retained compiler remains the reviewed scalar-aggregate numeric-loop implementation. This round adds two accounting regression tests and evidence; it does not retain the attempted runtime optimization. The qualified round27 result remains **27.748924× →20.259555× Roc**, a26.989763% elapsed reduction. The1× objective is not achieved. No larger-input throughput claim is made.

PR parent **eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b**. Experiment base **d75177747b6870d32265b5410c000b789552060b** (clean review6512686622); parent witnesses/design **2cd0b8edae555c24509f6531f68ee7344f3f9e2b**; candidate **e3640245eff14060c840a64f196429c09498ce8d**; immediately committed timing/profile **c7f6d414531ad6518ac1c043366601b33133bb3d**; immediately committed scalar receipts **97bbbaf23006264f457731987c419fa1f6c2992f**; production restoration **b2a2d2d392450adb7543615eb23d00e6180a7cf7**; unwind-only test guard **cf8779edda0bad423e8abfdf4f9345e69754125e**. IR/work/ABI/bytecode129/93/36/14 remain unchanged.

Every one of2342 tracked compiler/runtime/host/machine/harness files equals the reviewed base after removing only the two named new test functions and their attributes. All other bytes must match; the mechanical proof is [retained](qualification/restored-source-identity.json). The tests cover native-ceiling/overflow error precedence versus a foreign active account and narrower library cap, plus nested-call unwind restoration without refunding spent work. They pass on the original implementation before production edits. The panic-recovery test follows the existing unwind-only convention. No new payload owner, pointer loan, execution route or work policy survives.

## Actual copying evidence

A complete unchanged8-byte compression produces200 ordinary replacement calls:180 change a value,36 transfer through Core, and144 copy **2,157,068 bytes**. Four copies of the real262144-entry U16 tree account for2,097,152 bytes.88 calls have no compiler consumption candidate. All144 copied sources retain direct frozen arena aliases; none retains a private-frame alias. [Summary](qualification/summary.json).

The observer reads the original payload pointer, records the actual resulting pointer and exact changed flag, then inventories aliases after original candidate retirement. It never clones a Value or changes the helper/ownership decision. Both ordinary and observed visits agree on work45060848918, exact bytes, capacity13, aliases and zero final bytes/nodes. Source wrappers, all200 calls, pointers, read roles and failed diagnostic builds are archived.

This does not prove that all observed aliases are their only transfer restriction or that any live alias may be removed. Tree alias882 has a CollectionSource(26) read role; other sources have genuinely later reads. No field-sensitive retirement change is justified by this evidence. The current retained sample has only15/1283 exclusive memmove weights; the older much larger copy attribution is stale. Blind retirement or capture changes are outside this result.

## Measured accounting candidate

The candidate grouped the existing active-account, observer and relative-cap cells into one thread-local context while keeping independent RefCell/Cell boundaries and the original Rc account. Hot charge, preview, observation and call entry resolve it once. A fused native compute debit checks caller ceiling/overflow first, then foreign account identity, then all original local/relative/reserved/worker caps and both-dimension atomic commit. Work prices, limits, failure origins and unwind/retry behavior remain unchanged.

Actual optimized binary inspection confirms that separate charge/relative-limit TLS resolution becomes one context resolution; the debit helper receives explicit current limits. Mechanism-only C and Rust controls each emit two TLS lookups for separate fields and one for a shared context. These controls were compiled for assembly only: zero executed control programs and zero elapsed comparison. This supplies a mechanism, not proof of a large gain.

Protected complete8-byte compression: **19.980302721× →19.651006803× Roc**, **5.874209000→5.777396000ms**, common fresh reference **0.294000000ms /20,000 observations**. Pooled-median reduction **1.648102749%**; median paired ratio **0.990751374**. The four ratios are0.991416872,0.990085876,0.908574904 and0.994984179. Three pairs improve by only0.5–1.0%; this does not meet the requested substantial gain. The candidate is removed. [All timing observations and separate profile](timing/manifest.json).

Four fixed balanced alternating process pairs;12 primary observations/control, all40 observations retained. Separate693-repetition profile excluded. No compiler/test/audit/other benchmark ran concurrently with primary/profile or scalar controls. Same mimalloc2.1.7 and evaluate_into-only harness; compilation, output reserve13, clear/rebind, verification and drop excluded. Every observation checks exact ten raw bytes, independent inflation, aliases, capacity13 and zero cleanup. No source, algorithm, table, depth300/nice258, near-EOF limit, real262144/U16 tree or1500513/U32 cache, source allocation site or timing-boundary change.

Work45060848918, comparison47856, visits14894, helpers9500 and transport4758520 remain exactly unchanged in the candidate census. The route itself is unchanged. No new emitted-program Object size audit was run for this rejected runtime-only change.

The prior and candidate samples contain1283/1291 weights. Recomputed strict accounting attribution is187/163 exclusive weights, including72/38 TLS weights; whole-sample TLS101/48. This corrects the earlier186 tally by one omitted used() leaf (13→14); the original file remains historical. Whole with_call_limits bodies are explicitly excluded from the accounting-overhead claim. Samples are not predicted removable elapsed percentages. [Calculation and source hashes](qualification/accounting-profile-comparison.json).

All12 protected scalar controls pass unchanged≤1.05; max direct ratio **1.006709359**. Allocation requests/bytes agree and scalar-loop allocation slopes remain zero. [Immediately committed scalar packet](scalar/manifest.json). These qualify the removed candidate and are not a retained optimization claim.

## Qualification and dispositions

The candidate passes **785 maintained scoped Rust tests**: Engine228, GPU runtime301, GPU work12, exact transport admission1, component cohort232, selected scalar8 and bits3. Focused6 overlaps the232; parent12+228 overlap these test families. A separate complete-program callback census passes and supplies no performance observation. All expected values and allowances remain unchanged. Original/decoded execution, exact/one-less/overflow funding, worker credit, no-refund unwind, publication/aliases, ownership/capacity, cleanup and retry are exercised by these cohorts. No introduced production failure remains.

All12 architecture/catalog/layout/factory/warnings/quarantine/inventory/unsafe/format checks pass for the candidate; allowlists and warning policy are unchanged. Final restored source passes rustfmt on the two retained test files and git diff --check. Earlier retained architecture qualification still applies to byte-identical production code. The new tests also passed the clean parent; guard-only follow-up does not change their unwind-build body.

Nightly2026-03-03,aarch64-apple-darwin,locked/offline,jobs1,incremental0,-Dwarnings. Production opt3; maintained integration bodies opt1; actual GPU unit root opt0. Default focused build uses the existing aot/canonical-native/canonical-native-program/canonical-native-program-runtime/canonical-native-runtime/jit GPU profile. Actual default Cargo build completes7m53s; narrow test dependencies add only mech-runtime/compiler and complete49.17s. Every shared default/runtime test library hash agrees. Source verification recompiles160857 bytes to the identical4138627-byte artifact and confirms decoded reencoding.

Commands include:

- cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review
- The same command with --features mech-runtime/compiler for maintained runtime tests.
- cargo check --locked --offline --manifest-path src/engine/Cargo.toml --no-default-features --features resident-artifact --target-dir /private/tmp/mech-ownership-round29-20261029/feature-target:pass,41.66s.
- Direct actual GPU unit root with the Cargo-resolved libraries and production features; work selector12, runtime selector301 with the existing absent-stats test excluded, exact transport selector1.
- Maintained component root with absolute module-path adaptation only and the linked module handled separately:232 pass. No test-body, value or allowance changes.
- Protected timing/profile and scalar scripts, immediately archived/committed after each measurement. All exact rustc commands, libraries, feature arrays and outputs are in the [131-file receipt manifest](qualification/manifest.json).

Failure ledger: the original observer build used tuple children as Values and failed E0308; corrected direct-root alias inspection passes. An initial updated census assertion accessed work.compute although work is u64 (E0610); corrected to work and rerun passes. One guessed transport-test filter executed zero tests; its receipt is retained, it earns no coverage, and the exact maintained test subsequently executes1 and passes. Initial disassembler option/symbol attempts were invalid; corrected exact nm symbols and positional --dis-symname produce actual instruction listings. No allowances or expected values changed to resolve a failure. Exploratory missing-path reads do not represent compiler failures.

New linked/AOT qualification is intentionally skipped after rejection/restoration; no changed runtime is shipped. The previous actual linked qualification on identical production code remains in round28, without counting it fresh. The candidate's runtime-only linked project was not built. Larger/full-input timing, unchanged Core reruns, omitted not/xor all-primitives coverage, absent-stats whole-knucleotide and tests outside these cohorts remain skipped. Inherited resident nested-record/alternate geometry and default resident work-policy denials stay inherited; no earlier assertion/parser/corpus/CI package is reopened. CI is cancelled by user direction; the PR stays draft.

The fresh retained binary measures **19.980302721×** against this round's reference, versus its previous qualified20.259555× against an earlier reference. This is remeasurement of the same compiler, not another gain. Restoring byte-identical production needs no additional compression timing.

The next bounded investigation should choose a reached whole recurrence around managed child calls or prepared frame transfer/retirement, using the existing current-argument and funding machinery. Frame actions still account for337/1283 exclusive weights in the retained sample, but that is not a promised saving. Establish one concrete operation and generic correctness witness before changing selection or ownership. No next production optimization has started.

The pinned source's compression modules contain6187 physical lines (4745 excluding blank/comment-only lines), including all levels and embedded tests, excluding decompression and harness. CompressOptimal alone has1342 lines and uses shared modules. [Hash-verified count](qualification/reference-loc.json).
