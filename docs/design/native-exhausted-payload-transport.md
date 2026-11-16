# Exhausted payload transport in native reductions — R178

Fix review finding 4489282114. The old reduction could compute the correct minimum, then skip a required payload update while changing the active state to its terminal state. A generic program resetting best to zero returned native (9,1,18) instead of resident (0,1,18). The maintained regression fails on the clean published parent. Original and decoded programs now agree.

The compiler records exhausted latch payloads separately from advancing latch payloads. Before bypassing that source edge, the ordered reduction requires every payload word to be the same current carrier word on every exhausted path. Missing, unknown, conflicting or overflowed evidence declines only that reduction. The existing native source loop then executes the actual final updates. Completion/totality, current windows and complete-child funding remain separate proofs. The terminal dependency proof still excludes omitted private temporaries. No source price, error order, ownership, ABI or allocation contract is widened.

## Correctness evidence

The independent resident comparison covers identity, changed best, changed item, both changed and a conditional exhausted update; fresh current arrays/values across repeated invocations; original/decoded; direct/restored; zero, exhausted, reversed, short, vector and remainder intervals; aliases and cleanup. It checks every published output and a literal expected tuple. Identity exhaustion stays eligible; changed payloads execute the existing native source path.

The second source regression uses the existing finite-boundary search for control/library/comparison/memory/nodes/transport, without raising any allowance. It verifies exact success boundaries and the preceding rejection where applicable, unchanged prior storage/content and inputs on failure, cleanup, and retry. A proof regression rejects absent exhausted edges, missing payloads, unknown/out-of-range nodes and a bad second exit path.

Clean published parent plus the new test: build succeeds and the value test exits 101 with native 9 versus resident 0. Corrected source: 17 focused/635 GPU tests pass with the same seven inherited exclusions. All 218 native CI-selected names execute. The equality guard changes 215→218 solely for the three new named tests; selection and execution remain guarded. An independent external probe also confirms original and decoded native 0 equals resident 0, with changed-payload reduction declined.

An introduced conditional-test syntax failure is preserved. The same conditional case now uses an existing branch-local binding, rather than a direct transition argument; no parser change was made. Focused and full qualification were rebuilt/rerun afterward. The initial parent failure still exercises the unchanged valid constant-reset case.

Fresh 236-component/eight-composition/one-real-capacity-path qualification, all 15 architecture/catalog checks, 132 workflow tests, runtime-only, formatting and warnings-as-errors pass. Expected values, source resource allowances and the 1.05 latency/scalar gates are unchanged. All 12 original/decoded Bool/U64/I64/F64 and 0/64-loop controls pass without allocation growth or iteration-dependent allocation. The largest scalar after/before ratio is 1.004162, below the unchanged 1.05 gate.

## Compression timing

This is a correctness fix, with no speedup claim. The compressor retains exactly its prior canonical plan and complete numeric recipes. Its entire __TEXT/__text and actual entry bytes are byte-identical to R177. Object identity changes under IR198. Code stays 8,955,224 bytes; stack stays 15,792 bytes. The unchanged inspected scalar continuing visit remains 16 instructions without a length stack reload; the four-candidate vector body remains 25 with its existing length reload.

One predeclared eight-block balanced cohort contains 3,968 primary/4,048 total complete calls. All 20 unchanged 1.05 gates pass: 9 faster/11 slower pooled medians; geometric elapsed change -0.105306%. Natural32 has eight adverse blocks. Zero-resolution reference medians remain unresolved. No observation is trimmed and ratios from independent rounds are not compounded.

