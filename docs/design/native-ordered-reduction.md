# R164: compact scalar and four-lane ordered native reductions

The compiler replaces a complete proved immutable U32 minimum/associated-item child with a compact typed reduction. Short intervals and remainders use a scalar loop; intervals with at least16 candidates use four-lane costs and an ordered reduction. The source algorithm, allocation sites and complete-call timing boundary do not change. Natural32KB improves12.307670% and natural16KB11.065162% in matched complete compression; the twenty-case geometric improvement is1.725622%. These are different aggregates.

## Design and provenance

Base **209193766599c1b125665b6063e9edf50d8a7997**; measured source **9017ab40d19047f39bf1832c5c85d05f00accb5d**; before measured compiler **19cebb4ca84a611c149ba9ac5d652156cf4042a5** (the production compiler published as209193766599c1b125665b6063e9edf50d8a7997). IR/work/ABI/MBC **184/107/39/14**. The final commit folds test module ordering, docs and evidence into this same optimization, in PR973 only. The measured source hash remains explicit. Production file hashes are checked against candidate-source-manifest.json; final ordering affects only a cfg(test) module declaration.

Recognition consumes the existing opaque whole-child completion witness and fresh array windows. A value-expression description is recorded during that same source traversal, not a second permission analysis. It requires exactly one unit induction, U32 minimum and associated item, both selected by the same strict comparison. Costs must normalize to supported low-U32 ring operations over immutable captures and paired forward/reverse current-array reads. The item depends only on the winning index and unchanged captures. Every advancing edge must agree. Unknown, cyclic, overwritten, carried-dependent or ambiguous expressions decline. The same-current-array length must justify a reverse read.

The source proof checks all terminal action inputs before their operation and branch inputs afterward. It rejects skipped private temporaries that the original terminal traversal can observe, and canonical plan read roles exclude escaping private definitions. After the pure reduction, only the exact final carrier fields are installed. The original terminal traversal constructs the result. Incoming best wins ties; among new improvements the earliest index wins, including ties across vector groups. Sums wrap at32 bits; U64 node reads extract low halves. No candidate array, runtime intrinsic, new host callback, source/site name recognition or current-authority cache is introduced.

Current flat geometry, active tag, nonempty interval and index width are checked at entry. The original shared scalar version remains for unsupported structure/current admission. Existing exact complete-child prices and visits are settled once; the complete source-plan and numeric-price reports are byte-for-byte equal. All16 finite-funded parent/current observations agree within their configuration. Live aliases, error prefixes, failure publication, cleanup and retry remain covered in original/decoded and direct/restored tests.

Source **331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9**, bytecode **a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7**, actual Object **fe6631fd2b1c411dda498637317cfdb65a7d79f0bd4a1a163080bab9bf0805cb**, allocator **38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874**. Preserve depth300/nice258, original EOF limits, tables, real262144-entry U16 tree/1500513-word U32 cache/305257-word U64 path, source allocation sites, input aliases and output clear/reuse (capacity13 for eight bytes). Logical source work is not a hardware instruction count.

## Actual executable and costs

The actual complete-compressor Object contains four static `umin.4s` reduction bodies, with `add.4s`, `cmhi.4s`, `tbl.16b` and `bsl.16b`. Bounded actual machine excerpts and exact commands are archived under actual-machine. Static copies are not dynamic visit counts. The compact body directly loads typed U32 costs and low U32 node halves, compares/selects in registers, and advances its induction. It has no per-candidate source helper, narrowing guard, price debit, settlement increment or heap allocation. Original source emission remains in fallback and terminal paths.

Generated text **8,894,664→8,903,912B (+9,248)**; entry stack **16,224→16,480B (+256)**. Cold restoration, outside evaluate_into, is37.442639917→36.905942875 seconds in one descriptive pair; no repeated startup improvement is claimed. Source compile26.138s, producer1.995s, actual Object generation129.4s and worker link5.589s are descriptive build observations.

The earlier vector-only implementation9f2eebd1389193f0ed8a97d9cc007dfe1a587f52 is preserved separately. Its five-case screen had natural32 **+0.278578%** and natural16−0.487257%; it did not solve the important natural-text cost. The refined compact scalar plus vector implementation below is a distinct cohort against the same retained R163 baseline. No pooling or vector-trip histogram is claimed. A standalone kernel probe's3.056947× speedup at256 candidates is diagnostic only, not complete-compression speedup.

## Complete compression

