# Canonical Boolean control in S4

A Boolean match owns ordered arms and their optional guards. Its scrutinee is
read once. Only a matching arm's guard executes; only the first matching arm
whose guard succeeds executes its body. Wildcard and lexical bind patterns are
exhaustive without a guard. Source lowering requires exhaustive Boolean
coverage and one exact scalar result schema.

The engine artifact owns control blocks, captures, scoped parameters, local
ordinary operations and yields. Source text and diagnostic strings are not
execution operands. The artifact uses one tagged node body for ordinary
operations or Boolean matches. The bytecode-v1 graph payload uses revision 2;
all durable fixtures are regenerated with that producer.

Resident activation currently admits Bool, Index and F64 control values. Each
local operation binds through the existing ordinary provider and call-memory
contract. Its physical call and scratch identities are separate from the
outer artifact schedule. Capability witnesses include local operations, while
failures report the enclosing source owner. Every local is fully written when
its block executes, including after branch switches.

Match results use owned derived slots. State changes proceed through the
existing assignment and candidate-publication machinery. Kernel failures,
failed integrity constraints and explicit abort preserve the published epoch
and state. Both arms' storage is admitted at activation; an inactive arm does
not execute to produce its result.

Runtime reuse compares control structure and resolves schema, contract and
constant identities in each artifact's own tables. Equal numeric table IDs
are not evidence that independently compiled blocks mean the same thing.

This is a bounded executable-control increment, not S4 completion. Non-Boolean
patterns, nested executable control, composite match results, executable
comprehensions and FSM lowering still require their owning S4 implementation.
Their placeholder graphs or structured unsupported diagnostics do not count
as executable completion evidence.
