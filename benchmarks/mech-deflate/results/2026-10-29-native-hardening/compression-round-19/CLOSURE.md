# Private scalar named results: completed review packet

Published base **37407827c92f9fe885b439b164637f0e23aa7daf**. Production compiler **7f96b935684f15d3a09bbd9a4b949b29d6a625e0**. Baseline174a21b47c99f15617b6b5be65b1d10f53cbb2bd; finite-test correction62ee0eb18f309812612916171282edc68ebdce05; timing/profile **543681f1a8a4463bb0e4471f97a170b6e45d13ee**; image test99a8f783122919f6d3c0238d4f190530bbd52a18. The publication commit adds these receipts; its exact head is in PR metadata/task response. Draft #973, parent eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b; branch/worktree codex/level12-compression-consolidated, /private/tmp/mech-deflate-linear-stack. The user's separate checkout is untouched.

## Outcome

**Roc=1×:108.977027×→106.514339×;2.259823% less elapsed;1.023121× speedup.** Complete eight-byte pooled medians31.821292ms→31.102187ms; same fresh reference0.292000ms from20000 observations. The predeclared series has4 alternating-order pairs,3 primary observations per side/process,12 per control. All40 rows remain:24 primary,8 warm-ups,8 excluded profile-tagged. Pair ratios0.968268969,0.978405291,0.971390605,0.993980235; median-paired0.974897948. The separate129-repetition sampled process is excluded. This is a modest gain; no confidence interval or larger-input throughput is claimed. Prior round18 after111.149597× used its own fresh reference and is not a continuous paired series with this result. [Committed timing](TIMING.md).

The small component improves substantially: steady successes855→183 allocation requests and133634→33723 requested bytes;64-step ordinary visits1144→19. Original/decoded literal results, overflow origin, input aliases, unchanged publication on failure, cleanup and retry agree. These opt3 System/probe diagnostics are separate from primary. Initial rows are also retained, including862/861 prior requests versus189 current. Their different warm state is not suppressed. Both rejected visits retain176 requests; requested bytes32883→33075(+192), with the same arithmetic error and unchanged publication/aliases/cleanup. Whole-compressor timing does not show the same component-scale gain.

Exact ten-byte raw bytes, independent inflation, capacity13 and final zero bytes/nodes pass for every primary/profile observation. Output backing changes; copies stay timed. Same pinned mimalloc2.1.7, unchanged evaluate_into-only harness, no competing compiler/test/benchmark. Largest complete input here is8 bytes. Mech performs compression itself.

## Algorithm/source fidelity

Pinned source671077457ff7ef3a3d44caf715a3fc595addf379; reference compiler d99c7b838fb05bb1047eb0c18d4d06d0f86f345d; executable SHA2563941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324. Caller source SHA256331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9; bytecode SHA256a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7. Fresh compilation/reencoding reproduce the exact4138627-byte artifact. All25 components, algorithm/tables,262144-entry U16 tree,1500513-word U32 cache,depth300/nice258,near-EOF rules,6 Huffman initializers, source allocation sites and timing boundaries remain unchanged. No source rewrite, new syntax or ownership annotation is introduced.

## Design and actual reached requirement

The exact source requirement is deflate-optimal-path.mec:56–80: optimal-matches returns {best:u32,item:u32,cache-ptr:u64}, immediately consumed by all three field reads. The value-level disposition maps all32 retained executions across sites12797/15674/27273 to nine private values in three components. Each has pack→return-copy→result-copy transport, no public input/output, and constant unique selectors with offsets0/1/2. The original first rejection is Record in NativeScalarAggregates::words; selected record observations were also unsupported. Every one of those nine values now selects. These are current selection facts mapped to the retained round16 trace, not fresh whole-program call counts.

