All twenty packages are local unqualified adaptations or ports. The source
inventory is the independent read-only receipt
../../loc-normalization/20261012/PONY-RESEARCH.md; no published timing or physical
line count is a score or audited application LOC.

The four published families are:

- Hanabi binarytrees/1.pony and nbody/1.pony, at
  ad18b203dd1769724f4eea94fc3ac1e99f6593e0 in
  hanabi1224/Programming-Language-Benchmarks.
- Official ponyc examples/benchmarks/mandelbrot/{main,mandelbrot}.pony at
  85eb7c6a9a6b2a9433f70e9d4bc2f152509d30b4.
- Renato Athaydes spectral-norm gist 83461b6a789042f2e95f9860957444c6,
  revision 9cf63693e55f495a277362fe7b3b856b8e15587b.

Separate exact upstream snapshots are preserved in upstream/. Trees retain
allocated Node leaves and the long-lived root, replacing the monolithic GC-
blocking constructor loop with one allocated/checked tree per worker behavior.
Nbody retains every five-body/ten-pair step and exact sqrt, replaces the defective
custom formatter with standard fixed nine decimals, and yields between bounded
step batches. Displacement/mass/magnitude operations retain canonical order.
Mandelbrot is rewritten around F64, requested width/tails, canonical complete
PBM stdout, counted completion and ordered parent output, repairing upstream
F32/file/header and premature-disposal behavior. Spectral norm computes every
dense product for ten iterations, uses bounded row actors and counted completion
instead of assuming row n-1 arrives last.

Fannkuch, fasta, nucleotide, reverse complement, regex-redux and pi are local
pure Pony ports of the canonical workload. Fann initializes descending
factoradics, advances the ascending rotation carry, uses global rank parity and
performs actual prefix reversals for every rank. FASTA uses each floating-CDF
draw and a continuous LCG stream with the exact 287-byte ALU literal. Nucleotide
collects exactly THREE with space/tab/end boundary, comments/CRLF/EOF, then builds
all seven growable maps and every overlapping window, including lengths 1 and 2.
Reverse complement preserves headers and full IUPAC complement/tail output.
Regex performs nine non-overlapping scans and the ordered five replacements.
Pi performs both true endpoint divisions before comparing every candidate.
Actor behavior batching changes scheduling, not work or the numerical recurrence.

General libraries proposed for private immutable vendoring:

- ponylang/regex 1.1.7, commit 589eb008db250aab17d4c073a41c5562ac5b8dc7,
  supplying ordinary PCRE2 matching/iterators/global replacement. No-match
  replacement retains the unchanged subject. Pin its complete package and the
  exact PCRE2 library; JIT behavior requires qualification.
- tadashi9e/gmp4pony, commit b14db4ba347f6a2b840f135e1602e19f85625c39,
  supplying general immutable GMP arithmetic operators and finalizers. Its
  README labels it pre-alpha and tested only on Ubuntu 20.04: current ARM64 LP64
  ABI, alias/sign/division behavior, capability ownership and finalizer cleanup
  require actual qualification. Its example pi is not used.

All application helpers count in LOC. These general library packages are pinned
and audited separately under root's consistent standard-library support policy;
no native workload kernel is introduced. Source/compiler/runtime hashes,
normalized LOC, actual output/work traces and timings remain pending.

Display metadata: Pony was publicly released in 2015 (development from 2014),
per the official early history. It is native AOT, statically typed, and an
object/actor language with reference capabilities. GitHub Linguist defines no
Pony color, so any website color must be labelled as an official-branding
fallback rather than a GitHub color. The 0.74.0 native ARM64 compiler asset is
ponyc-arm64-apple-darwin.tar.gz, publisher SHA256
02a20dce1c283da67966a57c58093e87981eb896e0cddefdcafe949ff17870ad.
