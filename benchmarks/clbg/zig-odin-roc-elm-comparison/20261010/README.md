# Zig, Odin, Roc and Elm benchmark cohort

The candidate applications implement the same ten full workloads as the existing
Mech Benchmark Explorer. The reports select the fastest measured and smallest
normalized-LOC sources from the admitted candidate pool. A source may occupy both
endpoints. These are local measurements of pinned implementations and toolchains.

The completed comparison will be exported to `results/summary.md`,
`results/summary.csv` and `results/summary.json`. While measurements run,
`results/progress.md` contains only groups whose candidates and Mech control have
all three validated full-size samples. Incomplete groups are omitted.

Both timing profiles use the fresh unchanged fastest Mech AOT executable as their
reference. The fastest LOC profile compares with Mech's fastest source; the
smallest LOC profile compares with Mech's smallest source. The ratios are ordinary
division of the raw values. Below 1× means faster or fewer lines than the named
Mech reference. All tables show seconds and raw LOC alongside the ratios.

## Workloads and required work

| Benchmark | Full input | Required computation |
|---|---:|---|
| N-body | 50,000,000 steps | All ten symmetric interactions of the five bodies per step; momentum offset; initial/final energies. |
| Mandelbrot | 16,000 × 16,000 | Every pixel, up to 50 escape iterations, complete binary PBM output. |
| K-nucleotide | FASTA N=25,000,000 | Complete THREE record; every overlapping window for all seven growable maps of widths 1, 2, 3, 4, 6, 12 and 18. |
| Spectral norm | N=5,500 | Forty complete implicit N² matrix/vector products and final dot products; no precomputed matrix or omitted remainder. |
| Reverse complement | FASTA N=25,000,000 | Every record/header and all 250,000,000 nucleotides; complete wrapped output. |
| Fannkuch-redux | N=12 | All 12! permutations, canonical signed checksum and maximum flip count. |
| FASTA | N=25,000,000 | 2N/3N/5N records; fresh LCG state and direct binary64 division for every random symbol; unscaled cumulative probabilities. |
| Binary trees | N=21 | Separate, equal-size leaf/interior node allocations with two child slots; complete stretch, temporary and long-lived tree work; long-lived root retained through temporaries. |
| Regex-redux | FASTA N=5,000,000 | Complete input cleaning, nine case-insensitive counts, five ordered substitutions, all specified output lengths. |
| Pi digits | 10,000 digits | Unbounded LFT spigot; both extract(3) and extract(4) for each tested candidate; signed floor division. |

## Source provenance and execution