| Input | Before / Roc | After / Roc | Elapsed change | Improving/adverse blocks | Instructions change |
|---|---:|---:|---:|---:|---:|
|reymont-prefix-32769|4.174178×|4.193258×|+0.457096%|0/8|-0.019709%|
|reymont-prefix-16384|4.941493×|4.959524×|+0.364893%|2/6|-0.001461%|
|random-32769|5.370558×|5.389583×|+0.354246%|4/4|+0.000255%|
|16384-distribution-change|6.974300×|7.009917×|+0.510686%|2/6|-0.002516%|
|repeated-32769|3.169128×|3.199825×|+0.968629%|2/6|-0.000836%|
|33027-periodic-window|3.710310×|3.712902×|+0.069851%|3/5|+0.003631%|
|eight-literals|4.557791×|4.582620×|+0.544754%|2/6|-0.000841%|
|empty|unresolved (zero median)|unresolved (zero median)|-0.196110%|3/5|+0.017284%|
|one-stored|unresolved (zero median)|unresolved (zero median)|-0.121082%|5/3|+0.023604%|
|seven-stored-prefix|unresolved (zero median)|unresolved (zero median)|-0.139022%|3/5|+0.019569%|
|eight-overlap|4.469898×|4.454310×|-0.348731%|3/5|-0.000988%|
|320-overlap|4.429447×|4.355138×|-1.677599%|5/3|-0.000662%|
|eight-literals-prefix|4.756214×|4.679532×|-1.612258%|5/3|+0.000108%|
|128-biased-dynamic|3.919038×|3.883405×|-0.909222%|5/3|+0.000071%|
|32-high-entropy-stored|4.227907×|4.231434×|+0.083416%|5/3|-0.002264%|
|random-64|4.390098×|4.340521×|-1.129284%|6/2|-0.016144%|
|random-256|4.382191×|4.415186×|+0.752932%|1/7|-0.000202%|
|random-1024|4.062982×|4.043662×|-0.475516%|4/4|-0.000736%|
|repeated-1024|4.197727×|4.209217×|+0.273705%|4/4|+0.000288%|
|reymont-prefix-1024|4.946965×|4.955604×|+0.174627%|4/4|-0.001130%|

All calls retain exact reference output, independent inflate, identical logical source work, input aliases, prefixes, real capacities and final managed cleanup zero. Largest input 33,027 bytes. Natural32 worsens 0.457096%, natural16 0.364893%, repeated32 0.968629%, random32 0.354246%, distribution16 0.510686% and eight literals 0.544754%. These costs remain despite identical native entry code. No unmeasured cache/link-layout cause is asserted.

## Counters and memory

| Input | Instructions before | Instructions after | Instructions change | Cycles change |
|---|---:|---:|---:|---:|
|eight-literals|12543854.5|12543749.0|-0.000841%|+6.220448%|
|random-32769|79517271.0|79517473.5|+0.000255%|+0.639660%|
|16384-distribution-change|71845913.5|71844105.5|-0.002516%|-0.749775%|
|repeated-32769|74003088.5|74002469.5|-0.000836%|-0.364938%|
|33027-periodic-window|79763801.0|79766697.5|+0.003631%|+0.041361%|
|reymont-prefix-16384|136289863.0|136287871.5|-0.001461%|+0.249181%|
|reymont-prefix-32769|242296863.5|242249110.0|-0.019709%|-0.562409%|

Across all 20 cases, 8 instruction medians and 8 cycle medians rise. The eight-byte CPU cohort has an adverse 6.220448% cycle median despite nearly equal instructions. All contrary observations remain. Elapsed, counters, heap probes and sampling are separate cohorts, not a per-function retired-event ledger.

All 20 allocation-request, requested-byte, reallocation and before/after/peak usable-live-heap medians match. Equal usable heap is not a streaming, copied-byte or system-memory claim. Source allocation sites, real buffers and output reuse remain fixed.

| Input | CPU RSS before | CPU RSS after | Physical before | Physical after |
|---|---:|---:|---:|---:|
|eight-literals|244695040.0|311721984.0|233490112.0|233965248.0|
|reymont-prefix-32769|248152064.0|313180160.0|233490112.0|233965248.0|

Initial RSS rises about 27.39% for eight bytes, physical footprint 0.203493%. A separate four-session ABBA diagnostic retains all 40 exact full calls and memory maps: RSS change +0.017729%, physical change -0.007032%. The initial readings and repeat are both preserved. No allocator change or purge was used. No memory saving or proven initial cause is claimed.

## Exact-current profile and environment recovery

Five fresh successful exact-worker profiles cover 16,705 complete calls. Compilation-only labels regenerate a byte-identical whole current Object before current-PC mapping. Current worker/Object symbol bounds agree. No old source map is applied to changed code. Fine emitted-region weights are CPU-time samples, not retired events or an exact algorithm/overhead partition; backend merging can share provenance.

eight-literals: 10.768113% inside the main generated entry; 4.684946% unknown native attribution. Outside-entry includes other generated helpers, useful runtime algorithms, allocator and validation.
random-32769: 74.233351% inside the main generated entry; 34.911994% unknown native attribution. Outside-entry includes other generated helpers, useful runtime algorithms, allocator and validation.
16384-distribution-change: 56.077855% inside the main generated entry; 26.992288% unknown native attribution. Outside-entry includes other generated helpers, useful runtime algorithms, allocator and validation.
repeated-32769: 73.172063% inside the main generated entry; 28.592215% unknown native attribution. Outside-entry includes other generated helpers, useful runtime algorithms, allocator and validation.
reymont-prefix-32769: 82.441502% inside the main generated entry; 36.350444% unknown native attribution. Outside-entry includes other generated helpers, useful runtime algorithms, allocator and validation.

