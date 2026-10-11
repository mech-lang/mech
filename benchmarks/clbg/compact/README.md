# Compact-language Benchmarks Game cohort

This extends the existing v0.4 integration studies with the smallest listed
Python 3, Ruby, Perl, Lua, JavaScript (Node.js) and PHP programs. Julia is audited
against the current tables. LuaJIT runs the unchanged Lua sources as a separate
runtime.

The current chart workflow is the [full-cohort AC normal-power rerun](../retime/README.md).
It measures every existing build, including Julia, and all compact additions
again. The interrupted compact-only report and its archives preserve earlier
attempts; their elapsed times are not used in the fresh chart.

The ChapelCon 2024 deck's full summary already includes Python, Ruby and Perl;
the zoomed fast corner omits those offscreen points. This cohort restores the
compact-language comparison for Mech. It does not cover every language in CLBG,
and the eight completed Mech studies are not the complete ten-test CLBG suite.

## Selection and source size

`manifest.json` pins complete source files, author comments, execution notes,
source URLs and SHA-256 hashes. It preserves the eight listing tables used for
selection. All listed variants in the selected languages are inspected.

Select the minimum published CLBG `gz`, the minimum locally computed normalized
gzip size, and the minimum nonblank/noncomment physical LOC. Retain all ties.
These can select different programs. Do not reformat or rewrite upstream code.
The local metric uses language-aware comment removal, whitespace collapse,
UTF-8 and gzip level 1 with `mtime=0`; it is not byte-identical to CLBG's `gz`.
Required PHP delimiters/preprocessor tokens are retained. Lexer tokens that are
not classified are preserved and recorded rather than silently discarded.
Identifiers are not renamed: compressed size includes the submitted naming
style as well as syntax. LOC is also sensitive to how expressions are divided
across lines. Neither metric measures semantic complexity or simplicity.

The added languages are **compact-only** samples. A single measured compact
point does not establish that the language's fastest and smallest programs are
identical. Chart squares represent compact endpoints; no fastest dot or
fastest-to-smallest connector is inferred for the added runtimes.

## Runtime modes

Runtime versions, paths, executable hashes and exact invocations are recorded in
the result JSON. CPython uses its default interpreter mode; Ruby uses YJIT as in
the listed invocation; JavaScript uses Node/V8; PHP explicitly enables tracing
JIT with a 64 MiB code buffer; Lua uses its bytecode interpreter.

LuaJIT is built from the recorded upstream Git commit with:

```
env MACOSX_DEPLOYMENT_TARGET=14.0 make -j4 XCFLAGS=-DLUAJIT_ENABLE_LUA52COMPAT
```

That documented runtime option supplies compatible library names such as
`table.unpack`. It does not modify the benchmark sources or supply benchmark
kernels. Every listed Lua candidate is validated under LuaJIT before selecting
the smallest compatible entries. Any incompatible listings are explicit
exclusions. Both Lua and LuaJIT use identical source sizes for identical source
files, and fresh-process LuaJIT compilation is included in elapsed time.

## Workloads and timing

| Benchmark | Full workload |
|---|---:|
| nbody | 50,000,000 steps |
| Mandelbrot | 16,000 × 16,000 pixels |
| k-nucleotide | FASTA N = 25,000,000; 125,000,000 THREE bases |
| spectral norm | N = 5,500 |
| reverse complement | FASTA N = 25,000,000; 254,166,745 input bytes |
| fannkuch-redux | N = 12; all 479,001,600 permutations |
| FASTA | N = 25,000,000; 250,000,000 output bases |
| binary trees | N = 21 |

Validate the official small inputs first, then validate each complete full
output, then measure three sequential fresh processes in randomized order.
Output goes to a regular file. Startup, JIT and output are included. Builds,
tests, hashing and output comparison are outside the timed region. No parallel
benchmark processes, compilation, plotting or browser checks run during timing.
Timeouts kill the complete process group and are recorded; no missing time is
extrapolated or plotted as an observed time. The saved report can resume without
discarding finished samples. A load guard waits for competing compiler jobs to
finish and checks executable names every ten seconds during measurements.
Samples overlapping detected compiler load are archived, discarded and retried.
Ordinary desktop activity remains a limitation of these local measurements.
The interrupted k-nucleotide batches are preserved in the compiler-interference
and power-transition archives; their timings are excluded from reported medians.
The harness records power source and active Low Power Mode at sample boundaries
and every ten seconds. A detected change discards and restarts the entire timed
batch, including its control. Previously completed batches retain their original
measurements; their power states were not tracked. No system power setting is
changed by the harness.

The earlier compact-only workflow reran an unchanged control from the preserved
suite and used `new_seconds / new_control_median * preserved_control_score`.
The full-cohort rerun supersedes that normalization with fresh raw medians for
every chart point. Original measurements remain unchanged. A geometric mean
requires measured coverage of all eight studies; incomplete coverage is not
filled in or averaged over a different workload set.

Validation is byte-exact for sequence/image/tree outputs and printed spectral
norm. nbody uses the existing 1e-8 energy tolerance. Fannkuch checks both checksum
and maximum; one listed PHP program omits the final newline, which is recorded
as the sole permitted formatting difference.

## Algorithm differences

Keep unmodified listed compact entries visible, with algorithm labels. In
particular, some small FASTA sources use repeated probability subtraction rather
than a cumulative probability search. Python's smallest binary-tree source
reuses a constant leaf tuple; Perl's uses scalar leaves; PHP's constant array
leaves may use copy-on-write sharing. Their output can match
while allocating fewer objects than Mech's ordinary-node leaves. These are not
unqualified claims about equal allocation work.

The full chart includes the labeled listed FASTA cohort; a separate strict
cumulative-probability view is retained in the per-benchmark gallery. All Mech
source counts remain complete Mech programs, excluding generic compiler/runtime
support. This expansion changes neither the Mech compiler nor its benchmarks.

## Reproduce source selection and the earlier compact-only workflow

The source-selection script requires the pinned Pygments 2.19.2 lexer. Set
`PYTHONPATH` to its installed directory. Source pins refuse refresh by default.

```
python3 benchmarks/clbg/compact/run.py --prepare
python3 benchmarks/clbg/compact/run.py
python3 benchmarks/clbg/compact/finish.py
python3 benchmarks/clbg/overview/build_gallery.py
python3 benchmarks/clbg/overview/plot_mega.py
```

Only run gallery export and browser verification after benchmark timing stops.
The runtime paths at the top of `run.py` can be adjusted on another machine.
Results are local Apple M3 measurements, not the site's Linux timing values.
Use `retime/README.md` to reproduce the current full-cohort chart.

References:

- https://chapel-lang.org/chapelcon/2024/chamberlain-clbg.pdf
- https://benchmarksgame-team.pages.debian.net/benchmarksgame/
- https://luajit.org/extensions.html
- https://www.php.net/manual/en/opcache.configuration.php
