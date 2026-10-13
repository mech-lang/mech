# Julia safe trimming with concrete output handles

The original official safe-trim build is preserved as an actual failure.
The six verifier diagnostics contain three distinct calls, repeated twice:
`showerror(stderr::IO, exception::Any)`, `println(stderr::IO)` and
`print(stdout::IO, String, newline)` through the driver's `println(String)`.
The compiler subsequently failed in its error-reporting path with an undefined
`Base.Compiler.exit`; that secondary error is not the source diagnosis or an
excuse to bypass the verifier. No workload, link, benchmark or LLDB command
executed. The original pilot and all original outputs remain unchanged.

Root closed 10,038 pins in
`continuation/jobs/julia-safe-trim-build-closed-review-v1/receipt.json`, SHA
`929d8dc37de11c1dd5887dad8399e10cde5a3b90cabd86b85ec8be17a39c9666`.
The failed build outcome is
`42c319719f806306c73a89192dc1638f4810c28de9367eb794c14f998b5d5634`,
durable terminal
`63687cd23db5886735d4c2fb8b6238571e7c7544b87583773b5b1856c844fe7a`.
Its native compile receipt is 494570. The new worker binds the complete failed
request/start/worker/terminal identity, target 49624/worker 49618, exact source
argv, actual compiler PID 49668, normal exit 1, zero warnings and six saved
verifier diagnostics. All original prerequisite/available pins are rebound
and retained, and the failed private lease must be closed. This is a fresh
source-derived build rather than a retry against unchanged input.

The exact Julia 1.13.1 sources explain the concrete repair. `base/coreio.jl`
binds Base.stdout and Base.stderr as abstract IO globals. `base/boot.jl`
defines constant concrete `CoreSTDOUT`/`CoreSTDERR` instances and forwards
their writes to Julia's runtime output handles. The shipped
`test/trimming/hello.jl` uses `println(Core.stdout, "Hello, world!")`.
These three retained files are bound to their actual prior hashes 1b52bf,
575b5b and 5eb370 in the worker. They are read as source evidence; no shipped
test or new fixture is executed. The official builder/trim patches remain
byte-exact and the compiler verifier decides whether the new entry is closed.

Only three declared source spans change:

1. Each of the two energy prints receives `Core.stdout` explicitly. The
   `Base.Ryu.writefixed(energy(bodies), 9)` expression, newline, print order,
   state initialization and all runtime integration steps remain intact.
2. The catch wrapper no longer formats an arbitrary exception through
   abstract stderr. It prints the fixed `nbody entry failed` diagnostic to
   `Core.stderr` and returns the existing nonzero Cint 1. This error-text change
   is deliberate; it does not change success output or convert failure into
   success. Negative-input/error-status qualification remains a later gate.

The worker starts from the already accepted exact nbody workload adaptation.
It requires exactly two occurrences of the old energy-print expression and
one exact catch block in the original entry. Its span editor constructs the
new text from declared replacements plus complete untouched original spans.
The reviewed `entry_static.jl` must equal that derivation exactly. The
application module, original C glue, Project.toml and matched JIT driver are
copied byte-exact into a new private tree. The unchanged matched JIT driver's
relative include resolves the same new module and therefore the same derived
numerical kernel, initializer and output driver. No JIT source can silently
retain the old abstract output path. The existing JIT script's direct Julia
driver call and C ABI argument conversion remain their original separate
entry routes; this build does not claim those boundaries have been timed or
qualified yet.

The actual source derivation receipt records original and new workload/entry
paths, hashes, all three edits and every untouched span, plus the explicit
fixed-error-message change. Generated workload text is immutable before the
compiler starts. The compiled source-input map distinguishes its declared I/O
derivation from unchanged copies. The original numerical code, arithmetic,
state layout, @fastmath choices, steps, parse, energy formatting and outputs
are preserved. This is an output/entry compatibility adaptation, with no
measured optimization or source-count result claimed.

Four source files form this packet: `build.py`, `entry_static.jl`,
`build_options.jl` and this note. The options wrapper is identical to the
previous accepted body; only its comment now discloses the source I/O edits.
It retains the real JLOptions checks and official builder/root protocol:
trim=safe 1, strip_ir 1, strip_metadata 0, nonincremental 0, O3/g2, eight threads,
object/bitcode outputs, ccallable roots, empty build-time argument/load/depot
paths and uninitialized workload state. It still records options before the
compiler's final verifier, so that TOML alone is not a successful build.

The bootstrap 0756, prior pilot 39d2, original upstream 2def, adaptation 7edf and
closed no-native-retry calibration 5cdb remain exact. All original runtime,
sysimage, builder, trim-base/stdlib, ABI-exporter, Makefile, C glue and pilot
dependencies are preserved and rechecked. The original accepted owned C3
service remains unchanged. Only one new guarded compiler command is reachable,
bounded at 1,800 seconds and serialized under common inode 79859134:

```
<pinned Julia 1.13.1> --startup-file=no --history-file=no --threads=8,0
  --project=<fresh module> -J <pinned sysimage> -O3 -g2
  --output-incremental=no --experimental --trim=safe --strip-ir
  --output-o=<fresh object archive> --output-bc=<fresh bitcode>
  <reviewed options wrapper> <unchanged official buildscript>
  <fresh module source> --output-lib true <fresh ABI JSON> <fresh options TOML>
```

There is no unsafe trim, verifier suppression, arbitrary compiler root,
Base/compiler patch, source evaluator, application execution, linker or LLDB
retry. A closed shared lease guards the original failure; a separate private
owner lease and fresh output/build directories reject duplicate work or an
old-output overwrite. The same owned process identity/group, inherited common
FD, timeout, cleanup, output drains and terminal integrity rules apply.
Failures preserve new diagnostics and original evidence.

After root/peer source review and source capture/freeze, the next exact entry
is the prior accepted build invocation plus the new reviewed entry digest:

```
bundled-python3.12 -I -S -B -u static-io-safe-trim-v1/build.py
  --revision official-static-io-safe-trim-nbody-v1
  --worker-sha256 <actual worker SHA>
  --options-sha256 <actual options SHA>
  --review-sha256 <actual note SHA>
  --pilot-sha256 39d2d809b45780918c6aa11e6e3d7f9f41218fd940b12d29297244f734259e95
  --entry-sha256 <actual reviewed static entry SHA>
```

An exit 0 with nonempty object/bitcode, actual options and exact Cint/argv ABI
only establishes a successful safe build pending root diagnostic review.
Runtime AOT, actual IR removal, native owner/image, fallback closure, output
equivalence, negative cases, LOC and chart admission remain unqualified.
Those gates are retained and require later concrete reviewed steps. No new
native command, hash, parser, import, fixture, test or Git operation has been
executed by the author of this source packet.
