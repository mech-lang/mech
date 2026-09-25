# Embedding a numerical kernel in Rust

The `mech::kernel` interface compiles fixed-shape numerical Mech source and keeps its state between calls from Rust. The runnable example uses the bearing-only extended Kalman filter in [`examples/embedded_ekf/ekf.mec`](../examples/embedded_ekf/ekf.mec). Its equations and integrity constraints are the same as the benchmark kernel in [`hosts/gpu/fixtures/ekf-kernel.mec`](../hosts/gpu/fixtures/ekf-kernel.mec). The example replaces the benchmark's array-producing driver section with scalar defaults and supplies the measurement array from Rust.

Run the example from this repository:

```sh
cargo run --no-default-features --features kernel-jit --example embedded_ekf
```

The complete host program is [`examples/embedded_ekf/main.rs`](../examples/embedded_ekf/main.rs). Its numerical work is initiated by these calls:

```rust
use mech::kernel::{Backend, Kernel};
let source = include_str!("ekf.mec");
let kernel = Kernel::from_source(source)
    .input("bearing", [-0.55; 4])
    .export("state")
    .compile(Backend::Jit)?;
let mut ekf = kernel.start()?;
ekf.turn([("bearing", [-0.54; 4])])?;
let state = ekf.state("state")?;
```

## Compilation and state

Put all defaults, helper expressions and state updates inside one backend-neutral `@compute` section. Only imports may appear outside it. This interface rejects mixed driver/kernel documents and CPU/GPU placement annotations; select the backend through the Rust builder.

The source contains one `@compute` section. Its declarations define the per-instance types and shapes. `.input("bearing", ...)` marks that source binding as a live input and supplies its initial values. Declarations that are not selected as live inputs retain their source values. `.export("state")` selects a persistent source binding that Rust can read by name. The compiler resolves the source name to a state slot; the host does not need to handle slot identifiers.

`.compile(...)` parses and lowers the source and prepares the selected implementation. JIT compilation creates scalar native code in memory. AOT compilation emits and loads a native shared library. Both take place when `.compile(...)` runs, before `.start()`; `include_str!` embeds source text during Rust compilation but does not precompile the Mech kernel. `.library_path()` returns an AOT library's path, or `None` for JIT and evaluator backends. Use `.artifact_directory(path)` before `.compile(...)` to choose an AOT emission directory.

`.start()` creates a session with independent input and state buffers. Repeated calls to `.start()` reuse the compiled kernel and initialize separate sessions from the same source defaults. Compilation is not repeated for each turn or each session.

## Saving and loading AOT code

Build a persistent AOT bundle once, using the same source and input/export declarations:

```rust
let kernel = Kernel::from_source(include_str!("ekf.mec"))
    .input("bearing", [-0.55; 4])
    .export("state")
    .compile(Backend::AotSimd)?;
kernel.save_bundle("ekf.bundle")?;
```

Another Rust process can load the saved code and submit turns without parsing source, lowering, invoking Cranelift, or running a linker:

```rust
use mech::kernel::Kernel;
// SAFETY: this is an immutable bundle from our trusted Mech build.
let kernel = unsafe { Kernel::load_bundle("ekf.bundle")? };
let mut ekf = kernel.start()?;
ekf.turn([("bearing", [-0.54; 4])])?;
```

Complete producer and consumer examples are [`build.rs`](../examples/embedded_ekf/build.rs) and [`load.rs`](../examples/embedded_ekf/load.rs):

```sh
cargo run --no-default-features --features kernel-aot --example embedded_ekf_aot_build
cargo run --no-default-features --features kernel-aot --example embedded_ekf_aot_load
```

The bundle directory must not already exist. It contains `manifest.json` and `kernel.dylib` on macOS (`kernel.so` on other supported Unix hosts). Format version 1 describes the native pointer-table ABI, scalar or four-lane SIMD layout, target OS/architecture/pointer width/endianness, ordered inputs and state, initial values, named exports, and integrity-error mappings. Metadata and library SHA-256 digests detect corruption and accidental mismatches, including replacing checked code with another library. A saved bundle initializes new sessions from the original source state and configured inputs; it is not a checkpoint of a running session. Sessions retain the library after the `Kernel` is dropped. Loading does not recompile, although the current Cargo feature includes compiler dependencies in the consumer build.

`load_bundle` is unsafe because native library initialization may execute arbitrary code and hashes do not authenticate its origin. The caller must trust the entire bundle, prevent changes during loading, keep the native library immutable while any kernel or session uses it, and use a compatible native CPU and Mech build. The loader checks structural limits, manifest version, target identity and both digests before loading native code. Bundles are native deployment artifacts, not a portable or stable cross-version format. Arbitrary standalone libraries without matching metadata are not accepted. A generated-kernel `rlib` export and C++/Python wrappers are not provided by this interface.

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
| `Backend::Jit` | Scalar native code compiled once in memory by Cranelift | `kernel-jit` |
| `Backend::Aot` | Scalar native code emitted by Cranelift | `kernel-aot` |
| `Backend::AotSimd` | Four-lane SIMD native code emitted by Cranelift | `kernel-aot` |

The AOT SIMD backend requires an instance count divisible by four. The examples use four instances so the same source and inputs support either JIT or AOT SIMD. AOT emission requires a native linker toolchain on the producer; loading a saved bundle does not.

This interface executes a numerical kernel. It does not create the complete reactive coordinator, connect to ROS, or deliver external effects. A Rust application can pass measurements from those systems into the kernel.

The example demonstrates the embedding calls and checked execution. The poster's archived benchmark measurements describe the separately identified benchmark executors and workloads; they do not measure this host wrapper's compilation or input-update overhead.
