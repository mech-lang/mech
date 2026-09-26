# IROS executable article

This directory builds the workshop article, its executable browser examples,
and four charts from retained measurements. Building or serving `dist/` does
not publish the site. The publication destination is
<https://mech-lang.org/iros-r4r-2026/index.html>; publication is a separate
review and authorization step.

## Files and generated output

- `article.mec` is the authoring template. `BLOG...` tokens are build-time
  insertion points, not public source examples.
- `source/ekf.mec`, `behavior.mec`, `functions.mec`, and `matching.mec` are the
  complete downloadable Mech examples. The displayed listings retain their
  computational lines. The EKF is presented as four connected stages with
  explanatory prose; its downloadable source remains one compute program.
- `standard-examples.mjs` expands the listings into native, read-only Mech
  fences before the complete document is parsed. Behavior, functions, and
  matching use root scope; all four EKF stages use the same native `mech:ekf`
  namespace. The behavior
  listing adds a commented example invocation, `#Robot(:paused, :run)`; its download is
  unchanged. The three root-scope filename pills use native namespace-label
  markup only as presentation; each EKF pill identifies its shared namespace
  and stage. All
  formatter-generated block, source, and output IDs remain intact.
- `render/` is a small native `mech-syntax` parser/HTML formatter executable
  that also embeds the encoded AST of that same complete document.
- `build.mjs` formats the expanded article once, inserts the charts, diagram
  and live UI, and copies their runtime dependencies. Rust listings use native
  non-Mech fences with a presentation-only syntax-highlighting pass.
- `blog-shell.mjs` composes website navigation and the workshop host with
  `include/blog.html`. It does not replace the shared hero, content columns,
  metadata, TOC placement, or backmatter markup. The numerical application
  occupies the shared Output pane, selected initially; the article's launcher
  opens that pane or its existing fullscreen view. The Console tab retains
  the interactive document REPL.
- `include/palette.css`, `mech-source.css`, `mechdown.css`, `mech-repl.css`,
  `style.css`, `blog.css`, and `document.js` are copied unchanged into the
  publication. The shared controller starts the real v0.4 resident document
  and REPL, populates the three root-scope outputs, and supplies TOC expansion,
  active sections, mobile navigation, and scroll restoration. `article.css`
  styles workshop controls, figures, and Rust syntax tokens.
- `runtime.mjs` caches one WASM initialization promise shared by the document
  controller and workshop kernel host. `document-source.mjs` embeds the same
  expanded source used by the formatter and AST encoder for resident REPL
  reflection. The resident document and numerical demo have separate state;
  sharing the WASM instance does not couple their sessions.
  The numerical host fills the final EKF stage's standard output block,
  identified by `data-workshop-kernel-output`, from its actual accepted kernel
  state. All four EKF listings have `data-workshop-kernel-listing` markers;
  their variables do not offer inspection of unrelated resident-root values.
  The shared controller preserves application-owned Output content when the
  resident document refreshes its own outputs.
- `header-actions.html`, `separator.html`, and `footer.html` reuse the official
  blog's GitHub Star widget, Mika separator, complete link groups, release
  card, and artwork. The current shared styles also render footnotes and works
  cited. The Star count uses the official external GitHub-buttons script.
- `charts.mjs` recomputes median and unscaled MAD from all ten retained samples
  for each row/mode and checks them against the archived summaries. Charts and
  the pipeline have separate desktop and narrow-layout SVGs, selected at the
  720 px breakpoint; figures fit their containers instead of requiring a
  fixed-width horizontal scroller. Code can still scroll within its block.
- `hero.mjs` deterministically generates `hero.svg` from 40 checked turns of
  the actual Mech WASM kernel and the demo's sensor source. JavaScript draws
  the resulting state and covariance; it does not implement another EKF.
  This retained diagram is not the current article hero. The article uses
  `vendor/pittsburgh-hero.jpg`, a CC0 photograph with a light CSS color treatment whose
  provenance is in `vendor/PITTSBURGH.md`.
- `app.mjs` connects the browser controls to Mech; `drawing.mjs` supplies
  synthetic observations and SVG rendering; `verify.mjs` checks CPU/GPU parity.
- `dist/index.html` is the deployable page. `dist/article.mec` is the expanded,
  downloadable article with the complete Mech and Rust listings, rather than
  the template tokens. Its live UI is supplied by the accompanying browser
  host files; the article text alone is not a standalone UI bundle.