Disk exhaustion interrupted only the initial profiling/export stage and report-file creation. Primary timing, counter and scalar cohorts had completed successfully and were not repeated. The failed capture/export and driver are retained under profile-disk-exhausted and diagnostic-driver-initial.json. Removing 99 superseded compiled outputs from rounds 146–159 restored observed free space from 119 MiB to 7.6 GiB; source, receipts, reference executables, frozen R145 dependencies, baseline R177 and active R178 were preserved. After recovery, only profiling was rerun, with normal cleanup zero. The exact cleanup inventory and export failure are retained. This environmental failure is distinct from the inherited compiler bug and the repaired test-source syntax fault.

## Scalar controls

| Case | Form | Steps | After / before |
|---|---|---:|---:|
|scalar-bool|original|0|1.003481231|
|scalar-bool|decoded|0|0.998458134|
|scalar-u64|original|0|0.999785502|
|scalar-u64|decoded|0|0.996958485|
|scalar-i64|original|0|1.003222508|
|scalar-i64|decoded|0|0.999423329|
|scalar-f64|original|0|1.000171528|
|scalar-f64|decoded|0|0.999880502|
|scalar-loop|original|0|1.001066951|
|scalar-loop|decoded|0|1.001943932|
|scalar-loop|original|64|1.001247865|
|scalar-loop|decoded|64|1.004161130|

## Pins, commands and scope

Parent `369714775615e4adb85d67467b9e7409a69f92b4`; compiled production source `09c57a46a33fd7d5420a59f60b71e3abf373a3b7`; corrected test source `25a8fddfa354e2c36300476d9f36356ad219ebaa`. Production files are byte-identical across the test-only syntax correction. The initial source patch, correction patch and final nine-file tree are hash-pinned. IR/work/ABI/bytecode 198/107/39/14. Frozen R145 Core/Engine/math/dependencies remain. GPU performance library opt-level 3; unit library opt-level 1; nightly-2026-03-03/aarch64, canonical-native/program/runtime and jit/aot.

Object `25aa2d7d7ce2d0b9918bb82e158dfe508f7bf72fc10160bb91ba4b38ffad1525`; worker `69c8646281784a11d7d46fd0485d8d9207ed1e022c7a5a01f0861122f317e15f`. Entry SHA-256 `054351ca71c1175edbe74da393204c71ef4f1de18c27d8e95f04378c7d082a60`; whole text `5cf2479a4fc267672290a59fbd0ccc9bc24137bb9bb0b9963224b253b16cf624`, both equal to R177. Source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. The algorithm/tables, depth 300/nice 258, search/EOF/ties, 262144 U16 tree, 1500513 U32 cache, 305257 U64 path capacity, original allocation sites, output capacity 13 clear/reuse, aliases, pinned mimalloc and full evaluate_into boundary are unchanged. R172 source variant is separate. Logical work is not hardware instructions.

Exact commands/features/environments: parent-unit/{build,test}.json, unit/{build,test}.json, gpu-compatible-final*.json, ci-inventory.json, checks/receipts.json, qualification/*.json, review-exhaustion/after-*.json and all raw measurement/profiling specs. The expected negative parent failure is now fixed; the introduced conditional-test syntax fault is repaired; disk exhaustion is recovered. No unresolved introduced functional failure remains. The same seven inherited exclusions are recorded by exact name and clean baseline in the GPU spec.

Fresh all-feature/dependency builds, another standalone AOT harness, multi-megabyte inputs and the separate ten-workload suite are skipped. The full compressor executes through its linked Object. Ordinary hosted CI is cancelled by standing instruction, review automation preserved and #973 kept draft; no hosted-green-CI claim. The review thread is to be resolved only after publishing this fix and its evidence. Initial public replay verified all 39 original archives but exposed an inherited packaging whitelist omission: the qualified CI YAML was missing. A supplemental closure packet includes that exact source file. The original archives and failed replay are retained; no timing was rerun and no source hash was weakened.

Every timing case is sealed immediately. Public hash-indexed packets retain raw observations, positive/negative regressions, recovered and failed profiles, source/maps/machine-byte comparisons, fixture hashes, exact checks, and receipt-only replay of this report and results.json. Native binaries, bytecode, raw trace stores and private environment headers are excluded. One correctness commit in existing draft #973. The mutable-parent effect/fact brief remains unfinished. Next optimization selection should inspect hot unknown native state/guard transport and supply a real source requirement, actual machine evidence and acceptance witness before expanding analysis. Parity remains active.
