# R201: optional LLVM complete-compression comparison

**Do not adopt the tested LLVM22 configuration.** It executes the unchanged complete level-12 program correctly on all twenty cases, but seventeen elapsed medians and seventeen instruction medians are higher than the matched Cranelift control. Only three of twenty original 1.05 performance gates pass. Geometric elapsed is **77.109836% higher**. Retained #973 production stays at R199 / IR214; parity remains incomplete.

This is an **O3 IR / O3 machine-emission trial with fast register allocation and peephole plus pre/post machine LICM disabled**. The ordinary optimized-allocator configuration remains unmeasured. Its compile was sampled in expensive branch matching, machine LICM and then LiveVariables analysis on the large entry; stopping those build-only experiments does not establish their eventual runtime performance.

## Revisions and comparison boundaries

- Retained review head: `a7c216810387b049516078478165564a8c29cb56`; retained production change: `7748204bd12e57a20369ce7970f31f8d15c3901f`. PR base: `92f75db84e2523693648084bea16cd52ad60777a`.
- #990 reference head: `c39f78c403e3de12f4e65c222ddd0f2f7b227362`. The linked LLVM implementation tested here is `b638320000f7cb33837df2ee73f09221551826a0` plus the exact hash-listed isolated patch, not a claim that #990's published related-post timing is a compression result.
- Experimental IR/work/ABI/bytecode: **264/115/41/14**, Cranelift image9 / LLVM image10. Retained: **214/107/39/14**. Both experimental workers use the same final compiler/runtime libraries, selected catalog and pinned allocator.
- The pure backend effect is **LLVM versus experimental Cranelift**. Comparing either with retained R199 also changes revision, feature/catalog/build configuration and machine image. The separate control is slower than retained R199 on seventeen cases (16 gates fail), so that difference must not be attributed to LLVM.
- Source SHA-256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; MBC SHA-256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Canonical bytecode decode/encode is byte-identical. No source or bytecode is changed.
- Complete algorithm, tables, depth300/nice258, original EOF/search rules, tree262144 U16, cache1500513 U32, path305257 U64 capacity, source allocation sites, input aliases, output capacity/clear preparation, pinned mimalloc2.1.7 and evaluate_into timing boundary remain. No library performs compression for Mech.

## All twenty primary cases

All multiples use the same fresh level-12 reference denominator within each row. Lower is better. Three stored-case medians quantize to zero in the reference clock; their absolute times are retained and their multiples remain undefined.

| Input | Retained / Roc | Matched Cranelift / Roc | LLVM trial / Roc | LLVM / matched Cranelift |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.361839× | 3.634838× | 10.238742× | 2.816836× |
| random-32769 | 4.759803× | 4.784359× | 15.602979× | 3.261247× |
| 16384-distribution-change | 6.229034× | 6.672522× | 15.679642× | 2.349883× |
| repeated-32769 | 2.893338× | 3.888302× | 12.693051× | 3.264420× |
| 33027-periodic-window | 3.398392× | 4.503846× | 13.987677× | 3.105719× |
| eight-literals | 4.520977× | 5.781542× | 8.670089× | 1.499615× |
| reymont-prefix-16384 | 4.032735× | 4.271147× | 11.013461× | 2.578572× |
| empty | unresolved | unresolved | unresolved | 0.998507× |
| one-stored | unresolved | unresolved | unresolved | 0.996032× |
| seven-stored-prefix | unresolved | unresolved | unresolved | 0.996191× |
| eight-overlap | 4.342097× | 5.564297× | 8.468390× | 1.521916× |
| 320-overlap | 4.162891× | 5.377027× | 8.521042× | 1.584713× |
| eight-literals-prefix | 4.483665× | 5.736687× | 8.569012× | 1.493721× |
| 128-biased-dynamic | 3.739440× | 4.930964× | 7.795472× | 1.580923× |
| 32-high-entropy-stored | 4.044005× | 5.238804× | 8.081490× | 1.542621× |
| random-64 | 4.167241× | 5.419540× | 8.335559× | 1.538057× |
| random-256 | 4.270192× | 5.456265× | 8.714753× | 1.597201× |
| random-1024 | 3.876310× | 5.024765× | 8.686672× | 1.728772× |
| repeated-1024 | 4.055129× | 5.224295× | 8.937180× | 1.710696× |
| reymont-prefix-1024 | 4.584651× | 5.651294× | 9.738517× | 1.723237× |

