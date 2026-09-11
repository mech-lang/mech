# Historical Phase 0 REPL corpus

These 22 cases describe the retired syntax-owned command parser. They retain
the original bytes and snapshots for the frozen Phase 0 grammar audit.

The active command language lives in `src/runtime/src/repl_command.rs`, with
registry, command parsing, source/request boundary, argument quoting, and
serialization tests in that module. It intentionally differs from this old
corpus (for example, `:symbols` is retired and LF input is supported).

These files are not part of the active `mech-syntax` fixture manifest and do
not imply that REPL command parsing belongs in the unfinished document parser.
