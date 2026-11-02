# Resource planning

Canonical source compilation and native artifact planning validate resource
addresses, operations, provider contracts and declared port schemas. Native
artifact planning also checks the configured grants for each resource.
They inspect every retained resource operation, including effects that are
dormant on the initial turn. Planning representatives describe an observation's
schema and shape; they are not input values for arbitrary program execution.

Providers implement `preflight_write_schema` to check a payload's declared
structure without constructing a sample payload or observing the host.
`ResourceSchemaPreflight::Validated` records structural compatibility.
`DeferredToRuntime` records that the structure cannot be fully decided from the
schema. Neither result validates numeric domains or dynamic contents. The
default hook runs the existing address and operation preflight, then defers the
payload check, preserving custom providers that validate values during writes.

The CLI provider requires a scalar String. Console and robot command providers
retain their existing payload acceptance. Scene checks known record fields,
element kinds and fixed matrix extents while retaining conditional checks for
dynamic values, absent optional collections and potentially empty tables.

Every real write still goes through provider preparation before publication or
delivery. A closed application with no inputs, observations, mutable state or
suspension can additionally validate its actual computed payloads during a
bounded planning turn. That turn is aborted after the effect-free value checks;
it does not publish outputs or deliver effects. Open applications retain their
unknown inputs and defer value-dependent checks to execution.

Resource reads retain the artifact's observation contract. Native planning uses
the provider's input-driver metadata to determine whether an observation needs
driven host input; argument and stdin snapshots require no live input driver.
It checks a read representative against the artifact's declared schema and
shape without executing its downstream program.
