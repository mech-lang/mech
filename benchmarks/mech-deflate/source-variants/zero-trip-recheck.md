# Zero-trip guards on the current native compiler

Acceptance plan, before new observations: retain compiler `914ab57fab23f831f6bb99e5f8a0e8419d2c3004`
and its published R125 dependencies, allocator and native configuration. Compare
the unchanged compressor with the existing `zero_trip.rs` source variant. No
compiler modification belongs to this comparison.

The pinned source/MBC SHA-256 values are
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` /
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
The variant source/MBC SHA-256 values are
`85fba4bc9f55796c8a83c7ff0533590d4bf5e97bd8be42d4dd56c5d448427b36` /
`5d9318ae144d5d42b3bee53dd23ff6e26955ef6652c5f93ef05eeadf69a9ca26`.

Only zero-match and zero-skip branches differ. Tables, search limits, real scratch
capacities, allocation sites and output clear/reuse remain fixed. Timing covers
the complete `evaluate_into` call; setup, clear, rebinding and verification remain
outside it. Each source has its own independently observed work value.

Measure 8B, random 1KB and repeated-byte 1KB with alternating blocks and the
inactive peer paused. Keep the established 8B protocol and 128 observations per
source for each 1KB case. Require exact reference bytes, independent inflate,
input aliases, original/decoded agreement and zero final managed bytes/nodes.
Retain adverse observations. Gather instructions and allocation counts in
separate cohorts. No builds, tests or profiles overlap primary measurements.

Retention depends on the observed elapsed and resource tradeoffs; historical R118
gains are not predictions for this compiler. Keep the unchanged-source baseline
even if the variant is useful. This round does not implement no-match block
simplification, vector specialization or packed loads.
