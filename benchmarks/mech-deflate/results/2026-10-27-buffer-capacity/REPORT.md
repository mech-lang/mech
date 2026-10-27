# Packed buffer capacity and consuming ownership

Base: `304a1382c1d41586bb49dc1d9e6e7120d7ebadae` (#938, above the linear
architecture stack). Storage implementation:
`710e82ac685d637cc2e01afe854bf41549f3410d`.

This is the generic storage prerequisite for growable path-node/output buffers.
It does not register source operations, change native code generation, assemble
the parser/writer, or claim faster full compression. The six permanent collector
programs, reference cases, tables, limits and timing boundaries are unchanged.
The previously completed 320-position block remains the slow baseline.

## Concrete source requirement and first failure

The pinned level-12 parser's `CompressOptimal.roc` lines 72–76 and 720 reserve
305257 U64 path slots, then clear at line 429 and append at lines 436/439/503.
Unused capacity is not initialized as a matrix. Output reserve at line 707 is
`n + 5 * max(1, ceil(n / 5000))`; for the 6627202-byte input that is 6633832 U8
slots. The output allocation is retained across repetitions. The pinned source
revision remains `671077457ff7ef3a3d44caf715a3fc595addf379`.

A canonical-source witness using proposed `sequence/reserve`, `sequence/append`
and `sequence/clear` operations rejects at the first call with
`source-semantics/unknown-function`: reserve has no declared source semantics.
The failed receipt is from base 304a138 with only the standalone Rust witness
added; production compiler code was unchanged. The source and exact acceptance
harness are retained here as pending witnesses, outside the passing test suite.
They are not claimed implemented. The next patch must turn this witness into
resident/original/decoded native agreement, with a private lexical lifecycle that
can prove last use and actual same-backing reuse. It must also exercise aliased
inputs, an unselected path, failure without replacing prior publication and retry.

## Storage design

Four existing packed unsigned families (U8/U16/U32/U64) use Vec backing so that
logical length and physical capacity can differ. Views, shape/cardinality,
encoding, equality, hashes and receipt tokens see only initialized elements.
The retained-footprint calculation and incremental work visitor both account
for the entire physical capacity. The ordinary boxed-buffer constructors remain
available and preserve their exact-sized backing without payload copies.

A new detached buffer constructor reuses the existing checked numeric matrix
constructor. It validates exact scalar kind, schema and logical extents while
retaining spare capacity. It grants no runtime admission or source execution
authority. Managed callers must admit storage before allocating it.

A consuming transfer accepts only plain packed unsigned matrices with exclusive
wrapper and data owners in the same managed account. Ordinary aliases, imports,
physical payload tickets, wrong accounts and constrained kinds stay immutable.
The existing String transfer and this new family share the same private root
handoff: retire the old frozen identity, unlink its budget registration, preserve
its existing capacity claim, and retire shape/token/node metadata before reuse.
A later freeze has a fresh identity. Compiler liveness remains a separate duty;
this core API does not decide that a source value has reached its last use.

## Observed validation

- Minimal core profile: **81 passed, 0 failed, 48 filtered**, including all six
  new ownership/capacity tests and the existing snapshot regressions.
- Full core profile: **82 passed, 0 failed, 254 filtered**, including the existing
  physical-payload-ticket rejection through the shared handoff.
- Separate allocator-probed Rust consumer: **1 passed, 0 failed**. It compares
  16-slot controls with the actual 305257 U64 and 6633832 U8 reservations over
  three clear/append/refreeze rounds each. Every round requests **13 metadata
  allocations / 856 bytes**, independent of payload capacity. Payload pointers,
  original account charges and capacity remain unchanged; final usage is zero.
  These are core storage calls, not a compiled-source compression result.
- Canonical component rerun on clean implementation 710e82a: pending; retain the
  terminal receipt before claiming a result. Expected values and allowances are
  unchanged from the 143-test command at the linear-stack checkpoint.

No complete CI result is claimed or awaited.

## Failure disposition

The experimental partial core profile `program functions string u8 u16 u32 u64`
fails unit-test compilation with 34 diagnostics: unguarded nalgebra/F64 test
requirements and unused imports under warnings-as-errors. The exact command on
clean base 304a138 reproduces the same diagnostic headings. This is an inherited
test-feature combination failure, outside this storage change. The complete
minimal/full profiles above pass; warnings-as-errors remain enabled.

The first witness harness attempt used the reference helper's wrong signature;
that introduced test scaffolding error was corrected before the actual frontend
witness ran. The first core test build also exposed the existing encoding test
macro's Box-specific constructor, which this Vec representation changed. The
macro now accepts the expected storage with `.into()`; all original values and
byte expectations are preserved, and both snapshot profiles pass. Raw failure
receipts are retained alongside the corrections.

The inherited fixed-budget collector work/account denials remain visible in
#938. No global limit or qualification allowance changes here. No outstanding
storage correctness failure is known; executable source buffer operations,
compiled ownership proof, optimal parsing, writing, compressor orchestration and
complete raw-byte/decompression validation remain incomplete.
