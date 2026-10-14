Benchmark progress as of **2026-10-14 00:38 UTC** (20:38 New York, 2026-10-13).

The previous remotely verified checkpoint is `362660915bb0c3617a78374d6a606affa09cbfcb`
on `codex/clbg-native-benchmarks`. The language overview and complete Crystal
inventory are published. C3's exact source counts, runtime metadata and current
chart candidate have qualified and are now published through GitLab Pages
at `https://mech-lang.org/benchmarks/`. Nim, Gleam and Clojure
finished and passed independent reconciliation of all 90 timing runs each.
The 23 formatted-LOC source fixtures and all 20 native Crystal formatting cases
passed; the formatted axis and measured platform filters are now published. Durable jobs preserve their
actual request, process identity and exit.

The mobile filter controls now use a left drawer opened by a sticky **Filters**
button. The same benchmark checkboxes, languages, platform filters, year sliders
and decade buttons move into the drawer; desktop retains its sidebar. Done,
outside tap and Escape dismiss it. Navigation closes it before focusing the
language overview or program inspector. Body scroll is restored, selection
changes preserve checkbox focus, and resizing transfers focus between layouts.
Independent source review and browser checks at 320/390 px and desktop passed.
Samples Off still removes all recorded dots; density alone still has all 1,728
cells from 2,529 recorded runs. No measurement or timing data changed.
Website commit `7f67aeb3d048c6ac5c0fe23b23ce6bd8f7982db7` is live; GitLab
pipeline 2944757145 succeeded, all three changed assets match exactly, and the
public mobile drawer passed a final browser check.
Source: `overview/explorer/mobile-sidebar-v1/`; deployment and browser evidence:
`continuation/jobs/mobile-sidebar-publication-v1/`.

The independent Prolog source/WAM review now covers all 20 mains and both whole
helpers, including fresh tree leaves and retained lifetimes, all seven complete
nucleotide maps, and two separate Pi divisions. No unresolved source/WAM work
mismatch was found. The note in
`prolog-comparison/20261013/revisions/scored-work-review-v1/REVIEW.md`
is source evidence only: the 30 native work probes, LOC admission and all 90
full timing runs are still pending. Pony LOC, Julia archive inspection, Elixir
saved-compile owner/writer probing and Rebol SDK control packets also passed
independent source review; these acceptances do not claim execution.

| Cohort | Actual full samples collected | Present state |
|---|---:|---|
| Crystal | 90/90 | Published with 20 source forms and 60 candidate samples, plus 30 matched Mech samples in the complete study. |
| C3 | 90/90 | Published through GitLab Pages. Source counts, runtime metadata and current dataset qualified; all 20 forms / 60 candidate samples passed browser inspection. |
| Nim | 90/90 | Published with all 20 forms and 60 candidate samples. Exact LOC, runtime metadata and native formatting of all 21 authored inputs, including GMP glue, passed. All 20 formatted totals are now on the public chart. |
| Gleam | 90/90 | Published through GitLab Pages with all 20 forms, 60 recorded samples and 104 counted compiler inputs. The source inspector displays genuine per-source owners, including helpers. BEAM filtering and all 20 program rows passed browser checks. |
| Clojure | 90/90 | Published. Full-suite, LOC, JVM metadata and current dataset admission passed. Root verified 5,276 candidate pins; 20 forms / 60 candidate samples are retained, with 18 selected endpoints. Public browser inspection confirmed all 20 forms and 60 runs. |
| V | 90/90 | Full workload samples collected, including eight missing tree slots recovered additively. Independent all-ten reconciliation passed. Report-only; excluded from dataset, charts and Site. |
| Pony | 0/90 | Work qualification passed for all 20 reviewed forms and six retained traces. Root verified the scored-tree allocation evidence and 10,217 pins; temporary-root stack/register promotion is explicitly recorded. LOC admission and full samples remain. |
| Prolog | 0/90 | SWI-Prolog 10.0.2 ARM64 runtime qualified. All 156 bounded correctness cases across 20 source forms passed. The first successful case and compiled artifacts were reused; 155 new cases and 19 additional forms completed. Work/LOC checks and full samples remain. |
| Julia AOT | 0 qualified | Concrete Core I/O compiled successfully with safe trimming and IR stripping. A metadata-only correction sealed the actual15 build options and C ABI while retaining the original reader failure. Archives remain saved; member/native-owner/link/runtime/fallback/output checks and timings remain. |
| Rebol | 0/90 | Publisher runtime and all five native image/version/feature commands succeeded. A narrow parser correction reused those outputs, then obtained the full-commit source archive and 898 source files without another native command. SDK, extension, pipe I/O, work/LOC and full samples remain. |
| Elixir | 0/90 | Runtime probes passed for Elixir1.20.4/OTP29/ERTS17.1 ARM64 JIT. First N-body main/support compilation exited0 and produced two BEAM files; validation stopped on an unreachable defensive-arm warning. The empty compiler-options environment diagnostic is also retained. Owner/writer probe, remaining19 compiles, stdio/work/LOC/full checks remain. |
| Kotlin | 0/90 | Original source bundle is frozen and pushed. A source-only compiler intake packet reuses the qualified Clojure JDK and counts20 main/helper bundles; native intake and complete qualification remain. |

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
13 rows remain explicitly unclassified. Filters are now integrated into the published Site. Unproved aggregate backends remain Unclassified. Successful historical samples and
original failures remain immutable; additive repairs reuse successful evidence
instead of relabeling it as a fresh run.

