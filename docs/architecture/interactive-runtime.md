# Interactive runtime architecture

Status: accepted architecture for the portable resident REPL, served documents, browser compute, and presentation adapters.

## Ownership and event flow

`ResidentReplSession` and its typed requests/events are authoritative. Frontends collect input, satisfy host requests, schedule cooperative work, and render events; CLI, WASM, browser, editor, and native integrations are adapters, not alternate command, transaction, identity, output, diagnostic, or history implementations. Program output retains stable display identity and is not replaced by `ans`; program diagnostics go to Errors, interactive failures to the transcript, and causal event barriers prevent later mutations overtaking producer events.

## Transactional state

Replacement validates a candidate before retiring the accepted runtime, migrates only contract-named compatible state and live inputs, commits after retirement begins, and treats cleanup failure as a warning. Returned values are recaptured after migration and projection refresh so submission, `ans`, document output, and `:whos` share one epoch. Generation owns commands, completions, logical work, and stale-work rejection; physical revision separately owns devices, pipelines, layout, buffers, and transferable state. Observation never enlarges retention, and report-only values stay backend-resident until requested.

## Compute

Rust's `GpuExecutionPlan` alone defines bindings, state, aliases, dispatch, integrity encoding, layout, limits, and revision for native and browser consumers. JavaScript executes that plan without inferring compiler semantics. Browser compute has one lifecycle for generation and resource ownership, backpressure, selection, submission, completion, integrity rejection, terminal failure, compatible transfer, and disposal; document and standalone presentations adapt it.

## Browser and test contracts

Production uses only canonical `data-mech-*` component state; `hidden` owns ordinary visibility, console mode owns workspace fullscreen, output has one panel scroll surface, and scenes opt into fill geometry. The controller exposes source replacement, rendered values, typed requests, and disposal behind server host authority. Test instrumentation is separate and opt-in, and every real-browser scenario delegates Chrome/CDP/server ownership to one harness while preserving CPU, WebGPU, particle, EKF, N-body, REPL, output, error, replacement, backpressure, readback, and disposal coverage. CI rejects crate-wide warning suppression, parallel compute ABIs or plans, implicit sample instances, legacy browser schemas, and browser tests that bypass that harness.
