# R1–R6 execution proof

This example compiles and executes the five-line recurrence in
[trust-program.mec](trust-program.mec):

```mech
~x := 1.0
next := x * 2.0 + 3.0
safe! := next < 1000.0
x = next
x
```

The native proof uses the retained canonical document parser and `CanonicalSourceFrontend`.
Its ordinary node sequence is exactly `core/assign`, `math/mul`, `math/add`,
`compare/lt`, `core/assign`. Its four distinct primitive identities match the
original proof; the current canonical compiler emits an additional assignment node
that snapshots prior state. Activation initializes state to 1. The output slot is
derived from the next-value expression and is compared after each accepted turn.
The original example assumed an output alias initialized to 1; its rejection
on the current artifact is preserved in the audit evidence. The activated resident engine executes these primitive operations.
The source graph specifies the recurrence. This ownership distinction is recorded
as F-007 in the v0.4 audit; the external machine factory has separate execution evidence.

Run from the repository root:

```sh
./scripts/demo-r-stack.sh
```

The executable checks:

1. Lossless source parsing, checked operation contracts and schemas.
2. Artifact construction, exact operation list and bytecode round trip.
3. Activated schedule, slots, integrity mode and memory layout.
4. Seven accepted outputs against the literal values `5, 13, 29, 61, 125, 253, 509`.
5. Equal values and receipts from source artifacts and decoded bytecode.
6. A source change to `x * 3 + 4`, with a changed artifact revision and outputs `7, 25, 79`.
7. Rejection of candidate `1021`, preserving epoch, hash, state and output.
8. Syntax, type, missing-operation, truncated-bytecode, tampered-artifact and one-byte-budget rejection paths.

A failed assertion exits with a nonzero status. The scope is this recurrence and
the enumerated admission and publication checks. The example was absorbed from
PR 829 at `9ae5c00e0`, with prose and ownership descriptions updated for this audit.

The browser extension at `audit/v04/site/trust.html` presents recorded native
structures and independently checks first-turn source results through the built
canonical-source WASM adapter. Its evidence identifies each execution environment.

Full native structures are available through:

```sh
./scripts/demo-r-stack.sh --dump parse
./scripts/demo-r-stack.sh --dump artifact
./scripts/demo-r-stack.sh --dump activation
./scripts/demo-r-stack.sh --dump memory
./scripts/demo-r-stack.sh --dump all
```

The memory dump uses `ProgramMemoryPlan::diagnostic_text`, the pointer-free
representation used by the planner's audits and determinism checks.
