# Combined explorer source candidate

This additive directory composes the currently published language overview,
fixed hover strip and best-time interface with the reviewed formatted-LOC
projection and platform-v3 traits. It has not been imported, parsed, hashed,
tested, frozen, browser-qualified or deployed. Current canonical UI, Site,
dataset and frozen formatter packet are unchanged. No data file is copied
here; root must supply the independently qualified publication candidate.

The app keeps the current complete program catalogue, source navigation,
language/family overview, all actual timing runs, per-program run SD, coordinate
medians across benchmarks, covariance, selected-source comparisons and both
return controls. Source comparisons remain a pure view copy with the actual
selected inventory program, rather than substituting a canonical winner.
Metric changes do not reset language, trait, year, program, profile, inspector,
comparison or overview state. Returning restores the saved metric as well as
the original chart controls. Time ratios remain measured seconds divided by
the benchmark's fixed best-time baseline in both table and inspector.
Resident measurement columns use one display predicate accepting either the
original `execution_mode: resident` or the legacy adapter's preserved
`prior_runtime_metadata.fields.execution_mode: resident`. This retains actual
whole-process sample columns and sorting when qualified platform metadata uses
the execution category `interpreted`; source and timing samples stay unchanged.
Browser QA must verify these columns after actual legacy metadata integration.

Canonical LOC uses original counts and coordinates. Formatted LOC requires a
complete qualified positive integer whole-program total and independently
published positive formatted X. Missing counts/coordinates never fall back to
canonical LOC. The pure projection covers both chart and full source inventory.
Comparison maths rejects any timed point lacking a positive finite LOC/X/Y,
so incomplete formatted records cannot produce NaN geometric means or curves.
Canonical admitted coordinates already satisfy this condition and retain their
existing statistics. The saved canonical LOC/time baselines and source data
are never rewritten.

Language overview tables keep all published programs, original timings, runs
and SD even when formatted coordinates are unavailable. Canonical and formatted
counts have separate columns; blank formatted counts show their status. The
chart uses only eligible coordinates, and its summary labels the omitted
programs. Original source inspection remains available for an unavailable
formatted endpoint. The inspector shows both raw counts and the exact blank
reason while retaining its canonical audit/source and original timing samples.
Explicit program comparison cannot invent a coordinate for a pending source.

Metric-specific domain and release-history caches include the metric and exact
selected-source identity. Language and comparison domains include every
eligible inventory program and its actual positive runs, preserving points
below the median baseline. Y limits use the common envelope of canonical and
formatted modes, keeping a slower formatted-small endpoint from being clipped
on a switch. Domain calculations remain independent of visibility filters.
Language covariance centers retain the published log-coordinate median rule;
run-to-run SD remains a separate measurement.

The unmodified platform-v3 `platform.mjs` and `traits.mjs` provide Platform
choices JVM, BEAM, .NET, Native, JavaScript and Unclassified. Their producer
evidence gates are preserved: exact pinned report/runtime receipts, original
source hashes, verified backend/mode, and separate historical confidence.
Language names and source extensions never classify a platform. The app passes
the complete validated source catalogue to traits, so inventory-only measured
forms also participate in backend consistency. These facts are independent of
the selected LOC metric, endpoints and display filters. Unknown/mixed platform
evidence remains Unclassified. Explicit measured memory policy gates are also
preserved. Root's actual platform-data qualification must populate evidence
before this UI can expose a named category; the UI itself creates no facts.
Shared inventory/chart identities also require identical `measured_backend`
and `execution_mode` records before merging. A conflicting or missing shared
runtime record stops loading rather than being replaced by the chart copy.

Hover information remains in the existing fixed-height scrollable strip above
the SVG. Pointer and keyboard focus populate that strip without moving it.
There is no floating tooltip. The source metric selector and formatted status
note remain outside the SVG. Existing accessibility labels, language buttons,
source/run keyboard interactions, focus restoration and safe-centered hover
content are retained. The four inspector statistics use two columns.

`test_combined_axis.mjs` is a source-authored assertion script for projection
of full inventory, unavailable/coordinate-missing exclusion, preservation of
canonical counts/time/source/samples, formatted endpoint promotion, finite
empty domains, unavailable original-source inspection and qualified JVM filter
semantics including a conflicting hidden inventory form. It has not run.
Root's next common-lock phase must run this integration script and syntax
checks for the changed frontend, then browser-check metric toggling, filtering,
full language overview/navigation, selected-source comparison/return, runs,
medians/SD, hover/focus and narrow-screen layout with the actual candidate data.
Reuse the exact completed platform-v3/legacy-platform10 and formatted23 fixture
receipts and actual Crystal20 formatter result; no unchanged fixture or formatter
rerun is needed. The data producer must keep backend/mode records synchronized
between shared inventory/chart identities. Browser QA must also confirm that a
conflicting shared backend record produces the explicit load error. No new
fixture framework or UI qualification is claimed by this source copy.

The later source-only release-window addition replaces the single year cutoff
with two accessible overlaid native range inputs. Default spans all published
release years; either handle clamps at the other, and1950s through2020s presets
intersect the actual dataset span. From/To output, highlighted track and pressed
preset state identify the inclusive selection. Playback keeps the handles fixed
and uses a separate visible cutoff inside that window, with pause/resume and an
empty-window guard. Manual window selection clears that cursor. Both filtering
and median history exclude releases before the lower bound. Saved overview
return state includes both endpoints and the cursor. Existing formatted axis,
platform/resident/source navigation/hover/×best fixes are retained. Added window
assertions are unexecuted and await root's common-session QA. The matching
year-window-v1 candidate can publish this request against current canonical
data independently of the formatted/platform evidence qualification.

## Current chart view source synchronization

The released median-rankings-v1 year-window, coordinate-median/log-SD rankings, independent recorded-run Samples and full-grid Gaussian density behavior is now retained in this mutable combined UI. Existing formatted LOC projection/exclusions and platform/resident/inventory fixes remain. Rankings consume current projected eligible comparison entries; unavailable formatted coordinates supply no row or density/sample position, while original source/timing panels remain inspectable. Full-data fit includes actual runs from the unfiltered projected cohort; the shared canonical/formatted Y envelope is retained. Samples controls recorded runs exclusively in both directions: profiles off keeps actual samples; Samples off keeps enabled benchmark median marks and Mech pairs. No authored UI tests, parsing, hashing, browser, dataset or native execution occurred during this synchronization. Immediate canonical publication remains separate; root must coordinate new combined source review/QA before its later integration.

Changed combined app/style/math/traits/formatted-axis browser URLs use the distinct stable ?v=20261013-combined-views version. This avoids mixing cached immediate canonical math or trait exports with the later combined candidate. Module source contents and publication evidence remain unchanged by cache-version selection.

The existing unique source-metric selector and formatted omission note now sit
in the shared toolbar area outside the chart card. Median rankings hides only
that card, so canonical/formatted switching and the qualified-coordinate
explanation remain available in every view. The note explicitly covers ranking
exclusions and retained original source/timing records. Only index/CSS placement
and wording change; all existing selector handlers, saved metric/filter state,
projection, math and evidence gates remain exact. Browser QA must toggle the
metric while staying in Rankings and verify that the exclusion note appears
with Formatted LOC. The immediate canonical candidate and dist are unchanged.
