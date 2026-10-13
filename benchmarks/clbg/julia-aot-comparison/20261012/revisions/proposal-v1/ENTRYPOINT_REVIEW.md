# Original Julia entry/state review, source-only proposal

All seventeen distinct source files were read completely. Paths below are
relative to benchmarks/clbg. The twenty displayed-role mapping is preserved in
existing-source-roles.json; alias roles share their actual source identity. This
review is not compiler coverage, allocation proof, an executable derived-source
packet or native qualification. No hash/parser/compile/native run has occurred.

| Original source | Runtime driver/state obligation | Source-sensitive proof |
| --- | --- | --- |
| binarytrees/upstream/binarytrees-julia-3.jl | Replace only top-level guarded call with run_original_driver calling binary_trees(stdout,parse(Int,ARGS[1])). Preserve latch, nested @threads and locked count. | Actual scored fresh leaves/interiors and retained long graph; eight compute threads. |
| binarytrees/upstream/binarytrees-julia-4.jl | Preserve guarded driver and actual Distributed launch policy; current command adds no workers. @everywhere definition setup requires image-safe reviewed derivation, with no runtime definition/eval. | Allocation evidence and actual distributed execution/worker ownership; do not silently convert to threaded source. |
| fannkuchredux/upstream/fannkuchredux-julia-1.jl | Move final n parse, pf computation and both prints together into compiled runtime driver. Function body stays exact. | Full permutation/checksum work, no prebuilt permutation/results. |
| fannkuchredux/upstream/fannkuchredux-julia-2.jl | Move final parse/fannkuchredux call into driver; preferred_num_blocks=24 is algorithm constant. Construct Fannkuch/Perm scratch arrays inside normal runtime functions. | Same block partition, per-thread result arrays and reduction. |
| fasta/upstream/fasta-julia-3.jl | Runtime parse must assign original global n because perf_fasta(_n) actually reads global n. Construct/reset state Ref(42) and mutable probability arrays at runtime before driver; preserve original read-only CodeUnits byte literals, LCG sequence and record order. | Full 2n/3n/5n generation; no serialized warm random state. |
| knucleotide/upstream/knucleotide-julia-1.jl | Move perf_k_nucleotide() call into driver; stdin reads and every Dict are already in its body. Definition/import of Base.isless is build-time code, not a runtime evaluation. | Seven complete maps and all windows; preserve sequence parsing and full histogram lookups. |
| knucleotide/upstream/knucleotide-julia-6.jl | Move guarded main(stdin,stdout) call into driver. COUNTSFOR is literal query data. Preserve source's Base.hash method and UInt32/UInt map choice. | Seven complete maps, both frequency maps plus five query maps, worker completion and ownership. |
| mandelbrot/upstream/mandelbrot-julia-1.jl | Move final runtime parse/perf_mandelbrot call. Keep n%8 validation and full allocation/render/write in body. | Same complex recurrence, all 50 prescribed iterations/escape test and complete bitmap. |
| mandelbrot/upstream/mandelbrot-julia-7.jl | Move guarded driver call. Preserve Base.Cartesian macros, tuple lane arithmetic, per-row @spawn tasks and pruning. | Same all-row/all-lane work and observed compute cap; compile macro expansion before runtime. |
| nbody/upstream/nbody-julia-5.jl | Move literal bodies construction and push!(bodies,init_sun(...)) into fresh runtime initialization before the two prints/advance driver. Preserve original const-global vector semantics through a reviewed initializer rather than snapshot a warmed vector. | Exact energy/advance arithmetic and new per-process body state; any binding/type change must be explicit. |
| pidigits/upstream/pidigits-julia-3.jl | Move final parse/pidigits call into driver. Keep GMP bindings and actual two extract! calls, with all BigInts constructed per invocation. | Both actual quotient extractions on each candidate; preserve signed/truncated GMP semantics and current conformance source/report identity. |
| pidigits/upstream/pidigits-julia-4.jl | Runtime parse assigns original global n, also used in trailing-line label. Keep both extractDigit(3/4) calls and divrem, not a cached second extraction. | Both divisions/GMP semantics and padded complete output, including nonmultiple-of-ten cases. |
| regexredux/upstream/regexredux-julia-1.jl | Move perf_regex_dna call. Build original mutable variants/subs and Regex literals in fresh runtime initialization; preserve normal regex construction/substitution sequence. | Nine counts and all five substitutions; generic PCRE2 pattern compile/JIT disclosed. |
| regexredux/upstream/regexredux-julia-3.jl | Move guarded driver. Create Regex tuple objects per runtime initialization; retain original PCRE.exec override and explicit vendor pcre2_jit_match call. | Count task joined before length output; distinguish vendor regex JIT from forbidden Julia method JIT. |
| revcomp/upstream/revcomp-julia-7.jl | Bind original INIO/OUTIO to fresh runtime stdio and construct/fill COMPS table at runtime. Move guarded perf_revcomp driver; input remains seekable regular-file stdin. | Preserve chunk algorithm, seeks/newline boundary and all records; no serialized stream handles. |
| revcomp/upstream/revcomp-julia-9.jl | Same runtime stdio/table obligations; preserve original in-place two-end loop and guarded driver. | Same byte swaps, newline handling and complete record output. |
| spectralnorm/upstream/spectralnorm-julia-4.jl | Move guarded parse/main/format call. Working arrays remain allocated in main; preserve odd-n adjustment and static @threads. | Complete ten iterations and same matrix arithmetic/thread policy. |

Mutable global const bindings cannot simply become const aliases to build-time
objects. Candidate derivation needs a reviewed runtime __init__/binding route or
an explicit source/profile change with matched JIT comparison. None is assumed
resolved by this table. Each app is isolated so Base.hash/PCRE.exec definitions
cannot contaminate another scored source. Any code adaptation affecting original
algorithm/source semantics requires an additive provenance record and source
review; old JIT data is not silently relabeled as matching changed source.

The direct_driver.c + entry_template.jl candidate uses create_library's exported
@ccallable ABI and init_julia/shutdown_julia, avoiding the default app launcher's
runtime expression evaluation. It remains conditional on actual exported symbol,
image/IR/native coverage and runtime initialization evidence. The fixed native
command passes original input arguments followed by --julia-args and exactly
--compile=no --threads=8,0 --startup-file=no --history-file=no -O3. A missing or
changed policy fails before workload entry. All app-specific state/entry glue
and this generic C launcher are authored compiled input, counted in later LOC.
