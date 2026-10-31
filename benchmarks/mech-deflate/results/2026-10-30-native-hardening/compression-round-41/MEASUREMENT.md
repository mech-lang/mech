# Managed projection transport: measurement fixed before primary execution

Compare the frozen retained linked compressor from source
`552e537fcbcfc3811ce442c982010b5e1e999e6c` against the current same-build
linked Object/image/libraries. The implementation is
`0e4ff5d90f65c77becdec29a16424190522dd855`; subsequent changes add protocol
tests only. The
release receipt pins the exact build head and eight libraries. All seven
non-host library hashes must remain identical to baseline.

Use the unchanged whole level-12 compressor, eight input bytes `abcdefgh`,
exact ten raw bytes, independent inflation, real buffers, depth 300, nice 258,
near-EOF rules, original allocation sites, pinned mimalloc 2.1.7 and original
`evaluate_into` timer. Seed/clear/rebind, output slot preparation and validation
remain outside that timer, with output capacity 13 retained and reused.

Four independently prepared executable pairs, each with 16 alternating-order
blocks of 31 calls per version, give 1,984 primary observations per version
(3,968 total). Each version has one warm-up and one final diagnostic call per
pair: 3,984 calls including those. The two worker bodies normalize identically
apart from same-build binding paths/identity. Both workers are ready before
measurement; only the requested worker executes a call. No build, test, audit,
instrumented diagnostic or profile overlaps primary elapsed observations.

Run the original 20,000-call reference once before the pairs and use that common
median as 1x for both versions. Also retain the fixed historical 0.300 ms ruler.
Commit the reference, every pair and the aggregate immediately. Blocks within
one preparation are descriptive, not independent executable preparations.
Do not replace a live worker when a poll expires.

After this first primary comparison, collect hardware counters, allocations
and fresh sampling separately in the same linked configuration. Preserve exact
work, alias behavior, prior publication, cleanup/retry, ordinary per-field
fallback and scalar control efficiency. Retain a demonstrated positive small
gain; a counter or allocation decrease alone does not establish elapsed gain.
No larger input, full-size run or parity claim is authorized by this case.
