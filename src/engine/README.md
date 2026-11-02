# Mech Engine

`mech-engine` is the distribution-neutral retained execution engine. It owns
the public `MechProgram` and `Interpreter` types and coordinates:

- program-local checkpoints;
- stable input updates;
- reactive turns;
- integrity validation;
- runtime and bytecode execution;
- optional source syntax and elaboration;
- optional bytecode compilation.

The engine owns language intrinsics such as access, assignment, conversion,
concatenation, comprehensions, table operations, and variable definition. It
may map syntax such as `+` to the canonical operation `math/add`, but it does
not depend on or install the standard machine that implements that operation.

## Catalogs and constructors

`MechProgram::new` and `Interpreter::new` use a new empty function catalog.
`RuntimeBuilder::new` likewise starts bare. A caller that expects concrete
functions must supply a catalog explicitly:

```rust
let catalog = mech_stdlib::source_catalog();
let program = MechProgram::with_function_catalog(config, catalog);
```

Runtime builders use `RuntimeBuilder::function_catalog`. Bytecode-only
distributions normally inject `mech_stdlib::runtime_catalog()`; source and
compiler-enabled distributions inject `mech_stdlib::source_catalog()`. The
engine itself has no `mech-stdlib` dependency and no standard-catalog fallback.

Custom distributions can compose the engine's own catalog entries through
`install_intrinsic_runtime` and, with the `source` feature,
`install_intrinsic_source`.

## Feature profiles

- `runtime` enables retained execution, plans, checkpoints, bytecode decoding,
  and function-catalog runtime lookup without the parser or bytecode compiler.
- `source` adds `mech-syntax` and source elaboration.
- `compiler` adds bytecode lowering on top of the source layer.
- `runtime_default`, `source_default`, and `compiler_default` add the package's
  corresponding default value, shape, and language surfaces.

The package default is `source_default`; compiler/tooling distributions select
`compiler_default` explicitly. Machine-owned operation features in the engine
are syntax markers only. They do not install an implementation or activate a
machine.

Host services, scheduling, file watching, persistence, and concrete standard
distribution selection belong outside this package.
