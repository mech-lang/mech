# Default-input EKF symmetry rejection diagnostic

Date: 2026-09-26. The initial reproduction was read-only; the subsequent
candidate trial and authorized adoption are distinguished below. These
long-horizon checks are separate from the retained throughput benchmarks.

## Finding

The reported stop is not an intentional turn limit. The actual checked Mech
kernel rejects a candidate when accumulated floating-point asymmetry exceeds
the source's fixed absolute `ε = 0.0001f32`. The CPU reproductions below match
the reported counts exactly. A separate browser GPU run reaches the same kind
of failure on a different turn; it does not reproduce the user's exact GPU
turn or lane.

At the inspected rejections, all candidate entries are finite, the diagonal
is positive, and the symmetric part has positive leading principal minors.
The symmetry gap is only slightly over the absolute threshold and is less
than one millionth of the largest covariance entry. This is evidence of a
roundoff-sensitive symmetry invariant, not evidence of covariance blow-up at
these rejected steps. It does not prove arbitrary long-horizon stability or
exclude every possible backend accuracy issue.

## Reproductions

All runs use the default controls: velocity `1`, angular velocity `0.015`,
noise scale `0.02`, and step size `0.1`. The initial sensor-path state is
`[55, 25, 0.4]`. Each turn advances the path with the same JavaScript input
generator as the article, then passes f32 observations to the actual Mech
kernel. Lane `i` gets the phase `turn * 1.73 + i * 0.37`. There is no JavaScript
implementation of the EKF equations.

| Actual execution | Instances | Accepted | Rejected turn | Fault lane | Largest symmetry gap | Largest covariance magnitude |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| WASM CPU | 4,096 | 406 | 407 | 1,252 | 0.00010013580322265625 | 109.96923065185547 |
| WASM CPU | 65,536 | 359 | 360 | 57,410 | 0.00010061264038085938 | 107.83585357666016 |
| Browser WebGPU | 65,536 | 397 | 398 | 50,544 | 0.00010013580322265625 | 109.50740814208984 |

The user separately observed WebGPU rejection after 340 accepted turns at
65,536 instances (lane 57,938), and after 358 accepted turns at 4,096
instances. Those observations are not relabeled as locally reproduced GPU
results. GPU floating-point execution and the first failing lane need not
match CPU execution. A selected single-lane GPU replay also did not reproduce
the full-batch rejection within 500 turns, so the full batch was tested.

### CPU candidate details

At turn 407 of the 4,096-instance run, mirrored entries are
`6.224382400512695` and `6.224482536315918`, a 210-ULP gap. Candidate covariance,
shown as a row-major mathematical matrix, is:

```text
[109.96923065185547      6.224482536315918    -0.879287600517273
   6.224382400512695    41.75372314453125     -0.5829975008964539
  -0.8792864084243774   -0.582998514175415     0.01645432971417904]
```

Its symmetry gap divided by the largest absolute entry is
`9.105801925601332e-7`. The symmetric part's leading principal minors are
`[109.96923065185547, 4552.881251500823, 11.637393218736944]`.

At turn 360 of the 65,536-instance run, mirrored entries are
`5.470986843109131` and `5.471087455749512`, a 211-ULP gap:

```text
[107.83585357666016      5.471087455749512    -0.7650305032730103
   5.470986843109131    45.031890869140625    -0.6323177218437195
  -0.7650296688079834   -0.632318913936615     0.015900500118732452]
```

Its relative gap is `9.330165899724082e-7`; the symmetric part's leading
principal minors are
`[107.83585357666016, 4826.120142554352, 12.559382343587776]`.

Both checked CPU batches preserved every published mean/covariance bit on
rejection. The CPU candidate was inspected in a separate diagnostic-only
single-lane replay that removes only the three integrity declarations in
memory. Its last accepted covariance matched the original full batch
bit-for-bit. The retained helper additionally checks the entire accepted
single-lane prefix against a checked single-lane replay before reporting a
candidate. This is not a production retry or a proposal to disable checks.

### GPU candidate method

The GPU candidate was read directly from the inactive state buffer written by
the original **checked** shader on the rejected turn. The original generated
shader records an integrity fault and still writes candidate state; the host
does not swap that candidate into the active/published buffer. Only the
faulting lane's mean and covariance were copied to diagnostic readback buffers.

