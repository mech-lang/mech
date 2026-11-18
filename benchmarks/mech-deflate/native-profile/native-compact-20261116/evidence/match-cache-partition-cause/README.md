# Match-cache phase partitioning

## Static result

The table's 7,260-byte pair is `bt-search-loop`, a child of the 22,644-byte `optimal-block-loop` phase emitted by the control. The table build leaves `optimal-block-loop` in the checked root. It outlines `bt-search-loop` and four frequency and observation merge loops that the larger phase had included. Arithmetic retains the outer phase and increases that pair to 23,232 body bytes.

The inventory below uses Mach-O symbol extents, including alignment. Every listed numeric body has a direct branch relocation from the root and is present in the corresponding linked worker. Identical vector copies for the four merge loops and the three scorer families appear in cleanup metadata and have no object symbol or linked call.

| Encoder text contribution | Generic control | Table slot | CLZ slot |
| --- | ---: | ---: | ---: |
| Checked root | 56,816 B | 57,424 B | 56,896 B |
| Outer collector, general plus vector | 22,656 B | 0 B | 23,248 B |
| Separately outlined collector descendants | 0 B | 9,088 B | 0 B |
| Packed scorer family | 4,112 B | 2,752 B | 2,752 B |
| Other outlined numeric | 6,672 B | 6,672 B | 6,672 B |
| Remaining text | 1,300 B | 1,300 B | 1,300 B |
| **All text** | **91,556 B** | **77,236 B** | **90,868 B** |
| Read-only constants | 80,040 B | 14,504 B | 14,504 B |
| Object file | 179,152 B | 99,336 B | 112,760 B |

For the table, −22,656 B from the parent, +9,088 B in descendants, −1,360 B in the scorer, and +608 B in the root reconcile to −14,320 B overall. The 616 B increase quoted for the root excludes alignment. For arithmetic, +592 B in the collector, −1,360 B in the scorer, and +80 B in the root yield −688 B. All three encode the same generic-option study. Its exact 71,768 B fixed reference has seeded placement and omits the collector's general version using an explicit checked refusal; the table study remains 5,468 B larger than that reference. [Text and branch relocations](text-census.json)

## Homologous regions and admission

Reading the canonical bytecode's schema, constants, and control graph makes the phase alignment independent of changing native site numbers. The three outer collectors originate at wire block 33, operation 7. Their 22-field seed tuple at operation 6 contains the tree, the match cache, the input, and four frequency and observation matrices. The three `bt-search-loop` seeds have the same 18-field shape and three matrices. The two ten-bin observation merges and two 259-bin frequency merges retain matching source operation roles and enclosing branch positions across the three artifacts.

| Source role | Generic control | Table slot | CLZ slot |
| --- | --- | --- | --- |
| `optimal-block-loop` | 475, outlined | 475, checked root | 475, outlined |
| `bt-search-loop` | 914 inside 475 | 940 outlined | 952 inside 475 |
| Checkpoint observation merge | 2315 inside 475 | 2367 outlined | 2391 inside 475 |
| Checkpoint frequency merge | 2389 inside 475 | 2441 outlined | 2465 inside 475 |
| Finish frequency merge | 2600 inside 475 | 2652 outlined | 2676 inside 475 |
| Finish observation merge | 2686 inside 475 | 2738 outlined | 2762 inside 475 |
| `optimal-packed-path-scan` | 3009 outlined | 3061 outlined | 3085 outlined |

