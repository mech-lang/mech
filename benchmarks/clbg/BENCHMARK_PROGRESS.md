Benchmark progress as of **2026-10-13 22:37 UTC** (18:37 New York).

The previous remotely verified checkpoint is `40170427711079ebc789727f54b465104c0b879f`
on `codex/clbg-native-benchmarks`. The language overview and complete Crystal
inventory are published. C3's exact source counts, runtime metadata and current
chart candidate have qualified and are now published through GitLab Pages
at `https://mech-lang.org/benchmarks/`. Nim, Gleam and Clojure
finished and passed independent reconciliation of all 90 timing runs each.
The 23 formatted-LOC source fixtures and all 20 native Crystal formatting cases
passed; formatted-axis integration remains pending. Durable jobs preserve their
actual request, process identity and exit.

| Cohort | Actual full samples collected | Present state |
|---|---:|---|
| Crystal | 90/90 | Published with 20 source forms and 60 candidate samples, plus 30 matched Mech samples in the complete study. |
| C3 | 90/90 | Published through GitLab Pages. Source counts, runtime metadata and current dataset qualified; all 20 forms / 60 candidate samples passed browser inspection. |
| Nim | 90/90 | Published through GitLab Pages with all 20 forms and 60 candidate samples. Exact LOC, runtime metadata and current dataset checks passed; all 21 authored inputs, including GMP glue, are counted. |
| Gleam | 90/90 | Full-suite reconciliation and complete source admission passed: 20 bundles, 30 unique authored sources and 104 measured copies. Runtime publication metadata and chart integration remain. |
| Clojure | 90/90 | Published. Full-suite, LOC, JVM metadata and current dataset admission passed. Root verified 5,276 candidate pins; 20 forms / 60 candidate samples are retained, with 18 selected endpoints. Public browser inspection confirmed all 20 forms and 60 runs. |
| V | 90/90 | Full workload samples collected, including eight missing tree slots recovered additively. Independent all-ten reconciliation passed. Report-only; excluded from dataset, charts and Site. |
| Pony | 0/90 | Both tree observations and four Pi/map traces are retained. The existing-evidence capture exited normally; original authored main IR was overwritten by builtin output. Separate bitcode/ownership recovery remains. |
| Prolog | 0/90 | SWI-Prolog 10.0.2 ARM64 runtime qualified. Moving successful shutdown outside the catch wrapper fixed exit status; actual feature probe and seven-image closure passed. The 156 bounded workload cases, work/LOC checks and full samples remain. |
| Julia AOT | 0 qualified | Kernel-image binding repaired. Both bounded debugger calibration cells timed out; no native AOT helper executed. No permission cause is established. An official safe-trim/stripped-IR route is documented for the next additive attempt. |
| Rebol | 0/90 | Faithful source bundle and six runtime intake fixtures retained; actual SDK/runtime/build/work/LOC/full qualification remains. |
| Elixir / Kotlin | 0/90 each | Reviewed source bundles are frozen and pushed. Actual toolchain and complete qualification remain. |

These are snapshots of completed qualified sample records, not forecasts or
extrapolations. Each complete cohort comprises ten workloads, two source forms
and a fresh matched Mech control, with three rotated full samples (90 total).
Long JVM/BEAM/APL full runs and all heavy build/hash/normalization/Git work share
`/tmp/mech-clbg-timing.lock`; timing subjects never run concurrently.

Actual C3 three-sample medians from the completed full reports follow. The faster
C3 form is selected per workload. Independent reconciliation subsequently passed for all90 samples.
**1× means the matched Mech control; larger ratios mean slower execution.**

| Workload | C3 form | C3 seconds | Matched Mech seconds | C3 / Mech |
|---|---|---:|---:|---:|
| N-body | compact | 1.600197375 | 1.051185167 | 1.5223× |
| Spectral norm | optimized | 0.227356042 | 0.093663250 | 2.4274× |
| Regex redux | optimized | 4.221720042 | 0.505207917 | 8.3564× |
| Binary trees | optimized | 5.011552625 | 0.409757125 | 12.2305× |

The [Mech optimization history](MECH_OPTIMIZATION_HISTORY.md) already preserves
18 entries and 33 exact measured comparisons, with compiler/source/library
changes distinguished and regressions retained. Attribution to the Mojo work
on another computer remains unverified and is not invented. The overnight
comparison-language qualification work does not itself count as a new Mech
optimization.

