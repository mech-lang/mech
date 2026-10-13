# Work and original-source LOC after actual Prolog preparation

This is a concrete source-only next-stage plan, not a runtime/work/LOC
qualification or launch. No import, parser, hash, fixture, compiler, observer,
benchmark, Git or dataset write ran while preparing this note. The twenty
prepared programs, their QLF files, all actual bounded outputs and failed
historical preparations remain unchanged. Numeric source counts and full90
remain pending.

## Exact completed prerequisite

`qualification/prepare-explicit-goal-rest-v1/report.json`, SHA
`cab6ea7f3bd8a237f8f83833ed9b9cc293bbe1878cc8a894d74fdcf41f7ffbff`,
contains all twenty source-bound profiles, nineteen new build profiles,155
new bounded cases and the one separately reused successful compact-regex case.
Its body terminal SHA is
`716ba1503f06629304bba8932569d8418d214794a41d0b0395dab60bff20029d`;
closed durable terminal SHA is
`21d10035fea80c16dd4514f762e8a92f79af806662b2aed19f133c339906842f`.
Root's independent14,611-pin review is
`continuation/jobs/prolog-remaining155-closed-review-v1/receipt.json`, SHA
`139d9124d3df942bb3170112b3841c33b0191580a002f974835a627651be9bc8`.
It records156 distinct logical cases,20 Boolean checks per case, successful
first-three compiler/inspection commands and first-two QLF artifacts reused,
zero successful case reexecution, zero fixtures and zero benchmark runs.
That receipt still has `source_work_qualified=false`, `canonical_loc=null`
and `chart_admission=false`; this plan does not replace those fields.

The unchanged frozen source packet is
`revisions/prepare-success-halt-v1/`, with actual27-file capture
`continuation/jobs/prolog-prepare-source-freeze-v1/packet-pins.json`, SHA
`37076a11022e0cfbb07c86b420866eec4f81f1d722b606e186c41c25dcdc2660`.
All ten mains and both entire helpers were read. Ten source-only diffs confirm
each compact/optimized main pair is identical; aliases remain explicit, with
no new optimizer or copied timing claim. Their recorded source hashes are:

| Benchmark, both forms | Original main SHA256 |
| --- | --- |
| binarytrees | `00b81975683549caaff475bbb7bdc1fc649ef143a852298259605c408f99f625` |
| fannkuchredux | `53ca5622a6e6195a3ea9d5ce7ab3aa593763ddba9bc5e41ac7c537db27d4dfa6` |
| fasta | `b5d0670691272c9305fb71b825848bad4bbb8bac8be1e00d1d0b32a7c3493538` |
| knucleotide | `ba1fdb0abf8ab0be068462bc58946ec0e2668c67db2664e4cf4dfe0412f386d5` |
| mandelbrot | `97e78601578af9c54a73bdc1cfed6ae6a36c90e8438fbc849a935d7f657fe09d` |
| nbody | `d3e082e34d3fad47759021c921b016563f93a1aaec9d65220b34494ac31c3266` |
| pidigits | `70a0f6a01503c6d32e419d783c90676fdf1dbd520a5579b98eec0d8165d05f88` |
| regexredux | `912c4b20515cc8ad418697b1c8c1347b8db29b9efcc49fc162eb82920c2699b1` |
| revcomp | `2460ba65f9231222000980236c6c4ca7b101f6fef3313aa37f125dc171bc59fc` |
| spectralnorm | `1d868f9e772820b1b2e7a3c68e82f142b6704357f6fa1249ef3e828e78c02fda` |

Every profile counts its whole authored main plus `support/clbg_io.pl`
(SHA`bc6361457988de3b8d7dbce526bf738f0268e96e1c7bb59c35bb4798c71ebdb3`).
Both knucleotide and revcomp forms also count `support/clbg_records.pl`
(SHA`efb32920c4c8c42ff5c1d6d3a521b9c2a0fc93d06a6f1e994e00c80d39f61b19`).
This gives22 distinct original authored source paths and44 per-profile
compiler-bound main/helper file references. A future inventory sweep must
verify that exact total from the actual source-bundle/loaded-input records,
including each measured byte-copy path; it is not a newly executed inventory.
The preparation tool is an unscored inventory tool. Generic SWI/GMP/PCRE,
hashtable and other pinned library sources are vendor inputs. QLF and WAM are
compiled evidence, never substitutes for authored source units. The initial
compact regex uses its preserved `prepare-success-halt-v1/builds` tree; the
other nineteen use `prepare-explicit-goal-rest-v1/builds`.