Twelve balanced blocks use all six variant orders twice, putting each executable in each position four times: **8,928 primary observations / 9,048 complete calls**. Every call checks exact reference bytes, independent inflate, current aliases/capacities and work; experimental backend work agrees exactly. All cases and final sessions end at zero managed bytes/nodes. No compiler, profiler or archive runs during primary or CPU/heap measurement. Image loading is outside the existing compression boundary and is reported separately (concurrently loaded workers ready after about50.7/70.3/70.3 seconds; these are not isolated startup benchmarks).

## Independent instructions and memory

Separate CPU and heap cohorts each execute26 complete calls per case/variant (24 retained observations plus warmup/profile): **3,120 complete diagnostic calls** in total. Allocation requests, requested bytes, reallocations and incremental allocator usable peaks are equal between matched Cranelift and LLVM in all twenty cases. This is not a claim about copy volume or streaming. Source capacity reuse is checked; physical zero-allocation output backing reuse is not established.

| Input | Matched instructions | LLVM instructions | Matched cycles | LLVM cycles |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 216,829,232.0 | 759,272,211.5 | 48,677,125.5 | 143,200,922.0 |
| random-32769 | 76,267,476.5 | 309,883,386.5 | 16,530,080.0 | 52,747,678.5 |
| 16384-distribution-change | 70,604,350.5 | 194,305,993.0 | 17,354,993.5 | 39,892,867.0 |
| repeated-32769 | 79,354,617.5 | 288,188,328.0 | 18,011,164.5 | 55,148,695.0 |
| 33027-periodic-window | 85,482,881.5 | 296,058,855.5 | 18,187,700.5 | 56,671,456.5 |
| eight-literals | 16,644,759.5 | 24,497,247.0 | 5,654,483.5 | 8,792,050.5 |

Natural32 allocator requests are6,486 on both experimental backends, requested bytes24,505,396 and incremental usable peak18,362,880. Eight literals have3,227 requests,17,315,577 requested bytes and14,920,104 incremental usable peak on both. Process RSS after natural32 is about278MB/227MB, and physical footprint192MB/171MB for matched Cranelift/LLVM; these include image loading and retained setup state and are not compression-only peak RAM. Stack use is separate from allocator counters.

| Image | Accounted generated code | Reported native stack envelope |
|---|---:|---:|
| Retained R199 | 8,152,436 B | 16,480 B |
| Matched Cranelift | 8,750,808 B | 17,808 B |
| LLVM trial | 8,387,920 B | 481,792 B |

The LLVM code is4.15% smaller than matched Cranelift while its reported stack envelope is about27.05× larger. This configuration trades register quality for feasible compilation. The larger stack, higher retired instructions and actual spill-heavy hot-loop assembly support poor generated code as the next investigation; no exact dynamic spill ledger or optimized-allocator counterfactual is claimed.

## Fresh executable profiles and binary evidence

Four cases per experimental backend profile the exact primary executable hashes, sequentially after counter collection. Cranelift completes5,202 correct compression calls and LLVM2,720, all with zero final managed bytes/nodes. Wall-stack weights are disjoint and fully reconciled; they are not retired-instruction samples.

| Case | Cranelift generated-exclusive share | LLVM generated-exclusive share |
|---|---:|---:|
| 33027-periodic-window | 77.120% | 88.241% |
| repeated-32769 | 78.305% | 91.792% |
| reymont-prefix-32769 | 78.334% | 90.985% |
| eight-literals | 28.391% | 45.541% |

The numeric837 and numeric15296 functions dominate the large cases. The actual numeric837 prologues reserve about2.4KB under Cranelift and101KB under LLVM. Numeric15296 grows1,047→1,536 static instruction words and199→919 direct SP/FP memory operations. These are static counts, not dynamic traffic. Numeric837 uses more derived frame addresses in LLVM; direct-SP/FP counts alone are a lower bound and are not compared as total stack accesses. Full function assembly and receipt-replay counters remain in evidence. Useful host work remains present; no sample percentage is multiplied into hardware instructions.

## Narrow enablers and compiler-path findings

