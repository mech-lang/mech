# Root CLI warning cleanup

- Original root `cargo build --bin mech` warning count: 88.
- Final Linux root `cargo build --bin mech` warning count: 0.
- Final Windows root warning count: not run in this Linux container.

## Deleted or simplified items

- Removed obsolete root pretty-print helper functions `pretty_print_tree` and `pretty_print_symbols`.
- Removed the unused context-addressed-source detector family from `src/cli/run.rs`.
- Removed `collect_run_targets` in favor of the capability-aware run target collection path.
- Removed unused host-grant intersection and CLI stream validation helpers.
- Removed unused source-discovery result fields and unused broad collection wrapper/policy variants.
- Removed stale test helper `mech_bool`.

## Repaired behavior

- Replaced nonexistent root `wasm` feature cfgs with `target_arch = "wasm32"` cfgs.
- Declared build-script file-presence cfg names and rerun inputs.
- Reduced ambiguous root glob exports and kept namespaced crate access.
- `print_prompt` and `clc` now return `MResult<()>` and propagate terminal I/O errors.
- REPL `:clear` no longer accepts an ignored argument.
- REPL evaluated values now use their formatted output.
- CLI diagnostics are invoked from the binary presentation boundary.

## Deferred concerns

- `Value::Typed(..) => todo!()` remains intentionally unchanged.
- Clippy hardening still exposes unrelated broader CLI/library lints, especially large `MechError` result types, and needs separate architectural handling.
