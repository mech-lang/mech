# Post-architecture native cleanup

The audit preserves #935 checkpoints
`01b8b397011b6a774b7d9f730578a4ef92bf30f6` and
`a3e805aae775a8aa6bf181714c8cbe7c9e6619e9`. Execution adopts the same
architecture patch on the published-history-preserving #935 head
`7543c56d5efc4df30d0b60961922024d74caa9d7`, based on #934
`a093aea452340a70c1e72a1f58bd831f1edef70f`. This cleanup is a separate PR
before the compression prerequisites; downstream reconciliation belongs to
the compression work. No PR merge is part of this work.

The audit covers 363 Rust files and 110,157 lines including tests in the native
host, engine native-artifact and build owners. Exact body matching is a candidate
finder, not a proof that two semantic responsibilities can be merged. Full
inventories, original files, hashes and diagnostic history belong to the primary
`post-native-architecture-cleanup-v1` study in mech-lang/benchmarks.

| Candidate | Audit result and action | Required check |
| --- | --- | --- |
| Explicit default module paths | 256 attributes resolve to the exact Rust default; remove them. Preserve the two similarly named paths with different resolution. | Six independent profiles; module and unsafe guards |
| Inherited feature guards | 19 guards repeat their parent module condition; remove those repetitions. Keep compiler/test and hosted-only conditions. | Independent runtime closures and compiler profiles |
| Manual kernel/region diagnostics | Three ignored observation/timing probes belong to benchmarks; move their exact source plus a pinned runner. Keep all substantive correctness and differential tests. | Benchmark probe selection/build and preserved source hashes |
| Historical move manifest and implementation receipts | No code/guard caller consumes the move JSON. Archive the full ledger and receipts in benchmarks; keep maintained architecture/ownership documentation here. | Reference audit and archive checksums |
| Map admission primitives | Binding, packed and String owners duplicate checked arithmetic, finite-limit order and identical map errors. Give the unchanged operations one map-admission owner. | Exact/one-short admission, error and recovery tests; final timings |
| Sink adjacency | Histogram and TextJoin sinks have identical neutral linear-path/barrier checks. Share that predicate without merging either sink's eligibility policy. | Existing source/decoded sink validation and failure-order tests |
| Program application construction | Retained after prototype review: sharing the distinct managed/hosted preflight paths grew the file from 211 to 227 lines and introduced a second profile owner. Existing output assembly remains with each public generator. | Existing public generator and process contracts |
| Backend setup | One compiler-only constructor now builds JIT/Object modules for scalar, match, iteration and Program owners. Immutable family flag sets retain their exact values/order; each owner keeps its typed entry, preflight and definition. | Scalar-family original/decoded and Object equivalence checks |
| Repeated test snapshots | Six identical snapshot helpers share one private, test-only helper. All 18 existing test bodies and assertions are preserved. | Native unit suite |
| Scalar identity transcripts | Scalar, Match, Iteration and Program identities repeat scalar type/value, action/terminator and source-origin encodings. Share the frozen byte encoding; keep each family's transcript order, domain prefix, versions and independent linked/image validation. | Existing identity and original/decoded suites; exact guest artifact comparisons |

## Guarantees and limits

Public source, bytecode, native and embedding APIs remain available. Runtime-only
closures stay free of the parser, JIT and compiler. Bool/U64/I64/F64 short-call
native paths remain first-class. Image reconstruction independently checks its
untrusted DTOs; immutable prepared facts cannot replace that boundary.

Keep frame/identity/wire versions, exact resource charges, terminal failure,
publication and loan/drop order. A common helper cannot reorder checks or infer
new eligibility. No compatibility bridge, ambient optimization switch, alternate
runtime route or benchmark-only guest fast path replaces a deleted owner.

The #935 executable and its evidence remain preserved as the before-cleanup
subject. Benchmark studies keep exact retained W/R controls and the same
k-nucleotide source, input, outputs, work and process boundary. No primary timing
samples have been collected. The original architecture and this additional
cleanup each need their pinned before/after evidence; code equality alone does
not qualify speed.

## Deletion ledger

The audit precedes implementation. Update this ledger with actual deleted,
archived and consolidated code, validation and unresolved performance gates.

First deletion batch removes 256 redundant path attributes and 19 inherited
feature guards. Module targets, visibility and effective feature conditions are
unchanged. Independent compilation and existing guards verify the resolution.

The visibility batch removes 179 exact-current-module restrictions across 81
files and groups 68 reexported names into 17 declarations across nine files.
Default private visibility, feature gates, import aliases and module targets are
unchanged. Ancestor restrictions, inline-module cases and nondefault path cases
are excluded. Exact before/after inventories and exclusions are archived in the
benchmark study's `audit/visibility-reexports-v1` directory. The repaired base
and regenerated linkage coverage are adopted; final runtime qualification remains.

Map admission now owns `Limits`, storage `Error`, checked add/multiply/limit
checks, and ordered finite-limit checking. Binding and packed owners retain their
specific unbounded-dimension messages. Owner invocation geometry and frozen-read
checks remain separate because their error types/origins differ. Histogram and
TextJoin consumers share only the identical linear-path/barrier predicate;
consumer eligibility and error ordering remain in their original validators.

The three archived probes have a maintained runner and README in
[tools/native-compiler-probes](https://github.com/mech-lang/benchmarks/tree/codex/native-architecture-consolidation/tools/native-compiler-probes).
The tool installs an exact pinned test overlay in a disposable checkout and uses
the existing bounded process controller and common mutex. Preparation and tool
contract tests pass. At cleanup checkpoint `569aa84`, both probe targets compiled
and all three archived tests were selected exactly; no timing probes executed.
Raw receipts and earlier attempts are retained in `probe-compile-569aa84-v1`.
Production correctness and differential tests remain beside their owners.

Current verification: formatting, diff whitespace, module layout, runtime-factory
safety, build-analysis layout, warning policy, compiler-planning quarantine and
unsafe-boundary checks pass. The initial cleanup checkpoints passed all six
compiler/runtime profiles, 426 native, 84 engine, 34 scalar unit, 44 scalar
integration and 134 program integration tests. All 32 representative artifacts
matched the architecture checkpoint byte for byte. These receipts remain pinned
to their original sources in `qualification-checkpoint-83ee-v1` in benchmarks.
Final qualification covers the adopted base and additional visibility cleanup;
k-nucleotide timing qualification remains open.

The next authority batch gives the frozen scalar identity encoding one private
owner. It introduces no allocation, dynamic dispatch or alternate plan. Program
identity retains its existing noninlined word/byte boundaries. Family-specific
identity fields and every independent image/linked check remain with their
owners. The isolated `cb41553` checkpoint passes all six library profiles and
four runtime dependency closures. This published successor preserves its complete
Rust cleanup patch and adopts the test-only base repair. Exact final-head
correctness, artifact equivalence, process and runtime evidence remain pending.