The control and CLZ collector are checked Hybrid proofs. The original rebase comprehension at wire block 60, operation 0 forces the non-Tail cold export. Their metadata records the same cold export at native site 510 with 15 scalar words and seven owner outputs, and a 13-Tail body. Both admit a 55-word input packet, a 77-word result packet, six fixed and unique writable buffers, and the 262,144-element tree floor. The vector body adds flatness guards. The input geometry guard survives root cleanup. With the generic flags, nonflat geometry can select the general body and ownership or capacity refusal enters the original checked root before effects. The cold export also returns to its checked source continuation with current state. Plain complete/Fueled proofs and complete numeric trees reject a non-Tail descendant. [Exact proof code](https://github.com/mech-lang/mech/blob/78b064c5bb70937db72db21ebd3ab2fd556816c3/hosts/gpu/src/native_artifact/program/prepare/numeric_phase/mod.rs#L325-L363)

The table producer imports the literal slot matrix in two places inside that same collector. The canonical artifact identifies it as constant 1335, with contents matching all 256 bytes of `deflate-table-offset-slots`. The short record uses direct constant operands at wire operations `[99,9]` and `[100,1]`. The long search record reads the same constant at `[141,10]` and `[142,1]`. The two uses per producer are `sequence/length` and `access/scalar`. Neither producer's table is a carrier field in the bytecode. The control and CLZ artifacts have no slot matrix read inside the collector. [Canonical graph and source anchors](control-graph.json)

`phase_numeric::Provenance` resolves arrays from current carrier fields, their forwards, or proved nested child fields. It does not resolve a literal outside the carrier. The same Hybrid coverage cannot admit the table reads under `array read is not an invariant carrier field`. The complete capture-capable tree cannot replace that outer proof because the rebase remains a comprehension. The generic object confirms the outer phase is absent. Its compile log does not record the first failing Hybrid check, so the exact first-return string has not been observed. The public proof's top-level diagnostic keeps the original Complete rejection and could otherwise report the unchanged non-Tail reason for all three. [Carrier-only array read check](https://github.com/mech-lang/mech/blob/78b064c5bb70937db72db21ebd3ab2fd556816c3/hosts/gpu/src/native_artifact/program/prepare/phase_numeric.rs#L547-L566) [Whole-tree eligibility](https://github.com/mech-lang/mech/blob/78b064c5bb70937db72db21ebd3ab2fd556816c3/hosts/gpu/src/native_artifact/program/prepare/numeric_tree.rs#L110-L136)

`bt-search-loop` lacks that non-Tail descendant. Its immutable slot capture is representable by `readonly_numeric::derive_inner`, which registers exact immutable arrays alongside carrier arrays. The table's admitted child has a 35-word input packet, a 24-word result packet, two fixed writable buffers, and four array geometry guards, consistent with tables, cache, input, and the captured slot matrix. The child emits a 4,892-byte general body and 2,368-byte vector body. The four merge children add 1,784 body bytes plus alignment. Their vector bodies are equivalent to their general bodies and receive no separate emitted symbol. The table collector's root path therefore retains the source loop and checked operations that are outside those children. [Read-only capture discovery](https://github.com/mech-lang/mech/blob/78b064c5bb70937db72db21ebd3ab2fd556816c3/hosts/gpu/src/native_artifact/program/prepare/readonly_numeric.rs#L889-L944)

## Independent scorer effect

The stored slot removes four view words from the scorer packet and eliminates its input slot-table dependency. Both variants retain general and no-growth score loops. Both lose the composed no-growth loop that recognized the original slot lookup feeding an offset cost. Its root dispatch required enough reserved node capacity, at least 2,048 candidate records, and at least 545 cost words, with checked cursor and cache lengths. The removed 1,152-byte body references a pinned `_mech_compact_cost_codes_0` table of exactly 65,536 bytes. The ordinary no-growth body also shrinks by 96 bytes and the general body by 112 bytes. The two experiments cannot imply a standalone runtime gain from moving the slot calculation to the writer.

The table creates more reported phase bodies, including equal alternatives, while reducing body construction. Reported per-body cleanup inputs total 47,452 → 31,066 instructions and cleanup outputs total 16,797 → 11,126. Root cleanup inputs increase 47,662 → 48,373. The one archived native phase receipt is 1.5342 → 1.4012 s for the table and 1.5277 s for CLZ. These single compilations are an opportunity estimate. None binds exclusive time per outline or isolates the compile effect of the removed composed branch.

## Archived natural and random costs

The exact fixed reference is object `7bf9df1a…`, with 71,768 encoder-text bytes. The historical scorer count is from that exact object. The older matcher counter used a different root object and has a byte-identical 6,768-byte matcher atom with no relocations. Its runtime route counts belong to the older worker. The scorer atom is also byte-identical in the older worker, including both named relocations. [Digest and count replay](archived-costs.json)

| Input | Elapsed reduction needed for libdeflate parity | Archived matcher instructions | Archived composed scorer instructions |
| --- | ---: | ---: | ---: |
| Natural 4 KiB | 11.1–11.6% | — | 3,557,035 in five calls |
| Natural 8 KiB | 3.6–4.0% | 3,496,564 in one call | 7,752,576 in six calls |
| Random 4 KiB | 39.3–39.6% | 856,997 in one call | — |
| Random 8 KiB | 38.9–39.0% | — | 0 visits |
| Random 1 MiB | 27.9% | 254,842,426 in 35 calls | 0 visits |

The archived matcher includes cache maintenance and block selection. Its 1 MiB random trace contains 47.7 million stack accesses, 18.7% of instructions in that matcher body. Random 4 KiB has 176,037 stack accesses, 20.5%. These are dynamic work counts, and the scorer's zero visits bound its applicability. Earlier slot-precomputation evidence counts 14,484 stores and 86,904 consumer slot lookups on the 8 KiB natural case. Any improved route still pays for the new producer lookup and can lose the existing composed scoring route.

The older direct backing diagnostic accounts for a median 9.19 µs of 116.46 µs elapsed on random 4 KiB and 32.46 µs of 28.244 ms on random 1 MiB. It observes 4.76 MB and 6.62 MB of logically zeroed bytes, respectively. Direct mapping and allocator intervals leave deferred page faults and first writes in the main body. The earlier PC sampling also overweights `munmap` on small cases. The archives lack a trustworthy exclusive elapsed split among setup and first touches, cache maintenance, match search, block selection, and emission. A narrow staged census remains useful before funding a large random-route optimization.

## One experiment

1. **Goal** — Verify whether representing the existing table as an invariant carrier field restores the collector proof and measure the cost of restoring that phase. Establish why the table formulation changes size before choosing a compiler policy.
2. **Reference** — Use the identical `match-cache-offset-slot/compile_profile` binary, common generic flags, fixed linker inputs, and frozen cases. Compare the new source directly with `stored-table-slot`. Keep `control` and `stored-arithmetic-slot` as explanatory controls. Record the exact 71,768 B fixed reference separately as a historical and cost anchor. Focus any future timing on natural 4/8 KiB, random 4 KiB/1 MiB, a periodic case and a short case.
3. **Mechanism** — Move only the immutable table binding across the existing `optimal-block-loop` and `bt-search-loop` parameters. The table bytes, producer stores, cached bits, score operations, and public entry stay the same. The source rewrite is generated from the exact table-variant SHA and checks every touched call site. Confirm the compiled carrier includes the eighth matrix before attributing any result to the changed binding. [Prepared ablation](thread-table.patch) [Reproduction](thread_table.py)
4. **Potential** — The partition contrast accounts for 13,568 B of aligned outlined text. The large general collector body alone takes 15,904 B and has a checked root continuation already present in the generic artifact. A later generic selection policy could choose to retain only the useful vector body when geometry is proved or checked fallback is available and acceptable. The tuned reference already applies that choice to this collector. For the study, the one native receipt suggests an order of 0.1 s to investigate with exclusive time. Amdahl's bound for random 1 MiB requires 27.9% of elapsed time even if the targeted category could disappear completely. The observed direct backing intervals are about 0.12% in the older diagnostic, so they do not justify a setup-only effort.
5. **Contracts** — Preserve compressed bytes, capacities, unchanged borrowed input/output, checked source origins, owner cleanup, and the existing checked continuation before mutation. The immutable addition grants no write or ownership authority. Preserve admitted geometry, finite resource accounting where it applies, and default portable source behavior. In the diagnostic, capture resource counters and a checked smoke for longest/short records, joins, rebase, rewind, output aliases, and malformed bounds. Check origins by source correspondence when site numbering changes. Any later full reference candidate must declare the applicable stack bound explicitly.
6. **Experiment** — Compile the prepared threaded-table ablation once after quiet-slot clearance. Inspect the bytecode seed, phase admission and fallback, exact root and collector relocations, and constants. Use the same clock and compare observed fast, general, and checked route entry counts before treating emitted bytes as useful. If the mechanism merits dynamic study, use one scoped diagnostic acquisition on the same symbols. Keep counters for matcher spans and sampled PCs inside the call boundary, time only coarse phase boundaries, and leave hot-loop timings uninstrumented. Record actual first touches alongside setup, separate cache stores and matching from block decision and output emit using source or machine labels, and record unattributed work explicitly. The paired fixed-reference and immediate-control comparisons remain separate.
7. **Exit** — Stop initial diagnosis by 60–90 minutes or two adverse hypotheses. A successful ablation reproduces the expected eighth read-only matrix and restores the outer Hybrid outline or identifies its next specific rejection. If useful route loss explains the size difference, archive it and park this source formulation. A later general transport or selection change earns one bounded candidate only if the new diagnostics show recoverable hot coverage or meaningful generic size and compile potential. A two-order focus under v2 is a screen. Full promotion requires applicable correctness, resource checks and both 81-case orders.

This diagnosis used existing objects and traces. Compiling the ablation awaits a quiet measurement slot.
