# Private Crystal preparation proposal

This is an unsealed, unexecuted proposal for Crystal 1.21.1. The twenty source
drafts, package, compiler, standard-library behavior, outputs, work traces and
LOC remain unqualified. It authorizes no full performance processes, data
admission, explorer change or publication. Root reviews the sealed proposal
before `guarded_prepare.py --run`; each eventual full study needs a separate
manifest review.

The proposed command sequence is the locked standalone `test_guarded_prepare.py`
and `test_source_trace.py`, then `guarded_prepare.py --seal --proposal-file
proposals/private-v1.json`, then only after root's exact digest review
`guarded_prepare.py --run --proposal-file proposals/private-v1.json
--proposal-sha256 <reviewed digest> --revision private-v1`. A later changed source
requires a new explicit proposal and revision; earlier failures remain archived.

Use the official universal macOS release, source commit
`73ea5c2b1bc3b3ac0ba400aa9da77af96f63d6ef`, publisher archive SHA256
`ba34fa7fb8bc8b951f12d17e12f69c9d7ccce38900caa4b8a17f1b4f29d37605`.
Download into a fresh private revision and verify the publisher digest before
safe extraction. Never install globally or reuse an incomplete revision. The
release must contain exactly one compiler and standard-library prelude and a
bundled Boehm GC library; missing dependencies stop with preserved evidence.
No implicit Homebrew installation is permitted by this proposal.

Seal the source/authoring/proof/reference scripts, all downloaded primary
provenance, the frozen common native guard, current harness, existing GMP 6.3.0
and PCRE2 10.49 libraries/headers/pkg-config files, explicit Clang22 linker,
pkg-config executable and installed CommandLineTools MacOSX14.4 SDK sentinel
files. The SDK sentinel pins are not a claim that the whole SDK or operating
system was hashed. Compiler/package/dependency identities and actual ARM64
linked images are recorded during bootstrap under the same mutex.

The inherited `/tmp/mech-clbg-timing.lock` protects every native command,
including compiler/metadata programs. Every command starts in its own session,
passes the held lock descriptor, and establishes its cleanup scope before
`Popen`. An exception during identity/active-journal setup still reaps the
owned group and drains/closes both output pipes. Raising signal handlers are
installed before any seal/bootstrap command. Never signal another owner's process. Compiler, probe, cleanup or
late pin errors preserve the raw result and stop this revision without retry.
Fresh command journals, source snapshots, build caches, binaries, probe inputs,
expected/full captured outputs and trace files are immutable evidence.

Both variants use `crystal build --release --no-debug --mcpu native --threads 1`.
Crystal's release compiler performs LLVM code generation; `CC` selects the
explicit Clang22 linker. SDKROOT and linker `-isysroot`/deployment target 14.4
are explicit. Do not imply that a Clang linker flag changes Crystal's LLVM
floating arithmetic. No fast-math/GC-disable flag is introduced. Ordinary
standard BigInt/GMP, Regex/PCRE2 and Boehm GC stay enabled. Runtime environment
uses real pinned dependency directories in DYLD_LIBRARY_PATH and records both
dynamic lookup aliases and real image target pins to detect retargeting. It
does not opt into parallel execution: the optimized applications explicitly
resize their one default execution context to at most eight schedulers.

Before any compiler invocation, pass the new preparation ownership/restoration
fixtures and work-trace reconciliation fixtures under the common mutex. Reuse
the unchanged independently passed fourteen V reference fixtures by pinning
their receipt/log/source, and the new eight-case exact DNA oracle proof only
after its actual receipt passes. Do not rerun those unchanged fixtures. Their oracles
are shared independent output references, not measured programs.

The bounded matrix includes nbody 0/1/2/10000, exact PBM 0/1/2/3/7/8/9/15/16/17/
31/65/129/200, spectral 1/2/3/5/9/100, actual allocated trees 0/4/7/9, all Fann
permutations through n8 with awkward eight-worker ranges, pi zero/every ten-digit
tail through 97, FASTA 0/1/2/29/30/31/60/61/287/574/20000 and full DNA parser,
IUPAC, comment, CRLF, EOF, exact THREE-token and ordered-regex counterexamples.
Every attempt uses the frozen shared native sampler/resource/power/normal-exit/
complete-output gates and is explicitly unscored. Temporary WORK/INPUT/validator
overrides are protected by one unconditional `try/finally`, including errors
while preparing expected/input data; absent entries are restored as absent.

An isolated general stdlib proof exercises positive/negative floor and truncated
division, large/long BigInt constructors and immutable operand lifetime across
GC, PCRE2 nonoverlap/substitution and binary NUL stdout, plus completed capped
fiber/private-map handoff through standard channels. No workload host helper
is added. Each actual application receives separate isolated trace copies:
all seven real nucleotide maps after all joins; the two actual pi divisions
with full numerators/divisor/quotients and actual acceptance; and every actual
tree Node constructor/edge/root with separately allocated leaves, temporary
tree counts, stretch root and unchanged long-lived root identity.

The tree audit keeps its own bounded Node references to prevent address reuse
while checking distinct identities. This intentionally changes only unscored
audit-copy memory retention and must never become a performance source.
Independent Python reconciliation checks the complete graph forest and every
root's shape/count, not only a returned checksum. Original stdout must still
match the independent full bounded oracle for all trace copies. Trace helpers
only observe original arithmetic/maps/allocations; no benchmark calculation is
moved into host code. Spectral fresh-product, Fann rank coverage, Mandel escape,
and FASTA draw semantics additionally need the explicit source/work review
before full manifests are sealed; bounded output is not sole work evidence.

Successful preparation means only bounded output, native ownership, standard
primitive and required trace gates passed. Canonical normalized LOC review,
remaining required-work review, exact full workload/input/source/library pins,
three actual fresh canonical full samples per candidate and freshly matched
current Mech AOT controls remain mandatory. Every complete full output is
checked at each actual timing attempt; no estimated/scaled score or unscored
probe is admitted. Full reporting must use seconds and multiples of the exact
matched Mech timing/source, with corrected canonical LOC and retained producer
LOC evidence. The global LOC overlay review continues to hold publication.