## Gleam and Nim formatting published — 2026-10-14 00:19 UTC

Website commit `221f06f9846a62d2c31e9fc51a85900cd2a5d620` is pushed and
GitLab Pages pipeline `2944707326` succeeded. All ten public assets match the
reviewed publication. Data SHA is
`3c86a24a2315d87043339e85e21f38abbe6970424ded82d7473abb4b1dce8932`.
The chart has45 language families/51 variants,815 selected programs and100
complete retained inventory records. Ordinary sample display and density
contain2529 recorded runs; each new language overview retains all20 forms/60runs.
Native-formatted LOC now covers Crystal20 and Nim20; other rows stay blank.

Nim integration v2 verifies all original pins and loads only64 explicit receipt
references. Eight intentionally invalid upstream JSON parser fixtures remain
pinned source data. The metadata job closed56473/56479 at00:05:41; root checked
54,118 pins and all815/100 original source, timing and canonical values. No
formatter, source-count fixture or benchmark repeated. Root receipt SHA is
`bb567a2b48b613db123f4677eff579a7e03a5428694be55227c53c12747b43d6`.

Local and public browser checks reproduced Samples Off/Medians Highlight:
zero recorded marks and zero stray endpoints;101 median endpoints remain.
Density alone still renders1728 cells from actual samples; dim samples use
opacity0.15. Medians versus All samples framing expands both axes correctly.
At390px the page has no horizontal overflow. Gleam's real source-owner text,
all20 rows/60runs, BEAM filter, and both formatted cohorts passed, with no
console errors. Public data briefly served its old cache and was verified
again after propagation. Evidence: `continuation/jobs/gleam-nim-publication-v1/`.

Julia's metadata-only saved-build seal completed without another compiler
command. Root checked9,839 resolved pins, all15 actual build options and the
signed Cint/PtrPtrInt8 ABI. Root receipt SHA:
`1ebe64084ffe2996ad4be7e0f5814d200bb61721a4a2bcae0cd3ab9ca9c19d84`.
Archive membership, actual IR removal, native ownership and runtime fallback
remain unqualified. No Julia AOT runtime or speed claim follows from compilation.

Elixir's first subject compile exited0 at00:14:51 and retained two actual BEAMs.
The controller closed58755/58761 with exit1 on its empty-stderr assertion: the
compiler reports an unreachable defensive error arm after IO.binwrite. Stdout
also records `Ignoring bad term in ERL_COMPILER_OPTIONS`; the explicitly empty
environment value is parsed as a bad Erlang term and ignored. No writer probe
or benchmark ran. Root verified7,776 pins and preserved both diagnostics and
artifacts; receipt SHA is
`34de9e98daefecee561002031de45ffdc051cd54521588b5758347979c779abb`.
The next source repair will reuse that successful compile, retain its warning,
and remove only the empty compiler-options variable before the missing probe.

