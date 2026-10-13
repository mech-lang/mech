# Additive legacy LOC correction proposal

This corrects proven discrepancies between the explorer's global normalization
claim and its old physical counts. Measured application source bytes, runtime
configuration, samples, reports, active manifests and earlier audits remain
unchanged. This proposal is not admitted chart data.

## Candidate universe

Reconstruct the complete eight-workload native/scripting pool from
`retime/results/apple-m3-ac-normal-20261008.json`. Apply the same-id measured
overrides from `fast-scripting/results/apple-m3-ac-normal-20261009.json` and
`runtime-policies.json`. Add complete regex-redux and pi-digits reports and the
signed-division confirmation. Apply the selected nbody confirmations and latest
replication, the accepted floating-CDF Rust FASTA adaptation and the measured
two-extract Node pi correction. Preserve current required-work exclusions and
retire aliases of superseded native nbody records. Keep all admitted candidates
before selecting fastest and fewest LOC; the current explorer contains only
endpoints and cannot supply this complete pool by itself.

Include pinned unmeasured sources from all ten native manifests, the compact
manifest and new Lua/LuaJIT pi manifests as unscored pending records. Distinguish
unsupported/work-excluded sources from candidates needing qualification and at
least three full samples. A newly smaller pending source does not receive an
invented time or chart position. Existing explicitly normalized application
counts are retained and reconciled; no expansion is applied twice.
Producer-reported LOC for the exact normalized source is distinct from its
upstream original physical LOC (Lua62/77/63/57 versus45/57/46/42).

## Approved canonical rule; source-specific parses still require review

Preserve nonblank, noncomment producer physical LOC separately. Canonical LOC
counts each written executable control/function header as one unit, its opening
delimiter as zero additional units, each independent body statement/binding
separately, and each written executable closing delimiter/end as one unit.
An inline Lua `if … then body end` and its expanded spelling therefore agree;
` } else { ` contributes one close and one else header. A standalone executable
opening brace adds nothing, so canonical LOC can be lower than physical LOC.
Retain semantic preprocessor directives and the existing wrapping of literal
data. Expression-only function/composition forms stay one expression. Preserve newlines and character
offsets so every increment identifies its original line and source span.
Julia nested `#= … =#` comments are recognized outside literals, including
adjoint-apostrophe safety. String contents do not contain code separators.

Split executable semicolon sequences first. Within each segment, count separate
assignment targets, including nested/chained assignments, and independent
declaration bindings. Thus `a=1; f(); b=c=2;` has four units, not three.
`int a=b=0,c;` has three bindings. Independently declared identifiers—including
uninitialized comma declarations—receive the same treatment as Lua's separately
expanded uninitialized locals; those increments are labelled explicitly.

Explicit tuple/destructuring patterns remain one assignment. Lua/Go bare
multiple bindings with separate, equal-arity initializers count individually;
a single multiple-return producer stays one destructuring assignment. Arguments,
keyword/default parameters, loop-header semicolons and Rust repeated-array or
matrix-row separators are structural. Literal/record construction is data, not
several independent executable assignments.

Recognized packed inline executable headers/bodies/closures use this canonical
rule. Unknown forms, ambiguous expression callbacks, parenthesized sequencing
and executable macros require source-keyed review. An original Lua source is
reconciled with its formatting-only expanded copy rather than receiving a
packing discount. Old Mech `=(outputs):=` and multiline `=>type:=` block headers
are one definition; the final function-terminating dot counts as a written
close, including `count := flips +1.`. Member and fractional-decimal dots do
not close a block; inline `=>type:=expression` has no extra close. Lexer
errors, unbalanced delimiters, unresolved source identities and disagreements
with existing normalized audits also block certification.

## Execution and admission

`audit.py` and `test_normalizer.py` acquire `/tmp/mech-clbg-timing.lock` exclusively
and nonblockingly before source hashing, lexing, fixtures or pool sweeps. They
exit if timing is active. No benchmark process is started. A fresh audit file
records source hashes, original/physical/proposed normalized counts, exact
line-span expansion evidence, parser diagnostics, report/tool pins, complete
candidate mapping, proposed endpoint changes and pending prospective winners.

