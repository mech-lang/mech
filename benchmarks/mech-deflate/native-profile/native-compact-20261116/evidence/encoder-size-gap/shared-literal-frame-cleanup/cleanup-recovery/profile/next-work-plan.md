# Reuse the existing immutable use index in loop-route discovery

Current pool: the exact 64,444-byte worker's final cleanup has 223 stack samples across three complete process captures. Route searches account for 95 samples (42.6%), verification 50 (22.4%). The uninstru mented matched recipe costs 94.835 ms more than the 64,652-byte control. The initial two-second captures ended before final cleanup and are excluded from attribution, with their raw data preserved.

Removed work: loop routes currently walk reachable instructions separately for every skipped SSA definition. The acyclic identity-route pass already indexes uses once and applies exact stopped reachability and dominance checks. Move that existing index into shared route support and let the loop pass optionally use it during its immutable planning phase. Build it lazily only if a candidate reaches live-out discovery.

Expected effect: reduce repeated graph/instruction visits, retain the exact native encoder, metadata, bytecode and worker. Sampling bounds a pool, not an established saving. Use same-executable off/on complete source-to-link trials with current peers for the decision.

Correctness: never apply routes until all planning completes. Each index belongs to that one unchanged CFG. Preserve fresh-definition stop semantics, current dominance, branch-argument uses, aliases, all effect/exception refusals and SSA repair. Keep the original graph traversal as the off path. Differential-test the indexed query and selected IR against it; keep existing runtime oracle tests. Retain unconditional verification. No runtime proof or resource admission is weakened.

Decision: compare off/on artifacts and the exact fixed-support worker before inheriting runtime counts. Require source checks and matched timing before updating the named recipe. Keep unqualified variants and adverse measurements in the record. Do not change the Mech program or peer boundary.