- `dist/poster.pdf` is an unchanged copy of
  `../poster/IROS-2026-Mech-Poster-prose-v3.pdf`, retained beside the article
  and identified in the build manifest for downloadable poster links.
- `dist/build-manifest.json` records build time, repository revision, dirty
  worktree status, and hashes of the kernel, behavior source, WASM and copied
  evidence. Retain it with a publication.

`_build/`, `render/target/`, and `dist/` are build outputs. The small `vendor/`
directory retains the website logo and Fira Code font for subsequent builds.
It also retains the official Mika footer artwork and its provenance notice,
plus the temporary Pittsburgh illustration and its separate provenance notice.
The generated WASM package is copied from `src/wasm/pkg/`, not downloaded from
the current production website.

### Vendored asset provenance

The font and logo were copied unchanged from `public/fonts/FiraCode-Regular.ttf`
and `public/img/logo.png` in `mech-lang/web/website` at revision
`74a10b19e9df4a94201da8edc5142d18b701b0eb`. Their SHA-256 values are:

| Asset | SHA-256 |
| --- | --- |
| FiraCode-Regular.ttf | `12c91c94e095ffa5a48b2c2cccc23002cee4daf874e758cb13f773ef68428323` |
| logo.png | `898fc820b87992b5a408508ca041ed5e7ed252b7ff96f159b9d7226f57cdc031` |

