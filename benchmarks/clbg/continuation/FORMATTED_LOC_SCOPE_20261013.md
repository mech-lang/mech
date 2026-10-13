# Updated benchmark scope, 2026-10-13

The user removed Red. Rebol remains requested. The publication additions are
Nim, Crystal, Pony, C3, Gleam, Clojure, Prolog and Rebol, with platform filters
and a separate Julia AOT measurement. V remains a text report only and must not
enter the dataset, filters, charts or Site. Existing authorized benchmark work
continues serially under `/tmp/mech-clbg-timing.lock`.

The user also requested a second X-axis measure: formatted LOC. Run the exact
scored authored source through its language's native formatter when one exists
and retain the resulting line count. A language without a native formatter
has a blank formatted-LOC entry. Existing canonical LOC remains available.

Implementation is in progress. Formatting must operate on separate copies of
the exact scored application/helper bundle; it must never rewrite scored source
or change its measured executable/timings. Retain actual formatter command,
version, configuration, original and formatted source identities and output.
Use the existing source-line counting definition after formatting, and document
the definition in the UI. Do not silently fall back to another LOC measure or
replace a missing native formatter with an unrelated formatter. If any counted
helper cannot be natively formatted, the complete bundle total remains blank.

Unavailable, pending and failed formatter evidence may all have a blank numeric
entry, but their reason must remain distinct in evidence and source details.
Formatting, version probes, output hashes and checks share the timing mutex;
they cannot contend with an active benchmark. Formatter outputs are a second
source-size measure, not new performance measurements or language optimizations.

At this source checkpoint, no formatted-LOC output or numeric value is admitted,
no new axis is published, and no Site update has occurred.
