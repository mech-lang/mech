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

The concrete factory Rust type is authoritative for each runtime function's
input and output signature. Scalar and exact matrix representation features
are derived from that signature; declarations list only operation-specific
extra features. A runtime ID has one feature-invariant signature and cannot
select a different output representation when unrelated Cargo features are
enabled. The exact-closure contract treats every unique package/feature
combination as an inventory identity, rejects any signature/feature mismatch,
and compares the complete installed ID/name/signature surface. Bounded CI
compiles one bounded exact closure per owner plus the named cross-shape matrix
regressions in eight deterministic shards; it does not attempt tens of
thousands of redundant owner-crate compilations.

`MechFunction::solve_result` is the sole production execution API. Function
errors propagate through runtime and bytecode execution; production code does
not discard them behind an infallible `solve` wrapper.

The standard native host catalog is built from the same private registration
records as source-planning factories. Its exact provider surface is:

| Provider | Package | Feature | Generated factory |
| --- | --- | --- | --- |
| `cli` | `mech-terminal` | `provider` | `mech_terminal::CliHostFactory::new` |
| `console` | `mech-console` | `native` | `mech_console::NativeConsoleHostFactory::new` |
| `time` | `mech-time` | `native` | `mech_time::NativeTimeHostFactory::new` |
| `timer` | `mech-timer` | `native` | `mech_timer::NativeTimerHostFactory::new` |
| `scene` | `mech-scene` | `native` | `mech_scene::NativeSceneHostFactory::new` |
| `robot-arm` | `mech-robot-arm` | `provider` | `mech_robot_arm::RobotArmHostFactory::new` |

These providers support Unix and Windows native targets. Browser, Wasm,
unknown, and unsupported native providers are rejected.

Trusted actor calls are also exact: `actor/message/kind`,
`actor/message/payload`, `actor/state/id`, `actor/state/get`, and
`actor/state/put`. Each selects only its corresponding
`mech_runtime::__mech_native::install_*` function in generated `runtime.rs`.
Host-function installers configure `RuntimeBuilder`; they are not function
catalog installers. Linkage metadata marks these functions as requiring an
actor turn; native analysis never infers that context from a function-name
prefix.

## Actor entrypoints

`build.actor` is required whenever bytecode uses an actor-context host
function. Actor values are never fabricated by the build tool or generated
application. Configure the one explicit actor turn in `.mcfg`:

```mech
config := {
  build: {
    actor: {
      subject: "actor:main",
      message-kind: "startup",
      message-payload: "hello",
      initial-state: "initial",
    },
  },
}
```

`subject`, `message-kind`, and `message-payload` are required.
`initial-state` is optional; `initial-state: null` means the actor starts
without state. Subjects and message kinds must be nonempty after trimming,
while payload and present state strings may be empty. Unknown fields and
wrong value types are rejected, and there are no hidden actor defaults.

A generated actor executable creates the configured actor and message,
installs only the exact capabilities needed by its five actor functions, and
executes the bytecode in one transaction-backed actor turn. Success commits
state and acknowledges the message; failure aborts staged state and leaves the
message pending. Actor applications with live resource drivers are currently
rejected because this one-turn entrypoint has no live actor scheduling model.

## NativeBuildPlan

Analysis normalizes an optional `NativeRuntimeConfig`, validates requirements
against the trusted catalogs and target, and emits schema
`mech.native-build-plan.v1`. Grant paths use the runtime capability
normalizer before matching or hashing, and host settings reject non-finite
floats recursively so JSON plan identity remains lossless. A plan freezes:

- bytecode and bytecode version, application kind, target, profile, binary
  name, component version, normalized runtime configuration, and optional
  normalized actor bootstrap;
- exact runtime types and functions, application requirements, host
  instances, grants, packages, features, installers, and factories;
- registry or workspace dependency source, optional workspace fingerprint,
  the frozen dependency-resolution SHA-256, live status, bytecode SHA-256,
  and plan SHA-256.

Resource requirements retain two structured identities. The request records
the originally requested base URI, path, context name, operation, intent, and
delivery. The trusted owner records the exact host instance, provider, host
context, and canonical base URI selected during addressability resolution.
Exact structured grant keys are derived once from those records; the legacy
slash-delimited runtime target is rendered only at the final runtime-config
boundary.

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

Workspace generation seeds `Cargo.lock` from the trusted workspace lock;
registry generation uses the packaged native-resolution seed. The seed digest
is part of plan identity. Cargo may prune that universe for the exact generated
manifest, but every selected registry package must match a seeded
name/version/source/checksum tuple, and the seed cannot contain two
semver-interchangeable versions. A newly published or cache-only version is
therefore rejected instead of silently changing the binary for an existing
plan. The resolved lock remains in the generated project and all builds use
`--locked`.

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

Bytecode contract validation performs one shared instruction traversal for
execution and native analysis. The native build supplies the external host and
resource resolver rather than maintaining another instruction interpreter.
Every bytecode register is also one stable outer value cell: symbols, external
outputs, downstream factories, transaction rollback, and the final return
observe the same identity.

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

Workspace fingerprint v2 is SHA-256 over a domain-separated, length-framed
stream. It starts with the ASCII bytes `mech.workspace-fingerprint.v2` and a
little-endian `u64` entry count. Each entry is sorted by normalized
workspace-relative UTF-8 path and encoded as tag `0x01`, a little-endian `u64`
path length, the path bytes, a little-endian `u64` content length, and the
exact file bytes. Duplicate paths are rejected. Absolute paths, filesystem
metadata, temporary project paths, and output paths never enter the digest.

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
