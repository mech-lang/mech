# Embedding a numerical kernel in Rust

The `mech::kernel` interface compiles fixed-shape numerical Mech source and keeps its state between calls from Rust. The runnable example uses the bearing-only extended Kalman filter in [`examples/embedded_ekf/ekf.mec`](../examples/embedded_ekf/ekf.mec). Its equations and integrity constraints are the same as the benchmark kernel in [`hosts/gpu/fixtures/ekf-kernel.mec`](../hosts/gpu/fixtures/ekf-kernel.mec). The example replaces the benchmark's array-producing driver section with scalar defaults and supplies the measurement array from Rust.

Run the example from this repository:

```sh
cargo run --no-default-features --features kernel-aot --example embedded_ekf
```

The complete host program is [`examples/embedded_ekf/main.rs`](../examples/embedded_ekf/main.rs). Its numerical work is initiated by these calls:

```rust
use mech::kernel::{Backend, Kernel};
let source = include_str!("ekf.mec");
let kernel = Kernel::from_source(source)
    .input("bearing", [-0.55; 4])
    .export("state")
    .compile(Backend::AotSimd)?;
let mut ekf = kernel.start()?;
ekf.turn([("bearing", [-0.54; 4])])?;
let state = ekf.state("state")?;
```

## Compilation and state

Put all defaults, helper expressions and state updates inside one backend-neutral `@compute` section. Only imports may appear outside it. This interface rejects mixed driver/kernel documents and CPU/GPU placement annotations; select the backend through the Rust builder.

The source contains one `@compute` section. Its declarations define the per-instance types and shapes. `.input("bearing", ...)` marks that source binding as a live input and supplies its initial values. Declarations that are not selected as live inputs retain their source values. `.export("state")` selects a persistent source binding that Rust can read by name. The compiler resolves the source name to a state slot; the host does not need to handle slot identifiers.

`.compile(...)` parses and lowers the source and prepares the selected implementation. AOT compilation emits and loads a native shared library. `.library_path()` returns that library's path, or `None` for an evaluator backend. Use `.artifact_directory(path)` before `.compile(...)` to choose an emission directory.

`.start()` creates a session with independent input and state buffers. Repeated calls to `.start()` reuse the compiled kernel and initialize separate sessions from the same source defaults. Compilation is not repeated for each turn or each session.

## Input and state layout

The instance count is inferred from the initial input values. Here, `bearing` is scalar in Mech, so the four-element Rust array selects four independent filters. A single value can be broadcast over the established batch. When several live inputs provide arrays, their outer instance counts must agree.

For a matrix input, one instance contains all matrix elements in column-major order. A full batch concatenates those per-instance values. State slices use the same layout. The example's `state` is a three-element column vector, so each consecutive group of three values is one filter's position and heading. `kernel.instances()` reports the inferred instance count.

Input and export names are checked during compilation. Turn updates must name declared live inputs and have either one per-instance value or the full compiled batch length. Invalid update packets are rejected before any input buffer is changed.

## Checked turns

`.turn(updates)` binds a packet of input updates and executes one checked turn. `.advance()` executes one checked turn with the inputs already bound. Inputs omitted from a packet retain their previous values.

The EKF source declares finite-candidate, positive-covariance and covariance-symmetry constraints. Each turn computes candidate state and checks those constraints before replacing the published state. An integrity failure returns an error and preserves the prior published state for the entire batch. The new input values remain bound after an integrity failure, so a host should replace invalid measurements before retrying. A malformed update packet, such as an unknown input name or an invalid array length, changes neither inputs nor state.

`.state("state")` borrows the selected published state as `&[f32]`. It does not copy the result. Before the first successful turn, this is the source initializer. Rust's borrow rules prevent the host from advancing the same session while that slice is still in use.

## Backend selection

| Backend | Execution | Feature |
| --- | --- | --- |
| `Backend::Scalar` | Scalar evaluation of the lowered numerical instructions | `kernel` |
| `Backend::Simd` | Four-lane SIMD evaluation of the lowered instructions | `kernel` |
| `Backend::Aot` | Scalar native code emitted by Cranelift | `kernel-aot` |
| `Backend::AotSimd` | Four-lane SIMD native code emitted by Cranelift | `kernel-aot` |

The AOT SIMD backend requires an instance count divisible by four. The example uses four instances for that reason. AOT emission also requires a native linker toolchain on the host.

This interface executes a numerical kernel. It does not create the complete reactive coordinator, connect to ROS, or deliver external effects. A Rust application can pass measurements from those systems into the kernel. Loading an arbitrary pre-existing shared library without its compiled kernel metadata is not part of this interface.

The example demonstrates the embedding calls and checked execution. The poster's archived benchmark measurements describe the separately identified benchmark executors and workloads; they do not measure this host wrapper's compilation or input-update overhead.
