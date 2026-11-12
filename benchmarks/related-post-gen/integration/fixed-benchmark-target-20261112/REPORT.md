# Fixed benchmark comparison

Managed and compact Mech beat the fastest locally verified unchanged example of `zupat/related_post_gen` at **fc2982b0c929b1b701209d401a1dc30c773ef4ab**. The reference is the repository's `rust_con`, retained under the timer-descriptive name `rust-preinitialized`. It uses eight preinitialized Rayon threads. Mech creates and joins four native workers inside its timer.

| Implementation | Primary (31 rounds) | Independent (13 rounds) | Confirmation / example |
| --- | ---: | ---: | ---: |
| Unchanged repository `rust_con` | 1.870584 ms | 1.866250 ms | 1.000000× |
| Managed Mech IR355 guarded U16 | **1.433000 ms** | **1.400292 ms** | **0.750324×** |
| Compact Mech IR353 guarded U16 | **1.224125 ms** | **1.174375 ms** | **0.629270×** |

Both measured Mech candidates win all 31 primary and all 13 confirmation pairs. The comparison preserves Rust's original pool-before-timer advantage: Rust's timer is not relabeled as equivalent continuous post-JSON. Mech's larger interval includes admission, intervening setup, allocations, worker creation/run/join, native cleanup and result materialization. JSON IO/restoration and whole-process metrics remain separately visible; whole-process time is not an acceptance gate.

The expanded existing-example sweep established `rust_con` as fastest among locally verified unchanged examples. Unbuilt suite entries, including Julia, are not claimed measured. Current reference measurements come from the same new cohorts as Mech, rather than reusing historical timings.

## Source provenance and the corrected target

[The provenance audit](provenance/audit.json) pins the original source Git blobs and exact retained binaries. [Classification](provenance/classification.json) distinguishes four unchanged-source builds from twenty local Rust source experiments. The historical `kind=upstream` field mixed these categories; raw reports are preserved without edits, and their eligibility rules remain intact. The added custom controls are extra diagnostic data and do not raise the fixed completion target. No further Rust optimization is part of this goal.

[The fixed comparison](fixed-benchmark-comparison.json) retains all pairs, bootstrap method/seeds, exact report hashes, ratios and scope. [Compressed complete primary and independent reports](provenance/reports/) contain all 1,612 and 676 observations, including timings, exact outputs and original orders. The separate resource cohort has 371 exact outputs; selected resource medians are retained in [resource-summary-selected.json](resource-summary-selected.json). No samples or adverse observations were removed. The [independent root provenance check](root-fixed-benchmark-provenance-verification.json) verifies all 33 provenance members, all 24 source/binary pins and all four unchanged-source Git blobs.

## Mech changes and validation

The production compiler is IR355 / work127 / ABI42 / image11 at **c18d910c9dfd56d988a7c4e75a94b90ef8fb2b1a**. It retains Cranelift and lowers the numeric work and workers into native code. The changes include exact checked increment proofs, lazy successful Map diagnostic paths, genuine checked U16 arithmetic, and exact histogram key copying. There is no host scoring bridge or mandatory LLVM dependency.

The [measured source](../../mech/related_guarded_u16.mec) is byte-identical to the qualified candidate (SHA256 `6a3022a0385efd1bf3dcdc9a76409607d20237260b21bf6745416d84da3d027a`). Vocabulary ≤ 35 and posts ≤ 65535 prove its narrow storage; general scoring fallback remains for other inputs. Source-specific work bills are retained, not borrowed from wider sources.

Managed IR355 U16 improves on selected IR334 in both cohorts: paired −1.3908% [−2.8526%,−0.1296%] primary and −1.8318% [−3.3268%,−1.1355%] independent. The difference between cohort medians is 16.250 µs and 27.916 µs, respectively. Its separate resource medians are 27,219,043 all-thread instructions, 8,644,268 cycles, 620 allocation requests, 4,592,577 requested bytes and 2,593,193 additional peak requested heap bytes. IR334 uses 725 requests, 6,376,193 bytes and 4,211,067 additional peak bytes in the same resource cohort.

The compact IR353 U16 source gain versus its own IR353 U32 control is independently confirmed. Its aggregate comparison against prior compact IR349 crosses zero in confirmation; no stronger compiler-aggregate claim is made. Mask-reuse source attempts and combinations failed to improve the selected result and remain retained.

Exact355 compiler validation has 79 focused GPU passes; broad coverage has 1,108 passes and the same 16 inherited failures, 11 ignored and 35 filtered. Core lazy Map paths passed 403 tests. Arithmetic/index checks, finite limits, admission/error order, alias/ownership boundaries, unchanged publication and retry contracts remain checked. IR356 geometry work is frozen unbuilt/unselected research and is not included in production or these results.

Native code and complete-process costs remain higher. Managed IR355 U16 has 1,117,996 native code bytes and a 1,696-byte reported root frame, versus IR334's 1,021,740 bytes and 1,568-byte frame. Ordinary executable size rises from 17,095,936 to 18,164,960 bytes; primary whole-process peak RSS rises from 27,017,216 to 30,015,488 bytes. These are distinct from the requested-heap measurements above.

| Independent confirmation, launch through exit | Time |
| --- | ---: |
| Original repository `rust_con` | 17.980292 ms |
| Managed IR334 | 40.379292 ms |
| Managed IR355 U16 | 48.851250 ms |
| Compact IR353 U16 | 19.878875 ms |

The whole-process comparison includes JSON IO and restoration and remains visible without becoming an acceptance gate. Compact native Object and managed module/root-frame counters have different scopes; no stack equivalence is inferred.

The [complete IR350–355 compiler, source, qualification and failure archive](../u16-host-source-20261112/REPORT.md) retains all 5,063 observations across six cohorts and 3,040 separately scoped qualification outputs, plus original failures and offline reconstruction checks. This fixed-target packet was published first at `3df2faffda6e31a7cc15af53f5f22ced184314e9`; that commit preserves its initial publication state.
