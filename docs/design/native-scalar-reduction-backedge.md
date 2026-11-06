# Nonempty scalar reduction entries — R176

Retain the compiler-only control change. Natural32KB improves **4.301808× → 4.220265× Roc** (1.895541%, all 8 balanced blocks improve), natural16KB **5.105361× → 5.011239×** (1.843595%,6/8 blocks improve). Repeated32KB improves **3.268479× → 3.199856×** (2.099534%,5/8 blocks improve); distribution16KB **7.095649× → 7.034230×** (0.865578%). Across all 20 cases, 15 medians improve and 5 worsen; geometric elapsed improves 0.588763%.

The change consumes a fact already proved at the selected child entry: start < stop <= U32MAX. A short interval enters its scalar body directly. A vector prefix branches directly to finish when its remainder is empty. Each reached scalar body performs the same cost and strict earliest-index update, advances once, then tests its backedge. No compressor source, algorithm, allocation, quota, whole-loop price, error ordering, ownership, helper, callback or runtime ABI change is introduced. The vector16-candidate threshold and arithmetic are unchanged.

## Complete-call timings

One predeclared eight-block balanced cohort contains 3,968 primary/4,048 total complete calls. All20 existing 1.05 elapsed gates pass, with exact raw reference bytes, independent inflate, current input aliases, prefixes, real capacities, identical source work and zero final managed bytes/nodes. Largest input 33,027 B. Reference ratios use contemporaneous measurements; zero-resolution reference medians stay unresolved. R175 is a rejected experiment and is not the performance baseline; both the current public parent and retained R174 use the same production implementation.

| Input | Before / Roc | After / Roc | Elapsed change | Instructions change |
|---|---:|---:|---:|---:|
|reymont-prefix-32769|4.301808×|4.220265×|-1.895541%|-1.011277%|
|reymont-prefix-16384|5.105361×|5.011239×|-1.843595%|-0.812969%|
|random-32769|5.561223×|5.565673×|+0.080022%|+0.984485%|
|16384-distribution-change|7.095649×|7.034230×|-0.865578%|+0.311775%|
|repeated-32769|3.268479×|3.199856×|-2.099534%|+0.080104%|
|33027-periodic-window|3.766727×|3.733474×|-0.882825%|+0.082586%|
|eight-literals|4.552754×|4.514971×|-0.829894%|-0.010155%|
|empty|unresolved (zero reference median)|unresolved (zero reference median)|+1.224921%|-1.294394%|
|one-stored|unresolved (zero reference median)|unresolved (zero reference median)|+1.277927%|-0.105728%|
|seven-stored-prefix|unresolved (zero reference median)|unresolved (zero reference median)|+1.158304%|-0.105885%|
|eight-overlap|4.644608×|4.617422×|-0.585311%|-0.014125%|
|320-overlap|4.499353×|4.443750×|-1.235810%|-0.007876%|
|eight-literals-prefix|4.864511×|4.869977×|+0.112371%|-0.008069%|
|128-biased-dynamic|3.990863×|3.946491×|-1.111838%|-0.018835%|
|32-high-entropy-stored|4.284045×|4.269164×|-0.347371%|-0.000157%|
|random-64|4.383535×|4.340783×|-0.975280%|+0.006887%|
|random-256|4.441198×|4.423776×|-0.392270%|+0.040315%|
|random-1024|3.966869×|3.949327×|-0.442211%|+0.135391%|
|repeated-1024|4.395310×|4.355929×|-0.895975%|-0.010472%|
|reymont-prefix-1024|4.895708×|4.840091×|-1.136047%|-0.114274%|

Adverse elapsed medians remain explicit: empty +1.224921%, one-stored +1.277927%, seven-stored-prefix +1.158304%, random32KB +0.080022% and eight-literals-prefix +0.112371%. The three stored-only cases worsen in 7/8 paired blocks. Eight literals improve 4.552754× →4.514971× (0.829894%,5/8 improving blocks). Random64 and random256 pooled medians improve despite only 3/8 improving block medians; no observation or contrary block is trimmed. Source-specific loop savings are not claimed for stored-only inputs.

## Actual instructions and proof

