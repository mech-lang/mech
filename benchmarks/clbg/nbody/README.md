# Mech v0.4 n-body comparison

## Scheduling follow-up (2026-10-11)

Two additional 15-round cohorts found a small 0.2% improvement from changing the fast source's scheduling regions from 8 to 24 instructions: 1.041377 s to 1.039063 s in the replication. The native loop drops from 276 to 268 instructions, 51 to 48 stack loads and 23 to 20 stores. The compact form retains size 8. Five general compiler prototypes were tested; the best code is reproducible with the existing compiler, so the prototype edits were archived and restored. The experimental cohort is preserved separately from the published suite chart. See the [scheduling audit and complete results](experiments/20261011-scheduling/README.md).

## Latest compiler audit (2026-10-11)

Two seven-round cohorts confirm a 1.4% improvement for the unchanged 25-LOC fast Mech source with square-reduction simplification and eight-instruction graph scheduling: 1.054 s to 1.039 s. In the final matched cohort Zig takes 1.007 s, Chapel 0.970 s and C++ 0.899 s; the 20-LOC compact Mech source retains its prior scheduling choice and takes 1.090 s. All 46 native tests and independent full-state checks pass. See the [Zig/compiler gap audit](experiments/20261011-shared-products/README.md), [raw replication](experiments/20261011-shared-products/replication-50m.json), and [earlier compact compiler audit](experiments/20261011-competitor-techniques/README.md). The new opt-in `--share-scaled-products` pass reduces arithmetic but does not improve this benchmark, so it is not selected.