`queue_audit.py` freezes this proposal's small Python modules into a fresh job
directory and starts one durable waiter. That waiter acquires the same mutex,
runs the normalizer, admission and paired-metadata fixture suites, then produces the complete-pool candidate artifact
within the same held boundary. Any fixture or audit exception stops the job;
there is no automatic retry. Its launch receipt, state and logs remain available.
No source hashing, lexical scan or fixture is executed during queue setup.

Current Mech execution modes and stored paired comparisons are also bound to
their exact source hashes. Historical Mech controls are never replaced by
current endpoints. Display LOC/LOC ratios use reviewed numerator/denominator
counts; producer comparison fields, source counts and all times remain archived.
V is report-only and is never included in this chart pool.

The separate `overview/explorer/legacy_loc_data.py` importer has no builder hook.
It rejects candidate artifacts. Admission requires an explicitly reviewed
complete-pool audit with source-keyed decisions, reconciled existing audits and
no unresolved prospective compact winner needing full measurements. Derived
baselines, ratios and means are recalculated only after that review. Samples and
application source bytes are verified and preserved. Root receives the concrete
candidate mapping/rule/pending-winner list before any chart admission.

## Proven starting discrepancies

* Lua nbody currently107: six independent local declarations require14 extra
  binding units and its inline loop/header/close adds2 more: proposed123.
* Julia spectral norm currently40: proper block-comment removal gives36
  physical lines and two chained-binding increments give38.
* C mandelbrot currently43: chained/separate assignments require5 extra units;
  independent comma declarations contribute10 further binding increments under
  the explicit proposed declaration rule.

The focused fixtures cover these sources, mixed statements/chains, declaration
groups, named arguments, matrix/array separators, nested Julia comments,
destructuring, loop headers, offsets and preservation of standalone closing
lines. Full source-pool review remains necessary; passing fixtures alone does
not certify an arbitrary program.

## Executed receipts and current review status

`jobs/boundary-audit-01` acquired the common mutex on 2026-10-12 and ran 37
fixtures: 35 passed, two failed (EOF comment masking and Julia bang-function
call context). It stopped before the audit. Its code snapshot/logs are immutable.

After narrow repairs, `jobs/boundary-audit-02` ran 40 fixtures successfully and
completed an actual mutex-held pool audit at 13:53:43 UTC: 539 measured
candidates, 592 source identities, 140 benchmark/language groups, complete
saved measured legacy-pool coverage. Its 1,680 conservative diagnostics require
review; several are proven recognizer false positives, not program errors.
There were 14 proposed endpoint changes and 36 prospective pending compact
candidates, 33 still requiring parser review. These are candidate findings,
not approved chart data or invented measurement results.

`jobs/boundary-audit-03` was one frozen fixtures-to-audit waiter (PID 55754,
queued 14:19:27 UTC). It acquired the mutex and stopped at 14:26:51 UTC before
fixture execution or source-pool audit: the default Python launch omitted the
existing Pygments dependency path. Its failed receipt is immutable. A subsequent
launch pins the explicit Python executable/version and existing dependency root
(`/tmp/mech-clbg-compact-libs`, Pygments 2.19.2), checks them under the mutex and
records the executed environment. No dependency installation is needed.
Working-file edits made after queueing never alter a frozen snapshot.
The metadata fixtures in the third snapshot
bind previously unresolved paired LOC denominators to the producer's immutable
historical explorer snapshots; equal numeric counts never substitute for
source identity.

`jobs/boundary-audit-04` completed at 14:52:04 UTC: all 82 fixture methods
passed, followed by the actual 539-candidate/140-group legacy audit. Pinned
Python 3.14.8/Pygments 2.19.2 execution is recorded. Resolving all paired
denominators also exposed 219 additional existing foreign-language numerators
for auxiliary policy review; the 811 source identities and 672 diagnostics do
not certify those languages. The complete measured-pool coverage claim applies
to the original fourteen-language pool only. There are 27 pending candidates,
three with parser-pass compact proposals requiring qualification/three full
samples: PHP fannkuch-redux #8 (71), interpreted Lua Mandelbrot #2 (40), and
Node spectral norm #7 (45). PHP #8's seven-way chained assignment already
contributes seven units with six exact expansion lines, not one tuple unit.