## Concrete source meaning

| Benchmark | Authored full-work behavior read | Remaining evidence |
| --- | --- | --- |
| binarytrees | `functor(Tree,node,2)` precedes the depth test, then constructs both children; every leaf is a compound with two `none` children. All temporary checks traverse both subgraphs; stretch finishes before the long tree, which stays in `main` across `depths/2` and is checked last. | Source-matched scored WAM/native factory review plus actual bounded graph/identity probes. |
| knucleotide | Seven separately created generic growable tables scan every index with widths1,2,3,4,6,12,18, insert every overlapping window and preserve the complete tables for output; only the named five queries use lookup. | Complete-map/capacity/root-identity probes, including tails and keys beyond the query set. |
| pidigits | Each candidate forms `3*Q+R` and `4*Q+R`, performs both integer divisions, tests both estimates, updates the full unbounded recurrence and emits exactly the requested digits with tails. | Actual operands/quotients/recurrence states, accepted and rejected candidates. |
| fannkuchredux | Factorial ranks, prefix rotation/carry, full permutation copy, every reversal and global rank parity checksum/max are authored Prolog. | Existing bounded outputs plus later canonical full output and source ownership. |
| fasta | Exact float CDF accumulation, seed42 LCG draw for each random byte, authored linear selection,60-byte wrapping and seed continuity from TWO into THREE. | Existing bounded tails plus later canonical full byte/hash output and source ownership. |
| spectralnorm | Explicit coefficient division and every dense row/column term; ten rounds each apply A/transpose twice, totaling40 matrix operations; actual dot products produce final norm. | Existing bounded outputs plus later canonical full precise output and source ownership. |
| mandelbrot | Every real pixel performs up to50 strict float recurrence iterations, stopping on escape; eight-bit groups zero-fill tail lanes and output full PBM rows. | Existing odd/tail bounded outputs plus later canonical full PBM and source ownership. |
| nbody | Five canonical bodies; each step applies all ten pairs to both velocities, followed by five position updates; exact constants and energy loops remain authored. | Existing bounded energy outputs plus later full energy pair and source ownership. |
| revcomp | Authored record handling skips preheader/comments, keeps record order, reverses every sequence code with complete case-insensitive IUPAC table, then wraps60-byte output. | Existing EOF/CRLF/IUPAC/tail outputs plus later canonical full bytes and source ownership. |
| regexredux | Nine case-sensitive match folds and all five ordered global replacements are called from authored Prolog, with `jit(false)`; generic PCRE supplies matching. The authored fold callback increments the count. | Existing replacement/boundary outputs plus later canonical full output and selected library callback source ownership. |

Source-only reading does not turn these into actual execution receipts. The
actual156 bounded cases are reusable output/ownership evidence; the later
full90 remains gated separately.

The actual compact tree `original-scored.wam` contains `i_call(functor/3)`
before `c_fastcond` and the `Depth>0` branch, both recursive constructor paths,
and the two leaf bindings. In `main/0`, `b_firstvar(3)` supplies the long tree,
`depths/2` executes, then `b_var(3)` supplies that exact live slot to the final
`check/2`. This is positive original VM-code evidence; no new disassembly ran.
Pinned official SWI10.0.2 `src/pl-prims.c` functor/3 delegates a bindable term
with arity2 to `PL_unify_functor`; `src/pl-fli.c` constructs three fresh global
stack cells, advances `gTop`, initializes both arguments and binds the result.
Compound type/functor descriptors and the `none` sentinel are shared generic
metadata, not shared live Node graphs. Stack relocation/GC and released-storage
reuse are valid. The two original scored Pi `div` operations are also present
as separate `a_func2((div)/2)` instructions. The exact selected hashtable source
creates `ht(0,0,[](_))`, grows buckets by powers of two, and retains all inserted
key/value entries using ordinary table mutation. Final actual factory/work
review must bind these source/WAM/runtime paths and original recorded pins.

## Smallest next executable work route

Reuse existing `revisions/work-v1/{instrument_sources.py,observer.pl,
work_proofs.py,test_work_proofs.py,fixture_runner.py}`. Read mechanism is
source-preserving: exact unique text insertions reverse to the actual original
main; creator/check/window/division bodies remain unchanged. Both actual QLF
artifacts and scored WAM remain authority; only separate instrumented copies
may compile. The six existing meaningful source-hook/graph/map/Pi fixture
methods have never run; one actual closed six-method receipt is needed before
the independent work validator can admit native observations. This is separate
from the six already passed preparation contracts and eleven ownership tests,
which must be reused without rerunning.

