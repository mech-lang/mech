# Current phase-window facts in native loops (R226)

Retain this compiler experiment on the original pinned compressor source.
Natural text at 32769 bytes improves **4.368%**, from **2.463615× to 2.356008×**
the matched reference. The additional 32767/32768 window-boundary cases improve
4.643%/4.122%. Random32 regresses 0.860%; eight-byte block means regress 0.743%.
All 25 unchanged gates pass. The geometric block-mean gain across the 25 cases
is 0.780%; preserve the contrary cases.

The R225 cache-read source bindings remain independent. This is a compiler-only
experiment; maintained normal IR215 remains unchanged. Extra ordinary resource
restoration is not a prerequisite for the standalone comparison. Full parity
and the remaining brief are open.

| Input | Before / Roc | Candidate / Roc | Block-mean change |
|---|---:|---:|---:|
| Natural text, 32769 | 2.463615× | 2.356008× | −4.368% |
| Periodic, 33027 | 2.151214× | 2.150687× | −0.024% |
| Repeated, 32769 | 2.008186× | 2.004979× | −0.160% |
| Random, 32769 | 3.043909× | 3.070085× | +0.860% |
| Distribution change, 16384 | 3.003855× | 2.985862× | −0.599% |
| Eight literals | 0.525476× | 0.527454× | +0.743% |
| Natural text, 16384 | 2.393434× | 2.315709× | −3.247% |
| Held-out mixed, 131073 | 2.795245× | 2.770870× | −0.872% |

Ratios are per-call medians; changes use balanced block means. All 25 raw
observations and distributions remain in the evidence. The stored-only
sub-microsecond cases retain their clock granularity and all original gates.

## Design and source semantics

The real cached-match assertion establishes length 3..258. A compile-only trace
shows that the old analysis then loses the range through successful checked
subtraction by two, treating the resulting child count as potentially U64::MAX.
The trace emits an Object byte-identical to the frozen parent.

The existing current-traversal domain now carries conservative intervals through
checked or proved-safe mathematical arithmetic, preserving the distinction
from unproved wrapping expressions. Each query records the maximum upper bound
across every state reaching that exact current child-initialize site. Selected
branches, current word identities, joins and fresh visits retain their existing
rules.

The existing reduction windows and those bounds derive a phase guard recipe.
It uses no application/field names, source IDs or hard-coded 258/259 rule. The
four actual path phases derive node floor 258 and cost-table floor 259. Before
the synchronous call, current lengths and vector geometry are checked alongside
the existing owner/capacity gates. Refusal enters the original checked root CFG
before any phase action, preserving short/defaulting reads, zero trips, assertions
and error order.

Existing numeric membership fixes view identity; successful unit append is the
only admitted length-changing operation in this recipe. A growing view requires
a row vector. Each fresh traversal receives only its nondecreasing length floor;
old length equalities, scalar snapshots and pointers do not cross growth. Current
backing pointers still refresh through the existing growth and child-input path.
No source predicate is promoted to document-level constraints.

The reduction reuses the exact guarded geometry and current count bound. Its
active/nonempty tests, source assertions, stable winner selection, tails,
ownership and cleanup stay. No scalar minimum rewrite, threshold adjustment,
private-write or wider-match experiment is combined.

Final review found repeated traversal of shared arithmetic DAG nodes in the first
prototype. The final compiler visits each needed node once in ID order, with no
recursive depth cap. New 96-level shared and 64-level checked-chain controls pass.
It regenerates **15 Objects and 15 metadata files byte-for-byte**, including the
exact timed compressor. Measurements remain for the same executable; no speedup
is assigned to the compilation-complexity repair.

## Evidence and limits

