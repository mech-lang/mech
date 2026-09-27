# Browser verification record

Recorded on 2026-09-26 against the local article preview at
<http://localhost:8765/>. These are functional and numerical checks of the
browser implementation, not new native benchmark measurements or a
publication record.

This record preserves the earlier browser checks and distinguishes them from
the newer read-only native-block/resident-REPL integration below. Historical
source-editor checks describe a superseded build, not controls present in the
current article. Do not treat an earlier browser pass as validation of a
subsequent rebuilt page. The current camera/scene record is the final section;
earlier sections retain their original source and runtime identities.

## Source and environment

- Browser: the Codex in-app browser, with a real WebGPU adapter executing the
  generated shader. This was not the Node no-adapter fallback.
- Exact source: `source/ekf.mec`, SHA-256
  `a7cd4077c7bf2f9741559b5748f05cf06e9b48e156c4fdbabfbdc7feea065eb2`.
- Local WASM package identified during this build: SHA-256
  `99a400e89a925a959282ef1d2814fef40bd967e8b3f50b3ed03a267da2ac42d5`.
- CPU path: Rust scalar numerical instruction interpreter in WebAssembly.
- GPU path: WGSL generated from the same compiled numerical program and
  submitted through the shared browser compute host.

The complete verifier result from a repeated successful browser run is copied
in `browser-verification.json`; its elapsed time is for the correctness test,
not the throughput chart. Exact browser version and adapter identity were not
included in this summary; record those for future device comparisons.

## Automated CPU/WebGPU verification: passed

The page's `verifyKernel` check ran 20 paired deterministic turns with 256
independent filters. Each turn compares the first filter's mean and covariance;
after fault injection and recovery, it compares all 256 filters. Every finite
component satisfied:

```text
abs(cpu - gpu) <= 1e-4 + 1e-4 * max(abs(cpu), abs(gpu))
```

| Result | Observed value |
| --- | ---: |
| Paired turns before rejection | 20 |
| Filters in the verification batch | 256 |
| Maximum absolute mean-state difference | 3.814697e-6 |
| Maximum absolute covariance difference | 1.068115e-4 |
| NaN injection lane, zero-based | 255 |
| Filters compared after recovery | 256 |

Both implementations rejected the NaN bearing. Whole-batch snapshots of
accepted mean and covariance were bitwise identical before and after the
rejected turn. The GPU active publication buffer did not advance. Replacing
the invalid input allowed recovery, and all 256 recovered filters passed the
same component-wise tolerance. The reported maxima include the verifier's
successful comparison sequence and recovery; they are not throughput or
statistical-uncertainty measurements.

## Browser interaction checks

- GPU execution ran with 4,096 filters in the interactive figure.
- Manual invalid-bearing injection identified lane 4,095. Displayed accepted
  telemetry stayed unchanged, and the Mech behavior entered Fault.
- Reset cleared the fault and initialized a new episode. Fault handling did
  not silently publish the rejected candidate.
- The functions example executed in Mech and returned approximately
  `0.9668146928` for its heading-normalization expression.
- The pattern-matching example executed in Mech and returned `correct`.

The 256-filter automated snapshot test is the whole-batch rollback proof.
The 4,096-filter UI check separately covers interactive fault reporting and
display retention; its displayed first-filter telemetry is not a substitute
for a full-batch snapshot.

## Supporting checks

The implementation also passed three native numerical bridge tests, two
native WGSL finite-endpoint emitter/Naga tests, and the actual generated
JavaScript/WASM smoke test, including six state-machine transitions. Commands
and artifact provenance are in [README.md](README.md).

The Node smoke test does not dispatch a GPU. A verifier result marked
`unsupported` is not a passing GPU result even if CPU checks pass. The browser
verification above explicitly exercised real WebGPU execution.

## Previous-build integration pass — superseded UI

The following checks were performed before source editors were removed and
the standard resident document REPL was restored. They are retained as
historical evidence only; editing and restoration are no longer page features.

- Edited the source measurement covariance from 0.25 to 0.3 through the page,
  recompiled it, and executed an accepted turn. The interface reported edited
  source, kept the archived figures unchanged, and disabled the exact-paper
  verification until restoration.