Pony LOC, Prolog work/reader, Kotlin intake and Julia archive/link proposals are
source-only until their separate gates execute. No new Mech optimization or
benchmark timing is claimed in this publication.

## Julia safe compilation and Elixir runtime probes — 2026-10-13 23:50 UTC

The Julia static-I/O revision compiled successfully at 23:48:18 UTC (native
PID 53013), with safe trimming and IR stripping, empty diagnostic streams,
a 9,444,136-byte object archive and a 9,672,560-byte bitcode archive. The
controller then stopped because its TOML reader received Julia `Dict` display
text. The wrapper called `Base.TOML.print`, which is ordinary Base printing;
the TOML serializer is the separate nested Printer API. The emitted options
and successful compiler artifacts are preserved for a metadata-only reader
correction. Root verified 10,072 pins. No compiler rerun, workload, linker,
debugger or benchmark is implied. Root receipt:
`continuation/jobs/julia-static-io-safe-compile-closed-review-v1/receipt.json`.

Elixir's two missing probes completed normally at 23:45:11 UTC (worker52439,
target52452). Root verified 7,765 pins and both native exit0 receipts with empty
stderr. Actual Elixir1.20.4/OTP29/ERTS17.1 reports ARM64 JIT, eight schedulers,
one async thread, one dirty CPU and one dirty I/O scheduler. Publisher/source
intake, all twenty bundles, the successful dry command and eleven ownership
fixtures were reused. The first BEAM execution uses a fresh empty private
HOME/configuration. No workload or compiler command ran. Result SHA:
`ad8aca223866b085ba3e823538298b948bd427748909bff1913d874d608519d0`.

The first Nim formatted-count integration stopped before writing a candidate:
the old generic receipt loader tried to decode every pinned `.json` file as
JSON, including toolchain fixture/source files. Native formatting remains
qualified and is not repeated. The correction must keep every byte pinned
while distinguishing actual receipt paths from arbitrary toolchain files.
All published assets remain unchanged during this local validation.

## Gleam candidate and Pony work qualification — 2026-10-13 23:44 UTC

Gleam's additive schema bridge completed normally at 23:41:42 UTC (worker
51512, target 51516). It projects the existing per-source owner evidence into
the source inspector without inventing physical-line expansions or recounting
source. All 104 source slices and all 20 complete bundles were independently
checked. The candidate retains all 798 old program records and expands the
complete inventory from 80 to 100. Its endpoint catalog contains 815 programs.
All prior source, timing and coordinate values remain unchanged. Candidate
SHA: `76ab76e215082279eff8f1b5c132b4d6c875da6a8be6b4288a2207afd9ba3f9d`.
Root checked 53,558 pins; no native process, fixture or benchmark was repeated.
The companion source-audit UI passed Node syntax checking. This is a local
candidate pending Nim formatted-count integration and browser/publication QA.

Pony's additive metadata seal completed normally (worker 51895, target 51899).
It preserves the failed work report, reuses all six successful work traces,
and binds the independent source/ARM/IR review of both original scored trees.
The existing pure tree prerequisite passed. Root checked 10,217 pins. The
compatible work report is `aa7786b7cd16565a5ef21cb0bb56e8e0b7ccf77ddf5e98cbcfd1710b6b6aa421`;
the allocation receipt is `354f4eab47aef151f1f5cf29824c2b7b4f45f582b307b2373ca0ec316434d346`.
Fresh object semantics and the retained long-lived tree are proved; compiler
promotion of temporary roots to stack/register storage is disclosed. There is
no universal heap-allocation claim or inferred minimum-OS compatibility.
No new trace, compiler command, fixture or benchmark ran. LOC/full timing
qualification remains separate.

Julia's official safe-trim command exited 1 at 23:34:14 UTC. Root checked 10,038
pins. The compiler rejected abstract `Base.stdout` printing and `showerror` /
`println` on abstract `Base.stderr` in the entry wrapper. The secondary compiler
error while reporting that rejection is also preserved. No link or workload
ran. The next source proposal uses the official trimming example's concrete
`Core.stdout` / `Core.stderr`, preserving the N-body kernel and two energy
outputs; any altered failure text must be explicit. No AOT timing is qualified.

