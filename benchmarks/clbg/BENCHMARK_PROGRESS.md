Benchmark progress as of **2026-10-13 19:37 UTC** (15:37 New York).

The previous remotely verified checkpoint is `57bd9d43a5afaf498e465205c287b75dd9d208b5`
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
| Nim | 90/90 | Independent full-suite reconciliation and complete receipt/pin review passed. Exact source-owner diagnostics still need resolution before LOC admission. |
| Gleam | 90/90 | Independent full-suite reconciliation and complete receipt/pin review passed. Qualified Gleam and authored Erlang evidence still needs whole-bundle source admission. |
| Clojure | 90/90 | Independent full-suite reconciliation and complete receipt/pin review passed. Qualified whole-source LOC overlay is retained; runtime metadata/admission remains. |
| V | 90/90 | Full workload samples collected, including eight missing tree slots recovered additively. Independent all-ten reconciliation passed. Report-only; excluded from dataset, charts and Site. |
| Pony | 0/90 | Launcher repair reached the target and completed both tree observations. Diagnostic IR extraction failed because the compiler overwrote its authored main IR with builtin output. Both tree observations and all 16 emitted files are retained; additive diagnostic recovery remains. |
| Prolog | 0/90 | Original runtime fixtures and failed bootstrap retained. Archive failure was 40 equivalent repeated directory entries. The narrow extractor repair passed four fresh fixtures; native private bootstrap remains held for actual-argument review. |
| Julia AOT | 0 qualified | Native canary and n-body libraries built. Generic calibration failed before LLDB on comparing the real kernel framework image with Python's distinct launcher path. No debugger-permission cause is established; kernel-image binding needs repair. |
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
The platform frontend candidate passed its checks, but its filters have not yet
been integrated into the published Site. Successful historical samples and
original failures remain immutable; additive repairs reuse successful evidence
instead of relabeling it as a fresh run.

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
