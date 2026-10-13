# Pony primary-source preparation receipt

Read-only web/API research on 2026-10-12. No compiler download, install, build,
source/report hash sweep, benchmark run, or LOC certification occurred here.
Pony is outside the existing legacy LOC pool; qualify its source/work/timings
before applying the approved canonical LOC policy.

## Native compiler and execution

- Official release: [ponyc 0.74.0](https://github.com/ponylang/ponyc/releases/tag/0.74.0),
  dated 2026-09-30, LLVM 23.1.2.
- Native ARM64 macOS asset:
  `https://github.com/ponylang/ponyc/releases/download/0.74.0/ponyc-arm64-apple-darwin.tar.gz`.
  Official published SHA-256 (not locally verified here):
  `02a20dce1c283da67966a57c58093e87981eb896e0cddefdcafe949ff17870ad`.
- [Installation instructions](https://raw.githubusercontent.com/ponylang/ponyc/0.74.0/INSTALL.md)
  require Xcode Command Line Tools/macOS SDK; bundled linker and macOS 13 target.
  Compiler uses host CPU/features and full LTO by default. Do not copy published
  x86 `--cpu=broadwell` flags onto ARM64.
- Compiled application runtime `--ponymaxthreads 8` (or 1 for a sequential
  profile) caps ordinary scheduler workers. CLI settings override `Main` defaults.
  ASIO/pinned/main contexts are additional; use ordinary actors for bounded
  compute workers. `--ponynoscale` conflicts with `--ponyminthreads`.
  See official [runtime startup](https://raw.githubusercontent.com/ponylang/ponyc/0.74.0/src/libponyrt/sched/start.c)
  and [scheduler](https://raw.githubusercontent.com/ponylang/ponyc/0.74.0/src/libponyrt/sched/scheduler.c).

## Published candidate inventory

The Hanabi repository is **hanabi1224/Programming-Language-Benchmarks**.
Pinned commit `ad18b203dd1769724f4eea94fc3ac1e99f6593e0`:

- [nbody/1.pony](https://raw.githubusercontent.com/hanabi1224/Programming-Language-Benchmarks/ad18b203dd1769724f4eea94fc3ac1e99f6593e0/bench/algorithm/nbody/1.pony)
- [binarytrees/1.pony](https://raw.githubusercontent.com/hanabi1224/Programming-Language-Benchmarks/ad18b203dd1769724f4eea94fc3ac1e99f6593e0/bench/algorithm/binarytrees/1.pony)

Official ponyc commit `85eb7c6a9a6b2a9433f70e9d4bc2f152509d30b4`:

- [Mandelbrot main](https://raw.githubusercontent.com/ponylang/ponyc/85eb7c6a9a6b2a9433f70e9d4bc2f152509d30b4/examples/benchmarks/mandelbrot/main.pony)
- [Mandelbrot worker](https://raw.githubusercontent.com/ponylang/ponyc/85eb7c6a9a6b2a9433f70e9d4bc2f152509d30b4/examples/benchmarks/mandelbrot/mandelbrot.pony)

Published author [spectral-norm gist](https://gist.github.com/renatoathaydes/83461b6a789042f2e95f9860957444c6)
is a prospective source; pin its actual revision/download bytes during qualification.
No current official Benchmarks Game Pony listing was found. No published Pony
candidate was found for fannkuch-redux, fasta, k-nucleotide, reverse-complement,
regex-redux, or pi-digits. Label those implementations local Pony ports rather
than inventing a published source provenance.

## Required-work adaptations to review

- Binary-trees allocates leaves, but allocating the full workload in `Main.create`
  prevents GC until the behavior returns. Use bounded behavior batches and retain
  the long-lived tree. See [official GC caveat](https://tutorial.ponylang.io/gotchas/garbage-collection.html).
- N-body does the five-body/ten-pair workload. Validate/fix the custom nine-digit
  formatter's carry behavior instead of changing arithmetic/output requirements.
- Official Mandelbrot uses F32, file output/noncanonical header, roughly 1,000
  workers at width 16,000/chunk 16, and premature `remaining == 1` disposal.
  Adapt to required F64 arithmetic, canonical PBM stdout, at most eight compute
  workers, and counted completion before output/shutdown.
- Spectral-norm creates an actor per row and assumes the result with index n-1
  arrives last. Bound actor workers and count received results.

## General standard-library/FFI primitives

- [OutStream](https://stdlib.ponylang.io/builtin-OutStream/) `write`/`writev`
  accepts byte sequences asynchronously; `print` adds newline. Order chunks in
  a coordinator and include completed output/process shutdown in the timing.
- [InputNotify](https://stdlib.ponylang.io/builtin-InputNotify/) receives
  `Array[U8] iso` through `apply`; `dispose` signals end of input.
- [C FFI](https://tutorial.ponylang.io/c-ffi/calling-c):
  `use @name[Return](...)`, typed pointers/structs; variadic declarations need `...`.
- Reviewed numeric APIs expose I128/U128 but no arbitrary-precision integer type.
  A general GMP primitive binding is appropriate for pi-digits; keep the spigot
  loop and both extract(3)/extract(4) operations visible in Pony source.
- Official [ponylang/regex](https://github.com/ponylang/regex), version 1.1.7,
  uses PCRE2. `Regex(pattern, jit=true)?`; `matches`, `apply(subject, offset)?`,
  `replace(..., global=true)?`. [Match.end_pos](https://raw.githubusercontent.com/ponylang/regex/main/regex/match.pony)
  is inclusive, so next scan starts at end+1. A no-match replace raises; preserve
  the unchanged subject in the error branch. Builtin
  [IRegexp](https://stdlib.ponylang.io/iregex-IRegexp/) lacks the span/global-replace
  primitives needed here. General regex/bigint primitives are acceptable;
  whole-benchmark host executors are not.

## Display metadata

Pony publicly released in 2015, developed from 2014:
[official early history](https://www.ponylang.io/blog/2017/05/an-early-history-of-pony/).
Native AOT, statically typed, object/actor language with reference capabilities.
[GitHub Linguist](https://raw.githubusercontent.com/github-linguist/linguist/main/lib/linguist/languages.yml)
lists Pony ID 290 and `.pony` but no language color. Use a clearly labelled
branding fallback from the [official site palette](https://raw.githubusercontent.com/ponylang/ponylang-website/main/mkdocs.yml)
(brown primary/amber accent), not an invented GitHub color.
