# Next bounded candidate: transfer dead private-frame owners

The current profile places 345/1227 ancestry weights under private-frame
handling; 29 are under `BufferFrames::copy`. This is evidence of a reached
operation, not a predicted 2.36% saving. Large path reservations remain a
separate ownership candidate; they are not an 85 MB copying result.

The maintained `Operation::Copy` currently clones each managed field into the
destination and then runs the already-derived `action.retire()` list. Where
that exact list proves the source frame dead, investigate moving its owners
after the existing complete copy validation instead of cloning and immediately
dropping them. Round 43 moved disposable **pack inputs**; this candidate concerns
ordinary frame-to-frame Copy and must not be presented as the same optimization.

First establish a minimal tuple/record witness and a diagnostic eligible/executed
count on complete compression. It must cover a dead source and a live source,
nested composites, scalar fields, different buffers on repeated visits, equal
source/destination, and possible physical lane overlap. Reuse the maintained
same-plan last-use and field layout rather than inventing another liveness or
ownership ledger. Decline whenever disjointness or last use is uncertain.

Before production code, add a focused acceptance test that fails specifically
because the dead source still owns its field after Copy. Require original and
decoded forms, unchanged original prices and finite one-short failure prefixes,
late invalid fields before mutation, prior output/alias preservation, retry with
fresh data, and final cleanup. Validate the whole operation before taking an
owner. Keep live-source cloning and all fallback paths.

Then make the smallest general change, rerun the same eight-byte matched timing,
hardware/allocation cohorts, and profile the actual executable again. Preserve
negative or small results and do not promise a gain from sampled ancestry.
No benchmark source, algorithm, table, allocation site, or timer change is
authorized by this candidate. This round has not implemented it or established
its eligibility count.
