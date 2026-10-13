This additive, unsealed revision repairs metadata handling of the unchanged
official Crystal archive. The input-v3 phase passed its ten mocked checks once,
then stopped during its metadata seal: `Path.resolve()` followed the official
foreign-platform `lib_c/x86_64-linux-gnu/c/linux` circular symlink. It emitted
no proposal and launched no new compiler or benchmark. The successful SDK
metadata remains preserved. The failure journal accurately labels its error
text as a concise transcription; it does not claim an uncaptured full stderr
log or any native failure.

`proof_reuse_v4.py` changes only archive correspondence. Each regular payload
must match the original archive hash and file type. Each symlink must have the
exact original relative link text and a lexical target within the private
release root; checking it never follows the target, so official in-root cycles
are permitted. Absolute or escaping member/link paths, substituted symlink
parents, changed bytes, and changed hardlink topology fail. The unchanged
official archive digest still anchors all member metadata. The run checks
archive correspondence before new native work and again before final bounded
qualification, alongside the existing regular-file pins.
An inventory walk does not follow directory symlinks and rejects extra or
missing release entries, including added source files.

Every other source/binary/command/runtime/input/output/observation reuse
function is unchanged from v3. Metadata sealing still independently reconciles
the exact fifteen previous binaries and 112 real bounded records, and never
copies them into a new execution ledger. The all-ten input preflight and sole
zero-byte `empty-input` repair remain unchanged. The existing 12+12+8 and v3
ten successful fixtures are reused without rerunning them. Five fresh builds,
44 missing bounded output cases, and the pending generic/work proofs retain
their original scope; language source bytes, algorithms, SDK and compiler
flags do not change. No full timings, normalized LOC or chart admission are
authorized by this proposal.

Six proposed metadata-only mocked methods cover exact in-root cycles, changed
link text, member path escapes, link escapes, payload/type/parent substitutions,
hardlink topology and exact AST equality of the previously proved functions.
The hardlink method also rejects an extra release source file.
The standalone common-lock entry point first writes an exclusive v3 failure
journal, then captures the exact six-case log and receipt. A fresh seal uses
`proposals/private-input-archive-v4.json`; the v3 files, diagnostics, receipts
and old release are preserved. No v4 fixture, seal, compiler or benchmark has
run yet. Root must review the concrete sealed digest before a native run.
