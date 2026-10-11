# Benchmarks Game purpose

This suite exposes gaps in Mech's language, standard library, memory model, and
compiler across different domains. Expressiveness and complete source size are
part of the result, alongside execution time. A gap is a finding to implement
and measure, not a reason to conceal the algorithm behind a host function.

Keep benchmark construction, traversal, numerical updates, iteration, and
orchestration in Mech using canonical language forms. Do not replace a program
with `run-nbody()`, a tree kernel, an EKF kernel, or equivalent library calls
whose implementation contains the benchmark algorithm in Rust or another host.

When a form is difficult or unavailable, compare it with the other languages'
complete implementations. Determine whether Mech needs a general library
operation, a language feature, or support for an existing form in a backend.
Prefer existing syntax over inventing an ad hoc native operation; inline match
arms use `=>`, while `->` denotes a transition.

Runtime and compiler work must serve ordinary programs. General arenas, scoped
lifetimes, record layouts, recursion, match lowering, comprehension fusion, and
inlining policies qualify. Recognizing a benchmark, special tree allocation,
or domain-specific optimization modes do not.

Retain complete Mech sources, source-size metrics, pinned foreign submissions,
byte-exact workload validation, tests of the general feature on other programs,
and compiler snapshots. Preserve earlier measurements separately when sources
or compilation change. Document remaining restrictions rather than presenting
an admitted native subset as support for every form of the language.

All ten workloads now have source-visible Mech programs: nbody, mandelbrot,
k-nucleotide, spectral norm, reverse complement, fannkuch-redux, FASTA,
binary trees, regex-redux and pidigits. The last two added general byte regex
operations, ownership-safe byte-buffer reuse and entry-owned arbitrary-precision
arithmetic. Their algorithms remain in the `.mec` files. The existing record,
match, control-loop and independent-count pathways are reused.

The October 9 completion is recorded in
[`remaining-conformance/completion.json`](remaining-conformance/completion.json),
with full-workload seven-sample confirmations, general feature tests and paired
runtime checks of the previous seven sequence/control/grid workloads. The nbody
confirmation remains separately frozen. White PNG charts for all ten are
[`overview/mega-ten.loc.png`](overview/mega-ten.loc.png) and
[`overview/mech-ten.loc.stddev.png`](overview/mech-ten.loc.stddev.png).
The plotted source-size metric is LOC only. Lua has no ten-workload mean because
its pidigits output failed on the configured Lua 5.5 runtime; LuaJIT covers all ten.
