# Browser verification record

Recorded on 2026-09-26 against the local article preview at
<http://localhost:8765/>. These are functional and numerical checks of the
browser implementation, not new native benchmark measurements or a
publication record.

This record preserves the earlier browser checks and distinguishes them from
the newer read-only native-block/resident-REPL integration below. Historical
source-editor checks describe a superseded build, not controls present in the
current article. Do not treat an earlier browser pass as validation of a
subsequent rebuilt page.

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