The immutable fifth job (PID 64003, queued 15:08:18 UTC) terminated at
15:23:04 UTC after 107 fixture methods with 11 failures and no errors. It
stopped before the pool audit. Seven expectations predated the approved Perl
modifier header/body rule; one incorrectly required two lexer repairs when a
single repair retokenized both division operators. Three actual edge cases
were Go anonymous-type constructor braces, Julia's masked `:static` loop macro
option and Perl postfix dereference leaving an old lexer state active for the
following semicolon. The frozen fifth receipt and snapshot remain unchanged.

The next justified frozen pass repairs those exact cases, retains the original
operator bytes and validates source-equivalent constructor/block cases. It
also includes the approved prior-array source/provenance reconciliation,
Chapel named-argument classification and independent uninitialized Perl/PHP
declarations. It does not admit Uiua/APL counts or chart data. There is no V or
candidate workload qualification/timing launch in this job. Root owns the
three prospective compact candidate timing studies with LOC explicitly pending.

The sixth frozen job (PID 68989, queued 15:36:29 UTC) terminated at
15:45:06 UTC after 115 methods, with 114 passed and one failed. It stopped
before the pool audit. The added Go newline contrast found that regular
expression `$` accepts a position before a final newline: an anonymous type
declaration followed by a separate block was still classified as a constructor.
The working correction uses the exact `\Z` anchor. The constructor fixture is
retained. All fifth-pass lexical fixes and corresponding tests passed.

Source review found additional previously undiagnosed semantic ownership gaps:
Rust multiline named-function return signatures can leave their body classified
as data or their signature counted twice; Rust scalar value-only `if` and tail
`match` forms are composed expressions; Julia value-only conditional function
tails are expressions, while loops and blocks containing independent bindings
remain expanded. Those forms require source-specific review before counts are
admitted, regardless of the earlier `candidate-parser-pass` label.

The auxiliary foreign-source review now has separate grammar evidence in
`foreign-braced-grammar-review.md` and `foreign-keyword-grammar-review.md`.
It identifies both missed independent declarations and prior false expansions
inside parameter lists or switch arms. These receipts contain source pins and
line evidence, not new admitted totals or general parser certification.

Remaining exact-source review includes Ruby value blocks/modifiers, modern
Perl lexical constructs and source-audited array-language preservation. Existing
Uiua normalized audits can only be preserved for their exact qualified pins;
there is no blanket Uiua parser claim. Existing APL audits excluded standalone
closing `∇`, so those sources need written-close reconciliation under the
approved policy. V's separate report-only parser remains unqualified and is
never collected into the chart pool. Pony preparation research is recorded in
`PONY-RESEARCH.md`; it does not qualify compiler/source/work/timing or LOC.

The seventh immutable full-pool job completed at 15:56:34 UTC with 117/117
fixture methods passing, then the actual 539-candidate/140-group audit. Its
811 exact source identities still carry 569 review diagnostics, including the
219 current foreign registrations whose grammars were not yet implemented.
Four parser-pass pending compact candidates remain unscored: Ruby binary trees
#2 ties the measured #1 at 30, PHP fannkuch-redux #8 proposes 71, Lua Mandelbrot
#2 proposes 40, and Node spectral norm #7 proposes 45. Source review remains
required even for a parser-pass count. No chart was admitted.

The report-only V parser passed all 17 frozen fixture methods in
`jobs/vlang-fixtures-02` at 16:36:27 UTC. V remains outside collectors.
`jobs/braced-value-fixtures-01` stopped after 147 methods with two failures:
a stale Rust callback suffix expectation and an actual C# abstract-method
prototype ownership gap. No audit ran. All ten Zig/Odin methods and all eighteen
composed-value classifier methods passed in that same frozen job.

