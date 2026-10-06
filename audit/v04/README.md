# Mech v0.4 audit and executable demonstrations

Baseline: `c4777b7015fe8ff47fdfa48d18606c49aace7d97`. The working branch is `codex/v04-audit-demonstrations`. The original working checkout and its uncommitted files are preserved. See `data/baseline.json` for the original state, lockfile identity, comparison choice and isolation method.

## Read the audit

- `report.md`: measurements, findings, extraction accounting and status.
- `architecture.md`: responsibility map, historical requirements and four implementation walkthroughs.
- `data/census.json` and `data/census.csv`: every tracked baseline file with blob identity and two independent attribution dimensions.
- `data/capabilities.json`: behavior, interfaces, configuration, stage-specific evidence and limitations.
- `data/findings.json`, `data/review-coverage.json`, `data/subtraction.json`: scoped manual review and proposed revisions.
- `extraction.md`, `data/extraction-boundaries.json`, `evidence/extraction/`: separation experiment and measurements.
- `streaming-design.md`: adapter boundary, publication identities and measurement scope.

## Regenerate the census

From the audit checkout root, use Python 3.11 or later:

```sh
python3 scripts/audit-v04.py --capture-environment
```

The generator reads preserved Git blobs locally. The counter records UTF-8 physical LF-delimited lines and nonblank lines, including comments. Binary blobs have zero text lines and retain their byte counts. Every physical baseline line occurs once in each grouped view. Explicit generated outputs are attributed separately from their generators. Macro-expanded source is excluded.

Files containing `cfg(test)` remain in a mixed production/test bucket. The report gives a pure-production subtotal and a mixed subtotal. File-level roles are derived from documented path rules; detailed manual review is an independent record.

The historical comparison is the `v0.3.5-beta` release ancestor, `f7d551ca8914a72df575fd40b94dba82d1d23fe6`. Rename detection uses Git's 50% similarity threshold. This comparison measures the release-to-integration interval. The capability register separately attributes new and changed behavior.

## Build the browser package

The audit extends the maintained `scripts/build-wasm.py` browser-compute feature selection with optional inspection exports and exact integer widths. The normal browser profiles retain their existing exports.

```sh
wasm-pack build src/wasm --target web --out-dir pkg --no-default-features --features browser_project,browser_compute,u8,u64,u128,syntax_inspection,type_inspection,i64_publication
python3 audit/v04/record-wasm-artifact.py
python3 audit/v04/prepare-demo-fixtures.py
```

The recorder verifies required exports, copies the package into the site, creates gzip transport chunks and records exact source and executable hashes. `site/pkg/` is generated and ignored by Git. The current package uses the existing release settings with wasm-opt disabled in the package manifest.

## Run the website

Serve the repository root locally:

```sh
python3 -m http.server 8764 --bind 127.0.0.1
```

Open `http://127.0.0.1:8764/audit/v04/site/`. The atlas reads the generated JSON directly. The demonstrations load the `mech-wasm` package built from this checkout. Package identity and precise feature selections are recorded alongside the build evidence. The website runs on localhost.

The streaming adapter calls Mech’s `DocumentStream` and `DocumentSession` APIs. Application demonstrations use the retained runtime and maintained browser compute bridge. Browser transport preserves wide integers exactly using strings or `BigInt`.

The document editor shows syntax diagnostics while editing and compiles with Ctrl+Enter. Its preview attaches error indicators to canonical source regions. The preview marks the rendered code and connects its error ranges to numbered callouts. Source excerpts, caret underlines, secondary labels and suggested fixes in Diagnostics are formatted in Rust. The verification records for this interface are in `evidence/rich-diagnostics/`.

## Validation ownership

The baseline's `.github/ci/owners.toml` defines maintained owner commands. Each executed audit check records its actual command, features, environment, exit status, test count and evidence level. A successful zero-test build is recorded as such. Each feature closure and target has its own validation status.

The extraction experiment is reproduced with `scripts/audit-v04-extraction.py` and the commands recorded in `extraction.md`. It uses a separate temporary layout for the relocation experiment.

Build, browser and extraction commands, artifact hashes and outcomes are retained under `evidence/` and indexed in `data/evidence.json`. Failed attempts remain available with their corrections. Source additions, modified files and report/data costs are included in `data/audit-additions.json`. The baseline census remains tied to its preserved commit.

## Execute the demonstrations

With the server running, use the existing Chrome harness through these entry points:

```sh
python3 audit/v04/check-atlas.py
python3 audit/v04/check-streaming-browser.py
python3 audit/v04/check-preview-annotations.py
python3 audit/v04/check_types_browser.py
python3 audit/v04/check-product-browser.py
python3 audit/v04/check-diffusion-browser.py
python3 audit/v04/check_trust_browser.py
python3 audit/v04/check_inventory_browser.py
python3 audit/v04/index-evidence.py
python3 scripts/audit-v04.py
```

The heat-diffusion workload uses independently calculated neighbour fluxes and a literal first-step oracle. The inventory workload checks receipts, changing arrivals/demand, rejected updates and accepted state. The maintained n-body application remains available as additional qualification.

PR #829 is absorbed at revision `9ae5c00e0184fe341d2ff2bdc590ede712009d2f`. Its source proof can be reproduced with `./scripts/demo-r-stack.sh --dump all`; the browser extension is `site/trust.html`. The absorption record describes adaptations to current canonical compiler interfaces.