For the local 65,536-instance GPU run, the mirrored entries were
`6.084535121917725` and `6.084635257720947`. The relative gap was
`9.144203567737299e-7`. The candidate's symmetric part had positive leading
principal minors approximately `[109.507408, 4600.011, 11.7994]`.
Its row-major mathematical matrix was:

```text
[109.50740814208984      6.084635257720947    -0.8573192954063416
   6.084535121917725    42.3444709777832      -0.5923222899436951
  -0.8573179841041565   -0.5923237204551697    0.016339750960469246]
```

The mean was `[85.04737854003906, 50.19444274902344, 0.9970851540565491]`.
Active/published buffer index remained `1`; the rejected candidate was in
buffer `0`. The browser exposed adapter availability but no identifying
description, so this report does not attribute the GPU run to a device model.

Removing integrity checks from a separate GPU shader was explicitly **not**
used to infer this candidate: checked and unchecked GPU shaders produced
different bits from the first turn in an earlier probe. Similarly, a
single-lane run is not a substitute for the full-batch GPU reproduction.

## Reproduce the CPU check

The checked-in [replay helper](replay-ekf-symmetry.mjs) accepts explicit local
artifact paths, records their hashes, prints progress to stderr and the final
JSON report to stdout. It modifies no files. Use matching retained artifacts:

```sh
node benchmarks/iros-2026/blog/evidence/replay-ekf-symmetry.mjs \
  --source /path/to/retained/source.mec \
  --wasm-js /path/to/retained/mech_wasm.js \
  --wasm /path/to/retained/mech_wasm_bg.wasm \
  --instances 4096 --turns 600
```

Repeat with `--instances 65536` for the larger batch. The retained historical
source exports `state` and `covariance`; testing the current Unicode source
requires `--state μ --covariance Σ`. Different source/runtime artifacts are a
new experiment and must retain their own hashes and results.

Artifacts used for the table above were retained from the website's published
`public/iros-r4r-2026/` bundle:

| Artifact | SHA-256 |
| --- | --- |
| EKF source | `a7cd4077c7bf2f9741559b5748f05cf06e9b48e156c4fdbabfbdc7feea065eb2` |
| WASM binary | `99a400e89a925a959282ef1d2814fef40bd967e8b3f50b3ed03a267da2ac42d5` |
| WASM JavaScript module | `da1d1d748e0454230c7e815e23dd1ef160ce302451238a188073cd11e28394a1` |
| Browser compute host | `5d867e3d73085f7285e01a38c07635745a5aa57eee1fec78217247fc6576ef10` |

No new benchmark timings are claimed by this report. Publication of these
evidence files is a separate build/deployment step.

## Precision and correction rationale

The source already uses the Joseph covariance update:
`Σ₊ = A Σ̄ Aᵀ + K Kᵀ R`. That helps numerical stability but does not enforce
exact symmetry after separately rounded f32 operations.

Mech's general browser REPL supports f64 values; a tested f64 value preserved
`1.2345678901234567`. This does not mean the article's compute kernel supports
f64. The current [WasmKernel](../../../../src/wasm/src/kernel.rs) input/state
interface uses f32 arrays, and the shared
[fixed-shape admission code](../../../../hosts/gpu/src/batched/mod.rs)
explicitly rejects `FloatingPoint(W64)`. Both this kernel's WASM CPU execution
and its generated WGSL execution use the f32 compute representation. Merely
replacing the source's type annotations would therefore not make this
existing CPU/GPU application double precision.

A principled correction to evaluate separately is to maintain covariance
symmetry explicitly, for example by projecting a candidate to
`(Σraw + Σrawᵀ) / 2` before publication, while retaining checks that reject
nonfinite values and excessive *raw* asymmetry. A scale-aware raw symmetry
bound (`atol + rtol * scale`) would distinguish roundoff relative to matrix
magnitude from a large structural error. Changing only the acceptance
threshold would not itself stop asymmetric error from accumulating.

Such a change needs an explicit numerical contract and new long-horizon
CPU/GPU tests, including rejection/rollback/recovery. Positive diagonal
entries alone are not a positive-definiteness test. Double precision can
reduce rounding error on a supported backend but does not make rounded
matrix arithmetic exactly symmetric. No precision migration was attempted.
The following candidate was first an isolated test fixture; the authorized
adoption into the article's Unicode source is recorded separately below.

