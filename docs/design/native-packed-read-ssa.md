# R258: retain checked definitions while constructing packed-read SSA

**Archive R258; retain R256.** Archive the SSA-isolation component and keep R256 per the new brief until the exact dynamic-window geometry control is measured against R256 and full R257. Preserve the measured text/random gains, repetitive instruction increases and every adverse cohort; this is not rejection for a small gain.

Git parent `d2a06fc7577b0a6481006b110a82a7848db1757c`; retained performance baseline R256 `608ed69aedb98bb8c9d61b0bf6364580b304a6d1`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. Existing draft #973 only. Ordinary IR/work/ABI/MBC215/107/39/14 and the R255 backend remain unchanged.

## Compiler experiment

R257 mixed two changes: reuse of current-entry vector geometry and omission of the original checked source definitions during SSA construction. This experiment keeps the same current-view geometry proof, but retains newly constant checked paths while Words/SSA is built. Existing CFG cleanup removes unreachable code later. It is not a promise that fallback blocks remain in the final machine code. Old complete child packs retain their previous early replacement. A separate `elide_checked_path` emitter argument makes the distinction explicit.

The geometry proof remains confined to the current supported fixed carrier; growing arrays decline. Source-window safety is independent, and original arithmetic, selected branches, current logical length, canonical matrix translation, alias/owner gates, source funding and failure order remain. No source algorithm, table, depth300/nice258, EOF limit, source allocation site, real262144-U16 tree,1500513-U32 cache,305257-U64 path reservation, output clear/reuse, allocator or timing boundary changes. All 81 inputs run through one runtime-input executable.

The compact entry still omits the additional ordinary exact managed admission/accounting/publication/recovery contract. Tested private cleanup/error/alias behavior does not restore that contract. Ordinary R215 is unchanged.

## Measurements

The six-case screening witness was declared before execution and is kept separate from the two full81 primary cohorts. Each full cohort uses24 balanced blocks/all six actor orders, equal200ms warmup, fixed level12, runtime input, matched borrowed-output ownership and the same clock/allocator/validation placement. Full compression, scratch construction, output transfer and private cleanup remain timed. No build/test/profile/diagnostic compressor overlaps primary, CPU or heap cohorts. Hardware counters and heap use separate complete-call cohorts.

Pooled point gates 81/81; pooled balanced gates 81/81. Unchanged allowance1.05. Descriptive pooled geometric change -1.2325%; supplemental within-block geometric change -1.2245%. Do not treat pooled actor medians as paired causal effects.

| Case | Paired R256 / Roc | Paired R258 / Roc | Raw Mech median ms | Raw Roc median ms | Paired elapsed change | Instructions change |
|---|---:|---:|---:|---:|---:|---:|
| eight-literals | 0.431286× | 0.424178× | 0.119083 | 0.291208 | -1.6481% | +0.0433% |
| prefix-natural-32768 | 1.840990× | 1.812603× | 7.876167 | 4.342291 | -1.5419% | -3.2762% |
| prefix-random-32768 | 1.827033× | 1.762962× | 1.861000 | 1.054375 | -3.5069% | -4.5630% |
| prefix-periodic-32768 | 1.100891× | 1.100431× | 1.370021 | 1.249187 | -0.0418% | +0.9929% |
| prefix-repeated-32768 | 1.054136× | 1.060019× | 1.407854 | 1.334167 | +0.5580% | +1.0010% |
| prefix-natural-1048576 | 1.909417× | 1.880496× | 303.101500 | 161.066812 | -1.5147% | -3.5226% |
| prefix-random-1048576 | 2.167698× | 2.102709× | 60.423292 | 28.750708 | -2.9981% | -5.5568% |
| prefix-periodic-1048576 | 1.267487× | 1.294166× | 40.045000 | 31.280854 | +2.1049% | +1.0079% |
| prefix-repeated-1048576 | 1.225240× | 1.234041× | 42.175937 | 34.219834 | +0.7183% | +0.9977% |