The continuing selected scalar visit remains **19 instructions**; the inspected pinned reference counterpart has 11. The retained change improves activation/exit handling, not that steady count. In one source-matched physical kernel copy, the short path from missing-index initialization to winner selection falls from **19N+7 to 19N+1 instructions** for 1 <= N <= 15: **six fewer per activation**. The exact old/new PCs, conditions, four-instruction dispatches, 19-instruction continuing paths and 16-instruction final visit are retained in scalar-machine-comparison.json and actual assembly. Other copies may have different register allocation. No full-compressor dynamic short-call census is collected, and six must not be multiplied by a sample count or treated as a whole-call instruction partition.

All fallible source operations remain in their previous paths. Empty/exhausted entries still decline to the original traversal, preserving start.max(stop). A reached scalar cursor is below stop, so next=cursor+1 cannot overflow under stop <= U32MAX. The existing current-window and complete-child proof settles exactly the original visits/prices before selection; the internal native loop rotation does not alter source traversal accounting. Original terminal construction, item encoding and unexpected shared-scalar fallback remain on the same continuation. No totality, budget or ownership rule is widened.

Accounted generated text **8,951,784 →8,955,736B** (+3952B); reported entry stack remains **15,792B**. Canonical plan and complete numeric recipes are exactly equal. Pre-backend phi cleanup removes four additional local parameters and four control blocks, but that is not itself a speedup claim. Code growth and all contrary observations are part of the retained tradeoff.

## Hardware counters and memory

| Input | Instructions before | Instructions after | Instructions change | Cycles change |
|---|---:|---:|---:|---:|
|eight-literals|12545842.0|12544568.0|-0.010155%|+1.433227%|
|random-32769|79230068.0|80010076.5|+0.984485%|-1.754266%|
|16384-distribution-change|71861666.0|72085713.0|+0.311775%|-1.916710%|
|repeated-32769|74289907.0|74349416.0|+0.080104%|-3.302817%|
|33027-periodic-window|80047353.0|80113461.0|+0.082586%|-1.463255%|
|reymont-prefix-16384|139132808.0|138001701.5|-0.812969%|-1.438071%|
|reymont-prefix-32769|247948908.5|245441458.0|-1.011277%|-2.106308%|

Thirteen instruction medians fall and seven rise; ten cycle medians improve and ten worsen. Random32KB has 0.984485% more instructions despite favorable cycles; repeated32KB has 0.080104% more instructions despite its elapsed/cycle gains. Eight literals have fewer instructions and an adverse 1.433227% cycle median. CPU counters, elapsed and sampling are independent cohorts; no per-function retired ledger or causal register/layout explanation is inferred from their differences.

All20 allocation-request, requested-byte, reallocation and before/after/peak usable-live-heap medians remain exactly equal. Eight literals still use 3,354 requests and 15,279,752B incremental peak usable heap. Separate RSS/physical-footprint readings are retained in raw counters and do not establish a streaming or initialized-copy-volume improvement. No source allocation site or physical buffer policy is changed.

## Fresh current-executable profiles

Five fresh profiles cover **16727** complete calls. Compilation-only fine source labels regenerate a **byte-identical whole Object** before mapping any current PC; 32 labels distinguish entry, setup, vector body/finish, scalar body/backedge/finish and terminal emission across four source copies. The labeling adds no runtime instruction or host callback. The current Object and executable symbol/text bounds also agree independently. No old source map is applied to changed code.

Natural32 has 82.633206% of Running/Timer-Fired samples inside the generated entry. Fine emitted-region shares are 3.436989% scalar body, 3.891617% scalar backedge, 2.636843% scalar finish, 2.145845% entry, 0.509183% setup, 0.381888% vector body and 0.090926% vector finish. 37.279505% remains unattributed native code. Backend hoisting/merging can share provenance; these are CPU-time samples in emitted regions, not retired instructions, an exact algorithm partition or a removable-overhead estimate. Outside-entry includes other generated helpers, useful runtime algorithms, allocator and harness validation.

The separate symbolized profile still shows host work on small input: the generated entry accounts for about 11.7% of the eight-byte sampling cohort. This compiler change removes native control work; it does not lower another host operation or establish that all remaining outside-entry time can be eliminated.

## Regression qualification

