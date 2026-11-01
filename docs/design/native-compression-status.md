# Native compression: current status

This is the current-status owner. Historical experiment reports and all raw
observations are in the verified [external evidence archive](../../benchmarks/mech-deflate/EXPERIMENTS.md).
They are not additional current implementations or fresh qualification.

## Current measured checkpoint

R78 is retained at published source checkpoint
`6ab75037bc5d4ec138d735c7dcfe1c4b653d31fa`. Its production change is
`453bb17592bcc843c367cc70312496bda0d940a4`; the actual optimized compiler build
is `14d0c11ccda5a4a311dc017e3776d73a44a14e75`. Final qualification source
`84f180ca2bdadf139eb570912d8c01aedb08c971` adds only tests/observation migration.
The stack base is `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
[PR #973](https://github.com/mech-lang/mech/pull/973) remains draft. Publication
CI run 41267325781 was completed/cancelled at the user's direction; green CI
and merge readiness are not claimed.

Measurements cover **complete level-12 compression of `abcdefgh`**, with the
pinned allocator and only `evaluate_into` inside the timer. They do not measure
large-input throughput. The reference is Roc = 1×.

| Metric | Fresh reference | Matched parent | R78 |
| --- | ---: | ---: | ---: |
| Primary elapsed | 0.290000 ms | 3.5735000 ms / 12.322414× | **3.5681245 ms / 12.303878×** |
| Retired instructions, separate cohort | 1,364,372 | 40,126,037.5 | **40,167,249 / 29.440101×** |
| CPU cycles, separate cohort | 910,563.5 | 11,229,745.5 | 11,239,286.5 |
| Allocation requests, including reallocations | 95 | 6,791 | **6,791** |
| Requested bytes | 9,021,923 | 98,112,208 | **98,112,208** |
| Incremental allocator peak usable bytes | 9,257,328 | 20,482,072 | **20,482,072** |
| Generated compressor text | — | 11,800,572 bytes | **11,753,624 bytes** |
| Generated entry stack | — | 107,344 bytes | **107,280 bytes** |

R78's pooled elapsed improvement is **0.150427%**, with three of four independently
prepared pairs improving and one pair worsening 0.165241%. The 3,968 primary
observations include 38 improving and 26 adverse within-pair blocks. These blocks
are not independent experiments. Instructions worsen 0.102705%; cycles worsen
0.084962%. Separate CPU-cohort elapsed also worsens slightly. No significance
claim, per-function instruction attribution, or cross-cohort multiplication is
made. Requested capacity is not initialized bytes copied, live memory, or peak RSS.

R78 combines 807 managed moves and 2,790 structural observations with their
preceding admission callback, preserving both operation bodies and charge order.
External crossings fall 21,286→17,689, while all 12,450 logical admissions and
39,030 shared debits remain. Actual host assembly exposes a contrary cost:
the helper frame grows 144→320 bytes and adds unconditional register saves.
This explains why fewer crossings alone do not establish fewer instructions.

## Complete computation and fidelity

Mech performs compression itself as one compiled computation. The maintained
[25-component assembly](../../hosts/gpu/tests/fixtures/native/deflate_assembly.rs)
implements match finding, block collection, near-optimal parsing, Huffman models,
bit writing and `deflate-compress-into`. It is not a native compression-library
call. Its default complete entry remains separately pinned by its assembly test.
The measured caller-output entry has these unchanged pins:

- Source SHA-256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
- Bytecode SHA-256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
- R78 Object SHA-256: `d74a85290e40bf7a464f0f34259dd5197f9fae2bbb22356a1aec89ea87eddfbf`.
- Typed IR / work policy / private Program ABI / bytecode: **132 / 95 / 38 / 14**.
  Frozen scalar ABI is independently **1**.

The tree is 262144 U16 entries; the cache is 1500513 U32 words; path capacity is
305257 U64 entries. Depth 300, nice length 258, original near-EOF limits, tables,
source allocation sites and timing boundaries remain. Search and skip stay
separate; skip has no cache argument or recording path. Caller output capacity
is 13, cleared/rebound outside timing. Its physical public-boundary handoff remains
inside the measured computation.

Every measured eight-byte call checks exact raw bytes
`[75,76,74,78,73,77,75,207,0,0]`, independent inflation and input aliases.
Logical work is 60,260,219,686 and comparison work is 47,612; neither is a hardware
instruction count. Final managed bytes and snapshot nodes are zero.

## Maintained architecture and qualification

Source semantics and the canonical artifact codec feed Engine native facts and
prepared proofs. Shared preparation serves JIT, Object emission and independent
linked-image restoration. Scalar emission, private frames, Core snapshot owners
and the existing resource ledger remain the maintained owners. Selection facts
do not grant runtime ownership, funding or execution permission. No compressor
names, artifact IDs or buffer sizes authorize an optimization.

R78 has 44 passing required receipt checks. Executed selections overlap and are
not summed as distinct tests: GPU runtime 331; work/memory/value 32; components
236 with unchanged goldens/allowances; connected cases 8; native path witness 1;
linked original/decoded recovery 12; scalar controls 12 at unchanged 1.05 ceiling;
frame allocation/alias controls 8; exact GPU CI selection 37; Python workflow
checks 132; Engine projection 12 and shape 6 using the unchanged accepted Engine
binary; architecture 10 and type/memory 5. Formatting, warnings-as-errors and the
unchanged unsafe allowlist pass. Runtime-only GPU compilation uses pinned
feature-rich dependencies, not a freshly built minimal dependency graph.

Original/decoded and restored-image controls cover current arguments, Boolean
state, finite denials, ordered errors, aliases, unchanged prior publication,
cleanup and retry. Earlier Core/Budget/planner receipts are explicitly inherited,
not fresh R78 executions. All initial harness failures and their resolutions are
preserved in the archive. The historical R10 scalar timing violation remains a
historical unresolved observation; later passing cohorts do not erase it.

Historical saved-decoded runs completed 16384-byte block and 33027-byte window
transitions. They do not qualify the latest compiler at those sizes. Repeated
slides, full-corpus agreement and current large-input timing remain unqualified.
The cancelled full-corpus run has no timing result and is not restarted here.

## Next bounded work

Broader native parent composition is the main optimization project. The current
exact-one-immediate-child restriction excludes larger reached parents. First
support multiple ordinary child boundaries and acyclic pieces between them,
using the existing frame, provenance, liveness, admission and numeric proofs.
Reacquire current owners/views for each piece; no loan spans child execution.
Acceptance requires fewer executed host operations and instructions, not merely
more selected metadata.

A separate bounded experiment targets exclusive first admission through the
existing unique-root/data and empty-registry path, retaining the locked fallback
for shared or weakly observed values. Its candidate allocation count is not a
promised saving. Managed projections within native pieces and precise success
boundary synchronization follow as separately measured changes. Ordinary debit
function tuning is closed absent new mechanism evidence.

The pinned baseline's algorithm and source remain. Any source experiment gets
separate names, pins and measurements. Any representation/resource-policy change
must be explicit: preserve source error order, current authority, published output
atomicity and recovery. Do not conceal work by raising limits.