Remaining publication work is complete cohort reconciliation, exact canonical
source-owner/count admission where pending, complete runtime metadata,
platform filters, Julia AOT measurement, dataset integration and browser QA.
The platform frontend candidate passed its checks. Historical metadata now
qualifies 120 platform and 97 execution annotations across 133 retained rows;
13 rows remain explicitly unclassified. Filters have not yet been integrated
into the published Site. Successful historical samples and
original failures remain immutable; additive repairs reuse successful evidence
instead of relabeling it as a fresh run.

## Nim publication and current qualification — 2026-10-13 22:37 UTC

Nim is live in website commit `b0082b0da35f7e41b5f4c056c6b1130922081ee8`;
GitLab Pages pipeline `2944511241` passed. The deployed data SHA-256 is
`9d9eba51742dcf1afc76c0a45fa4125524197ed06ae540a1d142769fd859f60a`.
It retains all previous measurements and baselines, adding 20 Nim programs and
60 candidate runs. The complete original study also retains 30 matched Mech
runs. The current site has 44 language families, 50 execution variants,
798 selected programs, 80 inventory entries and 2,478 recorded runs.
Root independently verified 41,944 candidate pins after normal completion.
Runtime metadata binds Nim 2.2.12, its C backend, Clang 22.1.8 and the actual
ORC configuration, with the saved PCRE/GMP dependencies.

Local and public browser inspection confirmed all 20 Nim source forms and
60 runs. Optimized Pi includes its authored GMP adapter: 61 main units plus
23 helper units, totaling 84 canonical LOC. Samples Highlight shows 2,478
recorded marks. Samples Off shows zero recorded marks and zero background
program endpoints, while preserving the 99 aggregate endpoints. The live app
still matches the published sample-cloud fix; no UI code changed in this data
release. There were no browser console errors. Screenshot capture was
unavailable, so the saved QA report records DOM observations. Publication
receipts are in `continuation/jobs/nim-chart-publication-v1/`.

Platform history v4 passed ten literal-route fixtures and qualified all 133
retained row identities. It resolves five earlier literal proof cases, yielding
120 platform annotations, 97 execution annotations and 114 byte-verified
historical rows. Twelve Julia rows lack a saved runtime observation; one C
FASTA row lacks native-build binding. These 13 stay pending. The qualified
history SHA is `cfea973a22951b7efef504ba48f84b07745397e7dc8df71161b3a01182b026b9`.
The combined formatted-LOC/platform integration remains a source candidate.

Gleam's first semantic seal failed before source processing because a historical
helper worker was recorded with the Framework Python image, while the checker
assumed every initial listing used the launcher. The failed output is retained.
An independently reviewed three-job literal mapping fixed that assumption;
the subsequent metadata-only seal exited normally. Root verified 6,896 pins,
all 20 complete source bundles and both regex totals (130 and 167). The
qualified overlay SHA is
`dd51c9a33d0eade6b5f4f94266ad1561b0f36770ac03a9c1691d2f19cbc04fce`.
Original regex diagnostics remain preserved alongside the four exact semantic
resolutions; no parser, successful fixture, native runtime or timing was rerun.
Runtime metadata and the current chart candidate remain to be completed.

The Prolog preparation revision is fully source-reviewed and frozen, but has
not run. Pony's additive bitcode/ARM recovery revision is released for review;
it has not run. No new benchmark timings were collected during this update.

## Chart controls and statistical views — first publication

The inclusive two-handle release-year window, eight decade presets, Speed/LOC
rankings and full-chart density map are live at https://mech-lang.org/benchmarks/.
Ranking bands show signed distance from the current coordinate median in units
of the sample standard deviation of log2 values, extending beyond 3σ when needed.
This first density version used equally weighted language variants over a fixed
48×36 grid. It is superseded by the actual-run density layer described below.

All 2,364 recorded runs remain visible with both profile controls off. Hiding
Samples preserves the profile medians. Equal-year slider handles remain
independently reachable, and source navigation restores the selected year window.
Focused Node assertions, syntax checks, local browser checks and phone-width
layout checks passed. The public browser confirmed the new views, all 2,364 runs,
48 ranked variants and all 1,728 density cells, with no console errors.

