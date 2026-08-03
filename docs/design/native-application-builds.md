# Native Mech applications

`mech build` is the authoritative path from trusted Mech source or official
bytecode v1 to a deterministic native executable, bytecode file, build plan,
or generated Cargo project. Native generation preserves one boundary:
bytecode describes the program, while trusted Rust catalogs choose every
crate, feature, installer, and host factory.

## Source planning

Source builds use the maintained module-aware source pipeline in
`RuntimeExecutionMode::Plan`:

1. Canonicalize source roots while preserving explicit root order; sort files
   discovered inside directories.
2. Create the maintained file source resolver and load the selected `.mcfg`.
3. Create a deterministic `<binary-name>-planner` runtime configuration.
4. Materialize configured provider names, effect-free trusted planning host
   factories, host instances, run grants, and the trusted actor functions.
5. Inject `mech_stdlib::source_catalog()`, resolve retained root modules in
   order, and compile official bytecode v1.

Planning validates resource reads and effects but performs no external read,
prepare, apply, deliver, driver attach, or driver start. Planner diagnostics,
source paths, resolver roots, filesystem capabilities, and temporary limits
are not embedded in the generated application. Direct `.mecb` input bypasses
source compilation but is parsed as official v1 and checked against the same
ABI and native-analysis rules.

## Trusted linkage

Runtime factories declare `NativeFunctionLinkage` next to their owner-local
runtime registration. The record names the package, crate, exact hidden
installer path, and minimal sorted Cargo feature closure. Every installer
under an owner's `__mech_native` module inserts exactly one factory. Native
analysis resolves each bytecode runtime-function ID and exact name through
this catalog; bytecode cannot supply linkage metadata.

The standard native host catalog is built from the same private registration
records as source-planning factories. Its exact provider surface is:

| Provider | Package | Feature | Generated factory |
| --- | --- | --- | --- |
| `cli` | `mech-host-cli` | `provider` | `mech_host_cli::CliHostFactory::new` |
| `console` | `mech-host-console` | `native` | `mech_host_console::NativeConsoleHostFactory::new` |
| `time` | `mech-host-time` | `native` | `mech_host_time::NativeTimeHostFactory::new` |
| `timer` | `mech-host-timer` | `native` | `mech_host_timer::NativeTimerHostFactory::new` |
| `scene` | `mech-host-scene` | `native` | `mech_host_scene::NativeSceneHostFactory::new` |
| `robot-arm` | `mech-host-robot-arm` | `provider` | `mech_host_robot_arm::RobotArmHostFactory::new` |

These providers support Unix and Windows native targets. Browser, Wasm,
unknown, and unsupported native providers are rejected.

Trusted actor calls are also exact: `actor/message/kind`,
`actor/message/payload`, `actor/state/id`, `actor/state/get`, and
`actor/state/put`. Each selects only its corresponding
`mech_runtime::__mech_native::install_*` function in generated `runtime.rs`.
Host-function installers configure `RuntimeBuilder`; they are not function
catalog installers.

## NativeBuildPlan

Analysis normalizes an optional `NativeRuntimeConfig`, validates requirements
against the trusted catalogs and target, and emits schema
`mech.native-build-plan.v1`. A plan freezes:

- bytecode and bytecode version, application kind, target, profile, binary
  name, component version, and normalized runtime configuration;
- exact runtime types and functions, application requirements, host
  instances, grants, packages, features, installers, and factories;
- registry or workspace dependency source, optional workspace fingerprint,
  live status, bytecode SHA-256, and plan SHA-256.

Collections are sorted and deduplicated before hashing. The plan digest is the
SHA-256 of the complete normalized plan except its own digest field. It changes
when program data, executable behavior, selected code, configuration, target,
profile, component source, or workspace content changes.

## Generated Cargo project and exact catalog

A generated project has this fixed layout:

```text
target/mech-native/projects/<plan-digest>/
  Cargo.toml
  Cargo.lock
  build-plan.json
  program.mecb
  src/main.rs
  src/catalog.rs
  src/runtime.rs
```

Cargo artifacts share `target/mech-native/cargo-target/`. Cargo JSON messages,
not guessed paths, identify the target executable.

`catalog.rs` contains only the sorted exact runtime-factory installer calls
selected by the bytecode. `runtime.rs` contains direct Rust construction of
the normalized runtime, selected host factories and instances, grants, and
exact host-function installers. `main.rs` embeds `program.mecb` with
`include_bytes!`. Generated projects never depend on `mech-stdlib`,
`mech-syntax`, `mech-bytecode`, or `mech-build`, and never enable `source`,
`compiler`, or `native-plan`.

An engine application links `mech-core`, `mech-engine`, and only selected
machine crates. It excludes `mech-runtime` and all host crates. A hosted
application adds `mech-runtime` and only the selected native host crates.
Neither application parses source or configuration at run time.

Every generated executable accepts no argument or exactly `--once`; any other
argument prints usage and exits with status 2. Non-live programs execute once
in either form. A live hosted program:

1. constructs its exact catalog and runtime, installs embedded bytecode, and
   performs the initial turn;
2. prints a nonempty returned value and exits cleanly when `--once` is used;
3. otherwise installs a Ctrl-C handler using `Arc<AtomicBool>` with
   `Ordering::SeqCst`, starts selected drivers, and drains at most 64 inputs
   per iteration;
4. sleeps for 10 milliseconds after an empty drain, then on interruption
   stops drivers and shuts down the runtime.

Only live projects depend on exact `ctrlc = "=3.5.2"`. Cleanup also runs after
a primary failure; the primary error is reported before any driver-stop or
shutdown failures.

## Registry and workspace modes

Registry mode uses exact `=<MECH_COMPONENT_VERSION>` constraints for every
selected Mech component. Planning verifies that selected components share the
same component version. Production registry manifests contain no path
dependencies or `[patch.crates-io]` section.

Workspace mode is selected only by `--workspace-root`. It resolves trusted
workspace-relative package paths, rejects paths that escape the workspace,
and incorporates the deterministic workspace fingerprint into the plan.
Equivalent bytecode and normalized configuration therefore have the same plan
identity only when their selected dependency source is also the same.

The deterministic project cache always remains under the plan digest. Native
builds reuse the shared Cargo target directory. `--keep-project` copies the
complete project next to an emitted native, bytecode, or plan artifact; a
Cargo-project emit is already the requested project and rejects that flag.

## Command surface

The complete interface is:

```text
mech build <INPUTS...>
  --emit native|bytecode|cargo-project|plan
  --name <NAME>
  --out <PATH>
  --target <TARGET>
  --profile debug|release
  --config <MCFG>
  --no-config
  --workspace-root <PATH>
  --keep-project
  --offline
```

The defaults are native output, release profile, current host target, and
registry dependencies. `--out` is always the exact output path. Bytecode emit
does not perform native analysis or invoke Cargo; plan emit does not generate
a project; Cargo-project emit generates and locks a project without building
an executable.

## Security boundary

```text
program:       embedded bytecode data
runtime:       statically linked Rust code
catalog:       generated exact installer calls
mutable state: allocated at process startup
```

Bytecode cannot name arbitrary crates, Cargo features, installer paths, or
host factories. Only trusted catalogs compiled into the build tool cross from
program requirements into Rust dependencies. Generated applications contain
no dynamic-library discovery, third-party package discovery, source parser,
compiler, build tool, appended archive, or self-reading executable payload.