The font's name table identifies **Fira Code Regular, Version 1.205** and
the Open Font License. [vendor/OFL.txt](vendor/OFL.txt) is the unchanged
[official license for that release](https://github.com/tonsky/FiraCode/blob/37f16bc199c7618436b0aa7a241030302da2263a/LICENSE),
including its copyright and reserved-font-name notices. The upstream `1.205`
tag resolves to `37f16bc199c7618436b0aa7a241030302da2263a`. The build copies the
notice to `dist/assets/OFL.txt` beside the font. This font license does not
relicense the separate Mech logo.

The unchanged Mika image and reused footer are documented in
[vendor/MIKA.md](vendor/MIKA.md); that notice is copied beside the image in
`dist/assets/`. Its artwork provenance and rights are separate from the font
license. The adapted footer card identifies the workshop's v0.4.0-beta build
and links to its build manifest; it does not claim that a matching public
release or repository tag has been published.

The Pittsburgh hero is a photograph by Cbaile19 released under CC0.
[vendor/PITTSBURGH.md](vendor/PITTSBURGH.md) records the source, license,
resize, and CSS treatment. The builder copies that notice to
`dist/assets/PITTSBURGH.md` beside the image.

## Build

Run these commands from the Mech repository root. Requirements are the pinned
Rust nightly, the `wasm32-unknown-unknown` target, `wasm-pack`, Node.js, and the
normal native Rust build prerequisites. Network access is needed on the first
build if toolchains or dependencies are not already cached. `--offline` may be
added to Cargo commands when the local cache is complete.

Build the native document formatter:

```sh
cargo +nightly-2026-03-03 build \
  --manifest-path benchmarks/iros-2026/blog/render/Cargo.toml -j 2
```

Build the browser package with `browser_compute_canary`; the default empty
feature set does not provide the source compiler and numerical/browser APIs
needed by this page:

```sh
RUSTUP_TOOLCHAIN=nightly-2026-03-03 CARGO_BUILD_JOBS=2 \
  wasm-pack build src/wasm --target web --out-dir pkg --release \
  --no-default-features --features browser_compute_canary
```

The successful local build used `wasm-pack 0.12.1`, Rust nightly
`2026-03-03`, and the following cached/offline invocation from the repository
root. The shared target directory is a local build-cache choice, not a source
dependency; omit that environment override on other machines.

```sh
RUSTUP_TOOLCHAIN=nightly-2026-03-03 \
CARGO_TARGET_DIR=/private/tmp/mech-iros-workshop-20260924/target \
CARGO_BUILD_JOBS=2 \
  wasm-pack build src/wasm --target web --out-dir pkg --release \
  --no-default-features --features browser_compute_canary --offline
```

The crate's release metadata disables `wasm-opt`. The rebuilt v0.4.0-beta WASM
SHA-256 is
`18224e2cf04b246148029bff7bf498ed77c92780495a5e7cfc493a9cd94a8871`.
Its actual `:version` response reports `0.4.0-beta` for all 13 installed
product, library, and host components; these values come from compiled Rust
package metadata, not JavaScript replacements. The earlier browser checks
used SHA-256
`99a400e89a925a959282ef1d2814fef40bd967e8b3f50b3ed03a267da2ac42d5`,
retained as historical provenance in `VALIDATION.md`.
Other compiler or dependency versions can change that binary hash; the
generated build manifest records the package actually copied into the page.

The first article build needs the `public/` directory from the public
[Mech website repository](https://gitlab.com/mech-lang/web/website). It copies
only `img/logo.png` and `fonts/FiraCode-Regular.ttf` into `vendor/`. A local
reference checkout is `/private/tmp/mech-website-discovery-20260926`; its
inspected revision was `74a10b19e9df4a94201da8edc5142d18b701b0eb`.

```sh
node benchmarks/iros-2026/blog/build.mjs /absolute/path/to/website/public
```

Once both vendor assets exist, the argument is unnecessary:

```sh
node benchmarks/iros-2026/blog/build.mjs
```

The builder checks the current presentation-kernel hash, requires every source and figure
slot exactly once, regenerates desktop/mobile chart SVGs, and copies the three archived throughput
records into `dist/evidence/`. It does not rerun native benchmarks. Run the
build again after changing the article, examples, browser host, chart module,
or WASM package. It writes into the existing local output directory; use a
clean checkout or inspect the deployment file list to avoid carrying unrelated
old files into a publication.

The deterministic EKF diagram is retained as `hero.svg`, separately from the
current Pittsburgh article hero. Regenerate the diagram after an
intentional change to the sensor source, drawing, or generated WASM package,
then rebuild the article:

```sh
node benchmarks/iros-2026/blog/hero.mjs
node benchmarks/iros-2026/blog/build.mjs
```

The generator verifies the current presentation-source hash and uses 40 steps of 0.1 seconds,
velocity 1, angular velocity 0.015, and noise scale 0.02. Its position ellipse
uses covariance eigenvectors and twice the square roots of the eigenvalues;
it is not labeled a 95% probability region.

Serve the output over HTTP, not `file://`:

```sh
python3 -m http.server 8765 --bind 127.0.0.1 \
  --directory benchmarks/iros-2026/blog/dist
```

Open <http://localhost:8765/>. If a preview server already occupies that port,
use it or choose a different port. Localhost is a secure browser context for
WebGPU; deployed use requires HTTPS and a browser/device with WebGPU support.
GPU unavailability must be reported rather than replaced with a CPU result.

After a build, format only the authoring template for syntax inspection:

```sh
benchmarks/iros-2026/blog/render/target/debug/mech-iros-blog-render \
  benchmarks/iros-2026/blog/article.mec \
  benchmarks/iros-2026/blog/_build/blog-shell.html \
  /private/tmp/iros-article-syntax-preview.html
```

That file still contains insertion tokens; `build.mjs` creates the complete
page.

## Executed source and browser boundaries

The current presentation source has SHA-256:

```text
f18e37effb2fa63fadca69639f3a8eed218b73218decf65419e78a60b62bb46b
```

`build.mjs` and `verify.mjs` enforce this identity. The source includes every
initial value, prediction/correction equation, integrity constraint and
publication statement. It is the workshop EKF with a revised covariance
publication policy, not a bit-identical rewrite of the archived algorithm's
floating-point execution. It is not the literal
source used to collect the archived timing or source-count measurements.

The September 26 source revision infers all matrix dimensions, uses the
poster-style Unicode mathematical names in the actual source, and writes
column vectors as transposed rows, such as `[0 1 1]'`. The public numerical
exports are `μ` and `Σ`; the live input names `bearing`, `v`, and `w` are
unchanged. Element-kind annotations
remain where they are needed to select f32 arithmetic; mixing one f32 literal
with otherwise untyped f64 literals is not supported by the current compiler.
The preceding presentation source was identified by SHA-256
`a7cd4077c7bf2f9741559b5748f05cf06e9b48e156c4fdbabfbdc7feea065eb2`.
Before the symmetry-stabilization revision below, the compact/Unicode forms
produced bit-identical exported state
and covariance for 256 filters across 40 deterministic sensor turns. Both
rejected NaN and positive/negative infinity, retained identical accepted state,
and recovered after valid input. This is source-equivalence validation, not
a rerun of the archived benchmark campaigns.
The intermediate compact ASCII form had SHA-256
`438c5fd415c3d093b0438f80215846f635e99d6d2caea35cb03a84f79558d8af`;
that Unicode-only revision changed names, not equations or precision. Its
exact source, SHA-256
`cefe87b0ee184f1f30c34c66e626948f6d43236c8449dca0054a68e9e5cd932f`,
is retained as `evidence/ekf-before-symmetry-stabilization.mec`; its historical
equivalence checks still run independently of the current numerical policy.

The current source measures each raw Joseph covariance pair's asymmetry
against `ε + ρ*abs(a) + ρ*abs(b)`, with `ε=1e-4` and `ρ=1e-6`, then publishes
`Σraw*0.5 + Σrawᵀ*0.5`. Raw and projected finite/positive-diagonal checks
remain prerequisites to publication. This prevents accumulated antisymmetry
without letting the projection hide an excessive raw residual. It is not a
PSD guarantee. The [long-horizon diagnostic](evidence/ekf-long-horizon-diagnostic.md)
records the original default-input failures, exact policy, artifact hashes,
bounded CPU/GPU trials, and reproducible helpers. No archived native timing
or source-count measurements were replaced.

The browser compiler parses this source and constructs a fixed-shape numerical
program. CPU turns interpret its lowered instructions in the Rust runtime
compiled to WebAssembly; they do not use the native Cranelift JIT. WebGPU
generates WGSL and a binding manifest from that same compiled program and
submits them through `MechBrowserCompute.Device`. The browser supplies its
platform GPU backend. This is not a fresh measurement of the native Metal
chart.

The host binds `bearing`, `v`, and `w`; other source values are constants for
the selected compilation. Three live inputs give the GPU path eight storage
bindings. The displayed pose and covariance are the first filter in the
selected batch. Backend/batch changes reset the episode; cross-device state
migration is not implemented here. The source listings are read-only, and
there are no inline kernel, function, or matching editors.

The behavior, function, and matching fences execute in the resident root
scope. The EKF's four native named fences belong to the separate checked
kernel integration; only their final publication-stage output is populated
from the numerical host's actual accepted state. All seven native blocks
retain their formatter-generated IDs.
The full expanded Mechdown source, encoded AST, and rendered listings come
from one parse. The EKF's stage headings become introductory prose between
connected listings; other Mechdown headings become comments. The downloaded
EKF retains its `@compute` heading and is compiled separately, byte-for-byte
from the current presentation source, for the numerical demo. The behavior
listing's additional example call demonstrates Run from Paused without
altering its download or the demo's initial mode.

The EKF and behavior transitions execute in Mech. The host passes named mode
and event atoms into the typed Mech state machine and uses its result to schedule
updates. `drawing.mjs` is hand-written JavaScript for sensor simulation and SVG
drawing, not a second EKF implementation and not hidden executable Mech drawing
code. Its source is linked from the expandable demo details. Functions,
matching, and the behavior example are registered in the resident document
REPL; readers can submit further expressions there.
The demo's scheduling state machine uses a separate `WasmRepl` session so
interactive REPL experiments do not change its current mode.

The live timing summary excludes drawing and compilation but includes input
binding, numerical execution, validation, synchronization and one filter's
readback. It uses up to the latest 60 accepted turns after five warmup turns.
These correlated animation samples are not independent benchmark trials;
FPS additionally reflects browser scheduling and display limits.

## Verification

The native bridge tests passed all three cases, and the finite-endpoint
emitter/Naga regressions passed both cases during this implementation. The
reproduction commands below use the recorded nightly and profile. The second
command uses the macOS native-Metal feature to enable its Naga validation path;
it does not dispatch a GPU.

```sh
cargo +nightly-2026-03-03 test --offline -p mech-wasm \
  --profile kernel-bench --no-default-features \
  --features browser_compute_canary --lib kernel::tests -j 2 -- --nocapture

cargo +nightly-2026-03-03 test --offline -p mech-gpu \
  --profile kernel-bench --features metal-native \
  --lib wgsl_f32_finite_endpoints -j 2 -- --nocapture
```

The local runs set the same optional `CARGO_TARGET_DIR` cache override as the
WASM build above. Remove `--offline` when dependencies must be fetched.

The generated JavaScript/WASM interface has a smoke test that needs no browser:

```sh
node src/wasm/tests/kernel-smoke.mjs \
  benchmarks/iros-2026/blog/source/behavior.mec
```

It checks actual WASM CPU execution, source identity, input validation,
single-instance state sampling, rejected-turn rollback, recovery, reset,
WGSL manifest generation, and six Mech behavior transitions. It does **not**
dispatch a GPU. WGSL endpoint emission additionally has native regressions
for exact `±f32::MAX` bitcasts and Naga parsing/validation; this preserves the
paper's finite-value guards without changing their source.

This smoke test passed with Node 26.8.1 and the package hash above. The generated
package can produce a Node module-type warning without failing these checks.

The resident-document smoke test exercises the shipped initializer, requires
one runtime download for concurrent hosts, loads the encoded document and
source bundle, checks all 13 compiled component versions, reads the three
native resident outputs, checks ownership of all four named EKF stages and
the single live kernel-output marker, and submits arithmetic, function,
all six state-machine transitions, and matching probes through the actual
resident REPL. It also compares the current EKF against the retained
`src/wasm/tests/fixtures/paper-ekf.mec` source at the bit level across 256
filters and 40 turns, with non-finite rejection, whole-batch rollback, and
recovery:

```sh
node benchmarks/iros-2026/blog/test-document-runtime.mjs
node benchmarks/iros-2026/blog/test-language-examples.mjs
node benchmarks/iros-2026/blog/test.mjs
```

These checks require a rebuilt `dist/`; they do not validate a stale earlier
page or dispatch a GPU. The shared controller's focused navigation regression
is separately available as `node scripts/test-document-presentation.mjs`;
the workshop itself uses full document startup, not presentation-only mode.
The language-example test separately exercises all 12 mode/event combinations,
unknown and numeric input rejection, recovery, typed enum assignment, functions,
and matching against the generated WASM package.

For real device verification, open the local page's Output pane and select
**Verify CPU / GPU agreement and rollback**. The read-only page loads the exact
current presentation source;
verification also enforces its hash.
The verifier runs in separate sessions from the displayed demo and reports a
structured result. It compares 20 deterministic turns, injects NaN in the
last lane, requires bitwise whole-batch rollback, and compares every instance
after recovery. Its numerical tolerance is
`1e-4 + 1e-4 * max(abs(cpu), abs(gpu))` per finite component. Full readbacks used
for this proof are not included in the demo's turn-time display.

A passing GPU result requires `status: "passed"`, `passed: true`,
`gpuExecuted: true`, and no numerical failures. `status: "unsupported"` can
still include passing CPU checks but is not GPU evidence. Record the browser,
adapter, source hash and returned report when documenting a device check.

**Earlier-build browser verification passed on 2026-09-26.** The in-app browser
executed real WebGPU for 20 paired turns and recovery across 256 filters.
Maximum absolute differences were `3.814697e-6` for mean state and
`1.068115e-4` for covariance, within the element-wise tolerance above. NaN in
lane 255 caused rejection with bitwise-identical whole-batch accepted state;
recovery passed for all 256 filters. Additional UI checks exercised 4,096 GPU
filters, an invalid bearing in lane 4,095, displayed-state retention, the
latched Fault mode and reset, and both small language examples. See
[VALIDATION.md](VALIDATION.md) for the recorded scope and values. These are
browser correctness checks, not new native throughput measurements. That
historical pass predates the current read-only native-block/resident-REPL
integration; current-build checks are identified separately in that record.

**The preceding resident-REPL page also passed on 2026-09-26:** three native
resident outputs, the kernel-fed EKF output, REPL `40 + 2` → `42`, no inline
source editors, and the same real CPU/WebGPU verification. Desktop and
390 px mobile layouts had no horizontal page overflow, and all five mobile
figure variants fit their containers. Runtime/static/mobile/lifecycle tests
and all 19 shared style contracts passed. Hero, footer/backmatter, and mobile
pipeline visual checks also passed, as did mobile REPL input and accepted
CPU/GPU turns with matching live block output. This covers local desktop and
390 px responsive layouts, not physical-phone hardware or production checks.
It predates the four-stage EKF layout, Pittsburgh hero, inferred-dimension
source revision, and v0.4.0-beta rebuild; see separately dated validation
entries for those subsequent changes.

Also inspect normal UI behavior: run/pause/single step; each batch size; CPU
and GPU where available; velocity and noise changes; invalid input entering
Fault without changing displayed accepted state; reset; the three resident
outputs and final EKF publication output; REPL input; application persistence
when switching Console/Output and fullscreen views; and the absence of inline
source editors. Check the
complete footer, footnotes, hero, fitted mobile figures, browser console, and
layout at desktop and narrow widths. Neither a successful HTML build nor
the Node smoke test substitutes for those browser checks.

## Archived evidence

The four charts are derived from these records under `../results/`:

| Figure | Record | Execution boundary |
| --- | --- | --- |
| Six-system CPU | `apple-m1-cpu-equal-n10-2026-09-24.json` | Eight workers; selected rows fuse 40 turns. Mech checked/unchecked measurements share each process trial. |
| Five Mech backends | `apple-m1-mech-backend-pairs-n10-2026-09-25.json` | Same archived source, seven live bindings, five warmups, then 40 turns with per-turn publication; a fresh process per backend/mode case. |
| Six-system Metal | `apple-m1-metal-equal-n10-2026-09-24.json` | Resident state, completed publication after every turn. Rust is a Rust host plus handwritten MSL. |
| Same-source CPU/Metal | Both September 24 records | Mech **per-turn** CPU row, Taichi and Halide; no September 25 Mech substitution. |

Every plotted mode has ten retained measurements, summarized as median ±
unscaled MAD. MAD is descriptive spread, not a confidence interval. The CPU
and Metal records identify different Mech revisions and retain the untimed
CPU validation-tolerance patch. Read recorded source hashes and working changes,
not only the branch's current commit. The five-backend campaign uses archived
kernel SHA-256
`da531cddcb25d002d49f1a77800122e84b573e2685e191c1955908ef6fccd625`.

The article's normalized-character audit and checked dylib table are separate
experiments, linked in the article rather than recomputed by this build. The
dylib experiment uses seven processes per library, 10,000 filters, 200 turns,
one worker, and a minimal ABI loader. Its RSS is whole-process peak memory,
not private library memory. The scalar Rust dylib is not the packed SIMD Rust
implementation from the CPU chart. Browser throughput and REPL experiments
never replace or modify these archived results.

## Publication destination

The user confirmed the exact URL on 2026-09-26:
<https://mech-lang.org/iros-r4r-2026/index.html>.
The destination repository is
[mech-lang/web/website](https://gitlab.com/mech-lang/web/website), and the
complete generated bundle belongs only in `public/iros-r4r-2026/`.
Do not replace `public/index.html` or publish to the separate About repository.

The earlier About destination was an unverified assumption, not a decoded QR
result. That mistaken publication was reverted by About commit `10bf996`.
The restored About repository tree matches its pre-publication revision
`c56cc3a` exactly. The obsolete About staging script has been removed.

After explicit publication approval:

1. Refresh the website repository and preserve any intervening changes.
2. Create a review branch and run
   `node benchmarks/iros-2026/blog/stage-website.mjs /path/to/website-checkout`.
   The script requires a clean checkout and an absent workshop directory.
   It copies only into `public/iros-r4r-2026/` and does not push.
3. Review the complete file list, verify that all changes are under that
   directory, and test the page at its nested path before publishing to `main`.
   Do not change DNS, the homepage, or the existing Pages configuration.
4. Observe the successful Pages job, then verify the exact public URL, WASM,
   source hashes and browser checks. Confirm the main homepage and About page
   are unchanged.

The current development browser package is approximately 49 MB uncompressed
(49,168,648 bytes in the inspected build). The builder also emits
`_mech/pkg/mech_wasm_bg.wasm.gz`, approximately 6 MB. Where `DecompressionStream`
is available, the page fetches this compressed file and decompresses it before
WASM initialization. It checks gzip magic bytes so hosts that already decode
`Content-Encoding: gzip` are not decoded twice. Browsers without that API, or a
server that does not supply the gzip file, use the retained raw package.

This works with the ordinary local Python HTTP server and does not require
server-side compression configuration. Deploy both variants and serve raw WASM
with an appropriate WebAssembly MIME type. The static test verifies that the
gzip decompresses to the exact raw bytes; the build manifest identifies the
raw module. The deployable artifact must contain the complete runtime, not
just `index.html`.
