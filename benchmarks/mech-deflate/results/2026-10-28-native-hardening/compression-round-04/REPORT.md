# Round 04: reuse the owned turn template

Base: `88b9b89a3a425c9842e1e3cbf33bd914c6f48ebe`.
Protocol: `e4da27073c953ffc0aa50a758718117aae4a02f8`.
Measured implementation: `1e0f5ea48aa4fc0fdff456cf8b113f4a065ef8cb`.
Stack parent remains `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
The [acceptance protocol](../../../optimization-rounds/owned-turn-template.md)
precedes implementation and all primary observations.

## Eight-byte comparison

Apple M1, level 12, input `abcdefgh`, **Roc = 1x**:

| Subject | Median seconds | Divided by fresh Roc |
| --- | ---: | ---: |
| Pinned reference, 20000 samples | 0.000288 | 1x |
| Before, published runtime | 0.517336500 | 1796.31x |
| After, owned-template reuse | 0.490266166 | 1702.31x |

After/before is 0.947674: **5.23% less elapsed time**, 1.055x the before
throughput. Each Mech subject has three retained observations, plus excluded
warmup and extra calls. The previous 1904.05x anchor used a 0.000275-second
reference; both rows here use the same newly measured denominator. This is a
small diagnostic, not full-input qualification or parity.

Both subjects use the same selected-operation build profile, unchanged timing
harness, pinned mimalloc object and source/artifact. Only evaluate_into is
timed. JIT, output reserve/clear/rebind, verification, slots and disposal remain
excluded. No compiler or other compression process overlapped primary timing.
Normal system/user activity was not isolated. No larger input was run.

Before executable: `44ad75d2536c5975058933a97a1703de209930931dcf5bc7dcb6e98ece01c2e8`.
After executable: `6a9080cdaa8943c370253555ca33a39e2d001cceb5cc7052cbee875b4604790f`.
Harness: `8595dfc8b529d5ad06b57580314e3fe4fc6110bd161ffa768a3d59e6c6608fd2`.
Source: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
Artifact: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Every call preserves work **941491317987**, the exact ten raw bytes,
independent inflation, input aliases, output capacity 13 and zero terminal
managed bytes/nodes. Algorithm, 25 source components, tables, search/EOF limits,
real buffers, six Huffman initializers and allocation sites remain unchanged.

## Change and allocation evidence

Both turn derivation callers already own a local CallMemoryPlan copy. The
helper formerly copied it again to temporarily suppress its target limits
during the existing core derivation. It now borrows that local copy, takes the
limits, calls the same planner, and restores the limits before returning either
success or error. The shared program template stays immutable. Returned target,
live regions/footprints, published-side accounting, finite admission, errors and
resource charges are unchanged. No new owner, public API or executor is added.

All 24 unchanged real-buffer diagnostic observations pass. Eight stores into
either real buffer use **8833 allocation requests versus 9913**, eliminating
1080 requests (10.89%) and 124608 requested bytes. U16 requested bytes become
5177904; U32 become 49000016. Reported work is unchanged. Two retained diagnostic
observations give 3.268 ms for U16 versus 3.292 ms before, and 18.882 ms for U32
versus 18.687 ms before. Physical copies remain; the diagnostic does not show a
cache-store elapsed improvement. These System
allocator/probe measurements are separate from complete compression timing.

The separate two-second profile contains 1355 main-thread samples. All nine
profiling repetitions, warmup and extra call are excluded from the primary
median. The profile still shows allocation/freeing, budget evaluation, copying
and snapshot length encoding. Named inclusive-family counts and the raw sample
are retained; overlapping counts are not independent elapsed fractions.

## Validation and review scope

**120 distinct Rust tests pass**: two focused restoration/current-payload/retry
tests; 16 program memory-plan, 32 memory-runtime and five activation tests;
54 existing component tests; eight native scalar and three bit-operation tests.
The component selection preserves every maintained body, expected value and
allowance. The generated test wrapper only makes module paths absolute and
excludes the linked subprocess module; its source, generator and exact linked
libraries are archived. The first wrapper compile omitted explicit sha2 and
grapheme externs; that failed receipt remains. Adding those existing Cargo
artifacts fixes compilation without rebuilding a library or changing tests.
The native all-primitives case remains explicitly skipped because this catalog
omits not/xor. Linked subprocess and runtime-only application coverage was not
rerun. The two focused tests ran immediately before the identical files were
committed; the remaining suites ran from the clean implementation commit.

Fresh source compilation and decoded re-encoding exactly reproduce the pinned
4138627-byte artifact. All twelve architecture/catalog/routing/inventory,
warning, unsafe-boundary and format commands pass. Protected original/decoded
Bool/U64/I64/F64 controls retain exact allocation requests/bytes; after/before
medians range 0.9930–1.0145, meeting the predeclared 1.05 gate. Scalar-loop
controls retain zero allocation slope at zero/64 visits. Every raw observation
and command is retained.

The selected release rebuild took **450.57 seconds**. Cargo marked Core,
Syntax and every selected machine library (including math) fresh. This is an
iteration build receipt, not a clean-build comparison.

Review of the preceding published head found an existing unsigned matrix-read
ordering defect for multiple rows and columns. The retained U64 multi-row test
also used the wrong direct-storage expectation. Its passing result in this
cohort does not qualify multi-row ordering. That finding is handled by a
separate successor commit with a resident oracle. The compressor's row buffers
do not encounter the differing ordering. This PR remains draft; no green-CI,
clean-review, full-corpus or merge-readiness claim is made.