The additive working audit now has explicit proposed grammar pathways for the
current foreign families, and recognizes proven composed values before
canonical executable splitting. These are proposals until their frozen fixture
execution and exact-source review complete. The exact six-source APL written
closer approval is separate from generic APL support and preserves every prior
producer/source/manifest pin. The new scope correction receipt records the
approved zero-unit declaration markers, superseding the temporary Elm let
header decision without changing prior approval history.

`jobs/current-grammar-fixtures-01` was queued once at 16:49:05 UTC, PID 83083,
for seven narrow fixture suites under the common timing mutex. It freezes the
C# prototype repair, integrated composed-value counts, keyword grammars and
F#/Roc/Elm layout proposals. It executes no source-pool audit or benchmark.
A later justified full-pool job will include every frozen dependency, approval
artifact and qualified Smalltalk pathway, and will stop on any fixture failure.

The current grammar fixture job actually terminated at 17:12:45 UTC with
211 methods, 207 passed and four failures, no errors. The retained receipt
records Python/Pygments and every frozen module. All failures are in the new
layout proposal: missing Elm infix gate, a F# formal-record false builder gate,
F# iteration `in`/`do` ownership, and double-counted Roc record-field separators.
These exact ownership bugs are being repaired without changing the approved
expectations. The prototype repair, keyword grammars, Zig/Odin and integrated
composed-value fixtures passed. No source-pool audit or chart admission ran.

Further bounded static source review identified real old declaration gaps,
recorded in `c-typedef-source-review.json/.md` and `record_fields.py` source
receipts. The C review covers all twenty-five flagged registrations at fourteen
sites; its additions are scope-only, not complete source totals. Rust nbody #1
writes seven named fields across three physical rows, and Rust pi #1 packs two
enum variants. New helpers preserve explicit typedef/import lifetime ownership,
first alias/header suffix rules, independent named field/enum declarations and
unknown-type gates. Their fixtures and integrated checks remain unexecuted
until the next justified immutable whole-pool job under the common mutex.

`jobs/boundary-audit-08` stopped at 17:24:50 UTC after 361 fixture methods,
one Java callback-scope failure and 27 instances of one missing layout-helper
argument. No audit ran. The exact two implementation paths were repaired while
retaining the approved expectations. `jobs/boundary-audit-09` then completed at
17:48:17 UTC: all 361 methods passed, followed by the 539 measured candidates,
811 source identities and 140 original-language groups. The 1,239 explicit
diagnostics include newly implemented current-foreign grammar owners; those
counts remain source-review proposals. There are four parser-pass prospective
minima and thirteen review-first prospective rows. No chart was admitted.

`jobs/future-minima-fixtures-01` was queued once after job09 completed, PID 1800,
for six narrow suites: future lexical masks, Nim, Crystal, Pony, independent
pending-minimum full-record qualification, and additive pool reconciliation.
Its proposed 129 methods run only after acquiring the common timing mutex.
It executes no benchmark or source-pool audit and stops on failure. Future
grammars and new study adapters remain unhooked pending the actual receipt and
exact qualified-source review. Prepared producer LOC remains null.

The next working whole-pool audit freezes the preexisting isolated-study and
Mandelbrot-fingerprint dependencies beside its own parser snapshot. It refuses
live explorer imports and records used dependency hashes. This reproducibility
repair does not alter the immutable earlier jobs or any producer experiment.

The future/minima fixture job actually completed at 17:59:11 UTC: 129 methods,
127 passing and two failures, no errors, no source-pool audit. All twenty mask,
twenty-five Nim, twenty-seven Pony, sixteen study-record and eight pool-merge
methods passed. Crystal's postfix-control header incorrectly included an
enclosing callback close, and a malformed property brace was incorrectly owned
as a callback. The working repair retains those expectations and adds explicit
receiver-method and packed/expanded close contrasts. The next immutable pass
must qualify that repair before Crystal source counts are considered.

The working whole-pool integration now supplies the independently tested
pending-minimum adapter to the additive candidate collector, retaining missing
or incomplete studies as pending. Completed studies still require exact
manifest, source/runtime/binary, saved native record and whole-output identity
reconciliation under the common mutex. Fresh paired Mech controls are also
normalized by their own exact source SHA; their raw null LOC and sample vectors
stay archived. The unhooked explorer overlay adds a separately named
`canonical_loc` to reviewed fresh-control metadata and cannot borrow another
source's count. It does not relabel any earlier Mech timing control.

