# Browser verification record

Recorded on 2026-09-26 against the local article preview at
<http://localhost:8765/>. These are functional and numerical checks of the
browser implementation, not new native benchmark measurements or a
publication record.

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

## Final integration pass

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
