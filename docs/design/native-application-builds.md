# Native Mech applications

`mech build --backend resident` (the default) turns trusted Mech source or
canonical ProgramArtifact bytecode v1 into a deterministic resident native
executable, bytecode file, build plan, or generated Cargo project. The artifact
owns program meaning; trusted Rust catalogs and schema feature mappings choose
the linked implementations, Cargo features, and host factories.

## Source planning

Source builds use the maintained module-aware source pipeline through the
owned `ProgramCompiler` returned by `RuntimeBuilder::build_compiler`:

1. Canonicalize source roots while preserving explicit root order; sort files
   discovered inside directories.
2. Create the maintained file source resolver and load the selected `.mcfg`.
3. Create deterministic `<binary-name>-compiler` compilation configuration.
4. Materialize configured provider names, effect-free trusted planning host
   factories, host instances, and run grants.
5. Inject `mech_stdlib::source_catalog()`, resolve retained root modules in
   order, and compile a ProgramArtifact with its canonical bytecode-v1 encoding.

Compilation validates resource reads and effects but performs no external read,
prepare, apply, deliver, driver attach, or driver start. Compiler diagnostics,
source paths, resolver roots, filesystem capabilities, and temporary limits
are not embedded in the generated application. Direct `.mecb` input bypasses
source compilation but passes the same full artifact admission and native
contract checks.

The shared `decode_program_artifact_bytecode_v1` entry point requires artifact
sections, rejects parallel legacy register/instruction/type authority, and
checks equality between the outer and artifact-owned application requirement
tables. Instruction-only bytecode is a valid low-level format but is not an
executable resident application. Native planning rejects it before generating
a project; the generated runtime never falls back to instruction execution.

## Trusted linkage

Native planning derives value and shape features from the artifact schema
arena, and operation-specific resident features from canonical operation
identities. It validates the artifact through the configured resident catalog
and provider contracts before producing a plan. Every application requirement
must belong to an artifact operation node; unreferenced requirement authority
is rejected.

Generated catalogs install the maintained engine intrinsic resident binders.
They do not reconstruct a second instruction stream or choose implementations
from runtime-function IDs embedded in bytecode. Provider-owned planning checks
resource schemas and value-domain obligations without performing external I/O.
Fallible resident execution and resource preparation remain responsible for
validating actual runtime values before publication.

`NativeFunctionLinkage`, runtime-signature checks, and low-level instruction
contract tests remain available as compatibility and owner-inventory coverage.
The native builder's instruction and legacy type analysis helpers are
compiled only for tests. Those tests exercise their contract APIs directly;
they do not treat successful instruction validation as evidence that a native
application can load or execute it.

The standard native host catalog is built from the same private registration
records as source-planning factories. Its exact provider surface is:

| Provider | Package | Feature | Generated factory |
| --- | --- | --- | --- |
| `cli` | `mech-terminal` | `provider` | `mech_terminal::CliHostFactory::with_arguments` |
| `console` | `mech-console` | `native` | `mech_console::NativeConsoleHostFactory::new` |
| `time` | `mech-time` | `native` | `mech_time::NativeTimeHostFactory::new` |
| `timer` | `mech-timer` | `native` | `mech_timer::NativeTimerHostFactory::new` |
| `scene` | `mech-scene` | `native` | `mech_scene::NativeSceneHostFactory::new` |
| `robot-arm` | `mech-robot-arm` | `provider` | `mech_robot_arm::RobotArmHostFactory::new` |

These providers support Unix and Windows native targets. Browser, Wasm,
unknown, and unsupported native providers are rejected.

## Actor and host-function boundary

Generated native applications execute through canonical resident artifacts. The
production native host catalog registers resource providers, not actor host
functions. `NativeApplicationBuilder::plan` rejects any actor bootstrap with
`NativeActorBootstrapUnsupported` before bytecode admission or canonical
contract planning. Public project rendering enforces the same boundary;
there is no generated actor execution template.

