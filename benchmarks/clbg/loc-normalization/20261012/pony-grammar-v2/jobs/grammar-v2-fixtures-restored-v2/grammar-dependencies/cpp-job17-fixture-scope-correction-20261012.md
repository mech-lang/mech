# Prototype parameter scope fixture correction

Immutable `boundary-audit-17` stopped at20:14:40UTC after672 methods, with671
passing and one failure, before any source-pool audit. Its executed receipt
preserves Python/Pygments/frozen dependency pins and the common-lock proof.

The newly added negative fixture incorrectly expected a parameter name inside
`void f(Foreign Node(int));` to create a global `Node` shadow. A prototype has
no function body; that parameter name cannot hide the outer class after the
prototype. The actual empty global-shadow result was correct. The fixture now
checks three separate scopes: an unresolved ordinary global function named
Node remains gated, an unknown formal Node inside a function definition gates
the body but expires before the outer declaration, and a prototype preserves
the outer class. No parser implementation or normalization rule changed for
this correction. This file grants no executed result or chart admission.
