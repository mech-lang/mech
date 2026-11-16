# Integrated numeric phase compilation workers

The actual compact Deflate compiler now supports persistent machine-code
workers. Enable the measured configuration with
`MECH_COMPACT_COMPILE_THREADS=2`; the default remains serial (`0`). One and four
workers are also accepted. This scheduling setting is separate from the same
61 program code-generation flags.

Source commit: `1090de011646c92e30c2d4b648ed0792c50f300d`, based on
`0195125f11747fb5dccdeb02c482d4a0c85ccb1f`. Its complete seven-file diff matches
the patch captured before checking and building the compiler.

The later [updated-compressor cohort](native-refreshed-parallel-compile-profile.md)
incorporates PR #973's newer source/runtime while keeping these seven worker
files unchanged. Its complete-build means are 19.166 seconds serial and 19.196
seconds with two workers, with mixed blocks and no useful saving established.
The measurements below remain specific to the preceding smaller workload.

## Actual implementation

The coordinator constructs and cleans up the general phase's IR, declares its
function, and queues its owned Context. While a worker compiles it, the
coordinator prepares the vector variant against a `RecordedModule`. That module
stages declarations, data definitions and nested function definitions without
mutating the real Object. The vector Context can then enter the worker queue.

Publication waits for the general function first, replays all staged events in
their original order and checks IDs, then publishes the vector function. Variant
code comparisons, no-growth choices and shared `DenseTables` retain their current
order and dependencies. A report completion closure reads actual compiled code
size and stack metadata; no estimated or placeholder machine metrics are used.

Workers share the Object's immutable ISA and own separate Contexts/control
planes. The bounded queue moves Contexts without cloning their IR. Original
earlier errors return before later publication errors. The zero-worker path
retains synchronous `Module::define_function`.

This first implementation overlaps helper machine compilation with later helper
preparation. Parsing, frontend, native preparation, main CFG construction and
cleanup remain serial. It does not yet send the whole helper construction and
cleanup pipeline to workers or overlap unresolved phase choices.

## Balanced source-to-executable results

Eight serial complete builds use ABBA / BAAB order, the fixed cached runtime
adapter and identical native linker inputs. Both blocks improve. Each timed
build includes fresh source assembly/analysis, optimization/code generation,
program Object/metadata writes, Object handoff and the final native link.

| Boundary | Serial mean | Two-worker mean | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 17.656797 s | 17.379311 s | -1.57% |
| Source to Object within those builds | 17.518028 s | 17.253258 s | -1.51% |
| Final native link | 0.137866 s | 0.125081 s | -9.27% |

The complete-build saving is approximately **0.277 seconds**. The two blocks
improve 1.47% and 1.67%. Native-link movement is observational and is not
attributed to the worker change. Adapter first-use construction is excluded;
the program analysis and code generation are fresh on every run.

Before this cohort, a single full-build pair reverses direction: 17.191 seconds
serial and 17.292 seconds with workers. That adverse pair remains in the packet
and is excluded from the balanced means. It prompted qualification at the full
executable boundary. Per-build native receipts retain the generic one-shot
`primary_cohort:false` field; the orchestration's `native-cohort.json` explicitly
records which observations form the balanced cohort. No clocks are rewritten.

## Separate source-to-Object cohort

Eight additional serial ABBA / BAAB runs use the precise process-completion
driver and native instruction/memory counters. They are a separate cohort;
their means are not pooled with complete executable builds or old polling-tail
measurements.

| Work | Serial mean | Two-worker mean | Change |
| --- | ---: | ---: | ---: |
| Complete source-to-Object process | 17.292313 s | 17.124986 s | -0.97% |
| Internal compiler phases | 17.232531 s | 17.071035 s | -0.94% |
| Numeric helper generation | 2.137102 s | 2.014813 s | -5.72% |
| Retired instructions | 255.938289 G | 255.895546 G | -0.02% |
| Maximum compiler RSS | 5,912,449,024 B | 5,917,089,792 B | +0.08% |
| Frontend | 2.380468 s | 2.387993 s | +0.32% |
| Value compaction | 0.161514 s | 0.164912 s | +2.10% |

Compiler phases improve 0.56% and 1.31% in the two blocks, and helper time
improves in both. All phases, catalog/compaction regressions, slight memory
increase and unrelated cleanup movement remain in the
[complete numerical result](../../benchmarks/mech-deflate/native-profile/parallel-phases-compile-apple-m3-20261114.json).
The result supports a small elapsed-time gain, not a substantial reduction in
total instructions or a large fraction of the ten-second gap.

## Correctness and evidence

The real two-worker diagnostic queues **340 function jobs**. Full Deflate runs
with zero, one and two workers all reproduce every source, bytecode, Object and
complete metadata byte. All fourteen paired scalar/nested numeric fixtures
match. Both timing cohorts and every complete executable comparison retain the
1,878,568-byte executable with SHA-256
`557d0f5ef2b979e01851ae23b5b3944453a0bd9e772d68161be198916ca06c2c`.
No scalar retirement, grammar/language change, weaker proof check or new runtime
campaign is claimed.

Two focused production-module tests pass. One performs 24 complete Object
comparisons for one/two/four workers, both submission orders, libcall symbols,
repeated and anonymous declarations, interleaved data and function-address
relocations. The other preserves an earlier original publication error before
invalid later data. The first test-setup attempt finds a baseline Cargo rlib
removed while rebuilding; no tests execute in that attempt. The successful
harness uses the exact production worker module and real `NativeCompileError`
type, with production-equivalent backend error construction.

Compiler checking takes 33.064 seconds and rebuilding takes 259.231 seconds;
both are outside program compilation measurements. All builds, tests, profiling,
hashing and archival work are outside primary cohorts. Preliminary cold-start
screens and worker diagnostics are excluded from primary timing means.

Retain the opt-in worker path. The twenty-fifth
[complete packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
includes every command, observation, adverse result, exact source/patch, test
and fixture output, full program/executable and frozen native linker input.
Compiler/test executables are excluded and SHA-pinned. The repository copy has
been extracted, manifest-verified and replayed byte-identically against its
compact summary. Pure replay checks saved data without rerunning compilers or
timings. Complete compilation below ten seconds remains unachieved.