- Submitted invalid replacement source. Compilation failed with a diagnostic,
  and the previous published telemetry stayed identical. Restoring the paper
  source initialized a new working episode.
- Switched GPU → CPU → GPU and executed an accepted turn in each mode, then
  launched a separate verification while the GPU demo was resident. New GPU
  devices request fresh adapters; previously consumed adapters are not reused.
- The compressed WASM route initialized successfully in the browser. The
  static test verifies that the gzip artifact decompresses to the exact WASM.
- Checked source, CPU chart and pipeline rendering, working section navigation,
  and desktop document width: 1,280 px viewport and 1,280 px scroll width,
  with no missing images. A separate mobile/device compatibility campaign has
  not been performed.
- `node benchmarks/iros-2026/blog/test.mjs` passed asset, anchor, duplicate-ID,
  source-hash, expanded-document, chart, and WASM checks.

No archived CPU, Metal, dylib, memory, or source-size measurements were rerun
or changed by these browser checks. The observed live FPS is not a new native
benchmark claim.

## Previous shared-template pass — superseded presentation-only startup

That intermediate article composed `include/blog.html` and copied the shared
palette, source, Mechdown, shell, blog, and document-controller assets unchanged.
Article-specific CSS is limited to executable examples and workshop figures.

- Inspected the hero, section headings, syntax highlighting, CPU chart, and
  reactive pipeline at 1,280 × 720. The empty hero-art slot is hidden without
  reserving a second column.
- Checked the sticky TOC, expanded subsection links, active-section state,
  and restoration of the desktop content-shell scroll position after reload.
- Checked the 390 × 844 responsive layout: Contents opens, Escape closes it,
  and selecting a subsection closes it and navigates to the section. Page
  width equals viewport width; wide figures and code scroll within their
  containers. This is responsive-layout testing, not a physical-phone or
  mobile-GPU qualification.
- Repeated the actual CPU/WebGPU numerical verification: 20 paired turns,
  256 filters, rejected NaN with whole-batch rollback, and successful recovery.
  Both backends passed; the observed maximum errors matched the earlier record.
- No browser warnings or errors were reported in this integration pass.
- The shared presentation/TOC regression script verifies no second WASM
  runtime, active links for both window and content-shell scrolling, and
  scroll-margin/padding offsets. All 19 shared style contracts passed.

The first-use fixes hide absent hero artwork, wrap long identifiers in
footnotes, defer workshop anchor handling to the shared TOC, and contain
wide workshop figures on narrow displays. The controller's opt-in
presentation mode preserves ordinary document startup for other pages.

## Current native-block and resident-REPL integration

The current source replaces the intermediate presentation-only startup with
the actual v0.4 resident document controller and REPL. One cached WASM
initialization is shared with the numerical host. The whole expanded document
is formatted and encoded once, and the same source is embedded for REPL
reflection. All four Mech fences retain native block and output IDs. Behavior,
functions, and matching use root scope, with presentation-only filename pills.
The EKF uses the native `mech:ekf` namespace and keeps its native pill; its
standard output block is marked `data-workshop-kernel-output` for the numerical
host to populate from actual accepted kernel state.

An all-root-scope attempt exposed a resident projection failure when a REPL
submission included the full EKF document. The integration therefore keeps
the numerical example in its named namespace and runs it through the existing
separate checked-kernel path. This is a current runtime limitation, not a
claim that the complete numerical demo executes in the resident document REPL.

The source listings are read-only. Document headings become Mech comments in
each fence, and the behavior example adds the labeled invocation
`#Robot(0, 1)`. Original downloadable Mech files are unchanged. The EKF demo
continues to compile the exact original source, including its `@compute`
heading, in a separate kernel session.

Completed build-level checks on 2026-09-26:

- The actual native formatter parsed the complete expanded article and
  produced four native Mech fences: three root output addresses (`:0`) and
  one named EKF output with the kernel-host marker. It also produced three
  native Rust code blocks.
- Adding native filename-pill markup preserved every formatter-generated ID.
- All four downloadable Mech sources remained byte-identical to their inputs.
- The rebuilt-artifact resident-runtime smoke, static article, mobile-layout,
  and shared presentation/lifecycle regression tests passed. All 19 shared
  style-contract tests passed.

