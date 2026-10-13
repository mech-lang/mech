# Release-year window source candidate

This fresh directory contains only the six currently published UI source files,
with the release-window change in app/index/style/math, focused source assertions
and this review. There is no data.json copy. traits.mjs and mech-palette.css are
unchanged copies of the current canonical publication. No hashes, JavaScript
execution, syntax checks, fixtures, browser/server, freezing, dataset change,
native benchmark or publication has occurred for this candidate.

Two overlaid native range inputs share the actual published language release-year
span. Both are independently keyboard-focusable, have descriptive accessible
labels, inclusive year values and visible thumb focus. The lower handle clamps
at the upper handle and the upper handle clamps at the lower, allowing a
single-year window without a crossed interval. The highlighted track and
From/To labels show the selected span; All restores the full default span.

Eight buttons below the track select1950s,1960s,1970s,1980s,1990s,2000s,2010s
and2020s. Presets intersect the actual dataset span, so2020s ends at the actual
upper dataset year and the first partial decade starts at the earliest published
release. Out-of-span decades are disabled. The exact chosen preset receives
aria-pressed=true, while custom spans retain their explicit year output.

Filtering includes both endpoints. No language's release year/source/note is
changed. Median history starts at the lower year and excludes every older
language from each later cohort; a window with no matching release has an empty
history and disables playback. Playback steps only through matching
published release years inside the selected window. Its separate cursor leaves
both handles fixed, supports pause/resume and displays its cutoff independently
of the selected span. A manual handle/preset/All change clears that cursor.
Completing playback restores the full selected window, never a crossed range.

Language overview, explicit source navigation, comparison/return controls,
current resident timing policy, static hover strip, all measured samples,
canonical LOC, fixed LOC/time baselines and ×best displays remain as published.
Overview/Mech navigation intentionally starts with the full window, and returning
restores the saved lower/upper/cursor state. Existing single-upper-year API is
retained with an uncrossed upper adjustment; setReleaseWindow is additive.
The math-only domain remains based on the admitted cohort and does not move when
the year window is adjusted.

The matching change is also retained in combined-ui-v1, preserving its separately
reviewed formatted/platform behavior. This independent directory permits the
user's window request to publish against the current data while those additional
evidence gates finish. Root must independently source-review/freeze the exact
candidate, run focused assertions and syntax checks under the common mutex, then
browser-check mouse/keyboard handles, decades/full span, pause/resume/bounds,
language overview/source/comparison return, fixed ×best/hover and mobile layout.
Root owns any live staging, Git checkpoint and GitLab Pages publication.
