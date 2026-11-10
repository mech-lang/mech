# Private fixed-buffer nested phases with exact cold handoff

**Retain the compiler change.** Consecutive source positions now stay within a guarded native phase. The larger controlled prefixes are 22.2123% faster geometrically. Parity remains incomplete.

Git parent `d6b1ef3a9a4082ebf50e9d060ee026c53816a9d5`; PR base `92f75db84e2523693648084bea16cd52ad60777a`. Work stays on the existing #973 branch. The 825-file compiler closure is reproduced by `compiler.patch` and its source manifests: 8 modified files and 3 new files. The experimental backend is the retained R255 Cranelift 0.131.3 comparison-reuse backend; ordinary IR/work/ABI/MBC 215/107/39/14 remain unchanged.

The source and bytecode, original algorithm, tables, depth 300, nice length 258, near-EOF limits, source allocation sites, real 262,144-entry U16 tree, 1,500,513-word U32 cache and 305,257-word U64 reservation, U8 output reuse, allocator and complete-call timing boundary stay pinned. This is a compiler change; the program still performs compression. One executable accepts runtime input for all 81 configurations.

## Design and authority

The existing ordinary numeric proof still records checked child visits and its exact funding exits. Its complete-proof refusal remains `mutable children require current-account visit funding`. No ordinary selection, funding guard, debit, source price or budget is weakened. A separate sealed `PrivatePhase` contract is constructed only by the compact caller for this exact immutable proof. It admits fixed buffers, checked Tail descendants, exact buffer versions and source exports; it declines appends, growing storage, prepaid fragments and summarized children. The caller checks each writable buffer’s actual exclusive owner and fixed capacity. A missing or shared owner uses the original source CFG.

The shared checked numeric emitter executes source arithmetic, loads, stores, branches and current recursive arguments inside one synchronous phase. Before an unsupported operation, it serializes only the continuation’s exact live scalar words. Fixed current views are published from their existing bindings, with parallel reads before overlapping writes; owners are retained/released and retired through the existing ownership analysis. The original operation then executes at its original site. Subsequent entry takes current arguments and owners afresh. Failed computation preserves the compact entry’s existing output-publication and cleanup behavior.

This is independent private-buffer authority, not a reusable permission for the ordinary funded entry. The compact entry’s previously omitted extra exact managed admission/accounting/resource/recovery contract is not restored. The unit tests retain the original ordinary funding refusal and reject a copied proof.

## Observed mechanism

The collector at current header 837 now encloses 26 Tail loops. On 1 MiB it enters 35 times(random/periodic/repeated) or 36 times(text), with 31 exact window handoffs. Natural/random use continuation873; periodic uses2428; repeated uses both. These are actual call-boundary observations, not a claim of35 source iterations. Every original position/search/skip/cache operation remains in the shared source plan.

There are 43 new fixed-buffer numeric phases and 7 old matching phases folded into their parent; numeric phase count134→170. The 5 selected vector alternatives include the new collector. All 254 common numeric definitions, including unselected vector alternatives in the Object, retain identical bytes and stack metadata. The collector’s general/vector bodies are54748/42800 bytes, each with a928-byte frame,56 input words and79 output words. Nine wider-equality loops remain selected.

Root code940264→1040856 bytes; root frame50560→57088 bytes. Worker1884504→2100808 bytes(reference758128 bytes). These are real code/stack increases. Payload/heap allocations, initialized copying, zeroing and private peak medians remain identical on all 81 cases; this does not mean executable or stack memory is unchanged.

## Matched complete-call results

Two fixed 24-block cohorts use all six balanced actor orders, equal 200 ms warmup, fixed level 12, runtime input, matched live borrowed output and the same pinned clock/allocator. Full compression, scratch construction, output transfer and private cleanup are timed; validation is outside. No build, test, profile or other diagnostic compressor overlaps the primary or counter cohorts. Hardware and heap cohorts are separate. Ratios use fresh shared reference denominators; historical denominator shifts are not attributed to this patch.

| Case | Before / Roc | R256 / Roc | Balanced elapsed change | Instructions change |
|---|---:|---:|---:|---:|
| eight-literals | 0.427100× | 0.414364× | -3.4678% | -2.9654% |
| heldout-random-65537 | 2.789476× | 1.975392× | -29.1841% | -30.0327% |
| prefix-natural-32768 | 2.135596× | 1.837344× | -13.9658% | -15.7661% |
| prefix-natural-1048576 | 2.182609× | 1.908252× | -12.5701% | -14.9740% |
| prefix-random-32768 | 2.541539× | 1.828522× | -28.0545% | -28.2249% |
| prefix-random-1048576 | 3.069914× | 2.170161× | -29.3087% | -31.2393% |
| prefix-periodic-32768 | 1.439207× | 1.090658× | -24.2181% | -21.3068% |
| prefix-periodic-1048576 | 1.691711× | 1.267271× | -25.0894% | -21.0589% |
| prefix-repeated-32768 | 1.372313× | 1.048871× | -23.5691% | -21.2408% |
| prefix-repeated-1048576 | 1.583851× | 1.222946× | -22.7866% | -20.1630% |

