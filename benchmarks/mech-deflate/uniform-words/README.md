# Ordinary repeated unsigned collection construction

The real tree and match cache need 262144 U16 entries and 1500513 U32 words.
The pinned source, revision `671077457ff7ef3a3d44caf715a3fc595addf379`,
initializes them with `List.repeat` at `Matchfinder.roc:36` and
`CompressOptimal.roc:718`. Their Mech comprehensions previously executed about
12.3 million generated control visits before any match finding.

The preserved optimized baseline is in
[`2026-10-26-fd790ca-reality-check`](../results/2026-10-26-fd790ca-reality-check/REPORT.md).
Initialization and returning the state took a median 657.148 ms. A three-second
sample of that exact executable located the main cost in generated control
visits, helper dispatch and resource-ledger operations. Physical copying was a
smaller sampled cost. This is evidence for one ordinary collection-construction
improvement, not a profile of a complete compressor.

## Source requirement and selection

The minimal source is unchanged ordinary Mech syntax:

```mech
repeat(value<u32>, count<u64>) => (out<[u32]>) :=
  out := [value | i <- 0u64..=count, i < count].
repeat(word<u32>, count<u64>)
```

Shared `NativeProgramFacts` recognize this exact pure region: one unstepped
inclusive U64 range starting at zero, one `i < count` filter, and an unsigned
word already defined before collection initialization. The true arm contains
one push; the false arm is empty. The element and condition cannot escape the
region. Calls, assertions, extra filters, varying yields and other source shapes
retain their ordinary generated CFG. Widths U8, U16, U32 and U64 use the same
selection. Names, zero values and benchmark dimensions do not select it.

## Execution and accounting

Original range preparation executes first, including inclusive-range overflow.
The shared CFG emitter then attempts one collection-helper call at that exact
fresh header. On admission it reserves the existing packed builder once and
fills it with the current word and count. It exits through the original
collection finish and publication code. No extra frame lane, entry point or
alternate evaluator is introduced; scalar code generation is unchanged.

Selection checks complete construction and finish geometry against finite work,
copy, output, temporary, retained-node and managed-memory allowances before any
payload allocation. A pure eligibility decline retains the original loop and
its failure order. Execution does not catch and retry a failed kernel or
allocator. Byte work and copy limits remain charged; removed per-element control
visits are no longer executed or billed. Work policy 82 records that change.

The fact is derived from the validated plan during preparation. Build-owned
descriptor restoration derives it again from that same plan; serialized input
cannot independently grant the optimization. Typed IR 107 and program ABI 35
reject Objects from the previous helper protocol. Artifact bytecode revision
and fixture source programs are unchanged.

## Acceptance and limits

The new `uniform_words_qualification` tests cover all four widths, zero and
nonzero words, original/decoded artifacts, small resident/native agreement,
fresh arguments in repeated calls, range overflow, selected assertions, clone
and output failure order, unchanged prior aliases, cleanup and successful retry.
The existing real-buffer constructor test retains its original values and
allowances. The linked qualification runs a constructed buffer before a lexical
assertion, with success, rejection before division, and successful recovery.

The real-buffer initialization and six-position collector witnesses retain the
same sources, complete reference arrays and limits. This change does not provide
retained-capacity append/clear, complete block collection, parsing, block writing
or `compress_into`. The six-position witness remains an acceptance failure until
it finishes under its existing policy; measuring a rejection does not establish
a compression time.
