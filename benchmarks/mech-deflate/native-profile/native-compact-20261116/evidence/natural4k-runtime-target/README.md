# Natural 4 KiB runtime target: address continuations

The selected worker remains **63,804 B, 81/81 Roc and 62/81 libdeflate in both complete orders**. These experiments have **not** supplied a 63rd qualified win. Keep them available for repair; the original Mech source, canonical bytecode and selected recipe are unchanged.

The closest remaining C loss is `prefix-natural-4096`. Its qualified paired Mech/C ratios are **1.04960× forward / 1.07280× reverse**; approximately **6.8% less Mech time** would win both orders. The next case requires about 25.6%. [Qualified ranking](../early-codebook-emission/runtime-targets/nearest.json)

## Source change

`MECH_ORDERED_VECTOR_CARRY_POINTERS=1` enables generic native lowering for the existing source-derived ordered U32 reduction. Each immutable cost-window address becomes a loop parameter and advances by its signed four-element byte stride. The same initialized-window witness, cost expression, wrapping arithmetic, lane order, strict winner comparison, scalar fallback, source errors and accounting remain in place. Addresses belong to the current invocation. There is no codec-specific input check, source rewrite or new dependency.

The implementation is live in `hosts/gpu/src/native_artifact/program/codegen/ordered_reduction/{cost,emit}.rs`. It defaults off. The disabled build reproduces the selected object `c5aab01c…`, complete metadata `c187c6f4…` and bytecode `c6529d93…` exactly. `source.patch.gz` contains the complete change against `b104d014da5da9b87ef270909507e9bfe35558f8`; archived source copies and the normal compiler receipt bind the implementation to the measured artifacts.

## Runtime measurements

These are **single-case diagnostics**, using 96 balanced paired blocks in each order. Each row compares its actual linked worker with the selected worker in the same run. The ratios against C are from those same observations; do not subtract them from the older qualified cohort to estimate a gain.

| Source-defined variant | Encoder text | Mech / selected, forward | Reverse | Mech / C, forward | Reverse |
| --- | ---: | ---: | ---: | ---: | ---: |
| Delayed winner packing, graph-rebased layout | 63,820 B | 1.00878× | 1.00650× | 1.07506× | 1.08064× |
| Carry window pointers, fresh layout | 63,820 B | **0.99969×** | **0.99293×** | 1.06269× | 1.06941× |
| Carry pointers and lane ordinals, fresh layout | 63,836 B | 0.99775× | 1.00030× | 1.06076× | 1.07506× |

For pointer-only, the unadjusted diagnostic 90% bootstrap intervals against selected are **0.99459–1.00496× forward** and **0.98862–0.99709× reverse**. Thus the forward result is unresolved and the reverse result shows a small improvement in this probe. Both still lose to C. The lane-continuation intervals overlap parity in both orders. These are not complete qualification intervals, and small slowdowns alone do not reject a candidate under the current policy. [All point estimates, intervals and boundaries](summary.json)

Whole encoder text grows by 16 B or 32 B. These are not size wins. **Matched source-through-final-link build times have not been measured for these unselected variants.** The selected recipe's latest complete build cohort remains 2.209673 s; that timing does not belong to either pointer experiment. Compiler construction logs are separate from MechDeflate build latency.

## Evidence and repair

Sampling the exact selected worker puts **3,965 of 7,390 compression samples (53.65%)** in the scorer and **1,775 (24.02%)** in the matcher. The sampler also captures validation outside compression; those stacks are excluded from this denominator. Samples identify work to inspect and are not elapsed-time forecasts.

Both initial winner-packing options rejected the old scorer's instruction-offset layout. Removing only that profile allowed matched controls and both variants to compile. Delayed packing has the same 47-block ordered control graph, so the original layout was rebased exactly. Its slower result above includes that repair. The encoded version has a different graph and remains an available unqualified alternative; its default-layout measurements and failed stale-profile attempts are included.

The pointer implementations also change the graph: 46 and 48 blocks. Fresh profiles use counter traces from the four established natural-text fixtures. Counter instrumentation changes only diagnostic clones and is never timed or shipped; shipping objects come from the compiler source. All raw counts, profile inputs, layouts and both favorable and adverse timings are preserved.

The next repair is concrete: the current ending-pointer transport makes the backend prepare the scalar-remainder address **inside each vector iteration**. Drop that exit-pointer transport and compute the remainder's address from its already-admitted invariant base. This should remove the repeated calculation; its size and runtime effects still require measurement. Keep the existing versions as controls.

## Validation and measurement setup

Each pointer worker passes **162 exact output/decompression/alias observations, six checked-failure cases and 12 successful recoveries**. Peak active native stack is **4,848 B**, below the unchanged **5,280 B** ceiling. The stack stage reused the exact previously qualified probe object after its old temporary C source went missing; the probe SHA, actual worker captures and all per-case depths are included.

All **15 optimized native reduction tests pass in both configurations**: pointer continuation with the header test, and pointer/lane continuation with the latch test. These cover original/decoded programs, both storage widths, current geometry, wrapping, ties, vector remainders, exhausted payloads and finite publication/retry. GPU/compiler/JIT code is optimized at level 3; only the unrelated full math dev dependency uses level 0. Logs and commands are in `validation/`. Both full runtime orders and preservation of every existing peer win are still required before promotion. No 81/62 claim is transferred to a changed worker by output equivalence alone.

Some old temporary harness files were missing during this work. The exact original fixture inventory and all 243 fixture-file paths were restored from repository archives. Roc and C executable hashes are unchanged. Roc's original auxiliary metadata was unavailable; a different archived metadata file supplies the same input ABI and passes all **162** exact output/decompression/alias observations before use. Fresh pointer measurements bind its new metadata hash `cc65be36…`. Previous pins, replacement pins, metadata and the complete ABI verification are included. The earlier winner-packing runs retain their original pins.

The actual test build was verified stopped during the pointer measurements and resumed afterward; process identities and states are recorded. Correctness checks and untimed counters can overlap test compilation. Timed campaigns are serial.

## Reproduction

Build the normal optimized compiler from this PR using the parent README's toolchain and profile, then run:

```sh
python3 reproduce.py --compiler /absolute/path/to/compile_profile --output /tmp/natural4k-source-replay
python3 verify.py
```

`reproduce.py` restores each profile and checks the exact object, complete metadata, canonical bytecode and whole encoder text for all nine normal variants. It does not claim runtime qualification. `as-run/` preserves the original local commands as provenance; the portable emission entry point does not require those scratch paths.
