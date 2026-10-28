# Private record buffer transport

The compiler patch is [#943](https://github.com/mech-lang/mech/pull/943), directly
above #942. Base: `1de0ee88e9bb882d5dddbfbc1ae4b864f51cddc7`. Production code and
tests: `6903862d4ef979c5ba7de9c4f331b1dba4f8cdb1`. The later report commit changes
only this evidence packet. The published native stack contains 28 PRs with all
27 adjacent head/base pairs equal; the architecture PRs remain before the
compression work. No merge is claimed here.

## The blocking operation and change

The minimum-cost parser clears and appends its U64 node list, then returns
`{ nodes: $nodes }` from `CompressOptimal.find_min_cost_path` (pinned source
`671077457ff7ef3a3d44caf715a3fc595addf379`, line 506). The caller reuses the
returned list. The output writer similarly carries a reusable U8 list in its
return state. Ordinary record construction/projection previously copied these
buffers and lost their reservations: 305257 U64 entries became capacity 8;
6633832 U8 bytes became capacity 64. The original minimal source and original/
decoded red acceptance are retained in the preceding
[frame-transport packet](../2026-10-27-frame-transport/REPORT.md), including the
same failure on clean parent `851507304181308ddbadcd96bec24859e8deeed4`.

This patch extends the existing private argument-frame analysis to flat records
containing native scalar fields and packed unsigned matrices. It admits only
complete nonescaping pack/copy/project components with exact leaf schemas.
Named projections require a constant Id selector with one unambiguous schema
field. Unsupported uses and escaping records keep the canonical route.

The actual selected composite factory still checks the complete record shape.
The actual selected field factory checks schema identity, selector meaning,
same-account ownership, current coexistence and finite memory/work limits on
every visit. It borrows the already admitted child owner. A tail-return read
qualifies only when its existing exit action copies that exact returned value
to its identically typed result. Escape rejection propagates through that copy.
Last-use retirement follows the existing plan's loop and action sites.

The build-owned image rederives the private metadata. Work policy is 89; typed
IR remains 109, public ABI remains 35 and canonical bytecode remains revision
14. Scalar instruction emission and source syntax files are unchanged. All 28
existing GPU fixture files, including the six compressor source components,
remain byte-identical to the base. No existing expected value or allowance was
changed.

## Validation

The final qualification ran at tracked-clean `6903862d4` with
`nightly-2026-03-03`, the repository's warnings-as-errors configuration, locked
offline dependencies, one build job, no incremental compilation and test/dev
debug information disabled. `receipts.json` records complete commands, feature
profiles, environment, elapsed times and individual log hashes. Exact logs are
in `validation-logs.json.gz`.

The main command was `cargo test --locked --offline -j1 -p mech-gpu
--no-default-features --features canonical-native-program --lib --test
canonical_artifact_program -- ... --nocapture --test-threads=1`; the receipt
contains every selected filter.

It passed **167 integration/component/linked tests**, with 0 failures, 0 ignored
and 129 filtered out, in 601.72 seconds of test execution. This comprises the
same 143 previous component cases, 8 sequence cases, 7 frame cases, 6 new record
cases and 3 linked lifecycle/recovery cases. The separate caller-layout unit
test also passed (422 filtered out), retaining persistent size 1112 and
invocation size 1288. Including compilation, the command took 739.000 seconds.

The new tests cover real U64/U8 reservation sizes and exactly one additional
reservation payload when capacity increases; recursive/repeated returns and
clear/append reuse; mixed fields and fresh Boolean/scalar values; input aliases
and shared record children; assertion-before-arithmetic ordering; unchanged
published owners on failure, cleanup and retry; canonical escaping records;
and original/decoded artifacts plus image metadata restoration. The selected
record field helper allocates **0 bytes in 0 requests** and rejects output
element, output byte, temporary byte, retained node, compute and comparison
limits, with cleanup and successful recovery. Actual linked execution exercises
a recursive record return and valid → invalid → valid inputs.

Source-free engine/native, engine/runtime and GPU/runtime profiles, formatting,
resident routing, source/host catalogs, architecture/module layout and CI
inventory results are recorded separately in the receipts. No warnings policy
or architecture/catalog check was relaxed.

## Real collector witness

The unchanged collector test completes six consecutive positions of a 320-byte
input in one compiled computation, using the full **262144 U16 tree**, **1500513
U32 cache**, depth **300**, nice length **258** and original near-EOF handling.
All observed state agrees with the checked-in reference; aliases survive and
cleanup returns the account to zero. No resource denial occurs.

At the final code revision, original/decoded execution takes **13.798 / 13.751
seconds**, with **43322 / 43321 allocator requests** and **81179314 / 81179250
cumulative requested allocation bytes**. The preceding code's same witness
requested about 259.5 MB. These are unoptimized diagnostic executions with the
same finite 256 MiB account, 128 MiB temporary allowance and 10-billion execution,
library-compute and comparison allowances. Allocator requests measure neither
peak memory nor actual copied bytes. This is not complete compression timing.

## Review, CI and failed-command disposition

`@codex review` requested review of `6903862d4`; automated review completed at
2026-10-27 10:00:49 UTC with no findings or unresolved threads. See `review.json`.
CI run **40484947314** was canceled as requested. No completed remote CI pass is
claimed. The local qualification intentionally filters other integration and
unit cases; full CI, other operating systems and complete compression timing
were not run for this patch.

Four nonzero record-patch commands are retained:

- `record-transport-first-acceptance`: introduced Rust borrow error; fixed by
  obtaining coexistence before borrowing the library account.
- `record-transport-recursive-first`: exposed the inherited recursive return
  capacity loss, now fixed. Its allocation comparison also exposed unequal
  first-use metadata warming; both capacities are now warmed identically before
  measurement. No allocation tolerance was added.
- `record-transport-recursive-fixed-and-real-trace`: command-line filter
  placement error; corrected command passes.
- `record-transport-ci-inventory`: the system Python lacks PyYAML; the same
  inventory runs with the existing configured CI Python environment instead.

There is no unresolved failure in this patch's selected qualification. The
earlier resident default-budget denial of the real-size minimal record source
remains an inherited limitation, documented with clean-parent evidence in the
preceding packet; this native transport patch does not raise that default.

The compressor still needs cost-model orchestration, minimum-cost parsing and
code tally integration, the original Huffman buffer adaptation, block writing,
and `compress_into` orchestration before exact-byte/decompression checks and a
complete measured run. The separate path-search implementation is being
qualified next; it is not part of this compiler patch.
