# Standard-library extraction: measured boundary and feasibility experiment

Baseline: `c4777b7015fe8ff47fdfa48d18606c49aace7d97`. This report uses the file census in `data/census.json`. Units are physical text lines and bytes, including comments; mixed production/test files remain a separate category. Macro expansion and dependency source are excluded. The extraction experiment is under `/private/tmp/mech-v04-extraction`, with the remaining Mech tree in `mech/` and the external owners in `external/stdlib` and `external/machines`. The original `mech/src/stdlib` and `mech/machines` paths are absent.

## Checkpoint conclusion

The existing public catalog and runtime interfaces support an external standard library for the maintained scalar-add bytecode consumer. The same consumer passed before and after relocation, returning the independently specified `F64(3.0)` result through the resident engine path. The initial relocation changed zero Rust implementation lines. Direct execution of the external mathematical factory and generated resident application are recorded separately below. The experiment rewrote 25 manifests and introduced an independent workspace/patch table for `mech-stdlib`.

The selected public catalog and resident runtime boundary is verified. Migration acceptance requires the remaining profile, packaging, test-ownership and CI checks listed below. Individual results are retained in `evidence/extraction/` and updated below.

## Ownership decisions

| Component | Disposition | Reason and required treatment |
| --- | --- | --- |
| Nine machine crates: combinatorics, compare, logic, math, matrix, range, set, stats, string | Move | Their runtime kernels, factories, optional specializers/lowerers, manifests, private tests, examples, benchmarks, README/license material and local CI belong to the external operation owners. The experiment moves the complete trees. |
| `src/stdlib/src`, manifest | Move | Distribution composition selects engine intrinsics and concrete external installers. Its existing public `runtime_catalog`, `source_catalog`, `install_runtime`, and `install_source` are the interface. |
| `src/stdlib/tests` (9 files, 3,191 lines) | Separate | These cover Mech distribution identities, canonical types, resident memory, specialization and source integration. Keep Mech integration ownership; re-home the test targets while consuming the external library. The second experiment stage re-homes them in the Mech-owned `tests/stdlib-integration` crate; its 52-line manifest consumes the external library. |
| `src/engine/src/intrinsics` (27 files, 31,339 lines) | Retain | Access, assignment, variable definition, conversion, structural concatenation/comprehension and table operations implement language/runtime behavior. These contracts remain part of language and runtime execution after machine extraction. Public intrinsic installers let external distributions select them. |
| `src/core/src/stdlib.rs` (2,686 lines; 35 macros) | Separate | This file provides exported SDK kernel/factory/lowering machinery. Retain its public contract for the first extraction. A later separation of family-expansion helpers requires external macro-hygiene, feature and lowering validation; deletion credit is zero pending that validation. |
| Core catalog, operation signatures, value representations; ABI; runtime admission and publication | Retain | Independent libraries and engine intrinsics require shared stable IDs, representations and invocation contracts. Runtime construction remains distribution-neutral and receives a catalog explicitly. |
| Root/WASM product feature forwarding | Separate | Product selection remains Mech-owned. Replace path dependencies with pinned external versions or explicit development overrides without removing product features. |
| `tests/architecture/{function-system,distributions}`, catalog closure and product canaries | Retain | Mech owns standard/full identities, exact native closure and complete product compatibility. Their scope is external dependency compatibility; operation-private suites belong with their implementations. |
| `docs/stdlib` | Separate | The two pages mix concrete library operations with engine intrinsic entries. Split those sections; the missing-functions page still refers to the obsolete `src/engine/src/stdlib` path. |
| `.github/ci/owners.toml`, integration workflows, static profile/linkage/archive scripts | Separate | Keep Mech integration gates, route machine-private suites to external owners, and replace in-tree location assumptions. The local machine `.gitlab-ci.yml` files move with each owner. |
| Proven obsolete implementation | Delete: none | Deletion credit is zero. Usage and compatibility analysis governs any later removal. |

