# Binary trees — Apple M3

This eighth benchmark uses the official performance input **N=21**. The workload
constructs and traverses 613,766,494 nodes, including separately allocated leaves.
It discards the stretch tree, retains the long-lived tree throughout temporary
work, and checks that tree last.

## Purpose and current language findings

The [suite directive](../README.md) is to stress Mech's language and stdlib across domains. Benchmark
algorithms must remain in Mech source using standard forms wherever available.
Missing expressiveness is a finding to record, not a reason to hide the algorithm
behind a host library call. Runtime/library additions and compiler optimizations
must apply to other programs; benchmark recognition, dedicated kernels, and
benchmark-specific execution modes are excluded.

`binarytrees.mec` and `binarytrees-parallel.mec` define construction, recursive
traversal, iteration, checks, and output in Mech. Both are complete sources. The
backend contains no tree construction/traversal kernel or node-count shortcut.

This native subset lacked source recursion and linked dynamic records. It now
supports ordinary record literals, declared record kinds and aliases, nested
fields, field annotations, and named field access. Records can be function
arguments/results and loop state. Source recursion compiles to ordinary native
calls with checked depth; acyclic helpers inline and scalar results return in
registers. Recursive inlining has a general bounded budget (default 2;
`MECH_CONTROL_RECURSIVE_INLINE=0`, `1`, or `2`). It applies to source call graphs,
without recognizing any benchmark or domain. The cohort retains all three
parallel policies and the default sequential policy using the same complete
sources. The earlier 55-build report measured 50-line sources; the current
canonical-form versions are 29 lines each.

### Canonical matches and comprehensions

The backend now lowers existing inline matches and pattern functions. `=>` is a
match arm; `->` is a transition. Construction uses a pattern function and the
leaf test is an inline match:

```mech
Make(depth<u64>) => <ref>
  | 0 => memory/allocate({left: memory/null(), right: memory/null()})
  | d => memory/allocate({left: Make(d - 1), right: Make(d - 1)}).
count := (node.left ? | memory/null() => 1 | * => 1 + Count(node.left) + Count(node.right))
```

No branch callback or `control/if` operation is needed. The compiler evaluates
the scrutinee once, tests patterns and guards in order, and evaluates only the
selected body. Bindings belong to their arm. Native lowering currently admits
scalar constants, null references, bindings, wildcards, and nested tuple
patterns. Inline matches must be exhaustive; an unmatched partial pattern
function fails with a checked runtime error. Array/tag/enum patterns and optional
results from incomplete inline matches remain backend gaps.

An irrefutable arm lowers directly to the remaining control-flow path. It does
not test constant `true` at runtime or retain an impossible no-match failure.
Inspection of the first native match implementation found those instructions
in generated code; the optimization applies to every such match and to the
fallback arms used by fused comprehension filters.

Standard comprehension sums replace named loop-condition and loop-step helpers:

```mech
check := stats/sum([memory/with-frame(Tree, depth) | i <- start..end])
done := stats/sum([Depth(d, maximum) | d <- 4..2..=maximum])
```

This is general consumer fusion: `stats/sum` over a matrix comprehension lowers
to scalar accumulation without constructing an intermediate matrix. It supports
u64 ascending ranges, explicit steps, inclusive endpoints, nested generators,
immutable qualifier bindings, boolean filters, and u64/f64 values. Effects retain
source order, filters skip subsequent qualifiers and the body, and iteration
terminates safely at the maximum u64 endpoint. Materialized comprehensions,
other consumers, non-range generators, descending/signed ranges, and set
comprehensions remain explicit native-backend gaps.

The benchmark declares `<node> := <{left<ref>, right<ref>}>`; construction uses
ordinary record literals and traversal reads `.left` and `.right`. Managed links
use the new opaque `<ref>` kind, with an explicit record kind on reads. They are
nullable, scoped links into homogeneous record arenas. This implements linked
recursive data through general memory operations. Implicit recursive kind
aliases, mutable/cyclic links, and pointer-containing records that outlive their
scope remain unsupported; these are explicit language findings. Julia #4's
complete source is still smaller at 24 lines. Mech still requires explicit
function kinds, typed memory reads, capacity formulas, and scope callbacks.
Recursive typed links and lexical region forms/type inference need separate
general designs; this change does not hide that overhead in a tree library.

## Arena integration

The rejected initial implementation used one `Box` per node and a thread-local
`HashMap` ownership registry. It bypassed Mech's existing planned arenas. Its
unfinished measurements and complete compiler snapshot are preserved in
`results/apple-m3-individual-box-abandoned.json` and the corresponding archive;
its sources are in `experiments/individual-box/`. It is not a completed study.