The pinned community repository is
[Programming-Language-Benchmarks](https://github.com/hanabi1224/Programming-Language-Benchmarks/tree/ad18b203dd1769724f4eea94fc3ac1e99f6593e0).
Twenty-two original Zig/Odin sources and their MIT attribution are preserved in
`upstream/hanabi`. Twenty are ported or reused. The original fixed-bucket Odin
nucleotide table is replaced with a growable-map implementation; the x86-only
Zig fannkuch assembly source is preserved and excluded on this ARM64 machine.
`exclusions.json` records these decisions. The originals' Linux/x86 timings are
never substituted for local measurements.

Zig uses 0.17.0, LLVM, ReleaseFast, and the native CPU target. Odin uses
dev-2026-10 with speed optimization, the native CPU target and bounds checks off.
All compatibility/work corrections count toward application LOC. They include
complete PBM output, binary64 FASTA sampling, complete stdin handling, arbitrary
spectral dimensions, and modern I/O interfaces. The Zig vector n-body versions
initialize records from a scalar seed table at runtime to avoid a compiler
global-layout defect; this initialization is timed and counted. Full-size
fannkuch validation caught an upstream exclusive-bound error at N=12; the bad
output/source/executable are preserved in `evidence` and excluded.

Roc applications are authored locally against the established **alpha4 LLVM
compiler from September 9, 2025**, commit d73ea109cc2, with `--optimize` and
basic-cli 0.20.0. These numbers characterize that compiler rather than the newer
compiler rewrite. All benchmark sequencing, collections, numerical kernels,
formatting, and general big-integer/ASCII-regex helpers are Roc and count toward
LOC. The platform provides general process I/O and allocation only. Its general
helpers pass 200 independent integer arithmetic/floor-division cases and 14
regex count/replacement cases. The additional optimized search repeats those
same fourteen regex fixtures, for 228 total checks over the two implementations.
Its possible-first-byte table is derived once from the general regex AST and
case mode; it contains no benchmark-specific patterns or input values.
Odin also measures an ordinary PCRE2 JIT interface, with all FFI declarations,
matching orchestration and substitutions in the counted application source.
The regex helper implements the general ASCII
constructs needed here; it is not a claim of full PCRE compatibility.

Elm applications are authored locally, compiled with 0.19.3 `--optimize`, and
executed under Node 24.19.0. Every benchmark algorithm remains Elm. Ordinary
packages provide general unbounded integer arithmetic, regex, arrays and
growable hash maps. The pi application multiplies the multi-limb operand by the
one-limb scalar in the order suited to the library’s schoolbook implementation;
this preserves every LFT update and both signed floor extractions. An unfinished
original operand-order candidate is preserved and superseded without an admitted
full timing. `support/elm-runner.cjs` is a generic argv/stdin/port/stdout
adapter: it contains no benchmark computation or arithmetic. Its source and the
package manifest are inspectable separately from application LOC. Full-size
timing includes its input conversion and complete output writes.

## Timing and LOC audits

Compilation, verification and hashing are excluded from timing. Each sample
uses a fresh process and includes startup, module/library loading, complete
input, computation, complete stdout to a regular file and shutdown. There are
three sequential shuffled full-workload samples per admitted executable, on AC
power with Low Power Mode off. Competing compilers or power changes invalidate a
sample. The Mech controls keep their original source, executable and arguments.
Earlier explorer measurements remain unchanged.

Complete small and full outputs are checked against the cohort's canonical
references. Source and executable hashes are checked before each measurement;
libraries, package sources, runtime binaries, adapters and audit scripts are
pinned. Timing records contain all raw samples and their output validations.

LOC counts all nonblank/noncomment application source and expands packed
independent assignments/statements. Literal record fields, loop headers and
comments are treated structurally. Application helpers, constructors, formatting
and FFI declarations count; ordinary library implementations and generic process
I/O adapters are excluded. There is no compressed-size metric.

Allocation evidence checks distinct equal-size nodes at leaves and interiors and
the long-lived root's lifetime. Elm's original generated JS checked its root
before temporary work, despite the source's apparent order; that version is
quarantined. The corrected application carries the root through a strict tail
loop and checks it only after all temporary work. This is audited in generated
code and with an untimed instrumented copy. Roc's optimized LLVM IR retains
separate 32-byte allocations in both constructor branches and checks the retained
root after the temporary-work loop. Native optimized tree candidates use ordinary
standard arena allocators and at most eight native workers. Each node still
receives its own allocation, with two recursively constructed child pointers;
arena storage is reset only after the complete tree is checked. Disassembly of
the actual timed native executables additionally verifies retained 16-byte
allocation requests, both recursive child constructors/stores, pointer traversal,
and reset only after construction/check inside each temporary-work loop.

The first corrected frozen audit is `final-source-audit.json`, with 56 candidate
sources and ten controls requiring 198 full samples. The 75 validated native
samples are retained only with matching source/executable hashes. Measurements
resume in `results/final-timings.json`. After this phase completes, two new generic
regex alternatives are built, independently checked and admitted in
`source-audit-complete.json`, alongside a pure-Elm packed permutation/counter
alternative for fannkuch and a standard-thread/map-entry Odin nucleotide
alternative, plus an ordinary core Dict alternative for Elm nucleotide counting.
The completed cohort then contains 61 candidates
and ten controls, requiring 213 full samples in `results/timings-complete.json`;
all 198 unchanged validated samples are preserved and only the fifteen new
samples are added. The initial reports and superseded unmeasured Elm sources
remain unchanged for provenance.

All original sources, corrections, tool versions, complete-output failures,
allocation evidence, build logs, normalized LOC, pins and raw timings are retained
in this directory. The website includes only the final fastest/fewest-LOC
endpoints, their sources and timing samples.
