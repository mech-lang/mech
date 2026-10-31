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


The implementation now selects the generic parented child and unchanged
huff-assign-symbols. Before/after 256-step System/probe observations:
9251/646577 bytes/9811 visits →1619/172265/51 for the generic pair of calls;
4918/303186/9651 →340/52138/108 for huff-assign-lengths. Positive iteration
counts have flat allocation and ordinary-visit costs. These are not complete
timing results. Prototype JIT/Object, terminal scalar kinds/bits, aliases, exact
rectangular shapes, stale/escaping/multiple returns, short reads, ordered late
errors, secondary cumulative decline, restored metadata and actual linked
recovery pass. The first broad new cohort passed13/15; two fixture assumptions
were corrected and both reruns pass, plus the read-only mixed-result case.

The output-bound fixture had replaced one NoLimits dimension with None, creating
an invalid partial policy. It now preserves the established policy except for
the deliberate finite ceiling. The finite retry fixture's resident oracle
reaches65888 comparisons against its inherited65536 limit for two64-step calls;
that allowance is unchanged. It uses the already established independent model
to verify every output word and terminal scalar instead. These are new test
setup failures, not production regressions.

All four actual runtime units pass, with16 real mutated-prefix exits in each
of the old write and new mixed-result cases. The new normal-exit observer uses
the existing audited callback and records no production execution changes. Its
first fixture had an unsupported private-use path; its normal outer tuple
consumer now qualifies. Subtracting from complete work initially failed later
in public canonical output construction. The test now observes actual native
exit work and withholds traversal fuel there. No existing allowance or check
was relaxed; initial/diagnostic failed attempts are retained.

IR118/work policy93 adds an explicit Scalar/Array/Frame result recipe and bounds
terminal-field synchronization in existing admitted lanes. No Core/Engine
ownership change, public aggregate ABI, canonical-bytecode revision or source
rewrite is introduced. Existing broad component/scalar/static controls and the
separate complete timing/profile are the remaining acceptance work.
