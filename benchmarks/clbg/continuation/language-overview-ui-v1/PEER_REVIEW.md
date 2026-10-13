# Independent language-overview source review

Source review accepted after the corrections below. No imports, parsers, hashes,
tests, browser, formatter, benchmark, Git or Site operation was performed. Root's
guarded syntax, dataset and browser QA remain pending. The original frozen
REVIEW.md and baseline files were retained.

- Per-language coordinate medians now use `2 ** median(log2(coordinate))` for
  positive benchmark endpoints. Across-benchmark covariance retains its original
  shape/area and is translated to that median; recorded run-to-run SD stays
  separate.
- Explicit row/inspector Compare uses a separate view containing that exact
  qualified program as the chosen variant's endpoint pool. Reference endpoints,
  canonical data/programs/groups and normalization baselines are retained.
  The selected source id is labelled, its actual runs are shown and the domain
  includes those runs and its median, including coordinates below 1x.
- Comparison return restores the exact originating source, file index and
  inspector-open state. Comparing a row while another source is open cannot
  bookmark that other source. Overview variant/workload controls persist through
  render, and Browse opens a source from those selections.
- Catalog identities deduplicate by dictionary key. Conflicting shared measured
  identities, unknown source files, malformed coordinates and incomplete timed
  sample arrays reject instead of silently hiding a conflicting row. Pending
  medians, runs, SD and Mech ratios stay blank. Missing comparison timings are
  stated persistently in the overview summary.
- Clickable chart labels populate the fixed above-chart hover strip on keyboard
  focus. The existing strip, responsive layout and safe-centering CSS remain.
  Method copy distinguishes the normal endpoint pool from the overview's
  retained qualified inventory.

Reviewed canonical app.mjs, index.html and style.css are released to root for
copying into the already opened Site checkout and guarded QA. Only app.mjs and
index.html changed during this peer pass; style.css was reviewed without edits.
The separate formatted LOC widget still needs a later composition and was not
merged into the canonical UI by this work.

Root QA should check the log-coordinate median for an even ten-point cohort,
the 20 Crystal source rows and 60 candidate runs (the full 90-process evidence
also includes 30 paired Mech controls), an inventory-only comparison and its
return source file, missing coverage, previous-chart filter/year restoration,
all visible run/contour extents, mobile layout and keyboard navigation.