Further source review proposals include written GNU compiler annotation units,
standard integer import contracts, scalar C#/Dart string interpolation owners,
and the current foreign layout/keyword/Smalltalk/Zig/Odin repairs. The scalar
interpolation receipt covers thirteen exact source pins and only those lexical
owners, not whole-source certification. The C/C++ job09 review covers seventy-
nine sites across thirty-four measured source identities and distinguishes
scope-only packed-declaration increments from unresolved full-source counts.
No newer source-pool execution or chart admission is claimed by these drafts.

`jobs/boundary-audit-10` completed its fixture phase at 18:38:42 UTC with
572 methods, 569 passing, three failures and no errors. It stopped before
the source-pool audit and actual completed-study reconciliation. The repaired
Crystal suite and all future lexical/Nim/Pony suites passed; the scoped C++
named-type, compiler annotation, scalar interpolation, standard type/shadow,
pending-minimum and fresh-control overlay fixtures also passed. The three
remaining bugs are exact ownership overlaps in expanded F# sequence headers
and Zig explicit else arms, plus Odin named arguments incorrectly affecting
single-call producer proof. Those expectations remain fixed while the working
implementation paths are repaired.

The next worker retains a separate mutex-held executed fixture receipt on
either outcome, including interpreter/Pygments and all frozen dependency
hashes, even when fixture failure stops before the normal audit tool-pin
section. This does not rewrite the earlier failed or successful job evidence.

`jobs/boundary-audit-11` completed at 18:47:33 UTC, PID9370. All572 fixture
methods passed under the common timing mutex, followed by the actual complete
source-pool audit:543 measured candidates,811 source identities, coverage
complete,969 explicit diagnostics awaiting exact-source review. The fixture
receipt retains the executed Python3.14.8/Pygments2.19.2 identity and every
frozen module/artifact hash. This pass qualified the three ownership repairs
without changing their expected counts.

The four new prospective-minimum studies independently reconciled all actual
ordered full records against their pinned outputs, sources and runtimes. They
are now measured source-pool candidates; null producer LOC and every timing
record remain archived. Their fresh exact-source Mech controls propose trees30,
Fann69, Mandel17 and spectral12 canonical units, with zero source/timing
relabels. The remaining thirteen prospective entries are all parser-review
cases, rather than qualified parser-pass candidates awaiting measurement.
No chart or global normalized-count claim was admitted.

Further working review now targets the remaining current foreign diagnoses.
Continued independently typed declarations and Fortran BIND(C) type headers
have narrow ownership proposals in `keyword-continuation-job11-review.md`;
new fixtures protect formal/call/data groups and unknown attribute syntax.
Current C#/Dart and interpolation/callback paths are being reviewed separately.
Nim/Crystal/Pony future pathways remain unhooked until their exact qualified
source snapshots and source-specific count decisions reconcile. V remains a
separate report-only pathway and is never part of chart candidate collection.

`jobs/boundary-audit-12` stopped at19:09:50 UTC, PID11847, after611 fixture
methods:605 passed, six failures, no errors. No source-pool audit ran and no
counts were admitted. All Roc/Smalltalk changes and the other new keyword
fixtures passed. Five braced failures exposed callback suffix double counting,
unresolved nested generic type/call ownership and multiline call continuation
gates. Their expected counts remain fixed while the implementation paths are
repaired. The sixth fixture incorrectly depended on Pygments returning Error
for every invalid Ada based literal. The semantic requirement stays the same:
unknown/invalid numeric syntax must gate. A new explicit delimiter/literal
grammar diagnostic replaces that lexer-behavior assumption. The executed
fixture receipt preserves every frozen code/decision artifact and dependency
pin on this failure path. There is no automatic retry.

