# Rejected frontend cache trials

Both changes are rejected and production source is restored. Normalized-type
caching saves about 16 ms in frontend work but slows total compilation in both
balanced blocks. Preserving the canonical schema owner saves about 58 ms in
frontend work; complete executable builds remain flat with mixed blocks.
Neither establishes the complete-build improvement needed for the ten-second
goal. The previously retained compiler remains unchanged.

Both frozen trials start at `f6cdecef644a7eb9e2091b685542b2ae5e405da8`, whose
compiler source is the retained scoped-source implementation. They share the
same qualified control binary, current Deflate/runtime, 61 program flags and
two workers. The standard optimized compiler profile uses Thin LTO and one
codegen unit without PGO or host-CPU overrides. Earlier cohorts stay separate.

## Changes

The first trial adds a lazy normalized-type result to each immutable pending
schema owner. It runs the original `ResolvedType::from_schema_body` on first
demand, caches errors as well as success, and invalidates the result on every
mutable borrow. Raw destination drafts retain their uncached path. Its patch
SHA-256 is
`8027cc5c8cae67c30a4baabd64db7daf02ab728c3771e324d3f1c5f9eaf915b3`.

The second is independent and contains no normalized-type cache. Annotation
comparison accepts the existing finalization trait, preserving a pending
owner's canonicalization cache. Conformance borrows that owner instead of
erasing it to a raw draft. Both original canonicalization demands still run,
including the second when the first fails. The dimension relation mapping and
downstream conversion checks keep their order. Core validation and production
pending-schema storage are unchanged. Its patch SHA-256 is
`634b4e5baabc9847dc7790ef3a22a31ff1d55fa20d21ff11a7261780c2effd96`.

## Results

Each trial has eight serial source-to-Object observations in ABBA/BAAB order.
Screens, compiler construction, tests and cached executable gates are separate.
No heavy build, test, sampling, hashing or archival overlaps primary timing.
Nested phase intervals overlap and are not added together.

| Measurement | Type-cache control → candidate | Annotation-owner control → candidate |
| --- | ---: | ---: |
| Frontend | 2.567190 → 2.551027 s (-0.63%) | 2.565313 → 2.506961 s (-2.27%) |
| Internal compiler phases | 18.438515 → 18.511892 s (+0.40%) | 18.446353 → 18.447271 s (+0.005%) |
| Complete source-to-Object process | 18.496191 → 18.574497 s (+0.42%) | 18.505289 → 18.505185 s (-0.0006%) |
| Retired instructions | 275.380258 → 275.042013 G (-0.12%) | 275.380401 → 274.465614 G (-0.33%) |
| Maximum RSS | +0.068% | +0.031% |

Frontend and instructions improve in both blocks for each trial. Type-cache
compiler blocks are 0.11% and 0.69% slower. Annotation-owner compiler blocks are
0.35% faster and 0.36% slower. Source CFG construction is adverse at +2.08% and
+1.24%; parsing, preparation, footprint and other adverse observations remain
in the [complete result](../../benchmarks/mech-deflate/native-profile/frontend-cache-trials-apple-m3-20261115.json).
No balanced full-native cohort is run for the rejected type-cache trial.

Annotation-owner reuse then receives eight separate complete source-to-native-
executable builds. Means are **18.563041 → 18.587745 seconds (+0.13%)**; blocks
are +1.08% and -0.80%. The faster first control, 18.192438 seconds, remains.
Link time moves favorably without a linker change and is not credited to this
optimization. Timings include fresh source processing, optimization/code
generation, Object/metadata writes, handoff and final link. Fixed current
runtime/entry adapter objects are cached; first-use construction is excluded.

## Equivalence and evidence

All 15 exact copied-module tests pass for type caching, checking full normalized
predicate evidence, lazy demand, unused-declaration errors, mutation/repair and
concurrent readers. All 28 pass for annotation-owner reuse: both full original
and candidate module suites, record-axis relationships and new cache/error/
invalidation/demand tests. Initial prose undercounts the annotation suite; the
unchanged test stdout records 28 passes and its correction remains in evidence.

Each trial passes fourteen fixture pairs, every source/bytecode/Object/complete-
metadata byte gate and two cached full-executable gates. All eight annotation
native observations also preserve every executable byte, retaining SHA-256
`d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
No grammar/language change, weaker fresh proof, scalar retirement or new
emitted-program runtime campaign is claimed. Both sources and the qualified
canonical compiler tool are restored; rejected library caches need rebuilding.

The thirty-third [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains both sources/patches, all commands/logs, tests, observations, program/
executable bytes, fixed native inputs and pure replay. Historical scoped-source
stack diagnostics support target selection only. The first exploratory parser
includes the flat top-of-stack table; its result and the corrected call-graph
recipe are preserved. No CPU-share or timing claim is derived from those paths.
Compiler/test executables and dependency libraries are excluded and hash-pinned.
Fresh repository extraction verifies every file and Git blob; offline replay
applies both patches and reconstructs every mean, block and byte gate.
