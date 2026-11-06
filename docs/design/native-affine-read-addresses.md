# Current affine read-address bases — R177

Retain this compiler-only address change. All 20 complete-call elapsed medians improve; geometric elapsed improves 1.255563%. Natural 32 KB improves 4.242162× → 4.177357× Roc (1.527657%); natural 16 KB 5.015910× → 4.955960× (1.195194%). Random 32 KB improves 5.428153× → 5.375515× (0.969728%); distribution-change 16 KB 7.034618× → 6.933858× (1.432342%).

The existing selected-entry proof admits current immutable read windows. For each full window, the emitter now derives base + delta*width for a forward read and base + (length-delta)*width for a reverse read. Reached reads add or subtract k*width. Vector reads retain the reverse three-lane displacement, the same two 16-byte loads for U64, the existing low-half permutation, four-candidate grouping and scalar remainder. All address arithmetic is the same I64 ring identity; no load occurs at the adjusted base itself. No additional authority or fact survives an activation.

## Measured complete calls

One predeclared eight-block balanced cohort contains 3,968 primary / 4,048 total complete calls. All existing 1.05 elapsed gates pass. Every call agrees with exact pinned reference bytes and independent inflate, with unchanged logical source work, input aliases, prefixes and real capacities. Final managed bytes/nodes are zero. Largest input: 33,027 bytes. Zero-resolution reference medians remain unresolved. The optional R172 source variant is excluded; gains from independent rounds are not compounded.

| Input | Before / Roc | After / Roc | Elapsed change | Improving/adverse blocks | Instructions change |
|---|---:|---:|---:|---:|---:|
|reymont-prefix-32769|4.242162×|4.177357×|-1.527657%|8/0|-1.289263%|
|reymont-prefix-16384|5.015910×|4.955960×|-1.195194%|7/1|-1.235343%|
|random-32769|5.428153×|5.375515×|-0.969728%|7/1|-0.616624%|
|16384-distribution-change|7.034618×|6.933858×|-1.432342%|7/1|-0.337614%|
|repeated-32769|3.211379×|3.198994×|-0.385646%|5/3|-0.466268%|
|33027-periodic-window|3.734683×|3.699262×|-0.948414%|4/4|-0.437325%|
|eight-literals|4.630497×|4.619792×|-0.231182%|5/3|-0.006576%|
|empty|unresolved (zero median)|unresolved (zero median)|-1.235813%|8/0|+0.006969%|
|one-stored|unresolved (zero median)|unresolved (zero median)|-0.911771%|8/0|-0.002916%|
|seven-stored-prefix|unresolved (zero median)|unresolved (zero median)|-0.946205%|8/0|-0.002872%|
|eight-overlap|4.459328×|4.364406×|-2.128606%|8/0|-0.007589%|
|320-overlap|4.395762×|4.335815×|-1.363752%|8/0|-0.033292%|
|eight-literals-prefix|4.512316×|4.446155×|-1.466220%|7/1|-0.007285%|
|128-biased-dynamic|3.970790×|3.873397×|-2.452745%|7/1|-0.008320%|
|32-high-entropy-stored|4.245903×|4.218319×|-0.649673%|5/3|-0.009573%|
|random-64|4.381047×|4.318621×|-1.424924%|7/1|-0.012923%|
|random-256|4.396815×|4.327160×|-1.584212%|7/1|-0.030232%|
|random-1024|4.108472×|4.051945×|-1.375856%|7/1|-0.037862%|
|repeated-1024|4.283782×|4.233974×|-1.162704%|5/3|-0.099453%|
|reymont-prefix-1024|4.992781×|4.908309×|-1.691896%|8/0|-0.239942%|

All pooled medians improve, but that does not make every paired block favorable. Periodic 33 KB has four improving and four adverse blocks; repeated 32 KB and eight literals each have five improving and three adverse blocks. No contrary observation is dropped. Eight literals improve 4.630497× → 4.619792× (0.231182%). The stored-only gains do not establish that those source paths execute this reduction or save its instructions.

## Actual machine paths

The independently source-matched scalar continuing visit falls from **19 to 16 instructions**, and its length stack reload falls from **one to zero**. Four address-setup instructions execute once at entry to this scalar path. This address segment saves three instructions per reached visit but costs one extra instruction at N=1. The inspected pinned reference scalar counterpart has 11 instructions. No full-compressor activation census or per-function retired-instruction partition is claimed.

In the same physical source copy, the four-candidate vector body falls from **26 to 25 instructions**; including its three taken-header instructions, the continuing path falls 29 → 28. The backend still reloads length and reconstructs an adjusted base on vector visits. This limits the achieved hoisting; source placement alone is not proof of a removed machine operation. Exact old/new assembly, PCs, source tags and instruction lists are retained in scalar-machine-comparison.json and the selected .asm files. Each map is independently byte-identical to its own current Object. Other copies can have different register allocation.

