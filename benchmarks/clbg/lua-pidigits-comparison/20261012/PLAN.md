# Lua interpreted pi-digits closure plan

The explorer has nine measured Lua interpreter workloads; LuaJIT already has
ten. The recorded interpreter is `/opt/homebrew/bin/lua`, Lua 5.5.1, SHA256
`83ea84efb4f7692a4452cfedf9325cc1f342edc6cf0d50b262add4eab1b75bea`.
The historical pi-digits report is preserved: source #1 failed because its old
bn library lacked `number`; sources #5/#6/#7 lacked the required c-gmp module.
The old LuaJIT #1 uses a generic OpenSSL bn library, rather than GMP.

Four isolated formatting-only copies of the listed #1/#5/#6/#7 sources retain
the complete sequential unbounded spigot and both actual quotient extracts for
every tested candidate. Only declarations, multiple assignments, semicolon
packing and inline if bodies are expanded physically. Initializers are
independent and retain their original evaluation order. All original upstream
bytes and old build modules remain unchanged. The locked prepare stage will
verify each original manifest hash, regenerate the same normalized bytes,
compare original and normalized outputs, and record LOC expansion evidence.
Normalized nonblank/noncomment Lua lines are 62/77/63/57 respectively; standalone
Lua `end` lines retain the same counting treatment as existing Lua measurements.

`support/gmp.c` is an ordinary general library adapter. It supplies the legacy
c-gmp mutable API (initialization, assignment, addition, signed/unsigned scalar
multiplication, add/subtract-multiple, quotient-only truncation/floor division,
numeric conversion) and the immutable bn API (constructor, addition,
subtraction, multiplication, quotient, comparison, conversion). Every arithmetic
entry calls the corresponding GMP primitive. There is no loop over digits,
spigot state, extraction selection, input recognition or pi output in C.
Mutable APIs preserve ordinary GMP aliasing, reusable cell capacity and signed
rounding; division rejects zero. Lua userdata owns/retires mpz storage. Exact
small integral numeric values use Lua 5.5's integer representation, preserving
older Lua's ordinary `io.write` formatting; large conversions retain GMP's
floating conversion. Both module aliases are built into a new private directory;
existing bn, LuaJIT, compiler and cohort sources/libraries are not modified.

A single bounded qualification under `/tmp/mech-clbg-timing.lock` will compile
this adapter against the installed Lua/GMP headers and verify generic signed,
aliasing, large-integer, comparison, conversion, invalid-handle, division-error
and lifetime behavior. Output cases include zero, negative input, omitted input,
invalid CLI input falling back to 27, every small tail length, 9/10/11, 19/20/21,
27/30/97/100/1000 and complete 10,000 digits. Original and normalized sources
must produce identical complete bytes. An untimed Lua arithmetic-call tracer
records both exact divisions for each candidate at N97, verifies quotient
rounding and matching denominators, and never appears in timed commands.

Preparation freezes complete runtime/module/header/library/source/runner and
output-reference hashes, with historical exclusions and original upstream
provenance. This produces a sealed manifest for root review. **No full timing
launch or chart admission occurs before review of that exact manifest.**
The proposed complete study has three fresh full N10000 executions of each of
the four sources and three fresh executions each of the unchanged current Mech
fast 38-LOC and compact 35-LOC AOT sources, eighteen processes total, in rotating
orders. The primary time multiplier uses the fresh fast Mech control uniformly;
fastest/fewest LOC multipliers use 38/35 respectively. The extra compact control
also makes its own source-specific speed comparison inspectable. Runtime
startup, all computation, ten-digit rows/padding/counts, complete output and
shutdown are included uniformly. No previous samples, scaling or shortened
workloads are admitted. The frozen common runner's AC/normal-power, compiler,
resource and process-group qualification remains in force; the sampler and
native child inherit the held common timing mutex through cleanup/persistence.
Only environmental failures may retry within the recorded bound; other failures
are preserved and stop the study. The next additive importer must reconcile all
eighteen full records, retain every old language/source/result, and replace
Lua's nine-workload coverage note only after actual admission.
