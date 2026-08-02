# Function catalog execution architecture

Status: the PR3 static-composition transition is complete. Standard catalog
composition now belongs to `mech-stdlib`; see
[Static standard-library composition](static-stdlib-composition.md) for the
resulting package and feature boundaries.

The function system has one execution model on native and WASM. The obsolete
program and interpreter packages are gone; retained programs,
source elaboration, bytecode reconstruction, checkpoints, and reactive turns
live in `mech-engine`.

## Architecture

```text
mech-core
  FunctionCatalog interfaces and stable IDs
  MechFunction traits, value representations, and bytecode model

machine crates
  concrete runtime factories
  optional source specializers
  optional bytecode lowerers
  runtime and source installers

mech-engine
  bare execution and optional source elaboration
  engine-owned intrinsic installers
  FunctionEnvironment and FunctionResolver
  FunctionExtensions and user functions
  MechProgram and Interpreter
  no standard machines or standard catalog

mech-stdlib
  feature-selected static distribution composition
  intrinsic and standard-machine installation
  runtime and source catalog construction

mech-runtime
  distribution-neutral hosts, effects, scheduling, modules, and transactions
```

`FunctionCatalog` is immutable linked functionality shared as an
`Arc<FunctionCatalog>`. It owns concrete runtime factories, static source
specializers, and exact export metadata. Builders validate stable IDs,
canonical names, duplicate entries, and export relationships before producing
the read-only catalog. `FunctionCatalog::empty()` is the engine default.
`MechProgram::new`, `Interpreter::new`, and `RuntimeBuilder::new` are bare;
distribution entry points inject a catalog explicitly and retain exactly the
catalog supplied by their caller.

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

PR3 completed the composition transition. `mech-engine` has no dependency on a
standard machine and no standard-catalog constructor or fallback.
`mech-stdlib::runtime_catalog()` composes the exact runtime factories selected
by its feature closure. `mech-stdlib::source_catalog()` composes the same
runtime factories followed by source specializers and exports. Compiler builds
use that source catalog and enable lowering through Cargo features; they do not
create a third catalog kind.

Machine crates now separate concrete runtime support, source specialization,
and bytecode lowering behind `runtime`, `source`, and `compiler`. A machine's
`compiler` layer does not imply `source`, while a complete
`mech-stdlib/compiler` distribution includes both. Engine syntax may still map
operators such as `+` to the canonical name `math/add`, but only a selected
machine implementation can provide that operation.

`.mecb`-driven derivation of a minimal native application's exact
`mech-stdlib` feature closure belongs to PR4.
