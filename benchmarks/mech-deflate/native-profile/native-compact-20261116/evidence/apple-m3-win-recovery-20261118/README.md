# Apple M3 compiler and ownership trials

These trials compare general scalar lowering, array backing and owner-storage
changes without changing the MechDeflate source, canonical bytecode, fixtures,
allocator or the pinned Roc and libdeflate 1.26 level-12 executables.

The latest complete integrated comparison remains **81/81 Roc in both orders,
58/81 libdeflate forward and 60/81 reverse**. Two subsequent completed forward
orders reached **81/81 Roc and 60/81 libdeflate**; background activity interrupted
their reverse orders. No new complete pair in this record reaches 61/81
libdeflate wins in both orders. The earlier 61/81 paired result remains a
separate historical build.

[Verified order summaries](results.json) · [Every completed case timing](timings.csv) ·
[Earlier complete integration](../apple-m3-rebase-20261118/README.md)

## Measurement and replay

The boundary is the in-memory native diagnostic call, including Heap
construction, final output transfer, physical private cleanup and Heap drop.
Preparation, validation and process startup are excluded. All four actors run
serially through 96 balanced blocks per case. Measurements stop if a sampled
nonbenchmark process exceeds the predeclared CPU limit; partial observations
are retained and cannot qualify as a complete paired result.

The CSV reports raw actor medians in seconds and Mech multiples of both peers.
Each multiple is the geometric mean of the 96 paired block ratios, rather than
a ratio of the separate actor medians. A ratio below 1 is a win, above 1 a loss,
and exactly 1 a tie. Complete orders and interrupted pairs are labelled
separately. All published completed orders were replayed from their raw records
with the unchanged qualification verifier.

## Source and correctness

The opt-in `MECH_COMPACT_LITERAL_ALLOCATION_SETUP=1` pass supplies proved
literal element geometry in small private routines. They tail-branch to the
original checked allocation entry. Error results, initialization, accounting,
ownership and physical cleanup are preserved. Its default-disabled native object
and complete metadata reproduce the preceding build byte for byte.

The independent [native ABI oracle](abi/README.md) passes 99 execution checks.
The prepared benchmark executables also pass all 162 exact output, inflation
and borrowed-alias checks, six injected failure classes with recovery, and stack
checks. The 5,280-byte stack limit includes added caller-owned descriptor
storage and conservative alignment. Prototype ownership registries remain
separate experiments in the archive.

Compiler source for the literal-setup trial is commit `5e91c17df7`. Subsequent
commit `e981b748cf` integrates the upstream checked heap-call setup changes;
measurements of that integration are pending in this snapshot.

## Complete evidence package

[Download the verified 355,731,581-byte archive](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/pr995-deflate-m3-win-recovery-20261118-98256dc25c69.tar.gz).

SHA-256: `98256dc25c69b42c145ef899f3657f7b55c82baf364b375ab47fa77989c04d3d`.
The manifest verifies all 6,922 included members and identifies the preceding
archive by hash. It includes successful, adverse and interrupted trials,
compiler and runtime source snapshots, exact commands, pinned worker identities,
proof receipts, profiles and raw observations. Large rebuildable compiler and
test executables are recorded by hash with their source and build commands.
