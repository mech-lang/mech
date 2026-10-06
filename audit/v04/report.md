# Mech v0.4 audit

This audit uses integration commit `c4777b7015fe8ff47fdfa48d18606c49aace7d97`, fetched on 2026-10-06. Its package manifest reports version `0.3.6`; the branch designation is `integration/v0.4`. The checkout was clean before audit additions. Original local changes in the user's separate checkout are excluded and preserved.

## Baseline composition

The tracked census contains **1,976 files, 29,930,045 bytes and 826,254 physical text lines**. Physical lines include comments and supporting material. Binary contents contribute bytes but no text lines. The source-file language and source role are independently available for filtering.

| Role | Files | Physical lines |
|---|---:|---:|
| Production files without embedded test attribution | 709 | 235,500 |
| Files mixing production and embedded tests | 267 | 286,725 |
| Separate test files | 421 | 196,525 |
| Fixtures | 167 | 19,008 |
| Generated outputs | 11 | 4,478 |
| Build tooling | 97 | 34,278 |
| Configuration and lockfiles | 59 | 14,791 |
| Documentation | 155 | 22,334 |
| Examples | 52 | 5,018 |
| Benchmarks | 25 | 5,384 |
| Licenses | 12 | 2,213 |
| Other binary asset | 1 | 0 |
| **Total** | **1,976** | **826,254** |

The 286,725 mixed lines are reported in a separate production/test category. Their file records identify the embedded test attribution. Explicit generated grammar, typed AST and data outputs have known generator records; expanded macros are excluded. This census measures stored source. Build records separately report artifact size and elapsed time.

The prior release ancestor `v0.3.5-beta` contains 572 tracked files and 130,374 physical lines under the same method. The net release-to-integration increase is **695,880 lines across all roles**. The interactive history exposes file renames, additions, removals and modifications. The capability register attributes behavioral changes; the extraction records quantify relocation and remaining implementation.

Independent checks reconcile tracked blob count and byte totals against `git ls-tree -rlz`. Grouped role/subsystem totals and historical line deltas reconcile exactly in the generator. Path attribution has no unresolved role after inspection of browser assets, host configuration, build seed, installer and Mika documentation. Responsibility classification remains a navigational approximation; reviewed spans and contracts provide the semantic explanation.

## Architecture and capabilities

`architecture.md` explains source-to-activation, input-to-publication, edit-to-reconciliation and compute-to-completion paths. The capability register contains 24 scoped entries. It distinguishes parsing, semantic validation, artifact construction, activation, execution and correct publication. Declared Cargo dependencies, resolved build dependencies and runtime interactions have different meanings.

Initial executed checks include 44 streaming/editor tests, 17 core type-registry tests, six core interval-publication tests and 14 canonical descriptor/shape tests. These establish their listed configurations and API boundaries. Native and browser application qualification has separate evidence records. Two initial feature-minimal checks ran zero tests; their records identify that coverage explicitly.

The recorded quality findings are tied to inspected code, consequences, proposals and acceptance checks. Principal findings are:

1. The baseline runtime compiler discards structured semantic source anchors. The correction preserves the typed error and projects its matching document range; browser selection now highlights the exact rejected literal after a Unicode prefix.
2. The earlier editor contract required fragment parsing with full parsing as fallback. The baseline unconditionally parses the entire edited document before identity reconciliation. The historical commitment remains unresolved and requires a scope decision.
3. Scalar and option conversion tables repeat semantic policy. Consolidation needs complete nested-option/Dynamic behavior checks before any code reduction claim.
4. Selected architecture gates inspect source spelling. Dependency prohibition checks establish a static boundary; executed fixtures establish behavior. The subtraction queue identifies checks that can be consolidated.

Manual review is limited to the recorded spans in `data/review-coverage.json`. File size identifies inspection candidates. Memory-safety and numerical-accuracy claims require their respective invariant and reference checks.

## Standard library extraction

The initial physical move candidate comprises `src/stdlib` and all nine machine crates: **303 files and 44,508 physical lines**. That includes 27,828 pure-production lines and 9,205 lines in files mixing production and tests. Associated tests, fixtures, manifests, packaging and CI responsibilities are separately classified in the extraction records.

