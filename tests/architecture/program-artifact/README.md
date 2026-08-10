# C3 deterministic ProgramArtifact boundary

This contract freezes the pre-launch bytecode-v1 and compiler direction:

```text
source compiler -> ProgramArtifact -> bytecode v1 -> validated ProgramArtifact
```

`ProgramArtifact` is the sole semantic graph. Bytecode v1 carries its schema,
constant, interface, slot/producer, node, binding, output, constraint, and
operation sections directly. Earlier pre-launch bytecode-v1 layouts have no
compatibility adapter.

C3 produces and round-trips the artifact but does not activate or execute it.
The checker therefore freezes the C2 production execution paths against
changes in this PR, while allowing the compiler-only `MechProgram` method to
emit the artifact alongside its existing executable bytecode product. The
runtime `NodeId` qualification remains a non-semantic disambiguation. Neither
allowed edit routes execution through `ProgramArtifact`.
