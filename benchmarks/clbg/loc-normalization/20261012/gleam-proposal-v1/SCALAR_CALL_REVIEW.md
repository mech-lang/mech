# Selected callable grouping review proposal

Source-only manual findings, with no parser/normalization/hash/fixture replay.
This file proposes how to resolve current diagnostics; it does not waive them,
authorize counts, or change the unreviewed grammar. A later semantic receipt must
bind exact prepared source/helper bytes, the diagnostic/AST span and resolved
callee definitions. Source changes invalidate that receipt. No path-based count
exception or estimated LOC is proposed.

Both nbody mains have the exact grouping `{ squared *. io.sqrt(squared) }`.
The imported io alias resolves to the counted clbg_io.gleam declaration whose
external target is Erlang math:sqrt and whose signature is Float to Float. There
is no user callback argument or nested body in this grouping. The source writes
one arithmetic expression; propose folding its written grouping into the outer
initialized magnitude owner after exact binding/FFI and scalar result review.
Keep the generic sqrt declaration and external annotation in each compiled bundle.

Both K mains have `{ width == 6 || io.byte(line, 6) == 32 || io.byte(line, 6) == 9 }`.
Their io.byte declaration resolves to binary:at, taking BitArray/Int and returning
Int. The surrounding short-circuit expression stays exact, including the width
check; it is not transformed into an eager byte read. Propose treating this as
the scalar Boolean operand of the containing let, with both calls retained in
the owned expression AST. No executable block body or callback is hidden.

Optimized K also has `{ key * 4 + code(io.byte(sequence, index)) } % modulus`.
The local code function contains the already separately owned case alternatives
65|97, 67|99, 71|103, 84|116 and explicit invalid-nucleotide panic. It does not
perform I/O, mutate a map or receive a callback. io.byte is the same exact external
scalar read. The complete code definition/case/panic remains counted, and its
runtime branch behavior remains unchanged. Propose composing this call's returned
Int in the arithmetic initializer after exact local-callee binding review.

Both K frequency functions evaluate `list.sort(dict.to_list(counts), compare)`
before their directly nested `fn(entry)` passed to list.each. The conservative
prefix gate correctly retains this as unresolved. These are ordinary value
arguments in one written statement; the named compare function's complete case
body is independently counted at its definition, and vendor sort/dict internals
are excluded under the agreed generic-library boundary. Propose the same first
direct callback prefix composition only after resolving these exact standard
library calls and the named function reference. It must retain the real callback
header/body/close inventory and all preceding independent statements. A separate
write/let/control block or an additional callback cannot receive this waiver.

The fixture packet currently has no mechanism to consume such a source-bound
semantic approval. It therefore continues to diagnose these forms and returns
null normalized counts. After root approves a concrete binding/purity rule,
author it as a separate grammar revision with selected positives, changed-callee
and shadowed-name negatives, mutating/unknown call negatives, and prior fixture
receipt prerequisite. Preserve this entire v1 packet and any v1 terminal evidence.