The retained host-function linkage types and effect-free actor planning tests
are internal compatibility machinery. They do not establish production actor
support or authorize restoring the retired interpreter to execute host calls.
A future host-function implementation needs an explicit canonical resident
execution contract and its owned qualification before native planning can
advertise it. Supported hosted resource applications continue through their
provider-owned observation and effect contracts.

## NativeBuildPlan

Analysis normalizes an optional `NativeRuntimeConfig`, validates requirements
against the trusted catalogs and target, and emits schema
`mech.native-build-plan.v1`. Grant paths use the runtime capability
normalizer before matching or hashing, and host settings reject non-finite
floats recursively so JSON plan identity remains lossless. A plan freezes:

- bytecode and bytecode version, application kind, target, profile, binary
  name, component version, and normalized runtime configuration;
- application requirements, host instances, grants, selected packages,
  schema-derived value features, resident operation features, and host factories;
- registry or workspace dependency source, optional workspace fingerprint,
  the frozen dependency-resolution SHA-256, live status, bytecode SHA-256,
  and plan SHA-256.

The v1 `runtime_types` and `runtime_functions` fields remain in the public plan
schema but are empty in canonical native plans. `actor_bootstrap` is absent;
production plans do not restore retired instruction or actor execution.

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

`catalog.rs` reconstructs the engine-owned resident catalog under the planned
feature closure. `runtime.rs` constructs the normalized runtime, selected host
factories and instances, and grants. `main.rs` embeds `program.mecb` with
`include_bytes!` and installs it with `MechRuntime::load_bytecode_program`.
Generated resident projects never depend on `mech-stdlib`, `mech-syntax`,
`mech-bytecode`, or `mech-build`, and never enable `source`, `compiler`, or
`native-plan`.

Both engine and hosted applications link `mech-core`, `mech-engine`, and
`mech-runtime`. A host-free engine application needs no host crates. Hosted
applications add only the selected native providers. Neither parses source or
configuration at run time.

Planning and runtime loading use the same full bytecode admission entry point.
Artifact schemas, nodes, slots, and provider contracts own resident execution;
legacy registers and output seeds are not a second execution authority. The
native builder additionally checks deployment-specific requirements such as
target Index width, trusted provider availability, grants, and feature closure.

Engine applications accept no argument or exactly `--once`; other arguments
print usage and exit with status 2. Hosted applications accept launcher controls
`--once`, `--print-result`, `--runtime-info` and `--max-live-turns N` before
their program arguments.
The first other argument, or an explicit `--`, ends launcher option parsing.
All remaining arguments belong to the program, including flag-looking values.
Use `--` to pass a reserved launcher option as the first program argument.

The standard CLI host receives an immutable payload-only `Vec<OsString>` through
its explicitly registered factory constructor. Element zero is `@args/1`; the
snapshot excludes the executable and consumed launcher controls. Empty strings
and non-Unicode OS arguments remain intact until the provider validates the
particular resource read. No process-global arguments are changed. Custom host
registrations keep their selected factory and constructor input contract.
Existing registrations default to no constructor input; the omitted default
preserves v1 serialized plans and their digest. An explicit argument-input mode
is serialized and changes the plan digest. Argument values are launch inputs,
not build-plan data.

A hosted program installs its canonical resident artifact and completes the
initial publication and any driverless observation turn before selecting its
lifecycle. With no actual driven live-input bindings, it shuts down after this
initial work, including under `--once`. CLI argument, environment and stdin
observations use this lifecycle even when their retained resource delivery is
live. The plan's live flag continues to describe resource delivery.

Hosted launchers display no implicit result by default. Program output is
delivered through its admitted host effects. `--print-result` explicitly enables
launcher presentation of a nonempty initial result, or the final root result
under driven `--once`. This presentation is separate from program I/O and does
not change resource grants. `--runtime-info` independently opts into launcher
execution metadata. Both controls end at the same program-argument boundary.