The current implementation uses **existing core memory authority**:
`MemoryDomain`, `RealizedMemoryPlan`, and `PlannedArenaProjection<u64>`.
`memory/with-arena(capacity, <record-kind>, callback, arguments)` realizes one
planned contiguous block and invokes source code synchronously in that exclusive
projection. `memory/with-frame(callback, arguments)` retires a bounded nested
lifetime without another physical allocation. Arguments/results may be copied
scalars or records of scalars; references, including nested reference fields,
cannot escape or cross regions. Workers create independent realizations; the
long-lived tree stays in its coordinator region.

`memory/null`, `memory/allocate(record)`, and `memory/read(ref, <record-kind>)`
provide links and typed views of ordinary records. Supported layouts contain
1–32 u64, f64, bool, or reference lanes, including nested records. Each source
layout has a distinct same-object identifier: equal arity alone cannot authorize
a differently typed or named read. Allocation/read lower to checked native
stores/loads. A proven live matching layout may select a private native function
entry without repeating that check; capacity, live-handle, and recursion checks
remain. Proofs follow control-flow dominance and do not escape branches or
possibly empty loops.

The low-level `heap/null`, `heap/new-refs`, `heap/read-refs`, and
`memory/with-ref-arena` forms remain for compatibility/conformance. The benchmark
uses ordinary named records instead. Per-node `take`/`stats` APIs were removed.
There is no free list, ownership map, or special tree allocator. The program
chooses storage capacity and lifetime; that capacity calculation does not replace
constructing and visiting every node.

## Validation and timing

`conformance.py` checks byte-exact outputs, distinct leaves and aliases, scope
escape rejection, capacity/arity guards, bounded frame reuse, nested regions,
recursion depth, shift bounds, ordered IO, and closed workers. `regressions.py`
checks byte-identical FASTA and fannkuch native objects and full FASTA output.
`records_conformance.py` exercises non-tree payload records, nested fields,
record loop state, mixed scalar codecs, aggregate calls, layout mismatches, and
proof dominance, and argument-effect order. The two suites pass 75 checks.
`forms_conformance.py` checks match laziness and binding scope, pattern recursion,
guard order, aggregate results, comprehension fusion and effect order, nested
generators, range endpoints, and rejection of unsupported forms on non-tree
programs. Every report records the compiler hash used for these suites.
`audit.py` checks production unsafe boundaries while excluding
generated projects.

The foreign cohort pins 35 submissions across Chapel, Julia, Rust, Go, Java, C,
and C++. C/C++ use GCC and LLVM builds; Rust uses normal and thin-LTO builds
against the same locked bitcode-enabled dependencies. Java uses OpenJDK JIT,
explicitly labeled because the submissions were published with GraalVM AOT.
Documented macOS shims supply standard malloc declarations and CPU enumeration.
LLVM libc++ cannot compile C++ #7's required parallel execution policy; GCC can.
The report labels individual allocation, default GC, and library pool policies.

Timing runs one fresh process at a time with complete output to a regular file.
Three samples per build are shuffled with a fixed seed. Compilation, tests,
rendering, and output hashing are outside timing. Full N=21 output must match
455 bytes with SHA256
`341de11a51feab3d8122b4b5d6a68b038a2d14434aa9bc2372f39300bf5f48e1`.
The official N=10 reference and hashes are retained alongside pinned sources.

The performance target is Rust's approximately **0.7 seconds**, with C's
approximately **3.3 seconds** as an intermediate milestone. Measurements, rather
than that target, determine the result.

### Current canonical-form measurements

The optimized N=21 batch measures nine selected builds with three fresh-process
samples each. This is a matched comparison of both current Mech sources, three
generic recursion-inlining policies, and pinned Rust, C, and Julia controls.
It is not a replacement full-cohort ranking. Every build produces the exact
455-byte reference output, with all 613,766,494 logical node allocations.

| Source / build | Median seconds | Complete source LOC | gzip-1 bytes |
| --- | ---: | ---: | ---: |
| Mech parallel, default recursive inline budget 2 | 0.776333 | 29 | 634 |
| Rust #5 | 0.709921 | 69 | 748 |
| C #3, GCC, library pool | 0.881953 | 74 | 828 |
| C #5, LLVM, individual nodes | 3.483589 | 113 | 901 |
| Mech sequential | 3.123617 | 29 | 598 |
| Julia #3 | 3.935730 | 60 | 629 |
| Julia #4, smallest source | 5.342612 | 24 | 367 |

Mech's observed parallel median is 9.4% above Rust's in this batch. Mech samples
range from 0.768 to 0.862 seconds; Rust samples range from 0.674 to 0.735 seconds.
Three samples do not establish a statistically resolved ranking. The generic
irrefutable-arm optimization removes redundant instructions; the initial batch
below measured 0.849 seconds for Mech, but batches remain separate and have
their own contemporaneous controls. Source size fell from 50 to 29 lines (42%).
Julia #4 remains smaller, so the expressiveness work is not complete.

