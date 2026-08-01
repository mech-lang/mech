# Function-system compatibility baseline

This baseline records the language-visible and bytecode-visible behavior of the
function subsystem before the function-catalog rewrite. It is a compatibility
contract for the rewrite, not a contract for the descriptors, inventories, or
registries that currently implement that behavior.

The baseline was captured from `v0.4-beta` at
`f7768e0c6bbde69d27410be6d1ecacbd08c238c5`.

## Frozen contracts

The checked-in fixtures freeze:

- full effective source-specializer names and IDs;
- effective prelude-visible source operations;
- exact linked-module exports;
- representative type-and-shape specialization choices;
- selected concrete runtime factory names and IDs;
- bytecode version 1 compatibility for the checked-in fixtures;
- native and browser source behavior;
- native linked-module behavior; and
- standalone active-machine health.

The JSON snapshots are deterministic and describe semantic identities rather
than registration order. The legacy `.mecb` files are pre-rewrite artifacts
that must remain executable by a compiler-free runtime consumer.

## Deliberately not frozen

This baseline does not freeze:

- `FunctionDescriptor`;
- `FunctionCompilerDescriptor`;
- `ModuleItemDescriptor`;
- the internal layout of `Functions`;
- inventory registration order;
- exact duplicate registrations;
- prelude implementation-name prefixes;
- module prefix scanning;
- interpreter machine dependencies;
- Cargo feature fan-out;
- raw bytecode serialization order;
- binary size;
- compile time;
- WASM linked-module support; or
- `mechc` behavior.

These mechanisms may change during the catalog rewrite as long as the frozen
observable contracts remain intact or an intentional compatibility decision is
documented.

## Baseline update policy

`--check` is the CI path. It regenerates the semantic snapshot in memory and
compares it with the committed JSON without modifying the worktree.

`--write` regenerates deterministic JSON only. Review every diff produced by
this command. An intentional change to a source-visible operation name or a
concrete runtime factory ID must explain the compatibility decision that made
the change necessary. Ownership-only refactors should not alter these
baselines.

`--write-bytecode` is a deliberate compatibility-fixture operation. The
checked-in `.mecb` files must not be casually regenerated during the catalog
rewrite: doing so would replace the pre-rewrite artifacts whose compatibility
the consumer is meant to prove.

The exact maintenance and validation commands are recorded beside the JSON
snapshots in `tests/architecture/function-system/README.md`.
