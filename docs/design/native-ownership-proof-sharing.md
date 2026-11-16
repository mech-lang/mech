# Share converged ownership proof states

The compact compiler emits the exact same Object and executable while spending less time on ownership analysis. This is a compile-time change. The [81-case compression scoreboard](native-compression-scoreboard.md) still describes the same executable.

| Complete compiler run | Retained | Shared states | After / before |
| --- | ---: | ---: | ---: |
| Original `.mec` source | 672.689 s | 436.229 s | **0.649×** |
| Decoded revision-14 bytecode | 574.907 s | 396.210 s | **0.689×** |
| Recipes to source CFG, original | 432.890 s | 236.992 s | **0.547×** |
| Recipes to source CFG, decoded | 446.408 s | 265.114 s | **0.594×** |

These are individual compiler wall-clock receipts from separate launches. They do not estimate run-to-run variance. Some independent local qualification/build work overlapped. The frontend interval in the source launches was 57.486 s and 61.401 s; this interval includes parsing and source semantic compilation. The decoded launch reads an existing bytecode artifact and does not parse Mech source. An earlier diagnostic run with the retained ARM64 selection flag off took 368.873 s and emitted the corresponding pre-selection Object. It is recorded separately from the matching comparison above.

Ownership analysis asks which Boolean facts hold when it decides whether an old reference can be transferred. A path that crosses a straight-line edge typically does not change those facts. The old implementation copied both ordered maps and recomputed their fixed point. The revised state shares immutable maps within the current compilation, copies only a map that is about to change, and skips closure when that state is already at its fixed point. Defining or redefining an operand expires the same relations; a changed join intersects the same keys and invalidates closure. The ordering of inferred facts and the existing behavior when a fact is already present are unchanged. No state survives into execution or another compilation. The ownership decision, source checks, allocation, publication and recovery paths are unchanged in the exact output.

Base `36732e9fc3ddbd906f3f3f82aac2470298526146`. The optimized compiler uses nightly 2026-03-03, `aot,canonical-native,canonical-native-program,canonical-native-program-runtime,canonical-native-runtime,jit` and `-D warnings`. The benchmark source SHA-256 is `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`. The two builds match the retained revision-14 bytecode and their respective descriptors; both emit Object `dea88d78758f9d72251d7bbbb41bedcd9cabed2d143cb092b8cbf89625ed9ea1` and relink to timed executable `8fc50d3bc5b78bbb4e34807782250b19d8e828ef3a5b5f6378281fea6be1d81e`. There is no new runtime timing or profile claim.

All seven ownership-guard and eight private-write-chain unit tests pass with warnings as errors. A separate oracle compares the old and new transfer and join states at 786,432 points, with 52,164 joins and 25,859 changing joins. The two executable forms pass 324 fresh complete linked checks, preserving the exact stream, independent inflation, capacities, live aliases and cleanup. Changed-file formatting, compiler-planning quarantine, native host catalog, resident routing and interactive architecture checks pass.

The initial direct test build used the production dependency closure, which omits test-only externs and fixtures. Adding the dev externs exposed incompatible crate identities in unrelated runtime-source tests; the failed commands are archived. The focused closure uses the matching syntax/core/engine dependency set and excludes five unrelated test modules and their now-unreferenced test-only helper. It retains warnings as errors and does not modify production sources. Full CI and a separate managed performance rerun are not claimed.

[Compiler patch, exact build receipts and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-36732e9fc3-shared-owner-guards.tar.gz). SHA-256 `86b887b3fd09f34ca9b1af6a2df51a09ea5f9e2f5f55d4261e9b2d94b1c75fb0`.
