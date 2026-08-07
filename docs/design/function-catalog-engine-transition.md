# Function catalog and engine transition

This transition replaces the obsolete `mech-program` package with
`mech-engine`. The old package has been deleted; there is no compatibility
crate or re-exporting shim. `MechProgram` remains the retained program-instance
type and now lives at the engine boundary.

`mech-engine` owns retained program execution, program-local checkpoints,
input updates, reactive turns, integrity validation, and source, tree, and
bytecode execution. Bytecode compilation remains optional.

## Architecture

```text
                        mech-core
          FunctionCatalog / FunctionEnvironment
               ▲
               │
          mech-math
        math/add fragment
               │
               ▼
         mech-interpreter
      FunctionSystem / syntax
               │
               ▼
          mech-engine
          MechProgram
               │
               ▼
          mech-runtime
```

`FunctionCatalog` represents immutable linked functionality. A builder checks
names, stable IDs, runtime factories, source specializers, and exports before
construction. Once built, its indexes are read-only and the catalog is shared
by `Arc` across the interpreter, engine, and runtime layers.

`FunctionEnvironment` represents mutable, per-program visibility. It contains
only visible operation IDs, name bindings, and their dictionary. It contains no
runtime factory or source-specializer objects. The environment is program state,
so checkpoints clone and restore it.

`FunctionSystem` bundles the catalog with an immutable
`LegacyFunctionBoundary`. The boundary records which canonical operations and
runtime IDs must be resolved by this exact catalog and therefore may not
silently reach a legacy registry. Named ownership retains the canonical name as
well as the stable ID so hash collisions cannot select the wrong operation.
Both objects retain the same `Arc` identity across clear,
child, module, bytecode, and rollback paths. Custom compositions derive their
boundary only from the supplied catalog unless they explicitly inject a
different policy; they never inherit ownership from the standard distribution.

## Migrated execution slice

`math/add` is the only operation fully migrated in this PR. Its source
specializer, prelude-only export, and every enabled concrete runtime factory are
installed explicitly from `mech-math`. It is not introduced as a `math` module
export, preserving the PR0 surface. Native and WASM builds use the same
installer, and the installer does not enumerate `inventory`.

Numeric formula addition and canonical named `math/add` calls specialize through
the visible catalog operation before plan execution. Reactive plans continue to
store concrete `Box<dyn MechFunction>` nodes. Bytecode execution resolves all
six instruction arities from the catalog first. A missing catalog-owned factory
returns `RuntimeFunctionUnavailable`; only IDs outside the supplied boundary use
the explicitly named legacy fallback. Bytecode reconstructs the exact concrete
function identified by the artifact and does not rerun source specialization.

The existing mixed-kind and mutable-reference behavior is shared by the legacy
and catalog `math/add` specializers. Because those coercion paths can inspect
live values during specialization, the catalog specializer conservatively
reports `GuardFunctionSafety::Unsupported`. It must not claim `PureStatic` until
coercion can construct the graph without reading live contents. Named
`math/add(...)` activation guards therefore remain rejected by safety metadata;
infix formula guards retain their existing structural validation policy in PR1.
Aligning those validation paths without weakening the purity contract is
follow-up work.

## Legacy boundary

All operations other than `math/add` remain on the legacy path. Generic named
dispatch queries the supplied boundary and contains no operation-specific
exception. The legacy
`FunctionDescriptor`, `FunctionCompilerDescriptor`, and `ModuleItemDescriptor`
inventories remain in place for that transition boundary and for the frozen PR0
contracts. Legacy inventory descriptors are not imported into
`FunctionCatalog`, and linked-module catalog resolution is not introduced here.
User-defined functions and dynamic modules also remain in their existing tables.

Standard catalog composition currently lives in `mech-interpreter` and installs
only the migrated `math/add` fragment. The standard `FunctionSystem` is cached
after its first construction; explicitly supplied systems bypass that default
factory entirely. Standard composition will move to `mech-stdlib` in a later PR.

## Compatibility

Bytecode remains version 1. Concrete runtime factory names and IDs are
unchanged, including every frozen PR0 add case. The checked-in pre-rewrite
bytecode artifacts and function-system JSON corpora remain byte-for-byte
unchanged. Native and browser targets therefore observe the same explicit add
catalog without changing the serialized format.

## Later work

Later PRs will:

1. migrate all remaining operation families;
2. remove `LegacyFunctionBoundary` and the legacy `Functions` factory and
   specializer tables, leaving no standard-function fallback;
3. absorb or eliminate the remaining `mech-interpreter` package boundary;
4. move standard composition into `mech-stdlib`;
5. split machine runtime, source, and compiler features; and
6. build minimal native applications from `.mecb`.
