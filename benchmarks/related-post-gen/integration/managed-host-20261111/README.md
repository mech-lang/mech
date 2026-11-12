Optimizing the managed host reduces complete post-JSON time from **1.703 ms to 1.608 ms**, or **1.062× the fastest eligible Rust** at 1.514 ms. The unchanged managed control is beaten in all 31 matched rounds; paired median improvement is 5.59%, with a bootstrap 95% interval of 5.13–5.87%. Managed Rust parity remains unmet.

| Same cohort, fresh processes | Post-JSON median | × fastest eligible Rust |
|---|---:|---:|
| Rust ordered mask tables256 | 1.514 ms | 1.000× |
| Unchanged coherent managed IR320 | 1.703 ms | 1.124× |
| First host improvement | 1.658 ms | 1.095× |
| Both host improvements, four native workers | **1.608 ms** | **1.062×** |
| Retained managed IR317 | 1.682 ms | 1.111× |
| Retained compact IR320, distinct unmetered contract | 1.319 ms | 0.871× |

The core now quotes and imports an immutable snapshot using one private checked ownership witness. Packed String matrix footprints derive encoded size, retained size and nodes from one checked length sum. Schema/shape checks, exact memory/node admission, allocation-failure recovery and same-account alias rules remain. No per-value cache, serialized permission, host scoring implementation, compatibility bridge or new mandatory dependency is introduced. Cranelift remains the backend. The selected adapter is `../../hosts/managed-4.rs`; the maintained Mech algorithm is unchanged.

Native Object bytes are identical to the unchanged managed IR320 control. Both gains are in host work. Post-JSON instructions fall **32,252,914 → 31,504,511 (2.32%)**; cycles are 10,140,789 → 10,053,494. Allocation requests remain 735, requested allocator bytes 7,036,985, and peak additional requested allocator heap 4,871,859 bytes. These allocator figures exclude mappings, OS stacks and allocator internals. The report retains whole-process memory, code, executable and stack measurements separately.

The timer includes fresh input construction/admission, output slots, worker creation/run/join, native cleanup and result materialization. JSON IO and final input/output destruction retain the historical managed boundary. Whole-process time remains visible: candidate 46.958 ms versus unchanged managed 46.848 ms. It is not an acceptance gate. All 868 primary outputs and 196 separately instrumented resource outputs match the full Rust JSON result. Twenty eligible Rust controls and two labeled preinitialized controls remain; the latter are ineligible for the primary reference.

Full-path diagnostic profiles repeat fresh JSON decoding and post-JSON preparation. Their warm repeated timings are not qualification timings, and sampled stacks are not wall-time percentages. Mean measured host stages move from roughly 58 µs flattening plus 82 µs quotation/import to 56 µs flattening plus 6.4 µs combined quotation/import. Fresh-process admission is about 74 µs. The baseline stage label `schema_deep_copy` was misleading: SchemaTable contents already share storage, and that owner-clone step is negligible. Native query work remains the largest compiled CPU hotspot; the worker-budget lock remains visible.

The worker study confirms four chunks are fastest among the predeclared 2, 3, 4, 6 and 8 variants: 1.771, 1.690, 1.659, 1.789 and 1.874 ms respectively in that cohort. A shorter eight-round mutex backoff passes 42 worker-budget tests but has no processing gain; the original twelve-round policy is restored. Its apparent ratio improvement reflects a slower Rust control, not faster Mech. Source lookup-table, branch insertion, checked relations, funding and projected-debit experiments are preserved with their adverse observations; none replaces the selected compiler.

All **397 current core tests pass**, including independent generic footprint comparisons, exact/one-under byte and node limits, aliases, concurrent imports, allocation failures and retry. The broader first-host-revision native run has 1,047 passes, 16 failures and 11 ignored tests. All 16 failures reproduce in a freshly rebuilt unchanged baseline: one unavailable stats import and fifteen captured-input/string-map StaleHandle failures. This is not a broad native or hosted-CI green claim. A parallel-index prototype is unselected: native lowering rejects tuple collection elements, and its inner dynamic-shape compatibility still needs qualification. No scalar-packed workaround is adopted.

`report.json` pins the source, measured libraries, input, exact native Objects, boundaries, profiles and limitations. `records.json` maps every logical retained record to losslessly compressed, content-hashed bytes, including negative trials. Identical outputs and bundles share storage. Library binaries and executables are represented by their verified build/size hashes; native Objects and bundle bytes are retained. Earlier compact evidence remains in `../native-compact-20261111` and verifies against its own compiler commit.

Run the offline verifier from the repository root:

```sh
python3 benchmarks/related-post-gen/integration/managed-host-20261111/verify.py
```

It checks compressed records, both pinned source trees, the unchanged native Object, exact-output observations and resource medians. It reconstructs the Rust ratio and reports that the managed target is still unmet. Retained absolute paths document the original Apple M3 environment and require relocation for a new machine-specific build.
