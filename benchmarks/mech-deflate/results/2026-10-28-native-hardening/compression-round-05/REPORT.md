# Round 05: constant-time packed payload lengths

Starting published head: `615a39c5b8abb966ae7b3f85682068e024ef5efa`.
Protocol: `7ffb113e5239e24fd34822d65161443df042919a`.
Implementation and tests: `ced2c71c36d002a044b40f1f5cfb8ae293e92572`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

The existing private snapshot sink protocol now has a sequence operation. Its
default runs the unchanged element encoder. The length sink specializes packed
fixed-width kinds to saturated count-times-encoded-width plus the preceding
length. It uses initialized length, including after clear with retained capacity.
Strings, Unit and composite Values keep their original traversal. Byte, fixed,
hash and erased sinks retain their bytes and write order. Resource visitors,
ordered admission, value layout, canonical format and execution routes do not
change. No payload scan is needed to recover the U16/U32/U64 buffer size.

The reference binary's compression entry loads list pointer/length together;
the pinned ABI places length in the second word. This supports the metadata-load
pattern; it is not evidence of an equivalent standalone size benchmark. Before
Mech disassembly shows per-element saturating additions. After disassembly and
the decoded jump table show every fixed-width numeric kind entering one common
saturated multiply/add. String/composite fallback loops remain, including emitted
but unreachable Id/Index fallback code. Exact binary hashes, command arguments,
ABI excerpt, jump table and assembly are in `timing/`.

## Small complete-compression comparison

Apple M1, eight-byte input `abcdefgh`, unchanged level 12, **Roc = 1x**:

| Subject | Median seconds | Divided by fresh Roc |
| --- | ---: | ---: |
| Pinned reference, 20000 observations | 0.000285 | 1x |
| Before | 0.492213292 | 1727.06x |
| After | 0.464984292 | 1631.52x |

Elapsed time falls **5.53%** (1.0586x faster than this round's Mech baseline).
The previous published 1692.80x result used a different reference median and
cohort; this round reruns the retained before executable against the same fresh
reference as the candidate. Three retained calls per subject, one excluded warmup
and one excluded extra call are all archived. The separate two-second sample and
its nine profiling repetitions are excluded. No compiler overlapped primary or
protected scalar timings. No larger input was timed.

Before executable SHA-256:
`929f96c685c155f632a0b099660c4c5246e05667f389898063129b3c7247f1ae`.
After executable SHA-256:
`8ad35661bca41e0a3ab1c79bbe006c94b0474f8782d222b27050e5d4835ec33c`.
The selected catalog, pinned mimalloc 2.1.7 object and evaluate_into-only harness
are unchanged. Exact raw bytes and independent inflation pass on every call,
as do retained aliases, capacity 13, work **941491317987**, and zero terminal
managed bytes/nodes. The same source-derived and decoded artifact agrees exactly
(4138627 bytes). Its source wrapper is 160857 bytes; all 25 maintained components,
algorithm, tables, search/EOF limits, buffers and six deliberately unused Huffman
initialization sites remain unchanged.

## Mechanism and remaining cost

All 24 real-buffer allocation-witness observations pass. Eight stores still
request **8833 allocations**: 5177904 bytes for the 262144-entry U16 tree and
49000016 bytes for the 1500513-word U32 cache. Logical work and these allocation
counts/bytes are identical to the prior retained witness. That earlier witness's
U16 median is 0.0032405835 s versus 0.0006362915 s now; U32 is 0.018606042 s
versus 0.0032487505 s. These are instrumented System-allocator diagnostics, not
fresh paired primary timings or an isolated store-versus-reference comparison.
Full payload copies and substantial surrounding metadata allocation remain.

The new profile has 1344 samples. Named length-encoder exclusive samples fall
from 59 in the previous 1355-sample profile to one. Allocation/free, memory
copying, budget checks and TLS remain visible. Sampling is diagnostic; no
unsupported attribution is made for deduplicated or JIT addresses. The next
priority is consuming scalar indexed replacement, direct scalar operands and
changed-bit calculation, then ownership through actual recursive tree/cache
carriers. Another small metadata optimization will not substitute for this.
Application rewrites will be separate source revisions and comparisons.

## Qualification and disposition

**248 Rust tests pass:** 98 core schema/snapshot tests, 47 memory-runtime tests,
23 memory-safety tests, 69 native component tests, eight native scalar tests and
three native bit tests. Core checks use `--no-default-features`; memory tests add
`functions,u8,u64,f64,bool,string,matrixd`. Native consumers use the unchanged
selected-operation release profile. Exact commands and actual linked libraries
are retained. The component wrapper only resolves absolute paths and excludes
the linked subprocess module; maintained bodies, values and allowances do not
change. The all-primitives scalar test remains excluded because not/xor are not
in this profiling catalog.

Packed-kind byte/length/hash oracles include every integer width, raw floating
and complex bits, rationals, Bool, Id/Index, nested values, strings, empty arrays,
spare capacity and saturation. Existing tests retain exact/one-short admission,
aliases, failure atomicity, cleanup and retry. The no_std check and all twelve
architecture/catalog/routing/inventory/warning/boundary/format checks pass.
Protected original/decoded Bool/U64/I64/F64 controls retain exact allocation
counts/bytes and pass the predeclared 1.05 timing gate (ratios 0.9678–1.0155).
Zero/64-visit scalar loop allocation slopes remain zero.

One initial new-fixture build fails with Rust E0502: the test retained a mutable
borrow while invoking the immutable encoder. That test-authoring error is fixed;
its failed receipt remains. Snapshot/memory/no_std tests ran on the exact two-file
patch before commit; `source-identity.json` records the committed file hashes.
Release/static/artifact checks and final native tests use the clean implementation
head. No executed production failure remains.

The focused release rebuild takes **1007.38 seconds**, rebuilding all four
machine libraries because mech-core changed. This is an iteration rebuild, not
a clean-build comparison. Broader linked subprocess/generated runtime-only
applications, final block/window transitions and the full corpus are not fresh
passes for this change. Historical unsupported caller forms, decoded U8-add and
whitespace findings retain their disposition in the prior packets. CI is cancelled
by request; #973 remains draft and unmerged pending review of the new head.
Parity is not achieved. No cancelled long campaign was restarted.
