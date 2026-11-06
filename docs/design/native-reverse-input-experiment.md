# Reverse low-half input experiment — R175

Reject the production change and preserve its complete evidence. Natural32KB improves **4.327658× → 4.303257× Roc** (0.563841%) and natural16KB **5.135407× → 5.099904×** (0.691339%), but the twenty-case geometric elapsed time is **+0.224967% worse**. Seven medians improve and thirteen worsen. Eight literals rise0.896254%, repeated32KB0.505027%. The small natural-text gains are retained in the record; this is a tradeoff decision based on the complete matrix, not a minimum-gain cutoff.

The production compiler is restored exactly to the reviewed R174 GPU source. Four new lane-specific high-half/tie patterns remain in the existing generic regression test. No source algorithm, allocation, limit, timing-boundary or production ABI change is retained. The rejected native permutation is exported and reconstructible from the public parent; it can be revisited independently when the surrounding loop changes.

## Complete-call results

One predeclared balanced eight-block cohort contains3,968 primary/4,048 total calls over all20 inputs, largest33,027B. Every unchanged1.05 elapsed gate passes, as do exact raw reference bytes, independent inflate, current input aliases, prefixes, capacities, source work and final managed cleanup zero. Contemporaneous reference measurements define each ratio; zero-resolution reference medians remain unresolved.

| Input | Production / Roc | Candidate / Roc | Elapsed change | Instructions change |
|---|---:|---:|---:|---:|
|reymont-prefix-32769|4.327658×|4.303257×|-0.563841%|+0.007654%|
|reymont-prefix-16384|5.135407×|5.099904×|-0.691339%|-0.033147%|
|random-32769|5.467830×|5.469571×|+0.031845%|+0.001245%|
|16384-distribution-change|7.115729×|7.113437×|-0.032211%|+0.001117%|
|repeated-32769|3.218056×|3.234309×|+0.505027%|+0.005337%|
|33027-periodic-window|3.755436×|3.758453×|+0.080318%|+0.003580%|
|eight-literals|4.624854×|4.666304×|+0.896254%|+0.017770%|
|empty|unresolved (zero reference median)|unresolved (zero reference median)|-0.214823%|+0.107106%|
|one-stored|unresolved (zero reference median)|unresolved (zero reference median)|-0.000417%|+0.092886%|
|seven-stored-prefix|unresolved (zero reference median)|unresolved (zero reference median)|-0.225613%|+0.079290%|
|eight-overlap|4.437644×|4.443777×|+0.138217%|+0.020951%|
|320-overlap|4.307740×|4.311877×|+0.096046%|+0.020010%|
|eight-literals-prefix|4.592888×|4.615948×|+0.502088%|+0.026398%|
|128-biased-dynamic|3.927340×|3.953363×|+0.662606%|+0.011753%|
|32-high-entropy-stored|4.193201×|4.224396×|+0.743930%|+0.022439%|
|random-64|4.288513×|4.330177×|+0.971534%|+0.020681%|
|random-256|4.428956×|4.417727×|-0.253532%|+0.042806%|
|random-1024|4.014959×|4.062092×|+1.173929%|+0.006211%|
|repeated-1024|4.269172×|4.269555×|+0.008981%|+0.027797%|
|reymont-prefix-1024|4.782965×|4.816238×|+0.695667%|+0.007275%|

Natural32 improves in7/8 balanced blocks and natural16 in6/8. Eight literals improve in2/8, repeated32 in2/8, periodic33 in3/8. All contrary pairs and every observation remain in the archived packets. Passing the1.05 ceiling does not make the slower aggregate an accepted optimization.

## What changed and what did not

The source-derived complete-child proof already admits the paired immutable reads. The old reverse U64 input combines low-U32 extraction and reversal in one byte shuffle. Actual ARM64 emission uses a16-byte mask load and TBL. The candidate swaps the two already loaded vectors, deinterleaves low32-bit halves, then reverses adjacent32-bit pairs: UZP1 plus REV64. The forward path, scalar path, thresholds, initialized windows, two16-byte U64-source loads, U32 arithmetic, strict earliest-index ties, prices and failure paths stay unchanged. No new source fact, ownership permission, helper, callback, unsafe boundary or runtime ABI is introduced.

