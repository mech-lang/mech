# R162: unsigned word facts in native branch cleanup

The compiler now proves unsigned ranges from immutable machine values before its existing branch cleanup. Literal values use their actual scalar integer width; AND reuses the maintained range transfer, and zero extension, narrowing and unsigned comparisons retain only proved bounds. Unknown parameters, loads, joins and exhausted analysis keep their actual full width. Signed, vector and floating interpretations receive no unsigned assumptions. The existing checked operations, selected effects, source debits and one final backend data pass remain. There is no new runtime dispatch, allocation or permission cache. The complete current paired-view guarded children from R161 remain available alongside their checked bodies.

Published base #988: d597f5468c83f6547013e0007e4ab3bd380969e0. Test-only parent6036cbb059047be57ade160e0beb36213f60ae12 passes six controls and fails the new width-elision expectation. Semantic commit632780e00f67a582b0976cb55fad9b0ddf400a35; separate CI inventory a97aad5a19; canonical module layout and measured compiler **3ef813384ab5a1df7a90818ea89506c62ad03375**. IR/work/ABI/MBC184/107/39/14. The final and screen controls both use qualified R160 compiler4fe3888f4d98d1ea159cd3186a8bc0f3b81ef773, rather than the failed R161 timing candidate. The module move is verified whole-Object-identical to the initial implementation.

Actual primary Object20b3400b793d0a3af12d5650f77841197ffb64ea71b8a860347fa2f0ec2aaedb; code9,024,768 to8,955,288B and entry stack15,808 to16,208B. Early cleanup folds602 statically proved branches, versus0 at R160; pre-codegen whole-function data passes stay zero. Diagnostic source markers reproduce the whole primary Object exactly. These are static counts, not executed counts.

All20 complete-stream cases pass exact raw bytes, independent inflate, source work, output prefix/capacity/clear-reuse, immutable aliases and final managed bytes/nodes zero. Source SHA256331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9 and MBCa2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7 stay pinned. Keep the262144 U16 tree,1500513 U32 cache,305257 U64 path capacity, depth300, nice258, original EOF rules, source allocation sites, allocator38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874 and evaluate_into timing unchanged.

The final cohort contains3,968 primary observations with no trimming. 8 medians improve and12 worsen. Geometric after/before0.988343672917 (-1.165633%); unchanged1.05 gate failures: []. No separate development screen is pooled into this cohort. Ratios use the fresh reference cohort; zero reference medians remain below resolution and receive no invented denominator.

| Case | Before ms | Candidate ms | Before / Roc | Candidate / Roc | Change |
|---|---:|---:|---:|---:|---:|
|eight-literals|1.333438|1.355166|4.613971x|4.689157x|+1.629548%|
|16384-distribution-change|5.965791|5.898750|7.561206x|7.476236x|-1.123765%|
|33027-periodic-window|8.517813|8.322000|6.792514x|6.636364x|-2.298865%|
|random-32769|6.669583|6.772063|6.355010x|6.452657x|+1.536520%|
|repeated-32769|8.078688|7.887896|6.037883x|5.895288x|-2.361665%|
|reymont-prefix-32769|27.066688|23.228938|6.223658x|5.341214x|-14.178867%|
|reymont-prefix-16384|14.834688|12.789146|6.964642x|6.004294x|-13.788912%|
|random-1024|1.233625|1.253729|4.057977x|4.124109x|+1.629669%|
|repeated-1024|1.505937|1.515042|4.619440x|4.647368x|+0.604574%|
|reymont-prefix-1024|1.781438|1.719313|5.365776x|5.178652x|-3.487352%|
|320-overlap|1.325875|1.334000|4.375825x|4.402639x|+0.612765%|
|random-256|1.303813|1.323271|4.480455x|4.547323x|+1.492431%|
|128-biased-dynamic|1.128396|1.157542|3.918042x|4.019243x|+2.582958%|
|random-64|1.247729|1.269437|4.273045x|4.347389x|+1.739841%|
|32-high-entropy-stored|1.224562|1.244479|4.193707x|4.261916x|+1.626458%|
|eight-overlap|1.294896|1.315500|4.426995x|4.497436x|+1.591170%|
|eight-literals-prefix|1.340188|1.360667|4.669643x|4.741000x|+1.528107%|
|seven-stored-prefix|0.120125|0.119896|below resolution|below resolution|-0.190635%|
|one-stored|0.120041|0.119834|below resolution|below resolution|-0.172857%|
|empty|0.116333|0.116334|below resolution|below resolution|+0.000860%|

