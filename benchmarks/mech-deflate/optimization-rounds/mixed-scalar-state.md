# Round 08: mixed scalar state in the existing native aggregate layout

Starting published head: `8c98f46506e1baf9568cff2a395452f21acc1b07`.
Before executable SHA-256:
`97aa2b05a99ddc3efecc0fe04cda64a817bb8ef21bee0f8d61375e715b00d3d8`.
The latest paired eight-byte result is 495.21x Roc (fresh reference 0.000284 s).

The new profile retains repeated runtime visits, admission, allocation and data
movement after removing most schema traversal. The existing private scalar
aggregate layout accepts only U64 leaves. The actual compressor carries signed
(I64,I64) default-frequency state and U64/U32 optimal-path enum choices, so those
loops cannot select its existing word representation. First establish this miss
with direct signed-pair and mixed-enum reproductions against the clean parent.

Generalize the existing flat tuple/enum eligibility to the shared native scalar
vocabulary, using exact schema/kind equality and existing bit transport. Preserve
whole-connected-component escape analysis, 32-field limit, guarded enum payloads,
unsupported-form fallback, original selected operation evidence, shared native
emission and independent image derivation. Do not add a source rewrite, format,
operation, executor, runtime region, account authority or per-operation allocation.
Record support and nested aggregate generalization are outside this change.

Require original/decoded JIT execution and Object agreement, exact unsigned high
bits, signed extrema, float payload/negative-zero bits, fresh changing Boolean
state, overlapping/permuted copies, constants and variant tags. Establish zero
per-visit allocation slope for the selected scalar-only state. Keep observable
aggregate outputs canonical, preserve tight resource errors, lexical assertion
and arithmetic precedence, prior publication, cleanup and retry. Exercise the
actual permanent default-frequency and optimal-path components without changing
expected values or resource allowances. A failed eligibility assertion is the
expected initial red result; fixture/setup failures are recorded separately.

After implementation run the same pinned eight-byte paired timing and separate
profile, same allocator/catalog/artifact/source/timing boundary, with no competing
compilation. Check exact bytes, independent inflation, aliases, capacity and
cleanup. Retain logical debit changes explicitly; do not treat work as machine
instructions or weaken finite admission. Keep protected Bool/U64/I64/F64 timing/
allocation gates and existing component coverage. Archive every observation and
failure, commit before another production optimization, publish to draft #973,
request review and cancel CI. No larger complete timing is authorized here.
