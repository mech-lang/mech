# Borrow call-argument schemas during type resolution

`resolve_declared_call_context` previously materialized a complete draft for each
input, then borrowed its body and dimension declarations to construct a
`ResolvedType`. It now borrows the existing draft or destination directly. The
constructor, contextual checks, overload decisions and error anchors are
unchanged. No language, grammar, ownership proof or program optimization changes.

The separate [work-counter diagnostic](native-parallel-compilation.md) found
15,497 shared materializations copying 5,911,032 declaration entries at this
site. This candidate removes that materialization; it does not cache type
resolution or introduce diagnostic instrumentation into production.

Source commit: `f9786cb1ad4479727c1af8bf013f357a6e3ab01c`, based on
`bb9c4b35919174c6d5daf28542609ff4f7682bc8`. The source commit's complete diff
matches the patch captured before the candidate compiler build.

## Matched results

Eight serial level-12 compiles use the same 61 flags and optimized `compiler`
profile, in ABBA / BAAB order. Builds, fixtures, screens, native links and
profiling are outside this cohort. Both balanced blocks improve for frontend,
compiler, process and retired instructions.

| Work | Control mean | Candidate mean | Change |
| --- | ---: | ---: | ---: |
| Frontend | 2.545154 s | 2.522899 s | -0.87% |
| Internal compiler phases | 17.528712 s | 17.428234 s | -0.57% |
| Complete source-to-Object process | 17.592817 s | 17.482488 s | -0.63% |
| Retired instructions | 257.658929 G | 257.380511 G | -0.11% |
| Maximum compiler RSS | 5,912,285,184 B | 5,910,593,536 B | -0.03% |
| Parsing | 2.080881 s | 2.082272 s | +0.07% |
| Catalog | 0.036746 s | 0.037874 s | +3.07% |
| Main cleanup including compaction | 3.412079 s | 3.424553 s | +0.37% |

The targeted frontend saving is about 22 milliseconds. Native time also moves
down in this cohort; that unrelated movement is not attributed to the frontend
source change. All phases, nested intervals, counters and adverse observations
remain in the [numerical result](../../benchmarks/mech-deflate/native-profile/call-schema-borrow-compile-apple-m3-20261114.json).
The compiler-time blocks improve 0.45% and 0.70%. This is a small retained work
reduction, with no claim that it materially closes the ten-second gap.

The process driver now timestamps `process.wait()` completion. Historical
polling-tail measurements remain unchanged and separate. The internal timer
still excludes source assembly/write and final Object/metadata writes. The two
preliminary screens have approximately one-second startup gaps; neither is
included in the primary timing means. No cause for those gaps is established.

## Output and executable gates

Both screens, all eight primary compiles and all fourteen scalar/nested numeric
fixture pairs reproduce every source, bytecode, Object and complete metadata
byte. The fixed level-12 outputs retain these hashes:

| Output | Bytes | SHA-256 |
| --- | ---: | --- |
| Object | 1,238,656 | `afbb34f3e2a8b3ae21be659ee50f332c849fce4047d2a98e40f7a23fe187cd38` |
| Complete metadata | 2,292,881 | `4a6e18df12a127eed6537a103bcd6f51a18027bd867efe709674d3645576aecb` |
| Final executable | 1,878,568 | `557d0f5ef2b979e01851ae23b5b3944453a0bd9e772d68161be198916ca06c2c` |

The final executable comparison uses the frozen native adapter objects and full
link inputs from the [parallel planning packet](native-parallel-compilation.md),
canonical object paths and `ZERO_AR_DATE=1`. Control and candidate native links
take 0.122 and 0.089 seconds; these are link-only observations, not matched
complete source-to-executable timings. The generated executable is identical,
so no fresh runtime campaign or scalar retirement is claimed.

The separate compiler rebuild takes 374.220 seconds with four Cargo jobs and
is excluded from Mech compilation. Rebuilding the Rust compiler is distinct
from the proposed parallel compilation of Mech function jobs.

## Evidence and decision

Retain this borrowing change. The twenty-third
[complete evidence packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains every raw observation, build log, command, exact source/patch, fixture
input/output, program Object, metadata, final executable and frozen native linker
input. Compiler executables are excluded and SHA-pinned. Pure replay verifies
the saved data and computes every reported mean without running compilers or
timings. The repository archive has been extracted, manifest-verified and
replayed byte-identically against its compact summary.

The [parallel compiler architecture](native-parallel-compilation.md) remains a
design requiring a one-worker equivalence implementation and then measured
two-worker scheduling. Compilation below ten seconds remains unachieved.
