# Additional formatted source-line metric

This packet and the separate `overview/explorer/formatted-loc-ui-v1` copies are
source candidates. No formatter, test, parser, hash, compiler or application has
been run by these authors. The live dataset, source files, current UI, canonical
LOC evidence and measured samples are unchanged. Root must review the concrete
packet before fixture qualification and the concrete plan/digests before any
formatter measurement command.

The physical definition is the existing LOC audit's count of nonempty rows
after comments are masked, with source whitespace/newlines retained. Strings
remain code on their nonblank source rows; semantic directives remain code.
This is `foreign_core.count_owned`'s initial physical count and
`lexical.mask_source`'s corresponding mask. The new worker calls only a reviewed
`mask_source` or `mask_foreign`, then counts `row.strip()`; it never invokes
semantic normalization, expands packed statements, or substitutes canonical LOC.
The identifier is `nonblank-noncomment-physical-lines-v1`.

`overlay.py` is a pure transactional adapter. A guarded caller supplies the
saved dataset `{data,path,sha256}`, the actual decoded receipt objects and an
immutable qualification/terminal envelope from `load_evidence.load`. The
adapter preserves original source name/path/code/SHA, measurement identity,
samples, timing boundary and later unrelated LOC/style metadata. It binds the
full `program_inventory` together with selected chart programs and rejects
shared source, measurement or canonical-coordinate disagreement. It commits
only four per-program fields across existing inventory/chart rows and provenance
after every selected row passes:

- `formatted_loc`: a positive integer whole-bundle total or null.
- `formatted_loc_status`: qualified, unavailable, pending or failed.
- `formatted_loc_complete`: true only for a qualified complete bundle.
- `formatted_loc_evidence`: all file, inventory and receipt dispositions.

A file can have zero physical lines. A complete program must have a positive
total. A successfully formatted empty UTF-8 helper contributes zero; a missing
file, symlink, NUL byte or invalid UTF-8 output does not qualify. Every scored
authored main/helper/ABI input is represented once per
profile; shared helpers count again in each bundle using them. Any missing
native formatter, unresolved lexical coverage or failed invocation blanks the
whole-program total. Successful per-file evidence may remain, but contributes
no partial total. V and Red are excluded.

`endpoints.py` consumes the full admitted candidate pool before pruning. It
retains the union of canonical fast/small and formatted fast/small sources.
The full program inventory and its source references survive chart pruning.
Canonical LOC/time baselines and summaries retain their old definition.
The selector requires the saved canonical baselines and coordinates and retains
them exactly; it never recomputes them from the formatter-eligible pool.
Formatted X uses a separate full-qualified-cohort minimum. Original time Y is
preserved. This allows valid formatted counts from other languages when Mech's
formatted total is blank, and displays their raw counts in tooltips/details.
There is no automatic formatted LOC × Mech fallback: this candidate keeps that
extra ratio blank. A later source/receipt-bound control overlay is required
before any nonblank matched-control formatter ratio.

The UI copies add Canonical LOC / Formatted LOC without resetting language,
trait, manual program or release-year choices. Metric-specific domains and
history caches use the admitted full pool, independently of those choices.
Y domain limits use a shared full-cohort envelope of both modes, so a slower
formatted-small source is not clipped when switching metrics. Unqualified
formatted totals are absent from formatted comparisons; the source inspector
retains their original canonical source/audit and explains the blank status.
These copies are not a deployment and still need composition with root's
separate platform frontend before UI execution.

## Guarded formatter-only worker

First proposed fixture-only command after source review and packet freeze:

```sh
REVIEWED_PYTHON -B -u qualify_fixtures.py --revision UNIQUE_FIXTURE_REVISION
```

It runs exactly twenty-three source-bound synthetic methods in a fresh immutable
Python/unittest/source snapshot under the common mutex. It saves loader errors,
actual discovery and start IDs, all raw outcomes and terminal partial pins. No
formatter, compiler, download or application route is present in that command.

Only after actual twenty-three-method receipt/terminal review and a concrete plan:

```sh
REVIEWED_PYTHON -B -u measure.py --plan ACTUAL_PLAN.json --plan-sha256 ACTUAL_SHA --revision UNIQUE_REVISION
```

`measure.py` takes the named common mutex before all hashing/imports and keeps
that same open-file-description through terminal closure. Inherited FD >=64,
held/named inode equality, actual C3 proposal43438603...050 and the unchanged
actual eleven-method ownership receipt precede service code execution. The
separate actual twenty-three-method local receipt/terminal/source closure is a
literal prerequisite; measurements do not rerun these tests. Its
bounded cold source loader executes pinned `.py` bytes instead of cached pyc;
unrecorded authored/vendor imports reject, including arbitrary external paths.
Standard-library imports require the selected Python's actual standard namespace
and resolved library root, with site-packages/dist-packages excluded; their
observed source/native origins are hashed, recorded and closed under the mutex.
Declared vendor inputs require exact source/native image pins. The frozen generic `owned_command`
function code remains unchanged. A local subprocess namespace overrides only
its fixed `cwd` with the isolated file directory; stdin remains DEVNULL and
the exact new session/PGID, common FD, timeout and bounded cleanup remain.

Formatter commands receive isolated original copies, explicit approved argv,
fresh HOME/XDG configuration directories and a recorded C locale environment.
No compiler or scored application runs. Each tool version is queried once per
exact image/version identity; complete raw stdout/stderr/lifecycle receipts and
the observed version binding are retained. Different configuration still has
its own pinned descriptor. Formatter output is frozen after its physical count.
Scored original hashes are reconciled before and after formatting and again at
terminal closure. All toolchain/config/reader dependencies must be in plan pins.