`jobs/boundary-audit-13` completed at19:12:19 UTC, PID12018. All611 methods
passed under the common timing mutex, followed by543 measured candidates and
811 source identities, coverage complete. The explicit diagnostic count fell
to787; current displayed source registrations with unresolved owners fell from
110 in job11 to73 of292. Every source and raw producer/time record is preserved.
The four actual full candidate studies remain independently reconciled and all
thirteen prospective entries remain review-first. No chart admission ran.

Among distinct current grammar source identities, parser-pass proposals now
cover17/20 C#,17/18 Dart,15/15 Odin,11/17 Ada,12/15 Pascal,15/16 Fortran and
6/11 Roc. The remaining exact forms include true callback/argument chains,
one scalar-assignment arrow method, three wrapped extern signatures, Ada
generic/representation/extended-return/limited-record/statement-label syntax,
Pascal conditional import ownership, Fortran access semantics, and the earlier
Zig/layout/Smalltalk expression or callback owners. These statuses are parser
proposals, not proof of whole-source review or language-wide normalization.

`jobs/boundary-audit-14` completed at19:21:24 UTC, PID12784:618/618 methods
passed, then543 measured candidates,811 source identities,773 explicit
diagnostics and68 unresolved current registrations. Exact C# source recounts
confirmed FASTA145 (one written assignment body plus header) and regex114
(wrapped extern signatures owned as declarations, attributes separate), in
both JIT/AOT aliases. Ada Mandel162 now has no parser diagnostics. No admission.

Exact source followup corrected the draft description of two Ada record sites:
they are derived `new Parent with record` headers. The guarded additive owner
counts the parent/discriminants as header structure, the explicitly written
fields independently and the written record close, without inherited units.
`jobs/boundary-audit-15` completed at19:25:41 UTC, PID12980:619/619 methods,
543 measured candidates,811 source identities,769 diagnostics and67 unresolved
current registrations of292. Ada revcomp357 now has no parser diagnostics;
binarytrees221 retains its two genuine extended-return ownership gates.
Four actual minimum studies and thirteen review-first prospective entries stay
reconciled; every raw source/time/producer record remains preserved. No chart,
new future-language cohort or report-only V source was admitted.

The next additive pass freezes the approved C++ written member-initializer
counting and Fortran PRIVATE declaration clarification, bounded Zig scalar-if
composition proof, and the explicit unqualified C++ new-Type operand grammar.
Its proposed suite is668 methods: prior619 plus16 Zig,24 constructor helper,
seven constructor integration, one source-type operand, and one PRIVATE case.
The constructor helper preserves body work and gates wrapped initializer
data/unknown ownership. All new policy and static source reviews are copied
into the immutable snapshot. No new fixture or source result is claimed before
the worker acquires the common timing lock and completes its actual pass.

`jobs/boundary-audit-16` actually stopped at19:54:46UTC after668 methods:
666 passed, two failed, zero errors. No source-pool audit began. The immutable
failure evidence identified a constructor immediately after an access-label
colon and a negative fixture over-specifying the precise retained ownership
gate. The additive repair note documents both, plus bounded source-proven
access/call type-shadow grammar and three new methods. A future qualified pass
must preserve this failure rather than relabel it as successful validation.

The source-reviewed repair is frozen once as `jobs/boundary-audit-17`,
PID23326, queued20:04:55UTC for672 methods followed by the whole-pool audit.
It waits on the common timing mutex and stops on any failure; the prior job
and every measured benchmark record remain immutable. This queue entry is not
an executed result or chart admission.

`jobs/boundary-audit-17` actually stopped at20:14:40UTC:672 methods,671 passed,
one failure, zero errors, no source audit. Its single failure was a new fixture
wrongly imputing global shadow lifetime to a prototype parameter. The additive
scope correction preserves the parser/rule and distinguishes an ordinary
global function name, a definition's body-local formal, and a prototype's
non-global formal. Prior frozen fixture/result bytes remain unchanged.

The corrected scope fixture is frozen once in `jobs/boundary-audit-18`,
PID28623, queued20:18:32UTC for the same672 methods and a stop-on-failure
whole-pool audit at the common timing boundary. No parser rule changed for
this rerun, and no source result is claimed from queueing it.
