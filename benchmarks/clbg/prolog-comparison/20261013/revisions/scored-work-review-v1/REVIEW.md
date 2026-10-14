# Independent Prolog scored-source and original-WAM work review

Source-only review by the independent C3/LOC peer. All 20 authored compact/
optimized main files and both whole authored helpers were read. Original WAM
predicate/instruction witnesses were inspected for both forms, with detailed
constructor/lifetime, window/map and two-division review below. The selected
SWI factory/hashtable sources and prepared-work-v2 transformation/observer/
reconciliation mechanism were also read. No hash, import, parser/counter,
fixture, native command, new trace, benchmark or Git operation ran here.

Authority is the actual all-20 preparation
`qualification/prepare-explicit-goal-rest-v1/report.json`, SHA
`cab6ea7f3bd8a237f8f83833ed9b9cc293bbe1878cc8a894d74fdcf41f7ffbff`;
the root's 14,611-pin review is
`continuation/jobs/prolog-remaining155-closed-review-v1/receipt.json`, SHA
`139d9124d3df942bb3170112b3841c33b0191580a002f974835a627651be9bc8`.
Its recorded 156 bounded output/ownership cases are completed evidence;
`source_work_qualified=false`, LOC null and chart admission false remain exact.
Recorded digests below are references to that existing evidence, not new
digest calculations or a new physical closure audit.

Scored source authority is `revisions/prepare-success-halt-v1/sources/`
`<benchmark>/{compact,optimized}.pl`, plus entire `support/clbg_io.pl`
bc636145 and, for knucleotide/revcomp, entire `support/clbg_records.pl`
efb32920. This is 22 original authored paths/44 per-profile references; both
form bodies are identical explicit baseline aliases, without a distinct
optimization claim. The report's `source_bundle`, main source SHA,
`application_artifact_pins`, compiler receipts and loaded-input inventory bind
the actual measured copies/QLF to these originals. This review relies on the
already completed root physical closure for those byte identities.

For exact WAM selection use each report profile's
`compiler_bound_input_inventory.original_scored_wam` and recorded SHA.
Except compact regex, paths are
`qualification/prepare-explicit-goal-rest-v1/builds/prolog-<benchmark>-<form>/original-scored.wam`.
Compact regex instead uses the original preserved
`qualification/prepare-success-halt-v1/builds/prolog-regexredux-compact/original-scored.wam`.
QLF/WAM are compiled evidence; neither substitutes for counted authored source.

| Critical WAM | Compact recorded SHA256 | Optimized recorded SHA256 |
| --- | --- | --- |
| binarytrees | `7422729ac47d51a16d69e609547a9e3eafde237eaec71516d1b7085ba0b21bbb` | `bcd1abca7781f2e226733840bc8fc24a92b214b19d09651431f3bda6a01a236c` |
| knucleotide | `baa2607c0f9d08528dc21ac0701cb5e84ffb2ac65397dc1bc5317d26f2cd3a36` | `1365f5c0623df3a722e54db86a99853ae81169dce5ed0ffa2e8ee2f095b04ce7` |
| pidigits | `2c993dafa468e80975989ff060b611cba179a6e3f50247a04fe368e9b27ab3c7` | `69158b3b7e804eaf830f5d5a874c70b147b210ad60f3af64ff6aae874da5b1e3` |

**Binary Trees.** Both mains (source SHA 00b81975) call `functor(Tree,node,2)` before the
depth branch. Depth zero therefore still creates a compound leaf, binding
both children to the scalar sentinel `none`. Positive depth constructs left
and right recursively. `checks/4` constructs and then traverses a tree on
every iteration; depths advance by 2 from 4 through max(6,N), with
`1 << (Maximum-Depth+4)` iterations. Stretch completes before retained-tree
construction, and main references that retained tree after all temporary loops.
Original WAM `make/2` has `i_call(functor/3)` at VM offset6, preceding the depth
test, both recursive constructor paths and leaf bindings. `checks/4` uses
`b_firstvar(4)` before its make call on each recursive invocation. Main uses
`b_firstvar(3)` for the retained result, calls depths at28 and then passes
`b_var(3)` to final check at34. This preserves the live root across those loops.

Selected official source is
`.build/swi-runtime-directory-v1/official-source/swipl-10.0.2/src/pl-prims.c`
functor/3 at2196–2225, delegating an unbound arity2 term to
`PL_unify_functor`; `src/pl-fli.c` at 3167–3193 allocates from gTop, initializes
the two argument cells and advances by three cells before binding the result.
This establishes the fresh constructor route, including real leaves. Shared
functor descriptors and `none` are generic scalar metadata, not shared live
tree nodes. GC/stack relocation and reuse of dead temporary storage are valid;
this review does not require globally unique addresses across dead iterations.

