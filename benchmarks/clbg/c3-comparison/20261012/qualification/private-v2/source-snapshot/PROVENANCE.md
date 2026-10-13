# C3 source provenance

These application programs are freshly authored translations of the faithful algorithms and constants already present in the repository's Nim compact/optimized programs at benchmarks/clbg/nim-comparison/20261012/sources. Nbody planetary constants, FASTA ALU/probability/RNG constants, spectral matrix formula, Mandelbrot recurrence/PBM layout, fannkuch permutation enumeration/checksum, trees and long-lived lifetime, Pi recurrence and DNA/regex inputs/output conventions retain that benchmark provenance. No upstream shootout source was copied verbatim as a C3 implementation. The original Nim source attribution/provenance remains available alongside that cohort.

The counted C3 buffer, thread-launch, GMP and PCRE2 support files are newly authored generic glue. Official pinned C3 0.8.5 APIs were read at:
- https://raw.githubusercontent.com/c3lang/c3c/v0.8.5/lib/std/libc/libc.c3
- https://raw.githubusercontent.com/c3lang/c3c/v0.8.5/lib/std/math/math.c3
- https://raw.githubusercontent.com/c3lang/c3c/v0.8.5/lib/std/math/bigint.c3
- https://raw.githubusercontent.com/c3lang/c3c/v0.8.5/lib/std/collections/hashmap.c3
- https://raw.githubusercontent.com/c3lang/c3c/v0.8.5/lib/std/threads/thread.c3
- https://raw.githubusercontent.com/c3lang/c3c/v0.8.5/lib/std/threads/os/thread_posix.c3
- https://raw.githubusercontent.com/c3lang/c3c/v0.8.5/src/build/build_options.c

Function pointer aliases and &function addresses, Type::size properties, type-first arrays, extern @cname ABI signatures and explicit generic heap map initialization follow this tagged stdlib/source grammar. Bare min/max were corrected to explicit std::math imports/qualified calls during static review before sealing. Compilers still must validate every actual source spelling.

Release evidence: https://github.com/c3lang/c3c/releases/tag/v0.8.5 and https://github.com/c3lang/c3c/releases/expanded_assets/v0.8.5. The cached /releases/latest discrepancy is preserved in PLAN.md. Planned asset hash does not represent a local archive receipt.

Preparation process ownership/probe restoration and full-study controller are adapted from independently reviewed Nim work-full-v1 and frozen common twenty-gate sampler services. Independent Python output oracles are an exact source copy of Nim references.py and are pinned as benchmark reference service, not counted algorithm source or native workload kernels. No Mech or C3 process invokes a host application kernel.
