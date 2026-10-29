# Opaque compound siblings: completed review packet

Reviewed base **d48231ef0413b05e455576217d10898b3544e7fa**, clean review [6493894129](https://github.com/mech-lang/mech/pull/973#issuecomment-6493894129). Production compiler **1d2ef606f0cb576eee343b4806fd1c75e69a7b7a**. Baseline986601834cd582db8410d38699eab0f533937df9; test-boundary correction8b04906ed35a52e1d7cc5766392dde00f2fb361b; timing/profile **421ba09eb71d6a521ac7858fc3bffbf69b6bba43**. The publication commit adds these receipts; its exact head is in PR #973 and the task response. PR parent eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b. Branch/worktree codex/level12-compression-consolidated, /private/tmp/mech-deflate-linear-stack; the separate user's checkout is untouched.

## Result and scope

**Roc=1×:114.941552×→111.149597×;3.299029% less elapsed;1.034116× speedup.** Complete eight-byte medians33.103167ms→32.011084ms, same fresh reference0.288000ms from20000 observations. Three primary observations per Mech control; warm-ups and the separate125-repetition diagnostic profile are excluded. Previous round17 after was113.747852× under its own reference. This is a modest observed gain, not the larger improvement requested, and these samples do not establish a confidence interval or larger-input throughput. [Committed timing packet](TIMING.md).

Exact ten-byte raw output, independent inflation, capacity13 and final zero bytes/nodes pass throughout. Output backing still changes; copies stay timed. Same pinned mimalloc2.1.7 object and unchanged evaluate_into-only harness, with no competing compiler/test/benchmark. Largest complete input here is8 bytes. Real-sized reservation witnesses are not larger compression runs.

Source revision671077457ff7ef3a3d44caf715a3fc595addf379; reference compiler d99c7b838fb05bb1047eb0c18d4d06d0f86f345d; executable SHA2563941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324. Caller source SHA256331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9; bytecode SHA256a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7. All25 components, algorithm/tables,262144-entry U16 tree,1500513-word U32 cache,depth300/nice258,near-EOF rules,6 Huffman initializers, source allocation sites and timing boundaries remain unchanged. Fresh source compilation and decoded reencoding reproduce the pinned bytecode exactly. Mech performs compression itself.

## Design and blocking operation

The actual optimizer/controller returns records containing nodes beside costs/saved/tallied records. Exact source requirements are deflate-optimize.mec:6,26; deflate-plan-block.mec:48; deflate-compress-controller.mec:17 onward. Previously, frame eligibility rejected any tuple/record-valued field. Canonical snapshot construction clones child data and discards spare buffer capacity at this boundary.

The production change is11 inserted/4 removed lines in two files. Engine eligibility now admits opaque canonical tuple/record siblings in existing managed-owner lanes. It adds no runtime representation, owner authority, executor, payload flattening or global clone rule. Complete-component pack/copy/project use analysis, exact field schema identity, Turn-only geometry, admitted current owners, current shapes/selectors, generic fallback admission, liveness/retirement, finite memory/output/work bounds and atomic publication/retry remain authoritative. A child consumed by another pack stays canonical. Whole canonical escapes and true nested private-frame transport remain outside this slice.

IR120 fingerprints changed eligibility; image loading rederives it. Work policy93 prices, ABI35 and bytecode14 remain. Full-program work changes303735923752→308968619891(delta5232696139) because more calls take existing private-frame admission; this is not an equal-debit claim. No allowance increases or resource bypasses were used.

The retained round16 trace contains53 record-read calls at43 distinct sites. Rechecking exact primitive/origin identities against the current plan shows7 of those calls/sites now eligible for private frames;46 retained calls remain canonical. This is current selection mapped to retained dynamic evidence, not a fresh call-frequency measurement. It does not prove every complete-compressor reservation survives all remaining boundaries.

## Validation

**425 unique maintained Rust passes**, without changing existing expected results or allowances:

| Scope | Executed | Result |
| --- | ---: | --- |
| Actual Engine budget/worker/projection/frame/text/collection test root |227|Pass|
| Existing component cohort plus record-buffer boundary qualification |187|Pass|
| Selected native scalar correctness |8|Pass|
| Integer bits |3|Pass|

The focused3-test run is included in187. The two reservation tests cover records and tuples, both original/decoded artifacts, image rederivation, U64/U8 and capacity16 plus real305257/6633832. All16 visits retain the exact requested capacity and clean up. The third test runs two successes and three expected failures per form with fresh opaque scalar bits, assertion before division, checked overflow, unchanged prior output storage/aliases, cleanup and retry. The unchanged standalone driver independently records8 before/after rows: U64 capacities16/305257 previously became8; U8 capacities16/6633832 became64. All now retain the requested capacity with two private fields and zero final retained bytes/nodes. These are capacity/owner observations, not allocation-request or peak-memory measurements; no new allocation probe is included in primary.

All12 fresh protected scalar pairs meet the unchanged≤1.05 timing guard(max1.006731406), with exact before/after allocation requests/bytes and zero0-versus64-iteration allocation slope. No follow-up rerun was needed. The earlier observations remain recorded: round10 decoded-U641.0526036>1.05, and round17 initial original zero-step loop1.052454987, whose fixed balanced follow-up did not reproduce>5%. Current passing controls do not erase those inherited observations or declare scalar retirement complete.

All12 architecture/catalog/inventory/unsafe/warning/format checks pass on the corrected code. Warnings remain errors; the unsafe allowlist is unchanged. Non-native-artifact Engine metadata compilation passes(compile evidence, not another runtime test count). Current source compilation/reencoding, original/decoded execution and build-image checks pass; no new linked executable ran.

Nightly2026-03-03,aarch64mac,locked/offline selected-operation release,opt3,jobs1,incremental0,existing five arithmetic families. Clean production release450.902s. Semantic Engine unit root usesopt0 and compiles44.181s; production/primary/scalar/component dependencies stayopt3. Engine run7.179s process/5.85s displayed;187 components72.138s process/72.12s displayed; non-native metadata compile13.109s. Test times are not benchmark comparisons.

Commands, complete features, dependency/source/binary hashes and nonzero counts accompany the receipts:

```text
cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review
python3 /private/tmp/mech-record-boundaries-round18-20261029/components/qualify.py
python3 /private/tmp/mech-record-boundaries-round18-20261029/primary/run.py
python3 /private/tmp/mech-record-boundaries-round18-20261029/engine-units/build.py
python3 /private/tmp/mech-record-boundaries-round18-20261029/full-components.py
python3 /private/tmp/mech-record-boundaries-round18-20261029/primary/qualify-selected.py
python3 /private/tmp/mech-record-boundaries-round18-20261029/primary/scalar-controls.py
python3 /private/tmp/mech-record-boundaries-round18-20261029/object-audit/build.py
python3 /private/tmp/mech-record-boundaries-round18-20261029/object-audit/sections.py
python3 /private/tmp/mech-record-boundaries-round18-20261029/after-witness/build-nested.py
python3 /private/tmp/mech-record-boundaries-round18-20261029/after-witness/build-audit.py
python3 /private/tmp/mech-record-boundaries-round18-20261029/engine-units/nonnative-check.py
python3 /private/tmp/mech-final-static-20261028.py
```

## Binary/profile evidence and next step

Object11542304 bytes,SHA256565e297462e4c333fcd419ea077e78e14869b90b42ea93b9c7ecc09556bcb007; full metadata1704995 bytes,SHA2562aaf255774b46fe2629a99b17e03c0bdc9af93dd25f4112b0abfce1e8c84bf15. Full file identity changes; the11541572-byte __TEXT,__text section is byte-identical(SHA256e378341416d531cda12cb54fd5e7dd5201bf50c1f1bc2332eff7d17a26801013).190 numeric regions remain45 read-only/145 writes,26 private mixed results, zero reported successful-backedge helpers. Existing numeric guards continue to decline unsupported shapes. Raw metadata and section comparisons are retained. This slice changes selected runtime frame handling; there is no generated numeric-code growth. Archived pinned ARM64 loads/stores/allocation/backedge excerpts were reexamined; no named source-to-hashed-procedure or new per-function reference timing is claimed.

The fresh1356-sample disjoint partition is343 other-helper,318 frame-action,261 ordinary-visit,270 selected-library and164 remaining weights. Frame-input admission136. These are sampled ancestry weights, not exact removable elapsed time. The dominant call/frame/control costs remain. After review, the next bounded investigation should attribute those costs to an actual missed native recurrence; retained scalar-only record results and parent/child boundaries are concrete candidates. Require a source requirement, minimal witness and acceptance test before expanding eligibility. No further semantic change is included here.

## Failure disposition and omissions

No current compiler or Rust test failure remains. Introduced setup failures are closed and retained: default-limit audit wrapper(required120162192 definition bytes versus67108864) corrected to existing NoLimits; oversized resident oracle(required19575552 versus inherited16777216) corrected only to capacity16 while separate real reservation tests stay; initial expected parent regressions3/3 fail at capacity8 versus16; historical Rust evidence/new image-test placement outside the unsafe boundary corrected by compressed archival source and the existing approved helper location. The allowlist and production source are not widened. Initial static/wrapper failure receipts remain. Injected-unwind/default-limit diagnostics inside passing existing tests are expected. Primary has no denial; no environmental blocker remains. The modest gain and inherited scalar observations above remain limitations.

Skipped: larger/full-input compression/timing; Core21 contracts unchanged since their prior round17 pass; all-certified scalar primitives requiring omitted logic_not/logic_xor; new generated runtime-only linked application;4 GPU cfg(test) private-fuel units. Earlier assertion/corpus/CI/parser packages are not reopened. Existing finite admission/escape/alias/publication/recovery tests ran without relaxed allowances.

Draft #973 stays draft. New CI is cancelled by user direction, not green; exact publication head/run and fresh @codex review request are recorded in PR metadata/task response. The1× objective remains active at111.149597× on the measured input.
