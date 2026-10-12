# Mech Benchmark Explorer

Static interactive charts from the verified October 8–11, 2026 AC-power cohort.
The page includes benchmark and language selection, program visibility, source
inspection, raw timing samples, geometric means, and covariance SD ellipses.
Small language labels follow each visible endpoint line. Overlapping point clicks
choose the nearest center within their marker layer; coordinates are unchanged.
On narrow charts, labels that overlap another label are revealed on hover or
keyboard focus, keeping the full language set readable without moving its points.
Hovering a point or connecting line emphasizes its language and dims the other
languages' endpoints, samples, lines, labels, and covariance contours. Leaving
the target restores the original opacity. The hovered language's samples become
fully opaque; the **Samples** checkbox toggles sample visibility. Hover also reveals
that language's covariance contours without changing the oval button selections.
Keyboard focus provides the same emphasis; touch taps retain source inspection
without a persistent hover state.
Wide invisible line targets sit below every point so existing clicks still work.
Language filters intersect independent categories: Execution, Typing,
Language style, and Memory management. **Array + Dynamic** includes only languages with both traits;
choosing AOT and JIT within Execution allows either mode. Empty categories impose
no restriction. Filtering preserves manual language exclusions, benchmark
selection, and the fixed chart limits. Clear filters restores the unconstrained
categories; All enables all languages and clears filters. Execution describes
the measured backend: Java uses GraalVM native AOT, Ruby uses YJIT, and PHP uses
the verified tracing JIT. All three Mech modes are Array and Functional.
Memory strategies can overlap: Python, PHP, and CBQN use reference counting plus
cycle collection; Mech uses ownership and shared reference-counted values.
Foreign arithmetic/regex libraries do not determine a runtime's category.
MATLAB and GNU APL remain Unclassified for memory management pending runtime
evidence. Reference policies: [CPython GC](https://docs.python.org/3/library/gc.html),
[PHP cycles](https://www.php.net/manual/en/features.gc.collecting-cycles.php),
[CBQN](https://github.com/dzaima/CBQN/blob/master/README.md), and
[Futhark array lifetime](https://futhark.readthedocs.io/en/latest/c-api.html).

The language selection also provides All, None and Mech actions. None clears all
language selections, including entries hidden by the release-year filter, while
preserving the selected benchmarks, year, chart bounds and ellipse preferences.

Oval buttons keep covariance regions visible after hover. Each fastest and fewest-LOC
profile has a 1σ contour and a lighter 2σ outer contour.
Sample covariance is calculated in log₂ LOC/time space across the selected
benchmarks, centered at their geometric means. The 2σ contour doubles both
principal-axis radii and preserves their correlation. This shows benchmark
variation, not a confidence interval or run-to-run timing uncertainty. Fixed
chart bounds include the complete 2σ envelopes even when these regions are hidden
with **Focus on means** off. With it on, axes fit the full cohort's geometric-mean
endpoints; outlying samples and contours are cropped. Turning Samples off restores this means range and removes the full-data fit option. Re-enabling Samples keeps the means range until the user chooses otherwise. The control works in both
the geometric-means and Mech views and does not change any computed statistics.

All comparison, timing, and implementation tables have sortable column headers.
Click a header again to reverse its order. Comparison LOC and time ratios sort
independently; timing samples retain their original run numbers. Sorting uses
the raw numeric measurements and persists as selections change.

The **Release year** slider accumulates languages and steps only through years
represented in the comparison. It intersects manual language selections and trait filters
without forgetting them. Chart bounds and the full-cohort 1× baselines stay fixed
when the year changes. Mech uses **2026** for the benchmark development version
under test, rather than its first public introduction.
`language-years.json` records the first release or public introduction, the basis
for each date, and a source link. Early previews count; runtime versions inherit
their language's year. The page exposes these sources beside the controls.

The comparison table contains Language, LOC ×, Fastest ×, Slope, Length, and
sortable **2σ area** columns for the fastest and fewest-LOC profiles.
LOC × is the fewest-LOC endpoint's ratio; Fastest × is the fastest endpoint's
time ratio. For their connector, slope is Δlog₂(time) / Δlog₂(LOC) and length
is hypot(Δlog₂(LOC), Δlog₂(time)). These are data-space quantities, independent
of screen size. Coincident endpoints have length 0 and undefined slope (—).
Each 2σ area is 4π√det(Σ), with sample covariance in log₂ LOC/time space,
measured in squared doublings. Areas use the full ellipse before display clipping,
and update with benchmark/program selections independently of contour visibility.
Fewer than two benchmarks gives an undefined area (—); rank-one covariance gives
zero area. There is no separate 2σ checkbox.

Serve `dist/` with any static HTTP server. There are no browser dependencies or
external data requests. The measured source files are embedded in `dist/data.json`.
Source hashes, report provenance, sample medians, and independently verified
reference means accompany each update. `data-audit.json` records the packaging audit.
The dataset includes only each language and benchmark's fastest and fewest-LOC
programs, deduplicated when both profiles select the same program. Source and
timing records for other implementations are excluded from the website.

Haskell and OCaml each cover all ten benchmarks with locally measured fastest
and fewest-LOC endpoints. The Functional group includes Haskell, OCaml and
Futhark; Haskell and OCaml also belong to AOT. Checkboxes use alphabetical
display order. Their GitHub language colors are retained, with the existing
contrast adjustment against the dark background. The release-year control adds
Haskell (1990) and OCaml (1996), with primary history sources in the year panel.

The functional-language cohort considers all 60 sources on the pinned published
measurement pages. Original sources are preserved locally; modern API repairs
and required-work corrections are separately labelled and count toward LOC.
Small and official full-size outputs are checked before three randomized
fresh-process samples per candidate. General GMP, PCRE2, hash-map and array
libraries contain ordinary operations; algorithms remain in Haskell or OCaml.
Tree compiler evidence verifies complete construction, dynamic leaf allocation
and long-lived-root retention. Nucleotide maps grow from small initial tables
and include every overlapping window. Sampling uses the existing binary64
division/unscaled-CDF cohort policy. GHC 9.10.3 uses LLVM and -O2; OCaml 5.4.1
uses Flambda and -O3. Unchanged Mech AOT executables provide controls, while
earlier chart measurements remain unchanged. The reports, full sources, build
logs, work review and raw samples are under `functional-comparison/20261010`.

Erlang covers all ten workloads using sources from the published Benchmarks Game
measurement page. All 21 originals are preserved locally; eleven separately
labelled corrections repair tree leaf allocation, floating-CDF FASTA, nucleotide
tie ordering, the omitted last Mandelbrot pixel/byte output, or whole-input regex
work. Every correction and application helper counts toward statement-normalized
LOC. Twenty-one admitted candidates have three shuffled full-input samples each,
with ten unchanged Mech AOT controls. Fresh-process timing includes BEAM startup,
module loading/load-time JIT, full input, computation, output and shutdown;
`erlc` compilation is excluded. Erlang/OTP 29.1.1 uses eight online schedulers and
default GC/heap settings. Optimized BEAM evidence confirms dynamic tree allocation
and long-lived-root retention. Erlang belongs to Dynamic, Functional and JIT;
its GitHub color is #B83998, and its public-introduction year is 1990 under the
existing release-year policy. Reports, originals, corrections, compiler evidence,
LOC audit and raw samples are under `erlang-comparison/20261010`.
Original contributor attribution is retained in every source; its inspector
links to the pinned Benchmarks Game BSD license distributed with the Site.

Rebuild the data using Python with the original plotting dependencies available:

```sh
python3 build_data.py --clbg-root /path/to/mech/benchmarks/clbg \
  --array-audit-dir /path/to/benchmark-png/iversonian-sources
```

Run the numerical checks with `node verify.mjs`. `verify_ui.cjs` additionally
exercises the UI at desktop and mobile sizes against a server on port 8766.
It uses the existing workspace Playwright and Chrome installation.
`verify_sorting.cjs` checks numeric and text ordering, ascending and descending
toggles, keyboard focus, original timing run numbers, selection persistence,
and source preservation at desktop and mobile sizes.
`verify_years.cjs` checks release-year accumulation, recent-language filtering,
manual selection persistence, fixed plot geometry, and the compact metrics table.
`verify_axes.cjs` checks fixed tick positions and endpoint coordinates across
language, group, profile, program, sample, and ellipse visibility changes.
Browser checks accept `PLAYWRIGHT_MODULE`, `BROWSER_EXECUTABLE`, and
`BENCHMARK_EXPLORER_URL` overrides.

The current dataset includes the validated function-application revisions of
all ten Mech benchmarks and the implicit-output 20/25-line n-body sources.
BQN (CBQN) and K (Growler) cover all ten full workloads: five samples for
n-body and three samples for each of the remaining nine. Means state their measured
coverage; Futhark covers nine tests, and MATLAB covers all ten. The
shared-benchmarks control compares the same subset for
every selected language. Earlier spectral-norm and
nucleotide shortcuts remain archived and excluded; their faithful replacements
perform the required four-function multiplication and every hash-table update.
The Futhark allocation gap is labelled in its coverage control.

MATLAB R2026b has twenty measured local compact/optimized source candidates for
all ten required algorithms, each with three full, reference-validated samples.
Ten unchanged fastest Mech AOT controls also have three samples, for ninety
validated runs and no discarded samples. Immutable exports pin the published
snapshot. Base MATLAB standard JIT is
used without Coder or Parallel Computing Toolbox. Generic Java BigInteger
provides pi arithmetic, and an inspectable stream-only launcher preserves input
before MATLAB batch mode disconnects stdin. All algorithm work and application
helpers remain in counted `.m` source. Independent untimed cell-identity probes
verify independent tree allocations through even and odd constructor branches;
no MEX benchmark executor is used. MATLAB belongs to the
Dynamic, Array and JIT groups and uses GitHub's `#e16737` color and a 1984 release
year. Earlier sources and timing samples are preserved.
Benchmark algorithms remain in the language source. General GMP, PCRE2, khash,
and process-map bindings are pinned, inspectable in the source selector, and
labelled separately from application LOC. The process map is reused by three
benchmarks in both languages and runs at most eight interpreter workers.
The program inspector includes the audit that expands packed assignments into
separate LOC. Source and timing details remain available from chart points and
the main comparison table.

Swift covers all ten benchmarks, using the installed Apple Swift 5.10 compiler
on Apple M3 with `-Ounchecked -whole-module-optimization`. Published Swift 6.0.3
Linux timings are not reused. All 37 sources linked from the pinned
[Swift measurements](https://benchmarksgame-team.pages.debian.net/benchmarksgame/measurements/swift.html)
were considered; ten separately labelled corrections retain the originals and
their source hashes. Every admitted candidate has three shuffled fresh-process
full-workload samples and complete small/full output checks. LOC counts all Swift
algorithm code and corrections, excludes comments and blank lines, and expands
packed independent statements. Generic GMP header bindings support pi digits;
the full spigot algorithm remains in Swift. Swift uses its original Dispatch
scheduling on eight logical CPUs; scheduled task counts can exceed core counts.

The FASTA #3 integer-scaled probability cutoffs conflict with the
[current FASTA instructions](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/fasta.html).
FASTA #8 uses probability subtraction instead of the prescribed cumulative lookup.
The other FASTA corrections align binary64 sampling and explicit division with
this cohort; these are comparability decisions rather than blanket claims about
all published submissions. Tree corrections enforce stretch release before
long-lived allocation and keep the long-lived root through temporary work,
as required by the
[current tree instructions](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/binarytrees.html).
Optimized Swift IR confirms per-node allocation, including separate leaf boxes,
and retains every temporary construction/check. The audit, build logs, exclusion
reasons and raw samples are pinned in `swift-comparison/20261010`.
Unchanged current Mech AOT executables run alongside Swift as timing controls;
the website retains the existing Mech cohort samples. Swift's controls and their
ratios to those earlier samples are retained in the dataset audit.

Mojo 1.1.0 (MAX 26.6.0) covers all ten full CPU workloads. Futhark 0.27.1
covers nine; binary trees is excluded for missing required per-node allocations.
Both use at most eight workers and three fresh process samples per candidate.
Mojo N-body and the scalar Spectral Norm variant adapt pinned community sources;
the other Mojo programs and all Futhark programs are local ports. Futhark's
official N-body and Mandelbrot examples use different workloads. Its compiled
multicore backend is used here, so GPU results are excluded from this CPU cohort.
Futhark sources are available in the program inspector. General integer, hash-map,
regex and IO support is separately inspectable and excluded from application LOC.
All application algorithms execute in their named language; the Futhark C
adapter only passes arguments, stdin and stdout through the generated API.
Distinct compact and performance candidates are implemented for every test in
both languages: twenty pairs were attempted, nineteen pairs and fifty sources
are admitted, including earlier valid baselines. Each endpoint selects the
measured winner across this pool;
an optimization attempt that loses does not replace a faster baseline.
Source/build/library pins, independent library tests, required-work reviews,
and full output checks are in `additional-languages/20261009-full-suite`. The
earlier `20261009` cohort remains archived. The current Mech
fastest controls were remeasured with their original worker arguments and runtime
settings; these samples are kept separate from earlier measurement reports.

The general-library checks cover signed arithmetic through 16,384 bits,
map growth and reserved batches, regex replacements and arbitrary line widths.
The generic APR pool test checks 34,816 distinct allocations across seventeen
clear/reuse rounds. Generated binary-tree construction and traversal are reviewed
separately. Test-only allocator instrumentation verifies exact per-node allocation
counts and sixteen bytes for every leaf and interior node in both corrected Mojo
sources, at depths six and ten. Their full depth-21 outputs and three timing rounds
pass against the reference. The instrumentation is absent from timed runs.
An early Mojo candidate collapsed the requested tree loop to a multiplication;
that source, binary and evidence are archived and excluded. Subsequent flat/indexed
array trees in both languages were also rejected: they construct and walk the node
values but omit the required allocator calls. The corrected Mojo algorithm remains
in Mojo and calls a thin general Apache APR pool binding for each node, matching
the C reference's library allocation strategy. Futhark's flattened arrays do not
provide a faithful per-node heap-reference tree; no native tree executor is used.
See the [official tree allocation rules](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/binarytrees.html)
and [Futhark's value representation](https://futhark-lang.org/blog/2021-08-02-value-representation.html).
The new `futhark-allocation/20261010` capability audit tests both stock C and
multicore backends. A non-inlined allocation function retains its allocation
call, but copies each node into a flat array and releases the allocation inside
the construction loop. Recursive node types and a compiled allocator intrinsic
are unavailable. The latest interpreter-IO proposal does not add a compiled
C allocator interface. A faithful tenth workload needs a general heap-reference
or allocator compiler extension; no host tree executor is substituted.
The nine other benchmark cohorts retain their original 171 verified samples with
unchanged source, binary and used-library pins. Corrected Mojo trees and their Mech
control add nine fresh samples, for 180 admitted samples. Reuse and rejection
provenance is recorded in the final timing report and immutable snapshots.

Known algorithm and allocation mismatches are excluded before choosing each
language's endpoints. The current Mech spectral-norm source restores the four
required functions at 12 LOC. Both Mech FASTA profiles disable integer cutoff
conversion and retain floating probability comparisons. Their five-run full
measurements and all ten Mech work reviews are pinned by `work-audit/20261009`.
Historical timing reports remain archived. Binary trees records its use of the
general core library memory pool.

The Rust FASTA fastest endpoint is a labelled local adaptation of published #7.
It retains the threaded generation/output pipeline but uses unscaled f64 CDFs
and direct floating division. Its median is 0.563287167 seconds over five full
N=25,000,000 runs, all matching the reference output byte for byte. The unchanged
Mech optimized executable was remeasured in the same shuffled batch, at a
0.541193-second median. Source, binary, output hashes and all samples are pinned
by `fasta/investigations/20261009-floating-cdf/results.json`.

Chart selections preserve the current per-benchmark normalization baselines.
Axis limits use the full admitted cohort for the requested benchmark set, with
both profiles included whether displayed or hidden. Focus on means fits the
geometric-mean endpoints; unchecking it includes all benchmark endpoints and
complete 2σ envelopes.
Language, program, profile, sample, and ellipse visibility does not rescale the
chart. Available shared-coverage cohorts are included in the bounds so automatic
fallback does not change the axes either. Explicit changes to the benchmark
set, chart view, or Focus on means setting establish a new range. The language-selection Mech
button filters the current chart; the separate Mech view tab changes the view.
Disabled programs change the candidate pool within their language; the subset
means only include languages covering every selected benchmark. Ellipses use
sample covariance (`ddof=1`) of log ratios across selected benchmarks and are
unavailable for a single benchmark. Run-to-run SD is shown separately with the
recorded timing samples. Standalone Lua remains in the archived measurements but
is omitted from this explorer, as requested.

The dark theme uses the blog's canonical `mech-palette.css`. Mojo and Futhark use
GitHub Linguist colors `#ff4c1f` and `#5f021f`. Other language colors keep
the PNG palette, including Perl's requested teal override. Low-contrast colors
are lightened against the dark surface, preserving their hue.

Coverage details live in a collapsed note beside the language controls. The
shared-benchmark action keeps a permanent toolbar slot, and switching language
groups preserves the chart's document position as well as its axis limits.
Program details dock beside the plot on wide screens and below it on smaller
screens, before the comparison table. They never overlap the plot.

Mech has AOT, JIT, and interpreted entries for all ten workloads. JIT
uses fresh-process wall time including source loading and compilation. Interpreted
execution measures loaded execution including input and full output, and
retains whole-process timings in its sortable sample table. All three paths use
the same source candidates; each selects its measured fastest and smallest
endpoints independently. Gold solid, dashed, and dotted lines distinguish them.
The initial execution-mode audit records forty semantic checks, fifteen unchanged
AOT native objects, and old/rebuilt AOT timings collected in the same rounds.
The `20261010-final` cohort reruns all fifteen source candidates
in JIT and interpreted execution, with AOT controls in the same rounds. The
k-nucleotide control is freshly linked with the updated counting runtime;
other controls retain their original executables. It passes fifty-four semantic
checks and independently reproduces all fifteen native
objects. Structural admission shares one parsed canonical document, and JIT
emission reuses the admitted control graph and releases the parser document.
The standard counting container keeps its batch dispatch inlined with its caller,
retaining every update. Interpreted matrix products interpret
the source coefficient instructions in independent tiles; integer range proofs
retain checked-error semantics, tail lanes remain bounded, and sums keep their
original order. Every coefficient is recomputed for each product. The counted
program sources and timing boundaries are unchanged; prior timings stay archived.

The current interpreted measurements use the complete canonical after cohort from
the `20261011` interpreter integration audit: three validated full-workload runs
for each of all fifteen candidates, selected before computing the fastest and
smallest endpoints. The complete matched before/after audit retains ninety runs.
Source identities, LOC, algorithms and files are unchanged, and every point
archives its full prior measurements and source report. AOT and JIT measurements
are unchanged. LLVM codegen partitioning and the generated control interpreter
changed despite unchanged workload source and direct dispatch; the new f32
executor being uncalled does not establish timing neutrality. Default fannkuch
is 5.93% slower in the main cohort, with an independent three-round confirmation
measuring 5.67% slower. Confirmation timings remain separate from chart samples,
and static disassembly differences do not prove the cause of the slowdown.

## Global statistics and execution families

**Global median & SD** highlights the coordinate median of each visible profile
and fades per-language means and samples. Covariance remains the sample covariance
in log₂ space; its 1σ and 2σ contours are translated to the median without changing
shape or area. No whole-set average is plotted. Each execution variant has equal
weight. The overlay and the cumulative release-year median trail work in every
chart view. **Play years** runs through dataset release years; manual slider input
pauses playback. View changes and current filters are retained, and axes and
normalization baselines stay fixed as the graph builds up. Empty selections show
no median; one entry has a median and no covariance.

Alphabetically ordered language families have independently selectable children.
Lua includes **Interpreted** and **LuaJIT**, and Mech includes **AOT**, **JIT**, and
**Interpreted**. Parent checkboxes toggle eligible children and indicate partial
selection; disclosure buttons collapse children without changing measurements
or selection. No variant timings or means are merged. Group and release-year
filters retain their execution-path semantics. Lua's eleven recorded endpoints
restore nine full workloads from the existing reports; pi digits has no admitted
Lua record. All 407 prior program records, 396 source records and ten baselines
are unchanged. Lua source files were already included for LuaJIT.

`verify_global_variants.cjs` exercises global centers and contours, profile and
subset changes, both grouped families, partial selection, collapsing, release
years, one/zero-entry statistics, all individual benchmark charts, and the
conditional data-fit control at desktop and phone widths.


Common Lisp (SBCL) and Racket each cover all ten workloads. All 59 published
Benchmarks Game sources were considered; 19 separately labelled corrections
repair the shared required work, platform compatibility or output ordering.
Every eligible candidate has three fresh-process full-size timings, native
Apple Silicon precompilation, source/compiled-artifact pins and full output
validation. Fastest and fewest-LOC endpoints are selected from those local
measurements. Startup, core/module loading, full computation and output, and
shutdown are included. General GMP and CL-PPCRE libraries are allowed;
application FFI bindings, arithmetic macros and helpers count toward LOC.
Six tree candidates have allocation evidence. Common Lisp and Racket are in
Dynamic, Functional and AOT groups for these execution paths. Their GitHub
Linguist colors are #3fb68b and #3c5caa, with release years 1984 and 1995.
`lisp_racket_data.py` verifies the full cohort, and `verify_lisp_racket.cjs`
checks their filters, code and sample tables at desktop and mobile sizes.

Zig, Odin, Roc and Elm each cover the ten full workloads. Twenty pinned
Programming-Language-Benchmarks sources provide Zig/Odin candidates; forty-one
additional candidates are authored locally. The 61 admitted applications and ten
unchanged Mech controls have 213 validated full-size samples. All 75 earlier
valid samples in this cohort retain identical source/executable hashes. All
earlier languages' source records and raw samples remain unchanged. Fastest and
fewest-LOC endpoints are measured winners from this candidate pool.

The source inspector shows seconds, time relative to the fresh unchanged Mech
AOT control, and LOC relative to both Mech source profiles. Downloadable CSV and
Markdown tables include every fastest/smallest pair and those same references.
The chart keeps its all-language normalization and preserved Mech measurements;
the inspector labels the fresh-control comparison separately.

Zig 0.17.0 uses LLVM ReleaseFast and the native CPU target. Odin dev-2026-10 uses
speed optimization and native targeting. Roc uses the established alpha4 LLVM
compiler from September 9, 2025, with basic-cli 0.20.0; these measurements do not
characterize the newer compiler rewrite. Elm 0.19.3 compiles with `--optimize`
and runs under Node 24.19.0. Its separately inspectable generic adapter handles
argv/stdin/stdout ports only. Application helpers and FFI declarations count
toward normalized LOC; general libraries and process I/O support do not.

Pure Roc arithmetic and regex helpers pass 228 independent checks, including
the same fourteen regex fixtures on both search implementations. The optimized
Roc regex engine derives a first-byte table from the general parsed pattern;
Odin also measures the ordinary PCRE2 JIT interface.
Elm additionally measures base-sixteen packed permutations and counters,
bounded below 2^52, with a counted list/array fallback for larger inputs. Nine
separate smaller-input comparisons check the complete output against its original
implementation before full timing. Odin additionally measures seven ordinary
native workers and map_entry pointers for complete growable-map nucleotide
counting. Elm also measures its ordinary core Dict using complete thirty-six-bit
numeric keys against its generic hash-map implementation. Tree evidence
includes actual compiled Elm execution ordering, optimized Roc allocation and
lifetime IR, untimed Zig/Odin allocation counters, and the actual timed native
machine code. General native arena allocators still allocate a distinct 16-byte
node for every leaf/interior and
retain a separate long-lived tree through all temporary work. Elm's original
early root check is excluded; its corrected root stays in the temporary loop's
arguments until the final check. A same-calculation pi multiplication operand
order correction avoids quadratic use of an asymmetric general integer library.
Both superseded unmeasured Elm sources are retained in the audit.

The new entries use GitHub Linguist colors. Zig and Odin belong to AOT; Roc to
Functional/AOT; Elm to Functional/JIT. Release years are 2016, 2016, 2020 and
2012, respectively, with sources and explanations in the existing year controls.
`zig_odin_roc_elm_data.py` verifies source, binary, runtime, output and LOC pins
before import. `verify_zig_odin_roc_elm.cjs` exercises all endpoint sources and
timing tables, language/group/year selection and fixed chart bounds at desktop
and phone widths. Full sources, exclusions, build logs, audits and raw timing
reports remain under `zig-odin-roc-elm-comparison/20261010`.

C#, F#, Ada, Free Pascal, Fortran, Dart and Smalltalk complete the remaining
Benchmark Game cohort. C# and F# expose grouped JIT/NativeAOT variants, and Dart
exposes grouped AOT/JIT variants. All ten prescribed workloads use three
sequential fresh-process samples per admitted source. Compilation and preparation
are excluded; startup, JIT, Pharo image loading, full input, output and shutdown
are included. The source panel and downloadable
`remaining-languages-mech.csv`/`.md` tables give seconds, time × a fresh
unchanged Mech AOT control, and LOC × both Mech source profiles. Earlier chart
sources, LOC and absolute samples remain unchanged.

The runtimes are .NET 10.0.12, GNAT FSF 16.1.0-1, Free Pascal 3.2.2,
GNU Fortran 16.2, Dart 3.13.5 and native ARM64 Pharo 14. General GMP and PCRE2
operations are permitted; application helpers, orchestration and bindings remain
in the displayed language and count toward statement-normalized LOC. All
compiled Ada/Pascal application units are included in their aggregate sources.
Fortran uses the locally installed GNU compiler rather than published Intel
timings. F# joins Functional; Smalltalk joins Dynamic/JIT; native variants join
AOT. New colors come from GitHub Linguist, with the existing dark contrast
adjustment.

Ada's initial regex implementation overflowed the stack at the full input size.
The failed artifact and result remain archived. Its separately measured revision
uses standard GNAT heap strings and borrowed views, including its allocation
callback in LOC, with unchanged counts and ordered substitutions. Six additional
untimed conformance fixtures include empty input, overlapping matches, ordered
substitutions, read boundaries and a buffer larger than the default stack.
The revision has three new full samples and a separately timed unchanged Mech
control. `remaining_languages_data.py` verifies the combined frozen audits and
reports before import; `verify_remaining.cjs` checks endpoint code, seconds,
ratios, family/group/year controls and fixed axes on desktop and phone.
Evidence remains under `remaining-languages-comparison/20261011`.

The 2026-10-11 n-body compiler audit remeasures Mech, C++, Chapel and Rust in seven matched full-size rounds. The unchanged 20-LOC Mech source selects general pressure-aware scheduling and gains 2.7%; the 25-LOC fast form keeps its existing code and options. Its fresh control timing is not called a speedup. All other sources and samples are preserved. `nbody_compiler_data.py` verifies compiler, source, binary, object, state and report pins before import. The methodology links provide the matched × Mech table and the complete work/compiler audit.

The R/APL/Uiua importer accepts only independently audited canonical full-workload candidates with at least three complete validated samples and a qualifying fresh Mech AOT control. Partial suites retain explicit coverage; pending and failed tests contribute no estimates. Source/alias execution metadata, immutable attempts, summary hashes and control ratios are checked before admission.

`pidigits_work_data.py` retires the one-extraction Node.js #2 source and imports its separately validated and measured two-extraction correction. `interpreter_integration_data.py` verifies the complete matched 90-run before/after audit and replaces all fifteen interpreted candidate measurements with its complete canonical after rows. It preserves the full prior measurements and source reports, pins independent confirmation and exact-binary disassembly evidence separately, and leaves AOT and JIT unchanged. Its main-cohort helper snapshots are explicitly verification-time provenance; inherited ambient environment was not fully captured.