The full pinned computation first fails on vector constants, then byte shuffles and lane extraction in numeric12419 (block761/inst9397, block763/inst9416, block764/inst9443). Each has a minimal failing control against the unmodified emitter and a native regression. The patch preserves raw constant bytes, exact16-byte shuffle indices and typed lane extraction. Tests cover six vector kinds, NaN payloads/signed zero, unaligned buffers, every source lane at every destination and every extracted lane. No compression-specific intrinsic or source rewrite is added.

The LLVM22 fixture also fails on the clean parent: its reports use prologepilog while the original filter/parser expects prolog-epilog. The repair requests actual reports, accepts the two specific known spellings and retains closed definition sets, duplicate/overflow/missing/truncation/target rejection. Refusal tests now first require a valid real-object fixture. Rust and Python consumers both execute.

The native SSA handed to LLVM has128,708,015 bytes,714,593 blocks and3,733,244 lines; its entry alone has581,267 blocks. Ordinary O3 reduces it to47,206,640 bytes and122,203 blocks. Build samples localize expensive IPSCCP dominator updates, AArch64 branch peepholes, machine LICM and LiveVariables. These are compiler-build observations, not compressor timings. No resource denial occurs; long experiments are deliberately stopped and their receipts retained. A local SCCP/precleanup alternative remains unqualified and is not selected. O0 machine emission succeeds in4.433s but is not the timed configuration. O3 with fast allocation succeeds in97.050s in the standalone witness.

A build-only opt cache reuses the successfully completed original default<O3> output only after checking the exact fresh input hash and cached-output/real-opt hashes. It does not replace an executable descriptor. The final producer prepares the source again, checks the cache match, emits machine code afresh, derives real code/stack facts, constructs new LLVM-family metadata/identity, and audits final linked code. Its302.114s receipt includes this cache reuse and is not a cold compiler-speed claim. Different build witnesses overlap; their elapsed or hardware counters are not added into a compile-time comparison.

## Qualification, failures and scope

**14 Rust/native tests / eight real-object metadata tests pass**, zero ignored or filtered. Native tests execute the actual maintained emitter/parser modules with the selected dependency profile and warnings denied. All20 complete-program cases execute in original pinned MBC-derived images on both backends and all three worker kinds. Format, whitespace and complete patch reverse-application checks pass. Exact source-file hashes, Cargo.lock, selected features, compiler/tool versions and command arrays are archived. No math machine crate is rebuilt.

Initial isolated-manifest/registry/offline setup, helper-object linkage, auto-linked benchmark bins, a constant-pool indexing typo, a generated shuffle-field name, a raw-report hash expectation and formatting attempts are corrected. Expected parent regression failures and the LLVM22 parent fixture failure are retained. Intentionally stopped compiler alternatives remain explicitly aborted. The failed-command inventory and disposition ledger identify every retained nonzero command; there is no outstanding introduced functional failure. Optimized-allocator scalability and its runtime performance remain unqualified, and the separate experimental-control regressions remain unexplained by this backend comparison.

Broad GPU/native/compiler CI, registry-generated projects, all-features math distributions, separate workload suites, fresh failure/retry/publication qualification of the entire optional backend, multi-megabyte input and full-corpus timing are not rerun. Prior R199/R200 coverage is not relabeled as fresh LLVM coverage. This experiment supplies no production-adoption or whole-backend correctness certification. The existing #973 draft receives the review packet only; no new PR or default backend is created.

The user directed the next work back to the retained approximately 3.3× cases. Keep R199 as the performance baseline, profile that exact executable and target one demonstrated native-loop cost at a time, validating all twenty cases. Optimized LLVM allocator scalability is recorded as an unqualified future option, not the next active work package.

## Replay

After unpacking, run heap-summary.py then analyze.py. Profile summaries and their hash-pinned receipts are retained; rebuilding stack trees requires the local raw samples and analyze-profiles.py. They reconstruct all numerical outputs from retained receipts without executing a compiler or compressor. Native binaries, bytecode, executable descriptors, full generated LLVM IR and raw stack traces are excluded from the public packet.

[Immutable evidence and numerical replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r201-llvm-compression-comparison-1006d461b65f.tar.gz) — SHA-256 `1006d461b65fa396bfbc2ef400f133b9b016569fb5feb2f3b055e63cae18b6a8`, 4,272,254 bytes / 508 hash-listed files. A fresh public download verified every file and reproduced results.json byte for byte without compiler or compressor execution.
