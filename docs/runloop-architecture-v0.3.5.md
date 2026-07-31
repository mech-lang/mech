# RunLoop Architecture Plan (v0.3.5)

## Goal
Define a clear message-driven runloop contract so the Mech runtime can interact with `program` through explicit commands/events instead of direct parser/interpreter calls.

---

## 1) Runtime model

### Roles
- **Host Runtime (mech CLI/WASM/server):** Sends commands to the runloop.
- **Program RunLoop:** Owns a `Program` instance and serializes state transitions.
- **Program:** Owns parser/interpreter/core and executes work.

### Principle
All mutations and evaluations happen by sending `RunLoopMessage` into a single-threaded loop. This makes ordering deterministic and simplifies future multi-interpreter orchestration.

---

## 2) Message taxonomy

We split messages into:
1. **Commands (runtime -> runloop)**
2. **Events (runloop -> runtime)**
3. **Data-bearing notifications (optional, for UI/telemetry)**

---

## 3) Proposed command messages (runtime -> runloop)

```rust
pub enum RunLoopMessage {
  // Lifecycle
  Initialize(ProgramConfig),
  Stop,
  Pause,
  Resume,
  Reset,

  // Configuration
  Configure(ProgramEnvironment),

  // Source loading / execution
  LoadSource { source_id: String, source: String },   // compile/load into Program/Core
  EvalSource { source_id: String, source: String },   // parse + interpret now

  // Scheduling
  Step { rounds: Option<usize> },                     // defaults to env.rounds_per_step
  RunUntilIdle,

  // Data ingress / egress (future)
  InjectTransaction { txn: Transaction },
  RequestSnapshot,

  // Introspection
  RequestStatus,
  RequestMetrics,
}
```

### Notes
- `LoadSource` and `EvalSource` are intentionally separate.
- `Step` accepts override rounds but should typically use environment defaults.
- `Initialize` allows cold-start setup from the runtime.

---

## 4) Proposed event messages (runloop -> runtime)

```rust
pub enum ClientMessage {
  // Lifecycle/state
  Ready,
  Paused,
  Resumed,
  Stopped,
  ResetDone,

  // Ack / completion
  Configured,
  SourceLoaded { source_id: String },
  SourceEvaluated { source_id: String },
  StepDone { rounds_executed: usize },
  Idle,

  // Data/inspection responses
  Snapshot { summary: String },
  Status { state: RunLoopState },
  Metrics { step_ns: u128, parse_ns: u128, eval_ns: u128 },

  // Errors
  Error(RunLoopError),
}
```

---

## 5) RunLoop state machine

```text
Created -> Initializing -> Ready -> Running <-> Paused
                           |            |
                           +----> Idle -+
Any state --Stop--> Stopped
Any state --Error--> Ready (recoverable) or Stopped (fatal)
```

### State rules
- `EvalSource` in `Paused` is allowed (depends on policy; default: allowed).
- `Step` in `Paused` returns `Error(InvalidState)`.
- `Configure` is valid in `Ready/Paused/Idle`; rejected while actively stepping if non-atomic.

---

## 6) Processing loop contract

Pseudo-flow:
1. Receive one `RunLoopMessage`.
2. Validate against current `RunLoopState`.
3. Execute in `Program`.
4. Emit exactly one terminal event (`...Done`, `Error`, etc.), plus optional telemetry.

This “one command -> one terminal event” rule simplifies host logic.

---

## 7) Error model

```rust
pub enum RunLoopError {
  InvalidState { command: String, state: RunLoopState },
  Parse(String),
  Compile(String),
  Eval(String),
  ChannelClosed,
  Internal(String),
}
```

- Preserve typed categories so runtime can decide retry/abort behavior.
- Avoid flattening into plain strings where possible.

---

## 8) Environment integration

`ProgramEnvironment` controls defaults used by runloop:
- `trace_enabled`
- `debug_enabled`
- `time_enabled`
- `print_tree`
- `rounds_per_step`

Runloop applies `Configure(env)` atomically to `Program` and emits `Configured`.

---

## 9) Minimal v0.3.5 implementation phases

### Phase 1 (now)
- Keep current channel architecture.
- Add explicit `RunLoopState` and enforce command validity.
- Expand message enums with lifecycle/config/eval separation.

### Phase 2
- Add `RequestStatus`, `RequestMetrics`, and structured errors.
- Add `Step { rounds }` semantics tied to `rounds_per_step`.

### Phase 3
- Add transaction ingress/snapshot responses.
- Add multi-interpreter routing keys (e.g., `target_interpreter_id`).

---

## 10) Multi-interpreter future-proofing

Introduce optional routing envelope later:

```rust
pub struct Routed<T> {
  pub target: ProgramTarget,   // Root | Interpreter(u64) | Broadcast
  pub payload: T,
}
```

This allows one runloop to manage root + subinterpreters without redesigning every host integration.

---

## 11) Integration checklist for mech runtime

Runtime side should:
1. Start runloop and wait for `Ready`.
2. Send `Configure` once from CLI/WASM options.
3. Use `LoadSource` for file load and `EvalSource` for ad-hoc execution.
4. Drive execution via `Step`/`RunUntilIdle`.
5. Render output/errors based on typed `ClientMessage` events.

