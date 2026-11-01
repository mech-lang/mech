# R76: keep successful debits separate from cold error transport

**Decision: retained.** The production patch changes one Engine file. The ledger, checks, public error types, source work and invocation layout remain unchanged. The accepted source before this experiment is restored R75, whose qualified runtime is R70.

| Metric | Roc | Accepted baseline | R76 | Change |
|---|---:|---:|---:|---:|
| Complete-call elapsed | 0.291000 ms | 3.6131045 ms (**12.416167×**) | 3.5820835 ms (**12.309565×**) | -0.858569% |
| Retired instructions | 1,364,372 | 40,393,656.5 (29.606043×) | 40,125,535.5 (29.409527×) | -0.663770% |
| CPU cycles | 925,380.5 | 11,370,454.5 (12.287329×) | 11,273,971.5 (12.183066×) | -0.848541% |
| Allocation requests | 95 | 6,791 (71.484211×) | 6,791 (71.484211×) | +0.000000% |
| Requested bytes | 9,021,923 | 98,112,208 (10.874866×) | 98,112,208 (10.874866×) | +0.000000% |
| Reallocations, included in requests | 0 | 162 | 162 | +0.000000% |
| Incremental allocator peak usable bytes | 9,257,328 | 20,482,072 (2.212525×) | 20,482,072 (2.212525×) | +0.000000% |

Primary timing: four independently prepared pairs, 3,968 timed calls and 3,984 total; 4/4 groups and 54/64 blocks improve. The 10 adverse blocks remain archived. All workers are ready before measurement. Blocks within a prepared pair are descriptive, not independent experiments. Different rounds use different fresh reference clocks; their ratios are not successive matched comparisons.

The separate CPU-counter cohort observes elapsed 3.5465620→3.5178330 ms (-0.810052%). It is excluded from primary timing. Hardware counters, heap counters, full stack censuses, samples and instruction windows are separately serialized. No percentages or observations from independent cohorts are multiplied.

## What changes

The real accepted ARM64 debit returns a 40-byte result. Its successful no-worker edge commits counters and then carries inactive error payload fields through mutable-borrow cleanup. The new source releases that same account borrow explicitly in each outcome. Only rejected results go to a cold non-inlined identity forwarder; the success branch has no source-level error to forward. Actual code generation still transfers inactive payload bytes; the full copy was not eliminated. No new success-path helper API, context pointer, cached permission or second debit implementation is introduced.

The caller-ceiling and checked-add precedence remain untouched. The maintained local, reservation, relative and shared-worker checks run in their original order, and commit remains after successful admission. Full error variants, phase, dimensions, required/limit fields, account identity, observer behavior, failure atomicity, unwind cleanup and retry stay on the same path. The existing account is borrowed; no owner, cap or current state is retained by the forwarder.

The actual host disassembly and standalone success-tail reconstruction show 16→10 own instructions in the selected no-worker commit/borrow-release/result-write tail. Inactive payload transfer remains but uses fewer pieces, and borrow cleanup changes. The full routine grows 158→183 own static instructions and its own frame 128→176 bytes (+48); the cold forwarder is 5 own instructions. This is a measured speed/instruction gain with an adverse host-stack/static-size tradeoff, not completion of the intended payload elimination. Exact symbols and worker hashes are archived. Own static instruction counts and changed code size are mechanism evidence, not a complete dynamic instruction ledger. The whole-call retired-instruction reduction is measured separately.

## Actual host code and generated artifact

| Host region | Before: own static instructions | R76: own static instructions |
|---|---:|---:|
| Ordinary compute debit | 158 | 183 |
| Cold error forwarder | absent/inlined/deduplicated | 5 |

Linked host text changes by +148 bytes; worker file by +128 bytes. The freshly emitted compressor Object, descriptor and metadata remain byte-identical. Generated text remains 11,800,572 bytes and entry stack 107,344 bytes. Host function frames and generated-entry stack are different measurements.

Two small Rust witnesses use the retained actual Core/GPU public type layouts. Each executes 40 ordinary/cold equivalence and unchanged-denial cases. They observe ResidentKernelError/result 40 bytes and native/iteration errors 56 bytes. Both pass. Neither ordinary synthetic routine reproduces the real accepted routine’s unconditional inactive-payload stores; that contrary limitation is preserved in prototype/FINDINGS.md. The real linked code and complete-call results determine acceptance.

## Dynamic investigation and remaining work

Four instrumented complete calls agree on these counts. Their code, atomics and timings are excluded from every performance/allocator acceptance cohort. Counts overlap and are not added as independent work.