All 81 pooled point and balanced1.05 gates pass. Across pooled balanced results78 improve and3 worsen; raw medians78 improve and3 tie. Both cohorts confirm the larger-input gains. **Primary B has 3 individual balanced1.05 misses:** empty1.064419, one-stored1.059696, eight-literals-prefix1.056674. Primary A passes all 81. Pooled changes for those three are+4.1367%,+3.4691% and−1.0045%. Empty/one-stored raw medians remain83ns before and after(reference42ns). Preserve the contrary eight-byte prefix pair:−6.4712% then+5.6674%. No gate or allowance is increased; these individual control misses remain recorded, not relabeled green.

The independent hardware cohort has78 instruction/cycle decreases and3 increases. Text 1 MiB4525382687.5→3847750557instructions(−14.9740%); random1437168915.5→988206882(−31.2393%); periodic−21.0589%; repeated−20.1630%. Small stored windows increase20/21 retired instructions; no source-level causal attribution or fresh probe-overhead subtraction is claimed. CPU-time samples are not instruction partitions.

## Memory and hardware detail

| Case | Mech / Roc instructions | Mech / Roc cycles | Mech private payload peak | Mech / Roc heap requests |
|---|---:|---:|---:|---:|
| eight-literals | 0.835089× | 0.553881× | 8.982372 MB | 230 / 96 |
| prefix-natural-32768 | 2.229829× | 1.842667× | 9.015175 MB | 606 / 277 |
| prefix-random-32768 | 2.789351× | 1.834686× | 9.008351 MB | 274 / 115 |
| prefix-natural-1048576 | 2.302984× | 1.903487× | 10.550358 MB | 1556 / 695 |
| prefix-random-1048576 | 3.086299× | 2.166079× | 10.550486 MB | 914 / 373 |
| prefix-periodic-1048576 | 1.556961× | 1.267084× | 10.550486 MB | 742 / 291 |
| prefix-repeated-1048576 | 1.569318× | 1.223890× | 10.550486 MB | 742 / 291 |

Private payload excludes prepared input and code/stack. Whole-heap request counts include reallocations and the matched transfer/cleanup window; the current observed reference count is96 on eight literals, not an asserted semantic minimum. RSS/physical/lifetime fields remain in `counter-results.json`; they are process-level observations, not memcpy volume or exact per-call live memory.

## Current exact-image profiles

Six 5-second captures per executable cover eight literals,32 KiB text/random and1 MiB text/random/periodic. Complete calls still validate output and independent inflate. Profiles include harness/validation and are neither compression-only time budgets nor retired-instruction partitions.

| Mech case | Running samples | Root | Numeric phases | Other/external |
|---|---:|---:|---:|---:|
| eight-literals | 5509 | 836 | 786 | 3887 |
| prefix-natural-1048576 | 5360 | 62 | 5250 | 48 |
| prefix-natural-32768 | 5555 | 151 | 5298 | 106 |
| prefix-periodic-1048576 | 5546 | 66 | 5316 | 164 |
| prefix-random-1048576 | 5528 | 17 | 5424 | 87 |
| prefix-random-32768 | 5540 | 116 | 5191 | 233 |

| Reference case | Running samples | Generated compression | Builtin/ownership | Other/external |
|---|---:|---:|---:|---:|
| eight-literals | 5360 | 5184 | 4 | 172 |
| prefix-natural-1048576 | 5366 | 5275 | 0 | 91 |
| prefix-natural-32768 | 5367 | 5220 | 0 | 147 |
| prefix-periodic-1048576 | 5361 | 5283 | 1 | 77 |
| prefix-random-1048576 | 5358 | 5302 | 1 | 55 |
| prefix-random-32768 | 5372 | 5303 | 3 | 66 |

Fresh actual reference assembly has direct indexed loads and counted-loop min/choice selections. The C/Rust witnesses likewise retain a native backedge with no branch-link instruction; they are small structural witnesses, not complete-compression benchmarks. Narrow actual-image assembly windows are retained. Hashed reference function names are not assigned unverified source names.

**Next concrete opportunity:** the actual `837_vector` hot window still loads current row/column words and recomputes the unit-geometry condition. The parent’s `VectorContract` already gates that property; `readonly_numeric` extends the ordinary traversal facts but subsequently uses separate captured-child version facts. Prove and propagate only the same current geometry through supported fixed-buffer descendants. Do not broaden alias/funding permission or reuse pointer-dependent facts across a cold exit. The actual path scan’s scalar min loop also has more surrounding transport than the reference’s tight counted loop; broad identity forwarding remains rejected. Neither future change is implemented here.

## Qualification, commands and limits