All 46 source-form checks, 75 existing memory/record/tree checks, four earlier
benchmark regressions, and the production unsafe-boundary audit pass with the
measured compiler. Both prior FASTA and fannkuch sources still produce identical
native objects. The report includes all four validation reports and an exact
compiler/source archive.

Current artifacts: [optimized selected-build report](results/apple-m3-forms-optimized-21.json),
[LOC chart](results/apple-m3-forms-optimized-21.corner.loc.png), and
[compressed-source chart](results/apple-m3-forms-optimized-21.corner.png).

The initial 29-line batch is preserved separately in
[apple-m3-forms-21.json](results/apple-m3-forms-21.json). It measured Mech parallel
at 0.848627 seconds and Rust #5 at 0.676237 seconds. The isolation sources in
`experiments/canonical-forms/` keep the complete algorithm while changing matches
and comprehension sums independently. Their diagnostic report is
[forms-isolation-before.json](results/forms-isolation-before.json); its callback
control produces exactly the earlier native object. These are observations of
the first lowering, before the irrefutable-arm optimization, not the current
optimized measurements.

The earlier completed N=21 comparison meets that target. It includes 55 validated builds,
with three fresh-process samples each:

| Source / build | Median seconds | Complete source LOC | Allocation |
| --- | ---: | ---: | --- |
| Mech parallel, default recursive inline budget 2 | 0.723920 | 50 | Core planned arena |
| Rust #5 | 0.711672 | 69 | Library arena |
| C #5, GCC | 3.148073 | 113 | Individual nodes |
| C #5, LLVM | 3.412644 | 113 | Individual nodes |
| Mech sequential, default recursive inline budget 2 | 3.154127 | 50 | Core planned arena |

Mech's parallel median is 1.7% above the fastest Rust median in this batch;
Mech's samples range from 0.686 to 0.747 seconds. Three samples establish the
observed result, not a statistically resolved ranking at this small difference.
C #5 is the individual-allocation comparator, not the fastest C submission:
C #3's library-pool GCC build has a 0.833551-second median. The complete report
retains every build and allocator policy.

The same parallel source with recursive inline budgets 0 and 1 has medians of
0.950268 and 0.982892 seconds, respectively. This measures a general compiler
policy rather than changing the benchmark algorithm. Earlier complete stages
and the abandoned per-node implementation remain separate preserved reports.

Preserved 50-line artifacts: [complete report](results/apple-m3-21.json),
[LOC chart](results/apple-m3-21.corner.loc.png),
[compressed-source chart](results/apple-m3-21.corner.png), and the
[interactive eight-benchmark gallery](../overview/preview.html).
The gallery retains the last full 55-build cohort; it is not a mixed chart of
the old foreign batch and new Mech measurements. The corresponding 50-line
sources are preserved in `experiments/callback-forms/`.

## Reproduction

```sh
RUSTFLAGS='-D warnings -C target-cpu=native' \
  CARGO_TARGET_DIR=benchmarks/clbg/nbody/mech-driver/target \
  cargo build --release --locked --offline --features cranelift_build
RUSTFLAGS='-C target-cpu=native -C embed-bitcode=yes' \
  CARGO_TARGET_DIR=benchmarks/clbg/binarytrees/.build/rust-deps-bitcode \
  cargo build --release --locked \
  --manifest-path benchmarks/clbg/binarytrees/rust-deps/Cargo.toml
python3 benchmarks/clbg/binarytrees/conformance.py
python3 benchmarks/clbg/binarytrees/records_conformance.py
python3 benchmarks/clbg/binarytrees/forms_conformance.py
python3 benchmarks/clbg/binarytrees/regressions.py
python3 benchmarks/clbg/binarytrees/audit.py
python3 benchmarks/clbg/binarytrees/run.py --prepare-only
python3 benchmarks/clbg/binarytrees/inline_policies.py
python3 benchmarks/clbg/binarytrees/run.py --reuse-builds --samples 3 --timeout 600
```

For the current selected-build comparison after the same validation/build stages:

```sh
python3 benchmarks/clbg/binarytrees/run.py --reuse-builds --samples 3 --timeout 600 \
  --only mech-binarytrees-compact,mech-binarytrees-parallel,mech-binarytrees-parallel-inline0,mech-binarytrees-parallel-inline1,binarytrees-rust-5,binarytrees-gcc-3-gcc,binarytrees-gcc-5-llvm,binarytrees-julia-3,binarytrees-julia-4 \
  --output benchmarks/clbg/binarytrees/results/apple-m3-forms-optimized-21.json
```

Run stages sequentially. Exact source pins, author comments, licenses, build
commands, compiler snapshots, worker settings, and exclusions are retained.

- [Official work and allocation rules](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/binarytrees.html)
- [Published cohort](https://benchmarksgame-team.pages.debian.net/benchmarksgame/performance/binarytrees.html)
