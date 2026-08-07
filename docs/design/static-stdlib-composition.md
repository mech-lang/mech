# Static standard-library composition

Mech separates the execution engine from the concrete function distribution
linked into an application. `mech-engine` understands the language and runs
catalog entries, while `mech-stdlib` decides which engine intrinsics and
standard machine implementations are available.

This boundary makes a runtime's function surface an explicit Cargo feature
choice instead of an engine default.

## Ownership

```text
mech-core
  catalog interfaces and stable function IDs
  value representations and function traits
  bytecode program model

mech-engine
  bare retained-program and interpreter execution
  optional source syntax and elaboration
  engine-owned intrinsics
  no standard machine dependencies
  no standard catalog

mech-stdlib
  static, feature-selected distribution composition
  engine-intrinsic installation
  selected standard-machine installation
  runtime, source, and compiler distribution profiles

machine crates
  runtime kernels and concrete factories
  optional source specializers
  optional bytecode lowerers

mech-runtime
  distribution-neutral host and runtime integration
  scheduling, effects, modules, and external transactions
```

Dynamic modules remain program-local engine extensions. They are not part of
the static `mech-stdlib` composition and retain their library lifetime and
export ownership in the program that loaded them.

## Bare engine construction

`FunctionCatalog::empty()` creates a catalog with no runtime factories,
specializers, intrinsics, or exports. The convenience constructors are
deliberately bare:

- `MechProgram::new` uses an empty catalog;
- `Interpreter::new` uses an empty catalog;
- `RuntimeBuilder::new` starts with an empty catalog.

Callers that expect a concrete distribution must inject one explicitly with
`MechProgram::with_function_catalog`, `Interpreter::with_function_catalog`, or
`RuntimeBuilder::function_catalog`. There is no engine or runtime fallback to a
standard catalog.

For standard static distributions, use one of the catalog accessors from
`mech-stdlib`:

```rust
let runtime_catalog = mech_stdlib::runtime_catalog();
let source_catalog = mech_stdlib::source_catalog();
```

`runtime_catalog()` contains the runtime factories selected by the
`mech-stdlib` feature closure. It is appropriate for bytecode-only execution.
`source_catalog()` installs those same runtime factories and then installs the
selected source specializers and export metadata. Source execution and
compiler-enabled source paths both inject `source_catalog()`; there is no
separate compiler catalog.

On `std` builds, the runtime and source catalogs use separate
`OnceLock<Arc<FunctionCatalog>>` caches. On `no_std` builds the same accessors
construct fresh catalogs. Callers building custom catalogs may instead use
`install_runtime` and `install_source` with their own
`FunctionCatalogBuilder`; this does not initialize either cache.

## Intrinsics and standard machines

Engine-owned operations such as access, assignment, conversion,
concatenation, comprehensions, table operations, and variable definition live
under `mech_engine::intrinsics`. Their public composition hooks are:

```rust
mech_engine::install_intrinsic_runtime(builder)?;
mech_engine::install_intrinsic_source(builder)?;
```

`mech-stdlib` calls the intrinsic installer first and then calls each selected
machine installer in a stable order. Every selected machine exposes
`install_runtime`; with its `source` layer enabled it also exposes
`install_source`.

The engine may map language syntax to canonical operations. For example, it
may map `+` to `math/add` and construct the corresponding stable operation ID.
That mapping is language semantics. The engine cannot import, install, or
provide the `math/add` implementation. A caller receives that implementation
only when its selected `mech-stdlib` feature closure includes `math_add` and
the required value and shape features.

## Feature layers

The shared layer names describe implementation capability:

| Layer | Adds |
| --- | --- |
| `runtime` | Concrete factories and execution kernels. |
| `source` | Runtime plus source specializers and export metadata. |
| `compiler` | Bytecode lowering for the selected concrete plan nodes. |

In a machine crate, `runtime`, `source`, and `compiler` are orthogonal beyond
both optional layers requiring runtime. In particular, a machine's `compiler`
feature does not enable its `source` feature. This supports all of the
following machine builds:

```text
runtime
runtime + source
runtime + compiler
runtime + source + compiler
```

Every machine operation feature implies `runtime`. Runtime structs, kernels,
factories, and runtime installers stay available without source specialization
or lowering. Source-specializer types and installers are compiled only with
`source`; `MechFunctionCompiler` implementations and lowering helpers are
compiled only with `compiler`.

At the distribution layer, `mech-stdlib/source` implies `runtime`, and
`mech-stdlib/compiler` implies both `source` and `runtime`. Compiler-enabled
distributions therefore use the source catalog and add lowering capability
without changing operation names, operation IDs, runtime factory names, or
runtime factory IDs.

## Standard, full, and selected profiles

`mech-stdlib` provides explicit standard and full profiles at each layer:

| Profile family | Catalog and tooling surface |
| --- | --- |
| `standard_runtime`, `standard_source`, `standard_compiler` | The lean release surface: f64, structural values, dynamic row/vector/matrix storage, and ordinary machine operations. |
| `full_runtime`, `full_source`, `full_compiler` | The broad release surface: all supported scalar families and mature operation families over dynamic storage. |

The root product selects exactly one distribution:

```text
cargo build --bin mech
  -> distribution-standard

cargo build --bin mech --no-default-features --features distribution-full
  -> distribution-full
```

The standard runtime contains 1,300 factories and 63 source specializers. The
full runtime contains 9,010 factories and 119 source specializers. Their exact
package, feature, operation, host, count, and digest contracts live in
`tests/architecture/distributions/standard.json` and `full.json`.

Both release profiles use dynamic shapes (`row_vectord`, `vectord`, and
`matrixd`). Fixed-storage shapes remain individually selectable by custom and
exact generated applications. The 120,000-plus extended factory universe is a
nightly compatibility surface, not a monolithic CLI distribution.

Profiles are feature closures, not special catalog constructors. Custom
distributions select the same layer features together with only the required
operations, values, and shapes. For example, a bytecode-only scalar-add
distribution can select:

```toml
mech-stdlib = {
  version = "0.3.5",
  default-features = false,
  features = ["runtime", "f64", "math_add"],
}
```

Operation features activate their owning machine and its runtime
implementation. Value and shape features use Cargo's weak dependency feature
forwarding: they configure `mech-core`, `mech-engine`, and machines that are
already selected, but never activate a machine by themselves. Consequently,
the resolved dependency graph and catalog contain only the requested static
distribution.

The root CLI defaults to the standard compiler and standard host pack. The
WASM package selects its curated browser operation and value subset. Both
inject a source catalog explicitly. `mech-runtime` and host-provider crates
remain distribution-neutral and do not depend on `mech-stdlib`.

## Repository validation ownership

Machine directories are synchronized subtrees of their owning repositories.
Machine repositories own operation semantics, specializer/lowering behavior,
and their local runtime/source/compiler feature combinations. Their CI should
test the machine against a pinned, published Mech SDK or Mech development
container; it must not build the root standard or full Mech distributions.

The Mech repository owns the other side of that boundary:

- standard and full composition contracts;
- catalog identity, uniqueness, and linkage integration;
- exact native closure planning;
- standard source, bytecode, native, hosted, live, and browser canaries; and
- complete extended compatibility validation in the reusable full workflow.

A machine subtree change therefore runs Mech's static integration contracts
and standard vertical canaries here. Machine-private semantic suites run in
the machine repository. Cross-cutting Mech changes do not rebuild every
machine's private test matrix.

## Compatibility

This composition boundary does not change bytecode version 1, canonical
operation names, function IDs, runtime factory IDs, specialization choices, or
module exports. Runtime-only and source/compiler distributions select
different implementation layers around the same stable function identity.

Native builds read operation, value, shape, host, and resource requirements
from validated `.mecb` artifacts and derive an exact static feature closure.
Generated applications reuse the same installers and catalog identities while
linking only their planned requirements.