Current rebuilt-page browser checks passed on 2026-09-26:

- The resident document and console reached ready state. Native root outputs
  displayed `1`, `0.9668146928`, and `correct`; the EKF block displayed the
  numerical kernel's real initial state. Submitting `40 + 2` in the resident
  REPL returned `42` without errors.
- The only textarea was `.repl-input`. There were no inline kernel, function,
  or matching source editors.
- The real WebGPU verifier again passed 20 paired turns across 256 filters.
  NaN in lane 255 caused rejection and bitwise whole-batch CPU/GPU rollback,
  including an unchanged GPU active publication buffer; all 256 filters
  recovered. Maximum absolute errors remained `3.814697e-6` for mean state
  and `1.068115e-4` for covariance.
- At desktop width 1,280 px, document scroll width was 1,280 px. At
  390 × 844, document scroll width was 390 px. All five narrow-layout figure
  variants were displayed with 352 px container/scroll widths and 338 px SVG
  widths, with no horizontal overflow. The mobile CPU chart was also
  visually inspected and legible. This is responsive-browser testing, not a
  physical-phone or mobile-GPU qualification.

Final local visual and interaction checks also passed:

- The mobile hero preserved its full 800:640 aspect ratio without cropping
  or blank padding. Hero and Mika images loaded; footnotes rendered at
  16.96 px, and mobile tables fit within 354 px without overflow.
- The vertical mobile pipeline clearly showed the artifact, serialization,
  configuration, host, and turn branches. Every standard footer link group
  and the release card were visible without overflow.
- The mobile REPL drawer evaluated `#Robot(0, 1)` to `1` without errors.
  One CPU turn and one GPU turn were accepted, and the native EKF block's
  live output exactly matched the displayed telemetry. Source listings
  remained read-only.

This validates local desktop and 390 px responsive-browser layouts, not
physical-phone hardware. Production checks remain separate. No archived
native measurements were rerun or replaced by these checks.

## Inferred dimensions and compact columns — September 26 revision

This intermediate presentation source had SHA-256
`438c5fd415c3d093b0438f80215846f635e99d6d2caea35cb03a84f79558d8af`.
It replaces the preceding presentation source
`a7cd4077c7bf2f9741559b5748f05cf06e9b48e156c4fdbabfbdc7feea065eb2`.
Matrix dimensions are inferred, and column vectors use transposed-row syntax.
Required f32 precision annotations remain. The prediction, correction,
integrity constraints, and accepted-state assignments are unchanged.

A direct comparison using the actual WASM kernel checked all exported state
and covariance words for 256 filters after each of 40 deterministic sensor
turns. The two sources were bit-identical at initialization and every accepted
turn. Injecting NaN, positive infinity, and negative infinity into the last
lane caused rejection in both kernels; full-batch accepted-state snapshots
were unchanged. Replacing each invalid input allowed recovery, again with
bit-identical exported values. This initial comparison used the earlier
`99a400e89a925a959282ef1d2814fef40bd967e8b3f50b3ed03a267da2ac42d5`
WASM artifact. It is a CPU source-equivalence check and does not establish
WebGPU parity for the new source or replace the earlier browser record.

## Unicode mathematical source — September 26 revision (before stabilization)

At this revision, the presentation source had SHA-256
`cefe87b0ee184f1f30c34c66e626948f6d43236c8449dca0054a68e9e5cd932f`.
It replaces the intermediate compact ASCII source
`438c5fd415c3d093b0438f80215846f635e99d6d2caea35cb03a84f79558d8af`.
The mathematical names, including `μ`, `Σ`, `μ̄`, `Σ̄`, `μ₊`, `Σ₊`, `Δt`,
`δ`, `ν`, and `ε`, are actual parsed identifiers, not a visual replacement
layer. The live input names remain `bearing`, `v`, and `w`; the public
numerical exports are now `μ` and `Σ`.

