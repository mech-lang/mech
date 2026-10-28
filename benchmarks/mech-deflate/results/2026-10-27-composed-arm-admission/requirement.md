# Shared arm admission for composed programs

Base: `04de04a8537e3ee64330a01b0dea645c81d51392` (reviewed #965).

The complete level-12 compressor source emits 4,867 control arms in one root control declaration. The shared canonical count validator stops at 4,097 against its 4,096-arm allowance. The source is accepted by the maintained frontend; no native computation has executed at this checkpoint.

A read-only observation of the checked `SourceProgram`, before normal artifact finalization, measures the entire graph: 4,867 arms, 4,991 blocks, 18,121 operations, 60,897 operands, 21,270 local slots and maximum depth 20. Only the arm bound is exceeded. `measured-program.json` records the exact source byte count, source hash, compiler revision and pinned source revision. The complete source remains a separate compressor-assembly qualification; the ordinary Boolean reproducer in this patch establishes this compiler capacity independently of that work.

Raise only `MAX_CONTROL_ARMS` from 4,096 to 8,192. Artifact finalization and default wire preflight already share this constant. Keep the 8,192-block, 65,536-operation, 262,144-operand, 65,536-local-slot and depth-32 bounds unchanged. The U16 local encoding, bytecode revision, semantic representation and execution code are unchanged.

The regression builds an ordinary function with 2,049 Boolean checks. Its compiled graph exceeds the previous arm allowance, finalizes under the new allowance, and round-trips with default decoding. An explicitly narrower 4,096-arm decoder profile still rejects it before typed graph allocation. The corrected reproducer fails on the parent allowance with the same `ControlAdmissionLimit` as the real compressor. The first harness build used `&String` where an `&str` was needed; that test-only compile error was corrected before collecting the real rejection.

## Validation

All commands and environments are preserved in JSON receipts, with compressed full logs. Tests use `mech-engine --no-default-features --features full_source,native-artifact,sequence_buffers --lib`.

- `artifact::control::tests`: 15 pass, including the new source/codec regression and the unchanged other resource boundaries. The source qualification takes 74.68 seconds; its supporting metadata checks are small.
- `artifact::bytecode::`: 21 pass, including finite preflight, strict previous-revision rejection and canonical round trips.
- `lexical_collection_aggregate_step_limits_match_default_decode`: 1 passes at the unchanged operation bound.
- Routing, source catalog, native host catalog, architecture, CI inventory, formatting and diff checks pass.

Total: 37 distinct Rust tests pass. The full compressor rerun with the new arm allowance is pending; no existing component expected value or qualification allowance was changed. Any later native resource denial will be reported separately, rather than treated as proof of complete compression.