All20 pass exact reference bytes, independent inflate, input aliases, supplied prefix, capacity, clear/reuse, equal source work and zero final managed bytes/nodes. **3,968 primary /4,048 total calls**, eight balanced blocks, no trimming: **17 faster /3 slower** medians. Every unchanged1.05 gate passes. Three small adverse medians are retained; the worst is1.157951%. Reference zero medians remain below resolution with no invented denominator. Random1024 has a higher absolute level on both sides than older cohorts; its cause has not been established and cross-cohort differences are not attributed to this compiler change.

| Case | Before ms | Current ms | Before / Roc | Current / Roc | Change |
|---|---:|---:|---:|---:|---:|
|reymont-prefix-32769|22.839583|20.028562|5.242043×|4.596870×|-12.307670%|
|eight-literals|1.344979|1.348459|4.719225×|4.731433×|+0.258703%|
|33027-periodic-window|8.215854|8.079959|6.588496×|6.479518×|-1.654064%|
|random-32769|6.677834|6.648417|6.365904×|6.337862×|-0.440510%|
|repeated-32769|7.787583|7.634479|5.855326×|5.740210×|-1.965995%|
|16384-distribution-change|5.790854|5.744813|7.302464×|7.244405×|-0.795064%|
|reymont-prefix-16384|12.574375|11.183000|5.903462×|5.250235×|-11.065162%|
|random-1024|1.693021|1.657021|5.532747×|5.415100×|-2.126377%|
|repeated-1024|1.494563|1.485479|4.655958×|4.627660×|-0.607770%|
|reymont-prefix-1024|1.697688|1.651751|5.098162×|4.960212×|-2.705886%|
|320-overlap|1.323625|1.316833|4.397427×|4.374862×|-0.513136%|
|random-256|1.310396|1.308875|4.442019×|4.436866×|-0.115996%|
|128-biased-dynamic|1.141854|1.140062|3.910459×|3.904322×|-0.156938%|
|random-64|1.254375|1.252250|4.252119×|4.244915×|-0.169407%|
|32-high-entropy-stored|1.242125|1.241917|4.358333×|4.357604×|-0.016745%|
|eight-overlap|1.304417|1.319521|4.497988×|4.550072×|+1.157951%|
|eight-literals-prefix|1.341521|1.352937|4.516906×|4.555343×|+0.850974%|
|seven-stored-prefix|0.120542|0.120125|below resolution|below resolution|-0.345938%|
|one-stored|0.120604|0.120396|below resolution|below resolution|-0.173294%|
|empty|0.116875|0.116500|below resolution|below resolution|-0.320856%|

## Independent counters and memory

CPU counters and heap probes are separate cohorts: eight CPU/two heap observations per side above1024B, sixteen CPU/four heap otherwise. Eleven instruction medians decrease and nine increase; fourteen cycle medians decrease and six increase. CPU sampling does not partition retired instructions, and counter cohorts are not multiplied into elapsed claims.

| Case | Before instructions | Current instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
|empty|1061711.5|1061576.5|-0.012715%|-0.171486%|
|one-stored|1114290.0|1114168.0|-0.010949%|+0.298935%|
|seven-stored-prefix|1114049.5|1113928.0|-0.010906%|-0.167728%|
|eight-literals|12663376.0|12675121.5|+0.092752%|-1.312036%|
|eight-overlap|11856569.5|11868175.5|+0.097887%|+0.040818%|
|320-overlap|12456554.0|12432715.5|-0.191373%|-1.609293%|
|eight-literals-prefix|12663440.0|12677508.5|+0.111095%|+0.701954%|
|128-biased-dynamic|10635046.0|10644837.5|+0.092068%|-1.142710%|
|32-high-entropy-stored|11244802.5|11255788.5|+0.097698%|+1.688974%|
|16384-distribution-change|73492768.5|73634327.0|+0.192616%|-0.101908%|
|33027-periodic-window|149273162.0|147342040.0|-1.293683%|-1.727602%|
|random-64|11413827.0|11424550.5|+0.093952%|+0.058219%|
|random-256|12018395.5|12028295.5|+0.082374%|+0.806948%|
|random-1024|12406960.0|12418632.0|+0.094076%|-0.217886%|
|repeated-1024|15685235.5|15581686.5|-0.660169%|-1.223714%|
|random-32769|87025307.5|86888929.0|-0.156711%|-4.813545%|
|repeated-32769|143041611.0|141129115.5|-1.337020%|-10.596150%|
|reymont-prefix-1024|17442918.5|17054015.0|-2.229578%|-3.307470%|
|reymont-prefix-16384|150541157.0|142509226.0|-5.335372%|-10.395133%|
|reymont-prefix-32769|274604970.5|260132269.0|-5.270371%|-11.395186%|

All20 heap request counts, requested bytes, reallocations and incremental peak live usable bytes remain exactly equal. This is an instruction/control-flow optimization; no input streaming, copy-volume or memory reduction is claimed.