`data/extraction-boundaries.json` records file-level decisions, census roles and physical measurements. The test-owner adapter is measured at 52 manifest lines. Version/release coordination, generated workspace resolution and CI migration costs remain unestimated.

## Accounting

A coarse move of `src/stdlib` and all `machines/` removes **303 files and 44,508 physical lines** from the Mech directory. It contains 27,828 lines of pure production files, 9,205 lines of mixed production/test files, and 7,475 lines of supporting material. Subtracting only `src/stdlib` would account for just 4,074 lines and omit 40,434 machine-tree lines.

The proposed ownership estimate retains the 3,191 lines of Mech integration tests. Retaining the 10-line test linker script as well gives **41,307 moved lines** and **784,947 of the baseline 826,254 lines remaining before new boundary infrastructure**. Role-separated physical counts describe repository composition. Exact role-separated baseline, moved, remaining and combined totals are in the JSON.

The final measured ownership stage has the following composition. The test linker script resides in the retained integration test crate.

| Scope | Pure production lines | Mixed production/test lines | All physical lines | Files |
| --- | ---: | ---: | ---: | ---: |
| Baseline Mech | 235,500 | 286,725 | 826,254 | 1,976 |
| Remaining Mech, measured | 207,666 | 277,520 | 784,987 | 1,684 |
| External library, measured | 27,828 | 9,205 | 41,325 | 293 |
| Combined maintained trees | 235,494 | 286,725 | 826,312 | 1,977 |

The six-line combined Rust reduction comes from the separate source-profile cfg repair. Supporting configuration increases by 64 lines. Combined maintained material increases by 58 lines. The extracted lock refresh removes five dependency-list entries while preserving every resolved package/version pair.

For every role, the arithmetic is:

`remaining = baseline − moved − deleted + added boundary material`.

In the measured coarse experiment, deletion is zero and added Rust boundary implementation is zero. The remaining Mech directory contains the baseline files outside the two moved trees, with manifest paths adjusted. External implementation still exists and remains maintained. Relocation leaves combined Rust implementation unchanged. A separate baseline source-profile repair removes eight Rust lines and adds two. Added workspace metadata and later adapters are reported separately; a moved line receives no deletion credit. The JSON preserves the coarse first-stage measurement and the later test-owner stage.

## Reproduction

From the baseline audit checkout:

```sh
python3 scripts/audit-v04-extraction.py /private/tmp/mech-v04-extraction
python3 audit/v04/evidence/extraction/measure.py
```

Use a new destination directory for each preparation run. The preparation script exports the exact baseline using `git archive`, moves both trees, rewrites dependency paths by their actual previous targets, removes the old stdlib workspace member and creates an external stdlib workspace with explicit machine patches. It records relocations in `evidence/extraction/preparation.json` and the complete manifest change in `extraction-manifests.patch`. The original library paths remain absent; manifests resolve the external locations directly.

The baseline selected fixture has no tracked `Cargo.lock`. The first `--locked` attempt correctly failed. Resolve its lock offline once, preserve it, and copy the identical lock to the extracted fixture before comparing:

```sh
CARGO_TARGET_DIR=/private/tmp/mech-v04-target-extraction/baseline \
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 \
cargo +nightly-2026-03-03 run --offline \
  --manifest-path tests/fixtures/bytecode-runtime-consumer/Cargo.toml \
  -- tests/architecture/bytecode-v1/scalar-add-f64.mecb

# Run from /private/tmp/mech-v04-extraction/mech after copying that lock.
CARGO_TARGET_DIR=/private/tmp/mech-v04-target-extraction/extracted \
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 \
cargo +nightly-2026-03-03 run --offline --locked \
  --manifest-path tests/fixtures/bytecode-runtime-consumer/Cargo.toml \
  -- tests/architecture/bytecode-v1/scalar-add-f64.mecb
```