Evidence: `continuation/jobs/gleam-current-candidate-v3-closed-review-v1/`,
`continuation/jobs/pony-reviewed-work-metadata-closed-review-v1/`, and
`continuation/jobs/julia-safe-trim-build-closed-review-v1/`.

## Native formatter and retained compiler evidence — 2026-10-13 23:31 UTC

Nim's plan preparation passed with 39,221 checked pins. The native formatting
batch then passed for all 20 program forms and all 21 authored files using
the retained official nimpretty 0.2 image. Root checked 39,561 pins, every
formatter command's normal completion and each complete physical count.
The optimized Pi bundle has 79 formatted lines, including its 18-line authored
GMP helper. All source bytes and timing samples remain unchanged. Qualification
SHA: `4e6dd5fe9cc4eb33f8ff6205d8812f196220987406f3e01013f76c99af8ebaaf`.
These totals are qualified locally and await chart integration.

Pony's six remaining native evidence commands completed normally at 23:21:56;
root checked 10,198 pins. The vendor runtime bitcode actually declares
`arm64-apple-macosx26.0.0`, confirming the merge explanation for the earlier
target check. Separate entry and builtin IR are retained for both tree forms,
together with original production ARM. Source review records compiler stack/
register promotion of a temporary root, fresh recursive descendants and the
retained long-lived root. These facts are not a claim that every node uses a
heap allocation. A compatible work receipt and final semantic admission remain.

Rebol's first runtime intake produced five successful native command receipts,
automatic GC counts of 1 then 46, and all 256 exact regular-file bytes. Parsing
stopped because the real runtime prints `#(true)` / `#(false)`. Root checked
152 pins; the source archive had not yet been requested. The additive source
continuation subsequently passed at 23:31:59 UTC, reusing all five native
commands and obtaining 898 source files without another runtime download or
native command. Root checked 1,064 pins. The newly captured source archive SHA
is `48df5f2c4cc62d7b4085c908427dd6a698540c32ec4c2a7893a2e5c0d639f671`.
Generated headers, source-tree correspondence and extension/pipe/work gates
remain pending.

Elixir's publisher checksum/archive, extraction and all 20 source bundles were
preserved before an argv check stopped. The official launcher resets ERTS_BIN,
prints PATH-resolved `erl`, and retains `bin/../lib` spelling in its dry run.
The assumed absolute normalized argv was wrong. Root checked 7,741 pins;
version/configuration probes and subject compilation never ran. Any continuation
must use the actual launcher/PATH semantics and retain these successful steps.

## Complete Prolog bounded preparation — 2026-10-13 23:22 UTC

Prolog's remaining preparation completed normally at 23:16:51 UTC (durable
worker 36316, target 36320). Root independently rechecked 14,611 file hashes,
all 156 distinct case identities and all 20 output/ownership checks per case.
All 20 program forms have exact compiler-input inventories and bounded output
qualification. The run added 19 built forms and 155 cases, reusing the first
successful case, three compiler commands and two QLF artifacts. No full timing
sample or work/LOC admission is claimed. The report SHA is
`cab6ea7f3bd8a237f8f83833ed9b9cc293bbe1878cc8a894d74fdcf41f7ffbff`;
the root review is `continuation/jobs/prolog-remaining155-closed-review-v1/receipt.json`.

Gleam's first current-data integration stopped before binding inputs because
root omitted read-only permissions on reviewed sources. A separate corrected
attempt retained that failure and passed the original evidence checks, then
found a genuine adapter mismatch: its qualified LOC metric has no `expansions`
field expected by the older chart adapter. The second failure's 53,538 pins
were checked; no candidate or live dataset was written, and no fixture or
benchmark was repeated. Its source mapping needs an additive correction.

The public Samples Off fix remains live in website commit
`5934c73a39a9b565bda5c08c5cc9f502fa24b360`. The recorded live check with
Samples Off / Medians Highlight / Density Off finds zero raw-run marks and
zero extra background endpoints; the 99 language-summary endpoints remain.

