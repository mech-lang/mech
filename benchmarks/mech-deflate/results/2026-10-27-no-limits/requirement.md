# Explicit maximum resource budgets

Base: `45d18ea25922875b10d126050801f862daee7e74` (reviewed #966).

The assembled level-12 source needs approximately 120 MB of bit-packed definition facts. The previous hardcoded 64 MiB compiler allowance rejected it before execution. A later full-input attempt also found a 16 MiB library output-byte default, rejecting a 22,241,990-byte admission estimate. These are resource-policy failures, not missing compression operations.

`NativeProgramLimitMode::NoLimits` supplies every configurable native compilation, execution, library and managed-memory budget at its numeric maximum. One mode value provides `compilation()`, `execution()` and `memory_budget()`. The literal constructors enumerate every field, so adding a new field requires updating the mode. No call-site `Default` value can silently retain a small quota.

Native compilation now exposes `max_analysis_bytes`. JIT and Object lowering carry it through every current-visit proof pass. The normal default remains the published 64 MiB; the temporary local 512 MiB experiment is superseded. A rejected compiler analysis reports the stage, required bytes and allowed bytes as a structured `AnalysisLimit` error.

The immutable plan retains its analysis policy for subsequent proof passes. Policy is skipped during semantic image serialization. Restored images and same-Object rederivation check the same selected types, ownership and current-visit facts without reinstating the producer's compilation allowance. No bytecode revision, image format, operation certificate, generated operation or compressor algorithm changes.

## Acceptance

- Engine proof suite: 14 tests pass. A valid 24,576-copy chain exceeds the normal analysis allowance, is accepted at maximum, retains an identical semantic image, and still rejects a read of a future definition. Existing stale-carrier, unowned-edge, moved-phase, partial-match and scalar-route rejection cases pass.
- Native mode regression: 1 test passes for original/decoded artifacts. A zero-byte compiler allowance rejects with the actual analysis reason. An empty U8 buffer with 16,777,217 reserved bytes is rejected by the ordinary library budget, admitted by `NoLimits`, then exercised through bounded rejection and successful recovery with prior publication and cleanup checked.
- The first native regression invocation omitted its explicit `+> sequence` source import. That introduced test-scaffold failure was fixed; it was not a compiler failure. Its failing receipt is retained.
- Image/linked regressions: 2 tests pass, covering build-owned image rederivation and linked mixed-frame publication/recovery. Production routing, source/native catalogs, analysis ownership, CI inventory, formatting and diff checks all pass.

Total: 17 distinct Rust tests pass. CI is intentionally not awaited; drafts remain drafts.

The complete compressor source qualification is separate. It executes the reference algorithm itself and retains the pinned tables, search limits and buffer allocations. A full-input run is active; no completed full-file stream or fair performance comparison is claimed by this compiler patch.
