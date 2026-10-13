The original input-v2 preparation compiled compact pi-digits successfully,
then stopped compiling the optimized form. V 0.5.2 reports that
`strings.Builder.free` must be called inside an unsafe block. The published
source put `defer` inside `unsafe`; the deferred body does not inherit that
unsafe context. Original source copies, profile declarations and both compiler
receipts remain unchanged. No pi-digits output probe or full timing passed.

The sole optimized-source repair swaps these two block headers:

```v
defer {
    unsafe {
        sb.free()
    }
}
```

Every arbitrary-precision numerator, both quotient statements, production
matrix update, accepted-digit branch and output statement is unchanged.
Compact is an exact byte copy. New profiles and source copies have distinct
builder-v3 keys/paths; canonical LOC remains null pending the common audit.
The passed regex/reverse-complement sources, output records, binaries and
study manifests are not touched.

Root owns a separate versioned worker/selector and common-lock compilation,
complete canonical/edge output checks, and fresh two-extract trace evidence.
Previously compiled compact code alone is not output qualification. No build,
probe, full timing, or chart/builder admission is performed by this draft.
V remains report-only, with the exact existing standard-library algorithms,
strict compiler flags and native ownership/resource gates.
