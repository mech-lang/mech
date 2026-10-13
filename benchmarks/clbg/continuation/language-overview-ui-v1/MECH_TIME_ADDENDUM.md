# Recorded Mech time comparison — source addendum

Source-only change to canonical app.mjs/index.html. The ready native adapter
copies `matched_mech_control` from its actual matched-control record and retains
the candidate's recorded `time_ratio_to_matched_mech`.

The inspector now displays that recorded ratio and control `median_seconds`
when the existing `mech_comparison` object is absent and the measured program,
ratio and control median are positive finite numbers. This fallback adds no LOC
comparison or control source count, and derives no ratio from chart baselines.

The language-overview table now has a Time × Mech column. It uses an existing
positive finite `mech_comparison.time_x_mech`, or the recorded paired ratio with
a positive finite actual matched-control median. Pending programs or missing
bindings display a blank. All dataset values, source identities, canonical
baselines and existing comparison behavior are preserved.

No imports, parsing, hashes, tests, browser, benchmarks, Git or Site copy was
performed. Prior frozen review notes were retained. Root owns the next guarded
source check, copy and browser verification.
