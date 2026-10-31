# Shared numeric scratch storage

Parent: `f81cbfdc67cc591ed54bd861d63d693c005706ad`. Round 50 is published,
qualified and measured; it is progress toward parity, which remains unachieved.

The unchanged complete compressor has 239 numeric regions. Its separate numeric
shadows total 5,491 words, although the largest needs only 49. Closed numeric
regions contain no child loops; the runtime refuses a second active loan.
Parent segments reconcile before entering a child. Share only numeric scratch,
keeping all per-region readiness/fuel words and parent-segment storage separate.

Add an explicit numeric shadow base, reconstruct it on image loading, and update
entry, SSA word selection, materialization, export and abort consumers together.
Advance the typed-IR identity for the changed native storage/image layout; keep
bytecode and the external ABI unchanged. Preserve original logical setup work
and conservative combined transport admission while allocating fewer words.
Actual physical memory demand may fall; document that resource improvement.

Require disjoint per-region flags, a single sized scratch bank, strict image
reconstruction, sequential regions with different current arguments/views,
unselected loads, assertion/load/arithmetic errors, preserved publication and
aliases, finite-fuel reconciliation, cleanup and retry. Reuse maintained
original/decoded scalar/component/linked controls without looser allowances.

Measure the same complete eight-byte case, separately collect hardware,
allocation and sampling profiles, and commit every cohort immediately. Preserve
the pinned source/MBC, actual buffers, algorithms, allocator, clear/reuse and
timer. Report resource and elapsed effects separately; presume no speedup.
