# Crystal source and toolchain inventory

This is primary-source research, with no installation, archive download, build,
probe, hash sweep, source qualification or chart change. `inventory.json`
records all 21 published source candidates covering the ten current workloads.
Original source files and metadata are retained under `upstream`; no candidate
has a LOC count, timing sample or eligibility claim.

The current [Benchmarks Game index](https://benchmarksgame-team.pages.debian.net/benchmarksgame/index.html)
does not list Crystal. These candidates are published community implementations,
not current official measured submissions. The two pinned repositories are
[hanabi1224](https://github.com/hanabi1224/Programming-Language-Benchmarks/tree/ad18b203dd1769724f4eea94fc3ac1e99f6593e0)
and [kostya](https://github.com/kostya/crystal-benchmarks-game/tree/aaf7bd44118889605943df3200b7c9b0c6af6510).

| Current workload | Hanabi candidates | Kostya candidate | Static qualification finding |
|---|---|---|---|
| nbody | 1, scalar fields; 2, fixed three-element arrays | nbody/nbody.cr | All pair interactions, exact square roots/divisions and energies are visible. Test outputs and steps before selection. |
| mandelbrot | 1, pointer output arrays; 2, value arrays | mandelbrot/mandelbrot.cr | Both Hanabi forms emit MD5 and round dimensions; repair to complete exact-width PBM. Kostya scalar form already writes PBM bytes and tail bits. |
| spectralnorm | 1, implicit matrix loops | spectralnorm/spectralnorm.cr | Both use fresh implicit products. Normalize final output precision; preserve ten iterations/four products each. |
| binarytrees | 1, class Node and distinct leaves | binarytrees/binarytrees.cr | Hanabi matches current node-count workload. Kostya uses obsolete signed-item paired trees and extra output, so it is not a current candidate as written. |
| fannkuchredux | 1, fixed arrays and real flips | fannkuchredux/fannkuchredux.cr | Sequential permutation generation/global parity is visible. Validate small sizes and every permutation before a factoradic parallel adaptation. |
| fasta | 1, ordered Hash/CDF scan; 2, array/CDF binary search | fasta/fasta.cr | Direct binary64 division and carried RNG state are present. Test wrapping, zero size and ALU multiples; do not add generated-letter caches or integer-scaled CDFs. |
| knucleotide | None | knucleotide/knucleotide.cr | Seven growable string maps are computed, but THREE parser needs exact token boundaries, stop-at-next-record, comments and case repair; frequency ties need a lexical tiebreaker. |
| revcomp | None | revcomp/revcomp.cr | General IUPAC reversal/translation is present; qualify comments, CRLF, empty records and unterminated final lines. |
| regexredux | 1, current five ordered substitutions | regexdna/regexdna.cr | Use Hanabi's current redux algorithm and adapt file input to stdin. Kostya uses obsolete ambiguity expansion, so cannot be admitted as redux. |
| pidigits | 1, general LFT with extract(3)/extract(4) | pidigits/pidigits.cr | Hanabi has two actual arbitrary-precision divisions. Kostya has one divmod and infers the second condition from its remainder; this fails the agreed two-extract rule and also needs tail/zero handling. |

The smallest and fastest endpoints must be chosen after all faithful candidates
receive the same optimized compiler and canonical LOC rule. An upstream source
with a required repair cannot contribute a score merely because the canonical
input happens to conceal its error. Original sources remain separate from every
future adapted application source.

The native macOS toolchain is official
[Crystal 1.21.1](https://github.com/crystal-lang/crystal/releases/tag/1.21.1),
released 26 September 2026. Its signed tag resolves source commit
`73ea5c2b1bc3b3ac0ba400aa9da77af96f63d6ef`. The project provides a universal
[macOS archive](https://crystal-lang.org/install/from_targz/) for Intel and
Apple Silicon: `crystal-1.21.1-1-darwin-universal.tar.gz`, 59,333,512 bytes,
publisher SHA256 `ba34fa7fb8bc8b951f12d17e12f69c9d7ccce38900caa4b8a17f1b4f29d37605`.
The archive has not been downloaded and that digest has not been verified locally.
The isolated bootstrap must inspect its actual executable architecture, version,
LLVM version, package dependencies and all linked native libraries under the
shared mutex.

Both source forms should use `--release --no-debug --mcpu native`. The pinned
[compiler command implementation](https://github.com/crystal-lang/crystal/blob/73ea5c2b1bc3b3ac0ba400aa9da77af96f63d6ef/src/compiler/crystal/command.cr)
maps release to O3 plus one LLVM module, and maps native CPU to LLVM's host CPU.
Do not substitute compiler `--threads` for application worker configuration.
Default floating-point contraction/LLVM attributes still require inspection;
no fast-math or reciprocal approximation is assumed by this plan.

Crystal 1.21's [execution contexts](https://crystal-lang.org/reference/1.21/guides/parallelism.html)
need explicit opt-in to CPU parallelism. The default context has capacity one;
`CRYSTAL_WORKERS=8` alone is insufficient. Resize the default context to eight
or create a `Fiber::ExecutionContext::Parallel` with maximum eight, then use
ordinary spawned fibers and `Channel`/`WaitGroup`/`Sync` to collect results.
One capped context can keep all workload compute within eight concurrent fibers.
The parent blocks while children compute and restores result order before writing.
Runtime housekeeping system threads need separate accounting. The old
`preview_mt` model should not be assumed for this release.

[BigInt](https://crystal-lang.org/api/1.21.1/BigInt.html) is already a standard
GMP-backed arbitrary-precision type imported with `require "big"`. Pinned
`big_int.cr` implements `//` with GMP floored division and `tdiv` with truncated
division. Both pi extracts can therefore stay in Crystal application code.
The standard binding's [memory hooks](https://github.com/crystal-lang/crystal/blob/73ea5c2b1bc3b3ac0ba400aa9da77af96f63d6ef/src/big/lib_gmp.cr)
route GMP allocation/reallocation/free through Crystal GC. No new benchmark host
function or custom C spigot is needed. Pin the actual linked GMP library and
qualify constructors, signs, division, aliases and large buffers before considering
a distinct general mutable-primitive profile.

[Regex](https://crystal-lang.org/api/1.21.1/Regex.html) uses standard PCRE2 with
JIT support, so an optimized redux form can use ordinary library matching and
replacement APIs. Keep worker regex/match state separate, run no more than eight
count workers, and preserve the five replacement passes sequentially. Pin actual
PCRE2 and JIT availability; do not infer them from the language version alone.

The saved release [Boehm binding](https://github.com/crystal-lang/crystal/blob/73ea5c2b1bc3b3ac0ba400aa9da77af96f63d6ef/src/gc/boehm.cr)
uses normal traced allocation. Keep the collector enabled and keep binary-tree
nodes as actual separately allocated references, including leaves. General
`StaticArray(T,N)` is a fixed value array and can remove incidental numeric
heap allocations; its [release source](https://github.com/crystal-lang/crystal/blob/73ea5c2b1bc3b3ac0ba400aa9da77af96f63d6ef/src/static_array.cr)
is retained. No index-only tree, cached checksum or unbounded no-GC profile is
proposed. Standard [IO.write(Bytes)](https://github.com/crystal-lang/crystal/blob/73ea5c2b1bc3b3ac0ba400aa9da77af96f63d6ef/src/io.cr)
preserves full binary PBM payloads, including NULs; text can use ordinary builders
and bounded buffers.

For metadata, use release year **2014**, based on the official
[0.1.0 release record](https://crystal-lang.org/releases/). Development began
earlier; it is a different date. Proposed existing chart group is AOT, with
descriptors statically typed, object oriented and garbage collected. GitHub
Linguist's pinned [language mapping](https://github.com/github-linguist/linguist/blob/5fbdfcb8133be2bed88bf3ce62b2335f50474525/lib/linguist/languages.yml)
has Crystal color `#000100`. Its near-black value needs a reviewed visibility
treatment against the dark chart; this research does not choose an arbitrary hue
or change any chart.
