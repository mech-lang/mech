# Prolog comparison: source-only preparation

Status: all20 physical candidate application files and minimal preparation/full
framework are authored, unexecuted and unqualified. No
compiler/runtime discovery, installation, download verification, compilation,
hash, parser, fixture, native observer or timing has run for this packet. Every
such operation must later acquire `/tmp/mech-clbg-timing.lock` for its entire
lifetime and inherited child ownership. Root owns launches and Git checkpoints.

The user requested Prolog in the published comparison. The target is twenty
faithful compact/optimized applications over all ten workloads, with three full
fresh-process samples per exact source and fresh unchanged Mech controls. V
remains report-only. Initial inspection found no existing Prolog/SWI comparison
source packet or job in the benchmark corpus. The other language evidence and
frozen publication packets are untouched.

## Source availability and runtime candidate

The official [CLBG history](https://benchmarksgame-team.pages.debian.net/benchmarksgame/sometimes-people-just-make-up-stuff.html)
lists SWI Prolog and YAP Prolog among implementations no longer measured. Former
direct SWI binarytrees/nbody program URLs were unavailable through the web
reader. There is no claim that a current official Prolog ten-source suite was
found. Historical source recovery remains a provenance task, not a prerequisite
for freshly authored faithful ports.

The official [stable download page](https://www.swi-prolog.org/download/stable)
offers SWI-Prolog10.0.2-1 as a universal macOS bundle. Stable source is the
[SWI-Prolog/swipl V10.0.2 tag](https://github.com/SWI-Prolog/swipl/releases/tag/V10.0.2),
not a tag in swipl-devel. The initial mistaken swipl-devel tagged-source paths
returned404; the exact stable tagged library sources were then read successfully.
The planned distribution URL is
`https://www.swi-prolog.org/download/stable/bin/swipl-10.0.2-1.fat.dmg` with
publisher SHA256
`bf775f0b8d7880f4908dee513316013ef42a73793be392814fde2a0a8e9ddc5d`.
The source archive's published SHA256 is
`e42cc098f7b8a6051c4f79a99b55162d467098aba60f69649bdc7583f0734b57`.
These are publisher candidates, not locally verified archive/runtime receipts.

Later locked discovery must identify an existing suitable runtime or privately
extract the reviewed official bundle, pin the exact executable, selected ARM64
slice, shared images, boot state, library and foreign packages, and saved version
stdout. The live documentation site describes a development runtime, so its
API examples do not establish the installed stable version. Exact selected
tag/package source and actual receipts must settle API behavior.

The intended measured route is the SWI native bytecode virtual machine with
precompiled [QLF files](https://www.swi-prolog.org/pldoc/man?predicate=qcompile/1).
QLF compilation is preparation; startup and QLF/library loading remain inside
fresh-process timing. [JIT clause indexing](https://www.swi-prolog.org/pldoc/man?section=jitindex)
must not be relabelled as native machine-code compilation of the application.
The native PCRE2 regex library may itself use regex JIT; disclose that separately.
Provisional backend/memory labels remain false until actual runtime evidence.

## Ten workload source inventory

All ten compact algorithms and ten physical optimized aliases are authored at
`drafts/private-v1/sources/<benchmark>/{compact,optimized}.pl`. Optimized files
are intentionally identical baseline aliases, with the same single-worker
algorithm and no distinct optimization claim. All20 exact source/QLF/profile/
sample identities remain separate; sibling timings are never copied. The source
packet's executable candidates and actual unresolved gates are described in
`drafts/private-v1/REVIEW.md` and `RUNTIME_REVIEW.md`. Every command remains
unexecuted and unapproved.

| Workload | Authored Prolog work | Remaining actual qualification |
| --- | --- | --- |
| [binarytrees](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/binarytrees.html) | Fresh runtime functor before depth branch; both children and recursive checks; actual depth iteration loops and retained long-lived root. | Unchanged scored WAM/native factory, fresh leaf identity, actual iteration/lifetime and concurrent retained/churn graph proof. |
| [pidigits](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/pidigits.html) | Sequential unbounded LFT with both real endpoint divisions and ten-column zero/tail handling. | Exact unbounded runtime/library identity, independent generic arithmetic and both actual operands/quotients. |
| [spectralnorm](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/spectralnorm.html) | Separate coefficient/product/transpose/AtA predicates, ten pairs of AtA and all40 dense products. | Full exact source review, bounded/full outputs and strict numeric/runtime evidence; no stored matrix or foreign kernel. |
| [mandelbrot](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/mandelbrot.html) | Scalar full50-step escape recurrence for all pixels; complete PBM bytes/tails. | Complete bounded/full stdout and source/pixel work review; no symmetry/interior shortcut. |
| [fannkuchredux](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/fannkuchredux.html) | Factoradic prefix rotations, fresh source-filled work term, every actual flip and global-rank checksum. | Permutation-order/count/flip source review, bounded references and complete n12 feasibility/output. |
| [fasta](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/fasta.html) | ALU repeats, float cumulative probabilities, every fresh naive LCG draw, ordered seed continuity and60-column lines. | Direct floating policy/source review plus complete output; no cutoff conversion, RNG cache or generated answer stream. |
| [knucleotide](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/knucleotide.html) | Exact-token THREE reader; seven standard growing hashtables; rolling scalar keys; every window/tail; sorted1/2 outputs and five queries. | Deterministic/backtrackable mutation, actual complete maps/capacities/growth/window totals and full counter resource feasibility. |
| [revcomp](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/revcomp.html) | Buffered CR/LF records, preheader/comment handling, all IUPAC complement decisions and reverse indices with60-column output. | Full original helper/input binding and complete bounded/full byte output; no stat-based whole-file allocation or foreign reversal kernel. |
| [regexredux](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/regexredux.html) | General PCRE2 library plus counted Prolog cleanup, nine separate case-sensitive folds and five ordered replacements; jit(false). | Actual selected package/API/JIT-option/version/native images and complete output; matching library work explicitly disclosed. |
| [nbody](https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/nbody.html) | Fresh source-owned body terms; offset momentum; all ten pair forces/both contributions, all position updates and energy sums. | Actual strict sqrt/arithmetic behavior, trajectory/energy references and complete50M-step timing. |

All twenty application files and both counted support helpers are review
candidates only. They were not parsed, compiled or executed. No profile is
eligible for a canonical source count or chart. Native runtime/QLF behavior,
source work witnesses, canonical original-source ownership and full timing
remain required; identical aliases make no independent optimization claim.

## General library and allocation boundaries

Stable [hashtable.pl](https://raw.githubusercontent.com/SWI-Prolog/swipl/V10.0.2/library/hashtable.pl)
exports `ht_new/1`, `ht_get/3`, `ht_put/3`, `ht_update/4`, `ht_pairs/2` and
`ht_size/2`. It grows a standard closed hash table from a small initial state
and uses ordinary term mutation. All seven K maps must use it directly and
retain every lookup/update. The vendor module remains a pinned general library;
the application's parser, key/window loops, update wrappers and output are
authored counted source. Observe real capacity/growth and reconcile all keys,
counts and complete window totals rather than estimating work from final output.

[functor/3](https://www.swi-prolog.org/pldoc/man?predicate=functor/3)
constructs a new compound skeleton when its first argument is a variable.
The leaf and interior draft use this same factory. The standard
[same_term/2](https://www.swi-prolog.org/pldoc/man?predicate=same_term/2)
distinguishes compound identity, unlike structural equality. The future bounded
observer must retain real roots/children and use actual identity graphs, actual
iteration/worker intervals and long-lived root checks before/after churn. Native
VM/runtime evidence must confirm the scored factory still allocates all leaves.
Allocator address reuse after dead trees is valid; concurrently live churn and
retained graphs must remain disjoint. Default runtime GC policy is retained.

SWI [arithmetic](https://www.swi-prolog.org/pldoc/man?section=arith)
uses GMP-backed unbounded integers when built with that feature. A successful
version call alone is insufficient: later receipts must bind `bounded=false`,
actual arithmetic/foreign images and independent large positive/negative
division cases. Pi uses source-visible real `div` operations on both operands;
native GMP only supplies general number storage/arithmetic.

Standard [readutil](https://raw.githubusercontent.com/SWI-Prolog/swipl/V10.0.2/library/readutil.pl)
may use foreign buffered IO with a Prolog fallback. Actual loaded implementation
and image must be recorded. No custom foreign workload predicate is proposed.
SWI stack limits and GC settings must be explicit runtime provenance, with the
same limits across forms and no GC-disabling or benchmark-specific heap policy.

## Additive qualification pipeline to prepare next

1. Complete source-only compact/optimized packets, explicit main/support input
   inventories and provenance; peer/root review algorithms and worker bounds.
2. Review a metadata-only sealer that acquires the common mutex before any
   runtime discovery, source/tool/package hashes or imports. Preserve every
   draft/failure and exact source/toolchain/SDK/image identity.
3. Only after actual proposal/digest review, run locked toolchain discovery or
   private bootstrap, package/version/feature/ARM64 checks, concrete ownership
   corruption fixtures, QLF preparation and complete existing bounded output
   inventory. Use the original frozen generic sampler as a pinned service,
   with the exact current20 Boolean process gate schema and raw/supervisor
   receipts; no new mutable timing identity or foreground-only launch.
4. Bind all actual compiler-loaded authored modules/directives/includes to
   original bytes, with native runtime/boot/QLF/library artifacts separately.
   Prolog needs its own structural source-owner grammar: clauses, directives,
   conjunction/effects, disjunction/if-then-else, cuts, arithmetic bindings,
   list/data patterns and written callback/meta-call bodies. Do not alias the
   existing Perl `.pl` counter or assign estimated LOC. Every authored helper
   is counted; generated QLF/expanded terms are evidence, not source substitutes.
5. Qualify actual fresh-node/leaf/retention/iteration tree work, all seven real
   growing maps/windows, both Pi divisions, full source reviews and native
   process/sampler ownership. Observers are separate and unscored; pins cover
   original scored VM code and all actual observed bytes. Never replace the
   algorithm with a host witness or count formula.
6. Root reviews the actual qualified work packet and exact later full index,
   then launches90 complete full processes across ten manifests in rotated
   three-sample rounds. Reuse successful immutable exact-source records after
   ordinary failures; integrity/environment/ownership failures stop. Runtime
   limits may reject slow Prolog profiles; no smaller work, extrapolation,
   timeout-as-success or partial publication substitutes for completion.
7. Additive canonical20 overlay, independent90-record reconciliation and actual
   runtime/memory/library metadata precede publication. The current whole-six
   importer/backend contract needs a separately reviewed Prolog extension;
   none of its existing language lists, fixtures, data or counts is modified here.

## Explicit unresolved gates

- Current official Prolog ten-source availability is not established; historical
  sources/contributor attribution must be recovered separately if adapted.
- All10 compact sources and10 physical identical optimized aliases are authored,
  with20 separate candidate source/QLF/profile/sample identities. This is ten
  distinct algorithms, not evidence of20 separately optimized implementations.
- Exact local10.0.2 runtime, archive, selected ARM64 slice, compiled library and
  package versions/features remain undiscovered/unverified under the mutex.
- Tagged stable hash-table/IO sources were read, but installed package binding,
  backtracking/term-copy/thread-return behavior and native numeric semantics
  require actual qualified evidence. Web API docs may describe10.1 development.
- Fresh leaf allocation and retained graphs are a source strategy pending actual
  scored VM/runtime/allocation and observer proof, not an admission claim.
- QLF launch/initialization/source/include identities, all bounded outputs,
  source-owner grammar and full runtime feasibility remain unexecuted.
- Prolog language-origin metadata must be distinguished from SWI's1987
  implementation history; no unverified release year/color/backend is published.

Current reviewable deliverable: completed20 source files, both counted helpers,
exact profiles, a six-method local fixture candidate/frozen-unittest runner,
guarded metadata/QLF/156-bounded preparation, original-source/WAM extraction and
ten-manifest/90-process full framework. These files are unparsed and unexecuted.
Actual runtime discovery and actual tree/map/Pi observers plus canonical Prolog
ownership grammar remain the next source/preparation gates. Root owns every
later concrete digest review and durable queue; no permission question is needed
for continued source authoring.
