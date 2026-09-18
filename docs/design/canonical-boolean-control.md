# Canonical scalar control in S4

A scalar match owns ordered arms and their optional guards. Its scrutinee is
evaluated once. Only a matching arm's guard executes; only the first matching arm
whose guard succeeds executes its body. Wildcard and lexical bind patterns are
exhaustive without a guard. Source lowering requires an unguarded wildcard/binding or coverage of both Boolean
literals, and one exact closed result schema. Literal patterns reference constants
with the scrutinee schema; bindings expose that same schema to guards and bodies.
Wildcard-only matches impose no scalar type on an unused scrutinee.

The engine artifact owns control blocks, captures, scoped parameters, local
ordinary operations, nested match declarations and yields. Source text and diagnostic strings are not
execution operands. The artifact uses one tagged node body for ordinary
operations, scalar matches, or lexical comprehensions. The bytecode-v1 graph payload now uses
revision 8 after control-local operations gained one recursive body grammar for
ordinary calls, matches, and comprehensions; revision 7 remains the closed
structural-parameter grammar.
All durable fixtures are regenerated with the current producer.

Resident match literal comparisons admit Bool, Index and F64 scalar layouts.
Bindings, captures and results use the existing ordinary closed value layouts,
including managed tuples, records, matrices, strings and scalar snapshots. Each
local operation binds through the existing ordinary provider and call-memory
contract. Its physical call and scratch identities are separate from the
outer artifact schedule. Capability witnesses include local operations, while
failures report the enclosing source owner. Each local's turn plan uses that
local's physical call identity. Every local is fully written when its block
executes, including after branch switches. Nested blocks retain explicit direct
step lists: only the selected inner arm executes. One work budget covers the
selected path, including nested calls. Managed block-local payloads are released
after their yield is copied, on both success and failure.

Match results use owned derived slots. State changes proceed through the
existing assignment and candidate-publication machinery. Kernel failures,
failed integrity constraints and explicit abort preserve the published epoch
and state. Both arms' storage is admitted at activation; an inactive arm does
not execute to produce its result.

Runtime reuse compares control structure and resolves schema, contract and
constant identities in each artifact's own tables. Equal numeric table IDs
are not evidence that independently compiled blocks mean the same thing.

Lexical comprehensions own ordered generators, patterns, pure ordinary calls,
filters, and a yield in the same artifact. Single-writer local identities enforce
scope and dominance. Repeated pattern bindings compare values and implement
joins. Matrix iteration follows canonical row-major order; set construction uses
the core key relation for deduplication and float normalization. No diagnostic
qualifier strings or ordinary-operation placeholders participate in execution.

Resident comprehension execution currently supports Bool, Index, and F64
bindings and yields, scalar patterns, and qualified ordinary kernels. Every
inner call retains its ordinary contract and physical call-memory identity.
The owning control accumulates work across calls, so repeated individually
admissible scans cannot bypass the turn limit. Output growth is admitted before
allocation. Failed turns preserve published values and can recover on a smaller
subsequent input. Pattern depth and generator nesting are bounded in artifacts;
bytecode population limits apply before graph allocation.

The later S8 control revisions extend this S4 foundation with composite
comprehension bindings and yields plus recursive match/comprehension operation
bodies. Matches can nest in match bodies and guards, and either control form can
contain the other, with at most eight declarations on a path. Computed pattern
evaluation blocks and FSM lowering still require implementation. Unsupported
cases remain explicit errors and do not count as executable completion evidence.
