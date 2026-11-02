# Semantic type authority

The type solver is authoritative at every execution-binding boundary. Physical
selection consumes its validated semantic result.

The required order is:

```text
source
  -> ResolvedCall
  -> ResolvedValueDescriptor
  -> type-memory compatibility
  -> physical implementation selection
  -> BoundCall
  -> compiler, resident, and native planning
```

The type system remains the only semantic resolver. A `ResolvedCall` fixes the semantic
operation, overload, converted inputs, conversion plans, outputs, and output
schema rules. Its validated `ResolvedOperationDescriptor` carries the
operation ID, canonical semantic name, and operation-memory declaration as one
authority. The same descriptor is copied into `BoundCall`; compiler sidecars
are derived from that certificate and may only assert, never supply, its name
or contract. No physical factory or storage representation may replace or
repair those decisions.

`ResolvedValueDescriptor` connects a closed `ResolvedType` to its canonical
`Schema` and current `ShapeInstance`. Construction is checked in both
directions: the shape must instantiate the schema, and deriving a type from
that schema and shape must reproduce the supplied resolved type exactly.

Type-memory compatibility is mandatory before a descriptor is attached to a
physical backing. `FunctionValueRepresentation` remains physical metadata for
backing extraction, ABI calculation, and implementation-signature matching;
it is never a source of semantic type, operation, conversion, output schema,
or dimension authority.

`BoundCall` certifies the exact implementation selected for a resolved semantic
call. It retains the complete operation descriptor, immutable input and output
descriptors, origin, selected runtime or resident implementation identity, and
execution target. Artifact loading uses an explicit `ArtifactOperation` origin
when the bytecode does not retain the original overload identity. It does not
own allocation, capacity, alias, lifetime, or reclamation information.

Catalog construction rejects duplicate concrete capabilities for the same
semantic operation, execution target, and exact physical signature. Physical
selection therefore cannot settle an ambiguity by runtime name, registration
order, or implementation naming convention.

## Preserved boundaries

Semantic binding does not change bytecode-v1, canonical schema encoding v1, the
`ProgramArtifact` format, dynamic-module ABI v1, operation or runtime IDs,
native linkage names, or package versions. Semantic certificates are planning
sidecars and are not added to bytecode-v1.

## Memory ownership

Physical memory planning consumes validated descriptors and BoundCall to derive
layout, capacity, lifetime, alias, reuse, transfer, and budget requirements. The
managed runtime realizes those requirements without changing semantic authority.