Website commit `1aa753d45d819de0436c8dcf321118f5f7a7db1e` is pushed to `main`;
GitLab Pages pipeline `2944267125` passed. All 29 deployed assets match, with the
changed modules checked at the exact versioned URLs requested by the page.
Earlier rollout/cache mismatches are retained in the publication evidence.
Actual receipts are in `continuation/jobs/chart-views-publication-v1/`.
No benchmark was rerun. The published timing dataset remains
`4fe402c21360fbbbe9d650c15d1fd4c62422c17cfad8cafd24721fe7f0bd2947`.

Legacy platform metadata qualification for 133 existing rows failed on a literal
historical `chart_key` alias. Review also found that identical binary bytes could
have different authored source identities; the additive repair matches the full
source bundle as well. The failed attempt is preserved and contributes no
platform annotations.

## Clojure admission and composable chart layers — 2026-10-13 21:05 UTC

The Clojure candidate exited normally and independently passed physical evidence
and preservation checks. It adds all 20 source forms and 60 candidate samples;
the existing full study also contains 30 matched Mech samples. Prior records,
timings and baselines are unchanged. Candidate SHA is
`b088279074e85ec149466f876bdce49d4d497f7784a0d48be58e023e5b2c6a13`.
Local browser checks passed: all 20 forms, 60 language runs, both inventory-only
programs, regex and its counted helper are inspectable. Rankings show 49 variants
and the ordinary chart retains 2,418 samples. Website commit
`8ab71934cf26f84145ff66b538fd7236be751b91` is pushed and Pages pipeline
`2944288319` passed. The deployed data matches by SHA-256. The normal browser
initially reused earlier cached data; a subsequent reload confirmed all 20 forms
and 60 runs on the public site. The next UI change also adds data revalidation.
The site now has 43 language families and 49 execution variants.
No benchmark or successful fixture was rerun.

The composable layer update is now published in website commit
`224d1b1505037c7c92e67038a82082cc763bef0b`; GitLab Pages pipeline
`2944356909` passed. Samples, Medians and Density each have independent
Off / Dim / Highlight controls. The density layer counts actual recorded runs
with unit weight per program/run, retaining repeated identical observations.
It stays unchanged when dots, medians, profiles or global summaries are hidden.
Language, benchmark and source selections determine its data.

The restored Axes control is always available in the toolbar: Medians gives
the compact summary range; All samples zooms out to all recorded runs.
Visibility changes neither hide this control nor alter the selected range.
Density fills the chosen domain. Rankings now consists of two compact 1D
charts, Speed and LOC, with median centers, signed σ bands, one-decimal
summaries, collision-separated language dots and fixed external readouts.
Clicking or pressing Enter on a dot opens its language overview; Back restores
the chosen layers and framing. No floating ranking tooltip is created.

Three syntax checks and all three focused assertion scripts passed. Local
browser checks covered all eight layer combinations, Dim opacity, invariant
density colors/population, all-layer-Off framing, source/Back navigation,
keyboard dots, decade filtering, benchmark/Mech views and 390px layout.
All 2,418 runs fall within All samples; density has 1,728 cells from 778
programs and 49 variants. Both ranking charts retain 49 dots, with no
overlap or page overflow at phone width. Screenshot capture timed out;
rendered DOM geometry was inspected instead.

The public page and all five changed UI assets match the reviewed bytes.
The data SHA remains b088 above. Live browser checks confirmed the restored
Axes control, density-only framing, both rankings, 20 Clojure programs and
60 Clojure runs with no console errors. Six deployed assets were verified,
including unchanged data.json. Dist is synchronized. Evidence is saved in
`continuation/jobs/layer-controls-publication-v1/`. No benchmark was rerun.

The isolated Prolog runtime probe ran once at 21:12 UTC. Its output and owned
receipt now have distinct filenames, so the actual exit 1 is preserved directly.
All primitive results were written, but removing the imported compiler initializer
did not fix termination. Root verified all 13,213 referenced pins after the job
closed. No image-closure command, version rerun, fixture rerun or benchmark ran.
Evidence is in `continuation/jobs/prolog-runtime-probe-closed-review-v1/`.


A source review now identifies a plausible Prolog shutdown route: halt(0) can
enter the catch recovery, which calls halt(1) while printing an empty unwind
message. This is not yet an observation of the failed process. Two reviewed
marker-and-halt-only controls are prepared in
`prolog-comparison/20261013/revisions/runtime-halt-diagnostic-v1/`; they have
not run. The combined metadata integration v2 source now binds the qualified
Clojure baseline, but its successful legacy133 dependency remains outstanding
and no integration job has run.

