# Round 04: reuse the owned turn template

Declared before implementation and primary observations. Starting compiler:
`88b9b89a3a425c9842e1e3cbf33bd914c6f48ebe`. The retained complete-compression
before executable is SHA-256
`44ad75d2536c5975058933a97a1703de209930931dcf5bc7dcb6e98ece01c2e8`.
Its last small median was 1904.05x Roc, with a 0.000275-second reference.

The actual compressor reaches ordinary native collection calls throughout
buffer creation, reads and updates. In the last 1358-sample profile,
plan_current appears in 211 inclusive samples and plan_turn_memory in 165;
CallMemoryPlan and resolved-value descriptor cloning appear beneath these
paths. These overlapping sample counts are evidence of repeated metadata
work, not independent elapsed-time fractions or a speedup promise.

Both callers of derive_turn_call already own a copied template. That helper
copies the complete template again merely to suppress its limits during the
existing core derivation. Borrow the already-owned local template mutably,
temporarily take its limits, invoke the same core planner, and restore the
limits before returning either success or error. Keep the final returned
target and all current-footprint, region, publication and admission checks.
No shared program template is mutated. No new representation, authority,
executor, public API, source operation or resource allowance is introduced.

Qualify current payload changes, success/error/retry on the same local
template, limit restoration, existing program memory-plan regressions and
native finite error/recovery cases. The complete source must compile to the
same pinned artifact and decoded re-encoding must agree. Run protected
original/decoded Bool/U64/I64/F64 controls with unchanged allocation requests
and bytes and after/before medians <=1.05. Scalar-loop controls at zero/64
visits must retain zero allocation slope. Keep every observation and failure.

Use the exact same selected-operation build profile for both subjects. Reuse
the unchanged eight-byte timing harness, artifact and pinned mimalloc object.
Refresh 20000 reference samples, one warmup and three retained observations
per Mech subject, with the harness's extra call excluded. Report multiples of
Roc = 1x, both absolute medians and their ratio. Require exact raw bytes,
independent inflation, aliases, capacity 13, identical reported logical work
941491317987 and zero terminal managed bytes/nodes. No compilation may overlap
primary timing. Do not run a larger input.

Before the primary timing, rerun the unchanged 24-observation System-allocator
word-update witness with the real 262144-entry U16 tree and 1500513-word U32
cache. Compare allocation requests and requested bytes with round 03. Follow
primary timing with a separate two-second profile, excluding all profiling
observations. Commit code, tests and receipts before another optimization.
