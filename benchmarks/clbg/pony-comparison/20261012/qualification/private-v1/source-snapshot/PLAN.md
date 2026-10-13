This isolated Pony cohort is authored, unbuilt and unqualified. It adds twenty
compact/optimized packages without changing any frozen source, audit, runner or
result. Root owns compiler bootstrap, bounded preparation and later full studies.
No score, full workload or website admission follows from this document.

Use Pony 0.74.0, source commit 95ad18015d04f4414d0fd4f469ed6b2e9b9b9e73,
released 2026-09-30, with its official macOS arm64 toolchain and complete standard
library. Pin the publisher's archive, compiler/runtime/static libraries, package
sources, linker/SDK, generated objects and final executable under the common
mutex. Use release compilation without fast-math. Execute with --ponymaxthreads 8;
verify actual scheduler/native-thread counts before full admission. At most
eight compute actors run concurrently, and the parent orders all output.

The source layout is sources/<benchmark>/{compact,optimized}/main.pony. All
application workload helpers and independent statements occupy separate lines
and count in LOC. Ordinary published regex/GMP library support is pinned and
audited separately, excluded consistently with equivalent Lua/Nim generic ABI
support. No C workload executor, spigot, map, array or benchmark-specific kernel
is allowed. PCRE2 matching/replacement and GMP arithmetic are general primitives.

Actor heap churn yields at bounded behaviors. Tree workers allocate/check one
complete tree per behavior, including separate leaf Node objects; the Main
actor retains the long-lived root through all iterations. Fann workers process
bounded global-rank blocks and perform each actual prefix reversal. Bulk I/O
notifiers preserve chunk order, treat read failures as fatal, and finalize only
on EOF. Full PBM/binary stdout uses explicit lengths; the parent emits ordered
results and flushes, then allows natural quiescent runtime shutdown.

All preparation builds, runtime probes, tests and hash/pin sweeps must hold
/tmp/mech-clbg-timing.lock. Compiler and sampler/native ownership must persist
through normal exit, interruption, late PGID registration and cleanup. Never
interrupt the running R/APL continuation or modify its leased files. Root must
review a single bounded preparation proposal before queueing it. Require full
small/adversarial outputs and actual work traces for nodes/leaves, every Fann
rank/flip/global parity, all seven complete growable nucleotide maps, floating
CDF/LCG FASTA draws, both pi numerator divisions and strict numerical kernels.
Qualify EOF, CRLF/tails, exact PBM width/padding, joins/ownership and scheduler
limits, including worker data surviving completion. Full sampling later means
three actual canonical processes per selected profile plus fresh unchanged Mech
controls; no timings are estimated, scaled or fabricated here.