Both old and candidate permutations require **two steady machine instructions**. The candidate removes16 literal bytes per copy and the fixed register-pair constraint, rather than removing an executed instruction. Four copies reduce accounted generated text **8,951,784 → 8,951,720B** (−64B); entry stack stays **15,792B**. The inspected vector-loop body remains26 instructions. Canonical plan and complete numeric recipes are exactly equal. The actual compressor disassembly confirms UZP1/REV64; an independent Cranelift Object checks65,536 value/alignment cases against a C low-half/reverse oracle, including nonzero upper halves and all16 byte alignments. Clang emits the same two vector operations for the corresponding NEON helper; this is an instruction comparison, not another compression benchmark.

## Counters and fresh profiles

| Input | Instructions production | Instructions candidate | Instructions change | Cycles change |
|---|---:|---:|---:|---:|
|eight-literals|12543393.5|12545622.5|+0.017770%|-0.091599%|
|random-32769|79227029.5|79228016.0|+0.001245%|+1.125650%|
|repeated-32769|74288955.0|74292919.5|+0.005337%|+0.281081%|
|33027-periodic-window|80050370.0|80053236.0|+0.003580%|+0.630642%|
|reymont-prefix-16384|139176500.5|139130368.0|-0.033147%|+0.282344%|
|reymont-prefix-32769|247975421.0|247994401.0|+0.007654%|-0.825623%|

Nineteen instruction medians rise and one falls; fourteen cycle medians rise. These small hardware-counter differences are separate cohorts from elapsed and sampling. Natural32 has a favorable cycle median despite a slightly higher instruction median; natural16 has a lower instruction median despite an adverse cycle median. No per-function instruction partition, causal layout attribution or cycle-derived elapsed gain is claimed.

All20 allocation-request, requested-byte, reallocation and before/after/peak usable-live-heap medians are exactly equal. RSS and physical-footprint observations are retained separately; in the CPU cohort the candidate physical footprint is81,920B lower and sampled RSS is98,304B lower. These readings do not establish a heap, streaming or copied-volume improvement.

Five fresh profiles of the exact measured candidate cover **16587** complete calls. Native symbol and text-section bounds independently agree between its Object and executable. No previous source map is applied to changed code. Natural32 has84.057444% of Running/Timer-Fired samples inside the generated entry; outside-entry includes generated helpers, useful runtime algorithms, allocator and harness validation. CPU-time samples do not partition retired instructions.

## Stronger opportunity established by the address audit

A source tag covers more than the vector operation. Refining the exact retained R174 Object PCs in one cost-kernel copy gives natural-text sample shares: **8.864469% scalar condition/routing**, **1.172161% compact scalar body**, **0.384615% compact vector body**,3.058608% entry/selection,4.157509% compact final selection and4.157509% ordinary source/parent continuation. These are disjoint inspected physical intervals; other copies and unknown native origins remain outside this classification. Function offsets require the Object entry offset512. No entire initializer subtree is labeled admission, and these CPU samples are not a promise that all classified time is removable.

The current pinned natural32 reference binary is rehashed and freshly disassembled. Its selected scalar cost/minimum visit has **11 instructions**, versus **19** on Mech’s selected compact scalar visit. The reference uses pointer progression, a counted bottom test and one comparison shared by two conditional selections. Mech still reloads node length, rebuilds offsets, repeats comparison and moves carried state through extra routing. It encodes the winning item after reduction, while the reference encodes it within each visit; the static paths therefore retain their explicit assumptions. No SIMD is claimed for this reference cost loop.

The next bounded experiment is to bottom-test the compact scalar loop, consuming its existing selected nonempty-entry fact. Short entries can reach the scalar body directly; vector prefixes finish immediately when no remainder remains. A reached scalar body performs the same cost/tie update, advances once and tests the next cursor. Current stop<=U32MAX proves that internal increment. This is a proposed native-control simplification, not a landed change or measured saving. Affine address precomputation and coupled best/index transport are separate subsequent candidates.

## Final qualification and coverage

