# Matched linked executable comparison

The early JIT checks are retained as preliminary observations. Their initial
candidate changes direction on repeat: +2.75225% against its first baseline,
then 4.388104 ms against a 4.606896 ms baseline. The refined check is
4.426937 ms, 15.05761× the same second 0.294 ms reference. Do not select or
credit either compiler change from those sequential checks alone.

Use a new **matched linked-compressor configuration for all three versions**:
published round-38 source dc77024f2, initial selection 552e537fc, and transport
profitability refinement 892275fec. Each uses its own same-build pinned Object,
descriptor, identity and eight identical-profile frozen libraries. Seven library
hashes remain unchanged; only the native host changes. The worker bodies are
byte-identical after normalizing the exact image path and identity symbol. The
public checked linked-image API and the original invocation boundary are reused.
All three already pass two smoke calls with exact bytes, independent inflation,
input aliases/contents, capacity 13, expected work and final zero cleanup.

This keeps full-compressor compilation out of each worker and avoids retaining
three ~16.9 GB JIT preparation footprints. The same two tiny seed/clear helpers
retain JIT preparation outside compression. It is not a minimal runtime-only
binary, an algorithm/source/allocation change, or a JIT-to-linked speed gain.
Establish a new linked baseline and compare only within this configuration.

Four independently prepared executable triplets each run 16 blocks × 31 calls
per version, with rotating within-block order. All three are ready before any
timed call; only the requested worker executes and peers block on stdin. This
gives **5952 primary calls / 5976 including warm-up and final calls**, plus one
shared 20,000-call reference. Blocks share four prepared triplets and are
descriptive, not 64 independent executable preparations. No build, test, audit,
profile or instrumented diagnostic overlaps primary measurement. A live worker
observation timeout does not restart or terminate it.

The `evaluate_into` timer line, source and bytecode hashes, real 262144 U16 tree,
1500513 U32 cache, 305257 U64 path capacity, original depth/nice/EOF behavior,
pinned mimalloc object and output reuse remain unchanged. Every call checks the
same ten raw bytes, independent inflation, aliases, capacity and work; every
worker checks cleanup. Record every group immediately and report all three
pairwise comparisons, fresh-reference multiples and the fixed 0.300 ms ruler.

Choose from balanced results and actual instruction/allocation counters. Keep a
proven small gain; if the first candidate beats the refinement, retain that
evidence and do not discard it because of the earlier noisy check. Describe
mixed results as uncertain. No larger-input timing, CI wait or parity claim.
