# Public finite-policy execution

Implementation: `c24984b402a1228cccf934a478965143b539f611`, following typed
frames at `2b4adb7289cb81ffd0c37456ce1f968c9ed3d43d`. Source/decoded generation
receipts predate the implementation commit and include their dirty-file
inventory. Those edits are committed in c24984b40. The generation example uses
ordinary `main`, without the test harness's 32 MiB compiler-thread wrapper.

Both public generations succeeded. The source frontend took 56.230/57.687 s;
complete Object/application generation took 94.188/96.108 s. This disproves a
public stack-overflow concern for these entry points and this machine; it does
not prove all programs fit every host stack. Exact commands and build errors
are retained. Compilation policy is finite and explicit; generic defaults and
diagnostic unrestricted mode are unchanged.

The maintained source has 160992 bytes, SHA-256
`345324ce551aaa2485dcd0fe4262da96e660d6b31f5f772bff89b034e9b1997f`.
Its revision-14 artifact has 4140067 bytes, SHA-256
`a2daaaf746d6bcc8668c5439f84a782ecea2aff376546566c01d4fc0f8c6f585`.
The 25 components, tables, real tree/cache, six Huffman initialization sites and
complete-program entry remain pinned. This example uses the existing complete
entry's 6633832-byte output reservation. The separate borrowed-output timing
harness is not substituted for this public source qualification.

The original and decoded forms produce byte-identical source, bytecode, Object,
descriptor and identity. Object file size is 2999872 bytes (2999140 code bytes),
SHA-256 `08ae7972b06ba71e6d6e07d17a744b6d12d0b05ac549651d3e7bcfe2e4e55353`.
The program uses 100256 generated blocks, a 120170976-byte definition-table
estimate, and 416 native stack bytes, within the documented finite policy.
Definition-table admission is not a total compiler-memory bound.

Generated debug runtime applications build with the pinned toolchain and
`--locked --offline --frozen -j1`. Full Cargo compiler-artifact messages and
extracted package/feature inventories are retained, not merely manifest checks.
Both closures exclude the parser, bytecode producer, source compiler, Cranelift
and JIT. The generated executable SHA-256 is
`bc20be6ea0ae1f00d3273974002ec99dc3a714589952cfb58b48833de17abb13`.
Independent runtime-image restoration is exercised by each actual launch.

Each deployed form executed `eight-literals` under the checked-in finite launch
policy. Both returned the exact ten expected raw bytes, and Python zlib inflated
the actual output back to `abcdefgh`. The shell drops inputs and outputs and
requires zero managed bytes/nodes. Whole debug-process times were 65.016 and
63.358 s, including restoration, import, execution, export and cleanup. These
are qualification observations, **not compression benchmark timings**; the
original launch also overlapped a compiler check. Do not compare these times
with the optimized evaluation-only baseline.

Only one of nine cases was selected per deployed form. The receipt explicitly
sets `complete_small_corpus: false`; ignored or unselected cases are not passes.
The other deployed cases, final-head complete-program original JIT, block and
window transitions, repeated slides and full-corpus agreement remain separate
gates. Historical results retain their own exact revisions.

The first example build found an introduced example-only String-to-Arc
conversion error, corrected before the successful build. The first CLI unit
command disabled distribution features required by its existing source build
tests and failed in existing gated imports. The supported default-distribution
build succeeded, but the initial filename-based test filter selected zero tests;
that is not a passing test receipt. The corrected module filter executed all
six CLI tests successfully, including strict compilation-policy JSON, retained
source/decoded provenance and early rejection. Both attempts remain visible.
No clean-parent minimal-feature compilation is claimed for that failed profile.

The supported public API regression separately checks zero-block rejection
before filesystem publication, source/decoded generation and byte-identical
default Object/identity behavior. The managed launch-policy unit test verifies
all finite dimensions are required and rejects compilation-policy fields.

See [the maintained commands and policies](../../../public-application/README.md)
and `manifest.json` for all file hashes, execution receipts and actual runtime
dependency closures. CI is cancelled by user instruction. These local results
do not establish green CI or merge readiness.
