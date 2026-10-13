# Nested C-family data braces after actual job18

Actual job18 leaves the fastest nbody C++ source
`nbody/upstream/nbody-gpp-5.cpp` (identity
`90f5f496693e27c210bb9d29e84409c219c3c33ccd151910b07aaace9596d471:cpp`)
with a bare-block diagnostic at line161, inside `Body system[] = {
{{}, {}, SOLAR_MASS}, jupiter, ...};`. The scope mapper already classifies
these nested braces as data. The canonical diagnostic was triggered solely
because an outer function scope exists and the nested row has an empty
prefix, without checking the immediate known data-brace parent.

The working repair suppresses that diagnostic only for an opener already
classified as data whose immediate parent is also a C/C++ data brace. It
changes no scope classification, counting event, source text or literal row.
Explicit executable callbacks still take the existing code path, and complex
callback diagnostics remain. A GNU `({ ... })` group has a parenthesis group
as immediate parent and receives no new data exemption. A bounded new scope
diagnostic explicitly retains review for that written statement-expression
form, whose body must not be accepted as aggregate data. No new numeric
count or executable classification is claimed for the unreviewed GNU body.
Ordinary executable
lexical blocks keep their headers/body/closers under the existing rule.

Seven proposed fixture methods cover C/C++ aggregate rows, nested empty
aggregates, ordinary lexical blocks, explicit/unknown callbacks and GNU
statement expressions. Their semantic expected counts are hand proposals;
they have not executed. Syntax qualification and the next guarded full-pool
pass remain required. Frozen job18, all old source/count/timing records and
chart hooks remain unchanged. No score is admitted by this source review.