The existing thirty proposed diagnostic cases are the concrete work catalogue:
trees n0,4,7,9 per form (8), Pi n0,1,9,10,11,27,97 per form (14), and four
K-inputs per form (8): missing/empty THREE, one base, an overlapping-tail case,
and a recorded input covering arbitrary width18 keys beyond the five queries.
Final payload bytes and expected sequence must be fixed in the reviewed
controller before a run. They are new work observations, not repeats of the156
bounded preparation cases. Root may select a smaller meaningful catalogue
before freezing; no process is authorized by this note.

Use the existing actual preparation cold-loader/service and native ownership
interfaces, with an explicit `-g clbg_io:run(MODULE:main)` before numeric `--`.
Bind original source/QLF/WAM/runtime/loaded-library/execution fingerprints
before creating instrumented copies, and archive the exact insertion map,
observer source/QLF, native stdout/stderr/trace and original output oracle.
Use the unchanged `same_term/2` live graph/root checks; do not substitute
structural equality or copied `nb_setval` objects. Retain only the already
long-lived root, release temporary snapshots at callback return, and validate
all original iteration intervals and retained identities. K witnesses inspect
all seven complete actual maps and their unchanged returned roots, capacities
and overlapping-window totals. They explicitly do not claim per-window
instrumentation; source-loop meaning and original WAM remain additional gates.
Pi witnesses carry actual original operand/quotient values into the independent
unbounded recurrence oracle. Host validators are untimed evidence consumers,
never scored benchmark kernels.

## Original-source LOC route and two precise repairs

The selected never-executed owner mechanism is
`loc-normalization/20261013/prolog-proposal-v2/`; its31 method IDs and policy
already retain clause/directive headers, independent conjunction effects,
arithmetic/unification/cuts, both control arms, explicit executable closes,
literal continuation rows and selected meta callback/reference ownership.
Unknown operators, expansion, hidden meta targets or unqualified helper paths
stay diagnosed with null totals. Complete AST/character-to-byte/owner spans
must be retained and independently reviewed for every authored input.

Two narrow additive fixes must precede those unrun31 fixtures:

1. `scan_source.pl` currently ends `checked` with `nl,halt(0)` inside `catch`.
   Move only success `halt(0)` after the caught `checked`, preserving the
   original reader, JSON packet and failure exit1. The already diagnosed
   SWI uncatchable-halt recovery behavior otherwise produces complete JSON
   followed by exit1. Preserve the old unexecuted v1/v2 sources.
2. `run_fixtures.py` currently binds failed `runtime-v2/bootstrap_runtime_v2.py`
   and its historical status/mount schema. Adapt only this prerequisite block
   and actual service loader to the successful runtime configuration
   `runtime-qualified/success-halt-v1/runtime-config.json`, SHA
   `7fa470a9827e60e84eb9c6ad343cc8385e411b400f0ae8a322fbefda488bb448`,
   its source/library review SHA
   `eab625231fd424e78de80132566ca3c7fe1bb91b096b50874c0a515271bd6486`,
   and closed runtime/preparation root receipts. Require the exact current
   private ARM64 image/home/flag/source/plugin pins; no old failed receipt is
   promoted, and no image/version/bootstrap/seal/preparation is repeated.

The projector and31 original fixture bodies/IDs remain unchanged. Root must
review the concrete additive reader/runner source before freezing/executing.
After actual31 pass, replay only the twenty original main files plus both
whole helpers, bind each of the44 actual measured source copies through its
compiler inventory, and review all owner/event/callback/source spans. Numeric
counts must be whole per-profile sums from qualified originals; source aliases,
generic libraries, generated QLF/WAM and observer/tool inputs retain distinct
roles. No physical-line or generated-code fallback admits a null profile.

All fixture/parser/metadata hashing/observer/native work waits on the common
mutex and remains serialized, with exact private owner closure, bounded owned
groups and partial failure evidence. Root source acceptance, actual captured
digests and separate concrete command release precede each stage. No full
benchmark process or chart mutation is authorized here. The next source task
is the small additive31 scanner/runner repair and direct guarded work controller;
after their actual evidence closes, an additive work/LOC sealer can admit the
twenty prepared profiles and release the unchanged full timing route.
