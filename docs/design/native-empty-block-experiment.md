# Native empty-block experiment — R173

Rejected. Removing112720 empty intermediate blocks and redirecting122282 edges makes the twenty-case geometric elapsed time **+0.943248% worse**. Five medians improve and fifteen worsen. Every retired-instruction median rises. Generated text grows47336B; reported stack falls224B. All measured heap metrics are unchanged. This is a failed performance hypothesis, not a retained optimization.

The production compiler is restored byte-for-byte to the GPU sources at parent `dcc09ee9e75d02e516c9be636c0af5dbba0f7c67`: retained R171/IR192. The optional R172 frequency-reuse source variant remains separate and unchanged. Its gain is not mixed into this compiler comparison. Candidate IR193 is rejected.

## Complete-call evidence

All20 unchanged1.05 gates pass, with3968 primary/4048 total calls in one balanced, predeclared cohort. Preserved contrary blocks and all observations determine the rejection. Exact raw reference bytes, independent inflate, current input aliases, prefixes, capacities, source work and zero final managed bytes/nodes pass. Largest input33027B. The gate is a maximum regression allowance; passing it does not require retaining a slower experiment.

| Input | Before / Roc | Candidate / Roc | Elapsed change |
|---|---:|---:|---:|
|random-32769|5.430417×|5.430060×|-0.006575%|
|33027-periodic-window|4.231912×|4.226721×|-0.122670%|
|repeated-32769|3.671988×|3.670376×|-0.043909%|
|reymont-prefix-32769|4.260443×|4.308305×|+1.123399%|
|eight-literals|4.649535×|4.692437×|+0.922725%|
|empty|unresolved (zero reference median)|unresolved (zero reference median)|-0.000430%|
|one-stored|unresolved (zero reference median)|unresolved (zero reference median)|-0.138606%|
|seven-stored-prefix|unresolved (zero reference median)|unresolved (zero reference median)|+0.034583%|
|eight-overlap|4.453090×|4.514510×|+1.379283%|
|320-overlap|4.349325×|4.389870×|+0.932224%|
|eight-literals-prefix|4.608780×|4.659531×|+1.101178%|
|128-biased-dynamic|3.880137×|3.905536×|+0.654590%|
|32-high-entropy-stored|4.228365×|4.315267×|+2.055217%|
|16384-distribution-change|7.092563×|7.131646×|+0.551041%|
|random-64|4.261967×|4.344509×|+1.936700%|
|random-256|4.353743×|4.449606×|+2.201856%|
|random-1024|4.015401×|4.095366×|+1.991466%|
|repeated-1024|4.291857×|4.371103×|+1.846413%|
|reymont-prefix-1024|4.691582×|4.767412×|+1.616298%|
|reymont-prefix-16384|5.043425×|5.088432×|+0.892390%|

Reference zero-resolution observations are preserved; no invented denominator or ratio is used. Ratios use the contemporaneous reference cohort rather than historical ratios.

## Instructions, memory and native paths

| Input | Instructions before | Instructions candidate | Change | Cycles change |
|---|---:|---:|---:|---:|
|eight-literals|12540149.5|12549698.5|+0.076147%|-1.643138%|
|16384-distribution-change|72258535.0|72413346.0|+0.214246%|-0.542481%|
|random-32769|79867244.5|80147699.5|+0.351151%|-1.120708%|
|repeated-32769|80024218.5|80369631.0|+0.431635%|+0.406075%|
|reymont-prefix-32769|246676464.0|249206864.5|+1.025797%|+1.482101%|

All20 instruction medians increase; 9 cycle medians increase. Separate cycle improvements remain visible and do not overturn the primary elapsed result. All20 allocation-request, requested-byte, reallocation and incremental peak-live-heap medians are exactly equal. No streaming, copying-volume or live-memory reduction is claimed. CPU/heap cohorts are independent from elapsed and CPU-time sampling; they must not be multiplied together.

Five fresh exact-candidate executable profiles cover16247 complete calls. Raw Running/Timer-Fired samples are scoped by the actual current symbol and text section: no previous source map is applied to changed code. Natural32 has83.870968% inside the generated entry. Outside-entry includes generated helpers, useful host algorithms and harness validation; it is not a removable-host-overhead category. CPU-time samples are not retired-instruction attribution.

The original natural32 hot minimum-reduction continuation has a compare followed by conditional/unconditional jump routing. The candidate retains the same routing shape at actual PCs0x667694–0x6676cc; candidate-windows contains the sampled current disassembly. The intermediate-block reduction did not remove this hot machine path. The original and candidate unconditional-branch sample shares are8.962264% and8.801044%; this small sampling difference is not an elapsed win or an instruction partition.