All 14 focused tests and 632 GPU tests pass, with the same 7 inherited exclusions. All 215 names selected by the existing native CI profile execute and pass; workflow selectors/count remain unchanged. Eight additional nonzero-start cases cover just below, at and above the 16-candidate threshold and one-/two-element tails. The previous lane/high-half/global-tie cases and independent scalar oracle remain unchanged. Controls cover original/decoded artifacts, direct/restored execution, zero/exhausted/current arguments, vector/scalar remainders, U32 wrap and strict ties, finite funding, fallible operations, aliases, unchanged prior outputs, immediate cleanup and recovery.

The 236-component/8-composition/1-real-capacity-path qualification is freshly rerun with a byte-identical old wrapper and unchanged expected values/allowances. All 12 balanced original/decoded Bool/U64/I64/F64 and short-loop controls pass the existing 1.05 latency and allocation gates. Their eight pure scalar medians are all 0.441192–1.096288% slower, an explicit retained limitation rather than a claim of zero scalar regression; requests and requested bytes remain equal. All 15 architecture/catalog checks, 132 workflow tests, runtime-only, formatting and warnings-as-errors pass.

Exact commands, environments and features are in unit/{build,test}.json, gpu-compatible-final*.json, ci-inventory.json, checks/receipts.json, qualification/*.json and scalar controls/receipts. Source 0996d5e builds the unit binary from a clean tracked tree. Performance uses opt-level 3, unit library opt-level 1; nightly-2026-03-03/aarch64 with canonical-native, canonical-native-program, their runtime features, jit/aot. Frozen R145 Core/Engine/math/dependencies remain; only GPU/root harnesses are rebuilt.

No native build, test, check or replay failure is introduced in this round. The same inherited exclusions remain: three batched AOT controls, two axis/legacy controls, batched JIT rejected updates and the unchanged owned-map control; exact names and baseline9017ab40d19047f39bf1832c5c85d05f00accb5d are in the GPU spec. Fresh all-feature/dependency builds, another standalone AOT harness, multi-megabyte inputs and the separate ten-workload suite are skipped. The complete compressor executes through its linked Object. Ordinary hosted CI is cancelled by standing instruction, review automation preserved, #973 kept draft; no green-hosted-CI claim.

## Pins, replay and next work

Parent `1584097e757cecf1e3eaaff7e3eb18b9fbb8c31f`; measured compiler `0996d5ef649029e2be5de49f150619a222ef26a5`. Comparison compiler `65021c3593137219762477ccde0f2d3ad7993977` (retained R174; R175 contributes retained tests/evidence only). IR/work/ABI/bytecode 196/107/39/14. The exported patch reconstructs all three compiler/test file hashes from the public parent; the final commit adds docs/index with those source hashes unchanged.

Current Object `a679d276c70d766cb50ecb742f189d3e6f4d17374cee3673ef03097d6b92a163`; primary worker `0ec3d211b85f0e71a2cb19338025b42586498a51150040db12f1238220a06b45`. Source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Preserve the level 12 algorithm, tables, depth 300/nice 258, search/EOF/tie rules, 262144-entry U16 tree, 1500513-word U32 cache, 305257 U64 path capacity, source allocation sites, output capacity 13 clear/reuse, input aliases, pinned mimalloc and complete evaluate_into timing. Logical source work is not hardware instructions. The optional R172 source variant and rejected R175 permutation are not combined.

The broader mutable-parent effect/fact project remains unfinished; this round consumes a local fact the existing selected-entry proof already owns. Next bounded investigation: precompute affine read-window address geometry outside the scalar loop to remove its node-length stack reload and repeated offsets, while keeping current views and all guards. A later coupled best/index representation could address duplicated comparison/state transport. These are separate experiments with no gain yet claimed. Large numeric-phase outlining also needs direct spill/branch evidence before implementation.

Each timing case was sealed immediately. Public hash-indexed packets retain raw elapsed/counter/heap observations, current profiles and byte-identical source map, source/fixture hashes, actual assembly, exact test/check receipts and receipt-only replay of results.json and this report. Native binaries, bytecode, raw trace stores and private environment headers are excluded. One optimization commit in existing draft #973. Parity remains active.