Every individual miss: `[{"cohort": "primary-b", "case": "empty", "point_ratio": 1.0, "balanced_ratio": 1.0635145455807817, "point_gate": true, "balanced_gate": false}]`. All contrary observations and cohort directions remain. Historical R256/R257 keep their original reference denominators. The screening witness is not pooled into the main result.

## Machine code and memory

Exact common numeric definitions: 340; unchanged: 339. Changed: `{"_mech_compact_numeric_837_vector": {"before": {"code_bytes": 42800, "stack_bytes": 928, "sha256": "78f2daba78e8e570b6f372ec7400bf624538f09ae4d5b21d25581d7f376c14d9"}, "after": {"code_bytes": 23384, "stack_bytes": 800, "sha256": "4eb677d9eea2555c075c743d9eb1db8929bc72d1e7f07270f659f1f0d189071b"}}}`. Root: `{"code_bytes": 1040856, "stack_bytes": 57088}`. Object text: `{"before": 1385816, "after": 1366360}`; worker: `{"before": 2100808, "after": 2067784}`. Code shrink includes unexecuted fallback paths and does not establish a dynamic saving.

Independent CPU cohort: 43 instruction decreases, 35 increases; 54 cycle decreases, 27 increases. All 81 heap/payload/copy/zero/private-peak fields equal: **True**. Requested capacity, initialized bytes copied, per-call live storage and process RSS remain distinct. No input streaming or copy improvement is claimed.

## Current executable profile

| Case | Running samples | Groups |
|---|---:|---|
| eight-literals | 5320 | `{"root": 821, "other current image / harness / library": 3775, "numeric phases": 724}` |
| prefix-natural-1048576 | 5384 | `{"numeric phases": 5257, "root": 68, "other current image / harness / library": 59}` |
| prefix-natural-32768 | 5516 | `{"numeric phases": 5273, "root": 134, "other current image / harness / library": 109}` |
| prefix-periodic-1048576 | 5366 | `{"numeric phases": 5138, "other current image / harness / library": 149, "root": 79}` |
| prefix-random-1048576 | 5670 | `{"numeric phases": 5574, "other current image / harness / library": 81, "root": 15}` |
| prefix-random-32768 | 5526 | `{"numeric phases": 5135, "root": 149, "other current image / harness / library": 242}` |

Samples include harness/validation; they are not per-function retired-instruction counts or removable-time budgets. Only current-image addresses are used. Narrow actual-worker assembly and current/reference symbol weights are retained.

## Pinned reference profile

| Case | Running samples | Groups |
|---|---:|---|
| eight-literals | 5366 | `{"generated compression": 5205, "other image / harness / library": 154, "reference builtin/ownership": 7}` |
| prefix-natural-1048576 | 5394 | `{"generated compression": 5307, "other image / harness / library": 87}` |
| prefix-natural-32768 | 5374 | `{"generated compression": 5235, "other image / harness / library": 134, "reference builtin/ownership": 5}` |
| prefix-periodic-1048576 | 5368 | `{"generated compression": 5290, "other image / harness / library": 78}` |
| prefix-random-1048576 | 5385 | `{"generated compression": 5334, "other image / harness / library": 51}` |
| prefix-random-32768 | 5375 | `{"generated compression": 5301, "other image / harness / library": 74}` |

Samples include harness/validation; they are not per-function retired-instruction counts or removable-time budgets. Only current-image addresses are used. Narrow actual-worker assembly and current/reference symbol weights are retained.

## Remaining repetitive-path machine witness

The inspected R256/R257/R258 SIMD equality core has the same instruction sequence apart from register selection. On the scalar tail, R256 keeps the read base in x13; R257 reloads x13 from stack0xe8 and R258 from stack0xe0 immediately before base-plus-index. Thus SSA-stage preservation does not remove this particular reload. Narrow three-image excerpts and tail-transport-findings.json preserve the witness. This is static evidence, not its dynamic visit count or a complete explanation of the measured repetitive penalty. The exact dynamic-window control and search-versus-skip site census remain next.

