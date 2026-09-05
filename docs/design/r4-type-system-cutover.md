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

R2 storage compatibility is mandatory before a descriptor is attached to a
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

R4 does not change bytecode-v1, canonical schema encoding v1, the
`ProgramArtifact` format, dynamic-module ABI v1, operation or runtime IDs,
native linkage names, or package versions. Semantic certificates are planning
sidecars and are not added to bytecode-v1.

## Later phases

R5 Memory planner — complete. It consumes R4's authoritative descriptors and
compatible physical requirements to derive deterministic capacity, placement,
reuse, liveness, transfer, and budget plans. R6 Memory runtime cutover — next.
R6 consumes the R5 layouts, capacities, arena placements, lifetimes, alias
groups, reuse groups, transaction requirements, budgets, and transfer
requirements. R6 may implement allocation handles, pools, managed backing,
actual reuse, movement, publication, and reclamation. R6 may not silently
derive a different physical plan.
