# Reactive-diagram numerical output

The poster's `Example turn 800` output uses the bearing-only EKF equations from the [embedded Mech kernel](../../../../examples/embedded_ekf/ekf.mec), with the input values shown in the [Rust embedding example](../../../../examples/embedded_ekf/main.rs). Its robot pose and position-covariance ellipse come from the same numerical state. This is an illustrative repeated-input computation, not an additional throughput measurement or a recorded robot trajectory.

The values in [turn-800.json](turn-800.json) were obtained by executing an existing checked f32 build of the [Rust ABI control](../../rust-dylib/rust-ekf-dylib.rs). They are not a newly executed Mech result. An independent NumPy f32 matrix evaluation of the Mech equations was used as a cross-check. The earlier [turn-40.json](turn-40.json) remains available for the previous poster revision.

## Inputs and state

Each of eight hundred turns uses `dt=0.1`, linear velocity `1`, angular velocity `0.015`, bearing `-0.54`, and measurement variance `0.25`. The initial mean is `[55, 25, 0.4]`, initial covariance is `diag(100, 100, 0.15)`, landmark is `[140, 12]`, and process covariance is `diag(0.01, 0.0025)`. Inputs are converted to f32 before execution.

The Rust snippet first configures bearing `-0.55`, then replaces it with `-0.54` before its first turn. [`Kernel::start`](../../../../hosts/gpu/src/embed/mod.rs) prepares the initial state without advancing it. This fixture corresponds to the shown `turn()` followed by 799 `advance()` calls with unchanged inputs.

The full result is preserved in the JSON. The poster rounds it to:

```text
mu = [98.24, 55.19, -0.034]
Sigma = [107.89  -28.89  -0.907
         -28.89   19.88   0.104
          -0.907   0.104  0.012]
```

The state is `[x, y, theta]`. The source uses plain f32 coordinates, not unit-annotated quantities. Heading is in radians; no metre unit is asserted for position.

## Scene geometry

Let `a = Sigma[0,0]`, `b = (Sigma[0,1] + Sigma[1,0])/2`, and `d = Sigma[1,1]`. The off-diagonal average removes only the tiny f32 asymmetry when deriving display geometry; it does not change the recorded state.

```text
root = sqrt(((a - d)/2)^2 + b^2)
lambda_major, lambda_minor = (a + d)/2 +/- root
semiaxes = 2 * sqrt(lambda_major), 2 * sqrt(lambda_minor)
angle = atan2(2*b, a - d)/2
```

The semiaxes are `21.58926311865064` and `6.7080254414225955`, at a world-coordinate angle of `-16.642502461815663` degrees. The robot is centered at the mean's x/y coordinates, with heading `-0.034165628254413605` radians. For SVG y-down coordinates, the ellipse and robot rotations change sign. Equal x/y drawing scale preserves the actual ellipse aspect ratio; the robot symbol is schematic rather than a physical footprint.

This is a **2-sigma position-covariance ellipse**, not a 95% region: a 2D Gaussian ellipse with Mahalanobis radius two encloses approximately 86.5% of its probability. The [live localization example](../../../../examples/ekf/localization.mec) uses the same covariance-to-ellipse construction, but none of its range-and-bearing state was substituted here.

## Provenance and reproduction

Source revision for the current capture: `b6ad82ca0203b9f026f05e293e29644623d73b3b`. The source hashes are unchanged from the earlier turn-40 capture. SHA256 identifiers:

| Input | SHA256 |
| --- | --- |
| `examples/embedded_ekf/ekf.mec` | `da531cddcb25d002d49f1a77800122e84b573e2685e191c1955908ef6fccd625` |
| `examples/embedded_ekf/main.rs` | `1e034a6ac387a7352f26ccc668e9e7d42f00d03c31f62b82b00c7fbd5e338724` |
| `benchmarks/iros-2026/rust-dylib/rust-ekf-dylib.rs` | `5e32fcc5e2b17bf923718860756b233d4da145b20cff48a20d0c18ec51ccc9a7` |
| Executed Rust control dylib | `165672d40d46d551705528561de4a4500d17893ade5d80a5095f99fcbe9a7fcd` |

Python 3.12.14 and NumPy 2.3.5 were used for the capture. All 800 checked ABI calls returned zero. The independent matrix computation differed by at most `1.10626220703125e-4` in the mean and `1.983642578125e-4` in the covariance. These small differences reflect f32 evaluation order; the exact two outputs are recorded separately.

The selected later turn has a visibly elongated and angled positional covariance without changing its drawing geometry. Extending this particular repeated-input Rust f32 run reaches the covariance-symmetry guard at attempted turn 863 (ABI status 3). The turn-800 illustration therefore does not imply indefinite validity for the repeated-input fixture; the checked runtime rejects the later candidate.

Supply the path to a trusted build of the linked Rust control source. From the repository root:

```sh
python3 benchmarks/iros-2026/poster/reactive-output/reproduce.py \
  --library /path/to/librust_ekf.dylib --turns 800
```

The script requires NumPy, executes one filter, prints one JSON document, records the supplied library's SHA256, and writes no files. Eight hundred turns is the default; `--turns 40` reproduces the earlier poster state and `--turns 1` reproduces the first-turn alternative. The Rust source header contains its build command. A rebuild can differ in binary hash and floating-point rounding across compiler versions or targets; it is not expected to reproduce the archived dylib hash automatically.
