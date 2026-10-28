# Follow-up attribution of the round 02 profile

No new runtime timing was collected. The exact round 02 executable is
SHA-256 `a98a68366c1d3f807f5ab0ff78a105b72244d3a14f1da8bbcf7aa8ab0e484e6f`.
`llvm-nm --numeric-sort --demangle` resolves the sampled anonymous bodies to
schema body/dimension heap-size visitors. Correcting for the sampled image
slide gives body visitor address `0x106e89e5c` and dimension visitor
`0x106e8a210`. Several Rust monomorphizations share each body.

The recorded sample has 1138 main-thread leaf samples. Recursive stacks counted
once per leaf place 373 samples inside `SchemaTable::clone_allocation_bound_bytes`
and its children (about one third of this short sample). Body visitor leaves
account for 300 samples, dimension visitor leaves for 30, and the table visitor
itself for 48; not all body visitor calls originate in a table calculation.
These are sample counts, not exact elapsed-time fractions or a speedup promise.

The table contains immutable entries but each no-callback bound request walks
the entire arena again. Scalar-result construction and snapshot ownership
admission repeat that request. Current same-account and existing-import
shortcuts already exist and must be retained. The next candidate is an inline
immutable layout summary owned and derived by SchemaTable itself. The separate
callback/precharge walk must retain its ordered checks on every invocation.