Using the actual rebuilt v0.4.0-beta WASM, the Unicode source and the immutable
`src/wasm/tests/fixtures/paper-ekf.mec` reference were bit-identical for all
exported means and covariances at initialization and after each of 40 turns
across 256 filters. Tests explicitly map the current `μ`/`Σ` exports to the
reference `state`/`covariance` names. NaN, positive infinity, and negative
infinity in lane 255 each caused rejection without changing any accepted-state
bits; valid input then recovered with identical exports. This initial Unicode
comparison used intermediate binary
`f3502ebd286b125afef7268dc6ada80bad09746d42282f7eecddce464f13732b`.
It proves numerical source equivalence, not successful resident typed-atom
integration or WebGPU dispatch for that binary.

The complete bit-comparison and rollback/recovery sequence remains a permanent
part of `test-document-runtime.mjs`, using the retained pre-stabilization
Unicode fixture, the original reference fixture and the shipped `drawing.mjs`
sensor source. It is now separate from validation of the intentionally changed
stabilized kernel. The earlier benchmark records and browser verification JSON
remain unchanged.

## v0.4.0-beta rebuilt runtime — September 26 revision

The final generated WASM artifact has SHA-256
`18224e2cf04b246148029bff7bf498ed77c92780495a5e7cfc493a9cd94a8871`
and is 49,168,648 bytes before compression. It was compiled offline with
Rust nightly `2026-03-03`, `wasm-pack 0.12.1`, release mode, no default
features, and `browser_compute_canary`, using the cached target directory
recorded in the README. The native document renderer was also rebuilt.

The actual Rust/WASM `:version` response reports `0.4.0-beta` for all 13
installed components: Mech Web; standard library, math, compare, logic, range,
matrix, and string; console, browser, time, timer, and scene. No JavaScript
override supplies these version values. The package-version checker also
passed for active package manifests, internal requirements, and lock entries.
At this pre-stabilization revision, `dist/build-manifest.json` identified both
this WASM hash and the then-current Unicode source hash
`cefe87b0ee184f1f30c34c66e626948f6d43236c8449dca0054a68e9e5cd932f`.

Completed checks against this artifact and rebuilt page:

- `test-document-runtime.mjs` passed one-download/shared-initialization,
  complete encoded document startup, exact embedded/downloaded source
  identity, three resident outputs, and four independently owned named EKF
  stages with exactly one live publication output.
- The same test passed arithmetic, compact column output (`[0 1 1]'`),
  scalar output without an inferred type suffix, imported functions,
  all six named-atom behavior transitions, and pattern matching in the
  resident article REPL.
- It repeated the then-current Unicode-source versus immutable-reference
  comparison on this final binary: all exported bits for 256 filters over
  40 turns, NaN/positive-infinity/negative-infinity rejection, whole-batch
  rollback, and valid-packet recovery.
- `test-language-examples.mjs` passed all 12 mode/event combinations,
  wildcard precedence, unknown/numeric rejection and recovery, typed enum
  assignment, functions, matching, and enum payloads in initial, next,
  asynchronous, and mixed typed/untyped states.
- The generated-package kernel smoke passed CPU execution, readbacks,
  rejection/rollback, reset, WGSL manifest generation, and the six typed-atom
  behavior transitions. Its retained numerical reference fixture is unchanged.
- All 36 native formatter tests and 25 module/import tests passed. They
  cover the compact source vectors, explicit precision preservation, import
  token styling, structured function bodies, and corrected scientific-literal
  punctuation.
- All 20 shared style-contract tests passed. The global-body-selector check
  now matches selector tokens rather than accidentally rejecting the new
  `.mech-function-body` class; focused false-positive/true-positive cases
  are included.
- The article static test, shared Output-host lifecycle test, and shared
  presentation/TOC regression test all passed. Static tests validate nested
  publication paths, source identity, figures, links, shared assets, and
  exact raw/compressed WASM identity.

These are local native/Node checks. WGSL manifest generation is not GPU
dispatch, and they do not replace a current-artifact browser/WebGPU or
production-publication check. Historical measurements and prior-browser
records above retain their original source and artifact identities.

## Long-horizon covariance stabilization — September 26 revision