## Qualification and disposition

- 715 maintained tests pass,0 fail/ignored, same7 inherited exclusions; warnings denied, opt3 GPU library/opt1 test root, nightly2026-03-03 Apple ARM64, aot/jit/canonical-native/program/runtime. Exact command and frozen dependency hashes are in build specifications.

- 268800 fresh focused linked calls cover guarded/plain width2/4/8, current row/column/matrix geometry, logical length versus backing capacity, selected/unselected/zero-trip paths, max indices, checked arithmetic/load/assertion order, aliases, prior publication, failure cleanup and retry. Original/decoded and parent/candidate results, origins and status agree with the independent oracle. The wide controls explicitly reach native vector execution while further readable packs remain. These are executed calls, not unique test definitions.

- 162 complete immediate smoke calls cover all 81 exact reference streams and independent inflate. Every primary/counter/profile call also validates output, input/prepared-output aliases and capacity; final private bytes and owners are zero.

- Full feature-disabled Object/metadata restores R256 exactly. The full final source-location Object/metadata equals the timed image. Focused source images cover original/decoded; an additional decoded full compressor Object is not emitted.

- Inherited and not newly claimed: R223/R250 extended matrix/buffer qualification, prior11-category algorithmic-work census, separate ordinary resource-contract restoration, wider dependency/all-features/registry-project builds, multi-MB timing and the separate ten-workload suite. Hosted CI is cancelled under the standing instruction; review workflows stay enabled and #973 remains draft.

- Local inspection helpers initially used incorrect source paths and stopped before any mutation; corrected explicit codegen paths are recorded. A copied relative source path then stopped width qualification; it was normalized and only that interrupted setup resumed. Initial failed receipts remain, and completed benchmark/four-byte results were preserved. The initial archive scanner matched its own prefix literal; the corrected complete-credential pattern passes without excluding the script. No archive was uploaded before that correction. No compressor failure or allowance increase resulted. Lossless compression of13 completed map files recovered1,420,468,841 bytes; retained workers, source and receipts remain.

## Next bounded experiments from the new brief

First run the requested matched R256 / geometry with its dynamic readable-window gate and original fallback structure / full R257 control. R258 isolated early checked-definition omission, which is a different question and does not complete that control. Next map the empty outer path reduction, preserving its already-decremented header cursor; close it if no meaningful machine work remains. Then run the single current-baseline R247 placement control and inspect short-reduction finish/live-state transport. Root predicate refinement is currently absent from phase_function, but extending it is only a candidate after those real hot witnesses establish applicability. No general fact campaign, global repair arguments, new permission or compression-specific selector is proposed. Existing R247 phase-only isolation and R205/R252/R254 rejected routes are not repeated. Larger-input parity is incomplete.

## All 81 observations