The final restored production plus strengthened regression runs **14 focused tests** and **632 GPU tests**, with the same7 inherited exclusions. All215 CI-selected names execute and pass; the strict workflow count and selectors are unchanged. The added four patterns force each vector lane to win with distinct ignored high halves and cross-group ties, retaining the independent scalar oracle, all prior cases and all existing allowances. Original/decoded, direct/restored, short/nonzero/exhausted intervals,16-candidate boundary, remainders, wrapping, finite funding, fallible-operation ordering, aliases, prior publication, cleanup and retry controls remain covered.

All15 architecture/catalog checks,132 workflow tests, runtime-only, formatting and warnings-as-errors pass on the final source. The existing236-component/8-composition/1-real-capacity-path and12 scalar timing controls are explicitly reused from the byte-identical R174 production implementation; they are not rerun for this rejected candidate. Candidate qualification consists of the65,536 standalone emitted-permutation cases and all20 complete compression cases; it is not described as a fresh full candidate GPU suite.

Exact commands/features are in unit/{build,test}.json, gpu-compatible-final*.json, checks/receipts.json and qualification-reuse.json. Frozen R145 Core/Engine/math/dependencies remain; only GPU/root harnesses are rebuilt. Optimized performance library uses opt-level3; unit binary opt-level1; nightly-2026-03-03/aarch64 with canonical-native, canonical-native-program and runtime features, jit/aot. Fresh full-dependency/all-feature builds, another standalone AOT qualification, multi-megabyte inputs and the separate ten-workload suite are skipped. The complete compressor itself executes through the linked Object.

The inherited exclusions are the same three batched AOT controls, two axis/legacy controls, batched JIT rejected-update control and unchanged owned-map control, exactly named in the GPU spec with baseline9017ab40d19047f39bf1832c5c85d05f00accb5d. No new compiler/runtime functional failure remains. Copied GPU-run metadata initially retained the parent binary hash; the exact fresh build/source receipts correct it, both initial and corrected metadata are archived, and no command/result is replaced. A legacy archiver branch initially requested an absent reference-complete.asm while packing baseline profiles. The corrected branch packs the actual current baseline records; only its incomplete unarchived staging is removed, the first closure is retained, and a final closure overlay records the repair. Measurements and all native/test results are unchanged. The production performance regression is removed by restoration. Hosted ordinary CI is cancelled by standing instruction, review automation retained, and #973 stays draft.

## Pins and reproduction

Public parent `69b5b71c30da45fdedc67331c73cc7840dde9efc`; measured experimental compiler `e9bc10e386357c6332915743c7959316d96e9beb` (IR/work/ABI/bytecode195/107/39/14); final fresh test source `796936d3bdce585975004aeb84790f09c31bd10b`. Production remains `65021c3593137219762477ccde0f2d3ad7993977` / IR194/107/39/14, code-identical to the reviewed R174 parent. The final publication retains only test strengthening, report/status and evidence index. Exported tracked-source.patch reconstructs all three candidate files from the parent; production-restoration.json and final-source-files.json distinguish the restored source.

Candidate Object `6ca91723355d0045fa4f1eb2447e6e055f9230af4fda01bbf1a8d6ef5241cc3e`; worker `28c6081b87d1de9792f70dc1585be6c76ca4240582e3de5c7981b0b52d53f218`. Source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. The inspected reference binary is `40d2eb378f8fb3ee738be706924f8931a9171f4ba65f5dccddd53c4768c33ca4`. Preserve the original algorithm, tables, depth300/nice258, search/EOF/tie rules,262144 U16 tree,1500513 U32 cache,305257 U64 path capacity, source allocation sites, output13 clear/reuse, input aliases, pinned mimalloc and full evaluate_into timing. Logical source work is not hardware instructions. The optional R172 source variant is separate and not combined.

Each timing case was sealed immediately. Hash-indexed public packets retain the rejected patch, raw elapsed/counter/heap observations, source/fixture pins, current profiles, baseline address ledger, actual assembly, independent value witness and final test/check receipts. Receipt-only replay reconstructs results.json and this report without compiling or compressing. Native binaries, bytecode, raw trace stores and private environment headers are excluded. One experiment commit in existing draft #973; parity remains active and the wider mutable-parent fact project remains unfinished.