When the loaded program has driven bindings, the launcher installs a Ctrl-C
handler using `Arc<AtomicBool>` with `Ordering::SeqCst`, starts the selected
drivers, and drains at most 64 inputs per iteration. `--once` stops after one
accepted driven turn; `--max-live-turns N` bounds accepted driven turns. An empty
drain sleeps for 10 milliseconds. Normal exit and primary errors stop started
drivers and shut down the runtime; primary errors are reported before cleanup
failures. Only live projects depend on exact `ctrlc = "=3.5.2"`.

These generated applications execute through the resident runtime. Canonical
machine-code lowering is a separate compiler boundary described below.

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

The resident build interface is:

```text
mech build <INPUTS...>
  --backend resident|cranelift
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

## Canonical machine-code builds

The `mech-gpu` scalar canonical native API consumes `ProgramArtifact` through
`lower_pure_native_artifact`, using private semantic witnesses from the actual
selected resident binders. Its immutable typed IR supports stateless Turn
operations on `Bool`, `U64`, and `I64`. It rejects resources, constraints,
compute regions, retained state, computed Activation nodes, matches,
comprehensions, counted applications, and FSMs. Direct constant output aliases
are admitted only when their metadata identifies the same validated constant.

JIT compilation and object emission share one checked Cranelift definition
pass. JIT and linked invocation share the managed typed evaluation boundary:
finite setup/node/completion work, admitted transport storage, checked integer
arithmetic, and complete output validation before caller-visible publication.
Object emission alone is not linked execution evidence. The explicit linked
application qualification builds and executes generated applications and is
invoked by the full CI workflow.

The optional public `--backend cranelift` build consumes the larger checked
canonical program path, `generate_linked_hosted_program_application`. It uses
the same retained `ProgramArtifact`, exact compilation catalog and admitted
source-document owners as ordinary source compilation. It does not retry the
legacy sequence/control/grid frontends or resident execution on rejection.
Raw `.mecb` inputs are canonically decoded against the current standard catalog
and explicitly marked **source-unpaired**; they acquire no claimed source owner.

Within synchronous FSM transition blocks, an explicit mutable binding such as
`~scratch := xs` followed by `scratch[index] = value` lowers to successive local
values. Indexed updates use the maintained assignment kernel, with a fresh
candidate seeded from the explicit base. Closed rank-two unsigned matrices can
travel through structural state payloads; reconstruction admits the expanded
draft and temporary canonical cells before allocation. These are value copies,
so this support does not establish an in-place buffer throughput result.
Document-level mutable bindings continue to allocate retained state, which the
canonical native subset rejects.

This opt-in requires `cranelift_build`, a local workspace dependency tree, native
emit and the current supported host. Linked binary names must be 1–64 ASCII
letters, digits or hyphens, starting with a letter; they are never sanitized.
It rejects `--target`, other emit modes, `--scalar`, `--fp-contract-loop`,
`--debug`, `--trace` and runtime configuration (including discovered config;
use `--no-config`). Default resident builds and their bytecode remain unchanged.
The generated runtime-only binary uses the existing explicit launch contract:

```text
mech --no-config build program.mec --backend cranelift --name app \
  --workspace-root /absolute/path/to/mech --keep-project --offline --out ./app
