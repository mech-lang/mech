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
- Canonical component rerun on clean implementation 710e82a: **143 passed,
  0 failed, 0 ignored, 129 filtered**, 355.06 seconds of test execution and
  828.76 seconds including the 7m49s rebuild. Expected values and allowances are
  unchanged from the linear-stack command. No full-compression timing is inferred.
- The new managed-memory CI step is separately committed as
  `d0271b90ae51db78630cda2a76c01c61a75719da`. Its exact shell body runs locally
  with `--all-features`: nonempty inventory gates confirm **1 allocation case**
  and **6 new ownership cases**; execution passes **1 allocation case plus 13
  ownership cases** (including 7 existing String cases), 326 other unit cases
  filtered. This schedules the tests without weakening the warning policy.
- Workflow contract tests: **90 passed**. CI inventory tests: **42 passed** with
  the existing PyYAML 6.0.3 CI environment. The live integration inventory reports
  **zero gaps**. Formatting, native layout, warning policy, resident routing,
  source-catalog entrypoints and the working source whitespace check pass.
- Fresh reviews of #937 at 114eef487c, #938 at 304a1382c, and the storage code in
  #940 at 62860ea623 report no major issues. The later CI scheduling/receipt
  updates are not misrepresented as part of those reviewed code heads. The
  native GitHub stack has 25 PRs with all 24 adjacent base/head edges matching;
  the local series has zero merge commits above architecture cleanup a68fb2e.

Runtime-only host compilation (`canonical-native-program-runtime`) passes on
clean d0271b90a in 15.56 seconds. Core `--no-default-features --features no_std
--lib` compilation also passes in 2.44 seconds; its only dirty paths are receipt
and report updates. No compiler/runtime source changed after implementation
710e82a. Full linked/AOT qualification and a completed remote CI run were not
rerun for this storage prerequisite. No complete CI result is claimed or awaited.
The new #938/#940 CI runs were canceled at the user's direction; #929 and the
foundation were left untouched.

## Failure disposition

The experimental partial core profile `program functions string u8 u16 u32 u64`
fails unit-test compilation with 34 diagnostics: unguarded nalgebra/F64 test
requirements and unused imports under warnings-as-errors. The exact command on
clean base 304a138 reproduces the same diagnostic headings. This is an inherited
test-feature combination failure, outside this storage change. The complete
minimal/full profiles above pass; warnings-as-errors remain enabled.

The first CI-contract discovery attempt used the wrong directory and exited 5
with zero tests; correcting it executes 90 tests. The default Python environment
also lacks PyYAML for the inventory checker; the existing dependency-equipped
CI interpreter executes all 42 tests and the live inventory. These were command
selection/environment failures, now resolved; no empty run is called a pass.

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

Raw command logs are stored as deterministic `.log.gz` files; decompress them to
recover the exact captured bytes. `raw-log-encoding.json` records each original
filename, byte count and SHA-256. This preserves trailing whitespace/blank lines
from the test harness and diagnostics without introducing new patch whitespace
failures. The first uncompressed receipt commits exposed six such formatting
findings; compression resolves this introduced receipt-only problem. Source and
workflow diffs remain clean. JSON command receipts retain their original live
log paths as provenance, rather than pretending the commands emitted gzip files.
