# Native compression: current status

This is the current-status owner. Historical reports and raw evidence are in the
[external experiment archive/index](../../benchmarks/mech-deflate/EXPERIMENTS.md).
The source tree retains compact hash receipts, maintained code and tests.

## Current measured checkpoint: R79

Retain native pieces around multiple ordinary children. Complete pinned **eight-byte,
level-12** compression improves **12.424308× → 11.207469× Roc**, or **9.794019% less
elapsed time**. Larger-input throughput and parity remain unestablished.

Round base is `0eb0c3d9d8a59fbf14b4ec3563944a276b56b8a4`; production is
`c5e4d66188339025ddfcbf83f0ab1e9788f237f3`; the actual optimized compiler build
source is `374edf1410b5f975d793fb69d8fcf390cbfa10a5`. Later changes contain tests,
tooling and evidence. Stack base remains `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
[PR #973](https://github.com/mech-lang/mech/pull/973) stays draft; hosted CI is
cancelled at the user's direction. No green-CI or merge-readiness claim is made.

| Metric | Fresh reference | Matched R78 parent | R79 |
| --- | ---: | ---: | ---: |
| Primary elapsed | 0.289000ms | 3.5906250ms /12.424308× | **3.2389585ms /11.207469×** |
| Retired instructions, separate cohort | 1,364,372 | 40,167,971 | **36,057,179 (−10.234005%)** |
| CPU cycles, separate cohort | 943,541.0 | 11,262,383.5 | **10,326,071.5 (−8.313622%)** |
| Requests, including reallocations | 95 | 6,791 | **6,791** |
| Requested bytes | 9,021,923 | 98,112,208 | **98,113,216 (+1,008)** |
| Incremental allocator peak usable bytes | 9,257,328 | 20,482,072 | **20,482,648 (+576)** |
| Generated text bytes | — | 11,753,624 | **14,660,740 (+2,907,116)** |
| Generated entry stack bytes | — | 107,280 | **124,048 (+16,768)** |

Four independently prepared pairs contain3,968 timed /3,984 total observations.
All four pairs and63/64 blocks improve; the adverse block is retained. Within-pair
blocks are descriptive, not independent experiments. The separate32-observation
CPU cohort also improves; its elapsed is excluded from primary. Sample weights,
retired instructions, logical work, requested capacity and live memory remain
distinct quantities. OS RSS includes retained compiler/catalog/image storage.

## Compiler change and architectural owners

The maintained composition pass now inventories multiple ordinary child edges
and proves acyclic pieces between them. Child initialize/exit, mutation,
publication and recovery stay ordinary. Each piece reuses existing frame,
provenance, liveness, admission and numeric proofs, and reacquires current owners
and views. No view, private owner or prepaid fuel spans child execution.

Nested control retains one usize pending-edge marker. Its expected suspended
ancestor is derived once from immutable validated loop ancestry after selection.
Runtime still checks exact edges, current progress and source visits. Collection
initialization preserves its existing two-phase protocol. There is no additional
continuation stack, executor or ledger. Unsupported managed sources and ambiguous
provenance retain ordinary execution.

Static emitted pieces expand67→566; the candidate inventory contains76 parents.
Actual diagnostics count500→1,255 executed pieces, including82 in the nine-child
pilot parent. Artifact IDs locate evidence and grant no production permission.
Callbacks fall17,689→14,460; shared debit attempts39,030→32,738; frame packs847→336;
frame copies1,139→925; scalar projections1,361→153. **Managed projections remain
2,176**, the next structural target. Four instrumented calls agree; instrumentation
is excluded from timing, hardware and allocation acceptance.

Source semantics/canonical codec feed Engine facts and private proofs. Shared
preparation serves JIT, Object emission and independent image restoration. The
scalar emitter, frame/owner arenas, Core snapshot ownership and existing resource
ledger retain their responsibilities. Selection facts are not funding or alias
permission. Fast Bool/U64/I64/F64 execution retains its unchanged1.05 gate.

Typed IR / work policy / private Program ABI / bytecode are **133 /95 /38 /14**;
frozen scalar ABI is independently **1**. Existing piece prices apply at newly
selected sites. Actual charged compute work changes60,260,219,686→67,457,686,430;
comparison work47,612→23,063. Those units are not hardware instructions. No prices
or allowances are increased; exact/one-short finite boundaries and error-prefix,
publication and retry controls pass.

## Complete computation and fidelity

Mech performs compression as one compiled computation. The maintained
[25-component assembly](../../hosts/gpu/tests/fixtures/native/deflate_assembly.rs)
implements match finding, collection, near-optimal parsing, Huffman models, bit
writing and `deflate-compress-into`. Its default complete entry has its own
assembly pin; the unchanged measured caller-output entry retains:

- Source SHA-256 `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
- Bytecode SHA-256 `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
- Real tree262144 U16, cache1500513 U32 and path capacity305257 U64.
- Depth300, nice258, near-EOF limits, tables, source allocation sites and pinned allocator.
- Output capacity13 with clear/rebind outside the `evaluate_into` timer. Physical
  public-boundary copies remain; this does not claim physical output reuse.

Search and skip remain separate, with no cache argument or recording path in
skip. Exact raw bytes are `[75,76,74,78,73,77,75,207,0,0]`; independent inflation
and input aliases agree, and final managed bytes/nodes are zero. Large allocation
histograms retain five tree requests, one cache request and35 path requests.
Requested spare capacity is not initialized/copied byte volume.

Fresh CPU samples contain1,294 parent /1,300 candidate /1,515 reference exclusive
weights. Known frame-action ancestry falls320/1294→207/1300. Categories overlap
and are not instruction counts. The candidate retains98 unresolved exclusive
weights; zero named thread-local weights does not prove that work disappeared.
Code/stack growth is a measured tradeoff, not an unreported win.

## Qualification and remaining scope

Fresh executed selections overlap: GPU runtime334; work/memory/values32; unchanged
component236; connected8; native path witness1; exact CI inventory/execution40;
scalar controls12 (max ratio1.0331314 against1.05, request/byte equality and zero
loop allocation slope); frame controls8; new linked/AOT original/decoded cases16;
Python workflow132; architecture10 and type/memory5. Runtime-only GPU cfg metadata
compilation passes warnings-as-errors against pinned feature-rich dependencies,
not a freshly built minimal graph. Unsafe allowlist and warning exceptions remain
unchanged. Core/Budget/planner/Engine results retain their earlier receipts and
are explicitly inherited, not new executions.

The new witness covers collection, sequential/nested children, current buffer/
Boolean visits, unselected paths, assertion/arithmetic/read order, unchanged prior
outputs, aliases, cleanup and retry through original/decoded and restored images.
Append-child controls exercise real growth and resident agreement. The former
negative multiple-child eligibility test now verifies ordinary child boundaries;
compression output goldens and allowances are unchanged.

Initial collection-phase and runner errors remain archived with their fixes.
Unexpected missing tracked files in the temporary checkout were restored only
when absent and verified by Git blob. A recovered allocator wrapper reproduces
the measured executable byte-identically. Raw generated Rust/C lives outside the
source checkout, preserving the strict source audit. No required local failure
remains. The historical R10 scalar timing violation remains a historical unresolved
observation; subsequent passing controls do not erase it.

Historical saved-decoded16384-byte block and33027-byte window results do not qualify
this compiler at those sizes. Repeated slides, full-corpus agreement and fresh
large-input timing remain unqualified. The cancelled full-corpus run has no result.
The giant complete knucleotide unit and hosted CI are not counted as passes.

Next: measure exclusive first admission using current root/data uniqueness and
an empty registry, retaining the ordinary shared/weak fallback. Then extend managed
projections within native pieces and synchronize precise live/modified state as
separate measured changes. Ordinary debit-function tuning stays closed absent
new mechanism evidence. A source variant requires separate names/pins/results.