## Diagnostic-only candidate trial

[ekf-symmetry-candidate.mec](ekf-symmetry-candidate.mec) retains the original
prediction and Joseph measurement-update equations. Its checked publication
policy is:

```text
raw = Joseph-update(...)
budget(a,b) = 1e-4 + 1e-6*abs(a) + 1e-6*abs(b)
require each raw mirrored pair: -budget <= a-b <= budget
published = raw*0.5 + transpose(raw)*0.5
require finite mean, raw covariance, and published covariance
require positive raw and published diagonal entries
```

All operations execute in the actual f32 kernel. The relative contribution is
pairwise: a large unrelated diagonal does not loosen a small off-diagonal
check. Multiplication precedes addition in the budget to prevent an
intermediate `abs(a)+abs(b)` overflow; the projection halves each operand
before addition for the same reason. The checks are prerequisites to
publication; the declarative program need not evaluate them sequentially.
The original absolute floor is retained. `1e-6` per operand is an engineering
test budget, not a proved universal EKF error bound.

This does not hide arbitrary asymmetry: a separate test-only source adds `1`
to one raw off-diagonal entry, and the raw symmetry guard must reject it even
though the later projection would make the published matrix symmetric. The
raw finite and diagonal-positive guards also remain. Symmetrization is not a
positive-semidefinite repair, and these guards still do not prove PSD.

The candidate source SHA-256 is
`f6194e334c3348bdcec821f9304e6edd3e9dd934b25ec48fad100720a9f71720`.
Trials use the retained WASM binary hash recorded above, with the same
default observation sequence. The tests and reported limits are deliberately
separate from the original throughput measurements.

The [CPU test](test-ekf-symmetry-candidate.mjs) checks every lane on every
accepted turn for finite mean/covariance, positive diagonal and bit-identical
mirrored covariance entries. It injects NaN and both infinities into the last
lane before and after the long run, requires whole-batch rollback, and then
requires successful recovery. Its optional CPU-only instrumented companion
records raw residuals/budgets and must match every uninstrumented state bit.

```sh
node benchmarks/iros-2026/blog/evidence/test-ekf-symmetry-candidate.mjs \
  /path/to/retained/mech_wasm.js /path/to/retained/mech_wasm_bg.wasm \
  65536 1000 false

node benchmarks/iros-2026/blog/evidence/test-ekf-symmetry-candidate.mjs \
  /path/to/retained/mech_wasm.js /path/to/retained/mech_wasm_bg.wasm \
  64 10000 true
```

Confirmed CPU trial: **65,536 instances × 1,000 accepted turns** passed all
checks, plus the recovery turn. Every lane was inspected on every accepted
turn. The largest covariance magnitude was `274.7452392578125`; the smallest
diagonal was `0.01483859308063984`. NaN/±Infinity rejection and whole-batch
bitwise rollback passed at both ends, followed by successful recovery. The
injected raw asymmetry was rejected. This large run did not include the
additional raw-residual instrumentation or an uninterrupted reference copy.

Confirmed CPU trial: **64 instances × 10,000 accepted turns** passed all
checks, plus the recovery turn. The raw instrumented companion matched every
accepted mean/covariance bit. Maximum raw pair gap was `0.0001220703125`;
maximum raw-gap/budget ratio was `0.19707974732768144`. The largest published
covariance magnitude over those 10,000 turns was `803.063232421875`; the
smallest diagonal was `0.014838606119155884`. NaN/±Infinity rejection and
bitwise whole-batch rollback passed at both ends, and recovery matched the
uninterrupted reference bit-for-bit. The injected raw asymmetry was rejected.

The [browser GPU test](test-ekf-symmetry-candidate.html) is a standalone
diagnostic page. Place it in an isolated served directory with matching
`mech_wasm.js`, `mech_wasm_bg.wasm`, `browser-compute.js`, and a byte-identical
copy of the candidate named `candidate.mec`. Query parameters select `n` and
`turns` (defaults `65536` and `1000`). It uses the uninstrumented checked
candidate, inspects all published lanes every 100 turns and at the end, and
checks nonfinite rejection/whole-batch rollback/recovery and a separate gross
raw-asymmetry fault. It publishes its report as `symmetryCandidateReport` and
visible JSON.

