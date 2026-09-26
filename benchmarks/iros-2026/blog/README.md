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
  complete Mech examples displayed by the page.
- `render/` is a small native `mech-syntax` parser/HTML formatter executable.
- `build.mjs` formats the article and examples, inserts the actual Rust host
  programs, charts, diagram and UI, and copies their runtime dependencies.
- `blog-shell.mjs` composes website navigation and the workshop host with
  `include/blog.html`. It does not replace the shared hero, content columns,
  metadata, TOC placement, or backmatter markup.
- `include/palette.css`, `mech-source.css`, `mechdown.css`, `style.css`,
  `blog.css`, and `document.js` are copied unchanged into the publication.
  The shared controller's presentation mode supplies TOC expansion, active
  sections, mobile navigation, and scroll restoration without starting a
  second Mech runtime. `article.css` styles only workshop examples and figures.
  Run `node scripts/test-document-presentation.mjs` for the shared startup and
  scroll-aware TOC regression checks.
- `charts.mjs` recomputes median and unscaled MAD from all ten retained samples
  for each row/mode and checks them against the archived summaries.
- `app.mjs` connects the browser controls to Mech; `drawing.mjs` supplies
  synthetic observations and SVG rendering; `verify.mjs` checks CPU/GPU parity.
- `dist/index.html` is the deployable page. `dist/article.mec` is the expanded,
  downloadable article with the complete Mech and Rust listings, rather than
  the template tokens. Its live UI is supplied by the accompanying browser
  host files; the article text alone is not a standalone UI bundle.
- `dist/build-manifest.json` records build time, repository revision, dirty
  worktree status, and hashes of the kernel, behavior source, WASM and copied
  evidence. Retain it with a publication.

`_build/`, `render/target/`, and `dist/` are build outputs. The small `vendor/`
directory retains the website logo and Fira Code font for subsequent builds.
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
CARGO_TARGET_DIR=/private/tmp/mech-iros-workshop-20260924/target \
CARGO_BUILD_JOBS=2 \
  wasm-pack build src/wasm --target web --out-dir pkg --release \
  --no-default-features --features browser_compute_canary --offline
```

The crate's release metadata disables `wasm-opt`. The inspected resulting WASM
SHA-256 was
`99a400e89a925a959282ef1d2814fef40bd967e8b3f50b3ed03a267da2ac42d5`.
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

The builder checks the paper-kernel hash, requires every HTML slot exactly
once, regenerates the four SVGs, and copies the three archived throughput
records into `dist/evidence/`. It does not rerun native benchmarks. Run the
build again after changing the article, examples, browser host, chart module,
or WASM package. It writes into the existing local output directory; use a
clean checkout or inspect the deployment file list to avoid carrying unrelated
old files into a publication.

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

The paper source has SHA-256:

```text
a7cd4077c7bf2f9741559b5748f05cf06e9b48e156c4fdbabfbdc7feea065eb2
```

`build.mjs` and `verify.mjs` enforce this identity. The source includes every
initial value, prediction/correction equation, integrity constraint and
publication statement. It is the numerically tested compact presentation
of the archived EKF, with rewritten guard expressions. It is not the literal
source used to collect the archived timing or source-count measurements.

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
selected batch. Backend/batch changes and applying edited source reset the
episode; cross-device state migration is not implemented here.

The EKF and behavior transitions execute in Mech. The host passes numeric mode
and event codes into the Mech state machine and uses its result to schedule
updates. `drawing.mjs` is hand-written JavaScript for sensor simulation and SVG
drawing, not a second EKF implementation and not hidden executable Mech drawing
code. Its source is linked from the expandable demo details. The small
functions and matching examples are separately executed through `WasmRepl`.

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

For real device verification, open the local page and select **Verify CPU /
GPU agreement and rollback**. Restore the paper source before this check:
verification intentionally rejects an edited kernel with a different hash.
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

**Actual browser verification passed on 2026-09-26.** The in-app browser
executed real WebGPU for 20 paired turns and recovery across 256 filters.
Maximum absolute differences were `3.814697e-6` for mean state and
`1.068115e-4` for covariance, within the element-wise tolerance above. NaN in
lane 255 caused rejection with bitwise-identical whole-batch accepted state;
recovery passed for all 256 filters. Additional UI checks exercised 4,096 GPU
filters, an invalid bearing in lane 4,095, displayed-state retention, the
latched Fault mode and reset, and both small language examples. See
[VALIDATION.md](VALIDATION.md) for the recorded scope and values. These are
browser correctness checks, not new native throughput measurements.

Also inspect normal UI behavior: run/pause/single step; each batch size; CPU
and GPU where available; velocity and noise changes; invalid input entering
Fault without changing displayed accepted state; reset; source edit and
restore; and the functions/matching example editors. Check the browser console
and layout at desktop and narrow widths. Neither a successful HTML build nor
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
implementation from the CPU chart. Browser throughput and edited examples
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
(49,325,199 bytes in the inspected build). The builder also emits
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
