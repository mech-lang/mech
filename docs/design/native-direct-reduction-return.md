# Direct reduction-return experiment (R205)

The direct-completion candidate is rejected. Production remains R199, IR/work/ABI/MBC **214/107/39/14**. The candidate is correct on the exercised original/decoded, finite-budget and recovery controls, but it does not reduce aggregate complete-call time and increases larger-input instructions. This is not a rejection of a demonstrated small win merely for its size.

## Checkpoint and mechanism

Base: `cde340c22c1ded97954839f07b70e9ea83497602`; retained production: `7748204bd12e57a20369ce7970f31f8d15c3901f`; PR base: `92f75db84e2523693648084bea16cd52ad60777a`. Candidate IR217/work107/ABI39/MBC14 is not retained. The existing draft PR #973 is the only publication target.

The real paired-cost minimum scan returns three scalar fields: minimum, selected item and final induction. The experiment records each terminal edge in the existing complete-child symbolic traversal, accepts a return only when every exit agrees on current carrier copies, constants or unchanged captures, and writes those fields before entering the current parent's continuation. Unsupported expressions preserve the ordinary terminal traversal. Exhaustion identity, totality, temporary escape, exact prepayment, earliest ties, no-strict-improvement behavior and selected-item checks remain required.

All four compiled reduction sites select the recipe. This is a static eligibility count; dynamic per-call completion counts were not instrumented. The original parent copy/actions remain. No source operation, compression rule, buffer contract or funding price changes. `candidate-source.patch` and fourteen complete changed files are in the external evidence packet.

## Whole-program results

Eight balanced matched blocks per case, with fresh pinned reference elapsed measurements. Necessary compression allocation/copying stays inside `evaluate_into`; preparation, input import/rebind, output slot allocation, validation, independent inflation and drop remain outside on both sides. The same pinned mimalloc2.1.7 is used.

All **20/20 unchanged 1.05 gates pass**. Twelve elapsed medians fall and eight rise. Geometric elapsed changes **−0.021549%**, effectively flat. Natural32 improves 0.309921% in five of eight matched blocks; natural16 improves 0.696527% in six. Repeated32 worsens 0.569635%, periodic33 1.831239%, random32 1.140345%. No unchanged favorable rerun replaces these results.

| Input | Retained / Roc | Candidate / Roc | Candidate elapsed change | Instructions change |
|---|---:|---:|---:|---:|
| reymont-prefix-32769 | 3.315320× | 3.305045× | -0.310% | +0.093% |
| random-32769 | 4.720156× | 4.773982× | +1.140% | +1.936% |
| 16384-distribution-change | 6.118726× | 6.147585× | +0.472% | +1.514% |
| repeated-32769 | 2.923119× | 2.939770× | +0.570% | +1.389% |
| 33027-periodic-window | 3.431179× | 3.494012× | +1.831% | +1.290% |
| eight-literals | 4.478517× | 4.430436× | -1.074% | -0.021% |
| reymont-prefix-16384 | 4.023109× | 3.995087× | -0.697% | +0.229% |
| empty | — | — | -0.107% | -0.156% |
| one-stored | — | — | -0.069% | -0.138% |
| seven-stored-prefix | — | — | +0.104% | -0.139% |
| eight-overlap | 4.271048× | 4.254211× | -0.394% | -0.032% |
| 320-overlap | 4.130505× | 4.084936× | -1.103% | +0.140% |
| eight-literals-prefix | 4.404038× | 4.386200× | -0.405% | -0.024% |
| 128-biased-dynamic | 3.751298× | 3.719721× | -0.842% | +0.029% |
| 32-high-entropy-stored | 4.031572× | 4.017682× | -0.345% | -0.010% |
| random-64 | 4.105957× | 4.105742× | -0.005% | +0.005% |
| random-256 | 4.256443× | 4.270548× | +0.331% | +0.105% |
| random-1024 | 3.796485× | 3.815756× | +0.508% | +0.382% |
| repeated-1024 | 4.122909× | 4.104296× | -0.451% | +0.454% |
| reymont-prefix-1024 | 4.600380× | 4.621780× | +0.465% | +0.254% |

The empty, one-byte and seven-byte reference medians round to zero in the existing clock harness; no multiple is computed. Reference elapsed is fresh for every case. A matching fresh reference instruction cohort remains open; no historical instruction count is used as a new denominator.

There are **3,968 primary observations /4,048 complete calls**, plus **1,440 separate CPU/heap calls**. All exact bytes, independent inflate, logical work, input/prepared-output aliases, retained capacities and final managed-byte/node checks pass. The maximum input remains 33,027 bytes. No multi-megabyte or long full-scale run is attempted.

## Instructions, memory and actual machine code

Seven instruction medians fall and thirteen rise. Every larger-input instruction median rises: natural32 **208,201,886→208,396,228.5** (+0.093343%), repeated32 **66,843,140→67,771,435.5** (+1.388767%), periodic33 **72,490,195→73,425,620.5** (+1.290417%), random32 **71,015,543→72,390,546.5** (+1.936201%). Ten cycle medians fall and ten rise. The adverse primary periodic timing and favorable separate periodic cycle reading are both retained; independent cohorts are not multiplied or substituted.

All twenty allocation request, requested-byte, reallocation and incremental usable-peak medians are exactly equal. Natural32 remains **6,449 requests /24,793,116 requested bytes /18,722,528 incremental usable-peak bytes**. These are not initialized-copy bytes or process RSS. The real262144-entry U16 tree,1500513-word U32 cache,305257-entry U64 capacity, original EOF rules/depth300/nice258 and eight-byte output capacity13 remain unchanged. Capacity reuse is checked; physical output backing reuse is still not established.