## GitLab Pages publication — 2026-10-13

The Benchmark Explorer is now public at https://mech-lang.org/benchmarks/ with
the existing Mech website. The website repository is
https://gitlab.com/mech-lang/web/website; commit
`11cacf9ca1baa800736355759d4b6bd0ca815e68` was pushed to `main` and its remote
SHA verified. Pipeline `2944032278` passed. All 29 published assets match the
reviewed distribution by SHA-256; `/benchmarks` redirects to `/benchmarks/`.
Homepage, Blog and Explore now link to the chart.

The published dataset SHA is
`4fe402c21360fbbbe9d650c15d1fd4c62422c17cfad8cafd24721fe7f0bd2947`.
It retains prior records and fixed baselines and adds qualified C3's 20 source
forms, 60 candidate samples and 19 selected endpoints. All 20 forms remain
inspectable, including the inventory-only optimized N-body program. Local
subdirectory and public browser checks passed; C3's log2 coordinate medians
independently match the rendered markers. The interface uses time × best and
the fixed hover strip above the graph. Screenshot capture was unavailable.

The prior Sites packaging workflow was deliberately stopped after the user's
destination change, before any packaging helper or new Sites deployment. Its
original partial-session failure is retained, and qualified data was preserved.
No benchmark or prior successful fixture was rerun for publication. Actual QA,
commit, push, pipeline and complete public asset verification are retained in
`continuation/jobs/gitlab-pages-publication-v1/`.

All 20 Crystal programs also completed their copied-source native formatting
checks and qualified their counts, preserving original sources and timings.
The formatted-axis and platform UI candidate remains separate and unpublished.

Update2026-10-13 13:43UTC: useraddedRed andRebol topublication scope. Primary runtime/architecture discovery started; no samples collected for either. Whole-nine qualification/integration replaces the former whole-seven inventory. Independent C3 andV full90 checks have nowpassed. The progressfile open request returnedqueued ratherthan opened, so the current summary was also delivered inline.

Update2026-10-13 14:16UTC: Redwasremoved bytheuser. Activepublication scope isNim,Crystal,Pony,C3,Gleam,Clojure,Prolog,Rebol, plusJuliaAOT; Vremainsreport-only. Rebol3.22.1 nativeARM64/runtime/API primarysource discovery andtenbaseline source drafts arecompletebutunqualified; originalsource/nativeextension/counts/runtime checks stillrequired. Ponytwo-tree andPrologprivate-runtime nextstages aredurablyqueuedwithsuccessfulmetadata dependencies, noautomaticretry orfulldataadmission. Threeordered checkpointjobs v4→v5→v6 awaitthecommon timing slot; 8c80a49cb remains thelastverifiedremotecommit.

## Published language overview and Crystal inventory — 2026-10-13

Published the reviewed language overview at https://mech-benchmark-explorer.refreshthen.chatgpt.site from Site source c23339fe74db3623db8409d92826dc0d64b3b853. Deployment appgdep_6ace72e07a608191827dcca1ffac875a succeeded. Clicking names opens timings, all retained programs/runs, coordinate medians/covariance, source navigation and exact-program comparisons. Hover details stay above the graph. Interface time ratios use the fixed best-time baseline; conversation reports use recorded matched Mech controls.

Crystal now retains20 source forms across10 workloads with60 actual candidate samples (the complete study also includes30 paired Mech samples). Original four admission fixtures passed and were reused; prior rows and normalization baselines remain unchanged. All20 sources opened in browser tests, run markers were unclipped, and independent coordinate medians matched. Responsive DOM checks passed; screenshot capture was unavailable. A numeric LOC-audit renderer issue was fixed. Packaging needed the bundled Node directory on PATH and the actual helper127 exit binding; failed attempts are preserved. No new benchmark ran for this publication.

Actual evidence: continuation/language-overview-ui-v1/BROWSER_QA.md; continuation/jobs/current-chart-session-v2; continuation/jobs/current-chart-package-v4/publication.json. Other cohorts remain separate pending admissions.

## Qualified C3 source closure and formatted fixtures — 2026-10-13 18:40 UTC

