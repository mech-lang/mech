# Canonical control admission diagnostics

Base: `a83cb88378041ae8f6829fda9300a245aeac1d51` (reviewed #964).

The complete level-12 compressor source reaches canonical artifact finalization and is rejected at node 3 with only `InvalidControl { reason: "control graph admission limit" }`. The existing bounded count walk collapses nesting depth, arms, blocks, operations, operands, local-slot admission and arithmetic overflow into that one result. The trigger is preserved in the parent review packet's `complete-compress-large-test-stack` receipt.

The diagnostic now returns `ControlAdmissionLimit { node, resource, required, limit }`. Node-local resources and artifact-wide totals are identified separately. `ControlAdmissionOverflow { node, resource }` distinguishes arithmetic overflow without fabricating a count. Structural pattern admission remains a separate invalid-control reason. The errors propagate through artifact finalization unchanged; no temporary tracing or unchecked artifact construction is needed.

`required` records the usage observed at the first rejected step. It is not necessarily a complete-program total. Traversal still stops at the first denial and preserves the existing accounting order. Every admission bound stays unchanged: 4,096 arms, 8,192 blocks, 65,536 operations, 262,144 operands, 65,536 local slots and depth 32. The U16 local encoding, wire format, default decoder limits, and legacy-revision rejection are unchanged.

## Validation

The four JSON receipts preserve exact commands, feature profile, HEAD, worktree differences, environment, durations and results. `validated-sources.json` records the tested compiler contents. All Rust commands use `mech-engine --no-default-features --features full_source,native-artifact,sequence_buffers --lib`.

- `artifact::control::tests`: 14 pass, including seven new tests for each count class, cross-node totals and overflow, and strengthened nesting-boundary evidence.
- `artifact::bytecode::`: 21 pass, including bounded control preflight before typed allocation, original/decoded round trips and strict old-revision rejection.
- `lexical_collection_aggregate_step_limits_match_default_decode`: 1 passes at the unchanged source/wire operation boundary, now checking the structured rejection fields.
- Routing, catalogs, architecture, CI inventory, formatting and diff checks pass.

Total: 36 distinct Rust tests pass. Small metadata graphs exercise rejection paths without changing the compressor algorithm or adding runtime code to user programs. The separate complete-program rerun will identify the real compressor's first exceeded resource; this diagnostic patch does not increase any admission allowance or claim complete compression.