Generated text **8,152,436→8,153,220 bytes** (+784); reported stack **16,480→16,512** (+32). The inspected actual phase `_numeric_phase_15296` grows **4,016→4,208 bytes**, stack **368→400**, disassembled instruction sites **1,004→1,050**, static SP memory-access sites **29→47**. These static counts include cold paths/prologue and are not executed counts. The shortcut removes one cleaned CFG block but loses nine local-phi eliminations and one preheader. This is evidence that the changed join worsens emitted representation; it does not assign the full elapsed difference to a particular instruction.

Four fresh exact-worker stack profiles retain **6,677 correct complete calls**. Native-exclusive weights are natural32 **78.26%**, repeated32 **70.32%**, periodic33 **65.45%**, eight literals **8.84%**. Host-under-native weights are respectively19.52%,24.74%,30.56%,77.88%. These are disjoint timer stack weights with children subtracted, not retired-instruction shares or a removable-overhead partition. Raw samples remain local; commands/hashes, summaries and actual phase assembly are archived.

## Qualification and failures

Candidate optimized build uses nightly2026-03-03, Cranelift0.131.3, GPU/library opt3 and test-root opt1, `-D warnings`, aot/jit plus canonical-native, canonical-native-program, canonical-native-program-runtime and canonical-native-runtime. The unchanged dense frontend is used; the experimental sparse frontend is not adopted.

- **678 candidate native tests**, including17 focused reduction/prepayment controls; all **261 actual scheduled CI selections** are executed within that suite.
- **592 baseline and592 candidate same-configuration resource-policy observations**, original and decoded, are byte-identical. Each side executes1,200 native invocations, including a successful retry after every attempt. Current outputs, aliases, exact successful work/comparison reports, error text and failed-publication storage/cleanup are checked. No inherited finite-versus-unlimited report equality is assumed.
- Unchanged **236 source-derived components**, **eight native compositions** and **one native path-phase mechanism** pass; real capacities, expected values and allowances are unchanged.
- **15 architecture/catalog/warning/format contracts** and **132 workflow/inventory tests** pass. The same seven inherited exclusions below remain; no warnings-as-errors or architecture checks are weakened.

The first test-root build reports two introduced E0603 private test-helper references. The shared test-only constructor visibility is corrected; the final candidate build passes. That failure and correction remain archived. No candidate correctness or resource-policy mismatch remains. The runtime/compiler shortcut is reverted for its measured tradeoff; only independent terminal-return semantic coverage is retained. The restored source freshly passes **676 native tests /all259 actual scheduled CI selections**, plus **15 focused reduction/prepayment controls**. Its new terminal-result test checks48 native results against48 independent resident results over original/decoded artifacts and direct/restored native entry, retaining aliases and cleanup. Four restored-source format/layout/diff checks pass. The retained production source matches R199 byte-for-byte; the fresh matched control measurements above remain its current results.

The seven inherited native exclusions are:

- `batched::aot::tests::aot_rejected_updates_preserve_backing_buffers_before_dispatch`
- `batched::aot::tests::concurrent_aot_builds_publish_complete_isolated_libraries`
- `batched::aot::tests::isa_features_shared_flags_and_abi_change_cache_identity`
- `batched::axis_tests::gpu_matrix_solve_rejects_singular_inputs_without_publishing`
- `batched::axis_tests::legacy_matrix_multiply_artifacts_remain_gpu_lowerable`
- `batched::jit::tests::jit_rejected_updates_preserve_backing_buffers_before_dispatch`
- `native_artifact::program::runtime::collections::text_join::compiled_map::owned_tests::complete_unchanged_knucleotide_body_uses_the_owned_emitted_map`

Exact commands, complete feature/build inputs, artifact/binary hashes and observed counts live in the immutable packet: `*-build-spec.json`, `*-build.json`, `native-compatible.json`, `ci-coverage.json`, `contracts-{before,after}/contracts.json`, `qualification-*.json`, `architecture.json`, `primary/`, `cohorts/`, `assembly/`, `candidate-profile/`, `final-*.json` and `failure-disposition.json`. Numerical analyses use archived observations and do not execute a compiler or compressor.

## Remaining brief and next bounded step

R203/R204 complete raw compact compression remains diagnostic and unadopted. It still needs the existing packed-read/numeric specializations before its large-input result can establish a useful contract-cost floor. Separately measured accounting, precise recovery, current ownership/publication and complete external admission/import restoration are unfinished. The exclusive production host-operation-family ledger and fresh same-boundary reference instruction measurements remain open.

The next bounded production experiment is **induction-derived actual visit accounting at matching-loop exits**. It must remove the per-header visit counter as well as repeated balance arithmetic, preserve early mismatch/normal/arithmetic-error/handoff exits, and retain the checked tight-funding route. R202's balance-only candidate and R200's SIMD cutoff remain rejected; neither should be repeated. The direct-return proof is preserved externally for a future change that also solves its parent-join cost.

Parity and the full brief remain incomplete. Production remains R199/IR214. Ordinary exact-head hosted CI is cancelled under the standing instruction; review automation stays enabled and #973 stays draft.

## Immutable review packet

[Download compiler patch, commands, all twenty observations, counters and analysis](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r205-direct-reduction-return-a312c5f430ba.tar.gz) (2,418,555 bytes;403 files). SHA-256: `a312c5f430ba0f0c3df5db8cef4dae788b9832ca2476c9eb07a12f35b092cba0`. A fresh public download verifies every manifest hash, and both numerical scripts reproduce archived results byte-for-byte. No native binaries, bytecode, executable descriptors or raw profiles are uploaded.