**K-nucleotide.** Both mains (source SHA ba1fdb0a) find the THREE record, consume its complete
sequence until EOF/next header and scan widths 1,2,3,4,6,12,18 into separately
created generic tables. Every index advances by 1, rolls the two-bit masked key
and, once Index>=Width, increments that window's entry. Thus each width covers
exactly max(0,L-width+1) overlapping windows, including the tail. Query lookup
happens after all seven maps; widths 3/4/6/12/18 are not limited to named queries.
Both original WAMs have all seven widths in main, `ht_new/1` before
`windows/7` in all_maps, lookup/increment/ht_put in windows at57/67/77 and
Index+1 at85 before recursion. The selected loaded generic
`.build/swi-runtime-directory-v1/arm64-app/SWI-Prolog.app/Contents/Resources/swipl/library/hashtable.pl`
creates `ht(0,0,[](_))` at 80; insertion/growth at 148–206 uses powers-of-two
buckets from minimum 4 and preserves every occupied key/value. There is no
query-specific or preallocated benchmark table helper. Whole authored records
helper preserves sequence order and header/comment/CR/LF handling.

**Pi.** Both mains (source SHA 70a0f6a0) compute distinct 3Q+R and 4Q+R numerators and divide
each by T before their equality test. Original `digits/7` WAM separately stores
numerators in slots 7/8, performs `a_func2((div)/2)` at 28 and 37 into slots 9/10,
then compares them at 49. Accepted candidates emit a digit and update Q/R by 10;
rejected candidates perform the complete K recurrence and keep emitted Count
unchanged. The bounded=false guard permits unbounded integer arithmetic.
This excludes an authored one-division substitute or known-digit shortcut.
Ten-digit line/tail padding remains in original source and WAM. Generic SWI/
GMP arithmetic implements the two division operations; no allocation or timing
conclusion is inferred merely from their presence.

The remaining fourteen mains and their original WAM call/loop witnesses agree:

| Benchmark (both forms) | Source work and inspected WAM witnesses |
| --- | --- |
| fannkuchredux (53ca5622) | Factorial total, all rank permutations, a complete fresh work-copy and flips, prefix carry, rank parity checksum/max. `permutations/9` calls copy/flips at 15/23 and next_permutation at 88 before recurring at 118. |
| fasta (b5d06706) | CDF construction, every seed-42 LCG draw, linear probability selection, 2N/3N/5N records, 60-byte/tail output and TWO→THREE seed continuity. `random_codes/5` retains 3877/29573/139968, division and select; random_lines calls it for each chunk. |
| spectralnorm (1d868f9e) | Every dense row/column coefficient/product, ten rounds of two ATA applications (40 A/transpose operations), final two dot sums. `power/6` calls apply_ata twice at 6/14, row calls coefficient and performs multiply/add before Index+1 recursion. No library matrix kernel. |
| mandelbrot (97e78601) | Every actual pixel performs up to 50 floating recurrences with magnitude>4 escape. Pixel passes 50 at 15; escape retains both squares, complex updates, threshold and decrement/recursion. Eight-bit packing only zero-fills nonexistent tail pixels. |
| nbody (d3e082e3) | Five canonical bodies, ten unique pairs per step update both bodies' velocities, then five positions; energy before/after all N steps. `advance/3` calls forces/positions at 5/11 then decrements; forces/right_forces/force retain pair loops and all three velocity updates. |
| revcomp (2460ba65) | Every record/base, complete case-insensitive IUPAC complement, reverse index decrement, ordered 60-byte/tail output; records/reverse_codes/complement/emit_sequence remain in WAM. Both entire authored records and IO helpers are included. |
| regexredux (912c4b20) | Nine independent match folds and five ordered global replacements, case-sensitive with literal jit(false); count_match increments per match. Original WAM retains re_foldl, count callback and ordered replacement recursion. Matching is the disclosed generic PCRE library, not an authored Prolog regex engine. |

**Bound on future probes.** Read `prepared-work-v2/instrument_sources.py`,
`observer.pl`, `work_proofs.py` and run.py's original-source/probe route.
Unique exact-text insertions reverse to each original main; creators,
checkers, windows and both divisions remain unchanged. New main/observer
copies are explicitly unscored and cannot replace original WAM/QLF authority.
Trees use same_term identity, reject shared/cyclic graphs, verify all live
nodes/edges/leaf counts and concurrent retained disjointness; only the already
long-lived root is retained between callbacks. Maps preserve actual seven
roots/capacities/all entries and final order, but explicitly do not trace
individual windows: this original loop review is therefore necessary. Pi
records actual operands/quotients/states after both divisions; the host checks
the independent recurrence and independently validated output digits.

No unresolved source/WAM work mismatch was found within that scope. This is
not an actual 30-probe pass, new runtime closure, allocator measurement or full 90
output/timing qualification. A later consumer must bind this note's actual
frozen bytes alongside the actual 30 source-linked native receipts, original
WAM/QLF/bundles/runtime pins and full-output oracles. Missing/failed cases,
unresolved native identity or different transformed bytes must still block.
Synthetic six-fixture traces alone cannot qualify native work. No numeric LOC
admission, optimizer performance claim or chart change follows from this note.
