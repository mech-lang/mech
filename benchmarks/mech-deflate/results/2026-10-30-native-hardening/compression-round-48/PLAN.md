# Unique unadmitted snapshot root reuse

Parent retained source/result: b71c596bdd2f59fbb2893aa8847769f767660677, round 46. Round 47 is an isolated diagnostic census, not a speedup. The production patch must contain no census counters or exported getters.

A complete steady eight-byte level-12 computation executes 1,010 owning-wrapper paths; exactly 510 have an exclusive unadmitted root under Arc::get_mut and no budget ownership. Preserve the other 500 allocations. Expected difference: 510 allocations and 57,120 requested bytes per call, from 13,994 / 98,586,843 to 13,484 / 98,529,723. This is a prediction until verified by the allocator probe.

Reuse the existing root only after the existing validation, conservative byte/node envelope and registration preparation. Continue using the shared-data registry gate and concurrent deduplication. All fallible preparation precedes ownership/link publication. Shape/schema owners, frozen data identity and physical tickets remain intact. Shared/admitted/weakly held wrappers retain the original replacement path. Fresh roots intentionally omit the removed allocation failure point; registration and shared-wrapper failures remain observable.

Regression tests were committed before implementation. The baseline fails exactly the three mechanism-specific witnesses and passes the three negative/limit/concurrency controls. Preserve the passing-before and expected-failing-before receipts.

Fresh paired release builds are necessary because environmental cache loss rebuilt third-party artifacts with different hashes. Pin the same common dependencies on both sides and freeze every selected Core-dependent transitive artifact. Do not compare old versus rebuilt differences as though they came from this patch. Keep the existing selected operation features, profiles, warnings-as-errors and normalized host flags.

Run the unchanged complete abcdefgh computation only. Same source/MBC, raw bytes, independent inflate, input aliases/content, real tables/search limits/buffers/allocation sites, pinned allocator, capacity-13 clear/reuse and evaluate_into timer. Exact work remains 60,260,219,686 and comparison work 47,612; final cleanup is zero. Separate four matched primary pairs, CPU counters, allocator probe, Mech sampling and reference sampling. Immediately commit every measurement cohort. Run meaningful Core ownership/memory and downstream original/decoded qualification, scalar controls, architecture and CI-inventory contracts without changing goldens or allowances. Profile both actual executables numerically after the change; retain only a measured improvement.

Physical cleanup-domain analysis remains separate and unimplemented. Private matrix lifecycle, same-shape construction, shared shadows, physical arena slots, shared code, range facts and wider loads are later isolated candidates. Do not bundle them into this result.