Cranelift0.131.3 already performs final empty-block/branch editing. Its code also conservatively reserves worst-case veneer space for unresolved fixups. Those mechanisms motivated the experiment; no dynamic fixup-pressure or veneer census was collected, so they do not establish a specific causal explanation for the regression. Removing earlier IR blocks changed backend layout/register allocation, grew code and increased executed instructions. No further empty-block recognizers are being added.

## Experiment and correctness

The exported patch follows only parameter-free blocks containing exactly one zero-argument jump. Ordinary zero-argument Jump/Brif edges can bypass an acyclic chain. Entry blocks, argument transport, exceptional/jump-table edges, cycles, all source operations, guards, debits, effects and failure order remain intact. Memoization bounds the graph walk; current finite/error/publication contracts stay with the existing source operations. No new runtime ABI, permission cache or serialized proof is introduced.

Two focused controls cover a512-block chain, both selected effect paths, current varying arguments, untouched parameterized edges, idempotence, self cycles and paths into cycles. Candidate qualification passes631 GPU tests with the same7 inherited exclusions. All214 names selected by the existing CFG-cleanup CI filter execute in that candidate suite. The unchanged published production retains its previously qualified629/212 inventory. All15 architecture/catalog checks,132 workflow tests, runtime-only, formatting and warnings-as-errors pass. Exact commands/features and exclusions are in unit-borrow-fixed/*.json, gpu-compatible-final*.json and checks/receipts.json.

The first prototype build failed a Rust borrow check before execution; collecting the layout before mutating the dataflow graph fixed it. Both failed and corrected receipts are retained. A runtime-only check initially referenced a missing local dependency directory and emitted missing-extern diagnostics; using the identical frozen R145 artifact paths fixes that harness failure without changing feature flags or warning policy. Both check versions are retained in checks/initial-receipts.json and checks/receipts.json. No introduced functional failure remains. The introduced performance regression is removed from production, with its full source patch preserved externally.

The236-component/8-composition/1-real-capacity-path qualification and12 scalar timing controls are explicitly reused from the unchanged retained compiler; they were not rerun for this rejected candidate. Fresh all-feature/dependency builds, an extra AOT harness, multi-megabyte inputs and the separate ten-workload suite are skipped. No expected values or allowances are changed. Hosted CI remains cancelled by standing instruction, review automation stays enabled and PR973 stays draft. No green-hosted-CI claim.

## Pins, replay and next bounded target

Parent `dcc09ee9e75d02e516c9be636c0af5dbba0f7c67`; measured candidate `ee6db4cae83db9e673504a09e04fcff6c00274b5`. The measured provisional revision is reconstructible from the exported patch against the public parent; source-reconstruction.json verifies all five file hashes. The final publication contains evidence only. Production compiler `9a0fb83a20ae9ce12f4251d209c03b754dbc82da`, IR/work/ABI/MBC192/107/39/14; candidate193/107/39/14. Frozen R145 Core/Engine/math/dependencies; nightly-2026-03-03/aarch64, pinned mimalloc and complete evaluate_into timing. GPU only and the root harness are rebuilt.

Candidate Object `1da16bed9d11c916dd73c1fbc3d2de67ca6d294bead45aed092c779a4ddccb6a`; primary worker `0dc3f17490078f7af2c245cef902aca050f9b366cea1f23ece301d1dc4ea8da1`. Original source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Preserve algorithm, tables, search/EOF/tie rules, depth300/nice258, all allocation sites,262144 U16 tree,1500513 U32 cache,305257 U64 path capacity, output13 clear/reuse and input aliases. Logical source work is not hardware instructions.

The new brief’s source ledger and one selected frequency-reuse variant are completed in R172. The mutable-parent write-transfer limitation is real, but search/skip child setup still fails its requested approximately10% dynamic opportunity hurdle; those semantics remain deferred rather than expanded speculatively. The hotter DP initializer includes a compact reduction body and must not all be attributed to admission.

Next bounded compiler target: remove repeated funding machinery from already admitted pure early-exit word loops. The retained selected word path has25 machine instructions versus10 in the reference;10 of Mech’s25 implement funding/visit state. Repeated-input word extension carries about22.8% of the retained CPU-time samples; natural text is much lower. A generic witness must establish a current maximum-visit funding gate and exact actual-visit settlement, preserve overflow/error prefixes and cold fallback, then test the real same-source compressor. No speedup is yet claimed.

Every timing case was sealed immediately. The public hash-indexed packets preserve the patch, raw observations, profiles, counter logs, known fixture payloads, test receipts and receipt-only numerical replay. Native binaries, bytecode, raw trace containers and private environment metadata are excluded. One evidence commit in existing PR973; parity remains active.
