# Mech Engine

`mech-engine` owns retained program execution through the public `MechProgram`
type. It coordinates:

- program-local checkpoints;
- stable input updates;
- reactive turns;
- integrity validation;
- source, syntax-tree, and bytecode execution;
- optional bytecode compilation.

The package contains program-instance behavior. Host services, scheduling, file
watching, and persistence belong outside this boundary.
