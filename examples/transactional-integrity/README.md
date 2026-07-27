# Integrity-constraint declaration example

This source is a minimal declaration example. Running it evaluates one valid
initial candidate with a controller target of `90`, a maximum target of `120`,
and a named integrity constraint.

Run it with:

```sh
mech run examples/transactional-integrity/main.mec
```

The command evaluates the declaration once. It does not inject host inputs or
deliver a receiver command.

The complete host-input, staged-receiver, rollback, and recovery path is
covered by the runtime acceptance test
`integrity_invalid_host_input_aborts_staged_receiver_before_commit`.