162222 primary calls, 3900 separate CPU/heap calls, 24108 wall-profile calls and
95790 current-PC calls cover the same 20 original and five held-out cases through
131073 bytes. Both actors keep runtime inputs, fixed level12, actual borrowed
prepared outputs, equal 200ms warmup, the same Rust clock and miniz validation,
and 24 balanced order permutations. Scratch construction/compression/cleanup
remain timed; all outputs, inflate, aliases, prefixes and capacity agree.

Natural32 instructions fall **164913414.5→153114839.5**, down **7.154%**.
Instructions fall on 20 cases/rise on five; cycles fall on 22/rise on three.
All 25 before/after allocator and payload/copy/zero/peak fields match. Counter
cohorts and sampled percentages remain separate from elapsed acceptance.

Four actual path bodies shrink **2208→2096 B**. The other 130 numeric bodies and
two maps retain byte-identical code. Root text falls 656 B; root stack falls
32 B to 86736 B. Static path counts are 552→524 instructions including padding,
50→50 conditional branches, and 107→108 loads. No static load/branch reduction
is claimed. Current large-case wall samples put 93.4–96.7% in generated code;
that share includes useful computation.

Validation: **679 maintained tests**, **21 focused proof tests**, **160 linked
path controls**, and **480 repaired matrix controls** pass. The six-feature
profile and warnings-as-errors remain. The same seven inherited tests stay
excluded. Parent/candidate and original/decoded path controls match every value,
status and origin; they exercise growth, short arrays, matrix fallback, assertion
rejection, held inputs, prior publication, cleanup and retry. Candidate original
records 33 selections/19 fallbacks; decoded agrees.

Resident path comparisons have 34 inherited default work denials per 40-case form
(136 across four forms), with the same parent disposition and unchanged allowance.
Two resident numerical comparisons and four assertion errors pass per form;
all 160 native cases match the independent oracle and parent. The repaired matrix
controls have 400 resident values/80 errors and no default denials.

A separate full observer executes 50 correct calls across all 25 cases and records
**264 guarded admissions, zero refusals**. Natural32 admits ten phases per call,
repeated two, random three. These are phase admissions, not child-visit counts.
Controlled short/matrix cases separately prove refusal behavior. Instrumented
execution is excluded from primary/counter claims.

Test-harness source-kind, dependency-identity, unused-helper warning and resident
error-classifier failures are recorded and corrected. There is no unresolved
introduced compressor failure. No expected output, allowance, algorithm, table,
search limit, allocation site, real scratch capacity, allocator or timer endpoint
changes. No fresh multi-MB run, ordinary-entry timing, all-feature dependency build
or green hosted CI is claimed.

## Revisions, public source and next step

Base `ecec11ac6278f1fd520a84bd584ebb572ebf592c`; experimental compiler parent `3c2fadb93ab8254eef8bcd943bbf3b9c76d2876c`; maintained normal `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`. Twelve archived compiler files change. Final compiler-source manifest SHA-256: `0bd9fc533c299c06982b3f0302d40a5719abb7cbbe488dd0930d9bb69e959eb8`. Object SHA-256: `b6a5575f1ff520530357278410eb2af467372aa110c715cb396a63f2a99a6717`. Original source/bytecode hashes remain `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9` / `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.

[r226-phase-window-facts-667eac538795.tar.gz](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r226-phase-window-facts-667eac538795.tar.gz) — 4804389 B, 1418 hashed files, SHA-256 `667eac538795672e5c3603179bf49750ad2ab38a55793837151335fbc82d57d3`. Local and downloaded hashes plus numerical replay verify. The archive includes the complete candidate Rust source, minimal patch, measured Mech source, witnesses, build specifications, raw structured cohorts and failure receipts. Native binaries, bytecode, full executable metadata and raw traces are excluded.

Next bounded experiment: same-address wrapping increments and proved-private
indexed writes on the slower random/distribution inputs. Keep shared
materialize-on-change and error order, and measure it separately. General read
reuse, duplicate-compare, phase packet storage and wider matching remain separate
follow-ups. Combine R225 only after an explicit combined measurement.