C3's additive grammar replay retained the original 46 diagnostics and all old
counts. Manual meaning review covered 1,110 owner events, 46 complete data
certificates, 24 originals and 20 effective bundles, with no unresolved item.
The unchanged sealer exited normally at 18:32:49 UTC. Its qualified overlay SHA
is `4465bcc760401a7fd40aa516390a3681cd20edbb91538d9f5607a4060f1a5263`.
The unchanged runtime metadata qualifier exited normally at 18:37:16 UTC;
metadata SHA is `c9c33e008e52280f5ec0a4051b2d57c0a9f052f3e89614ca2310c87e67250e07`.
Both actual closures and their complete pins were reviewed under the shared
mutex. No successful fixture or timing sample was rerun. C3 chart admission
remains a separate next step.

The formatted-LOC packet passed its exact 23 synthetic source fixtures.
Receipt SHA is `7567ddfb5a03df5ed4813dd7d87bfd48b8f300ef0a5be66c90504d377fca81b5`;
terminal SHA is `5b3da339f1f8e575d34742f119996b684b40e03e32365cad749e2a23a61b94bd`.
Root verified the raw log, all discovery/start IDs and all 52 source/unittest
pins. These fixtures ran no formatter or benchmark. Whole-bundle counts,
inventory-only sources, independent recorded launched-image digests, missing
native routes, and transactional preservation of timings/baselines are covered.
Actual copied-source native formatting and the composed X-axis interface remain.

Nim full-v6, Gleam full-v4 and Clojure full-v1 finished all ten timing studies.
Their existing independent full-suite reconciler was launched once per cohort
at 18:39:48 UTC. Exact current outcomes will be recorded after private lease
closure; launch is not admission. All these jobs share the timing mutex and
perform no scored execution. The active goal remains unfinished.

## Independently reconciled Nim, Gleam and Clojure — 2026-10-13 18:53 UTC

All three original full suites passed the existing independent reconciler,
normal durable exit and a separate complete receipt/pin inspection. Each has
20 actual candidate sources, 60 candidate samples and 30 matched Mech samples.
No native, fixture or compiler execution occurred during reconciliation.
Actual receipt: `continuation/jobs/completed-native-review-v1/review.json`.
The full three samples, selected program IDs, exact medians and matched Mech
controls remain in that receipt and each original suite. Source-count/runtime
admission and publication are separate remaining steps.

The following is **median time / that study's matched Mech median**, using
the fastest source form per workload. Larger values mean longer execution.
These chat/history ratios do not change the interface's best-time baseline.

| Workload | Nim / Mech | Gleam / Mech | Clojure / Mech |
| --- | ---: | ---: | ---: |
| Regex redux | 6.035× | 18.997× | 6.474× |
| Reverse complement | 4.740× | 64.762× | 66.413× |
| Pi digits | 1.430× | 5.562× | 13.511× |
| Spectral norm | 4.306× | 38.529× | 6.325× |
| Mandelbrot | 2.805× | 109.002× | 7.573× |
| FASTA | 4.596× | 10.710× | 17.753× |
| K-nucleotide | 6.041× | 224.025× | 13.796× |
| Binary trees | 3.414× | 17.358× | 2.827× |
| N-body | 1.450× | 70.021× | 159.233× |
| Fannkuch redux | 1.983× | 55.008× | 100.250× |

The selected form is optimized except Gleam N-body and Clojure reverse
complement/N-body, where compact is faster. These are comparison-language
results; no Mech compiler optimization or unverified attribution is inferred.

The Pony/Prolog followup-v2 repair retains distinct recorded launcher and
framework images, exact worker-started/terminal identity, PID, birth and other
arguments. Both entries reached their unchanged targets without repeating their
successful seals/fixtures. Prolog failed in exact source-archive extraction;
Pony's two bounded tree observations remain under its original common-lock
controller. The preserved v1 failures remain failed.

## Samples Off correction — 2026-10-13 21:45 UTC

A user screenshot exposed 952 small per-benchmark endpoint marks still drawn
inside the Medians layer. Those redundant background marks are removed;
actual recorded runs now provide the background sample cloud. Summary Medians
retains 97 fastest/fewest-LOC endpoints for 49 execution variants. With Samples
Off there are zero run marks and zero background program marks. Samples On
restores all 2,418 actual recorded-run dots; Medians Off removes summary endpoints.
Dedicated benchmark/language program medians remain available in their views.

