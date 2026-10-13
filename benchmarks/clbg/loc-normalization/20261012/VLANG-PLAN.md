# Separate V report-only normalization proposal

This is unhooked and unqualified. It does not modify the frozen
`jobs/boundary-audit-01` modules and does not add V to any chart candidate pool.
The existing whole-pool fixtures/audit and their source-specific review run
first. V fixtures execute later under the same common timing mutex, with a
preserved receipt, before any V LOC is used in a sealed comparison report.

API: `vlang.normalize_vlang(source: str, language="vlang")`. The result contains
physical and proposed canonical LOC, exact offset/line evidence, diagnostics,
review status and `chart_pool_eligible=False`. The existing generic normalizer
continues to reject unsupported V; no `v` lexer alias is used, since it identifies
Verilog in the installed lexer package. The separate V masker recognizes nested
comments, quoted/raw strings, escaped rune literals and nested interpolation.
Statement-bearing interpolation is an explicit review gate.

Use the approved header1/open0/body/close1 rule for functions, imperative
controls and error-handler blocks. Independent bindings/semicolon statements
are separate; iteration headers, typed literals and tuple multi-return bindings
are structural. Existing literal data wrapping remains recorded physically.
Type declarations retain a header, independent declared fields/members and a
written close, so packing them does not imitate a compact constructor literal.
Access-section markers retain their separate written unit. Comma-packed type
declarations require source-specific review until their grammar is resolved.

Value-only `if` chains and `or`/`unsafe` fallback expressions belong to their
surrounding binding or return. Thus both packed and wrapped
`x := if c { 1 } else { 2 }` propose one canonical unit. A handler
`x := read() or { panic(err) }` proposes four: binding, handler header, body and
written close. Their expanded spellings must agree. An anonymous block function
has one definition header, its independent body statements and a written close.

Unknown function-valued fallback expressions, statement-bearing conditional
expressions, complex interpolation, compile-time controls, captures/receiver
declarations and imperative unsafe/shared/match/select forms are diagnosed for
source-specific review. They never silently receive an approved compact count.
The fixtures are a proposal and have not run as of this file's creation.

Grammar references: the locally saved official `v-report-only/20261012/upstream/
v-api/docs.md` and [V comments documentation](https://docs.vlang.io/comments.html).
Qualification must also preserve the pinned compiler's build/correctness
evidence; normalization alone does not establish valid executable syntax.