Flat, parameterless records with1–32 supported scalar fields now use the existing NativeScalarAggregates Pack/Copy/Observe representation. Record constants retain canonical schema field order. The original selected constant-selector proof is factored once and reused by BufferFrames and scalar aggregates: exact operation/template relation, constant Id, unique matching hash and field offset. The scalar consumer additionally proves the exact supported field kind. The common memory/SSA aggregate emitter already loads these words, so no emitter, accessor, executor or ownership authority is added.

The complete fusion-expanded escape/copy/result-use analysis remains. A public escape or unsupported consumer rejects the affected component; nested fields, record enum payloads and unsupported kinds/widths do not gain scalar eligibility. BufferFrames eligibility and numeric result recipes are unchanged. The existing scalar-result record accessor remains for canonical cases; this work removes qualifying canonical records and access machinery rather than duplicating that accessor.

IR121 fingerprints selection/image rederivation; work93/ABI35/bytecode14 remain. All current scalar bits, source predicates, selected paths and checked arithmetic retain native emission. Actual frame/control work is admitted; canonical construction/binding/lookup that no longer executes is not fabricated. No earlier source failure is moved after another control visit. Whole work308968619891→307973385655(delta−995234236), using existing route prices/envelopes. This is not an equal-debit claim and no allowance increases.

## Validation

**429 unique scoped maintained Rust passes**:

| Scope | Executed | Result |
| --- | ---: | --- |
| Actual Engine budget/worker/projection/frame/text/collection root |227|Pass|
| Maintained component and record-buffer cohort plus scalar records |191|Pass|
| Selected native scalar correctness |8|Pass|
| Integer bits |3|Pass|

The4 focused tests are included in191. They cover state-machine return/copies/branch merges and every field, high unsigned bits, I64 extrema, Bool revisits, F64 signed zero/NaN payloads, original/decoded artifacts, current native image reconstruction, width32/33, public/nested escape fallback, assertion versus arithmetic in both orders, overflow, finite-boundary rejection, input aliases, unchanged prior output storage, cleanup and recovery. The selected witness runs with library compute/comparison caps1, establishing that removed canonical calls are not executed. Existing expected values/allowances remain unchanged.

A new test initially inferred an exact allowance from spent work939616, but setup required947844. This is an introduced test assumption, not a compiler denial regression or a raised allowance. Corrected probes start from the same published owner, restore it after every isolated attempt, and find the actual minimum within the unchanged successful policy. Minima947892/948688 and exact one-short rejections947891/948687 pass repeatedly in both artifact forms/source orders, preserving retained demand and publication. Initial failed source/run receipts are retained. Four parent regressions originally failed at missing record selection; no parser/build setup error was hidden.

All12 protected scalar timing pairs pass the unchanged≤1.05 guard(max1.008750146), with exact allocation requests/bytes and zero0-versus64-step allocation slope. No follow-up rerun was used. Inherited unresolved observations remain round10 decoded-U641.0526036 and round17 initial original zero-step loop1.052454987, whose fixed balanced follow-up did not reproduce>5%. Passing current controls do not erase either observation or declare scalar retirement complete.

All12 architecture/catalog/inventory/unsafe/warning/format checks pass; warnings are errors and the unsafe allowlist is unchanged. Non-native-artifact Engine metadata compilation passes(compile evidence only). Native image restoration rederives scalar-record selection through the existing approved test helper. Fresh original/decoded source artifacts match. No newly linked application runs.

Nightly2026-03-03,aarch64mac,locked/offline selected-operation release,opt3,jobs1,incremental0,existing five arithmetic families. Frozen production build455.240s. Semantic Engine units useopt0 and build43.123s; production/primary/scalar/GPU-test dependencies remainopt3. Engine run7.200s process/5.87s displayed;191 components72.131s process/72.12s displayed; non-native metadata13.102s. Non-primary test times are not performance comparisons.

Commands, full features/dependency hashes, source/binary hashes, exact outputs and nonzero counts accompany the qualification inventory:

