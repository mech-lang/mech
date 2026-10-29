# Round 13 checkpoint: private mixed numeric results

Base: `1087e7b8160894f542b81d46b887eb32a450125c`.
Production baseline: `d105f884bc79d464a205b3022da4abdc00da41aa`.
The three write-specific acceptance checks from the new brief were already
completed and published in the round-12 closure. Do not duplicate that package.

The direct regression executes on unchanged production libraries under the
existing `unit-qualification` Cargo profile. Both original and decoded artifacts
produce correct arrays and current terminal scalars through the maintained path.
Object inspection rejects the generic child at header16 / parent0 and unchanged
`huff-assign-symbols` at header42 / parent0 with exactly:
`leaf result is not a scalar or a current numeric array`.
The observed-transport test passes; the native-selection regression fails for
both sources and artifact forms. Actual execution and exact library/binary
hashes are in `direct-baseline/`.

The generic terminal returns i +17 rather than its carrier lane. Its ordinary
parent invokes it twice with changing arrays and scalar starts. The real witness
uses unchanged `huff-assign-lengths`, which consumes the returned i to select
the next run of symbols. Each checks every output cell independently.

A preliminary conditional prefix/cumulative witness also reproduces the result
restriction and measures its slope, retained in `initial-conditional/`. The new
brief correctly excludes predicate correlation from this patch. It is retained
as a secondary post-change decline observation, not the acceptance target.

Implement a typed scalar/direct-array/private-flat-tuple recipe using the existing
exact nonescaping frame fields. Admit exactly one current unsigned-array owner
and supported scalar fields; keep records, multiple arrays, nesting, escapes,
growth and child native regions unsupported. Validate return freshness on every
reaching path and synchronize actual terminal frame lanes after freezing private
cells. Reuse pre-admitted BufferFrames storage; do not pin another owner before
the first consuming store. Extend the shared JIT/Object and independent image
validation. Preserve all selected construction bounds, source failure order,
finite accounts, aliases and publication atomicity. Keep compressor source and
allocation/timing boundaries unchanged. IR/work identities must describe the
new proof and bounded result synchronization.

Acceptance includes parented generic and unchanged source selection, zero calls
on already-private successful backedges, lost per-element allocation/visit slope,
current terminal values, no-op/empty/short/shaped cases, stale-result rejection,
fresh invocations, finite-fuel observation, actual linked recovery and restored
image corruption. Retain prior write contracts and scalar controls. Repeat the
same eight-byte complete pair and separate profile, then commit the full packet
before the next optimization. Larger inputs are not queued. The historical
round-10 decoded-U64 1.0526036 ratio remains unresolved.