This is a blog case study of execution time and program compactness, following
[Brad Chamberlain's ChapelCon 2024 presentation](https://chapel-lang.org/chapelcon/2024/chamberlain-clbg.pdf).
It connects each language's fastest and smallest observed implementations.
It is also suitable as work in progress for NJPLS. A paper about Mech's reactive
semantics should additionally evaluate reactive workloads and the claims made
in that paper. One n-body result is not a language-wide performance result.

Execution time and source size are separate axes. Their ratio does not measure
productivity or readability. For a suite summary, select endpoints per benchmark
and take geometric means over the same supported benchmark set for all languages.

## AC-power follow-up, October 9, 2026

The [seven-sample confirmation](experiments/20261009-local-simd/final-confirmation-50m.json)
times 50,000,000 steps on the Apple M3, with AC power and Low Power Mode disabled.
Fresh processes run sequentially in a seeded shuffled order. Builds, full-state
validation, and one full-workload warmup per executable precede timing. Every
sample validates both energy outputs; compiler activity and power changes are
monitored. No samples were discarded. This is a new matched n-body measurement,
separate from the immutable October 8 full-suite results.

| Program | Median seconds | Sample SD, seconds | LOC |
| --- | ---: | ---: | ---: |
| C++ #6, LLVM native O3 | 0.903768 | 0.010862 | 202 |
| Chapel #3 | 0.960899 | 0.009566 | 79 |
| Mech mass-first pairwise, SIMD AOT + state-tail packing | 1.053052 | 0.004041 | 37 |
| Mech original pairwise, compiler reassociation + state-tail packing | 1.064306 | 0.037648 | 38 |
| Rust #3, native O3 | 1.087667 | 0.012835 | 196 |
| Mech compact, SIMD AOT | 1.121837 | 0.006827 | 32 |
| C #8, LLVM fast-math/unrolling/LTO | 1.539446 | 0.005996 | 127 |

The revised fast form takes 6.8% less time than the previous 38-line fast form
(1.129855 seconds, retimed in the same batch), and has one fewer line. The
unchanged 32-line program takes 4.0% less time than its previous compiled binary
(1.168187 seconds). The revised fast form takes 3.2% less time than Rust #3,
9.6% more time than Chapel #3, and 16.5% more time than C++ #6. These are local
ARM-compatible programs; the pinned x86 AVX submissions cannot run natively here.
C is slower than Mech in this cohort; C++ is the leading C-family entry.

Three changes explain the measured improvement:

- **Canonical vector broadcasts.** A single-vector shuffle now selects both
  lanes from its first input. This lets Cranelift recognize a broadcast and emit
  indexed FMA instructions directly, as LLVM does. The fix preserves arithmetic
  order and rounding. The compact loop falls from 314 to 288 instructions.
- **Pack odd-width state tails.** `--pack-state-tails` pairs leftover scalar
  lanes across rows of each state, without changing its external layout or its
  arithmetic graph. It helps the new 37-line source. After the broadcast fix it
  slows the compact source, so it remains optional and is omitted there.
- **Overlap constant scaling with expensive dependencies.** The general
  `--reassociate-constants` pass recognizes fused constant-scaled products and
  uses estimated dependency costs to multiply the earlier-ready operand first.
  For the existing pairwise source, this moves mass scaling before the wait for
  square root and division. It has no source-name or benchmark-shape tests.
  This opt-in fast-math transformation changes rounding and assumes finite
  arithmetic. The unchanged 38-line program improves with it and tail packing.

The [37-line source](nbody-pairwise-mass.mec) expresses the useful multiplication
order directly. It replaces these two lines in the 38-line source:

```mech
 dv := d * mag
 v[body,:] += dv[interaction,:] * weighted-m
```

with one explicit array expression:

```mech
 v[body,:] += d[interaction,:] * weighted-m * mag[interaction]
```

All ten interactions, integration updates, initialization and energy calculations
remain authored in Mech. The source uses general array arithmetic, selection,
combinations and reductions. It contains no host physics kernel. The best
37-line configuration uses `--fp-contract --algebraic-powers --pack-state-tails`;
the 32-line configuration uses `--fp-contract --algebraic-powers`.

The revised fast loop still has 282 instructions, 59 stack loads and 21 stack
stores, versus Chapel's audited 204/25/9 and C++ #6's 172/11/2. SIMD grouping,
shuffle selection and register allocation remain targets. Static counts do not
isolate the cause or the fraction of the remaining runtime gap. The fastest
local C++ loop uses ordinary sqrt/divide, without reciprocal estimates. Both
Mech hot loops likewise contain no runtime or library calls.

A [same-graph LLVM diagnostic](experiments/20261009-local-simd/matched-mass-and-backend-50m.json)
compiled generated arithmetic from Mech, solely to distinguish graph effects
from backend effects. Its original pairwise form was almost the same speed as
Cranelift, and its compact form was slower. These diagnostic binaries are not
Mech performance endpoints. Greedy local-lane policies also lost; they were
removed from the compiler and retained in experiment snapshots.

Validation passes **21 n-body/compiler tests and 13 map/Mandelbrot tests**,
including complete official Mandelbrot bitmaps. [Post-validation](experiments/20261009-local-simd/post-validation.json)
compares all 30 state components at 0, 1, 1000 and 10,000 steps. The broadcast fix
preserves every state bit; the largest scaled difference from the instrumented
Rust #3 reference among the selected Mech configurations is `2.871e-12`.
All three current [Mandelbrot program objects](experiments/20261009-local-simd/mandelbrot-regression.json)
are byte-identical to their previous objects. Other benchmark backends are not
modified by these f64 compiler changes. Reassociation and tail packing stay
disabled by default.

The report records source, object, executable, driver and compiler hashes,
raw samples, exact commands and a full compiler/runtime change snapshot. To
repeat the matched comparison after rebuilding the standalone driver:

```sh
python3 benchmarks/clbg/nbody/experiments/20261009-local-simd/probe.py \
  --steps 50000000 --samples 7 --include-before-broadcast \
  --output benchmarks/clbg/nbody/experiments/20261009-local-simd/new-confirmation.json
```

## Program and execution path

The base is v0.4 integration commit
`c4777b7015fe8ff47fdfa48d18606c49aace7d97`. Compiler, runtime, source and script
snapshots accompany new measurements because these changes are uncommitted.

All initialization, the momentum offset, force calculation, velocity and position
updates, and both energy formulas are authored in Mech. The current compact
programs end with `energy`, which is returned through Mech's existing final
statement semantics. No explicit export is required. The generic
`mech-native-f64` runner uses the program result by default and
prints it before and after the requested native turn loop. It contains no n-body
constants, body counts, force equations, energy equations, or state-layout offsets.
The observer evaluates the output without advancing or committing state. Named
selection with `--output energy` remains available for the older source snapshots.

The shorter sources are `nbody-matrix-short.mec` (20 LOC) and
`nbody-matrix-pairwise.mec` (25 LOC). They inline one-use constants and expressions
and initialize mass/coordinate matrices directly. Each assignment and sequential
statement occupies a separate line. The existing transpose operator now lowers
through the generic native f64 backend as an operand permutation. Both programs
produce byte-identical native objects and generated runner C to their respective
32/37-line baselines, and every state component is bit-identical through 50 million
steps. Reproduction and hashes are in
`experiments/20261009-source-compact/prepare.py` and `source-compaction.json`.

The primary source-size metric counts **only the `.mec` program**. Compiler,
standard library, runtime, generic CLI, generated code, and compiler command-line
flags are excluded. Runtime support needed to execute this program belongs in
Mech, under `hosts/gpu/src/fused_f64/`; it is not handwritten benchmark physics.
Other languages use their complete pinned submissions. These source boundaries
are stated explicitly; Mech's host-provided turn count and output formatting are
runtime services. The original three Mech candidates have 32 nonblank,
noncomment lines; the original pairwise alternative has 38 and the mass-first
alternative has 37:

| Mech source | Force expression | LOC |
| --- | --- | ---: |
| `nbody.mec` | `dt / (distance² * distance² ^ 0.5)` | 32 |
| `nbody-power.mec` | `dt * distance² ^ -1.5` | 32 |
| `nbody-factored.mec` | square root; mass/magnitude product before coordinate broadcast | 32 |
| `nbody-pairwise.mec` | shared displacement/magnitude product; interleaved pair contributions | 38 |
| `nbody-pairwise-mass.mec` | displacement/mass products before magnitude; interleaved pair contributions | 37 |

The scalar Cranelift AOT emitter is backported from the IROS workshop branch,
commit `b10593b75ec8b9283ae33e4ac330c00dda9957d7`. Its f32 AOT/JIT paths retain
v0.4 remainder and recurrence behavior. The autonomous f64 extension lowers the
canonical artifact, folds static arithmetic/ranges/combinations, resolves static
gathers, expands reductions and repeated indexed updates in source order, and
puts all turns inside one native loop. It accepts pure autonomous fixed-shape f64
programs. Inputs, effects, constraints, control nodes, and unsupported operations
are rejected. Resident activation supplies authoritative initial values.

Cranelift AOT emits the simulation and energy observer as native object code.
Clang compiles the generic CLI and links that object. Cranelift JIT emits the
same arithmetic IR directly into executable memory; fresh-process timing includes
source compilation, activation, lowering, and JIT compilation. The optional LLVM
comparison emits C as an intermediate representation and lets Clang generate
the simulation machine code. It is separately labelled.

These are experimental native capabilities in the host crate. The standard
v0.4 `mech build` command is not yet wired to this execution path. The autonomous
loop does not measure per-turn reactive scheduling, publication, recording, or
host-input transactions.

## Workload and correctness

The [Benchmarks Game specification](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/nbody.html)
requires the same simple symplectic integrator. We use five bodies, ten unique
interacting pairs per step, three coordinates per body, `dt = 0.01`, the momentum
offset, and initial/final energy output. The canonical input is 50,000,000 steps.
Compiler folding, fusion, vectorization, and removal of unused intermediate
values are allowed optimizations of this algorithm.

Every program must pass the published 1,000-step check within absolute error
`1e-8`. Every timed invocation is also checked against a validated Rust #3
reference at the requested input; nonfinite values, incorrect energies, missing
lines, or extra stdout are rejected. The canonical reference is
`-0.169075164` then `-0.169059907`.

The compiled ablation additionally compares **every position and velocity** at
0, 1, 1,000 and 10,000 steps with resident execution and an independent Rust #3
executable. A separate generated copy adds only a stderr state diagnostic to
Rust; timed Rust sources remain unchanged. Maximum scaled state error must be
at most `1e-10`. Compiler tests cover repeated scatters, broadcasting, multiple
states, odd SIMD tails, reduction summand order, signed zero, named outputs,
constant outputs without state, and rejection of unknown outputs. Default SIMD
and independent-instruction scheduling must preserve scalar state bits.

The 50M-step ablation completed 96 full-state comparisons across the selected
native variants and small inputs. Maximum scaled error was `2.77e-12` against
resident Mech and `2.88e-12` against the independent Rust program.

## Optimization policy

The default native path preserves the authored arithmetic order. It folds
squares to multiplication and half powers to square root only with a nonnegative
expression proof. It does not silently reassociate arithmetic or replace generic
powers with a different formula. Packed f64x2 state, explicit lane shuffles,
reuse of packed operands, and demand-based dead-instruction removal avoid
unnecessary packing and materialization. Coordinate pairs and independent
reductions are vectorized; repeated updates keep their accumulation order.

Additional optimizations are explicit compiler options, recorded per result:

- `--fp-contract` permits fused multiply-add/subtract. This changes rounding.
  Finite negative constant coefficients can be absorbed into the fused
  operation's sign, allowing opposite contributions to share constants.
- `--algebraic-powers` reduces `x ^ -1.5`, including a surrounding scale, to
  `scale / (x * sqrt(x))`. This changes rounding and intermediate range.
- `--refined-reciprocal` uses a single-precision square-root/reciprocal seed and
  two double-precision Newton refinements for inverse-cube expressions. It
  assumes positive inputs within the normal f32 range and changes arithmetic.
- `--schedule` changes the order of independent pure instructions by demand,
  preserving each operation's inputs and arithmetic order.

Flags that change floating-point behavior are assessed using the benchmark's
accuracy contract. They are not advertised as bit-identical to strict execution.
Reciprocal refinement and scheduling are measured even when they lose; adding
an optimization does not establish that it improves this machine's performance.
The ablation also includes LLVM automatic vectorization disabled, FMA enabled,
and broad fast-math, plus C/C++ with GCC and Clang under native `-O3` and
fast-math/unrolling/LTO. AOT builds finish before any timing begins.

The fastest published C, C++, and Rust entries use x86 AVX/SSE intrinsics and
reciprocal-square-root estimates with refinement. These exact submissions cannot
execute natively on Apple M3. Their sources are pinned and their architecture
exclusions are explicit. The Cranelift refinement experiment currently uses a
rounded f32 sqrt/divide seed, not the x86 estimate instruction. An x86 run is
needed to compare directly with those published winners.

## Measurement protocol and cohort

The expanded manifest pins 43 submissions: Chapel, Julia, Rust, Go, Java native
AOT source, C gcc, and C++ g++, including starred submissions. Source hashes,
author comments, original build notes and the BSD license are retained. Starred
means the site's classification, not a portable/strict-arithmetic guarantee.
For example, unstarred Julia #5 also uses reciprocal-square-root refinement.

Java sources run locally as OpenJDK JIT, not GraalVM native AOT. Upstream timings
are metadata and are never mixed with local timings. Compiler versions, full
commands, options, generated-object hashes and exclusions are recorded.

Each measurement starts a fresh process and includes startup, initialization,
simulation, and output, plus source/JIT compilation when applicable. One discarded
run warms file caches; it does not remove JIT preparation from later processes.
Three or more samples run sequentially in seeded shuffled order. Individual
samples, ranges and medians are saved. Kernel-only diagnostics are supplemental.
This macOS protocol does not reproduce CLBG's Linux isolation/cache/statistics
protocol, and host activity can affect measurements.

Source compactness uses nonblank, noncomment lines of code. The follow-up
report and current suite charts use LOC only. Older archived reports contain
additional size fields; those fields are not the current evaluation metric.

The GCC 16 bottle was built against SDK 26 while this host has SDK 14 headers.
The benchmark tooling uses the installed SDK's stdio definitions and suppresses
only unavailable, unused `quick_exit` declarations in libstdc++. This isolated
header compatibility file is archived with the commands; pinned benchmark
sources and their arithmetic are unchanged.
LLVM C++ configurations uniformly receive `-include cstdlib`, because one
submission relies on GCC's indirect inclusion of its declarations. This supplies
standard declarations without modifying the pinned source or its calculations.

## Historical measurements before the AC-power rerun

The earlier [Chapel-style optimization check](results/apple-m3-chapel-tricks-50m.json)
measures nine configurations with five shuffled samples each at 50M steps.
The complete [timing and code-generation table](results/apple-m3-chapel-tricks-50m.table.md)
retains ranges and strict/FMA/scheduling alternatives. Selected default-schedule
medians from this run are:

| Program | Seconds | Noncomment lines  |
| --- | ---: | ---:  |
| Chapel #3 | 1.706 | 79  |
| Rust #3 | 1.954 | 196  |
| Mech pairwise, Cranelift SIMD AOT + FMA | 1.984 | 38  |
| Mech original, Cranelift SIMD AOT + FMA | 2.029 | 32  |
| Mech compact inverse power, Cranelift SIMD AOT + FMA + algebraic powers | 2.030 | 32  |

The pairwise source replaces two indexed velocity statements with eight, a net
six extra source lines. `interaction` repeats each pair index for its two body
contributions; `body` names the receiving body; `sibling` and `other` select its
partner; `sign` chooses subtraction/addition. `weighted-m` folds these signs and
partner masses into constant coefficients. `dv` computes displacement times
magnitude once per pair, and one indexed update applies both contributions in
pair order. These static selectors are compiler-folded. The source changes
multiplication and accumulation order, while retaining all ten interactions and
the same integrator. It passes all energy checks and 28 full-state checks, with
maximum scaled error `2.644e-12` versus resident Mech and `2.257e-12` versus Rust.

The pairwise version is 2.2% faster than the original in this run, 1.5% slower
than Rust #3, and 16.3% slower than Chapel #3. Its extra source size creates a
speed/compactness tradeoff relative to the 32-line compact source. The native
loop falls from 314 to 251 instructions, 89 to 58 stack loads, and 33 to 22 stack
stores. Chapel's earlier audited loop has 204 instructions, 25 stack loads and
9 stack stores. These counts point toward further register-allocation and lane
layout work; they do not isolate the cause of the remaining performance gap.

Mech already implements Chapel's constant folding, static loop expansion,
unchecked resolved selectors, SIMD and explicit fused arithmetic. The pairwise
source adds a related update organization. Experiments also retained losing
results rather than assuming every compiler transformation helps:

- [Odd-coordinate packing](results/apple-m3-compact-layout-pilot-50m.json)
  was 10.5% slower and was reverted; the exact compiler is archived.
- Demand scheduling and a tested pressure heuristic produced identical hot-loop
  machine code after Cranelift optimization. The ineffective pressure heuristic
  was removed; its [pilot and snapshot](results/apple-m3-chapel-tricks-pilot-5m.json)
  preserve the experiment. Default-schedule and demand-scheduled ranges overlap.
- Replacing fused `x*x + +0` with multiplication reduced instructions and spills
  but slowed the original source from 2.029 to 2.067 s and pairwise source from
  1.981 to 2.009 s in the [five-sample ablation](results/apple-m3-square-zero-ablation-50m.json).
  It was reverted. This illustrates why smaller assembly is not itself a speed
  result.

The [earlier optimized language comparison](results/apple-m3-v04-native-optimized-50m.json)
contains 73 validated configurations from 37 distinct programs: 34 portable
upstream sources and three complete Mech sources. Nine pinned x86-only sources
account for 27 excluded configurations. No portable configurations failed.
The [chart](results/apple-m3-v04-native-optimized-50m.svg) and
[endpoint table](results/apple-m3-v04-native-optimized-50m.table.md) use only local
measurements. Here are the fastest observed entries:

| Language / selected compiler | Program | Seconds | Noncomment lines  |
| --- | --- | ---: | ---:  |
| C++, LLVM native O3 | #6 | 1.634 | 202  |
| Chapel | #3 | 1.703 | 79  |
| Rust | #3 | 1.956 | 196  |
| Mech, Cranelift SIMD AOT + FMA + algebraic powers | square-root source | 2.029 | 32  |
| C, LLVM fast-math/unrolling/LTO | #8 | 2.691 | 127  |
| Go | #3 | 2.767 | 135  |
| Julia | #5 | 2.799 | 90  |
| Java, OpenJDK JIT | #5 | 3.453 | 130  |

Mech is 3.7% slower than Rust #3 and 24.2% slower than the fastest measured
C++ entry. Mech's fastest and smallest candidates have medians 2.028862 s and
2.029187 s, with overlapping sample ranges. Both programs have 32 LOC. These three
samples do not establish a meaningful runtime difference. Go's endpoints are
the same program. Julia's smallest candidate takes 4.386 times its fastest.

**Rust #3 is the fastest of five measured ARM-compatible Rust submissions,
not the fastest published Rust submission.** Rust #9, the published winner,
uses x86 AVX/SSE intrinsics and reciprocal-square-root estimates and cannot run
natively on this M3; #7 and #5 are also excluded. The 3.7% statement is relative
to Rust #3, not to Rust's best implementation across all hardware.

The [Chapel/Rust code-generation audit](results/apple-m3-chapel-rust-audit.json)
finds 44 fused multiply-add/subtract instructions in Chapel #3's hot loop and
none in Rust #3's. Chapel's body-pair loops use compile-time `for param` unrolling
and update velocities immediately; Rust #3 separates displacement, magnitude
and update passes. Both generated loops use scalar and vector sqrt/divide.
These code-generation differences plausibly contribute to Chapel's local lead;
no isolated experiment establishes how much each contributes. Published CLBG
timings are on different hardware/toolchains and order Rust #3 ahead of Chapel
#3, so the local ordering is not a general language ranking.

The separate [optimized JIT check](results/apple-m3-v04-native-optimized-jit-50m.json)
passed all four configurations with no exclusions. The compact Mech source
takes 2.230 s and the square-root source 2.246 s, including source compilation,
activation, lowering and JIT preparation in every fresh process. AOT excludes
those ahead-of-time build costs. Both paths use the same Cranelift arithmetic.

The best time within each C/C++ compiler-option category is shown below; rows
can select different source submissions. Aggressive flags are not uniformly
beneficial, so the primary chart admits all four categories:

| Compiler policy | Best C (s) | Best C++ (s) |
| --- | ---: | ---: |
| GCC native O3 | 2.760 | 2.307 |
| GCC fast-math/unrolling/LTO | 3.048 | 2.321 |
| LLVM native O3 | 2.722 | 1.634 |
| LLVM fast-math/unrolling/LTO | 2.691 | 1.675 |

The current compiler ablation is saved in
[apple-m3-native-output-ablation-50m.json](results/apple-m3-native-output-ablation-50m.json).
All rows below use 50,000,000 steps and the median of three fresh processes:

| Mech execution/optimization | Elapsed seconds |
| --- | ---: |
| Cranelift scalar AOT, square-root source | 2.421 |
| Cranelift SIMD AOT, strict arithmetic | 2.167 |
| Cranelift SIMD AOT with FMA | 2.029 |
| Compact inverse-power source, strict SIMD AOT | 9.264 |
| Compact source, algebraic power reduction + FMA + SIMD AOT | 2.031 |
| SIMD reciprocal refinement experiment | 3.343 |
| LLVM square-root source, broad fast-math | 2.022 |

Rust #3 takes 1.956 s in that ablation. SIMD plus FMA is approximately 3.8%
slower; the two compact/explicit Mech formulations overlap in sample range.
The primary comparison therefore applies **SIMD + FMA + algebraic power
reduction** consistently to all three Mech sources. Reciprocal refinement and
the independent-instruction scheduler remain off because they did not help.

[Native instruction inspection](results/apple-m3-optimized-codegen.json) confirms
five f64x2 square-root/divide operations per turn, covering all ten interacting
pairs, FMA instructions, and no library calls in the optimized inverse-power
loop. The loop still contains 89 stack loads and 33 stack stores, excluding
one-time register saves. Register pressure remains an optimization opportunity;
these static counts alone do not establish the limiting performance cost.
The fastest measured C++ entry, #6 under LLVM native O3, has 172 loop
instructions, 11 stack loads and 2 stack stores. It also uses square root and
division, with no reciprocal-estimate instructions. This provides a concrete
target for subsequent layout, register-allocation and scheduling improvements.
For comparison, C++ #9 under LLVM fast-math has 169 loop instructions, 21 stack
loads and 9 stack stores, with no reciprocal-estimate instructions. Mech has
314 loop instructions. The C++ configuration allows broader reassociation;
different arithmetic schedules and lane layouts are part of this diagnostic.

The chart uses `x = size / minimum_size` and `y = time / minimum_time`, normalized
to successfully validated local entries. Both coordinates of each endpoint come
from that actual program. A connecting line is a tradeoff between observed
implementations, not a continuous frontier. Coincident endpoints mean one
candidate is both fastest and smallest; one candidate trivially coincides, and
small timing differences can be noise. Normalization is restricted to the
measured ARM-compatible cohort, including explicitly reported exclusions.

## Reproduction

Use Rust/Cargo, Clang/LLVM, GCC/g++, Chapel, Go, Julia, and OpenJDK. The runner
selects toolchains with `--clang`, `--gcc`, `--gpp`, `--julia`, and `--java-home`.
The nested lockfile pins the benchmark driver; its compiled entry point is a
thin call to the generic Mech runner.

```sh
cargo run --release -p mech-gpu --features fused-cli --bin mech-native-f64 -- \
  1000 benchmarks/clbg/nbody/nbody.mec jit-simd --output energy \
  --fp-contract --algebraic-powers

# Direct Cranelift AOT emission; Clang only builds the generic CLI and links.
cargo run --release -p mech-gpu --features fused-cli --bin mech-native-f64 -- \
  0 benchmarks/clbg/nbody/nbody.mec emit-cranelift-simd \
  /tmp/mech-nbody.c /tmp/mech-nbody.o --output energy --fp-contract
clang -O3 -isysroot "$(xcrun --show-sdk-path)" /tmp/mech-nbody.c \
  /tmp/mech-nbody.o -lm -o /tmp/mech-nbody
/tmp/mech-nbody 50000000

python3 benchmarks/clbg/nbody/compiled_ablation.py \
  --include-llvm --include-c --steps 50000000 --samples 3 \
  --output benchmarks/clbg/nbody/results/ablation.json

python3 benchmarks/clbg/nbody/chapel_tricks.py --steps 50000000 --samples 5 \
  --output benchmarks/clbg/nbody/results/chapel-tricks.json

python3 benchmarks/clbg/nbody/run.py \
  --julia /path/to/julia --java-home /path/to/jdk \
  --mech-mode aot-simd --mech-fp-contract --mech-algebraic-powers \
  --compare-cc --steps 50000000 --samples 3 --allow-partial \
  --output benchmarks/clbg/nbody/results/full-input.json

python3 benchmarks/clbg/nbody/plot.py \
  benchmarks/clbg/nbody/results/full-input.json

cargo test --release --locked \
  --manifest-path benchmarks/clbg/nbody/mech-driver/Cargo.toml --test fused
python3 -m unittest discover -s benchmarks/clbg/nbody -p 'test_*.py'
```

`--allow-partial` is required to finish the expanded cohort on ARM because x86
intrinsic submissions are retained as explicit exclusions. Inspect those exclusions
before interpreting a chart. `--only` can select IDs, with Rust #3 required as the
energy reference. `plot.py` requires matplotlib and writes SVG, PNG and a table.
`fetch_sources.py --extend-native` appends missing pins without refreshing old
ones; removing the manifest explicitly is required to refresh all source pins.

## Historical experiments

Earlier sources put energy math in handwritten adapters. Their timing and size
boundaries must not be confused with the complete current Mech program. Exact
sources and compiler code are retained in the referenced archives:

- [Initial resident pilot, 1M steps](results/apple-m3-v04-pilot.json) and
  [5M steps](results/apple-m3-v04-5m.json): the original resident execution path.
- [Initial scalar Cranelift AOT, 50M steps](results/apple-m3-v04-cranelift-aot-50m.json):
  22 upstream programs plus three earlier Mech sources; original full-boundary
  version in `apple-m3-v04-cranelift-aot-50m-full-program.json`.
- [Initial greedy SIMD ablation](results/apple-m3-cranelift-ablation-50m.json):
  excessive lane packing/spills; SIMD was slower than scalar.
- [Structured SIMD before generic runtime outputs](results/apple-m3-structured-simd-ablation-50m.json):
  SIMD sqrt AOT improved to 2.181 s from 5.013 s for the initial strategy.
- [Earlier LLVM exploration](results/apple-m3-compiled-ablation-50m.json):
  optional LLVM prototype, not the Cranelift execution path.

The earlier compiler/source snapshots are
`compiler-greedy-v1.tar.gz` and `compiler-structured-v4-pre-runtime.tar.gz`.
The removed handwritten energy adapters are preserved in
`legacy-harness-v1.tar.gz`; historical reports identify that archive and hash.
New reports include their own `.compiler.tar.gz` snapshot with a SHA256 hash.

The native compiler validation now passes 18 focused Rust tests, the locked host
crate check, the unsafe-boundary audit and six benchmark-harness tests.
The unsafe audit predates the latest safe arithmetic changes; those introduce
no additional unsafe boundaries. Saved archive hashes were checked against the
compiler hashes in the new optimization reports. The original working
checkout's existing changes were left intact.