## Formatted/platform publication and actual native preparation — 2026-10-13 23:07 UTC

Website commit `5934c73a39a9b565bda5c08c5cc9f502fa24b360` is pushed;
Pages pipeline `2944567551` passed. The combined dataset SHA is
`8b85d35f9ff8c267171c9d91f3467ba765bf019ac438c64ed2071898a7656378`.
Root verified46,991 integration pins. All798 original program records,80
inventory records,2,478 timing runs, source bytes, canonical coordinates and
fixed time/LOC baselines remain. The full catalog is restored after endpoint
selection, so legacy non-endpoint sources are not silently removed.

The Source metric control now switches between canonical and native-formatted
LOC. Crystal has20 complete qualified formatted source bundles; other totals
remain blank. JVM/BEAM/.NET/Native/JavaScript/Unclassified filters consume
recorded backend evidence across the complete retained inventory. The133
historical records include120 proved platforms and97 execution modes; unknown
historical image bytes remain distinct from proved platform facts.

Local and public browser checks preserve the Samples Off fix: zero raw-run
marks and zero stray background endpoints, while99 summary endpoints remain.
Density-only retains all2,478 runs/798 programs across1,728 grid cells. Dim
samples have15% opacity, and both Medians and All samples framing remain
available. Canonical/formatted metric switches preserve the time-axis envelope.
The two compact rankings show50 variants canonically and Crystal alone for
formatted counts. Crystal's language overview shows20 rows/60 runs; Nim's20
formatted counts stay blank while its original timings and code remain
inspectable. Resident Mech retains whole-process columns. The390px layout has
no horizontal overflow and no console errors. Four existing assertion scripts
passed; one exact-floating comparison was corrected to1e-12 tolerance, with
its original failed assertion retained. All ten exact browser asset URLs match
the staged bytes after Pages propagation. Evidence:
`continuation/jobs/combined-ui-publication-v2/`.

Gleam metadata qualification passed normally, with6,907 pins independently
rechecked. It binds Gleam1.19.0, OTP29.1.1/ERTS17.1 JIT, the actual20 source
bundles and original90 samples. Default BEAM process-heap GC is a documented
policy, not a claimed collector probe. Metadata SHA:
`24c2fc111a79e9093fd2e92625bf552c1539d68ef0d53523b084eebf565b2625`.
The next current-data candidate must preserve this published formatted/platform
baseline; Gleam is not yet added to the public chart.

Prolog preparation sealed20 profiles/156 bounded cases without native reruns.
Its first native preparation compiled two QLFs and inspected the original VM
code, then failed: helper preload `-l` suppressed automatic main initialization,
and the REPL consumed the input. Those three successful build commands and
both artifacts were retained. Adding the exact source-declared entry via `-g`
passed a separate all-replacements case: all20 ownership/output gates,254
exact bytes,empty stderr and normal0. Root verified13,486 pins. Remaining
preparation is155 unexecuted cases and19 unbuilt profiles. No full Prolog sample
or work/LOC qualification is claimed. Source and actual failure/success records
are in `prolog-comparison/20261013/` and the named closed reviews.

Pony's LLVM23.1.3 reader qualified four ARM64 linked images (10,107 pins).
An initial recovery stopped before native commands at an overly strict saved
worker-image spelling check. The narrow repair retained exact PID/birth/argv
and the original reader evidence. It produced compact entry and builtin
bitcode separately, and decoded the entry successfully. A literal target check
then stopped: main IR says `arm64-apple-macosx26.0.0` although the original
production argv requests14.4. Root verified10,142 failure pins. Pinned compiler
and LLVM sources explain a possible newer-Apple runtime-bitcode merge; the
continuation must actually decode the pinned vendor header before accepting
that explanation. Completed compact commands and original six traces are not
repeated. No Pony source-work or benchmark result is claimed.

Julia's official safe-trim build proposal and the Prolog/Pony continuations
are saved as source proposals. No new full timing sample or Mech compiler
optimization occurred in this checkpoint. Native work, UI QA, hashing and Git
remain serialized under the common mutex.

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
