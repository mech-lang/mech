# Transfer dead private-frame owners after the original preflight

Parent 8ef19b0c8a899a0f0f4f8d09433e5745368f51c3 is published and clean. Round 52 is progress: it provides a fresh current profile and distinguishes reservations from copied bytes. Parity remains open at 13.696806 times its fresh reference, with no production change in that round.

The actual profile reaches BufferFrames::copy (29 of 1227 ancestry weights) within frame handling (345). The current implementation clones managed fields into the destination and then executes the original same-plan retirement. Count executed last-use candidates on the unchanged complete compressor first. Counts and samples do not predict a speedup.

The proposed bounded change passes the exact prepared frame action into Copy. After the unchanged complete field validation and original charge, move managed owners only when that action already retires its source. Keep live-source copying, scalar readiness and original ordered retirement. All selected root layouts have disjoint recursively allocated lanes in the maintained Engine derivation; do not infer disjointness from payload identity. Self/overlap cases must keep the conservative path. No payload is mutated.

Add an actual-layout failing acceptance test before production code. Cover flat/nested fields, real dead/live actions, global lane disjointness, self copy, late invalid fields before mutation, exact/one-short copy and retirement funding, repeated fresh arguments, output/alias preservation, errors/retry and zero managed cleanup, original/decoded. Preserve efficient scalar entry and architecture boundaries. Keep any CI selection/count update separately reviewable.

Only GPU runtime owner transport changes. Keep the frozen coherent Core/Engine/Stdlib/Compute graph, source/bytecode, algorithms, real tree/cache/path sizes, source allocation sites, pinned allocator, timer, work 60260219686 and prior comparison work 47612 unchanged. Repeat matched complete eight-byte timing, hardware, allocation and executable samples. Commit each cohort immediately. Retain negative/small results, report in multiples of one shared fresh reference, and keep PR 973 draft.
