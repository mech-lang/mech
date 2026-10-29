# Round 12: native private unsigned indexed-write recurrences

The completed round reduces unchanged eight-byte level-12 compression from
**345.68× to 251.38× Roc** (Roc = 1×): **27.28% less elapsed time**, or
1.3751× faster Mech. There are **231 scoped Rust passes**, twelve passing static
contracts, and twelve passing paired scalar timing/allocation controls. This is
a small-input result; parity and large-input throughput are not established.

## Revisions and unchanged application

- Round base: `efa2ff81741362394553d5c7eae7d9dfc39766c4`.
- Core handoff: `02fb501b3935d16f75b1eda4fbe049bdf1051465`.
- Native proof, shared emitter and runtime: `ed75406f3893e05070542dc523634251947a7fdc`.
- Private carrier-pin correction: `c0bef396bbc88cdd8dd94037053866b182b7a68d`.
- Final measured production code: `d105f884bc79d464a205b3022da4abdc00da41aa`.
- Final unit/linked qualification and test-only error matching:
  `e7767697e1b1bdc6e8013a499a1cecbb80319a22`.
- Published draft PR: [#973](https://github.com/mech-lang/mech/pull/973), stacked
  #934 → #935 → #939 → #973. PR parent:
  `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

The reviewed head `a30f6a4d67defc176ad1cc498df3f6bbb459c56a` received a completed
code review with no findings. The closure adds test/receipt/documentation changes;
a fresh review is requested on publication. CI is cancelled by user request and
is not claimed green. The PR remains draft and unmerged.

The 25-component assembly remains 160992 bytes, SHA-256
`345324ce551aaa2485dcd0fe4262da96e660d6b31f5f772bff89b034e9b1997f`.
The timing caller source remains 160857 bytes, SHA-256
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`;
its 4138627-byte canonical artifact remains
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Fresh source production, original-artifact encoding and decoded re-encoding agree
exactly. Algorithms, tables, the 262144-entry U16 tree, 1500513-word U32 cache,
depth 300, nice length 258, original EOF limits, six Huffman initializers,
allocation sites and timing boundaries are unchanged. Mech performs compression.

## Final paired measurement

| Executable | Median seconds | Multiple of Roc |
| --- | ---: | ---: |
| Fresh pinned Roc reference, 20000 observations | 0.000289 | 1× |
| Round-11 compiler, three primary observations | 0.099901542 | 345.6801× |
| Budget-correct round-12 compiler, three primary observations | 0.072649542 | 251.3825× |

`validated-primary/results.json` retains every observation. The warmup and extra
observation are excluded, as is a separate 56-repetition sampled process. No
compilation or test process overlaps protected primary or scalar timing. The
same pinned mimalloc 2.1.7 object is used without the allocation probe. The timer
encloses only `evaluate_into`; reserve13, clear/rebind, JIT, verification and drop
retain their existing boundaries. All five observations per side preserve exact
raw bytes, independent inflation, capacity 13 and terminal zero bytes/nodes.

`same_backing` is false on every call on both compilers. Capacity is retained,
but physical copying at the immutable public argument boundary remains timed.
The diagnostic `NoLimits` policy is unchanged. Logical work changes from
789930572624 to 530905237224 because proved native iterations eliminate canonical
frame/store construction work; IR117/work policy92 prices the new representation.
Canonical bytecode remains revision14 and ABI35 remains unchanged.

Executable SHA-256:

- Before: `7d1553881fc896b541d93345170e49a01bd7bbcf15cdce0abb12522508962dcf`.
- After: `390370ca3419a4fa03646979a3ff121db07fc85cbca83e7226cf89db38d409a5`.
- Reference: `3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`.
- Allocator object: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.

The executable build receipt records pre-commit HEAD `cfcd9d6e9` with the final
output-limit patch applied; that exact production patch is committed as `d105f884b`.
The timing result identifies `d105f884b`. Library and executable hashes bind the
actual measured build; receipt/documentation-only commits are not new timings.

Earlier independent cohorts are preserved, with their own before/reference pair:

| Stage | Packet | Before → after seconds | Roc multiples |
| --- | --- | --- | --- |
| Initial implementation | `primary/` | 0.099616958 → 0.073028666 | 344.70× → 252.69× |
| Corrected private handoff | `final-primary/` | 0.099590208 → 0.072269500 | 345.80× → 250.94× |
| Corrected unchanged-write output bounds | `validated-primary/` | 0.099901542 → 0.072649542 | 345.68× → 251.38× |

These cohort differences are not additional paired speedups. The full historical
checkpoint narrative is preserved byte-for-byte in `CHECKPOINT-HISTORY.md`.

## Design and authority

This extends the existing canonical Program numeric-leaf and closed-tail
pipeline. Selected unsigned indexed replacements and either a scalar result or
one numeric-array result are eligible. A forward must-analysis checks current
functional array versions on every acyclic source path. Stale reads/returns,
discarded updates, swaps, unknown effects, escapes, nested activations and
unsupported result shapes retain the maintained path. Names, values and table
sizes never select the optimization. Exact frame recipes, schemas, geometry,
selected factories and dataflow remain authoritative.

Arrays retain invocation-current owners separately from scalar SSA state. A leaf
may execute beneath an ordinary suspended parent, which cannot run while native
views are live. All unsigned widths and qualified rectangular linear indexing
are supported. Shared inputs and actual aliases remain immutable.

At the first changed write, native fuel settles and the original selected
consuming replacement receives the current operands and carrier owner. Core
checks dynamic exclusivity: a private buffer retains its initial backing; a
shared one copies once. A fixed-shape private owner holds an already admitted
future canonical wrapper and ownership registration. Subsequent successful
iterations emit direct typed loads/stores. No-op stores preserve original
identity. Entry declines when a writable view exceeds the selected output-element
ceiling, allowing the original operation to enforce its error at the reached
source position, including unchanged stores and unselected paths.

A pre-admitted value-arena ticket binds publication to the exact arena,
generation, type, account and retained bound. Freezing allocates nothing and
performs no new admission. At fuel exit current cells freeze and current scalar
fields synchronize. The first changed traversal remains prepaid when execution
resumes on the same post-materialization account: it is not funded twice.
Failure discards private owners and preserves prior publication, aliases,
cleanup and retry. Restored-image proofs and prices are independently rederived
from the validated plan. JIT and Object emission share this implementation.

## Qualification and commands

Nightly `2026-03-03`, aarch64-apple-darwin, warnings as errors, one compiler job,
locked/offline Cargo and `CARGO_INCREMENTAL=0` were used. Default qualification
uses `benchmarks/mech-deflate/native-profile/Cargo.toml`: full Core/Engine compiler
values, canonical native Program, and only the five required math operations.
The optional `unit-qualification` profile adds linked qualification and maintained
runtime source-routing support. Exact rustc arguments, selected dependency hashes,
Cargo feature metadata and executable commands are retained, not inferred.

| Executed qualification | Passed | Receipt / command owner |
| --- | ---: | --- |
| Core snapshot unit tests | 87 | `validation/round12-core-snapshot-cohort.*`; `core-publication-tests snapshot:: --nocapture --test-threads=1` |
| Existing and new maintained component cases | 129 | `qualification/qualified-components.json` and compressed log |
| Selected native scalar correctness | 8 | `validated-primary/selected-native-run.json` |
| Selected bit-operation correctness | 3 | `validated-primary/selected-bits-run.json` |
| Actual runtime fuel and adversarial publication units | 3 | `final-qualification/units/unit-run3.json` |
| Generated runtime-only linked retry application | 1 | `final-qualification/linked/component-v2-run.json` |
| **Rust total** | **231** | Zero failed/ignored tests within these selected cohorts |
| Architecture/catalog/inventory/unsafe/warnings/format contracts | 12 | `validation/round12-final-static.*` |
| Fresh paired scalar timing/allocation controls | 12 pairs | `validated-primary/scalar-results.json` |

The actual unit command selects `numeric_fuel_exit_synchronizes`,
`numeric_write_fuel_exit` and `prepared_publication_is_bound`, with `--nocapture
--test-threads=1`. Sixteen observed write-fuel exits (eight per artifact form)
freeze prefixes of 64/63/61/57/49/33/17/1 updated cells, with current i/end fields,
released loans, unchanged prior output and stable final bytes/nodes. Retry
succeeds. The retained read-only unit observes ten actual synchronized exits;
six additional denied cases stop before native entry. Publication adversaries
cover wrong arena, generation, type, foreign account, retained bound and successful
publication at full admitted memory.

The linked command selects
`linked_private_numeric_child_freezes_recovers_and_preserves_error_order`.
Its generated binary executes inputs **3 → 0 → 2** through a child with a private
write beneath an ordinary parent. The invalid invocation asserts after its first
write and before failing division; prior output is unchanged, all temporary
state is cleaned, and the final valid invocation succeeds. Generated project
pins, Object metadata, binary hash and actual compilation-unit closure are in
the compressed run log. The deployed program uses the runtime-only feature
closure and excludes parser, compiler and Cranelift. Its binary SHA-256 is
`140a94404a5c5421f85a6334952978e94534c16752ec179b7c71648447e8ed05`.
This is linked feature qualification, not a full linked compressor benchmark.

After the test-only typed-error correction, five applicable checks were rerun:
formatting, diff whitespace, unsafe boundaries, warning policy and Rust module
layout. All pass; receipts are in `final-qualification/round12-late-checks.*`.
The full twelve contracts passed before that test-only change. The final
unit-profile build completed in 964.15 seconds; no timing ran concurrently.

All twelve fresh scalar pairs meet the unchanged 1.05 ceiling, retain exact
allocation requests/bytes, and preserve zero loop allocation slope. The maximum
direct original/decoded ratio is **1.0180374**. The historical round-10 decoded
U64 ratio **1.0526036** against 1.05 remains unresolved; a fresh passing cohort
does not erase that earlier failure.

## Allocation and machine-code evidence

Component diagnostics use the System allocator and probe, separate from complete
mimalloc timing. At 256 iterations the generic counter changes from
4090 requests / 275146 bytes / 4628 visits to **276 / 38610 / 6**. The actual
unchanged Huffman counter changes from 5629 / 827362 / 13846 to
**280 / 44906 / 6**. Positive counts stay flat from 1 through 256 iterations,
for original and decoded artifacts.

Real shared-buffer typed-store witnesses request 561530 bytes for the U16 tree
and 6039294 for the U32 cache, with 276 requests / 6 visits each; one and eight
updates agree. The pre-existing private-allocation contract is unchanged:
large-minus-small requested bytes are exactly **6000900** for zero, one and eight
updates, in both forms. This proves one initial private payload allocation.
The test caught and drove correction of an initial extra payload copy.

The complete Object selects **91 write regions**, 88 parented, with **117 typed
store sites and 315 load sites**; the retained 45 read-only regions have 72 loads.
An emitted-CFG audit proves zero calls on successful already-private backedges.
First-change cold helper arms remain explicit. These are compiled sites, not
runtime invocation counts. Actual disassembly includes the checked private
branch and `str w26, [x9, x1, lsl #2]`; excerpts and hashes bind the full Object
(7566856 bytes) and 1891534-line disassembly retained privately.

The native reference binary was inspected passively. Hashed procedures contain
typed memory operations, but their identifiers do not prove a mapping to named
source helpers. No per-function reference timing or source attribution is
claimed. Large binaries/full disassemblies are excluded from Git; hashes,
metadata and excerpts are retained in `qualification/`.

The final separate profile has 1356 sampled stacks. Inclusive ancestry contains
helper 1005, buffer-frame action 446, frame-word and input admission 318 each,
ordinary visit 232, selected library call 189, selected replacement 58 and
selected read 33. These overlapping counts are not additive time buckets or
predicted speedups. Ordinary mixed array/scalar frame admission remains a
measured cost. The next bounded candidate is the existing Huffman cumulative
loop's `(countsArray, usedU64)` result, currently outside this slice's result
shape. Establish its generic witness and acceptance before widening support.

## Failed attempts and remaining coverage

- Introduced compile mistakes (missing `SequenceView` import and a memory/snapshot
  error conversion) were corrected before release.
- A Core command used a nonmember package manifest; another selected zero tests.
  Neither is passing evidence. Corrected execution passed 8 ownership tests and
  then all 87 snapshot tests.
- Initial width fixtures requested unsupported narrow addition. Typed constants
  now isolate the selected-store capability; application source is unchanged.
- A new exact-budget fixture omitted prior-output retirement; an unwarmed
  receipt caused the failure. It now uses the actual warmed receipt. A separate
  cold metadata difference (261 versus 256 requests) was removed by warming both
  measurements. No expected component result or allowance was increased.
- The initial broad cohort passed 125/126 and exposed an introduced extra private
  copy. `c0bef396b` transfers the carrier pin; the original allocation contract
  passes unchanged. An interrupted unit-profile build is retained separately.
- The new unchanged-write test exposed an introduced output-element-bound bypass
  (length8, limit4). `d105f884b` restores the selected-operation boundary. Clean
  parent evidence in `baseline-review.json` shows the original selected check.
- The timing overlap guard rejected an attempt while a freshness build was still
  active, before samples. Measurement restarted only after completion and
  library hash verification. The rejected attempt is retained.
- Initial formatting failed and was corrected. The full twelve contracts and
  final five applicable checks pass.
- The first fuel unit assumed every work denial used a `WorkLimit` debug string.
  Actual synchronized denial correctly used `Control::Work`. The test now
  matches explicit maintained typed work failures; runtime classes were not
  merged or changed. Initial and diagnostic failed runs are retained alongside
  the final three passing units.
- The inherited historical round-10 scalar timing failure remains unresolved.
  No remaining introduced correctness failure is known in the executed scope.

The selected-operation catalog deliberately omits an all-primitives not/xor test;
it is not counted as a pass. Full-catalog/all-unit sweeps, larger complete input
boundary refreshes and a complete deployed compressor benchmark were not run.
No cancelled full-corpus or long-input campaign was restarted. CI is cancelled,
not green. General mixed results, growing producers, nested native regions and
SIMD are outside this completed round. No further optimization is included here.

`closure-inventory.json` hashes every packet file other than itself. Earlier
stage inventories remain unchanged and describe their checkpoint, including
checks which were pending at the time. Actual executable commands, selected
profiles, failures and observations are retained in those receipts. About 35 GB
of inactive debug build cache was removed after preserving the successful Core
unit executable and receipts; release and measured executables remain private.
