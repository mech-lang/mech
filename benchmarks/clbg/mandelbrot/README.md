# Mandelbrot: complete compiled Mech source

The revised sources include argument access, the grid operation, an authored
Mech kernel and PBM output:

```mech
size := io/argument-u64(1)
image := grid/map-until(size, size, pixels)
io/write(image/pbm(image))
```

`pixels` is defined in Mech in the same file. It contains coordinate mapping,
mutable iteration state, recurrence arithmetic, escape tests and stopping.
The compact source computes one pixel per call; the optimized source computes
eight lanes with five recurrence steps between escape checks. Neither calls a
host Mandelbrot function or uses symmetry, precomputed answers or interior tests.

[Compact program](mandelbrot-compact-program.mec): **20 LOC / 289 gzip bytes**.
[Optimized program](mandelbrot-program.mec): **29 LOC / 456 gzip bytes**.
These are complete authored source programs, including the function definitions
and orchestration above. Compiler, general grid scheduler, PBM codec and CLI are
excluded as runtime/library support. Competitors use complete unchanged sources.
Older 17–36 line kernel-only sources, claims and timing reports are preserved
as history and excluded from the revised complete-source corner.

## Implemented pathways

This extends the v0.4 integration base
`c4777b7015fe8ff47fdfa48d18606c49aace7d97` with an engine-owned grid coordinator,
a standard-library grid/IO catalog and a Cranelift application build path.
The kernel is compiled through the existing canonical `ProgramArtifact` compiler,
including Mech statement functions and local mutable state. Cranelift emits both
the iterative numerical kernel and column traversal/bit placement. The general
runtime schedules disjoint rows and encodes/writes PBM output. No numerical
algorithm, escape limit or coordinate formula is stored in that runtime.

`mech build --backend cranelift` uses this path when the CLI is built with
`cranelift_build`. Native build support is implemented in the maintained engine,
standard library, backend and CLI; benchmark drivers use the same emission API.
The coordinator is an additional native pathway rather than reactive resident
IO semantics. These changes describe the extension in this worktree, not the
previously released distribution.

Scalar and f64x2 SIMD lowering, integer Boolean masks and packed native results
are retained. The optimized source doubles by addition, matching Rust #4.
Loop-only FMA is explicit opt-in: it contracts state-dependent recurrence
arithmetic while preserving immutable coordinate rounding. Full FMA previously
changed the official bitmap and remains rejected for this comparison.

## Correctness and measurements

The [task](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/mandelbrot.html)
requires every pixel of an N×N binary PBM, with a maximum of 50 iterations and
escape radius 2. The full workload is N=16000: **256 million pixels** and
32,000,015 output bytes. All candidates must first match the complete official
N=200 image. Every timed full image is hashed and compared with unchanged
Rust #4; differing pixels are retained in raw reports and filtered in the exact
cohort. New complete-source Mech builds are measured separately from older
kernel-only builds.

Twelve compiler/native-map tests cover canonical function compilation,
resident/native agreement, scalar/SIMD modes, bitmap packing, IEEE edges,
termination, state reset and rejection of unsupported coordinator programs.
`verify_program.py` independently checks a **stateful diagonal half-plane**
with no Mandelbrot algorithm: 252 rectangular images across lane counts
1,2,7,8,9,63,64, scalar/SIMD emission, odd-width tails and threaded row scheduling.
It also checks malformed/missing dimensions and worker arguments.

The preserved baseline attempted all 45 upstream sources for C, C++, Chapel,
Julia, Rust, Go and Java, including every listed Rust source. C/C++ uses native
GCC and LLVM builds, with separately recorded fast-math variants. Java uses
OpenJDK JIT with the GraalVM-submission source and is labeled accordingly.
Architecture, build and correctness exclusions retain their diagnostics.

The revised corner keeps the upstream baseline and replaces every Mech endpoint
with complete-source measurements. Each batch is normalized to its own Rust #4
control. It is a combination of recorded studies, not a fresh simultaneous
full-cohort run. The fastest/smallest line describes sampled implementations.

Timing uses fresh processes, sequential seeded shuffled rounds and eight
requested workers. Initialization, JIT where applicable and complete PBM output
to a regular-file sink are included; AOT builds and validation are excluded.
Source-specific worker policies, all samples, source/object/runtime hashes and
exact compiler archives are recorded. No builds or tests run during samples.

- [Revised corner chart](results/apple-m3-revised-corner-16000.svg)
- [Endpoint table](results/apple-m3-revised-corner-16000.table.md)
- [New raw measurements](results/apple-m3-revised-16000.json)
- [Generic grid conformance](results/revised-grid-conformance.json)

## Revised results on Apple M3

Five fresh-process samples at N=16000 measured **0.687 s** for the 29-line
SIMD/loop-FMA program, versus **0.664 s** for unchanged Rust #4: Mech takes
**3.5% longer**. The same Mech source without loop FMA measured **0.820 s**.
The 20-line compact program measured **3.354 s**, or **4.88×** the optimized
program’s time. All complete images are byte-identical to Rust #4.

## Reproduce

```sh
cargo build --release --features cranelift_build --bin mech
mech build mandelbrot-program.mec --backend cranelift --fp-contract-loop \
  -o mandelbrot --keep-project
./mandelbrot 16000 8 > image.pbm

# From repository root:
python3 benchmarks/clbg/mandelbrot/verify_program.py
RUSTFLAGS='-D warnings -C target-cpu=native' \
python3 benchmarks/clbg/mandelbrot/run.py --size 16000 --threads 8 --samples 5 \
  --only mandelbrot-rust-4,mech-program-compact,mech-program-simd,mech-program-simd-loop-fma \
  --output benchmarks/clbg/mandelbrot/results/new-revision.json
```

For source size, comments are removed, whitespace collapsed and UTF-8 compressed
with gzip level 1 and timestamp zero; nonblank noncomment physical LOC is also
reported. These local gzip sizes do not claim to reproduce the website metric.
