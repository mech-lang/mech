# Program module revival plan (v0.3.5)

## Goal
Re-enable `src/program` as the orchestration layer above `core` with a minimal, maintainable API for v0.3.5.

## What was kept now
- `Program` wrapper that owns a `Core` and compiles source through `mech-syntax` compiler.
- `ProgramConfig` with an initial `rounds_per_step` field for core round semantics integration.
- `ProgramRunner` + `RunLoop` channel loop with only `Load`, `Step`, and `Stop` commands.
- `ClientMessage` event channel for ready/step/stopped/error signaling.
- `Persister` placeholder type to preserve crate shape for future persistence work.

## What was intentionally disabled for now
The legacy code remains in git history but is not compiled in v0.3.5:
- Dynamic machine download/loading (`libloading`, registry download, machine repository).
- Remote core discovery and networking orchestration (UDP/WebSocket maestro flow).
- Capability token exchange and distributed core syncing.
- Legacy persistence protocol that serializes internal changes.
- Legacy utility dependencies (`mech-utilities`) and old protocol message types.

## Upgrade notes applied
- Crate version and dependency versions updated to v0.3.5-compatible values.
- Rust edition moved to 2024 to match workspace.
- Removed old nightly-only feature flags and outdated crates.
- Retained module names (`program`, `runloop`, `persister`) so imports stay stable while internals are modernized.

## Next upgrades (follow-up)
1. Define a concrete `Program` API for args/filesystem/runtime configuration.
2. Add core-level round semantics hooks and expose round stepping from `ProgramRunner`.
3. Reintroduce persistence behind a feature flag with a stable format.
4. Reintroduce distribution/networking behind a separate feature flag and modern transport stack.
5. Add crate tests for compile/load/step lifecycle and event sequencing.
