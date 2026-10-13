# Rebol native runtime and source research

Source-only proposal dated2026-10-13. No runtime/download/architecture/version,
hash, parser, build, fixture, timing, source count, seal, publication or Git
operation has occurred for this cohort. Actual qualification flags are false.

## Concrete maintained native candidate

Use the maintained Oldes Rebol3 fork, identified explicitly rather than calling
it the old closed REBOL2 distribution. Its
[latest published release](https://github.com/Oldes/Rebol3/releases/tag/3.22.1)
is3.22.1, abbreviated commit7fe158a. The complete
[publisher asset list](https://github.com/Oldes/Rebol3/releases/expanded_assets/3.22.1)
includes `rebol3-bulk-macos-arm64.gz`, published2026-05-27, SHA256
`6f078cfba45dcfc43502fee6795361180ce0f1ed6964521278a488613408311a`.
The Core ARM64 asset is also listed, SHA256
`bce639f203042e772f57871875ac907b8b5aa96728fe9d1aa03064f6c2c1785a`.
These are observed publisher digests, not locally verified downloaded/native
images. Default candidate is the single Bulk native ARM64 interpreter.

The tagged [README](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/README.md)
calls this an interpreter, describes Core/Bulk variants and its extension
mechanism. Tagged
[build definitions](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/make/rebol3.nest)
explicitly include a macos-arm64 Bulk build. This supports a concrete private
bootstrap path. Exact gzip bytes, decompressed runtime hash, tag/commit,
ARM64 image/linkage, actual version/options, imported modules and extension ABI
remain later common-lock receipt requirements. Do not infer native AOT from a
compiled interpreter or bundle/encapsulation. No optional native-code JIT
extension is part of the candidate.

The original REBOL site's [SDK2.7.8 downloads](https://www.rebol.com/downloads/v278/sdk/)
list old macOS Intel/PPC products; they are not this ARM64 candidate. Display
language history and the selected maintained implementation/version separately.

## Required primitives and counted general glue

Tagged [integer implementation](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/core/t-integer.c)
uses signed64-bit storage; the maintained
[datatype documentation](https://rebol.tech/docs/datatypes.html) describes
64-bit integers, IEEE64-bit decimals and interpreted user functions. This is
enough for36-bit K encodings but not unbounded Pi state. The native maps are
real hash associative maps; the exact
[map implementation](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/core/t-map.c)
provides the later allocation/capacity inspection route. Runtime source/ABI
and literal behavior must still bind the actual selected binary.

Do not assume REBOL2 routine! FFI exists in this exact Rebol3 build or that its
RSA internal bigint is a public arbitrary-integer datatype. A proposed reusable
native extension exposes only ordinary GMP new/free/set/add/multiply/compare/
quotient/string operations and PCRE2 compile/match/substitute/free, with no Pi
state or regex-redux orchestration in C. The tagged
[extension ABI header](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/include/reb-ext.h)
and official [extension concepts](https://www.rebol.com/r3/docs/concepts/extensions.html)
are the starting interface references. Its exact initialization/handle/GC
ownership/native-library source route needs source review before authoring a
guarded extension build. A pure reusable Rebol limb implementation is an
alternative if that ABI cannot be qualified, with all arithmetic source counted.

All extension C code, native ABI declarations and Rebol loader/wrapper code are
authored application helpers and count with each using workload. General vendor
GMP/PCRE2 runtime images are pinned/disclosed separately. IO and benchmark
algorithms remain source-written Rebol. No domain host kernels or external
benchmark executable wrappers.

## Ten programs and two honest forms

Author one faithful baseline per workload, preserve compact/optimized baseline
aliases if no distinct measured optimization is justified, and disclose those
aliases. Planned compute workers1, within the common maximum8; interpreter
startup/source loading and output are inside each fresh-process timing boundary.

| Workload | Rebol source path and later proof |
| --- | --- |
| binarytrees | Fresh mutable block/object before every depth branch including leaves; two recursive children and original long-root retention. Actual same? identity, graph lifecycle and unchanged interpreter allocator/source lowering reviewed later. |
| knucleotide | Seven maps and all windows;36-bit integer or fixed-width binary keys, actual growing map capacities/complete entry totals, source-written ranking and tie order. |
| pidigits | Rebol-written spigot and both numerator divisions on general unbounded GMP/limb values; independent zero/tail/full-digit oracle plus actual operand traces. |
| fasta | Floating CDFs and fresh LCG update for every random base, exact repeat output and60-column wrapping, including boundary cases. |
| revcomp | Binary record parser, ambiguity complement table, actual reversal, preheader ignore and exact wrapped output. |
| regexredux | All nine full generic pattern scans and five sequential substitutions controlled in Rebol; PCRE2 primitive/library boundary explicit. |
| spectralnorm | Source-written A/At triangular matrix loops for all ten iterations and final scalar reductions. |
| nbody | Source-written five-body momentum, pair force/velocity updates, complete step loop and both energy reductions with strict doubles. |
| mandelbrot | Full source pixel recurrence up to50 iterations, bit packing/padding and exact PBM byte output. |
| fannkuchredux | All permutations/counters/flip loops and checksum/max in Rebol; integer boundary/permutation oracle checks. |

Tagged [host argument parser](https://raw.githubusercontent.com/Oldes/Rebol3/3.22.1/src/os/host-args.c)
is authoritative for a later exact noninteractive command. Avoid importing
current live-doc options as historical3.22.1 receipts. Private child-local
security/module policy and extensions must be explicit, with no global
permission/package changes or interactive prompt in timed runs.

## Next packet and full qualification path

Propose a private `.build` bootstrap that acquires the common timing mutex,
checks named/held inode and retains a duplicate FD throughout every download,
unpack/owned query, build/native run and cleanup. Verify publisher gzip hash
before exclusive extraction; record decompressed actual ARM64 executable,
version, image closure and source/extension ABI. Root reviews that concrete
packet and owns one durable launch. No install or architecture probe is being
performed by this research note.

Then author/review all twenty source/helper bundles plus general extension,
bounded native outputs/primitive/ownership tests, actual tree/map/Pi observers,
original runtime allocation/retention and source-library policy. Count exact
original Rebol plus authored C/glue with a separate conservative canonical
reader/owner grammar, never compiled extension or generated artifacts in place
of authored source. All live metadata fields remain false until actual receipts.

Full timing uses ten sealed three-profile manifests, unchanged Mech controls,
three rotated rounds and90 actual owned fresh processes. Reconcile raw/supervisor
records and entire outputs before any metadata/LOC/publication admission.
Ordinary source failures with complete cleanup preserve their failed record
and allow independent manifests; integrity/ownership failure stops. No old
successful evidence is overwritten, restarted or assigned a new source identity.
