# Round 03: immutable schema retention summary

Declared before implementation and timing. Starting published compiler:
`fb9c64ab1500b165dc79c0ed4464ffad6467dbbf`. Retain the round 02 executable
`a98a68366c1d3f807f5ab0ff78a105b72244d3a14f1da8bbcf7aa8ab0e484e6f` as the
before subject. Its numeric implementation is `335de1c16`.

The [profile attribution](../results/2026-10-28-native-hardening/schema-bound-analysis/README.md)
finds 373 of 1138 main-thread samples inside repeated full-schema allocation
bound calculation. This is runtime overhead in scalar result construction and
buffer/snapshot admission, reached throughout the real compressor. The actual
compressor algorithm, 25 source components, tables, limits, buffers, allocation
sites, six Huffman initializers and timing boundaries remain unchanged.

Add one private immutable `Option<u64>` summary to SchemaTable. Derive it with
the existing checked visitor whenever a new entry arena is constructed. Clones
copy the summary with the same immutable entries; extensions and rooted/component
closures derive a fresh summary. Overflow remains None and never rejects a
previously constructible table earlier. No public caller can supply the summary.
Only the no-callback query reads it. The ordered callback/precharge method still
walks every contribution and preserves first-denial behavior. This is layout
metadata, not permission to execute, reserve or publish.

The inline field has a real metadata cost. All bounds must include its concrete
size through the existing size_of::<SchemaTable>() accounting. Never subtract
the cache from retention charges to preserve an old numeric receipt. Public
wire bytes, schema IDs and keys remain unchanged. Exact memory comparisons
are against the full uncached visitor for the NEW concrete layout. Logical work
may increase where that true metadata size is charged; report the exact before
and after totals and the reason, without changing runtime allowances. No node,
payload, clone or finite-denial check is removed.

Acceptance: all table constructors and clones, independent full traversal,
ordered callback denials at every contribution, canonical schema identity,
source/decoded artifacts, immutable aliases, exact/one-short import admission,
cleanup and retry. Run existing pertinent schema/snapshot and native component
regressions unchanged. Check no_std/runtime-only profiles and maintained
architecture/format/warning guards; no new compiler/runtime owner or route.

Use the same unmodified eight-byte timing harness, source/artifact and pinned
mimalloc object as round 02. Refresh 20000 reference observations. Run before
and after with one warmup, three retained calls and one excluded extra call,
without another local compilation or compressor. Compare with Roc = 1x; retain
the round 02 2494.04x historical denominator separately. Keep every observation
and failure. Exact raw bytes, independent inflation, aliases, capacity13 and
zero terminal managed bytes/nodes must pass. Compare allocation requests/bytes
with the same real-buffer diagnostic, explicitly including the added schema
metadata. Do not time a larger input.

After primary timing, use a separate process and two-second sample with enough
repetitions for about four seconds of execution. None of those observations
enter the primary median. Confirm the targeted walk leaves the hot path and
record remaining costs. Commit this round's code and measurement before another
production optimization. A regression is evidence to retain, not a reason to
silently change the cohort or omit results.
