# Program Interface Design (v0.3.5)

## Problem
Today, execution call sites still pass ad-hoc flags (`tree`, `debug`, `time`, `trace`) into helper functions. That keeps parser/interpreter startup and runtime policy outside the `program` abstraction, which is the opposite of the intended architecture.

## Design goals
1. `program` is the runtime boundary for consumers (CLI/WASM/other hosts).
2. Parser + interpreter lifecycle are owned by `Program`.
3. Runtime behavior flags are configuration, not per-call plumbing.
4. Keep a small core API now, with room for multi-interpreter/sub-interpreter expansion.
5. Preserve compatibility with existing `runloop` usage while moving toward environment-based configuration.

## Proposed API

### 1) Program environment
Introduce a dedicated environment/config struct owned by `Program`.

```rust
pub struct ProgramEnvironment {
  pub trace_enabled: bool,
  pub debug_enabled: bool,
  pub time_enabled: bool,
  pub print_tree: bool,
  pub rounds_per_step: usize,
}
```

This replaces passing flags into `run_mech_code`/`run_string` at call sites.

### 2) Program config and construction

```rust
pub struct ProgramConfig {
  pub name: String,
  pub environment: ProgramEnvironment,
}

impl Program {
  pub fn new(config: ProgramConfig) -> Self;
}
```

`Program::new` initializes parser/interpreter state and applies environment defaults (for example, interpreter trace setting).

### 3) Program execution surface

```rust
impl Program {
  pub fn load_source(&mut self, source: &str) -> MResult<()>;   // compile/load path
  pub fn eval_source(&mut self, source: &str) -> MResult<Value>; // parse+interpret
  pub fn step(&mut self) -> MResult<()>;                         // runtime step/round hook
  pub fn set_environment(&mut self, env: ProgramEnvironment);
  pub fn environment(&self) -> &ProgramEnvironment;
}
```

- `eval_source` uses parser + interpreter directly.
- instrumentation/printing behavior is derived from `environment`.
- `step()` is reserved for round semantics integration.

### 4) RunLoop/Runner contracts
`RunLoopMessage` should carry environment mutation and execution intents, not raw debug flags:

```rust
enum RunLoopMessage {
  Load(String),
  Eval(String),
  Step,
  Configure(ProgramEnvironment),
  Stop,
}
```

This keeps host concerns (CLI/WASM/UI) separated from runtime policy.

## CLI/WASM integration model
- CLI builds a `ProgramConfig` from command options once, then runs through `Program`.
- WASM builds a `ProgramConfig` from UI defaults and mutates via `Configure` when needed.
- No direct parser/interpreter startup at host call sites.

## Migration plan (small, safe steps)
1. **Introduce `ProgramEnvironment` and attach to `ProgramConfig`** (no behavior change).
2. **Move flag handling from `src/run.rs` helper signatures into `Program::environment` reads**.
3. **Update `src/bin/mech.rs` and `src/wasm/src/lib.rs` to construct `ProgramConfig` once and stop passing flag tuples.**
4. **Extend `RunLoopMessage` with `Configure`/`Eval` and route through `Program` methods.**
5. **Add focused tests** for config propagation and eval/step behavior.

## Immediate constraints and non-goals
- This design does not reintroduce legacy networking, dynamic loading, or persistence.
- Multi-interpreter orchestration is planned but deferred; the interface is shaped to allow it.

## Why this resolves the current muddle
- The host no longer decides runtime semantics on every call.
- Program becomes a real orchestration boundary with a stable environment model.
- Future rounds/sub-interpreters can be added behind `Program` without changing every consumer.