| Separate CPU cohort | Before instructions | Candidate instructions | Instruction change | Cycle change |
|---|---:|---:|---:|---:|
|empty|1072107.0|1061562.0|-0.983577%|-0.186190%|
|one-stored|1124746.5|1114195.5|-0.938078%|-0.179355%|
|seven-stored-prefix|1124510.0|1113875.0|-0.945745%|+0.430956%|
|eight-literals|12685635.0|12669178.5|-0.129725%|+0.534138%|
|eight-overlap|11877269.5|11862667.5|-0.122941%|+0.548192%|
|320-overlap|12560292.5|12478847.5|-0.648432%|+0.423801%|
|eight-literals-prefix|12687291.5|12671531.5|-0.124219%|-1.263320%|
|128-biased-dynamic|10663263.0|10638464.5|-0.232560%|+1.017777%|
|32-high-entropy-stored|11275444.0|11251155.0|-0.215415%|-1.039784%|
|16384-distribution-change|76086639.5|73462231.5|-3.449236%|-13.710444%|
|33027-periodic-window|152604197.0|150451866.5|-1.410401%|-2.761415%|
|random-64|11453156.0|11420548.5|-0.284703%|-0.064351%|
|random-256|12104835.5|12026224.5|-0.649418%|+1.198427%|
|random-1024|12559537.5|12416142.0|-1.141726%|+1.832205%|
|repeated-1024|15954906.5|15743441.0|-1.325395%|-3.279427%|
|random-32769|90169161.5|87321441.5|-3.158197%|+0.128678%|
|repeated-32769|146353779.0|144214043.5|-1.462030%|-2.966852%|
|reymont-prefix-1024|18297886.0|17564003.5|-4.010750%|-4.228382%|
|reymont-prefix-16384|169969901.5|153045872.0|-9.957074%|-12.511041%|
|reymont-prefix-32769|309933473.5|279098833.5|-9.948793%|-13.313405%|

All20 heap equality for request count, requested bytes, reallocations and incremental peak live usable bytes: **True**. Requested bytes are not initialized copying or live memory. CPU counters, heap instrumentation, samples and primary elapsed are separate cohorts. All12 scalar controls use the unchanged1.05 gate; ratios0.995578 to1.003904; failed controls[].

| Current complete-call heap cohort | Requests | Requested bytes | Incremental peak live usable bytes |
|---|---:|---:|---:|
|eight-literals|3354|17643969|15279752|
|128-biased-dynamic|3162|15161401|15275488|
|reymont-prefix-16384|6805|24316733|18702048|
|reymont-prefix-32769|6449|24793116|18722528|
|repeated-32769|3135|15712250|15313264|
|33027-periodic-window|3679|15798298|15314928|

| Separate CPU cohort OS residency after call | Before RSS bytes | Candidate RSS bytes | Before lifetime peak footprint | Candidate lifetime peak footprint |
|---|---:|---:|---:|---:|
|eight-literals|323403776|323272704|233490112|233522944|
|reymont-prefix-32769|324714496|324616192|233490112|233522944|
|repeated-32769|324698112|324550656|233490112|233522944|
|33027-periodic-window|324091904|323944448|233490112|233522944|