./app launch.json report.json -- [PAYLOAD_ARGUMENTS...]
```

The launch JSON alone supplies grants and execution/resource/acquisition limits;
compiler planning uses empty CLI grants and does not embed launch authority.
The same runtime admits original requirements, executes generated closed
activation, captures provider inputs, resumes the same native session and
validates/commits effects. There is no runtime parser, JIT or resident body.
The current shell installs the CLI provider only, supports stateless String
Live observations/Snapshot sends and no ordinary turn inputs, and rejects
unsupported programs or providers rather than falling back. Root values are
not implicitly printed; only explicit resource effects can write output.
A program with no resources needs no grants and remains on this same shell.

`--keep-project` retains the exact pinned Object/bytecode/identity and Cargo
project. Its `application.mec` is a JSON provenance-only bundle, not executable
Mech source: ordered accepted roots/dependencies carry URI, revision, nominal
origin/package and exact UTF-8 bytes. Packaging does not reopen source files.
Building uses the shared frozen workspace lock seed, isolated pinned Cargo,
`--locked` and Cargo's exact reported executable. The native program's supported
slice is selected by the maintained artifact compiler, not by benchmark names.
A release Cargo profile does not itself change the emitted Object's optimization
setting. Emission/build metadata alone is not execution or timing evidence.

This adapter implements the secured launch contract only. Setting the legacy
`MECH_NATIVE_SECURITY=trusted` environment variable does not bypass grants.
A controlled same-body trusted/secured lifecycle comparison, full-input suite
qualification and benchmark publication remain separate outstanding work; this
CLI integration does not complete them or establish new timing results.

Mixed `@compute`, `@cpu`, and `@gpu` applications use the separate
`mech-compute` backend/session contract. Their physical CPU and wgpu executors
are not substitutes for the resident application coordinator. The stable
product lowering and supported backend scope are documented in
[`hosts/gpu/README.md`](../../hosts/gpu/README.md).

## Compatible live replacement

Compatible resident replacement transfers an accepted publication, not an
in-flight computation. It rejects `SuspendedContinuationMigration` before
copying retained inputs or state when either instance owns a suspended frame.
The old instance keeps its captured frame, ready queue, generation-bound wakeup
and epoch; the host can finish that computation and retry. Completed FSM
results may migrate, and projection refresh must not resume a continuation,
trigger a dormant activation, observe a provider or invent another turn.

Document revisions must retain their defining nominal origin. Independently
owned nominal declarations are not interchangeable even when their shape or
source spelling matches. Anonymous state migration between different artifact
revisions requires one unambiguous semantic writer/initializer match, not
equality of artifact-local node offsets. Within one exact immutable revision,
the declaration's own cell identity is retained unless state was explicitly
mutated.

Explicitly mutated or newly declared state keeps the candidate's ordered
initial publication. Mixed projections must still incorporate independently
migrated state: preserving the whole candidate projection would retain a stale
initializer, while replaying a prefix from the final state would apply a
transition twice. Refresh therefore reads candidate-owned state initializers
through the existing inactive state buffers, while final writer results and
state outputs reuse their actual publication. Retained state uses its migrated
publication. Hidden invariants use the same ordered read view. The temporary
view is cleared on success and failure, without advancing the published epoch
or cloning a state arena; bookkeeping is bounded by the declared state count.

The coordinator separately retains the input snapshot associated with the
last accepted publication. A rejected turn cannot replace that snapshot.
Payloads use shared immutable ownership and each retained snapshot is bounded
by the existing input limit, but the older accepted snapshot can outlive the
latest capture: this has additional retained-memory cost.

A declared observation can remain unpublished until its first accepted input.
Optional interactive symbol inspection returns absence in that interval, just
like output-ID inspection. Rendering a placeholder neither reads a provider nor
invents a turn, and absence alone is not a document failure. Strict root-value
queries still report unavailable values; genuine query errors still propagate.

## Security boundary

```text
program:       embedded bytecode data
runtime:       statically linked Rust code
catalog:       maintained resident binders and selected host factories
mutable state: allocated at process startup
```

Bytecode cannot name arbitrary crates, Cargo features, installer paths, or
host factories. Only trusted catalogs compiled into the build tool cross from
program requirements into Rust dependencies. When resident activation resolves
an operation through the dynamic ABI loader rather than a catalog factory,
planning retains the engine's `dynamic-modules` feature in the generated
application. The executable resolves that module through `MECH_MODULE_PATH`
or `target/mech-modules` and validates its ABI, operation, and layout at
activation, without a source parser or compiler. Native generation does not
copy module libraries; they must also be available when the executable runs.
Static resident operations do not enable this loader.

Generated applications contain no third-party Cargo package discovery, source
parser, compiler, build tool, appended archive, or self-reading executable
payload.
