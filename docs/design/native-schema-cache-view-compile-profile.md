# Preserve schema cache owners through assembly and visitors

Schema-table assembly and nested control visitors accepted `&SchemaDraft`.
Passing a `PendingSchema` through those interfaces erased its owner by deref
coercion, selecting uncached canonicalization. A private `SourceSchemaRef` now
distinguishes borrowed pending owners from borrowed raw drafts. Owner-backed
requests use the existing pure canonicalization cache; raw drafts keep their
existing path.

Visitation, schema insertion, validation and error anchoring retain their order.
The cache contains pure schema data. Shape, factory, ownership and current-visit
proofs remain fresh. No grammar, language or program optimization changes.

Source commit: `2822e6d8905f73873936a108b0d7c6b20231217d`, based on
`4057eb4364d496c66bad911394249007b1426590`. Its complete diff matches the patch
captured before building the candidate. The control already includes the
qualified [call-argument borrowing change](native-call-schema-borrow-compile-profile.md).

## Results and decision

Retain this targeted work reduction. Eight serial 61-flag compiles in ABBA/BAAB
order show lower frontend work and instructions in both balanced blocks.
Whole-compiler time is effectively flat with mixed blocks; a reliable overall
compiler saving is not established.

| Work | Control mean | Candidate mean | Change |
| --- | ---: | ---: | ---: |
| Frontend | 2.533016 s | 2.482907 s | -1.98% |
| Internal compiler phases | 17.450489 s | 17.420488 s | -0.17% |
| Complete source-to-Object process | 17.505736 s | 17.491231 s | -0.08% |
| Retired instructions | 257.531827 G | 256.946216 G | -0.23% |
| Maximum compiler RSS | 5,910,372,352 B | 5,913,088,000 B | +0.05% |
| Parsing | 2.085258 s | 2.088934 s | +0.18% |
| Catalog | 0.036894 s | 0.037949 s | +2.86% |
| Source CFG construction | 2.329580 s | 2.373656 s | +1.89% |
| Main cleanup including compaction | 3.411665 s | 3.426391 s | +0.43% |

Frontend improves 2.28% and 1.68% in the two blocks; instructions improve 0.23%
and 0.22%. Compiler phases change +0.09% and -0.43%; complete process changes
+0.18% and -0.34%. The targeted frontend saving is about 50 milliseconds.
Adverse source-CFG, native, parsing, catalog and memory observations remain in
the [complete numerical result](../../benchmarks/mech-deflate/native-profile/schema-cache-view-compile-apple-m3-20261114.json).
Unrelated-stage movement is not credited to the frontend change.

The process driver uses precise `process.wait()` completion. Historical polling
cohorts remain separate. Both preliminary screens have larger startup gaps and
are excluded. Compiler rebuilding, tests, fixtures, native linking and the
standalone worker probe are outside primary timings. The separate compiler
rebuild takes 376.177 seconds; no compiler-bootstrap improvement is claimed.

## Correctness and complete executable

Five focused production cache tests pass, including unique-owner invalidation
after cached success, shared-owner/error repair, repeated mutations and
concurrent readers. All fourteen paired scalar/nested numeric fixtures match.
Every full source, bytecode, Object and complete metadata byte matches in both
screens and all eight primary compiles. The final 1,878,568-byte executable
retains SHA-256
`557d0f5ef2b979e01851ae23b5b3944453a0bd9e772d68161be198916ca06c2c`.
There is no scalar retirement or new runtime campaign.

Two additional full source-to-executable observations take **17.526 seconds**
for control and **17.514 seconds** for candidate. They include source assembly,
frontend, optimization/code generation, program Object/metadata writes, Object
handoff and final native link. The fixed adapter objects are reused; their
first-use construction is excluded. Native linking takes 0.140 and 0.137
seconds. These are separate observations, not a balanced improvement cohort.

## Parallel backend boundary probe

A standalone probe moves owned Cranelift Contexts into workers and publishes
code/converted relocations through `Module::define_function_bytes` in original
order. It includes two functions, local and imported calls, global data, a
function-address relocation in data, and a data definition between function
definitions. Results retain their compiled Context and are ordered by ordinal.

Against the normal serial `define_function` path, one, two and four workers
each pass eight repeats: **24 identical complete Objects, optimized-IR and
compiled-metric comparisons**. The first standalone link fails because system
LTO cannot read the dependencies' newer LLVM bitcode. Building the same source
with Rust ThinLTO succeeds; both setup observations remain.

This is an API-boundary proof over two synthetic functions. It is not an
integrated Mech scheduler, a proof about competing errors or conditional phase
declarations, or a measured Deflate parallel speedup. The production
[parallel architecture](native-parallel-compilation.md) still needs the
one-worker split, dependency handling and bounded scheduling qualification.

The twenty-fourth [complete evidence packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
includes all commands/logs, source/patches, tests, fixture outputs, full program
files/executables, native linker inputs, probe source/Objects and rejected setup.
Compiler/test/probe executables are excluded and SHA-pinned. The repository
archive has been extracted, manifest-verified and replayed byte-identically
against its compact summary. Pure replay checks saved observations without
rerunning compilers or timings. Sub-ten-second compilation remains open.