| Case | Bytes | Paired R256 / Roc | Paired R258 / Roc | Paired change | Pooled descriptive change | Instructions change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 0 | 2.144376× | 2.241353× | +4.5224% | +3.9527% | +0.0000% |
| one-stored | 1 | 2.060887× | 2.116773× | +2.7117% | +3.5776% | +0.0000% |
| seven-stored-prefix | 7 | 2.255525× | 2.286786× | +1.3860% | +2.1164% | +0.0000% |
| eight-literals | 8 | 0.431286× | 0.424178× | -1.6481% | -1.6803% | +0.0433% |
| eight-overlap | 8 | 0.432430× | 0.418731× | -3.1681% | -2.3917% | +0.0507% |
| 320-overlap | 320 | 0.468237× | 0.459448× | -1.8771% | -1.8127% | +0.3577% |
| eight-literals-prefix | 8 | 0.429535× | 0.423096× | -1.4990% | -0.8869% | +0.0502% |
| 128-biased-dynamic | 128 | 0.441947× | 0.424435× | -3.9625% | -3.0222% | -0.2739% |
| 32-high-entropy-stored | 32 | 0.449154× | 0.435637× | -3.0096% | -2.6545% | -0.0158% |
| 16384-distribution-change | 16384 | 1.917162× | 1.858699× | -3.0494% | -3.0519% | -3.0131% |
| 33027-periodic-window | 33027 | 1.117490× | 1.121883× | +0.3931% | +0.2079% | +0.9810% |
| random-64 | 64 | 0.455886× | 0.452017× | -0.8487% | -1.2788% | -0.1005% |
| random-256 | 256 | 0.491874× | 0.479432× | -2.5295% | -2.2471% | -0.4390% |
| random-1024 | 1024 | 0.557195× | 0.540849× | -2.9336% | -2.8668% | -1.5998% |
| repeated-1024 | 1024 | 0.533032× | 0.527405× | -1.0555% | -1.3595% | +0.4999% |
| random-32769 | 32769 | 1.813870× | 1.747146× | -3.6786% | -3.8255% | -4.5342% |
| repeated-32769 | 32769 | 1.051828× | 1.055581× | +0.3568% | +0.4843% | +0.9927% |
| reymont-prefix-1024 | 1024 | 0.694921× | 0.674911× | -2.8795% | -2.4651% | -1.8169% |
| reymont-prefix-16384 | 16384 | 1.755997× | 1.733796× | -1.2643% | -1.4903% | -2.8311% |
| reymont-prefix-32769 | 32769 | 1.838015× | 1.809927× | -1.5282% | -1.5471% | -3.2602% |
| heldout-window-32767 | 32767 | 1.837307× | 1.814858× | -1.2219% | -1.3679% | -3.2731% |
| heldout-window-32768 | 32768 | 1.838765× | 1.809368× | -1.5987% | -1.4726% | -3.2705% |
| heldout-slides-65537 | 65537 | 1.897493× | 1.866704× | -1.6226% | -1.6889% | -3.5329% |
| heldout-random-65537 | 65537 | 1.977842× | 1.912964× | -3.2802% | -3.4349% | -5.1457% |
| heldout-mixed-131073 | 131073 | 1.913995× | 1.887560× | -1.3812% | -1.3648% | -2.5643% |
| prefix-natural-1024 | 1024 | 0.698886× | 0.680666× | -2.6071% | -2.6396% | -1.8157% |
| prefix-natural-4096 | 4096 | 1.315068× | 1.290780× | -1.8468% | -1.8150% | -2.6066% |
| prefix-natural-8192 | 8192 | 1.588189× | 1.562704× | -1.6047% | -1.4209% | -2.6033% |
| prefix-natural-16384 | 16384 | 1.764085× | 1.736189× | -1.5813% | -1.5601% | -2.8311% |
| prefix-natural-32767 | 32767 | 1.837630× | 1.809590× | -1.5259% | -1.5778% | -3.2735% |
| prefix-natural-32768 | 32768 | 1.840990× | 1.812603× | -1.5419% | -1.4768% | -3.2762% |
| prefix-natural-32769 | 32769 | 1.840881× | 1.810124× | -1.6708% | -1.5679% | -3.2594% |
| prefix-natural-33027 | 33027 | 1.839937× | 1.811631× | -1.5384% | -1.4955% | -3.2675% |
| prefix-natural-65535 | 65535 | 1.906815× | 1.880087× | -1.4017% | -1.4218% | -3.0352% |
| prefix-natural-65536 | 65536 | 1.907067× | 1.881024× | -1.3656% | -1.3335% | -3.0339% |
| prefix-natural-65537 | 65537 | 1.910351× | 1.886090× | -1.2699% | -1.4090% | -3.0343% |
| prefix-natural-131072 | 131072 | 1.901644× | 1.878458× | -1.2192% | -1.2853% | -3.0828% |
| prefix-natural-262144 | 262144 | 1.908713× | 1.884679× | -1.2592% | -1.3009% | -3.1058% |
| prefix-natural-1048576 | 1048576 | 1.909417× | 1.880496× | -1.5147% | -1.4905% | -3.5226% |
| prefix-random-1024 | 1024 | 0.558899× | 0.540177× | -3.3499% | -3.0473% | -1.6059% |
| prefix-random-4096 | 4096 | 0.957356× | 0.933474× | -2.4945% | -2.4152% | -2.9550% |
| prefix-random-8192 | 8192 | 1.212853× | 1.168427× | -3.6629% | -3.8288% | -3.6434% |
| prefix-random-16384 | 16384 | 1.554140× | 1.500638× | -3.4425% | -3.8395% | -4.0058% |
| prefix-random-32767 | 32767 | 1.819407× | 1.759537× | -3.2906% | -3.4306% | -4.5635% |
| prefix-random-32768 | 32768 | 1.827033× | 1.762962× | -3.5069% | -3.5637% | -4.5630% |
| prefix-random-32769 | 32769 | 1.806557× | 1.746879× | -3.3034% | -3.1364% | -4.5286% |
| prefix-random-33027 | 33027 | 1.820614× | 1.754667× | -3.6222% | -3.9110% | -4.5347% |
| prefix-random-65535 | 65535 | 1.998761× | 1.934400× | -3.2200% | -3.3059% | -4.9779% |
| prefix-random-65536 | 65536 | 2.000420× | 1.928215× | -3.6095% | -3.5652% | -4.9779% |
| prefix-random-65537 | 65537 | 1.990614× | 1.920380× | -3.5283% | -3.5330% | -4.9590% |
| prefix-random-131072 | 131072 | 2.081694× | 2.015862× | -3.1624% | -3.1540% | -5.1582% |
| prefix-random-262144 | 262144 | 2.187060× | 2.119610× | -3.0840% | -3.0198% | -5.2497% |
| prefix-random-1048576 | 1048576 | 2.167698× | 2.102709× | -2.9981% | -3.1701% | -5.5568% |
| prefix-periodic-1024 | 1024 | 0.542944× | 0.536819× | -1.1281% | -1.0988% | +0.4076% |
| prefix-periodic-4096 | 4096 | 0.751350× | 0.744152× | -0.9580% | -0.7004% | +0.7133% |
| prefix-periodic-8192 | 8192 | 0.887487× | 0.877530× | -1.1219% | +0.2016% | +0.8073% |
| prefix-periodic-16384 | 16384 | 0.975433× | 0.974809× | -0.0639% | -0.6577% | +0.9552% |
| prefix-periodic-32767 | 32767 | 1.094476× | 1.104366× | +0.9036% | +0.8143% | +0.9931% |
| prefix-periodic-32768 | 32768 | 1.100891× | 1.100431× | -0.0418% | +0.1737% | +0.9929% |
| prefix-periodic-32769 | 32769 | 1.101625× | 1.097282× | -0.3943% | -0.8811% | +0.9847% |
| prefix-periodic-33027 | 33027 | 1.095348× | 1.095559× | +0.0193% | +0.1218% | +0.9805% |
| prefix-periodic-65535 | 65535 | 1.173077× | 1.178337× | +0.4485% | +0.1512% | +1.0066% |
| prefix-periodic-65536 | 65536 | 1.172190× | 1.175459× | +0.2789% | +0.3529% | +1.0066% |
| prefix-periodic-65537 | 65537 | 1.169380× | 1.175339× | +0.5096% | +0.5843% | +1.0023% |
| prefix-periodic-131072 | 131072 | 1.216661× | 1.222509× | +0.4807% | +0.6555% | +1.0137% |
| prefix-periodic-262144 | 262144 | 1.241065× | 1.248862× | +0.6283% | +0.5473% | +1.0176% |
| prefix-periodic-1048576 | 1048576 | 1.267487× | 1.294166× | +2.1049% | +0.8796% | +1.0079% |
| prefix-repeated-1024 | 1024 | 0.538971× | 0.527723× | -2.0870% | -2.0932% | +0.5037% |
| prefix-repeated-4096 | 4096 | 0.738026× | 0.737942× | -0.0114% | -1.4602% | +0.7554% |
| prefix-repeated-8192 | 8192 | 0.860430× | 0.863419× | +0.3474% | +0.4847% | +0.8314% |
| prefix-repeated-16384 | 16384 | 0.955212× | 0.956072× | +0.0900% | -0.1359% | +0.9703% |
| prefix-repeated-32767 | 32767 | 1.057194× | 1.062135× | +0.4674% | +0.1439% | +1.0009% |
| prefix-repeated-32768 | 32768 | 1.054136× | 1.060019× | +0.5580% | +0.0933% | +1.0010% |
| prefix-repeated-32769 | 32769 | 1.057997× | 1.060103× | +0.1991% | +0.3078% | +0.9928% |
| prefix-repeated-33027 | 33027 | 1.074167× | 1.076923× | +0.2566% | +0.1954% | +0.9689% |
| prefix-repeated-65535 | 65535 | 1.129600× | 1.135591× | +0.5304% | +0.2565% | +1.0004% |
| prefix-repeated-65536 | 65536 | 1.134282× | 1.138779× | +0.3965% | +0.3807% | +1.0004% |
| prefix-repeated-65537 | 65537 | 1.133514× | 1.138188× | +0.4123% | +0.3616% | +0.9961% |
| prefix-repeated-131072 | 131072 | 1.178138× | 1.185634× | +0.6362% | +0.6794% | +0.9998% |
| prefix-repeated-262144 | 262144 | 1.230138× | 1.238729× | +0.6983% | +0.4492% | +1.0030% |
| prefix-repeated-1048576 | 1048576 | 1.225240× | 1.234041× | +0.7183% | +0.7689% | +0.9977% |

