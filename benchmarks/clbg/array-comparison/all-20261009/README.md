# Full-workload BQN and K comparisons

These are faithful ports for all nine benchmarks beyond the existing, measured
n-body entries in `../nbody-20261009`. The explorer combines those five-run
n-body measurements with this report's three-run measurements, giving both
languages coverage of all ten official workloads.

Run `python3 run.py` from this directory to resume. `results/faithful-timings.json`
is marked complete only when every one of the nine benchmarks finishes both
languages and its Mech control. Small and full outputs are checked against
canonical hashes, energies or exact reference text before timing. Samples use
fresh processes, complete stdout to regular files, randomized sequential rounds,
AC power with Low Power Mode off, and compiler/power guards. No benchmark
processes run concurrently with another benchmark. Up to eight interpreter
workers are part of a single program's execution; startup and IPC are timed.

Time ratios against Mech use the explorer's fastest measured AOT endpoint for
the same workload. The Mech controls in this batch repeat existing binaries;
some are compact variants rather than the fastest variant. Their samples are
kept separately and are not pooled with another implementation's measurements.

`source-audit.json` pins every application, interpreter, library and support
source. Its work reviews are separate from output checks. The earlier matrix-
precomputing spectral sources and query-only nucleotide sources remain archived
and explicitly disqualified. New spectral ports define the four required
functions and evaluate coefficients during all forty matrix-vector products.
Nucleotide ports perform all 874,999,961 hash-table lookups and updates over the
seven widths, in the language source. Each reading frame is split into ordinary
ranges, and results from independently grown tables are summed after updates.

The native bindings in `stdlib/native.c` contain only general operations:

- GMP integer arithmetic: allocate, add, subtract, multiply, divide and compare.
- PCRE2 compile, count matches of one pattern, and global replacement.
- The unmodified khash 0.2.8 library's one-key get and set operations for K.
  BQN uses its built-in `HashMap`.
- POSIX pipe, fork, wait and exit. Children resume the BQN or K interpreter and
  execute the source callback. No callback computation occurs in C.

The language-level process map preserves input order, handles serializable
immutable results, and uses at most eight processes. It is reused unchanged by
fannkuch, nucleotide counting, and binary trees in both languages. All permutation
generation, prefix reversals, DNA packing, hash-table updates, tree construction
and tree traversals are in the application sources. There are no native benchmark
executors or input-specific shortcuts. Source selectors in the explorer label
these general bindings as runtime support, separate from application LOC.

Application LOC is nonblank, noncomment physical LOC plus the packed-statement
and inline-assignment expansions listed in the audit. Structural separators in
arguments, tuples, cases and lists are preserved. Compressed size is not a metric.

The pinned CBQN commit is `c893d3e7828899a50150df03ad33c9052fa51d3c`;
Growler is `48067b30de7c63f2d457727245ba34f7a714d1ce`. Both are compiled with
LLVM 22, `-O3 -mcpu=apple-m3`, with FFI enabled. CBQN links libffi 3.8.0;
bindings link GMP 6.3 and PCRE2 10.49. Growler's public extension API symbols
are exported so ordinary shared libraries can call its value constructors.

Build the general bindings with:

```sh
/opt/homebrew/opt/llvm@22/bin/clang-22 -O3 -mcpu=apple-m3 \
  -isysroot /Library/Developer/CommandLineTools/SDKs/MacOSX.sdk \
  -I/opt/homebrew/include -L/opt/homebrew/lib -lgmp -lpcre2-8 \
  -dynamiclib stdlib/native.c -o stdlib/libarray-standard.dylib
/opt/homebrew/opt/llvm@22/bin/clang-22 -O3 -mcpu=apple-m3 \
  -isysroot /Library/Developer/CommandLineTools/SDKs/MacOSX.sdk \
  -I/opt/homebrew/include -I/path/to/growler -L/opt/homebrew/lib \
  -lgmp -lpcre2-8 -DGROWLER -undefined dynamic_lookup -dynamiclib \
  stdlib/native.c -o stdlib/libgrowler-standard.dylib
```

K's load directive and library path identify the pinned local support files;
update them together and re-audit before a new measurement batch on another host.
Keep previously measured source and runtime snapshots unchanged.