The original-runtime GPU candidate trial **passed 65,536 instances × 1,000
accepted turns**, plus recovery turn 1,001. Every 100-turn checkpoint inspected
all lanes and found finite values, positive diagonals, and bitwise symmetric
mirrored entries. NaN/±Infinity last-lane inputs before and after the run
rejected with whole-batch bit rollback and unchanged active-buffer index.
Injected raw asymmetry of `1` also rejected with unchanged state. At turn 1,000,
maximum covariance magnitude was `274.7449951171875`, and the minimum diagonal
was `0.02398156188428402`. This does not assert bit-identical CPU/GPU arithmetic
or unbounded stability.

## Adoption into the current article source

The authorized article-only revision adapts this tested policy to the actual
Unicode bindings in [source/ekf.mec](../source/ekf.mec). Its SHA-256 is
`f18e37effb2fa63fadca69639f3a8eed218b73218decf65419e78a60b62bb46b`.
The pre-stabilization Unicode source is retained unchanged in
[ekf-before-symmetry-stabilization.mec](ekf-before-symmetry-stabilization.mec),
SHA-256 `cefe87b0ee184f1f30c34c66e626948f6d43236c8449dca0054a68e9e5cd932f`.
Archived native benchmark sources, measurements and original WASM kernel
fixtures were not modified.

The adopted source uses the current v0.4.0-beta WASM,
SHA-256 `18224e2cf04b246148029bff7bf498ed77c92780495a5e7cfc493a9cd94a8871`.
The actual adopted source **passed the 64-instance × 10,000-turn CPU test**
including all publication, raw-residual audit, invalid-input, rollback,
recovery and gross-asymmetry checks described above. Its statistics matched
the ASCII candidate trial. The complete result is retained in
[ekf-stabilized-cpu-64x10000.json](ekf-stabilized-cpu-64x10000.json).

The actual adopted source also **passed 65,536 instances × 1,000 CPU turns**,
plus recovery, on that current runtime. Every lane was inspected on every
accepted turn. The finite/positive-diagonal/exact-symmetry, NaN/±Infinity
rollback/recovery and gross raw-asymmetry checks all passed; statistics matched
the original-runtime large candidate trial. The complete result is retained in
[ekf-stabilized-cpu-65536x1000.json](ekf-stabilized-cpu-65536x1000.json).

The current source/runtime also passed the identical 1,000-turn CPU test at
the other two offered batch sizes, with all publication, invalid-input,
rollback/recovery and injected raw-asymmetry checks:
[256 filters](ekf-stabilized-cpu-256x1000.json) and
[4,096 filters](ekf-stabilized-cpu-4096x1000.json).

The same current source/runtime **passed the 65,536-instance × 1,000-turn
WebGPU trial**, plus recovery, with every 100-turn full-batch checkpoint and
all invalid-input/raw-asymmetry checks passing. Rejections preserved all
published bits and the active-buffer index. The observed browser summary is
[ekf-stabilized-gpu-65536x1000-summary.json](ekf-stabilized-gpu-65536x1000-summary.json).
It is labeled as a summary, not a complete raw event log. These are bounded
invariant checks, not a long-horizon CPU/GPU numerical-error bound or an
assertion of global EKF stability.

The WebGPU trials also passed the same checks at
[256 filters](ekf-stabilized-gpu-256x1000-summary.json) and
[4,096 filters](ekf-stabilized-gpu-4096x1000-summary.json). Thus all six offered
backend/batch combinations (CPU and WebGPU, each at 256, 4,096 and 65,536
filters) completed 1,000 default-input turns on the adopted source/current
runtime, plus invalid-input rollback/recovery and the separate raw-asymmetry
fault test. These browser reports are observed summaries, not benchmark data.

To run the test on the actual adopted source instead of the diagnostic ASCII
fixture, append `benchmarks/iros-2026/blog/source/ekf.mec` to the CPU command
above and supply the current WASM module/binary paths. For a browser test,
copy that exact source as `candidate.mec` beside the current WASM and browser
compute host. The helper recognizes the actual `μ`/`Σ` exports.