Both commands inject `mech_stdlib::runtime_catalog()` into `RuntimeBuilder`. They use the resident bytecode loader and assert its returned numeric value. The consumer explicitly installs the selected catalog through the public builder.

## Build and dependency measurement

The selected fixture has two direct dependency declarations. `mech-stdlib` declares two required SDK dependencies and nine optional machine dependencies; the nine machine manifests additionally declare their normal and development dependencies. The per-manifest declared counts and feature counts are in the boundary JSON. Resolved compiled dependencies are measured independently from these declarations.

For the actual scalar-add consumer, both resolved normal/build graphs contain **106 distinct packages**, including the consumer. `cargo tree -e normal,build` excludes development edges. Baseline and extracted fixture lockfiles have the identical SHA-256 `0293f0b1da01fa1cac449302c6237ac6155f858b7d2c8847474cc0fd8b7922c7`.

| Native artifact, unoptimized dev profile, debug=0, incremental=false | Bytes |
| --- | ---: |
| Baseline executable | 56,245,152 |
| Extracted executable | 56,148,416 |

Measurement scope: linked Mach-O files under the stated development profile. Release-product size and memory usage are unmeasured here. Different source locations can change embedded paths and linker layout. The resolved package count and combined implementation stay constant across this relocation. SHA-256 identities and section measurements are in `selected-build-comparison.json` and the `*-selected-sections.txt` files. Recorded build elapsed times include cache and environment effects; controlled performance comparisons are outside this measurement.

## Discovered coupling and validation matrix

The matrix follows the current composition contract: standard/full runtime, source and compiler layers; selected runtime consumer; browser document and compute profiles; generated application construction. Each layer is checked separately with `--no-default-features`.

- External `standard_runtime`: compilation passed.
- External `standard_compiler`: compilation passed.
- External `standard_source`: the original build failed under `-D warnings`. Restricting two unused engine helpers to their `semantic-compiler` consumers then exposed a missing feature dependency: stdlib source catalog installation requires engine semantic compilation. The repair forwards `semantic-compiler` to the engine and selected machines. Repaired baseline and extracted standard-source builds pass. The resolved graph includes semantic compilation and excludes Mech `compiler` features and `mech-bytecode`. Original failure logs and both patches are retained.
- External `full_runtime`, `full_source`, `full_compiler`: compilation passed. WASM `browser_project_core`: compilation passed after refreshing the extracted root lock. WASM `browser_compute_canary` and native `standard_compiler,compute_backends_native` compilation also passed. Exact results are retained as command/result JSON.
- Selected bytecode execution: passed before and after extraction, with output `F64(3.0)`. External direct-kernel execution also passed: the public catalog selects `AddSS<f64>`, binds it through `ExecutionTarget::DirectRuntime`, creates a managed `SpecializedFunction`, and returns 3 for inputs 1 and 2. Updating the first input to 5 and executing the same instance returns 7. This separately verifies the external machine implementation and accepted output changes.
- Generated-native resolution: public workspace and registry planning both succeed for the scalar-add artifact. That artifact selects `mech-core`, `mech-engine` and `mech-runtime`; its mathematical operation executes through the resident engine. `src/engine/src/resident/numeric/mod.rs:691` registers resident math/add. `standard_workspace_registry` still maps explicit machine package requests to `machines/*`, so that separate location boundary requires migration. The registry-generated manifest preserves exact version selections; a development copy uses explicit local SDK patches following the existing `registry_generated_project` test. That copy builds and its executable returns `3` with `--once`. Its linked development artifact is 42,870,720 bytes with SHA-256 `0b5e2249d7eb471eb55747871188bc6b1ec82f0598ae08930e776e237aabdad1`.
- Independent stdlib integration tests: three fixture includes initially resolved outside the external package and were absent. Re-homing the existing tests under `tests/stdlib-integration` preserves their root fixture ownership. All three includes now resolve. The selected profile contract compiles and verifies three runtime factories, zero source specializers/intrinsics/exports, and digest `a006c5b25aa925939f4973273e2aea9cac2897fbcca32dc25edd6be74631445d`. Rust test lines changed: zero.
- Machine development dependencies: matrix and combinatorics tests depend on engine/runtime/syntax/bytecode SDKs by relative paths and define separate patch tables. The preparation rewrites these declarations; private full-matrix execution remains unverified.
- Generated inputs: the inspected machine production source has no external generated-source includes in machine production sources. `machines/string/src/lib.rs` includes package-local allocation-probe test support, which moves intact. `mech-stdlib/build.rs` supplies a macOS test linker setting and moves to the retained integration test crate.
- CI and spelling checks: `path-coupling.txt` captures 172 references in the inspected build/check/contract locations. The count measures references; each migration change requires scope-specific validation. Owner configuration selects integration test targets on `mech-stdlib` and watches `machines/**`; the targets now belong to the retained test harness, and path ownership needs external routing.

