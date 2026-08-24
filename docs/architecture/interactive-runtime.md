# Interactive runtime architecture

Status: accepted architecture for the portable resident REPL, served documents, browser compute, and presentation adapters.

## Ownership

`ResidentReplSession` and the typed runtime request/event protocol are the authoritative interactive system. A frontend may collect input, satisfy host requests, schedule cooperative work, and render typed events. It must not reimplement command behavior, source transactions, symbol identity, retained output policy, diagnostic ownership, or undo/redo semantics.

Requests flow `frontend input -> typed REPL request -> ResidentReplSession -> runtime`; presentation flows `runtime -> typed program/REPL event -> frontend renderer`.

CLI, WASM document, standalone browser presentation, editor, and future native application integrations are adapters on those two directions.

## Output and diagnostic ownership

Program output is published by the running program and retained by stable display identity. Interactive evaluation may update `ans`, but it does not replace the document's fixed program output. Directed program output owns the target display until that directed lifecycle ends.

Program diagnostics belong to the Errors stream. Interactive parsing, commands, and host-interaction failures belong to the REPL transcript. Typed events cross a causal barrier before a subsequent interactive mutation, so an inspection or clear cannot overtake a producer event.

## Transactional document replacement

Source replacement activates and validates a candidate runtime before retiring the current runtime. Compatible state and live inputs move only after planning succeeds. Once retirement begins, the prepared candidate commits; cleanup failures are warnings and never resurrect a partially stopped runtime.

The value returned by an accepted edit is recaptured after migration and output projection refresh. The submitted value, `ans`, document projection, and `:whos` therefore describe one accepted state epoch.

## Runtime generation and physical revision

Runtime generation identifies commands, completions, logical ownership, and stale-work rejection. Physical revision independently identifies compatible devices, pipelines, binding layouts, buffers, and state resources.

A compatible logical generation may inherit resources from the same physical revision. An incompatible revision reports an explicit reset. A completion from a retired generation is always rejected even when physical resources were transferred.

## Retained values

The replacement contract distinguishes declared outputs, statically retained outputs, active sample subscriptions, lazily materialized sample cache entries, and backend-resident unretained outputs.

Compatible replacement migrates only state explicitly named by the replacement contract. Runtime-only observation must not enlarge that contract. Report-only compute keeps values on the backend unless an actual read is requested.

## Physical compute plan

Rust owns the physical compute model. `GpuExecutionPlan` is the sole definition of bindings, physical state, logical output aliases, dispatch shape, integrity encoding, layout, limits, and physical revision. Native wgpu and browser WebGPU consume that plan. JavaScript may allocate and execute the encoded plan; it may not infer compiler semantics or invent physical layout.

Browser compute has one session lifecycle: generation ownership, device and pipeline ownership, command backpressure, output selection, submission, completion, integrity rejection, terminal failure, compatible transfer, and disposal. Document and standalone presentation behavior are adapters over that lifecycle, not separate compute implementations.

## Browser component contract

Production controller and stylesheet code use one canonical component schema: `data-mech-repl-host`, `data-mech-console-pane`, `data-mech-console-panel`, `data-mech-console-tab`, `data-mech-console-resizer`, `data-mech-console-mode`, and `data-mech-presentation-view`.

State is represented once. The `hidden` property owns ordinary visibility and `data-mech-console-mode` owns fullscreen workspace mode. Compatibility markup, if ever required for an external embed, is normalized before the canonical controller starts and is not queried by core controller or component CSS.

Output fullscreen has one panel-level scroll surface for ordinary document and text output. Scene/canvas content explicitly opts into fill geometry.

## Production and test APIs

The production document controller exposes source inspection/replacement, rendered value access, typed request invocation, and disposal. Server-provided host configuration is a real authority boundary.

Test instrumentation is opt-in and separate. Browser scenarios observe one structured snapshot and inject faults/completions through a test bridge instead of accumulating production `__MECH_*` globals or observed-state attributes.

Real-browser scenarios share one Chrome/CDP/server harness. Scenario modules contain only product setup and assertions. Consolidation must preserve the CPU, WebGPU, particle, EKF, N-body, REPL, output, error, replacement, backpressure, readback, and disposal proofs.

## Structural rules

- No crate or build script uses crate-wide `allow(warnings)`; necessary lint allowances are local, named, and explained.
- `ComputeSession` exposes one request-based dispatch operation, and browser compute exposes one versioned completion operation.
- Sample selection names the sampled instance explicitly; physical-plan derivation occurs in Rust and crosses one wire encoder.
- Core browser code uses only the canonical `data-mech-*` schema, and every browser test delegates process and CDP ownership to the canonical harness.

These rules are checked by the inexpensive interactive-architecture contract in CI. Changes that need a new compatibility path must update this record and add a removal boundary rather than reintroducing parallel semantics.
