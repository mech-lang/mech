# Complete C3 data-fragment ownership correction, unexecuted

The actual restored v4 source replay retains 46 diagnostics of one kind,
`c3-comma-expression-review`, across six mains. All other 18 authored inputs
have zero diagnostics. No old source, fixture, normalized unit, replay record,
compiler evidence, neutral counter or published score is changed here.

| Actual authored source | Flagged lines and source offsets | Existing brace data owner (half-open) |
|---|---|---|
| `revisions/private-v2/sources/regexredux/compact.c3` | 7–14; 108,133,166,199,232,265,298,331 | [106,397) |
| `revisions/private-v2/sources/regexredux/optimized.c3` | 8–15; 130,155,188,221,254,287,320,353 | [128,419) |
| `sources/fasta/compact.c3` | 89–92; 2190,2258,2326,2394; 95–96; 2481,2541 | [2188,2451), [2479,2605) |
| `sources/fasta/optimized.c3` | 94–97; 2299,2367,2435,2503; 100–101; 2590,2650 | [2297,2560), [2588,2714) |
| `sources/nbody/compact.c3` | 70,72–73,75–76,78–79,81–82; 1730,1862,1934,2092,2165,2327,2399,2561,2633 | [1728,2718) |
| `sources/nbody/optimized.c3` | 87,89–90,92–93,95–96,98–99; 2077,2209,2281,2439,2512,2674,2746,2908,2980 | [2075,3065) |

These offsets, diagnostics and scopes were read from the saved actual
`c3-source-replay-v1/jobs/c3-v4-source-replay-restored-v2/source-count-proposals.json`.
The original sources show string-table continuation rows, probability-record
rows and numeric body-record rows inside those scopes. The complete C3 parser
already proves the matching brace data owner and rejects hidden callback bodies,
statement sequences, writes and mutation. The per-physical-row binding callback
then sees a comma without the opening brace on that row and treats it as an
unowned executable comma sequence. This is a lost-context defect.

V5 adds a bounded source-local proof before that one diagnostic is emitted. The
frozen neutral callback supplies exact original/cleaned fragment text and the
active context, but no absolute fragment interval. The new language-owned
adapter requires a **unique** exact original/cleaned substring match with that
same exact active parser context. Ambiguity or mismatch retains the diagnostic.
It then requires the **whole meaningful fragment**, including its final token,
and **every comma** to fit inside one existing brace data scope after the last
executable/type scope. This proves the complete comma region; knowing only its
first offset would be insufficient. A close followed by a comma, effect,
callback or write extends beyond the owner and fails this proof.

Each accepted fragment emits its actual source interval, original text, comma
offsets and complete owner interval in `complete_data_fragment_certificates`.
Array-index/parameter/call/control contexts do not grant this brace-initializer
proof. The normalizer still projects all original declaration, initializer,
chained-write, literal wrapping, header/body/close and remaining row units.
No count override, diagnostic-kind whitelist, first-offset shortcut, shared
counter change, executable-owner suppression or grammar fallback is added.

The v4 mask and all 19 existing fixture method bodies are copied unchanged.
Four new methods exercise multiline string tables and nested probability/body
records, complete interval certificates and preserved literal wrapping; they
also require callback/write/mutation/semicolon-body negatives and comma/effect/
callback/write negatives after an initializer close. Exactly 23 distinct methods
are expected for the additive fixture qualification. The actual prior v4
19-method passed receipt, raw log and terminal receipt pin remain mandatory and
are checked against the exact old discovered/started ID inventory. All earlier
failed receipts and source replay evidence stay untouched.

The copied direct worker uses the same original common mutex, immutable packet,
actual Python 3.14/Pygments 2.19.2 and frozen LOC18 neutral dependencies. No
benchmark source is parsed by its fixture stage. Root must review/freeze the
packet and run its single fixture stage under the existing durable launcher:

```text
/opt/homebrew/opt/python@3.14/bin/python3.14 -B benchmarks/clbg/loc-normalization/20261012/c3-proposal-v5/queue_fixtures.py --direct --job benchmarks/clbg/loc-normalization/20261012/c3-proposal-v5/jobs/fixtures-v5 --core-snapshot benchmarks/clbg/loc-normalization/20261012/jobs/boundary-audit-18/snapshot --dependency-root /tmp/mech-clbg-compact-libs --prior-fixture-receipt benchmarks/clbg/loc-normalization/20261012/c3-proposal-v4/jobs/fixtures-v4/fixture-receipt.json
```

Only after actual fixture success may an additive replay entry bind this new
receipt and grammar snapshot while retaining the original actual all-20/24-source/
25-copy compiler inventory. It must preserve the old v4 replay/diagnostics/units,
produce fresh source-local certificates, retain every diagnostic and keep parser
pass distinct from root's complete owner/span meaning review. All chart/count
admission remains false. No hash, import, parser, fixture, native process,
source count or Git operation has run during authoring.