The proposed ownership boundary retains 3,191 lines of Mech integration tests and their 10-line build script. Its estimate moves 41,307 lines and leaves **784,947 lines before new boundary infrastructure**. This differs from the coarse directory experiment. With the directory move alone, the all-role equation is:

```text
826,254 baseline lines − 44,508 moved − 0 deleted + unmeasured boundary additions
= 781,746 lines + boundary additions in Mech
```

The moved implementation remains maintained externally. The combined source therefore remains 826,254 lines before actual simplification and boundary additions. Relocation has zero implementation-deletion credit. Pure-production and mixed/support arithmetic is presented separately in the extraction data; the experiment's manifest changes are measured independently of the estimate.

The final extraction layout contains **784,987 core lines and 41,325 external lines**, for **826,312 combined lines**. Relative to the ownership estimate, core changes contribute 40 net lines and external changes contribute 18. These include the retained test-harness manifest, path/feature changes and the source-profile repair. The exact path accounting and earlier coarse experiment remain in `data/extraction-boundaries.json`.

A maintained bytecode consumer executes `F64(3)` with the original library and machine directories physically absent. The selected dependency graph has 106 normal/build packages in each layout. With identical unoptimized, debug-zero, nonincremental settings, the native executables measure 56,245,152 bytes before and 56,148,416 bytes after relocation. Paths and link layout affect these artifacts; source deletion credit remains zero.

The external public catalog probe invokes `AddSS<f64>` directly, producing 3 and then 7 after an input update. A separate generated native application executes through the resident engine. Tracing that application shows scalar arithmetic implemented by retained engine intrinsics. The extraction therefore establishes both the external catalog boundary and the retained resident arithmetic owner. F-007 records the documentation discrepancy and the remaining architecture decision.

The matrix includes standard/full runtime, source and compiler profiles, native compute, WASM document and compute compilation, selected rehomed integration tests, and generated native execution. Exact commands and outcomes are in `extraction.md` and the evidence register. Permanent registry publication, version coordination and CI ownership are migration work following this feasibility experiment.

## Executed demonstrations

| Surface | Executed behavior | Evidence |
|---|---|---|
| Streaming and editor | 111 deterministic chunk schedules, UTF-8 byte transport, publication reconstruction, recovery, resource limits, insertion/deletion/repair, source selection and lifecycle clearing | `evidence/streaming-browser-result.json`, `streaming-ui-result.json`; 44 maintained native tests |
| Types and publication | Thirteen source examples, interval endpoints, nested options/records, inference, shape/schema checks, exact u128 transport; one instance accepts 9, rejects 10 while retaining state, then accepts 3 | `evidence/types-browser.json`; native adapter and interval/type tests |
| Browser CPU and GPU | Identical source and inputs at 3 and 1,024 columns; completed outputs match independent arithmetic; hard GPU placement rejects a CPU-only selector | `evidence/compute-browser-result.json`; Apple Metal hardware adapter and completion tokens recorded |
| Native GPU | Current-input and transactional-publication test executes wgpu with `MECH_REQUIRE_GPU=1` | `evidence/native-gpu-publication.log`; 1 test passes |
| WASM application | Maintained ten-body application and independent two-body traces at timestep 0.01 and 0.02; invalid allowance, restart, disposal and visible SVG output | `evidence/application-browser-result.json` |
| Native application | The identical two-body fixture agrees exactly with the independent reference after two accepted turns | `data/application-evidence.json`; maximum absolute error 0 |
| Inventory | A Mech replenishment recurrence executes the same nine native and browser updates; four guard rejections preserve output, epoch and state hash, with recovery to stock 12 | `evidence/inventory/native-result.json`, `evidence/inventory-browser.json` |
| Heat diffusion | New 4 × 4 and 8 × 8 grid workloads complete on CPU and hardware GPU; changed timestep and heater schedules agree with an independent stencil, conserve heat and preserve positivity | `evidence/diffusion-browser.json`; six runs, maximum absolute error 0 |
| R-stack proof | PR #829 recurrence, literal oracles, bytecode/artifact comparison, memory planning, perturbation and rejected-candidate protections; canonical browser extension | `evidence/trust-native.json`, `trust-browser.json` |

