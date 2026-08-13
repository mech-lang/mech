# D3 resident external-turn contract

This directory freezes the D3 boundary between authoritative external requirements,
captured observation facts, resident candidate execution, prepared provider effects,
one state publication, retained receipts/outbox entries, and replay.

The two `.mec` files are ordinary source fixtures. Their source and decoded bytecode-v1
artifacts must have identical revisions, requirement tables, nodes, and zero-output
external declarations. `generate-d3-contract.py` owns the JSON projections and schemas;
`check-d3-contract.py` enforces the source and feature boundaries.

D3 is deliberately not product routing. The `resident-external` feature is opt-in,
normal native/browser/WASM selection is unchanged, and unsupported durability or
delivery modes fail closed.
