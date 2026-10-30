# Native parent pieces around ordinary scalar child control

Previous goal turn: progress. Published starting head
`369583114cdc10b60372eebaff4b63e3a2721fb6` establishes the round-38 memory
reduction but no elapsed gain: 4.6224375→4.6378545 ms, 15.88466×→15.93764× the
shared reference. Current measured production source is dc77024f2.

The actual unchanged optimal-path scan accounts for 606 + 1212 + 606 ordinary
admissions in parent iterations 136/170/300. Its immediate child is
OptimalPathMatches, with a scalar record result and nested OptimalPathLengths
control. These child carriers are already connected private scalar aggregates.
Current composition requires an immediate qualified leaf returning an array
frame and skips this whole parent. The prefix/suffix themselves use existing
scalar, matrix-read and private-frame operations. No parent loan or owner needs
to survive the child boundary.

First add a failing mechanism check to the unchanged real optimal-path fixture.
Then admit existing acyclic native parent pieces around an ordinary child only
when its result is an already-proved flat scalar aggregate, and its entire
descendant loop tree has private scalar carriers with no buffer-frame carrier.
The last condition keeps the existing single pending child edge sufficient:
none of those descendants can select a second private-frame parent composition.
Retain the original qualified array-child path, append/multiple-child refusal,
ordinary child initializer/exit, finite funding/fallback, error reconciliation,
ordered retirement, publication, aliases, cleanup and retry.

Reuse the same derivation during compilation and build-owned image restoration.
Do not add an executor, owner bank, continuation stack, source rewrite, resource
allowance or new operation. Every piece closes before the ordinary child; its
successor reads that visit's current result. Add actual callback and recovery/
finite-boundary tests, retain existing component expectations and scalar/frame
controls, then measure the same complete eight-byte compression against frozen
round 38. Inspect actual instruction/allocation counters and fresh binary/profile
afterward; commit each observation immediately. No larger-input timing or CI wait.