The default-input investigation reproduced the original CPU rejection after
406 accepted turns for 4,096 filters and after 359 for 65,536 filters. A local
65,536-filter WebGPU run rejected after 397 turns; the user's earlier GPU run
rejected after 340. The inspected raw candidates remained finite with positive
symmetric-part leading principal minors, while roundoff had pushed mirrored
entries just beyond the fixed absolute symmetry tolerance. This was not an
intentional turn limit. The [diagnostic report](evidence/ekf-long-horizon-diagnostic.md)
retains exact candidate values, artifact identities and reproduction methods.

The current article-only source is now
`f18e37effb2fa63fadca69639f3a8eed218b73218decf65419e78a60b62bb46b`.
It checks raw pairwise asymmetry against `1e-4 + 1e-6*abs(a) + 1e-6*abs(b)`,
then publishes `Σraw*0.5 + Σrawᵀ*0.5`. Raw and projected finite/positive-diagonal
guards remain. The original Unicode source is retained byte-for-byte as
`evidence/ekf-before-symmetry-stabilization.mec`; its old-source equivalence
test still runs independently. The current source is checked against the
tested stabilized candidate, not incorrectly claimed bit-identical to the old
floating-point algorithm. Native benchmark sources and archived results remain
unchanged.

On the current `18224e2c…` WASM, the adopted source passed:

- CPU: each offered batch size (256, 4,096 and 65,536 filters) × 1,000
  accepted default-input turns, checking every published lane on every turn;
  and 64 filters × 10,000 turns with a raw
  residual/budget audit matched bit-for-bit against the uninstrumented kernel.
- WebGPU: the same three offered batch sizes (256, 4,096 and 65,536 filters)
  × 1,000 accepted turns, inspecting every published lane at each 100-turn
  checkpoint and at the end. Together with CPU, all six offered backend/batch
  configurations passed this bounded default-input trial.
- Both: finite values, positive diagonals and bit-identical published mirrored
  pairs; NaN/±Infinity last-lane rejection before and after the run;
  whole-batch bitwise rollback and successful recovery. GPU rejections also
  retained the active-buffer index. A separate finite raw asymmetry of `1`
  rejected before it could be hidden by projection.

The full CPU reports and observed GPU summary are linked from the diagnostic
report. These establish the stated bounded invariant checks, not PSD, global
EKF stability, or long-horizon CPU/GPU numerical equivalence. Diagnostic
elapsed times are not benchmark measurements. Article build/browser layout
and production publication remain separate checks.

## Earlier stabilized page and embedding verification

The rebuilt page's actual verification button passed with source `f18e37…`
and WASM `18224e2c…`. It dispatched WebGPU, compared 20 paired turns and
recovery at 256 filters, and verified whole-batch bitwise rollback for an
invalid last-lane bearing. Maximum observed absolute differences were
`3.814697265625e-6` for the mean and `9.918212890625e-5` for covariance;
every comparison met the existing `1e-4 + 1e-4*max(abs(cpu),abs(gpu))` bound.
The observed report is retained in
[browser-verification-stabilized.json](evidence/browser-verification-stabilized.json).

The exact downloadable Rust JIT, AOT producer, and bundle-loader examples
compiled under the repository's `-D warnings` configuration and executed
against this same stabilized source. Four exported `μ` states advanced and
agreed within f32 tolerance. Run `test-rust-examples.mjs --native` to repeat
this integration check; its static mode also verifies display/download identity.

The article, resident document, named-atom behavior, formatter, source identity,
mobile chart geometry, Output-host lifecycle, and shared TOC tests passed.
Desktop browser inspection confirmed the standard info-callout rendering and
responsive, full-width black footer. The hero uses the CC0 photograph credited
in `vendor/PITTSBURGH.md`, with the color treatment specified in CSS.

The shared fullscreen handlers passed simulated unsupported/rejected native
requests, native entry, and external exit. In the in-app browser, the native
fullscreen session did not persist after the automated click; successful
interactive fullscreen on another browser is not claimed by this record.

## Current Mech camera, scene tables, and vectorized tolerance

The local browser run on September 26 used Chrome 153.0.8010.53 and real
WebGPU. Its complete report is
[scene-browser-verification.json](evidence/scene-browser-verification.json).
The checked artifacts are:

