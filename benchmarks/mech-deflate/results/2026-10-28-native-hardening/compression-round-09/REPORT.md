# Round 09: independent cost-table source experiment

Published compiler baseline: `a59117d63d2a07d3e0b5cdac45dab202114b95f7`.
Protocol: `ac27c9dd549d901cf38b4177c5c68cacd6967416`.
Source variant and reference cases: `88691a2b7f6e34d4fd0a0230e819ac42611cde90`.
Fallback boundary qualification: `f8d621521e8ad28835a82e5321045ad5d26bc77f`.
Empty-lookup qualification: `70416afa704f7e5c12b89339fa713e609ed58778`.
Compiler/runtime code is unchanged from `4af4662097dd570f0821fc105a1c4d9de4eb5d89`.
Stack parent: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

This is an **application source experiment, not a compiler speedup**. The default
25-component assembly and its source pin remain untouched. An explicit benchmark
helper retains the nine original cost-table loops as sequential fallbacks and
adds independent 256/259/32-entry comprehensions. Default literal fill uses the
existing uniform constructor; length entries zero through two are preserved.
Default/code-derived/adjusted cost arithmetic, masks and tables are unchanged.
No histogram/candidate reuse, dead-initializer deletion, header/writer rewrite,
cache change, search change or output-buffer change is included.

The variant treats cost arrays as one-dimensional lists, as the actual compressor
does. Nonstandard vector lengths/starts use the original helper, then an explicit
row copy so both match branches have the same exact source schema. That fallback
has additional allocation. General multidimensional matrix use is outside this
experiment; the pinned compiler baseline retains its qualified matrix semantics.
The variant is optional and does not replace the public/default source owner.

## Small complete timing

Apple M1, eight-byte `abcdefgh`, level 12, **Roc = 1x**:

| Subject | Median seconds | Divided by fresh Roc |
| --- | ---: | ---: |
| Pinned reference, 20000 observations | 0.000290 | 1x |
| Pinned Mech source | 0.130136250 | 448.75x |
| Independent-table source variant | 0.122244917 | 421.53x |

The variant uses **6.06% less elapsed time**, or 1.0646x the paired baseline's
speed. This is a modest whole-program improvement, despite the much larger
component allocation reduction. It does not deliver the larger win being sought.
The prior 454.89x result uses a different reference cohort. Both subjects here
rerun against the same fresh reference and use the **same retained executable**:
`0fa38254f8f5773f49749201a07ad2032356d38b5d7f47a44f4c581a6bdc6570`.

Three retained calls each, warmup/extra calls excluded. The two-second sample
and 33 profile repetitions are separate/excluded. No compiler, qualification or
other benchmark overlaps primary timing. Same pinned mimalloc 2.1.7, selected
catalog, Diagnostic NoLimits, evaluate_into-only boundary, eight-byte input and
caller capacity-13 reserve/clear/rebind reuse. Every call checks exact raw bytes,
independent inflation, input aliases, output capacity and terminal zero managed
bytes/nodes. No larger complete input was run.

Pinned source SHA-256:
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
Pinned artifact SHA-256:
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Variant source SHA-256:
`278c8bf6ec66d6406fb922fbce936f89a3917fa2b2c9f8099da1fa07ceda1e10`.
Variant artifact SHA-256:
`33168977b1d9a0c873a57d6c8e6bd2f87035847cd4619323277403549a14a358`.

The variant has 166317 source bytes / 4482787 bytecode bytes versus 160857 /
4138627 for the pinned caller wrapper: bytecode grows **8.32%** because both
paths remain present. This is a tradeoff, not a production-default recommendation.
Both artifacts use the maintained frontend and revision-14 codec. The final
helper's declaration-filter correction produces byte-identical full source to
the measured candidate. Exact executable and library hashes are verified.
No Cargo or machine-library rebuild is required.

Complete logical work changes from 875610515844 to **784281752407**, delta
**-91328763437**. Source operations and allocations differ by design. This is
not an instruction count and is not attributed to Engine changes. The real
262144 U16 tree / 1500513 U32 cache, depth 300, nice 258, near-EOF limits,
optimization passes, other tables, six unused initializers, path/output reserves
and clear/repetition boundaries remain. Cost-array construction allocation
sites change explicitly in this variant.

## Functional evidence and dispositions

The final focused cohort has **six passing Rust tests**, including unchanged
pinned-source assembly, original and variant reference cost models, original
statistics and signed-wrap checks, and vector-boundary comparisons. Original/
decoded forms use the same reference JSON values and finite execution allowances.
The boundary test covers preserved nonzero prefixes, suffix starts, long and
short vectors, zero/late starts, empty lookup arrays, arithmetic overflow, input
aliases, unchanged prior output identities on failure, complete cleanup and
successful retry. General 2D matrices are not qualified for this experiment.
The inherited compiler's 109-test receipt remains separate, not counted fresh.

In the default-6-109 original-artifact component case, allocation requests fall
**43980 to 14601**, bytes **1773909 to 633848**, while exact output remains.
Other code-derived and adjusted cases also agree; raw diagnostic rows retain all
values. These System-allocator observations are distinct from primary mimalloc
whole-compression timings. Their speed changes must not be extrapolated into a
large-input or whole-program result.

Setup/authoring failures are retained:

- Direct row-comprehension/general-matrix branch joining fails the existing exact
  source-schema requirement. A nominal-record join and three small adaptation
  trials show the same restriction. The final vector fallback uses an explicit
  row comprehension. No compiler/type-system patch is added for this experiment.
- A new Rust fixture initially passes Vec where boxed matrix data is required;
  the conversion is corrected. This is an introduced test-authoring error.
- The source helper initially mistakes a top-level call for a second definition.
  It now requires the pinned typed declaration. The timed full source is unchanged
  by this fix, and the focused bounds test passes afterward.

All twelve static routing/catalog/layout/architecture/inventory/warning/unsafe/
format checks pass on f8d62152. The later empty-lookup addition changes tests only;
its focused rerun and formatting/whitespace checks are recorded separately.
The unsafe allowlist, warning policy, compiler features and all production Rust
remain unchanged. Scalar performance controls are not rerun: the compiler binary
and libraries are exactly identical for both subjects, with their prior protected
controls retained. Linked/generated applications and larger complete-input
correctness/performance are not refreshed for this optional source variant.
CI is cancelled, #973 stays draft/unmerged and parity remains unmet.

## The next compiler target

The full-program stack analysis explains why this source experiment is modest.
In the pinned program's 1370-sample profile, ordinary argument-frame handling is
an ancestor of **600 samples (43.8%)**. Its immediate children include prepared
frame admission (292) and repeated immutable Schema::key hashing (177). The latter
comes from NativeProgramType::schema_key recomputing definition.key on every
projection. Runtime visits have 236 inclusive samples. The variant profile still
has frame handling in 656/1492 samples (44.0%). Counts overlap; they are diagnostic
ancestry, not additive exact time percentages or independently resolved JIT code.

The next substantial work should target ordinary argument/frame transport in the
pinned source: remove repeated type/geometry work and ultimately whole frame/
helper transactions where the compiler proves their contents and ownership.
Preserve fresh arguments, shapes, actual aliases, finite admission, error order,
publication and recovery. A larger native-region change needs a minimal real
read-only buffer-loop witness and explicit acceptance before implementation.
The source variant remains a separately measured experiment; further source
polishing is not the principal next target.
