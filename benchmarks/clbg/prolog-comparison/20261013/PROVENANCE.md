# Initial Prolog provenance

The ten draft algorithms are newly authored Prolog translations of the
faithful algorithm structure already present in this repository's Nim/C3/Gleam
cohorts. No historical SWI shootout source was copied. Existing contributor
attribution remains in `../nim-comparison/20261012/PROVENANCE.md`; official task
descriptions remain the required-work authority.

Binarytrees retains the standard min/max/stretch/depth iteration schedule, real
recursive tree checks and retained long-lived root. Its deliberate runtime
functor factory avoids a compiled literal shared leaf. Pi retains the sequential
unbounded LFT recurrence and both real extract3/extract4 divisions. Spectral
retains the coefficient and two orientations as separate procedures and all40
dense products. Mandelbrot retains the full50-iteration escape loop, allpixels,
PBM header and zero padding. These are source designs awaiting actual validation.

All ten physical `optimized.pl` sources are deliberate identical aliases of the
corresponding `compact.pl`, with the same single-worker algorithm and explicit
`source_alias_of` metadata. They are separate candidate profile/QLF/source/sample
identities; no distinct optimization or transferred speedup is claimed. Native
metadata seal must prove byte equality before accepting this declared alias.

Counted support is ordinary CLI/argument/error/stream setup in
`drafts/private-v1/support/clbg_io.pl` and buffered CR/LF record operations in
`support/clbg_records.pl` for K/revcomp. Both helpers are original Prolog glue,
with no foreign domain predicate, subprocess or timing. The record reader uses
the general `read_string/5` delimiter API without padding, so spaces in input
data are preserved and preheader text is ignored by the application state.
All application algorithms remain in the corresponding Prolog main files.

Concrete algorithm predecessors consulted are
`../nim-comparison/20261012/sources/{fannkuchredux,fasta,nbody,regexredux,revcomp,knucleotide}/compact.nim`
and the faithful tree/Pi/spectral/Mandelbrot structures already reviewed in the
Nim/C3/Gleam packets. The exact constants, prefix-rotation checksum order,
case-sensitive nine regex patterns and ordered replacements are retained.
The new K port uses generic stable SWI hash tables with rolling integer keys,
all seven complete widths and every window; it does not adapt a host K kernel.
Fann uses fresh source-filled work terms before each actual prefix-flip count.
FASTA writes60-byte source-generated lines with each fresh LCG draw and float
CDF comparison. Nbody uses source-owned body terms and scalar destructive
numeric updates. Revcomp keeps record chunks/string storage and performs every
reverse index and IUPAC complement decision in source. Any native general
string, number, table or PCRE operation is disclosed and later pinned.

`tools/prepare_qlf.pl` is a separate unscored compiler/inventory/WAM extraction
tool. It is pinned as preparation evidence and is absent from every scored
application command and counted source bundle. Compiler inventories must
classify it explicitly; its VM/listing/JSON services cannot perform a workload.

Primary sources read during this source-only preparation:

- [Official CLBG history](https://benchmarksgame-team.pages.debian.net/benchmarksgame/sometimes-people-just-make-up-stuff.html) establishes that SWI/YAP are no longer measured, not current program coverage.
- [Official task descriptions](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/binarytrees.html), plus all nine linked descriptions in PLAN.md, establish workload requirements.
- [SWI stable distribution](https://www.swi-prolog.org/download/stable) and [stable V10.0.2 release](https://github.com/SWI-Prolog/swipl/releases/tag/V10.0.2) establish planned publisher/tag metadata only.
- [Stable hash-table source](https://raw.githubusercontent.com/SWI-Prolog/swipl/V10.0.2/library/hashtable.pl) establishes generic public growable table APIs for later K sources. Vendor source is not copied into an authored application.
- [Stable readutil source](https://raw.githubusercontent.com/SWI-Prolog/swipl/V10.0.2/library/readutil.pl) informed initial IO research; final scored record glue uses the documented built-in [read_string/5](https://www.swi-prolog.org/pldoc/man?predicate=read_string/5) rather than copying that module.
- [Term construction](https://www.swi-prolog.org/pldoc/man?predicate=functor/3), [identity](https://www.swi-prolog.org/pldoc/man?predicate=same_term/2), [arithmetic](https://www.swi-prolog.org/pldoc/man?section=arith), [QLF](https://www.swi-prolog.org/pldoc/man?predicate=qcompile/1), [clause indexing](https://www.swi-prolog.org/pldoc/man?section=jitindex) and [command options](https://www.swi-prolog.org/pldoc/man?section=cmdline) inform the proposed runtime/proof route; selected stable source and actual receipts remain mandatory.
- [General PCRE package](https://www.swi-prolog.org/pldoc/man?section=pcre) supplies matching for authored regex sources. Case-sensitive matching, nine separate nonoverlapping folds, five ordered global replacements and explicit `jit(false)` are source-visible; PCRE2 matching is native library work. The selected stable package must prove that option is honored rather than silently ignored.
- [Stable VM utility source](https://raw.githubusercontent.com/SWI-Prolog/swipl/V10.0.2/library/vm.pl) establishes `library(vm)`/`vm_list/1` for later original scored WAM extraction, which remains unexecuted. An earlier guessed `library/prolog_vm.pl` URL returned404; the module name is `prolog_vm` but its file is `vm.pl`.

No website-reported result, publisher checksum or source API observation is
relabeled as a local successful native measurement. All exact toolchain/source
hashes, downloaded archive verification, native/library receipts, source counts,
work proofs and timing are still absent.