Accounted generated text: 8,955,736 → 8,955,224 bytes (-512). Entry stack: 15,792 → 15,792 bytes. Canonical plan and complete numeric recipes remain exactly equal. No vector threshold, arithmetic, tie, source operation or logical price changes.

## Counters and memory

| Input | Instructions before | Instructions after | Instructions change | Cycles change |
|---|---:|---:|---:|---:|
|eight-literals|12544672.5|12543847.5|-0.006576%|+0.897216%|
|random-32769|80010940.5|79517574.0|-0.616624%|+0.658430%|
|16384-distribution-change|72087852.5|71844473.5|-0.337614%|+0.372431%|
|repeated-32769|74351955.5|74005276.0|-0.466268%|+1.148197%|
|33027-periodic-window|80114844.5|79764482.0|-0.437325%|-0.135937%|
|reymont-prefix-16384|138023620.5|136318556.0|-1.235343%|-0.733384%|
|reymont-prefix-32769|245438812.0|242274461.0|-1.289263%|-0.454434%|

Across all 20 cases, 1 instruction medians and 10 cycle medians rise. Those adverse results remain in raw observations and results.json. Elapsed, counters and CPU-time sampling are separate cohorts. No sample weight is multiplied by whole-call instructions or treated as removable overhead.

All 20 allocation-request, requested-byte, reallocation and before/after/peak usable-live-heap medians remain exactly equal. Separate RSS/physical-footprint readings remain available without a streaming or copied-byte claim. Source allocation sites, table/cache/path capacities and output reuse are unchanged.

The initial CPU cohort has adverse eight-byte RSS +24.289147%, while physical footprint changes -0.042114%. The separate heap cohort also has higher absolute RSS. A fresh four-session ABBA confirmation does not reproduce the increase: RSS -0.017717% and physical footprint -0.070080%. Both initial cohorts and the confirmation remain. All 40 confirmation calls agree and clean up. Read-only memory-map snapshots are retained; the initial cause is not proved. No allocator change or purge was used, and no memory improvement is claimed.

## Exact-current profiles

Five fresh profiles cover 16,803 complete calls. Compilation-only source labels independently regenerate the byte-identical whole current Object before mapping any PC. Thirty-two fine labels separate reduction entry, scalar body/backedge/finish, vector body/finish and terminal emission in four copies. Current worker/Object text-symbol bounds agree. No old map is used on changed code.

eight-literals: 11.129207% inside the main generated entry; 5.519363% remains unknown native attribution. Outside-entry includes other generated helpers, useful runtime algorithms, allocator and validation.
random-32769: 74.307943% inside the main generated entry; 34.467161% remains unknown native attribution. Outside-entry includes other generated helpers, useful runtime algorithms, allocator and validation.
16384-distribution-change: 55.575935% inside the main generated entry; 27.274395% remains unknown native attribution. Outside-entry includes other generated helpers, useful runtime algorithms, allocator and validation.
repeated-32769: 73.379461% inside the main generated entry; 28.404953% remains unknown native attribution. Outside-entry includes other generated helpers, useful runtime algorithms, allocator and validation.
reymont-prefix-32769: 83.227331% inside the main generated entry; 37.270580% remains unknown native attribution. Outside-entry includes other generated helpers, useful runtime algorithms, allocator and validation.

Backend hoisting/merging can share source provenance. Fine emitted-region weights are CPU-time samples, not an exact algorithm partition or a retired-instruction ledger. The next analysis should inspect hot unknown/native state transport before selecting a larger structural compiler change; parity is not achieved.

## Correctness and architecture

All 14 focused and 632 GPU tests pass on the corrected final test layout, with the same seven inherited exclusions. All 215 CI-selected names execute. The independent scalar oracle is extended to all four forward/reverse U32/U64 storage-width pairs, offsets 1/7, current arrays with different lengths and values across invocations, nonzero starts, vector thresholds/remainders, original/decoded artifacts, direct/restored execution and retained input aliases. Existing zero/exhausted, U32 wrapping, earliest/global ties, malformed/current views, finite-budget/error-prefix, fallible source operations, unchanged prior publication, immediate cleanup and retry controls remain.

The new test helper initially violated the repository file/directory module contract. It was moved to a sibling test module; the initial failure is retained under initial-test-layout/checks. A receipt assertion initially expected a simpler cfg(test) attribute and was corrected to verify the actual cfg(all(test, canonical-native-program)) gate. Both are repaired introduced development failures. No expected value, existing allowance, production emitter or timed executable changed during this correction. The complete focused and GPU qualification was rebuilt/rerun afterward.