- Numerical source: `69480e5b46a4da7b9391755dc2a40a5e45a3e39e0899351910c83e5352893689`.
- Camera/scene source: `dd99c4bea1ecdaeab20d9d1d6bb0889ca9c511a04df653e0959fc201d4f2c87d`.
- WASM: `ba2fe6330931fdeb32f0417dfd903185cee392d1bb4bbae026876415168ba880`.

Eight UI cases passed: CPU and GPU, each at 1, 256, 4,096, and 65,536 filters.
Every case checked an accepted turn, camera range loss, a prediction-only turn,
a selected-landmark change and correction, injected last-lane rejection, and
Reset. Editing a control did not advance the numerical state or trail;
rejection preserved both. The actual scene snapshot used the dotted muted-gold
trail and gray heading requested for the figure. No uncaught browser exceptions
were recorded. Layout checks at 320, 360, 390, 900, and 1,920 pixels, with the
REPL open and closed, found no page-level horizontal overflow. The hero date
stayed on one text line without overlapping the independently wrapped authors.
Visual inspection confirmed the taller, warmer Pittsburgh
photograph and removal of the pink application-output border.

The independent 256-filter verifier passed 20 paired turns, including four
prediction-only turns and a landmark change, then fault injection and recovery.
Maximum absolute CPU/GPU differences were `1.9073486328125e-5` for the mean and
`5.340576171875e-5` for covariance, within the existing componentwise tolerance.
Whole-batch accepted state was bitwise unchanged after rejection on both paths,
and the GPU publication buffer did not advance. All 256 recovered filters were
compared. These are correctness checks, not new benchmark samples.

The native numerical regressions compare the new gathered/vectorized tolerance
with its preceding scalar form bit for bit, including fault and recovery
behavior. Elementwise `abs` lowering is covered for scalar, row, column, and
non-square matrix shapes. The built WASM document test executes the actual Mech
camera and scene source for 40 turns and preserves the separate historical
source-equivalence fixtures. Scene-specific native checks and discovered source
limitations are described in
[scene-source-integration.md](evidence/scene-source-integration.md).

The browser-host error regression covers 14 CPU/GPU scenarios. Preparation
errors and display errors after numerical publication stop the application and
require Reset; they do not count as integrity rejection or claim numerical
rollback. The one-filter case also exercises the runtime's dynamic-matrix
fallback when dependency feature unification selects an unavailable fixed
matrix backing. No feature profile or unsafe-policy exemptions were added.

Final native checks passed all eight scene/interface tests and all seven
numerical-kernel tests under `browser_compute_canary`; standard and full
distribution-contract checks also passed. The generic scene renderer passed
its JavaScript regression, and the shared application-output host, TOC,
presentation mode, language examples, and mobile-chart tests passed.
The three exact downloadable Rust programs also compiled and ran against this
source: JIT embedding, AOT bundle production, and bundle loading. Their four
exported states agreed within f32 tolerance.

## Live controls and distinct truth/estimate rendering

The subsequent regression report is
[scene-controls-verification.json](evidence/scene-controls-verification.json).
It records the updated scene hash separately from the preceding camera build;
the numerical source and WASM binary are unchanged.

Real pointer drags move velocity, angular velocity, noise, and camera range
through multiple values while 4,096-filter CPU and WebGPU runs continue. Sensor
controls no longer become disabled during each numerical turn. Coordinator
tests hold a GPU turn pending and verify that it retains its original input
snapshot, while the next turn consumes the new controls. Compilation and
verification still lock controls that would conflict with those operations.

The scene uses a transparent cyan ring and solid path for simulated truth,
and a smaller yellow marker and dotted muted-gold path for the estimate.
Actual Mech/WASM checks verify distinct coordinate bindings, visible coincident
markers, independent histories, and twelve accepted EKF updates. The eight
CPU/GPU browser cases also verify both paths advance on acceptance and remain
unchanged on rejection. Neither the simulated motion nor the filter equations
were changed to exaggerate tracking error.

Responsive checks cover 320, 360, 390, 900, and 1,920 pixels with the output pane
open and closed. The drawing fills its available width at the scene's 200:130
aspect ratio, authors wrap beside a single-line date, and the desktop grip is
centered on the divider. These are desktop-browser responsive checks, not
physical-phone qualification or performance measurements.