A clean recorded positive formatter exit may produce a failed file only after
its exact command/mutex/final status/owned cleanup and absence of owned group
members are proved. Timeout, signal, cleanup, command identity or source/pin
failures stop the controller. No failed input is retried. Missing native routes
are unavailable or pending with primary evidence/reasons. A completed batch can
contain explicit failed/null profiles; completion is not numeric qualification.

Every original input, copied/output source, config, tool, compiler inventory,
raw command receipt, lexical count receipt and cold-import identity is pinned
under the common mutex. Early/late failure keeps all readable partial bytes and
an immutable stopped or failed-terminal-integrity envelope. The saved
qualification is never rewritten by the integration loader.

## Exact plan contract

An actual root plan has status `root-reviewed-formatted-loc-plan`, false
`chart_admission` and `application_execution_authorized`, and the definition
above. It contains:

- `pins`: exact packet/Python/toolchain/config/qualified lexer dependency SHA
  closure, including the root-reviewed plan's dependencies.
- `source_dataset:{path,sha256}`: admitted data artifact whose program catalogue
  includes the complete `program_inventory` as well as selected chart programs.
- `fixture_qualification:{receipt_path,receipt_sha256,terminal_path,terminal_sha256}`:
  root-reviewed actual complete twenty-three-method source fixture closure.
- `profiles` and `measurement_identities`: exactly matching program-key sets.
  Each profile supplies language, ordered files and an independent actual
  compiler inventory descriptor (`evidence_path`, `evidence_sha256`,
  `binding_field`, `binding`). Its binding has the canonical qualified exact
  authored inputs, source/binary SHA and complete files path/SHA pairs.
- `measurement_evidence` per profile, separate from the compiler descriptor:
  `path`, `sha256`, `binding_field`, `binding` and `fields`. The guarded worker
  and integration loader read the independently pinned actual measurement
  artifact. Its exact JSON-pointer record must equal `binding`; relative field
  pointers select the recorded profile id, benchmark, main-source SHA,
  executable SHA, command, exact samples and `command_image_sha256`. The first
  six fields must match this saved catalogue program and compiler inventory.
  The command-image pointer must select an actual recorded digest from that
  independent receipt; the pinned bytes at `command[0]` must equal it. Native
  records using the existing native importer convention select the same
  `/binary_sha256` field for both executable and command image. An interpreter
  with a separate compiled artifact selects its separately recorded runtime
  image digest; it never substitutes the artifact digest for the runtime.
  A saved `runtime_sha256` must agree with the recorded command image when
  the execution mode is interpreted/JIT/bytecode or the recorded image differs
  from the compiled-artifact digest. Legacy AOT metadata can retain a separate
  compiler distribution digest without confusing it with the launched image.
  No digest is inferred from a current path pin. A status word or a matching
  source digest cannot create this proof.
  Legacy descriptors with only the first six pointers are pending/null until
  recorded command-image evidence is supplied. Missing independent
  measurement/compiler evidence, a saved command or recorded command-image
  evidence leaves every input pending and the total null, without invoking a
  formatter.
  A supplied contradictory or nonindependent record rejects the transaction.
- File records: `published_file`, actual original `path`, `sha256`, source
  `language`, and role application or application-helper.
- `formatters[language]`: qualified reviewed native route, or unavailable /
  pending with primary official sources and a reason. A runnable route supplies
  provider language-project, official_sources, tool path/SHA/version, config
  path/SHA, explicit environment, version_arguments, exact expected version
  stdout/stderr, output_mode in-place/file/stdout, arguments with only
  `{input}` / `{output}` substitutions, and bounded timeout_seconds.
- `counters[language]`: pinned module name/path/SHA, mask_source or mask_foreign,
  and the explicitly reviewed language/lexer argument.
- `code_modules`: exact import-name/path/SHA catalogue, with package roots for
  any qualified Pygments/other lexical dependencies. Complete pin/import closure
  is mandatory, not a version or PATH fallback.

Native means the language/compiler project's own formatter; see FORMATTERS.md
for primary-source candidate facts and unresolved routes. No third-party tool
is silently counted as native, and no absence is inferred solely from PATH.
The plan needs selected-source formatting and lexical ownership review before
actual output admission; this source packet supplies no plan, count or tool
qualification by itself.

Source-authored fixture catalogue: thirteen endpoint methods, six overlay methods,
four formatter boundary methods, plus the separate frontend assertion script.
All actual discovery/start identities and zero failure/skip outcomes must be
recorded by root's later inherited-lock fixture runner. None have run here.

Independent source review covers literal preservation of canonical baselines and
coordinates, actual independent measurement record matching, the missing-record
pending route, full program-inventory source closure, explicit physical
row-boundary preservation and rejection of unrecorded external/vendor imports.
The twenty-three-method catalogue covers inventory-only and shared profiles,
inventory disagreement and excluded rows, and source references after endpoint
pruning. The existing measured-execution method also rejects a newly repinned
native image that differs from its recorded executable digest, a missing or
wrong image pointer, and contradictory actual/saved interpreter image records.
It admits a separately recorded interpreter image while preserving its
different compiled-artifact digest. The whole-bundle method rejects an exact
zero total even when all file count receipts agree. These assertions add no
test method; the exact 23 IDs remain unchanged. The existing boundary method
also rejects a same-length lexical mask
that removes source rows and accepts a zero-line empty helper. These are source
changes, with all actual outcomes still pending.

The Mech repository has its own formatter at
`src/cli/commands/format/mod.rs` (`mech format`, explicit input / `--out`). Its
source path is affirmative native ownership evidence. The current command also
loads stylesheet/shim resources before text output; a Mech formatter plan must
pin explicit local resources and prohibit network/ambient fallback, then prove
the exact executable/output route. No measured Mech binary is assumed to include
that CLI and no Mech formatted number is inferred from formatter source alone.
