# Backward path-search checkpoint

Published change: [#944](https://github.com/mech-lang/mech/pull/944), directly
above #943. Base: `2a8ef64a83ed5cd36a87cca1485ced634082635a`. Validated code/source
head: `93a7be76e36a6e364152c50b1557cf6c49d07e0f`. The later evidence commit changes
only this packet. The native stack contains **29 PRs and 28 matching adjacent
head/base pairs**, with the architecture changes preceding the compression work.

Two separately reviewable commits address the concrete requirement:

- `b93fa92a30db8e35f1ca124695fed5a014d4cb9c`: compact storage for the compiler's
  existing current-visit proof, plus its minimal regression.
- `93a7be76e36a6e364152c50b1557cf6c49d07e0f`: actual backward path search,
  executed pinned reference producer, fixtures and qualification.

## Executing compressor functions

The new permanent source is `hosts/gpu/tests/fixtures/native/deflate-optimal-path.mec`:
`optimal-find-min-cost-path`, `optimal-path-scan`, `optimal-path-position`,
`optimal-path-guards`, `#OptimalPathMatches`, `#OptimalPathLengths`,
`optimal-node-top`, `optimal-offset-slot`, and the typed access/cost helpers.
They implement `CompressOptimal.find_min_cost_path` from pinned package
`671077457ff7ef3a3d44caf715a3fc595addf379`, using compiler
`d99c7b838fb05bb1047eb0c18d4d06d0f86f345d` for the independent reference.

The search preserves the packed U32 cache and U64 nodes, U32 wrapping costs,
guard nodes, backward cache traversal, nearest-offset length scan, strict
less-than tie choice, zero-fallback reads, and original check/arithmetic order.
Its caller reserves **305257 U64 entries**; the search clears and appends that
list and returns its record, preserving the reservation. There is no initialized
path matrix or whole-buffer concatenation on append.

All six reference cases match every node in original and decoded artifacts:
empty; eight literals; eight overlapping bytes; 320 overlapping bytes; 320 bytes
with tied costs; and eight literals with wrapping U32 costs. The largest path
case is **320 bytes / 578 nodes**. All cases use the real **1500513-word U32
cache**, **256/259/32-entry U32 cost tables**, and the pinned **256-entry U8
offset-slot table**. The new producer executes the unchanged pinned function;
it verifies all 14 tracked package modules and records source/binary hashes.
Numeric fixture compaction was reviewed by comparing parsed JSON values before
and after formatting: all values were identical.

The connected qualification executes `optimal-first-block` and then the path
search as **one compiled Mech computation**. Mech allocates and updates its own
**262144-entry U16 tree** and **1500513-word U32 cache**, with depth **300**, nice
length **258** and the original near-EOF rules. Both eight-byte cases match the
independent reference path, for original and decoded artifacts. The test supplies
cost models; their generation/iteration is not yet connected.

All 28 preexisting GPU fixture files, including the six preceding compressor
source components, are byte-identical to the base. Search and skip remain
separate; skip has no cache argument or recording path. No existing expected
value or allowance changed. Source syntax, semantic frontend and scalar code
emission files are unchanged.

## The compiler blocker and bounded fix

Before execution, joining the collector and path search failed native lowering
with `InvalidBinding { node: None }`. Temporary diagnostics located the refusal
at the existing **16777216-cell** current-visit analysis preflight. It stored
one byte per definition flag and tested a dense blocks × cells estimate. The
checks had not reached an invalid-value/ownership finding.

A generated source containing one ordinary recursive function and 320 distinct
calls reproduces the rejection independently of compression. Its lowered plan
has **6422 blocks and 2890 values**, or **18559580 dense cells**. With compact
word storage, its rounded payload plus row-owner estimate is **2568800 bytes**.
The actual connected bytecode also reproduces the old rejection on tracked-clean
parent `851507304181308ddbadcd96bec24859e8deeed4`; see the paired receipt.

The fix changes the private proof set to one bit per definition and performs
join intersections a U64 word at a time. Initial definitions, read-before-write
checks, lexical ownership, current-loop carriers, collection liveness, tail
returns and output/effect checks retain the same rules. The **16 MiB finite
row-storage admission estimate** remains, now accounting for rounded words and
row owners. This estimate is not a measurement of total compiler peak memory;
other verifier scratch and graph metadata remain additional. The independent
generated-block ceiling and project defaults are unchanged.

Native work policy is **90**. Typed IR remains **109**, public ABI **35**, and
canonical bytecode revision **14**. Private proof sets are scratch data and are
never serialized or used as execution authority.

## Passing receipts

Final tests ran at tracked-clean `93a7be76e`, using `nightly-2026-03-03`, locked
offline dependencies, warnings as errors, one build job, no incremental
compilation and no test/dev debug information. `receipts.json` contains exact
commands, profiles, revision/status snapshots, times and log hashes; raw logs
are retained in `validation-logs.json.gz`.

| Qualification | Observed outcome |
| --- | --- |
| GPU `canonical-native-program`, maintained component/linked selection | **171 passed**, 0 failed/ignored, 129 filtered; 746.60 s test execution |
| Same command, caller-layout unit | **1 passed**, 422 filtered; persistent 1112 / invocation 1288 bytes |
| Engine `full_source,native-artifact,sequence_buffers`, proof units | **12 passed**, 0 failed/ignored, 1076 filtered; 0.98 s execution |
| Engine source-free native, `native-artifact,sequence_buffers` | check passes |
| Engine runtime-only, `sequence_buffers` | check passes |
| GPU runtime-only, `canonical-native-program-runtime` | check passes |
| Formatting, routing, source/host catalogs, module/analysis layout | pass |
| CI integration inventory | 0 gaps |

The GPU command is `cargo test --locked --offline -j1 -p mech-gpu
--no-default-features --features canonical-native-program --lib --test
canonical_artifact_program -- ... --nocapture --test-threads=1`; its receipt lists
every selected filter. Including compilation it took **882.323 seconds**.
The same 167 previous selected integration cases pass, plus three new path tests
and the compact-proof reproducer. The 12 engine units include both word-boundary/
storage tests and ten existing positive/negative ownership, phase, stale-value,
forged-edge, uninitialized-element and nominal-enum proof tests.

The path tests also exercise two failed checks together (entry-table check wins),
invalid cached lengths on both sides of the allowed range, unchanged published
owners and input aliases, cleanup and successful retry. The generic compiler
reproducer executes success → work denial → success in original/decoded forms.
Actual linked buffer/assertion/record recovery remains in the passing selection.

## Execution baseline

These are **unoptimized diagnostic component timings**, excluding source/native
compilation. They are not complete compression timing or a comparison with
another language. Requested allocation bytes are cumulative allocator requests,
not peak memory or measured copied bytes.

| Program/case | Original / decoded elapsed | Requested allocation bytes |
| --- | --- | --- |
| Connected collector + path, eight literals | **11.703 / 11.694 s** | 97481234 / 97481170 |
| Connected collector + path, eight overlapping bytes | **13.719 / 13.718 s** | 112684492 / 112684492 |
| Standalone path, 320 overlapping bytes | **0.5695 / 0.5692 s** | 818236008 / 818236008 |

The connected program has **5379 generated blocks** and uses the existing
explicit full-collector diagnostic profile: **16384 compile blocks**, **1 GiB
account**, **128 MiB temporary allowance**, **1 trillion execution/compute/
comparison work**, and **1500513 output elements**. The standalone path uses a
256 MiB account with the same finite work diagnostics. These do not establish
default-budget completion. The 305257-entry reservation survives every
successful path result; substantial allocation cost remains in the full path.

Largest completed sizes at this checkpoint: standalone path **320 bytes**;
connected collector + path **8 bytes**. The previous full collector's 320-byte
result remains separate evidence. A complete compressor has not executed.

## Review, CI and failure disposition

`@codex review` was requested on #944. Review of `93a7be76e` completed at
2026-10-27 10:52:16 UTC with no findings or unresolved threads. CI run
**40491641499** was canceled as requested; no completed remote CI pass is claimed.
Full CI, other platforms, all filtered tests and full compression timing were
not run. See `review.json` and the stack summary.

All nonzero development commands in this packet are classified in
`failure-disposition.json`. They comprise corrected draft source/generator
mistakes; the inherited analysis-storage refusal now fixed; an environmental
reference-compiler cache denial resolved by its authorized build environment;
and two reduced engine **unit-test** profiles that fail identically on clean
parent 851507304. The first lacks String backing for existing assignment tests;
the second has existing unused test helpers under warnings-as-errors. Those
exploratory configurations remain inherited limitations, outside this compiler
fix. The owning proof tests pass under the existing full-source feature family;
the reduced **library** profiles also pass. No warning suppression was added.

There is no unresolved failure in this patch's selected qualification.

## Next bounded source step

Still needed for `compress_into`: cost-model creation/adjustment and optimization
passes; `tally_and_build_codes` with the real Huffman widths/buffers/reversal
table; all-literal/static alternatives and true-cost decisions; block header,
bit writing/flush and final orchestration/carryover/output reuse. Exact raw-byte
agreement, independent decompression and full-input timing follow connection.

Seven fresh reference cases for the next Huffman adaptation were generated,
including **513-entry literal frequencies → 288 symbols**, **270-entry offset
frequencies → 32 symbols**, and **19-symbol precode**, with the actual 14/15/7
maximum lengths. That draft is not qualified or part of this patch. Its first
attempt encountered the existing closed-payload restriction of named FSM
states; the tested minimal source is retained as
`pending-dynamic-state.mec.txt`. The next source step expresses those variable
buffer loops with ordinary recursive functions, retaining the same operations
and allocation sites, then compares every resulting code length/codeword with
the seven pinned cases before connecting tallying. No broader FSM compiler
project has been started.