| Current heap | Requests | Requested B | Incremental peak usable B |
|---|---:|---:|---:|
|eight-literals|3354|17643969|15279752|
|16384-distribution-change|5679|24105386|18702352|
|reymont-prefix-32769|6449|24793116|18722528|
|repeated-32769|3135|15712250|15313264|
|33027-periodic-window|3679|15798298|15314928|

All12 scalar controls pass the unchanged1.05 gate with equal requests/bytes and no iteration allocation slope. After/before ratios range0.991571280–1.004312972. Adverse scalar observations remain in scalar-balanced/summary.json.

## Current profile and next step

These fresh complete-call CPU-time samples use the actual current executable, five maintained inputs, concurrent stdout draining and the same exact-byte/work/cleanup checks. Contexts are exclusive; they are not a retired-instruction ledger.

| Case | Generated exclusive | Host callbacks/cleanup | Outside |
|---|---:|---:|---:|
|128-biased-dynamic|12.830%|71.851%|15.319%|
|eight-literals|12.457%|78.439%|9.103%|
|reymont-prefix-32769|82.482%|15.992%|1.526%|
|repeated-32769|83.622%|14.314%|2.064%|
|33027-periodic-window|80.892%|17.377%|1.731%|

The selection study refines the same retained R163 exact-PC samples using four inspected intervals: natural path-length time lower bound25.842697%, repeated word-extension44.953628%. Those are historical selection contexts, not newly measured R164 shares or whole-call instruction counts. The retained reference's inspected cost kernel is scalar; this experiment does not claim it uses SIMD there.

The selected path-cost compiler experiment is now qualified. The alternative match-extension unroll/wider comparison was not implemented. Step7 remains: inventory prior source variants, then choose one separately pinned source experiment using the remaining profile. Source and compiler changes must retain separate comparisons. Parity is not achieved.

## Validation and disposition

Nightly2026-03-03/aarch64; GPU library opt3, unit/component roots opt1; warnings denied. Core/Engine/math/dependencies remain frozen R145 artifacts. Native feature superset: aot,jit,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime. Runtime-only also passes. Exact commands/features are retained in library-build.json, compact-qualified/build.json, gpu-compatible.json and checks/receipts.json.

- 615 compatible GPU tests pass, same seven exclusions.
- 27 focused source/recipe/original/decoded/direct/restored tests pass.
- 198 CI-inventory tests are listed, equality-guarded and executed.
- Unchanged236 component,8 composition and1 real-capacity path controls pass.
- 132 workflow tests,15 architecture/catalog checks, runtime-only and formatting pass.
- 12 balanced scalar controls pass, without larger allowances or changed expected values.

Focused controls cover zero/short/max/remainder counts, nonzero/exhausted starts, cross-group and incoming ties, U32 wrap, nested/repeated visits, current lengths, aliases, finite resource boundaries, unchanged publication and recovery. Deliberate mutations reject changed tie predicates, different min/item predicates, carried-item dependencies, wrong-array length identities, overwritten captures and terminal dependencies on skipped private state. Inherited fallible arithmetic, early-exit and nonunit-step controls remain.

Repaired introduced failures: initial Rust expression-recording/variable-name errors; an overconservative terminal-dependency observation initially recorded branch inputs before its defining action (fixed to observe afterward, without weakening the proof); test module ordering failed formatting then passed. Environmental failure: disk exhaustion stopped the worker link after Object generation, corrected by removing inventoried obsolete test executables. Harness/diagnostic failures: a stale descriptor path was correctly identity-rejected before compression; the GPU runner initially lacked its copied specification; the first disassembly matcher expected the wrong LLVM mnemonic spelling. Corrected receipts and contrary observations remain. No unresolved functional failure remains.

The same seven inherited GPU exclusions, fresh full-dependency/all-feature/minimal graphs, an additional standalone AOT suite, full multi-MB timing and separate ten-workload suite remain skipped. The actual restored linked compressor runs all20 cases. Ordinary hosted CI is cancelled by standing instruction: no green-CI claim, and PR973 remains draft pending green CI and clean exact-head review. No new PR is opened.

Evidence excludes native binaries, MBC, raw trace stores, copied assignments and private environment metadata. Run checkpoint.py INPUT OUTPUT to reconstruct results.json and REPORT.md without compiling or compressing. Raw archives retain the earlier implementation and all adverse cohorts.

Decision: **Retain compact scalar and four-lane ordered reduction: all twenty unchanged case gates and twelve scalar gates pass; natural32 improves 12.307670 percent and natural16 11.065162 percent, with 1.725622 percent geometric elapsed improvement. Preserve three adverse small-case medians, nine adverse instruction medians and six adverse cycle medians.**
