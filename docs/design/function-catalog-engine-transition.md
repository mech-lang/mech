# Function catalog execution architecture

The function system has one execution model on native and WASM. The obsolete
program and interpreter packages are gone; retained programs,
source elaboration, bytecode reconstruction, checkpoints, and reactive turns
live in `mech-engine`.

## Architecture

```text
mech-core
  FunctionCatalog interfaces and stable IDs
  MechFunction traits and function-definition data

machine crates
  concrete runtime factories
  source specializers
  explicit catalog installers

mech-engine
  temporary standard catalog composition
  FunctionEnvironment and FunctionResolver
  FunctionExtensions and user functions
  MechProgram and Interpreter

mech-runtime
  hosts, effects, scheduling, modules, and external transactions
```

`FunctionCatalog` is immutable linked functionality shared as an
`Arc<FunctionCatalog>`. It owns concrete runtime factories, static source
specializers, and exact export metadata. Builders validate stable IDs,
canonical names, duplicate entries, and export relationships before producing
the read-only catalog. Custom program and runtime constructors retain exactly
the catalog supplied by their caller; they do not initialize or derive state
from the standard catalog.

`FunctionEnvironment` is mutable per-program visibility. It records enabled
catalog operations plus exact visible-name bindings to either a catalog
operation or a program-local extension. Prelude and internal visibility is
derived from catalog export metadata. Static module imports use exact
`(module, item)` exports; they do not infer membership from name prefixes.

`FunctionExtensions` stores program-local specializers for native closures,
runtime host functions, and dynamically loaded modules. Entries retain their
captured contexts and dynamic-library ownership through `Arc`. User-written
functions remain separate as `FunctionDefinition` values in
`UserFunctionTable` so their scopes, match arms, retained plans, and checkpoint
state preserve language semantics.

`FunctionResolver` is the only function-resolution boundary. Named calls use:

```text
user-defined function
  then current FunctionEnvironment binding
    then catalog operation or program-local extension
      otherwise a structured missing-function error
```

Syntax operators bypass named bindings and resolve their canonical
`OperationId` directly, so a same-named user or extension function cannot
replace language syntax.

## Execution and checkpoints

Source execution specializes through the resolver and stores concrete
`Box<dyn MechFunction>` nodes in reactive plans. Bytecode execution resolves a
`RuntimeFunctionId` only through `FunctionCatalog::runtime_factory` and
reconstructs the exact concrete function encoded by the artifact. Missing IDs
retain the existing arity-specific structured errors. There is no registry or
fallback path.

Program checkpoints restore the function environment, program-local
extensions, and user-function definitions and retained state. The immutable
catalog is not checkpoint state, and its `Arc` identity remains unchanged
through child interpreters, module operations, clear operations, and rollback.

## Compatibility and composition

Bytecode remains version 1. Canonical operation names, concrete runtime factory
names and IDs, the frozen source surface, specialization selections, and the
checked-in pre-rewrite bytecode artifacts remain unchanged.

Until the next architecture step, `mech-engine` explicitly composes the
standard machine installers. Moving that composition to `mech-stdlib` and
separating machine runtime/source/compiler feature profiles belongs to PR3;
`.mecb`-driven minimal native application generation belongs to PR4.