```text
cargo build --release --locked --offline --manifest-path benchmarks/mech-deflate/native-profile/Cargo.toml --message-format=json --bin native_frame_transport --bin artifact_review
python3 /private/tmp/mech-scalar-records-round19-20261029/components/qualify.py
python3 /private/tmp/mech-scalar-records-round19-20261029/primary/run.py
python3 /private/tmp/mech-scalar-records-round19-20261029/full-components.py
python3 /private/tmp/mech-scalar-records-round19-20261029/engine-units/build.py
python3 /private/tmp/mech-scalar-records-round19-20261029/primary/qualify-selected.py
python3 /private/tmp/mech-scalar-records-round19-20261029/primary/scalar-controls.py
python3 /private/tmp/mech-scalar-records-round19-20261029/object-audit/build.py
python3 /private/tmp/mech-scalar-records-round19-20261029/object-audit/sections.py
python3 /private/tmp/mech-scalar-records-round19-20261029/after-witness/build.py
python3 /private/tmp/mech-scalar-records-round19-20261029/after-disposition/build.py
python3 /private/tmp/mech-scalar-records-round19-20261029/engine-units/nonnative-check.py
python3 /private/tmp/mech-final-static-20261028.py
```

## Generated code, profile and remaining work

Unboxed aggregate values91→117. Object11542304→10844664 bytes; executable text11541572→10843932 bytes; stack86496→86208 bytes. Current Object SHA256bf557a6bc5bb3086e8d89a28c575bff1380e86343975f3c64e792ecbec7a213d; text36e3f9f6487dfc38f728dc0a0a8c28b329a7e6ab4b41c5c5db88d424b9e064fb; full metadata28e9ba51fe81421f0bfb453134b2b1bd40227d06201f5484d8349a59e72efe44. Raw metadata and section comparison are retained. The190 numeric regions(45 read-only/145 writes),26 private mixed results and zero reported successful-backedge helpers remain. This is Object evidence, not a new linked executable or an instruction-cache bottleneck claim. Archived reference ARM64 loads/stores/allocation/direct-backedge excerpts were reexamined; no named source-to-hashed-procedure or new per-function reference timing is claimed.

The new1313-sample disjoint partition has289 other-helper,328 frame-action,320 ordinary-visit,226 selected-library and150 remaining weights. Frame-input admission133. Shares are diagnostic ancestry weights, not removable elapsed guarantees. Structural call/frame/control costs remain substantial; the small-input result does not establish how frequently inner match loops run on larger data.

Next is the separately assigned nested private-record composition: trace current tallied/costs/saved producers and consumers, then use one bounded witness in which parent pack consumes a private child and projection recovers its current array owners without intermediate canonical records. Preserve shape/owner/alias/failure rules and real reservations. Round18 opaque canonical siblings do not complete that work. Numeric multiple-array results and ordinary scalar-path grouping remain later assignments. No nested composition change is in this packet.

## Remaining failures and omissions

No current compiler or Rust-test failure remains. The introduced finite-test setup mistake is corrected and retained; expected parent red tests and injected-unwind/default-limit diagnostics are classified above. Primary has no resource denial or environmental blocker. The modest whole-program gain and inherited scalar observations remain limitations.

Skipped: larger/full-input compression/timing; unchanged Core21 contracts last run in round17; all-certified scalar primitives requiring omitted logic_not/logic_xor; a new generated runtime-only linked application;4 GPU cfg(test) private-fuel units. Previous assertion/corpus/CI/parser packages are not reopened. Current original/decoded components, public finite fuel/publication/retry, native image, Object and selected scalar/bit execution ran.

The latest completed automated review still names d48231ef04; request6495640889 for the prior37407827c is pending during this explicitly assigned next slice. Do not claim it is complete. PR #973 stays draft; new CI is cancelled by user direction and a fresh exact @codex review request accompanies publication. The task response/PR metadata give its exact head/run. The1× goal remains active at106.514339×.
