# R77: precise private failure return — rejected

**Decision:** restore the accepted R76 implementation. Returning an error directly from its cold identity helper changes the Rust type but provides no further measured instruction, size or stack benefit. The targeted linked routines have identical machine bytes.

| Metric | Roc | Accepted R76 | R77 candidate | Change |
|---|---:|---:|---:|---:|
| Complete-call elapsed | 0.285000 ms | 3.6157290 ms (**12.686768×**) | 3.6191250 ms (**12.698684×**) | +0.093923% |
| Retired instructions, separate cohort | 1,364,372.0 | 40,122,933.5 | 40,124,964.0 | +0.005061% |
| CPU cycles, separate cohort | 921,649.5 | 11,336,724.5 | 11,228,063.5 | -0.958487% |
| Allocation requests, separate cohort | 95.0 | 6,791.0 | 6,791.0 | +0.000000% |
| Requested bytes, separate cohort | 9,021,923.0 | 98,112,208.0 | 98,112,208.0 | +0.000000% |
| Incremental allocator peak usable bytes, separate cohort | 9,257,328.0 | 20,482,072.0 | 20,482,072.0 | +0.000000% |

Four independently prepared pairs, 3,968 timed calls and 3,984 total. One of four groups improves, while 36/64 blocks improve. The favorable median/geometric block ratios (0.997723877/0.999570763) and the slower absolute second group are preserved. Blocks within a prepared pair are descriptive, not independent experiments. This does not establish another primary speed gain.

The separate CPU-counter cohort favors the candidate on cycles and elapsed: 3.5371250→3.5030000 ms (-0.964767%). That contrary cohort remains visible and is excluded from the primary result. No independent cohort percentages are multiplied. Historical reference clocks differ; the fresh R77 baseline ratio must not be treated as a regression from R76’s earlier reference ratio.

## Exact source and code result

The experiment changes three lines in one Engine file: the private cold helper returns ResidentKernelError instead of Result<(), ResidentKernelError>, and its caller explicitly wraps Err. Public signatures, diagnostics, checks, ledger, allocation behavior and source semantics are untouched.

Both selected actual ARM64 routines are byte-identical between accepted/candidate: the compute debit remains183 own static instructions with a176-byte frame; its selected successful tail remains10 instructions. The cold forwarder remains5 instructions. Inactive payload transfer on success remains. Linked text and worker file sizes are unchanged. The whole-text digests differ, so no full-executable or whole-text byte-identity claim is made. Bounded original routine bytes, their hashes and independent disassembly are archived; success-tail.py rehashes those slices and verifies their exact equality during replay.

Thus narrowing this private helper alone does not remove the remaining public result transport. The R76 gain remains retained; no R77 gain is discarded on the basis of a size threshold. There is no demonstrated additional gain to retain.

Production experiment `5dd8209bdb63a4cf8155da75049106de53078519`; actual candidate compiler `3edbcc985a5049b5214f25ef9dd90a5fdf134377`. Accepted checkpoint `0f8b961afbcde8d17b9bfde7018b60f855225e6e`, actual baseline compiler `d6ee13b886e50502dc413c41cf13a6259bb6db15`. Exact restoration `024a5ea932905010e27535cafcef4674c6a56866` matches all accepted Engine/GPU/CI source. Experimental source and every timing remain committed. Fresh emitted compressor Object, descriptor and metadata stay byte-identical; IR/work/ABI/bytecode132/95/37/14.

## Qualification and limitations

**48 bounded rejected-package checks pass.** Both sides execute Budget108 and planner22, plus overlapping projection12, shape6 and four retained debit-equivalence/error-prefix tests. Candidate compilation is fresh. Cached parent Rust binary reuse requires a complete staged-source and linked-library match; all executions are fresh. Two separate40-case synthetic controls cover all public error variants and unchanged rejection state. Actual non-native Engine metadata compilation passes warnings-as-errors. Four instrumented full compression calls agree on every mechanism count.

Post-restoration Budget108/planner22, memory-runtime, formatting and whitespace checks pass. This candidate does not receive a full new GPU/component/scalar qualification after the null code result; the exact restored R76 source retains its completed92-check qualification. No historical run is relabeled as R77.

- full GPU runtime329, work/value32, components236, connected8, linked12, scalar1.05, frame8, CI35/Python132 and architecture/type selections: Not launched after the main timing/counter/code audit establishes no further gain. Accepted R76 already has full92-check local qualification; no historical execution is relabeled as R77 coverage.
- fresh full Core suite: Core source/library frozen. Prior371-case Core coverage inherited.
- remote CI and larger/full-scale inputs: Not awaited/run as requested. The full pinned eight-byte computation only.

One introduced analysis-only failure is retained: after both Mach-O byte extractions succeeded, a Path/name concatenation typo stopped the proof reader. The exact bad reader and a labeled reproduction are archived; parentheses were fixed and only the deterministic reader rerun on captured bytes. No compiler or timing was repeated. No unresolved compiler/test failure is introduced.

The complete pinned eight-byte `abcdefgh`, level12 call preserves source/bytecode pins, algorithm/tables, depth300/nice258/near-EOF rules, source allocation sites, real tree262144U16/cache1500513U32/path305257U64 capacities, input aliases, pinned allocator and evaluate_into timer. Exact output `[75,76,74,78,73,77,75,207,0,0]`, independent inflate, output capacity13 clear/reuse, logical work60260219686, comparison work47612 and final managed bytes/nodes0 remain verified. No larger timing or allowance increase. Requested capacity, initialized/copied bytes, allocator peak and post-call OS memory remain separate metrics.

## Evidence for the next bounded target

The retained metadata already selects all239 eligible numeric leaves. Its81 rejected candidates comprise76 non-leaf cases and five array-provenance cases; none is rejected for a flat-carrier requirement. Typed unsigned matrix construction already avoids payload scans. Those existing optimizations are not proposed again as missing.

The next investigation is the separate admission plus no-word helper sequence:1,341 managed Move and2,806 StructuralObservation helpers per call. At most4,147 native callback crossings are candidates for combining the existing admission and existing helper in their exact order. This is a candidate count, not a speed forecast or an implemented feature. Admission results used to choose another path must remain separate. The exact visit/scalar-admission-word/result-pointer/error order, spent prefix, aliases, publication and retry need acceptance controls. Any private program tag extension requires the appropriate ABI revision; frozen scalar ABI1 remains separate. Details and source pins: remaining-work-evidence.json.

## Reproduction

`python3 reconstruct.py /absolute/path/to/new-replay-directory` verifies all archived versions and reconstructs ten byte-identical numerical/proof outputs without a compiler, compressor or external scratch input. The selected routine bytes are part of the archive; private binaries are not needed. reconstruction.json records actual archive/file counts. Raw favorable and adverse observations, exact commands/features, full censuses, code bytes, disassembly and restoration proofs remain available. Hosted CI is not counted as passing; publication cancellation is user-directed and verified separately. The PR remains draft pending green CI and clean review.