The browser package is a Mech runtime hosted in WASM. Every live demo verifies the loaded WASM SHA-256 before initialization. The build record identifies the baseline, changed source hashes, features, target, toolchain and glue/WASM hashes. The selected release package measures **208,709,347 WASM bytes**; it includes broad development and inspection features. Package-size optimization is a separate subtraction candidate.

The compute browser records requested, selected and completed backends independently. GPU results are read after queue completion and are acknowledged to the runtime with the returned dispatch token. The native GPU run initially failed because sandboxed device enumeration returned no adapter; the identical compiled test passed with Metal access. The original failure is retained.

## Corrections and remaining requirements

The source-profile experiment exposed two configuration defects: source-only builds compiled unused helpers under strict warnings, and standard source composition omitted semantic registration features. The repair narrows helper compilation and forwards semantic registration through selected machine features. Structured compiler diagnostics now preserve their source information. The maintained n-body README uses the current resident command. Browser integration corrections use an SVG scene root and checked conversion of exact integer publication offsets. Each original reproduction and revised result is retained.

The historical editor requirement for fragment parsing remains **unresolved**. Current edits parse the complete document and then reconcile subtree identities. The demo separately displays parsing and reconciliation work. The earlier requirement and current implementation are linked in F-002; selecting a new accepted scope requires a product decision.

The interval contract grants equality, key and ordering capabilities. Arithmetic uses base integer schemas. The inventory example therefore expresses its stock bounds as Mech integrity guards over signed integer arithmetic; the recorded interval-arithmetic rejection matches the accepted type contract.

The selected compute application paths are `cpu-scalar` and `wgpu`. SIMD/JIT implementations remain scoped to their library and benchmark paths. Multiple compute regions and permanent extraction/release migration retain their recorded status. Manual review covers the spans listed in `data/review-coverage.json`.

The subtraction queue prioritizes the demonstrated extraction boundary, repeated conversion/type policy, migration and source-spelling gates, retained arithmetic ownership, and optional inspection packaging. Each entry lists affected consumers and acceptance checks. The extraction moves 41,307 baseline lines; completed repairs and all added audit/demo infrastructure are separately counted in `data/audit-additions.json`.

## Final review

- **What changed in v0.4:** the capability register attributes source/streaming/editor contracts, canonical semantic and artifact boundaries, resident memory/publication, compute selection/completion and native/browser integration against the prior release. Historical line growth is 695,880 across all roles.
- **Where the implementation resides:** the atlas exposes every baseline file by responsibility and role. The architecture report traces source-to-activation, input-to-publication, edits-to-reconciliation and compute-to-completion with ownership transitions.
- **Where complexity concentrates:** findings identify discarded diagnostics, full edit reparsing, repeated type/conversion policy, source-spelling gates, feature closure and duplicated arithmetic ownership. Their consequences and inspected spans are explicit.
- **What can move or simplify:** the library extraction has an executable isolated proof. Further consolidation remains in the prioritized queue with required behavioral checks.
- **What remains after extraction:** 784,987 measured physical lines in Mech, including tests, configuration and documentation; external ownership contains 41,325 lines. The combined measured implementation is 826,312 lines.
- **What executed:** streaming/editor and type browser interfaces, native and browser resident applications, browser CPU/hardware GPU, native GPU publication, external catalog invocation and generated native execution. Records distinguish compilation, API tests, application execution and independent comparisons.
- **What still requires a decision or follow-up:** historical fragment parsing, arithmetic ownership reconciliation, release/CI migration for permanent extraction, and the explicitly unverified configurations in the capability register.

## Added audit and demonstration source

The added-material census counts every changed source file against the preserved baseline, including reproduction code stored with evidence. It separates the supplied brief and absorbed example from new inspection and audit work. Generated datasets, transcripts, patches, screenshots and compiled packages have separate size records.

| Change category | Net physical lines |
|---|---:|
| absorbed R-stack proof | 815 |
| audit and demo | 3,042 |
| optional WASM inspection | 1,078 |
| production corrections and integration | 96 |
| reproduction sources retained with evidence | 559 |
| supplied assignment | 518 |

The machine-readable totals and file hashes are in `data/audit-additions.json`. These additions are separate from the extraction experiment. The current package contains optional syntax/type/publication adapters; normal browser feature profiles retain their existing exports.