The exact screenshot settings were reproduced locally and on the public site.
Density-only still includes 2,418 runs/778 programs/1,728 cells. Axis bounds remain
unchanged, N-body detail medians remain, and the 390px layout has no page overflow.
The app syntax check and browser regression checks passed with no console errors.
Website commit `624c7f9b8b190134034f75fdeb8e010eceec21cc` is pushed;
Pages pipeline `2944391226` passed, and both changed public assets match.
Evidence: `continuation/jobs/sample-cloud-fix-publication-v1/`.
No timing/source data or normalization baseline changed.

The legacy 133 v3 metadata repair passed all ten extended fixtures, but its
qualification stopped at the historical FASTA compact row selector. Root
verified 207 failure pins and its normal exit 1; no partial metadata was admitted.
An all-133 mapping diagnosis is underway before another qualification attempt.
Nim/Gleam source-semantic admission remains active.

Two sequential Prolog halt-only controls completed at 21:45 UTC. Direct halt(0)
exited 0; the catch-wrapped route caught unwind(halt(0)), entered recovery and
called halt(1), exiting 1. Root verified all 13,233 recorded pins, the closed
private lease and normal outer controller exit at 21:46 UTC. The wrapped native
failure remains recorded as a failure; this diagnosis does not qualify the
runtime. No feature probe or benchmark ran. A source repair will move successful
halt(0) outside the checked catch in the probe, QLF preparation and authored I/O
wrapper before fresh feature/image qualification.
Evidence: `continuation/jobs/prolog-halt-controls-closed-review-v1/review.json`.


## Nim source admission, Prolog runtime and platform diagnosis — 2026-10-13 22:15 UTC

Nim's exact source admission passed for all 20 forms and 21 authored inputs.
The counted GMP helper has 13 physical and 23 canonical lines; optimized
Pi totals 84 canonical lines including that helper. Five helper fixtures
passed, and root verified 36,838 evidence pins after normal job completion.
The original 90 timing samples and source bytes are unchanged. An earlier
fixture incorrectly wrapped a type alias and failed; that actual failure is
retained separately. The corrected fixture tests only template wrapping.
The qualified overlay SHA is
`55ef7197d8caa62a5087cf93380e524d8a0fac11d2d82a731e44675e3d288398`.
Evidence is in `nim-comparison/20261012/source-counts/helper-owners-v2/`
and `continuation/jobs/nim-source20-loc-v2-closed-review/`. Runtime metadata
and dataset publication remain pending; Nim is not yet in the public chart.

Prolog's corrected feature probe exited 0. The repair moves successful halt
outside the catch handler, while retaining failure handling. The actual
ARM64 runtime reports SWI-Prolog 10.0.2, enabled GC, unbounded integers and
PCRE2 10.40. Thirty-five native image-inspection commands passed for seven
loaded images. Root verified 13,399 evidence pins; the qualified runtime
configuration adds exact official archive, version and library-source
bindings. Its SHA is
`7fa470a9827e60e84eb9c6ad343cc8385e411b400f0ae8a322fbefda488bb448`.
Portable receipts and logs are saved in
`continuation/jobs/prolog-success-halt-closed-review-v1/`; configuration is in
`prolog-comparison/20261013/runtime-qualified/success-halt-v1/`. No benchmark
has run. Twenty-program QLF preparation, 156 bounded output cases, workload
ownership, exact LOC and all 90 full samples remain pending.

The platform diagnosis completed all 133 historical records: 115 resolve,
13 lack required historical evidence, and five have proven literal source
and timing routes requiring an adapter repair. All five preserve exact
source, sample, executable and build identities. Twelve missing rows are
Julia runtime observations; one is the C FASTA native-build output binding.
Those 13 remain unclassified. Root verified 384 pins. The first diagnostic
attempt rejected a launcher-versus-process-image spelling difference before
visiting any row; the corrected attempt binds the exact saved process image
and all remaining argv. Both outcomes are retained. No platform annotation
or timing was changed. Evidence is in
`continuation/legacy-platform-history-diagnosis-v2/` and its closed review.

Gleam's source-owner semantic packet is ready for independent review but has
not run. Platform repair, Nim runtime metadata and Prolog bounded preparation
are separate additive source packets. The public timing dataset remains
`b088279074e85ec149466f876bdce49d4d497f7784a0d48be58e023e5b2c6a13`.
The Samples Off fix remains live in website commit
`624c7f9b8b190134034f75fdeb8e010eceec21cc`; published page and app bytes were
rechecked at 22:14 UTC. No new Mech optimization or benchmark timing is claimed.
