# First-admission eligibility census

Diagnostic only; no elapsed-time result. Baseline b71c596bdd2f59fbb2893aa8847769f767660677. Counter implementation 3b0d6a9ff (full revision recorded in build receipts), built at c33d3e62780a6a4a4dadaabb4810ab5b11dee60a. All instrumentation is removed before the ownership experiment.

Eight steady complete eight-byte calls each observed 1,011 admissions: one same-account shortcut, zero import shortcuts, 1,010 wrapper paths, 510 exclusive unadmitted roots, and zero eligible roots sharing data. Original allocation count 13,994 and requested bytes 98,586,843 remain unchanged. The isolated optimization predicts 510 fewer 112-byte wrapper requests, or 57,120 bytes per call. Shared-data registry synchronization remains required and tested despite zero such eligible values in this input.

Exact raw bytes, independent decompression, aliases/content, capacity 13, work 60,260,219,686, comparison work 47,612 and final zero cleanup pass. Profiles and timing from this instrumented build are excluded.

The selected-operation release build succeeded in 1,189.553 seconds after the environmental cache loss documented in round 46. A receipt adapter initially used the wrong script filename; its resume preserves this completed build. A dependency hash check found rebuilt third-party differences despite equal feature/profile settings. The census uses one coherent frozen graph; no old-versus-rebuilt elapsed claim is made. Round 48 requires a freshly matched baseline against the same common dependencies.

Physical cleanup-domain investigation remains analysis only, with explicit outstanding producer/fallback and restored-image proof obligations. It is not bundled with ownership.