## Fixed cameras, field wrapping, and bounded sensor preparation

This extension executes `source/camera-ekf.mec`, rather than the preserved
bearing-only `source/ekf.mec`. Four fixed cameras supply world-referenced range
and bearing observations; one motion prediction is followed by a correction
for each enabled, in-range camera. Motion and measurement noise are independently
adjustable in the Mech scene. The numerical inputs are `control`, `cameras`, and
`measurements`, and the published outputs are `μ` and `Σ`.

The final numerical and scene source identities are:

| Artifact | SHA-256 |
| --- | --- |
| `source/camera-ekf.mec` | `18c016bc35189b43f21689af955b1cd41ad680a851e210506aea9ae5466aeb5f` |
| `source/scene.mec` | `1a9e9fc8d4ba8b2bca63a7b39e0e0024fd8bcee8fd386b7997b298accd37d433` |
| `mech_wasm_bg.wasm` | `ba2fe6330931fdeb32f0417dfd903185cee392d1bb4bbae026876415168ba880` |

`test-camera-kernel.mjs` passed against the actual WASM kernel and an independent
double-precision reference. It checks 200 turns for four filters with changing
camera masks and four measurement-noise levels, including prediction-only
intervals. Separate cases check every camera's contribution, unavailable cameras
at otherwise singular geometry, the absence of a direct robot-heading observation,
NaN and both infinities in the last filter, bitwise whole-batch rollback, recovery,
and rejection of an intentionally asymmetric raw covariance. Every checked
published covariance is finite, has positive diagonal entries, and has bitwise
equal mirrored entries. These checks do not establish positive definiteness.

Position wraps into the 200×130 field while heading continues. A camera's noisy
polar observation selects the nearest periodic representation of the predicted
position before its ordinary range-and-bearing correction; this handles truth
and estimate crossing an edge on different turns. Camera visibility and measured
range still use ordinary Euclidean field coordinates. The numerical tests cover
all four edges, with noise zero or one and cameras available or unavailable,
for 20 turns per case. Two additional CPU/WASM trials each run four filters for
1,000 turns with biased simulated motion and four field crossings. One includes
a 100-turn measurement dropout. The first filter's maximum periodic position
error stays below two field units without the dropout and below five with it;
finite-state and covariance checks cover every filter. These are deterministic,
bounded cases, not a claim of stability for arbitrary controls or noise.

All eight native scene/interface tests passed. They include actual Mech scene
execution with 1, 256, and 4,096 sensor lanes, measurement availability, camera
enable-state changes, preparation without motion or trail advancement, and scene
publication after an accepted turn. `test-drawing.mjs` separately checks fixed
camera-centered range circles, independent motion and measurement noise, queued
camera clicks, distinguishable truth/estimate geometry, accepted histories, and
trail restart at each of the four field edges.

Large populations use sensor-preparation packets of at most 4,096 global lane
indices to respect the resident runtime's temporary-value budget. Real WASM host
tests cover 65,536 and 4,097 filters, including the partial final packet. The
assembled observations are checked against direct Mech evaluation at global
lanes 1, 4,096, 4,097, and the final lane; the last packet must not repeat the
first packet's noise. Preparation leaves the accepted-turn counter unchanged,
and acceptance advances the scene once. Only sensor preparation is packetized:
the numerical kernel still executes and validates one full-population turn,
with whole-batch publication or rejection.

The final packetized-host browser regression passed and is retained as
[fixed-camera-verification.json](evidence/fixed-camera-verification.json); it
records its artifacts and completion status separately from the earlier
selected-landmark reports. `test-browser-scene.mjs` exercises CPU and real WebGPU
at 1, 256, 4,096, and 65,536 filters. Its eight cases cover actual pointer clicks
to disable and re-enable all four cameras, prediction-only turns when all cameras
are disabled or out of range, restored four-camera correction, last-lane rejection,
retention of both pose histories, and Reset. Additional checks drag all five
motion/noise/range controls while turns continue, click cameras during running
turns, and observe a field crossing and trail restart on each backend. The
independent 256-filter verifier compares 20 CPU/GPU turns, including unavailable
and individually enabled cameras, followed by rejected-turn rollback and recovery.
Responsive checks cover 320, 360, 390, 900, and 1,920 pixels with the pane open and
closed, including scene aspect ratio, metadata fit, and divider alignment.
All eight backend/population cases, both five-slider runs, the CPU/GPU verifier,
and all ten responsive layouts passed with no uncaught browser exceptions. Each
backend's observed field crossing occurred at 132 accepted turns with zero
rejections, and the scene restarted its trail at the boundary.