## Reproduction

The archive retains the full826-file compiler closure, exact reversible patch to825-file R256, commands, feature profiles, source/bytecode/backend/allocator pins, independent observations, compact current/reference profiles and narrow assembly. Offline scripts reproduce eight reports without executing the compressor. Executables, Objects/libraries, bytecode, complete compressor metadata/IR/maps and raw traces/stacks are excluded from public evidence. Source programs and focused control metadata remain.

[Verified evidence archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r258-packed-read-ssa-e260b0057577.tar.gz) — SHA-256 `e260b0057577b7365947e47838d2f05f3de0aa9c0b4ae5140801db902e13ae31`, 11,322,499 bytes, 2,512 hash-listed files. Fresh public verification checks every file, exact826-file source reconstruction/reversal to825-file R256 and eight numerical report replays. The archive contains compressed diagnostic observations and their original content hashes. Native binaries, bytecode, full compressor metadata/IR/maps and raw traces remain excluded.

## Family counter changes and remaining parity budget

These are the archived R258 candidate measurements, with R256 still retained. Elapsed effects and reference ratios are computed within balanced blocks. Instructions/cycles come from the separate CPU cohort. Required time reduction is `1 - 1 / paired_reference_ratio`, floored at zero; it is arithmetic on elapsed ratios, not a hardware-counter attribution.

| Input | Paired R258 / Roc | Elapsed change vs R256 | Instructions change | Cycles change | Further elapsed reduction for1x |
|---|---:|---:|---:|---:|---:|
| eight-literals | 0.424178× | -1.6481% | +0.0433% | -3.9000% | 0.00% |
| prefix-natural-1048576 | 1.880496× | -1.5147% | -3.5226% | -1.2771% | 46.82% |
| prefix-random-1048576 | 2.102709× | -2.9981% | -5.5568% | -3.1171% | 52.44% |
| prefix-periodic-1048576 | 1.294166× | +2.1049% | +1.0079% | +0.7860% | 22.73% |
| prefix-repeated-1048576 | 1.234041× | +0.7183% | +0.9977% | +0.7331% | 18.97% |
