# Investigate cleanup at ordinary private-frame actions

Starting published head: `4686620853914517474a23e9d8a6ff1cf11db5cf`.
No candidate optimization is enabled yet.

The round-36 complete eight-byte profile has 384/1339 disjoint exclusive weights
under frame actions, including 50 in the ordinary buffer-frame action helper.
That helper still retires each last-use frame/arena slot with an individual
shared-ledger debit, unlike the newly batched lexical tail cleanup. Pack,
projection and copy are reachable operations in the real Huffman parent.

First observe the existing Huffman source witness's reached action retirement
inventory in original/decoded repeated invocations. Keep its exact [3,1,3,2]
output, 91452918 work and zero cleanup. If it reaches multiple contributions,
investigate reusing the existing exact paid receipt for this inventory. Preserve
its different original semantics: frame retirement OR arena retirement per slot,
with no added lexical scan fee. Unsupported/short/scoped/worker cases must keep
the original ordered entries and exact spent prefix. Do not enable a wider path
without the current-state/error-boundary witness.

Keep the compressor source, generated program, allocator and timing boundary
unchanged. Compare complete eight-byte compression against the frozen round-36
worker, report both the common fresh reference and historical 0.300 ms ruler,
and commit every observation cohort immediately. No larger-input timing.
