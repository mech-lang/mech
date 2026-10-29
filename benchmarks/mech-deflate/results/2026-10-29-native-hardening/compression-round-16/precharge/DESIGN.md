# Ordinary aggregate metadata precharge, round 16

Base: c6054fcf72be0a196d700899360283108a29ccb8. Production control: 5cc14c28ccf786bceeb3ea0db7448530caed537c (later checkpoints change tests/evidence only).

The unchanged compressor executes 53 generic record-field calls in its eight-byte run. `numeric::aggregate_access::fund_metadata` manually visits the entire immutable schema table for each dynamic aggregate closure. The existing budget helper already has an exact successful-debit summary and an ordered denial fallback. Ordinary aggregate access now reuses it. This prepares no new runtime authority, retained owners, shape facts or cached success. All metadata sizes and logical charges remain identical. Current shape reconstruction, factory selection, payload copying, output limits and publication are untouched.

The helper is available without the native-artifact feature. Its batch requires a successful same-current-account preview under the actual resident turn. Shared worker scopes keep incremental traversal because credit can change between preview and debit. Unavailable summaries, arithmetic overflow and denied previews keep the old visitor, preserving its first rejection, spent prefix and estimate. No IR/work/ABI/bytecode identity changes are needed: 119/93/35/14 remain.

Differential unit cases run default meters within active turn plans against the original visitor across each nonzero contribution boundary for policy, account and call caps, with preexisting work and u64 overflow. Worker cases compare results, estimates and settled account work, checking cleanup. Existing dynamic-projection integration tests exercise current selectors, fresh owners, rectangular/empty shapes, original/decoded artifacts and success/failure/retry publication and aliases. The primary pairs the unchanged pinned allocator/source/evaluate_into-only eight-byte harness against the retained round-15 executable. Logical work must remain exactly 303735923752.

Only this optimization will be measured in this packet. No compressor source or allocation sites change.