- `python3 run-spec.py candidate-gpu candidate-producer`: opt3, retained native feature profiles, warnings-as-errors. Final minimal source restores every timed/profiled Object and metadata byte. Default-off restores the entire R255 Object/metadata.
- `python3 run.py private-contract-tests ./candidate-tests private_fixed_nested --test-threads=1 --nocapture`:3 focused tests pass. They are included in the715 maintained total, not added again as distinct tests.
- `python3 run.py maintained-native-tests` with `maintained-test-command.json`:715 pass,0fail,0ignored,7 inherited exclusions;226.62 s.
- `python3 qualify-cold.py`:36864 original/decoded candidate/parent calls, including two real collection exits, live old buffer, borrowed fallback→native, current values, exact error origins, assertion-before-failing arithmetic/load, overflow, prior publication, cleanup and recovery.
- `python3 qualify-source.py`:3500 original/decoded source calls,700 zero trips; original values/errors/alias/publication assertions unchanged. Physical/call-boundary counters are not asserted equal when their representation intentionally changes.
- `python3 run.py all 81-immediate-smokes python3 smoke-all.py`:162 complete golden/inflate/alias/capacity/cleanup calls.
- `python3 run.py current-phase-executions python3 measure-phase-executions.py`:162 observed complete calls;4685 uncovered phase-count comparisons exact. Fused-child entry events intentionally disappear. The older eleven-category algorithmic-work census is inherited, not presented as newly rerun.
- `python3 run.py primary-a-measurement python3 measure-primary-a.py`; `python3 measure-rest.py`: two all 81 elapsed cohorts plus independent all 81 CPU/heap cohorts complete. Every original observation and contrary result is retained.
- `python3 run.py current-pc-profile python3 profile-compact-pcs.py after`; same with `reference`:12 exact-image captures. No profile overlaps a primary or hardware/heap cohort.

Full additional decoded complete-compressor images and the older extended matrix/buffer qualification are not rerun; the pinned bytecode, maintained suite and focused original/decoded source pairs supply this round’s coverage. No complete6.6 MB corpus run or optional tuning is added.

Seven inherited exclusions remain exactly:

- `batched::aot::tests::aot_rejected_updates_preserve_backing_buffers_before_dispatch`
- `batched::aot::tests::concurrent_aot_builds_publish_complete_isolated_libraries`
- `batched::aot::tests::isa_features_shared_flags_and_abi_change_cache_identity`
- `batched::axis_tests::gpu_matrix_solve_rejects_singular_inputs_without_publishing`
- `batched::axis_tests::legacy_matrix_multiply_artifacts_remain_gpu_lowerable`
- `batched::jit::tests::jit_rejected_updates_preserve_backing_buffers_before_dispatch`
- `native_artifact::program::runtime::collections::text_join::compiled_map::owned_tests::complete_unchanged_knucleotide_body_uses_the_owned_emitted_map`

Introduced setup failures are preserved: a stale working directory caused one relative producer invocation to exit127; a new Rust control had one extra closing brace; and the observer build initially lacked its copied helper source. Each orchestration stopped on failure, was repaired, and the exact failed step was rerun. No stale producer output is used. Read-only path guesses also failed during discovery and were corrected before use. No semantic failure remains. Recursive formatter-only changes to two unrelated files were restored exactly; the final rebuilt Object/metadata remain byte-identical to all timed/profiled artifacts.

CI is deliberately cancelled only for the exact pushed head under the standing instruction; no green completed CI claim is made. Keep #973 draft pending green CI and clean exact-head review. Public packet verification and the final published head are recorded externally after sealing.

## Pins and reproducibility

- bytecode: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
- benchmark_source: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
- candidate_object: `1f4b7d43df21629b96e3c955fe6a52f9a2b2f75ea81d506450b490c81699e631`.
- candidate_metadata: `11a6c889a596c95d01d00f7a7108aa4244c69b412c665fa3b9d0897e777aa772`.
- candidate_worker: `96091f758139371d0f8455d18be30c2086af70904c5604a0b11307c651648297`.
- compiler_source_manifest: `2e2334553a85f102268778a18ac9602cbdeaca0813eaaab587d6038c2042c7f6`.
- compiler_patch: `25aefc9e891f454e3575ca0386ee873aeb6b34a13da069e08859becb2517c7b1`.
- retained_backend_patch: `83c5fc6c14689ba80238615ae6e75c00103cbc447c8ae6681a62fae97607dc54`.
- reference_worker: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`.
- allocator_object: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.

`verify-source.py` verifies all 825 current source files, reverses the patch to verify all 822 base files, then reapplies it exactly. `replay.py` regenerates six reports byte-identically from the retained aggregate/observation inputs without compiling or running a compressor. Full executables, Objects, bytecode, complete compressor descriptors/IR/CFG/maps, raw traces/stacks and secrets/private environment exports are excluded.


## Verified public evidence

[Evidence archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r256-private-nested-b406c5af5f18.tar.gz),20,028,159 bytes, SHA-256 `b406c5af5f1868a6e21522e181b90b0f6468a2128aa26f302b53804faeeb96f1`. All 2,206 file hashes,825 current source files,822 reversed base files and six offline reports verify. This archive supplies the actual source patch; maintained ordinary source/IR215 is unchanged. The evidence excludes native binaries, bytecode, full descriptors/IR/CFG/maps and raw traces.