OS residency and lifetime peaks include prepared code, metadata, harness and earlier case history; they are not isolated call peaks or copied-byte measures.

| Actual executable CPU-time samples | Generated exclusive | Host callbacks/cleanup | Outside/unrecognized |
|---|---:|---:|---:|
|128-biased-dynamic|11.09%|74.03%|14.88%|
|eight-literals|13.27%|77.98%|8.76%|
|reymont-prefix-32769|84.39%|14.15%|1.46%|
|repeated-32769|85.68%|12.53%|1.79%|
|33027-periodic-window|80.40%|17.28%|2.33%|

| Source origin | R160 bytes | Candidate checked bytes | Candidate guarded bytes |
|---|---:|---:|---:|
|15551|12|12|0|
|15556|8|8|4|
|15566|16|0|0|
|15578|12|12|0|
|15583|4|4|4|
|15590|16|0|0|
|15595|16|0|0|

Origin bytes can include shared/hoisted/cold instructions; CPU-time samples are not retired-instruction partitions. The exact dynamic visits of removed guards have not been instrumented.

Passing qualification:602 compatible GPU tests with the same seven inherited exclusions;22 focused controls;185 CI-selected and actually executed tests;236 unchanged components,8 composition and1 real-capacity path;132 workflow tests;15 architecture/catalog checks; runtime-only, formatting and warnings as errors. Current GPU/root harnesses use nightly-2026-03-03 aarch64 with frozen R145 Core/Engine/math dependencies. The selected test graph is the retained native feature superset, not a newly rebuilt minimal/all-feature graph. Original/decoded, direct/restored execution, masked/immediate masks, actual8/16/32-bit truncation, unmasked/too-wide values, raw noncanonical truth, unknown joins/loads, signed/vector/float refusal, depth exhaustion, source arithmetic errors, current aliases, unchanged publication and retry remain covered. Six finite resource boundaries preserve their exact existing allowances.

Repaired introduced issues: the first test-report format requested a nonexistent Debug implementation; printing the numeric report field fixed it. The initial broad runner was omitted from the local harness copy list and failed before test execution; the unchanged adapted runner then executed602 tests. The new module initially violated the file/directory contract; moving it into word_ranges/mod.rs fixes the contract without an exemption. The initial exit receipt and exact failure reason are retained; per-check stdout was overwritten by the successful resume. The post-move Object is byte-identical, and22 focused checks are rebuilt against that current source. One overlapping final image build received external SIGKILL with no compiler denial; its exact OS cause is unknown. The serial producer then completed and wrote all image files, but the optional OS-statistics wrapper returned1 on restricted kern.clockrate access. Both nonzero receipts remain; matching primary/initial/diagnostic whole-Object hashes permit finishing the link without another compiler run. No peak-RSS value is claimed from that failed statistics wrapper.

The prior R161 biased128 gate failure remains preserved. This changed compiler uses a new matched cohort with substantially different absolute timing levels on that case; passing the current gate does not establish a causal explanation or erase the earlier failure. Eight-byte instructions fall while elapsed and cycles rise; both cohorts remain reported separately.

Ordinary hosted CI is cancelled by standing instruction; this is not a green hosted-CI claim. New PR stays draft and receives exact-head automated review. All diagnostic evidence excludes native binaries, bytecode and raw trace stores. Full multi-MB timing and the separate ten-workload suite remain outside this bounded matrix. The inherited cold image-validation cost remains outside evaluate_into; this round makes no startup repair or matched startup-speed claim.

Decision: **Retain the combined current-window and unsigned-word compiler candidate: every unchanged per-case/scalar gate passes and geometric elapsed improves against qualified R160.** Parity is not achieved. Whole-body totality, exact settlement of the existing prepaid loan, ordered reduction/unrolling/SIMD and a separate pinned source variant remain incomplete.

Replay checkpoint.py INPUT OUTPUT to reconstruct results.json and REPORT.md without compiling or compressing.