## Scope and pending acceptance

Verified scopes include external catalog composition, selected resident consumer execution, profile compilation and retained integration-catalog identity. Registry publication, machine-private full matrices, default/full CLI release builds, browser execution, GPU completion and final CI migration remain unverified in this extraction experiment. WASM evidence records compilation. Permanent migration acceptance remains pending the outstanding checks.

Audit additions include the preparation/measurement scripts, evidence wrappers, this report, JSON data and the native public-boundary probe. They are counted as supporting audit material. Final added-line accounting is part of the parent audit delta.

## Experimental revisions and remaining package boundary

The source-only feature defect is present in the original checkout. Its two repairs are preserved in `source-only-cfg-fix.patch` and `source-profile-feature-fix.patch`. Re-homing the test linker build script while earlier Cargo jobs were queued produced two harness errors: a missing build script and a test-linker directive on a package whose integration tests had moved. The script was restored for the queued jobs, and the external library now sets `build = false`; the retained test crate owns the active linker setting. The temporary inactive external copy was removed after those jobs completed. These logs identify experimental sequencing errors separately from the baseline configuration defect.

`cargo package --list` successfully enumerates the external library inputs. Offline package construction fails because the local crates.io index has no matching `mech-engine` package. A published/pinned SDK or a separately configured local registry is required for registry-only packaging. The stdlib manifest also retains `publish = false`; a registry release requires an explicit publishing policy change. Local path and development-patch compilation provide the present separation evidence.

## Reproducing the second ownership stage

Use the complete preparation path in a new directory:

```sh
python3 scripts/audit-v04-extraction.py /private/tmp/mech-v04-extraction-reproduction --finalize
python3 audit/v04/evidence/extraction/measure.py \
  /private/tmp/mech-v04-extraction-reproduction \
  /private/tmp/mech-v04-extraction-reproduction-counts.json
```

`--finalize` exports the locked baseline, applies both recorded source-profile patches, re-homes the nine test files and linker script, creates the retained test manifest, removes external dev-dependency declarations, and refreshes the root lock offline. The script requires the recorded toolchain and cached Cargo dependencies. The fresh reproduction returned exactly 784,987 remaining, 41,325 external and 826,312 combined physical lines. `reproducibility-check.json` records matching files, bytes and physical/nonblank lines for both independently prepared trees.

The core increase over the ownership estimate is 40 lines: 52 harness manifest lines, minus six Rust cfg lines, one workspace member and five lock dependency entries. The external increase is 18 lines: 13 workspace/patch lines, minus six dev-dependency lines, plus ten source-feature lines and one explicit build setting.

`native-probe.rs` is the public catalog and native-planning probe. Its standalone development manifest points to `mech-build`, `mech-core` and external `mech-stdlib` with `runtime,f64,math_add`, and explicitly patches all nine external machine crates. The exact probe manifest and generated manifest are retained with the evidence. `build-generated.py` creates a development copy of the registry-generated application and records its five local SDK overrides.