| Selected mechanism | Calls/visits per complete compression |
|---|---:|
| Shared compute debit attempts | 39,030 |
| Admission callbacks | 12,450 |
| Helper callbacks | 8,836 |
| Private-frame projections | 3,547 |
| Scalar results from all private-frame projections | 1,361 |
| Managed results from all private-frame projections | 2,176 |
| Composite-child projections | 10 |
| Static field-count calls, including recursion | 2,020 |
| Static fields traversed for counts | 9,237 |
| Action-required classifications | 12,452 |
| Managed-move classifications | 8,311 |

Native numeric regions already emit scalar frame-word operations, and prepared library call routing already exists. Those completed mechanisms are not proposed again as missing. The new counts identify remaining callback projections and repeated immutable classification/geometry outside their existing coverage. Any follow-up must map an executed site to the maintained word/frame proof and preserve finite error prefixes before code expansion. No new speed forecast follows from counts alone.

Allocation totals and size/kind histograms remain unchanged and reconcile with both full call censuses. The existing large path reservations and extra tree copies remain ownership targets; requested capacity is not initialized/copied bytes or live memory. Post-call OS RSS/footprint includes retained compiler/catalog/image storage and is not peak RSS. Final managed bytes/nodes remain zero.

## Revisions and fidelity

- Accepted checkpoint `e1c8dc066d91e848f4cc55433dea094248cbab91`; actual baseline release source `3979d6e360ed6d5def3aea745df94d970c92dc4d`.
- Production change `03ec9fa29c8eaf95d2525e6958835572f5acb52c`.
- Actual R76 release source `d6ee13b886e50502dc413c41cf13a6259bb6db15`. The intervening witness commit changes evidence only; the frozen complete source diff identifies the one production file.
- Engine, compute, stdlib, runtime and GPU rebuild coherently with the exact retained selected-operation/release flags. Core/math/syntax and external libraries remain hash-frozen. The math library is not rebuilt.
- IR/work/ABI/bytecode 132/95/37/14. Compressor source SHA 331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9; bytecode SHA a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7.
- Complete eight-byte `abcdefgh`, level12; exact output `[75,76,74,78,73,77,75,207,0,0]`, independent inflate, output capacity 13 clear/reuse, logical work 60260219686 and comparison work 47612. Algorithms, tables, depth 300, nice 258, near-EOF rules, source allocation sites, tree 262144 U16, cache 1500513 U32, path 305257 U64, input aliases, pinned allocator and evaluate_into timing remain unchanged. No larger timing.

## Qualification and disposition

**92 required local receipt checks, status passed.** Candidate Rust unit compilation is fresh. Parent binary reuse is permitted only after the complete staged Engine tree and every linked library hash match. All parent executions are fresh; historical compilation is not relabeled.

Both sides execute Budget 108 and planner 22, plus overlapping projection 12, shape 6 and retained four debit-equivalence/error-order tests. Fresh candidate qualification runs the maintained selected GPU runtime 329, work/memory/value 32, component 236, connected 8, linked 12, scalar 12 at unchanged ceiling 1.05, frame-allocation 8, selected GPU CI 35, workflow Python 132, ten architecture checks and five type/memory checks. Inventory and execution must both be nonzero and match the existing counts. Expected values, warnings-as-errors and resource allowances are unchanged. Exact flags and commands are archived in their receipts.

The linked suite covers original/decoded validity, rejection, unchanged publication and recovery. Non-native Engine metadata compilation is fresh with warnings-as-errors. Core source and library are unchanged; Core 371 is inherited coverage, not a fresh run. The synthetic 40-case selections are separate controls, not additional unique Budget tests.

- complete source-checker mutation suites: These checker implementations and their mutation tests are unchanged from the accepted parent. Their production source checks execute; their complete mutation suites are not rerun in this bounded error-transport experiment.
- complete_unchanged_knucleotide_body_uses_the_owned_emitted_map: Existing runtime selection exclusion retained; unrelated complete knucleotide body is not requalified here.
- full remote CI: Not awaited as requested. Local selected tests with their actual retained feature configurations and architecture checks are reported separately.
- larger compression inputs/full-scale timing: Not run; this optimization measures the complete pinned eight-byte level-12 call only.

Failed setup/test attempts recorded in this round: 0. None is silently counted as successful. Full remote CI is not claimed. Publication CI cancellation follows user direction; its actual state is verified separately after push and is not passing CI. The PR remains draft pending green CI and clean review.

## Standalone reproduction

`python3 reconstruct.py /absolute/path/to/new-replay-directory` verifies every archived version and reconstructs the scorecard, complete allocation ledger, samples, instruction windows, validation and diagnostic counts without executing a compiler or compressor. `reconstruction.json` records the observed archive/file counts and ten byte-identical derived outputs. Raw observations, exact commands/flags, source diffs, complete stack censuses, bounded disassembly and SHA-256 manifests are committed with the packet.
