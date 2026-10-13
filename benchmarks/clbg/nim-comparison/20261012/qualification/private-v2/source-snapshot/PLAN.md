This additive Nim cohort is authored and unqualified. No source, compiler or
result belongs to the frozen R/APL/Uiua cohorts. No full sample, score or website
admission is implied by this plan.

Use the official Nim 2.2.12 macOS arm64 distribution, released 2026-09-08:
https://github.com/nim-lang/nightlies/releases/download/2026-09-08-version-2-2-8e8fbf60693418dc95bb0d762fd660231d08a583/nim-2.2.12-macosx_arm64.tar.xz
The publisher's asset SHA256 is
130a43eff76c1c3cc78b89329a7efd892c1f65839dbc6d8e53651f3f77516fe5.
Compiler/stdlib source is Nim commit
8e8fbf60693418dc95bb0d762fd660231d08a583 (signed v2.2.12 tag).
Install only in this cohort's private .build directory. Pin the archive, compiler,
complete stdlib/config, generated C, object/executable, exact Clang and SDK inputs
before qualification. Proposed common flags are c -d:release --cc:clang
--mm:orc --threads:on with a separate nimcache for each source. Do not enable
fast-math or silently inherit user/global Nim configuration or Nimble packages.

The compact pi source requires the pure Nim generic arithmetic package
nim-lang/bigints v1.1.0 at 3b5a106b88ec1baa904a059a1430c8335d885075.
Vendor that exact package privately and provide its src via an explicit path.
The optimized pi source uses the public FedeOmoto/nim-gmp 0.2.4 declaration set
at commit 4e9a6a05bfc274abd02a241f2d7c9162b2300191, vendored privately without
editing its source. support/gmp_ops.nim is a generic name adapter over its header
mode, with the old csize spelling aliased to csize_t. This generic library support
is excluded from application LOC consistently with the Lua GMP binding. Its
complete package, adapter and exact GMP headers/library must be pinned. Both forms
evaluate two distinct true integer divisions on every candidate and preserve
the canonical spigot recurrence. No remainder substitute, cached digit or
benchmark executor is used. Std/re requires PCRE1, not PCRE2; pin its exact
header/library and use usePcreHeader plus explicit compile/link paths rather
than the unversioned dynamic search path.

Twenty sources implement all ten canonical tasks. Every helper definition and
independent statement occupies its own line; source LOC must be audited after
compiler formatting/repair. Benchmark workload helpers are counted; ordinary
generic arithmetic/ABI library support is audited separately.
Sources with threads create at most eight workers, write disjoint results and
join before parent output. Qualification must prove worker-created maps survive
worker exit and parent reads/destruction, including empty and tail maps.
ORC supports shared pointer arguments whose owner
outlives joined threads. The sequential nbody/fasta/reverse-complement/pi forms
remain ordinary language loops and standard library calls.

Published source inventory and adaptations are recorded in PROVENANCE.md.
Preparation must hold /tmp/mech-clbg-timing.lock for every build, runtime probe,
validation and pin sweep; all compiler and sampler/native descendants inherit
the held descriptor and use authoritative PGID cleanup. Queue only one bounded
preparation job after root review. Do not interrupt the running R/APL study.

Required qualification includes complete small/adversarial outputs, all seven
growable nucleotide maps and overlapping-window traces, both actual pi division
operands/quotients, every fannkuch permutation and actual prefix flips with global
rank parity, leaf/node allocation/check work for trees, float-CDF/LCG draws for
fasta, full dense spectral products and canonical nbody step/energy behavior.
Mandelbrot must produce complete exact PBMs at odd/tail widths and N200, using
requested dimensions and zero padding. Include actual worker counts and joins.
Root owns the later guarded full study: three actual canonical full samples per
successful compact/optimized source and fresh unchanged Mech controls, with
complete output, resource/status/environment/pin failures fatal and no scaled
scores. No full run is authorized by this preparation proposal.
