# R4 type-system cutover

R4 makes the semantic result produced by the R3 type solver authoritative at
every execution-binding boundary. It is an authority cutover, not a new type
system and not a storage-planning project.

The required order is:

```text
source
  -> ResolvedCall
  -> ResolvedValueDescriptor
  -> R2 storage compatibility
  -> physical implementation selection
  -> BoundCall
  -> compiler, resident, and native planning
```

R3 remains the only semantic resolver. A `ResolvedCall` fixes the semantic
operation, overload, converted inputs, conversion plans, outputs, and output
schema rules. No physical factory or storage representation may replace or
repair those decisions.

`ResolvedValueDescriptor` connects a closed `ResolvedType` to its canonical
`Schema` and current `ShapeInstance`. Construction is checked in both
directions: the shape must instantiate the schema, and deriving a type from
that schema and shape must reproduce the supplied resolved type exactly.

R2 storage compatibility is mandatory before a descriptor is attached to a
physical backing. `FunctionValueRepresentation` remains physical metadata for
backing extraction, ABI calculation, and implementation-signature matching;
it is never a source of semantic type, operation, conversion, output schema,
or dimension authority.

`BoundCall` certifies the exact runtime implementation selected for a resolved
semantic call. It retains immutable input and output descriptors, operation
identity, origin, runtime function identity, and execution target. It does not
own allocation, capacity, alias, lifetime, or reclamation information.

## Preserved boundaries

R4 does not change bytecode-v1, canonical schema encoding v1, the
`ProgramArtifact` format, dynamic-module ABI v1, operation or runtime IDs,
native linkage names, or package versions. Semantic certificates are planning
sidecars and are not added to bytecode-v1.

## Later phases

Allocation and lifetime planning remain R5. Managed allocation, enforcement,
and reclamation remain R6. R4 exposes the authoritative descriptors and
compatible physical requirements those phases consume, but performs no
capacity, placement, reuse, liveness, transfer, or budget planning.
