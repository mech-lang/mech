# Round 03: schema retention summary and focused build

Measured head: `ac96b275d23c1219bb3c8c8067f4e05d19a44b47`.
Runtime implementation: `ef0e5d37a47c547a7c4a28f1bfae508f4d977cb8`.
Previous published head: `fb9c64ab1500b165dc79c0ed4464ffad6467dbbf`;
its retained runtime executable implements `335de1c167`.
Stack base remains `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
The [protocol](../../../optimization-rounds/schema-retention-summary.md),
including the build-profile amendment, precedes these observations.

## Small runtime comparison

Apple M1, eight-byte level-12 input `abcdefgh`, **Roc = 1x**:

| Subject | Median seconds | Divided by fresh Roc |
| --- | ---: | ---: |
| Pinned Roc, 20000 observations | 0.000275 | 1x |
| Retained before executable, full catalog | 0.757143375 | 2753.25x |
| Schema summary plus selected catalog | 0.523613000 | 1904.05x |

The after/before ratio is 0.691564: 30.84% less elapsed time, or 1.446x the
throughput of the before subject. There are three retained Mech observations per
subject, plus an excluded warmup and extra call. All observations are retained.
The earlier round's 2494.04x used a different 0.000305-second reference median.
This cohort measures the combined schema-summary and build-profile change;
it does not isolate their individual contributions. The 1x goal is unachieved.

Both subjects use the byte-identical harness
`8595dfc8b529d5ad06b57580314e3fe4fc6110bd161ffa768a3d59e6c6608fd2`,
the same pinned mimalloc 2.1.7 object, and no allocation instrumentation.
Only `evaluate_into` is timed. JIT, output reserve/clear/rebind, verification,
disposal and output slot preparation are excluded as before. No other local
compiler, artifact compilation or compressor was active during timing.
Ordinary user/system activity was not isolated.

The complete source hash remains
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
the artifact remains
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
All 25 components, algorithm, tables, buffers, allocation sites, six Huffman
initializers and search/EOF limits are unchanged. Every call returns the same
ten raw bytes, independently inflates, preserves the input alias and output
capacity 13, and finishes with zero managed bytes/nodes. Before executable:
`a98a68366c1d3f807f5ab0ff78a105b72244d3a14f1da8bbcf7aa8ab0e484e6f`;
after: `44ad75d2536c5975058933a97a1703de209930931dcf5bc7dcb6e98ece01c2e8`.

## Runtime change and accounting

SchemaTable derives a private checked allocation-bound summary whenever its
immutable entry arena is constructed. Clones copy the summary with the same
arena; every new arena recomputes it. The ordinary no-callback query reads it.
The callback/precharge visitor still traverses every contribution in order.
Overflow remains None. Canonical identity and serialized bytes are unchanged;
the summary is neither caller-supplied authority nor a new execution route.

The real inline header grows from 16 to 32 bytes. Existing concrete layout
accounting includes those bytes. Reported logical work rises from
939091080723 to 941491317987 (delta 2400237264); no allowance or charge was
weakened. The layout diagnostic independently agrees with the full visitor.

The unchanged System-plus-allocation-probe witness passes all 24 observations.
For eight stores into the real 262144-entry U16 tree, the retained two-sample
median changes from 3.776 to 3.292 ms; for the 1500513-word U32 cache, from
21.320 to 18.687 ms. Both retain 9913 allocation requests. Requested bytes rise
by 128, one additional 16-byte schema header per store: 5302512 for U16 and
49124624 for U32. Physical copies remain. These diagnostics are separate from
the primary whole-compression timing.

The separate two-second profile contains 1358 main-thread samples. No named
no-callback allocation-bound traversal appears, versus 373 inclusive samples
in the previous profile. The ordered precharge traversal remains visible
(81 samples). Remaining visible costs include thread-local access, allocation
and freeing, memory-budget evaluation, copying and snapshot length encoding.
These sample counts are not exact elapsed fractions. Deduplicated/JIT symbols
are not assigned unsupported names. All eight profiling repetitions, the
warmup and extra call are excluded from the primary statistics.

## Focused build and validation

Building a GPU example activates its full-catalog dev-dependencies even with
`--no-default-features`. The new [ordinary consumer](../../../native-profile/README.md)
uses the existing catalog feature gates: five arithmetic operations, six
comparisons, and/or, and inclusive ranges. Actual Cargo artifacts contain only
the math, compare, logic and range machine crates. Resolved Core, Engine and
Syntax features are identical to the previous build. Registry dependency
versions match the workspace lockfile. The full qualification profiles remain
unchanged. Exact Cargo commands and linked rlib hashes are recorded.

The broad build was deliberately stopped after 1376.43 seconds while still
compiling full math. The first selected build completed in 990.50 seconds,
including Core/Engine recompilation after the feature graph changed. A repeated
unchanged build completed in 0.313 seconds with every artifact fresh. These
are observed build receipts, not a controlled clean-build speedup comparison.

Fresh source compilation (160857 bytes) produces the exact pinned 4138627-byte
artifact; decoded re-encoding also agrees. Eleven existing native Rust tests
pass using the actual selected Cargo libraries: eight scalar tests covering
resident agreement, finite denial, error order, cleanup and recovery, plus
three bit-operation tests. The all-scalar-primitives test is explicitly skipped
because it includes not/xor, which this profile omits. No test source changed.

Core qualification adds 96 schema/snapshot, 47 memory-runtime and 23
memory-safety tests: **177 distinct Rust tests pass in total**. The initial
no-default-feature memory invocation selected zero tests; its receipt remains
and it is not counted. The corrected feature-enabled command and no_std check
pass. Protected original/decoded Bool/U64/I64/F64 controls pass all predeclared
allocation and timing gates; after/before medians range 0.9901–1.0211, below
1.05. Scalar-loop controls at zero/64 visits have zero allocation slope.
All twelve existing architecture, catalog, routing, inventory, warning,
boundary and formatting commands pass on the measured implementation.

The full GPU component/linked suite and generated runtime-only application
builds have not been rerun for this runtime change. Their previous passes
retain their original revisions. No larger input, block/window boundary,
full-corpus result, green CI, merge readiness or clean current-head review is
claimed. The PR remains draft. Setup failures, the cancelled broad build,
zero-test selection and corrected diagnostic assertion remain documented in
the raw receipts and `setup-outcomes.json`.