Fresh qualification passes 236 unchanged components, eight compositions and one real-capacity path. All 15 architecture/catalog checks, 132 workflow tests, runtime-only, formatting and warnings-as-errors pass. Exact commands/environments are in unit/{build,test}.json, gpu-compatible-final*.json, ci-inventory.json, checks/receipts.json and qualification/*.json. Performance GPU library opt-level 3; unit library opt-level 1; nightly-2026-03-03/aarch64, canonical-native, canonical-native-program, runtime, jit/aot. Core/Engine/math/dependencies remain frozen at R145.

All 12 balanced original/decoded Bool/U64/I64/F64 and 0/64-loop controls pass the unchanged 1.05 latency and no-allocation-increase gates. Every scalar ratio and paired observation is retained below. No iteration-dependent allocation is introduced.

| Scalar control | Form | Steps | After / before |
|---|---|---:|---:|
|scalar-bool|original|0|1.001804579|
|scalar-bool|decoded|0|0.999310059|
|scalar-u64|original|0|0.998804398|
|scalar-u64|decoded|0|1.000999714|
|scalar-i64|original|0|1.000079924|
|scalar-i64|decoded|0|1.005361439|
|scalar-f64|original|0|0.995263368|
|scalar-f64|decoded|0|1.001132004|
|scalar-loop|original|0|1.005805584|
|scalar-loop|decoded|0|1.001882885|
|scalar-loop|original|64|1.002043598|
|scalar-loop|decoded|64|1.006465757|

No unresolved introduced functional failure remains. The same inherited seven exclusions are recorded by exact name and clean baseline in the GPU spec. Fresh all-feature/dependency rebuilds, another standalone AOT harness, multi-megabyte inputs and the separate ten-workload suite are skipped. The complete compressor executes through its linked Object. Ordinary hosted CI is cancelled by standing instruction, review automation preserved, #973 kept draft. No green-hosted-CI claim.

## Open inherited correctness finding

The review scan found an unresolved exhausted-transition payload finding, comment 4489282114. A generic one-call candidate-cost program changes its exhausted transition to Done(k, 0u32, item). Both retained R176 and this R177 candidate incorrectly return (9, 1, 18), while resident execution returns (0, 1, 18), for both original and decoded artifacts. Both negative tests exit 101 after confirming the reduction is selected. The exact source, builds and failure receipts are retained under review-exhaustion. This is inherited by the address patch, not repaired by it. All 20 pinned compressor cases still match the reference. The finding blocks readiness; the review thread stays open. The next bounded correctness task is to preserve or independently prove every exhausted transition payload before bypassing that source edge, with this reproducer and retained compiler-only benchmark checks as acceptance. No further performance experiment should precede that fix.

## Pins and reconstruction

Parent `0bca6a54cdc2d69df71ca2763a6f36993448fe02`; measured compiler `379e7e140453265675df9230f4e3fedcc18290fa`; corrected test source `5276932c5eb76dadf95e1735acfdbb1d90dd9ebf`. Comparison compiler `0996d5ef649029e2be5de49f150619a222ef26a5` (retained R176). IR/work/ABI/bytecode: 197/107/39/14. The final review commit changes evidence/docs only after qualification, with production and corrected test source hashes pinned. The measured source patch reconstructs the initial four files; the separate test-layout patch and five-file qualified source tree establish the test-only correction. Production files are byte-identical across those sources, and the changed modules are excluded by cfg(test) from the timed library.

Object `760ee4db9dfdf928eb150a439f20df23d7fc120d216df8b9c21b59cbce94cc49`; worker `f4cfd32f4f70fc597a8a6b4a9ddec7a90fae39c4e3452ee7bb31f11d31380112`. Source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Preserve the level-12 algorithm/tables, depth 300, nice length 258, search/EOF/tie limits, 262144-entry U16 tree, 1500513-word U32 cache, 305257 U64 path capacity, output capacity 13 with clear/reuse, original allocation sites, aliases, pinned mimalloc and full evaluate_into timing. Logical source work is not a hardware instruction count.

Every timing case was sealed immediately. Public hash-indexed packets retain all raw elapsed/counter/heap/scalar observations, exact-current profiles/maps, actual assembly, source and fixture hashes, corrected and initial test/check receipts, and receipt-only reconstruction of this report and results.json. Native binaries, bytecode, raw trace stores and private environment headers are excluded. One optimization commit in the existing draft PR #973. The broader mutable-parent effect/fact project remains unfinished; this local address change does not claim it.
