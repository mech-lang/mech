# Round 02: packed single-assignment planning

Compiler: `335de1c16743343fa6745d7ed2e638f6f1626fa9`.
Before: reviewed `376cabbe2d204e9f12ad3b52c9f26b56dd6988ec`, whose Rust
implementation matches the retained c24984b40 executable. The
[protocol](../../../optimization-rounds/packed-single-assignment.md) precedes
implementation and timing. No compressor component source changed.

On Apple M1, **Roc = 1x** for the eight-byte level-12 input `abcdefgh`:

| Subject | Median seconds | Divided by fresh Roc |
| --- | ---: | ---: |
| Pinned Roc, 20000 observations | 0.000305 | 1x |
| Before, retained executable | 1.676382750 | 5496.34x |
| After, new compiler/runtime | 0.760682250 | 2494.04x |

All three retained observations per Mech executable remain in `results.json`.
Each process also retains its excluded warmup and extra diagnostic call. The
after/before ratio is 0.453764. This is a small diagnostic, not a statistical
campaign or full-input result. The previous round's 5756.19x anchor used a
0.000290-second reference median; the fresh before/after rows here share the
new reference denominator. The 1x goal remains unachieved.

Both Mech executables and the reference use the same pinned mimalloc 2.1.7
object, without allocation tracking. Source artifact, external output capacity
13, real tree/cache, six deliberate Huffman initializers, algorithm, tables,
search/EOF limits and timing boundary are unchanged. The harness source hash is
`8595dfc8b529d5ad06b57580314e3fe4fc6110bd161ffa768a3d59e6c6608fd2`;
it is byte-identical to round 01. JIT, output reserve/clear/rebinding, result
checks and disposal remain outside timing. No other local compiler or old
compressor was active. Normal user/system activity was not isolated.

The before executable is SHA-256
`f3d665c649ebe49442f56fd13d04edf808b5b32bbb4eae81e41bd9e862a0c74b`;
after is `a98a68366c1d3f807f5ab0ff78a105b72244d3a14f1da8bbcf7aa8ab0e484e6f`.
The source is `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
and artifact is `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Every call preserves **939091080723** reported logical work, exact ten raw
bytes, independent inflation, input aliases, capacity, and complete managed
cleanup. The observed immutable output handoff still changes physical backing;
that cost remains included.

The optimization replaces the full destination planning walk for a validated
single-destination/single-source packed unsigned assignment. It derives the
exact bounded primitive source fee using the maintained visitor, previews the
total charge through the existing meter, and prepays only when it fits. Tight
budgets and unsupported cases retain the original traversal and first-denied
charge/prefix. Copy construction, copy admission, ownership, equality and
publication are unchanged. This is general resident/native library machinery;
there is no compression intrinsic or competing executor.

Three optimized engine tests pass, including every compute cap across the
incremental/fast boundary for all five unsigned widths, scalar/packed sources,
different destinations and selector traversal costs. Live work-ledger tests
compare exact errors and spent prefixes. Unsupported kinds and overflowing
summaries decline without spending. The build took 1082.404 seconds; those
tests ran in 0.01 seconds. The GPU unit filter selects zero and is not counted.
The [qualification follow-up](qualification/REPORT.md) records 58 distinct
passing tests including source/decoded recovery and linked assertion ordering.
It preserves two failed new-fixture attempts, their correction to the actual
scalar write, and a corrected test-file layout violation. Matrix-source caller
forms remain separately unqualified; no compressor operation depends on them.

The unchanged real-buffer witness also passes all 24 observations. At the real
sizes, eight stores take about 3.776 ms for U16 and 21.320 ms for U32. Its
allocation requests, requested bytes and reported work equal the retained
historical probe: U16 uses 9913 requests/5302384 bytes, U32 uses 9913 requests/
49124496 bytes. Physical buffer copies remain. Historical probe times were
105.141/590.437 ms, but that older executable includes other earlier compiler
differences; the complete-program table is the matched compiler comparison.
This witness uses System plus tracking and is separate from goal timings.

After all primary observations, a separate process performs six repetitions
(plus warmup and the harness's extra call). All are excluded from primary
statistics. A two-second sample captures 1138 main-thread samples and actual
native execution. Its physical peak is reported by sample as 774.1M, including preparation/JIT;
that is not a managed-runtime peak. The former full destination walk is no
longer the dominant visible stack. Remaining samples include schema clone-bound
walks, ownership witnesses, call planning/binding, resource checks and managed
transport. Deduplicated symbols still need resolution before assigning exact
fractions or choosing the next implementation. The raw profile is preserved.

The retained C near-optimal and Rust miniz compression functions were inspected,
not rerun on a large input. Both contain direct indexed 16-bit load/store
sequences (C at 0x100021fd0; Rust at 0x10000fe58), which illustrate the target
shape of efficient numeric updates. They do not implement Mech's admission and
immutable-alias contracts. Rust uses a different compressor/level, so this is
implementation-pattern evidence, not a language-speed comparison. Exact binary
hashes, disassembly commands, excerpts and compressed bodies are retained. The
Rust symbol-name filter initially yielded no instructions; bounded-address
disassembly fixes inspection without changing either binary.

The first direct probe link selected an old GPU rlib after the engine unit build
had refreshed its dependency; it failed with E0463 and produced no executable
or timing. Building the maintained GPU example refreshed the ordinary runtime
library in 77.566 seconds, then both exact probes linked successfully. Failed
and successful build receipts and actual library hashes are retained.

Only eight-byte complete compression was timed. No larger input, boundary or
full-corpus run was started. CI and broader qualification do not substitute for
this measurement; neither parity nor merge readiness is claimed.
