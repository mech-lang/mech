# N-body competitor and compiler audit, 2026-10-11

Mech's strong result is reproducible, but it is not the fastest native n-body
implementation on this machine. In seven matched, shuffled full-workload rounds,
C++ takes 0.890626 s, Chapel 0.952136 s, Mech's existing fast form 1.046597 s,
and Rust 1.069720 s. These are local Apple M3 results, not published x86 timings.

The unchanged compact source improves from 1.119698 s to 1.089513 s, a 2.7%
median-time reduction. The source remains 20 normalized LOC; the fast form
remains 25 LOC. The fast scheduling/seed combination does not improve its
seven-run median: 1.047539 s versus 1.046597 s. The chart therefore keeps the
unchanged fast program and labels its new samples as a fresh control.

## Work and timing boundaries

Both Mech sources perform all ten unordered pairs, both velocity contributions
in three coordinates, and all five position updates on every one of 50 million
steps. Initial/final energy is evaluated from the actual state. The final source
expression is the output; there is no benchmark-specific host gravity kernel.
The generic native runner supplies the turn count, state storage and process I/O.
Native arithmetic is emitted by Cranelift; Clang links the generic CLI wrapper.

The admitted sources use the same simple symplectic integrator. Static array
dimensions, constant masses, unrolling and FMA are compiler optimizations, as
they are for C++ and Chapel. Mech exposes the complete fixed-size graph to the
compiler. Newly added sources often retain runtime pair loops and indexed
loads/stores. The fastest local C# source uses Vector256, including a zero
fourth lane; ARM64 executes those vectors as two 128-bit halves. That is a real
code-generation difference, rather than evidence of less work in Mech. The
locally measured C# AOT, Ada and Fortran n-body entries take approximately
1.709 s, 1.546 s and 1.759 s in their archived cohort.

Each recorded sample measures a fresh process through exit, including startup,
initialization, full computation and output. AOT compilation is excluded for
every AOT entry. One full untimed validation/warmup precedes each candidate.
Samples run sequentially, with deterministic shuffled rounds, AC power and
Low Power Mode disabled. No compiler or other benchmark ran alongside them.
The seven-run confirmation discarded no samples. Correct energies are required
at 1,000 and 50 million steps for every admitted candidate and every timed run.

All 30 position/velocity components are checked at 0, 1, 1,000 and 10,000 turns.
Selected scheduling transformations preserve the unchanged Mech state bits.
The independent Rust state reference differs by at most 2.871e-12 after scaling,
below the 1e-10 audit limit. Correct final output alone is not the work audit.

## General improvements and measured losses

The retained opt-in scheduler groups independent arithmetic while estimating
last uses, critical paths and register pressure. It keeps SIMD partners together
and emits bounded compute regions so later instruction selection can retain
the ordering. There are no source-name, input-size, body-count or benchmark
recognition branches. Its native register budget depends on the host ISA.
`--schedule-regions 8` is selected for the compact source; ordinary compilation
and the existing fast source's flags remain unchanged. Compact stack-load
instructions per unrolled step fall from 81 to 74, with 288 to 284 instructions.
This narrows the gap without changing formulas or their accumulation order.

`--simplify-square-seeds` removes an FMA's positive-zero accumulator only when
both multiplicands are the same square operand. This preserves signed zero and
all later summand ordering. Its compact-only candidate takes 1.093153 s, also a
confirmed improvement. It is available generally but is not selected over the
faster measured compact scheduling candidate. The initial prototype attached
this rewrite to FMA contraction; the final compiler exposes the explicit flag.
Final recompilation verifies the same native object for that opt-in candidate.

General column gathers and repeated column add/subtract updates now lower to
the same numeric graph machinery as row operations. Their selectors accumulate
in source order, including duplicates. This enables column-oriented pure Mech
n-body variants without introducing an application kernel. Those variants are
slower (approximately 1.100–1.171 s), so the current benchmark sources are kept.

Scalar sqrt/divide inside vector arithmetic lost roughly 16–17%; selective SIMD
packing lost roughly 8–12% for the fast form. Moving negative constant signs
through products did not help. Within-block demand/pressure scheduling and
uniform constant broadcasts generated identical native objects. All pilots and
their losing binaries remain archived. The small fast-form gain in a three-run
pilot did not survive the independent seven-run confirmation; no fast speedup
is claimed. C++ and Chapel still generate fewer spill/load instructions.

Paired-round geometric ratios are 0.9762 for compact scheduling and 0.9767 for
square-seed simplification. Their within-cohort bootstrap intervals are
[0.9700, 0.9822] and [0.9730, 0.9790], respectively. The fast trial is 1.0024,
with [0.9978, 1.0065], consistent with no gain. These are diagnostics for this
cohort and do not remove host/frequency effects or predict other architectures.

## Evidence and reproduction

`summary.json` pins the selected measurements, all pilot reports, final compiler
sources, tests and independent state audit. `confirmation-50m.json` preserves all
56 raw samples, stdout, exact commands, binaries, objects, source hashes, state
checks, environment observations and sample order. `confirmation-mech.csv`
gives raw seconds, time × the matched Mech fast control and LOC × its 25 lines.

`compiler-before`, `compiler-math`, `compiler-pressure`, `compiler-consumer`,
`compiler-regions`, `compiler-regions-combined`, `compiler-columns` and
`compiler-final` preserve source snapshots. Driver copies pin the corresponding
generation executables. Repeated driver copies may share identical file storage;
their bytes and recorded hashes remain unchanged. The final driver reproduces
byte-identical native objects for both selected chart endpoints and the seed
candidate, as recorded in `final-object-verification.json`.

The native fused test suite passes 31 tests, including perturbed initial states,
unrelated nonlinear recurrences, repeated column updates and signed zero.
The native map suite passes 14 tests, including complete Mandelbrot bitmaps with
the new policies enabled. The existing benchmark sources and other runtime
paths retain their options. Scheduling and seed simplification are opt-in.

From the worktree root:

```sh
cargo test --release --locked --manifest-path benchmarks/clbg/nbody/mech-driver/Cargo.toml --test fused
cargo test --release --locked --manifest-path benchmarks/clbg/mandelbrot/mech-driver/Cargo.toml --test native_map
python3 benchmarks/clbg/nbody/experiments/20261011-competitor-techniques/probe.py --programs benchmarks/clbg/nbody/experiments/20261011-competitor-techniques/confirmation-programs.json --steps 50000000 --samples 7 --output /tmp/nbody-new-confirmation.json
```

`prepare.py` can generate unchanged source candidates from a selected driver and
source-independent options. Use a new tag and output manifest to preserve old
timed artifacts. Builds and tests must finish before running timings.
