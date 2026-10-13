# Official formatter candidates and endpoint contract

This is a source-only proposal. No formatter invocation, output count, fixture
execution or tool identity qualification is claimed by this document. Tagged
upstream source establishes candidate ownership and interfaces; a local path or
an upstream version string does not establish an admitted formatter image.

The formatted overlay is additional to canonical LOC. It consumes the exact
already admitted application source bundle, including counted support files.
Its worker must record the selected formatter, exact tool and module identities,
explicit configuration, input and output file receipts and terminal result for
every counted file. Generated compiler output, reconstructed bytecode and runtime
or vendor source cannot replace an admitted application source. Canonical source
bytes, canonical LOC and measured seconds remain unchanged.

## Candidates supported by official project source

| Counted source | Candidate and version route | Proposed interface and unresolved qualification |
| --- | --- | --- |
| Nim | The Nim project ships `nimpretty`; the [Nim 2.2.12 source](https://raw.githubusercontent.com/Nim-lang/Nim/v2.2.12/nimpretty/nimpretty.nim) identifies it as the standard pretty printer. | Use an isolated copy with explicit indentation and line length, or its documented `--stdin` interface after qualification. Its tool version is `0.2`, so that version alone cannot bind the compiler distribution. Pin the actual image and Nim 2.2.12 origin. No output is qualified here. |
| Crystal | `crystal tool format` belongs to the compiler project; the [Crystal 1.21.1 formatter](https://raw.githubusercontent.com/crystal-lang/crystal/1.21.1/src/compiler/crystal/tools/formatter.cr) parses the original source and handles comments. The [official command reference](https://crystal-lang.org/reference/1.14/man/crystal/index.html) describes the CLI. | Bind the exact qualified compiler image and format isolated file copies. Record arguments and every resulting file. Do not infer identity from the wrapper's location or the command name. |
| Gleam | `gleam format` is implemented by the official compiler; the [Gleam 1.19.0 CLI source](https://raw.githubusercontent.com/gleam-lang/gleam/v1.19.0/compiler-cli/src/format.rs) supplies the tagged version route. The [official CLI reference](https://gleam.run/documentation/command-line-reference/) documents the command. | Bind the actual 1.19.0 image, select explicit copies or the reviewed stdin route, and capture output and terminal status. Count authored `.gleam` sources, rather than generated Erlang. Authored `.erl` support requires its own route. |
| C support | LLVM supplies `clang-format`; the [LLVM 22.1.8 command source](https://raw.githubusercontent.com/llvm/llvm-project/llvmorg-22.1.8/clang/tools/clang-format/ClangFormat.cpp) and [official documentation](https://clang.llvm.org/docs/ClangFormat.html) establish project ownership. | Bind the actual formatter image independently of the C compiler image. Use an explicit style, for example `--style=LLVM --sort-includes=false`, rather than ambient configuration. Include sorting must remain disabled for the Rebol extension's required `reb-host.h` then `host-lib.h` ordering. This route remains unqualified. |
| Authored Erlang support | OTP provides [`erl_prettypr:format/2` in OTP 29.1.1](https://raw.githubusercontent.com/erlang/otp/OTP-29.1.1/lib/syntax_tools/src/erl_prettypr.erl). This is an official abstract syntax tree pretty printer, rather than an already qualified full-source CLI. | A source reader and comment attachment driver still needs review and qualification. Pin OTP and all driver modules, preserve original source forms and comments, and set explicit paper/ribbon values. The tagged source's ribbon constant and its documentation disagree; do not choose a default from documentation alone. No BEAM reconstruction or `erl_tidy` refactoring route is accepted here. |

These are candidates, not receipts. Candidate discovery does not set
`formatted_loc_status` to `qualified` or provide a numeric count. Every counted
file must have an admitted formatter route before a bundle can become complete.
A successful main-file formatter does not complete a mixed-language bundle with
unformatted support files.

Before any formatter runs, the measured profile's independently pinned actual
receipt must bind its source, executable or compiled artifact, command and
samples. Its `command_image_sha256` pointer selects the actual recorded digest
of the image launched at `command[0]`, and the pin for that path must equal the
recorded digest. Native rows can select their existing recorded binary SHA;
interpreter rows can select a separately recorded runtime image SHA while
retaining a different compiled-artifact SHA. A current image pin by itself
supplies no recorded measurement identity. Missing image evidence leaves every
authored input pending and the complete count null before formatting.

## Unverified full-source routes

| Language and measured version route | Current source review finding |
| --- | --- |
| Pony 0.74.0 | An official full-source formatter route has not been verified. Keep the formatted count null pending an affirmative ownership and interface review. This is not a claim that no tool exists. |
| C3 0.8.5 | An official full-source formatter route has not been verified. The C3 project's [showcase](https://raw.githubusercontent.com/c3lang/c3-showcase/main/README.md) lists `c3fmt` among community projects and states that showcased projects are not officially endorsed. That listing does not qualify a compiler-owned formatter. |
| Clojure 1.12.6 | The official [`clojure.pprint` code dispatch source](https://raw.githubusercontent.com/clojure/clojure/clojure-1.12.6/src/clj/clojure/pprint/dispatch.clj) supplies a printer for read forms. A whole-file route preserving reader forms, comments and original source meaning has not been verified. Third-party formatters do not satisfy the official-project requirement. |
| SWI-Prolog 10.0.2 | Official [`portray_clause/2`](https://www.swi-prolog.org/pldoc/man?predicate=portray_clause/2) prints a clause term. Its existence does not qualify a full-source formatter or pin a 10.0.2 source-preserving driver. Operator state, directives, comments and variable naming need a reviewed route. |
| Oldes Rebol 3.22.1 | An official full-source formatter route has not been verified. Value serialization alone does not establish preservation of the original authored source bundle. Keep the formatted count null until that route is qualified. |

No unverified status is inferred from `PATH`, a failed lookup or a guessed
repository URL. The worker may mark an affirmative unavailable result only with
its recorded reason. Pending, unavailable and failed results remain null, with
`formatted_loc_complete: false`; there is no canonical LOC fallback.

## Selector schema and staging

`endpoints.finalize_endpoints(data)` receives the full admitted catalogue from
`program_inventory` and selected chart programs before pruning. Shared entries
must agree on their source, measurement and canonical coordinates. The adapter
owns file-level receipt and toolchain admission.
The selector validates these additional program fields:

| Program field | Accepted meaning |
| --- | --- |
| `formatted_loc` | Exact positive integer for a qualified complete bundle; `null` otherwise. Booleans and floats are rejected. |
| `formatted_loc_status` | Exactly `qualified`, `unavailable`, `pending` or `failed`. |
| `formatted_loc_complete` | Exact boolean; true only for a qualified complete bundle. |
| `formatted_loc_evidence` | Object whose receipt contents were qualified by the adapter. The selector checks the object type, without independently qualifying it. |
| `formatted_x` | Qualified `formatted_loc / baseline_formatted_loc`, or `null`. |

Absent overlay fields on older rows stage `pending`, `null`, false and an empty
evidence object. A claimed qualified count that is missing, incomplete or not a
positive integer rejects the entire update. So does a numeric count on a
nonqualified record, an unknown status or a malformed completeness/evidence field.
A partial bundle can retain per-file evidence while its program count remains
null. An existing, nonsymlink UTF-8 file may contribute zero physical lines,
including a successfully formatted empty helper; a complete program still needs
a positive total. Consistent nonqualified records exclude only formatted selection.

Canonical `fast` and `small` identities retain the existing stable tie rules.
`baseline_loc`, `baseline_seconds`, `x` and `y` retain their exact saved values;
the selector cannot recompute canonical normalization when formatting or adding
formatted endpoints. `reference_summary` uses those preserved canonical
coordinates. Each group additionally receives `formatted_fast` and
`formatted_small`, with null identities when no suitable source exists.
Formatted ties use seconds then formatted LOC for fastest, and formatted LOC
then seconds for smallest, retaining input order for exact ties.

`group.programs` retains the ordered deduplicated union of canonical fast/small
and formatted fast/small. A third source selected by formatted LOC therefore
survives with its original source bytes and timing. `baseline_formatted_loc` is
the minimum count among all complete qualified sources in the original benchmark
pool; it is null if there are none. The existing time baseline also normalizes
formatted endpoints. A source-only smallest endpoint keeps a null time and
cannot create a complete timed suite.

Endpoint selection retains the full `program_inventory` and every source file
referenced by that inventory. Chart pruning cannot remove an inventory-only
source from the inspector. Formatted overlay fields and coordinates stay equal
on any program represented in both inventories; source and measurement fields
remain unchanged.

`formatted_reference_summary` is separate from `reference_summary`. Its `x`
coordinates use formatted LOC, and its `y` coordinates use the unchanged time
baseline. It contains only languages with both formatted endpoints timed for
every benchmark. Languages also receive `formatted_timed_benchmarks` and
`formatted_source_benchmarks`. V and Red remain excluded.

The selector stages source pruning, ratios, audits and both summaries in a deep
copy, then commits after all prerequisites pass. `test_endpoints.py` authors
fixtures for third-source promotion, null/partial coverage, independent
baselines, source/timing preservation, stable ties and late transactional failure.
Those fixtures have not been executed by this source-only authoring task.
