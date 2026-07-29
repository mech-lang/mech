# Optional bytecode compiler boundary

Mech supports two deliberately different function compositions. The local
`compiler` feature changes the `MechFunction` contract at compile time:

```text
runtime-only machine:
  MechFunctionImpl
  MechFunctionFactory
  concrete function factory registration

compiler-enabled machine:
  all runtime pieces
  + MechFunctionCompiler
  + CompileConst-dependent lowering bounds
```

In a runtime-only build, a function node needs only its execution
implementation. In a compiler-enabled build, every value stored behind
`Box<dyn MechFunction>` must also implement `MechFunctionCompiler`. This is
intentional static enforcement, not accidental coupling: a compiler-enabled
plan can never contain a node that lacks bytecode lowering.

Linked machine crates must therefore enable their local `compiler` features in
lockstep with `mech-interpreter/compiler`. Runtime-only machine builds omit both
the lowering implementations and the `CompileConst` bounds needed solely by
those implementations. They still include their runtime factories and concrete
function registration.

## Ownership

The dependency direction is:

```text
                     mech-core
        runtime model + conditional compiler SPI
                  ▲             ▲
                  │             │
          machine crates   mech-bytecode
                  ▲             ▲
                  │             │
           mech-interpreter  mech-program
                                  ▲
                                  │
                             mech-runtime
```

`mech-core` owns the backend-neutral `BytecodeCompilerContext` lowering SPI.
Machine functions compile against that object-safe interface, so machine crates
do not depend on the concrete backend.

`mech-bytecode` owns `CompileCtx`, including register allocation, constant and
symbol collection, instruction collection, section writing, and the final CRC
trailer. It depends only on `mech-core` and the binary-writing libraries.
`mech-program/compiler` activates it and constructs a compiler locally while
walking a plan. Neither `Interpreter` nor `MechProgram` retains that compiler
after emission.

Program execution must not imply compiler support. `mech-program/program` and
`mech-runtime/program` activate the runtime model, decoder, interpreter, and
required function factories, but not `mech-bytecode`, `mech-core/compiler`, or
`mech-interpreter/compiler`. This keeps runtime consumers free of source
lowering and writer code.

## Format ownership

For this extraction, `mech-core` continues to own bytecode version 1's format
model and decoder: headers, requirement flags, type tags and sections,
constants, instruction models and decoding, and CRC validation when reading a
`.mecb`. `mech-bytecode` owns only the stateful compiler and binary writer.

This boundary intentionally makes no bytecode change. Version 1, requirement
encoding, instruction encoding, constant alignment, section order and layout,
and the CRC32 trailer remain unchanged. Moving the decoder or format model is
separate future work.

## API migration

Bytecode compilation is now a `MechProgram` concern. The public call changes
from:

```rust
interpreter.compile()
```

to:

```rust
program.compile_bytecode()
```

For the temporary legacy native-packaging artifact, use:

```rust
program.compile_bytecode_artifact()
```

Direct interpreter embedders that need to compile bytecode must wrap their
interpreter use in `MechProgram`. Runtime-only interpreter builds intentionally
do not expose bytecode compilation.

`compile_bytecode_artifact` is temporary compatibility scaffolding for legacy
`mechc`; it is not the permanent native-build contract.

## Legacy native packaging

`MechProgram::compile_bytecode_artifact` temporarily returns both the emitted
bytes and the collected requirements. The legacy `mechc` native packager uses
that requirements vector to preserve its existing generated-Cargo-feature
pipeline without retaining `CompileCtx` in the interpreter.

The artifact is compatibility scaffolding, not the long-term native build
contract. When native packaging derives requirements directly from the emitted
`.mecb`, the extra requirements vector and its sole production consumer can be
removed.