The document test preserves the historical bearing-only equivalence checks in
their own source fixtures and separately runs the displayed camera kernel and
resident scene for 40 turns at 256 filters, with rejection and recovery checks.
The reference listing and live listing have separate namespaces. No archived
native benchmark source, timing, or source-size measurement is replaced by this
extension, and the browser and numerical tests are not new throughput samples.

## Workshop runtime, controls, and startup revision — 2026-09-27

The `browser_workshop` profile retains the document REPL, displayed language
examples, CPU/WebGPU numerical compiler, and console/scene hosts. It excludes
unused delegated-server authority and DOM/time/timer providers. Official
wasm-bindgen regeneration removes debugging names without changing noncustom
WASM sections. The JavaScript bindings and WASM are regenerated together.

| Artifact | SHA-256 |
| --- | --- |
| `mech_wasm_bg.wasm` | `2bf653d0a467f800a9df301c434df14d32d41d7e2d26c50187e1a2d41e1ed99e` |
| `source/scene.mec` | `70547e59ec03a0bde301c7b7e2054c368a85689d927acdfa4bece626c0ce1588` |

The numerical sources retain the identities in the preceding section. The
scene change darkens each robot heading line within its marker's color family.
The production runtime is 30,066,611 bytes, or 4,530,509 bytes gzipped, versus
49,267,038 and 5,485,756 bytes respectively for the preceding runtime.

The document/runtime, drawing, camera-kernel, application-turn, document-output,
document-presentation, and scene-renderer tests passed against this artifact.
The camera-kernel checks include the two 1,000-turn cases described above.
All five WASM build-script tests passed. These tests retain integrity checking
and whole-batch rollback; the smaller profile does not weaken those checks.

[The full browser report](evidence/workshop-trim-browser-verification.json)
records CPU and real WebGPU at 1, 256, 4,096, and 65,536 filters, all camera
toggles, unavailable-camera prediction, rejection/recovery, and CPU/GPU
agreement. Five actual slider drags per backend changed controls while turns
continued. Structural controls were disabled, visibly dimmed, and did not gain
hover highlighting. Both backends crossed the field boundary at 132 accepted
turns with no rejection. Ten responsive layouts passed with no uncaught
browser exceptions.

The page preloads the shared runtime, omits unused Mermaid loading and the
redundant document fetch, and loads footer artwork lazily. FAQ and idea callouts
replace the generic EKF question and numbered integrity-story subsection; the
introductory callout floats right on wide layouts and returns to normal flow
on narrow layouts.

[The cold local startup trace](evidence/workshop-trim-local-startup.json)
recorded first content at 356 ms and a ready EKF at 4,561 ms. It is a local
diagnostic, not a public-network speed comparison or benchmark result. The
remaining long tasks primarily prepare the resident document and scene, then
compile the numerical program synchronously. Browser tests confirm that the
runtime is fetched once and no unused Mermaid, `/code/article.mec`, or eager
footer-Mika request occurs during startup.

The subsequent presentation-only border correction was checked against the
published v0.3 blog's computed styles. [The border regression
report](evidence/code-border-verification.json) covers all twelve Mech fences,
three Rust listings, and the application output at 1,440 and 390 pixels.
Standalone code has a complete neutral outline; visible code/output pairs
share one divider and their outer rounded frame. Empty, hidden, and absent
outputs close the code frame, and restoring output rejoins it. The checks
passed without page overflow or browser exceptions. Desktop and mobile
screenshots were visually reviewed. The numerical sources and runtime did
not change after the full browser campaign above.
