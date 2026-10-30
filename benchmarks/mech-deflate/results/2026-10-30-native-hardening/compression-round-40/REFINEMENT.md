# Rebuild ordinary groups around selected pieces

The first valid candidate (`411c2e1f01d9435a1c78d28f3a0a123dcd2ba56e`)
regresses in all four independently prepared pairs: 4.370542→4.409583 ms,
15.22837×→15.36440× the SAME fresh 0.287 ms reference (+0.89328%). Only 5/64
descriptive blocks improve. Hardware instructions rise 49,008,964.5→49,214,104.5
(+0.41858%); allocations (13,994), requested bytes (98,586,843) and additional
peak heap (20,575,432) stay unchanged. Both linked executables have fresh 695-call
profiles. These results are retained and are not called an improvement.

Static evidence: selected pieces remain 67, but excluding entire intersecting
ordinary groups reduces grouped sites 8642→8406 and groups 1372→1285. It also
loses the actual scalar tuple/enum state packs immediately after the optimal-path
scan's native prefix (full-program sites 12541/12542, plus the other reachable
copies). Those two existing packs can share admission while remaining outside
the piece and before the unchanged child initializer.

Strengthen the real path fixture's acceptance before changing code: its parent
must group these scalar state packs, while all final groups and native pieces
remain disjoint. The first candidate fails that exact new assertion using its
frozen release library. No source program or algorithm changes.

Refinement: reuse the SAME acyclic admission derivation with selected piece
sites excluded, instead of deleting each complete intersecting group. This
creates an ordinary entry at the remaining suffix, with its own exact source
frontiers and unchanged finite prices. Select pieces once from raw groups;
retain owner-backed-group priority and the existing conservative entry refusal.
Do not iteratively add newly exposed pieces. No new executor, owner, bank, wire
field, operation, price or source allowance. Source preparation and checked image
restoration use the same raw→pieces→outside-groups sequence.

Programs without selected pieces keep their raw layout without a second pass.
All original/decoded, finite fallback, error-order, publication, alias, cleanup,
retry and scalar controls remain. Repeat matched isolated complete eight-byte
timing; compare before, first candidate and refinement together with a fresh
shared reference. Repeat separate hardware/allocation counters and profiles.
